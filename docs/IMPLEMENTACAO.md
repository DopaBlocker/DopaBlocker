# DopaBlocker — Lógica dos Arquivos na Ordem de Implementação

Este documento percorre o repositório **na ordem real em que o código nasceu** (extraída do `git log --reverse`). Para cada arquivo, o foco é o problema que ele resolve e o papel que cumpre — não o passo-a-passo do código.

## Como ler este documento

- Cada **Marco** corresponde a um commit (ou um pequeno grupo de commits) que representa um avanço concreto do projeto.
- Dentro do marco, os arquivos aparecem na ordem em que fazem mais sentido entender.
- Quando o "porquê" depende de um conceito grande (Bloom Filter, JWT, SQLCipher, WFP), há um link para [docs/CONCEPTS.md](CONCEPTS.md) em vez de re-explicação.
- Detalhes técnicos só aparecem quando são **a razão** do arquivo existir (ex: `PRAGMA key`, prefixo `dt_`, regra do pai imune).

---

## Marco 1 — Estrutura inicial do projeto (`1885a4b`)

Esse commit é o **esqueleto da casa**: cria as pastas, declara as crates do workspace Rust, configura o monorepo (pnpm-workspace, Cargo workspace) e gera os arquivos placeholder de cada subprojeto. Quase nenhum código de negócio existe ainda — é a planta baixa.

As pastas-raiz que nasceram aqui:

- [backend/](../backend/) — API REST em Rust + Axum. Fonte-da-verdade dos dados (usuários, blocklist, vinculação parental).
- [desktop/](../desktop/) — App Windows feito em Tauri 2 (frontend SvelteKit + backend Rust nativo).
- [mobile/](../mobile/) — App Android em Flutter + Kotlin nativo (esqueleto, sem implementação completa).
- [shared/](../shared/) — Crate Rust compartilhada entre backend e desktop. Modelos, Bloom Filter e matcher de domínio.
- [infra/](../infra/) — Docker Compose, regras do Firestore e configs do Firebase.
- [docs/](.) — Documentação humana (esta pasta).

Configuração do monorepo:
- [Cargo.toml](../Cargo.toml) e [pnpm-workspace.yaml](../pnpm-workspace.yaml) — declaram quais crates Rust e quais pacotes pnpm fazem parte do workspace, permitindo `cargo build` ou `pnpm install` em qualquer pasta.
- [package.json](../package.json) — scripts globais (`tauri:dev`, `tauri:build`).
- [.env.example](../.env.example) e [backend/.env.example](../backend/.env.example) — modelo das variáveis que cada subprojeto espera.
- [.gitignore](../.gitignore) — ignora targets de build, node_modules, `.env` real e arquivos `.db` locais.

---

## Marco 2 — SQLCipher entra em cena (`f7143e3`)

A virada conceitual deste commit: **dados sensíveis nunca podem estar em texto plano** — nem no backend, nem no cache local do desktop, nem no banco do mobile. Em vez de SQLite puro, todas as três camadas passam a usar [SQLCipher](CONCEPTS.md#7-sqlcipher), que cifra o arquivo `.db` inteiro com AES-256.

Arquivos tocados:

- [backend/src/config.rs](../backend/src/config.rs) — passa a ler a env var `SQLCIPHER_KEY`. Sem essa chave, qualquer query no backend falha.
- [desktop/src-tauri/src/db.rs](../desktop/src-tauri/src/db.rs) — incorpora a chave de criptografia local e o ritual obrigatório do `PRAGMA key`.
- [mobile/lib/core/database_service.dart](../mobile/lib/core/database_service.dart) — usa `sqflite_sqlcipher` (drop-in replacement do `sqflite` que aceita uma `password`).
- [docs/CONCEPTS.md](CONCEPTS.md), [docs/ARCHITECTURE.md](ARCHITECTURE.md), [docs/DEVELOPMENT_GUIDE.md](DEVELOPMENT_GUIDE.md) — atualizados para registrar a decisão.
- [infra/compose.yml](../infra/compose.yml) e o [Dockerfile](../backend/Dockerfile) do backend — passam a injetar `SQLCIPHER_KEY` no container.

> **Detalhe técnico que não dá pra esconder:** `PRAGMA key` precisa ser o **primeiríssimo** comando após abrir a conexão. Qualquer outra query antes — mesmo um inocente `SELECT 1` — faz o SQLCipher tratar o arquivo como texto plano e quebrar tudo.

---

## Marco 3 — Modelos compartilhados e schema inicial (`cbabb15`)

Define o **contrato de dados** que vai vigorar em todo o projeto, em um único lugar.

- [shared/src/lib.rs](../shared/src/lib.rs) — apenas declara os módulos públicos da crate (`models`, `bloom_filter`, `domain_matcher`). É a vitrine da biblioteca compartilhada.
- [shared/src/models.rs](../shared/src/models.rs) — structs `User`, `Device`, `BlockedItem`, `ParentalLink`, `AdultFilterSettings`, `DeviceToken` mais os enums `BlockMode`, `Platform`, `BlockedType`, `LinkStatus`. Vivem aqui porque backend e desktop precisam serializar/deserializar exatamente os mesmos campos. Renomear um campo em um lugar e esquecer no outro quebraria a sincronização — esse risco fica zero quando só existe um arquivo.
- [backend/migrations/001_initial.sql](../backend/migrations/001_initial.sql) — cria as cinco tabelas básicas (`users`, `devices`, `blocked_items`, `parental_links`, `adult_filter_settings`). Cada coluna tem `CHECK`s e `UNIQUE`s deliberadamente restritivos. O `UNIQUE(user_id, item_type, value)` em `blocked_items`, por exemplo, garante que um mesmo usuário nunca consiga adicionar `instagram.com` duas vezes.

Motivo de modelar um `BlockMode` separado da escolha "Pessoal/Pais/Filhos" da tela inicial: o fluxo "Filhos" **não cria conta**. O device do filho é registrado sob a `User` do pai (cujo `mode` é `Parental`) com `Device.is_child = true`. Existem dois modos no banco, três opções na UI.

---

## Marco 4 — Bloom Filter (`05b9687`)

- [shared/src/bloom_filter.rs](../shared/src/bloom_filter.rs) — implementação de [Bloom Filter](CONCEPTS.md#1-bloom-filter) sem dependências externas. Resolve um problema de escala: o filtro de conteúdo adulto precisa checar, para cada consulta DNS, se o domínio está em uma lista pública de mais de 100 mil entradas. Carregar um `HashSet` na memória custaria dezenas de MB; consultar um SQLite em disco mata a latência. O Bloom Filter cabe em ~2 MB, responde em nanossegundos, e tem uma propriedade-chave: pode dar falso positivo (~0,1%), **nunca** dá falso negativo. Ou seja, um site adulto jamais escapa; um site legítimo bloqueado por engano é raríssimo e o usuário pode adicionar exceção.

Internamente, o arquivo usa duas hashes (FNV-1a + uma variante com sufixo), combina-as via *double hashing* para gerar `k` posições, e implementa `Serialize`/`Deserialize` de forma que o filtro já populado possa ser persistido em disco com `bincode` — evita reprocessar 100k linhas a cada boot.

---

## Marco 5 — Domain matcher e ajustes parentais (`11ab985`)

- [shared/src/domain_matcher.rs](../shared/src/domain_matcher.rs) — três funções que padronizam como domínios são comparados em todo o sistema: `normalize_domain` (`https://www.YouTube.com/feed` → `youtube.com`), `extract_domain` e `is_domain_blocked`. Sem esse arquivo, `Instagram.com`, `instagram.com/` e `https://www.instagram.com` seriam tratados como domínios diferentes. A lógica de subdomínio é deliberadamente conservadora: `youtube.com` na blocklist bloqueia `m.youtube.com`, mas **não** `notyoutube.com` (o ponto separador é exigido — há teste explícito para esse bug clássico).
- [backend/migrations/002_parental_fixes.sql](../backend/migrations/002_parental_fixes.sql) — adiciona dois índices à tabela `parental_links` e cria a tabela **`device_tokens`**. O índice unique parcial `WHERE status = 'pending'` impede que dois pais tenham simultaneamente o mesmo código de 6 dígitos (com 10⁶ combinações, colisão é rara mas possível, e mandaria o filho para o pai errado). A tabela `device_tokens` é o que viabiliza o fluxo "Filhos sem conta": o filho não tem Firebase, então o backend gera um token aleatório, guarda apenas o **SHA-256** dele (nunca o texto puro), e devolve o token plain ao app uma única vez.

---

## Marco 6 — Dependências do backend (`70449e8`)

- [backend/Cargo.toml](../backend/Cargo.toml) — declara o que o backend precisa para sair do esqueleto:
  - `axum` + `tokio` — servidor HTTP assíncrono.
  - `rusqlite` com a feature `bundled-sqlcipher` — compila o SQLCipher dentro do binário, sem depender de instalação no sistema.
  - `tokio-rusqlite` — wrapper async para o `rusqlite`, que é síncrono por natureza.
  - `jsonwebtoken` + `reqwest` — validação dos tokens do Firebase (JWT assinado pelo Google) e download das chaves públicas.
  - `tracing` + `tracing-subscriber` — logs estruturados.
  - `dotenvy`, `serde`, `chrono`, `uuid`, `sha2`, `rand` — utilitários de configuração, JSON, datas, IDs e hashing.

Esse commit não escreve lógica nova — só sinaliza ao Cargo o que o próximo commit vai usar.

---

## Marco 7 — Estruturação do backend (`fe46912`)

Aqui o backend ganha **arquitetura**. Três camadas bem separadas: rotas (HTTP), serviços (regra de negócio), config/erros (infraestrutura).

### Infraestrutura

- [backend/src/config.rs](../backend/src/config.rs) — `AppConfig` com porta, caminho do `.db`, chave do SQLCipher, project-id do Firebase, secrets de email. **Nenhum outro módulo lê env vars diretamente** — passa tudo por aqui. Defaults só existem em dev (em prod, `SQLCIPHER_KEY` ausente é falha de segurança).
- [backend/src/errors.rs](../backend/src/errors.rs) — `AppError` único: `BadRequest`, `Unauthorized`, `Forbidden`, `NotFound`, `Conflict`, `InternalServerError`. Implementa `IntoResponse` do Axum para que cada handler retorne `Result<Json<T>, AppError>` e o framework converte automaticamente em `{"error": "..."}` com o status correto.
- [backend/src/models.rs](../backend/src/models.rs) — re-exporta tudo do `shared::models` e adiciona os **DTOs** específicos da API REST (`RegisterRequest`, `CreateBlockedItemRequest`, `ConfirmLinkRequest`, etc.). A separação entre model e DTO é o que impede bugs como "o frontend mandou um `user_id` falsificado e o backend confiou": o DTO simplesmente não tem `user_id` — esse campo vem do token validado, não do body.
- [backend/src/main.rs](../backend/src/main.rs) — boot do servidor em seis passos: logs, config, conexão SQLCipher, montagem do `AppState`, composição do router (público + protegido) e `axum::serve`. A separação entre `public_routes` e `protected_routes` é o que permite `/devices/link/confirm` ser anônima — o filho ainda não tem credencial quando confirma o código pela primeira vez.

### O guardião

- [backend/src/middleware.rs](../backend/src/middleware.rs) — autenticação dual. Inspeciona o header `Authorization: Bearer <algo>` e decide:
  - Se começa com `dt_` → trata como Device Token (filho): remove o prefixo, calcula SHA-256, busca em `device_tokens` com `revoked_at IS NULL`.
  - Caso contrário → trata como Firebase JWT: baixa as chaves públicas do Google (cacheadas por 6h via `JwksCache`), valida assinatura RS256, checa `iss`/`aud`/`exp`, extrai o `firebase_uid` e resolve o `user_id` local.
  
  Em ambos os caminhos produz um `AuthUser { user_id, source, device_id }` injetado no request. **Regra crítica:** se `source == DeviceToken` e o método é POST/DELETE/PUT, retorna 403 antes mesmo de chegar no handler. Isso garante que tokens de filho são sempre read-only, mesmo que algum handler novo esqueça de checar.

### Rotas (portas de entrada da API)

- [backend/src/routes/mod.rs](../backend/src/routes/mod.rs) — apenas declara os submódulos.
- [backend/src/routes/auth.rs](../backend/src/routes/auth.rs) — `POST /auth/register`, `POST /auth/login`, `GET /auth/me`. Os dois primeiros **validam o JWT manualmente** (não passam pelo middleware global) porque o `firebase_uid` precisa ser extraído das claims antes de o usuário existir no banco.
- [backend/src/routes/blocklist.rs](../backend/src/routes/blocklist.rs) — `GET/POST/DELETE /blocklist` e `PUT /blocklist/adult-filter`. O `user_id` sai do token, nunca do body.
- [backend/src/routes/devices.rs](../backend/src/routes/devices.rs) — `POST /devices/register`, `GET /devices`, `POST /devices/link/generate` (Firebase JWT obrigatório — filho não pode gerar código), `POST /devices/link/confirm` (público).

### Serviços (onde mora a regra de negócio)

- [backend/src/services/mod.rs](../backend/src/services/mod.rs) — declara os submódulos. Comentário central: handlers fazem parsing/extract/auth, **services** falam com o banco e aplicam regras. Se trocássemos Axum por outro framework, services não mudariam.
- [backend/src/services/user_service.rs](../backend/src/services/user_service.rs) — `create_user`, `get_user_by_firebase_uid`, `get_user_by_id`. Cada inserção gera UUID v4 para o `id`.
- [backend/src/services/blocklist_service.rs](../backend/src/services/blocklist_service.rs) — CRUD da blocklist + toggle do filtro adulto. Toda query filtra por `user_id` (defesa em profundidade: mesmo que alguém descubra um id, não acessa dados de outro). `INSERT OR REPLACE` no `adult_filter_settings` evita lógica de "existe ou não existe".
- [backend/src/services/device_service.rs](../backend/src/services/device_service.rs) — registrar device, gerar código de 6 dígitos com TTL de 5 minutos, e a função-chave `confirm_link_code`: valida o código, cria o device do filho com `is_child = 1`, marca o `parental_link` como `active`, gera um token aleatório e salva apenas o SHA-256 em `device_tokens`. Esse fluxo todo numa única transação.
- [backend/src/services/auth_service.rs](../backend/src/services/auth_service.rs) — placeholder. A autenticação real mora no middleware; este arquivo só existe para crescer com utilitários quando o sistema de email-verification chegar (Marco 14).

---

## Marco 8 — Refatoração: nasce o `db.rs` (`e11c7ff`)

- [backend/src/db.rs](../backend/src/db.rs) — extração do código de conexão SQLCipher e migrations do `main.rs`. Antes era tudo inline; agora o `main` apenas chama `db::connect(...)` e `db::run_migrations(...)`. O ganho real é o controle de migrations: existe uma tabela `_migrations` que registra quais já rodaram, e o servidor pode chamar `run_migrations` em todo boot sem medo (idempotente). Migrations são incorporadas ao binário via `include_str!`, então o executável é auto-contido.

---

## Marco 9 — README do backend (`8d53557`)

- [backend/README.md](../backend/README.md) — instruções específicas de como rodar o backend isolado (sem o desktop e o mobile). Documenta o ritual de instalação do OpenSSL via vcpkg que o `bundled-sqlcipher` exige no Windows, e os comandos `cargo run` / `cargo test` para o ciclo de desenvolvimento local.

> **Commits 8–12 do log original (`abe3a93`, `51c2157`, `0a44ec5`)** são pequenas iterações em cima do mesmo conjunto de arquivos: comentários, ajuste fino de validações e atualização do README do OpenSSL. Não introduzem arquivos novos.

---

## Marco 10 — Cache local criptografado do desktop (`3e80004`)

O backend está pronto. Agora o app desktop precisa de seu próprio armazenamento local — para funcionar offline e dar resposta instantânea ao usuário sem ida-e-volta ao servidor.

- [desktop/src-tauri/migrations/001_local_cache.sql](../desktop/src-tauri/migrations/001_local_cache.sql) — duas tabelas. `blocked_items_cache` espelha o que o servidor tem (com um `synced_at` para saber quando foi a última sincronização). `blocking_state` é um simples key-value para guardar entre boots: se o engine estava ligado, qual era o DNS original do sistema, qual usuário estava logado por último, etc. Esse key-value é o que permite o **crash recovery** (ver Marco 12).
- [desktop/src-tauri/src/db.rs](../desktop/src-tauri/src/db.rs) — abre o SQLCipher local em `%APPDATA%\com.dopablocker\dopablocker-local.db`. Detalhe importante: a chave AES desse banco **não** é embarcada no binário (qualquer pessoa com o `.exe` extrairia). Em vez disso, na primeira execução o app gera 32 bytes aleatórios e salva no **Windows Credential Manager** via crate `keyring`. Em boots seguintes, lê de lá. Se o usuário apagar a credencial, o `.db` vira lixo (esperado — não queremos recuperação sem a chave).

---

## Marco 11 — Comandos IPC do Tauri (`a17f1c2`)

Tauri funciona com dois processos: o frontend Svelte rodando no WebView e o backend Rust nativo. A única forma de eles se comunicarem é por **IPC** — funções tipadas que o JS chama via `invoke()` e o Rust executa.

- [desktop/src-tauri/src/commands.rs](../desktop/src-tauri/src/commands.rs) — todos os handlers `#[tauri::command]`. Cada função recebe `State<Connection>` (banco local) e/ou `State<Engine>` injetado pelo Tauri. Os comandos cobrem: ler a versão do app, listar/salvar/adicionar/remover itens do cache, ligar/desligar o engine, ligar/desligar o filtro adulto, instalar a CA local. Comentário central do arquivo: a fonte-da-verdade é o backend; este cache é só espelho. Sempre que o cache muda e o engine está ativo, o engine recebe `update_rules` para não ficar com regra obsoleta.
- [desktop/src-tauri/src/lib.rs](../desktop/src-tauri/src/lib.rs) — o `setup` do Tauri. Antes de a janela abrir: inicializa o DB, cria o `AdultFilter` (o build do Bloom roda em background, não bloqueia a UI), constrói o `Engine` parado e dispara, também em background, o **resume**: se o último estado salvo dizia "bloqueio estava ativo", restaura DNS órfão de um possível crash anterior, religa o engine e reaplica o DNS do sistema. Tudo registrado como `State` para os comandos acessarem.

---

## Marco 12 — Frontend SvelteKit + DNS engine inicial (`7294f24`)

O maior commit do projeto até aqui. Três blocos de coisa nova entram juntos: o miolo do engine de bloqueio (camadas DNS), o frontend Svelte completo, e o login Firebase no desktop.

### 12a — Núcleo do engine de bloqueio

- [desktop/src-tauri/src/blocking/mod.rs](../desktop/src-tauri/src/blocking/mod.rs) — declaração dos submódulos.
- [desktop/src-tauri/src/blocking/dns_cache.rs](../desktop/src-tauri/src/blocking/dns_cache.rs) — cache em memória de respostas DNS recentes, respeitando o TTL retornado pelo upstream. Evita perguntar duas vezes ao Cloudflare nos próximos segundos sobre o mesmo `google.com`.
- [desktop/src-tauri/src/blocking/dns_upstream.rs](../desktop/src-tauri/src/blocking/dns_upstream.rs) — pool de servidores DNS "reais" (Cloudflare 1.1.1.1, Google 8.8.8.8) que recebem o que não está bloqueado. Cuida de timeout, fallback e balanceamento.
- [desktop/src-tauri/src/blocking/dns_proxy.rs](../desktop/src-tauri/src/blocking/dns_proxy.rs) — o servidor DNS local de verdade (porta 53, IPv4 + IPv6, UDP + TCP). Para cada query: parseia o domínio, normaliza com o `domain_matcher`, checa contra a blocklist do usuário e contra o filtro adulto. Se bloqueado, devolve `127.0.0.1` (o tráfego cai na página de bloqueio do próprio app, ver Marco 13). Se permitido, encaminha ao upstream e cacheia a resposta. Veja [docs/CONCEPTS.md → DNS Proxy](CONCEPTS.md#2-dns-proxy).
- [desktop/src-tauri/src/blocking/system_dns.rs](../desktop/src-tauri/src/blocking/system_dns.rs) — usa `netsh` para apontar o DNS de cada interface de rede do Windows para `127.0.0.1` (IPv4 + IPv6). Antes de mexer, salva a configuração original em `blocking_state`. Sem essa snapshot, um crash deixaria o usuário sem internet — daí o resume do Marco 11.
- [desktop/src-tauri/src/blocking/engine.rs](../desktop/src-tauri/src/blocking/engine.rs) — orquestrador. `start()` sobe DNS proxy, eventualmente WFP e a página de bloqueio; `stop()` desfaz tudo na ordem inversa; `update_rules()` propaga mudanças em quente sem precisar reiniciar nada.

### 12b — Frontend SvelteKit

Camada de **serviços** (sem UI):

- [desktop/src/lib/types.ts](../desktop/src/lib/types.ts) — espelho TypeScript dos modelos Rust. Manter manualmente em sincronia com [shared/src/models.rs](../shared/src/models.rs).
- [desktop/src/lib/services/firebase.ts](../desktop/src/lib/services/firebase.ts) — wrapper do Firebase Auth SDK. Login email/senha, login Google (popup), logout, `getIdToken()`, `onAuthChange()`. Leia [docs/CONCEPTS.md → Firebase JWT](CONCEPTS.md#4-firebase-jwt) para o porquê.
- [desktop/src/lib/services/api.ts](../desktop/src/lib/services/api.ts) — cliente HTTP do backend Axum. Injeta o JWT em `Authorization`, parseia erros no formato `{"error":"..."}`, e tem retry automático em 401 (caso o token tenha expirado, faz refresh e tenta de novo).
- [desktop/src/lib/services/tauri-bridge.ts](../desktop/src/lib/services/tauri-bridge.ts) — funções tipadas que chamam `invoke()`. O frontend **nunca** chama `invoke` diretamente; passa por aqui. Centraliza tipagem e nome dos comandos.

Camada de **estado reativo** (Svelte stores):

- [desktop/src/lib/stores/auth.ts](../desktop/src/lib/stores/auth.ts) — máquina de estados de autenticação. Fases: `booting`, `signed_out`, `authenticating`, `pending_local_registration` (logou no Firebase mas ainda não tem conta no backend), `backend_unavailable`, `authenticated`. A UI escuta esse store para decidir qual tela mostrar.
- [desktop/src/lib/stores/blocking.ts](../desktop/src/lib/stores/blocking.ts) — itens da blocklist + estado do engine. Backend é a fonte-da-verdade; o cache local Tauri recebe um espelho. Atualizações são otimistas: o store muda na hora e reverte se o backend recusar.

Camada de **componentes** (Svelte/Tailwind):

- [desktop/src/lib/components/ModeSelector.svelte](../desktop/src/lib/components/ModeSelector.svelte) — três cards (Pessoal, Pais, Filhos) da tela inicial.
- [desktop/src/lib/components/LoginForm.svelte](../desktop/src/lib/components/LoginForm.svelte) — abas "Entrar" e "Cadastrar", campos de email/senha/nome, botão Google.
- [desktop/src/lib/components/BlockList.svelte](../desktop/src/lib/components/BlockList.svelte) — itera sobre `blocking.items` e renderiza cada bloqueio com badge de tipo.
- [desktop/src/lib/components/AddBlockModal.svelte](../desktop/src/lib/components/AddBlockModal.svelte) — modal com abas Site / App / Palavra-chave para adicionar um item novo. Normaliza domínios antes de enviar.
- [desktop/src/lib/components/DeviceLinkCode.svelte](../desktop/src/lib/components/DeviceLinkCode.svelte) — botão "Gerar código" + display do código de 6 dígitos com countdown de 5 minutos.
- [desktop/src/lib/components/ParentalDashboard.svelte](../desktop/src/lib/components/ParentalDashboard.svelte) — lista de filhos vinculados, com botão para revogar.
- [desktop/src/lib/components/ui/Modal.svelte](../desktop/src/lib/components/ui/Modal.svelte) — `dialog` reusável (Esc fecha, click fora fecha).

Camada de **rotas** (SvelteKit):

- [desktop/src/routes/+layout.svelte](../desktop/src/routes/+layout.svelte) — layout raiz com sidebar e estado global de auth.
- [desktop/src/routes/+layout.ts](../desktop/src/routes/+layout.ts) — config de pré-render/SSR (este app é puramente SPA dentro do Tauri).
- [desktop/src/routes/+page.svelte](../desktop/src/routes/+page.svelte) — dashboard pós-login (saudação dinâmica, status do engine, métricas).
- [desktop/src/routes/login/+page.svelte](../desktop/src/routes/login/+page.svelte) — entry point do fluxo de auth.
- [desktop/src/routes/blocking/+page.svelte](../desktop/src/routes/blocking/+page.svelte) — tela com a `BlockList`, toggle de bloqueio mestre e toggle do filtro adulto.
- [desktop/src/routes/parental/+page.svelte](../desktop/src/routes/parental/+page.svelte) — tela exclusiva do modo Pais (gerar código + dashboard de filhos).
- [desktop/src/routes/settings/+page.svelte](../desktop/src/routes/settings/+page.svelte) — info da conta, logout, gerenciamento de dispositivos.

### 12c — Configuração da casca

- [desktop/src-tauri/Cargo.toml](../desktop/src-tauri/Cargo.toml), [desktop/src-tauri/tauri.conf.json](../desktop/src-tauri/tauri.conf.json), [desktop/package.json](../desktop/package.json) — declaram dependências (Tauri, hickory-dns, rcgen, rustls, windows-rs) e configurações da janela (tamanho, ícones, identificador). O `tauri.conf.json` em particular controla o que vira `.msi` no build de produção.

---

## Marco 13 — DNS Proxy + WFP completos + página HTTPS (`2b5bed2`)

O DNS proxy do Marco 12 já bloqueia, mas com dois buracos: (1) navegadores podem usar DNS-over-HTTPS direto (DoH), pulando o proxy; e (2) quando um site bloqueado é HTTPS, o navegador mostra um erro feio em vez de uma página explicativa. Este commit fecha os dois.

### Anti-bypass

- [desktop/src-tauri/src/blocking/wfp.rs](../desktop/src-tauri/src/blocking/wfp.rs) — integração com a [Windows Filtering Platform](CONCEPTS.md#3-windows-filtering-platform-wfp). Instala filtros kernel-level que bloqueiam: DNS plain (porta 53) que não vá para `127.0.0.1`, DNS-over-TLS (porta 853), e conexões HTTPS (porta 443) para os IPs conhecidos dos resolvers DoH (Cloudflare, Google, Quad9, AdGuard, CleanBrowsing). Os filtros são "dinâmicos" — o Windows derruba todos sozinho se o processo morrer, então não há lixo persistente para limpar.

### Página de bloqueio em HTTPS

Quando o DNS proxy responde `127.0.0.1` para `instagram.com`, o navegador conecta no próprio app — que precisa servir HTTPS válido para não disparar erro de certificado.

- [desktop/src-tauri/src/blocking/ca.rs](../desktop/src-tauri/src/blocking/ca.rs) — gera uma CA local auto-assinada uma única vez, salva em `app_data_dir`, e instala no Windows Root Store via `certutil -addstore Root`. Mesma técnica do Fiddler/Charles. Depois disso, qualquer cert assinado por essa CA é confiado pelo Chrome/Edge.
- [desktop/src-tauri/src/blocking/tls_resolver.rs](../desktop/src-tauri/src/blocking/tls_resolver.rs) — handshake TLS interceptado: lê o SNI (nome do host que o navegador pediu), gera um cert leaf assinado pela CA local **on-demand** para aquele hostname, e devolve. Cacheia certs já gerados para não pagar a CPU em cada requisição.
- [desktop/src-tauri/src/blocking/block_page.rs](../desktop/src-tauri/src/blocking/block_page.rs) e `block_page.html` — servidor HTTP/HTTPS local que renderiza a página "este site está bloqueado", com o domínio que o usuário tentou acessar e a razão.
- [desktop/src-tauri/src/blocking/block_reason.rs](../desktop/src-tauri/src/blocking/block_reason.rs) — diferencia "está na sua lista de bloqueios" de "filtro de conteúdo adulto", para a mensagem da página de bloqueio ser específica.
- [desktop/src-tauri/src/blocking/adult_filter.rs](../desktop/src-tauri/src/blocking/adult_filter.rs) — baixa a lista pública (Steven Black `alternates/porn/hosts`), parseia o formato `0.0.0.0 dominio.com`, popula um `BloomFilter` do crate `shared` e persiste em `bincode` para boots futuros. Recarregamento a cada 7 dias. O build é assíncrono — enquanto não terminou, `contains()` devolve `false` e o DNS proxy se comporta como se o filtro estivesse desligado, em vez de travar consultas. Estado `enabled` é `AtomicBool` (sem lock) porque é consultado uma vez por pacote DNS.

### UX que apareceu junto

- [desktop/src/lib/components/OnboardingModal.svelte](../desktop/src/lib/components/OnboardingModal.svelte) — modal "Bem-vindo" exibido pós-cadastro, lembra que admin é necessário, que existem duas camadas, etc. Aparece uma vez por usuário (registrado em `localStorage`).
- [desktop/src/lib/components/ui/Toast.svelte](../desktop/src/lib/components/ui/Toast.svelte) e [desktop/src/lib/stores/toast.ts](../desktop/src/lib/stores/toast.ts) — sistema simples de notificações verdes/vermelhas no canto inferior direito.
- [desktop/src/lib/components/ui/ConfirmModal.svelte](../desktop/src/lib/components/ui/ConfirmModal.svelte) — diálogo de confirmação reusável (usado pelo logout, por exemplo).
- [desktop/scripts/dev-with-backend.mjs](../desktop/scripts/dev-with-backend.mjs) — script para `pnpm tauri:dev` que sobe o backend Rust em um child process antes da janela abrir, facilitando o ciclo dia-a-dia.

---

## Marco 14 — Verificação de email no cadastro (`18d5e42`)

Antes deste commit, qualquer email aceito pelo Firebase virava conta no backend. Risco: o cadastro Firebase não exige confirmação imediata, então o usuário podia digitar `naomeucorreio@example.com` e o backend criaria a conta antes de o domínio ser validado.

- [backend/migrations/003_email_verification.sql](../backend/migrations/003_email_verification.sql) — tabela `email_verifications` com `code_hash`, `token_hash`, `attempts`, `expires_at`, `last_sent_at`. Códigos e tokens são guardados como **HMAC-SHA256** (não plain text). Cada código vale 10 minutos, aceita no máximo 5 tentativas e respeita cooldown de 60s entre reenvios.
- Atualizações em [backend/src/services/auth_service.rs](../backend/src/services/auth_service.rs), [backend/src/routes/auth.rs](../backend/src/routes/auth.rs) e [backend/src/models.rs](../backend/src/models.rs) — dois novos endpoints públicos: `POST /auth/email-code/start` (envia o código) e `POST /auth/email-code/verify` (valida e devolve um `email_verification_token` opaco). O `POST /auth/register` agora exige esse token para concluir o cadastro com provider `password` (Google é exceção: o email já vem verificado pelo provider).
- Adições em [backend/src/config.rs](../backend/src/config.rs) — variáveis `EMAIL_DELIVERY_MODE` (`smtp` ou `log`), `EMAIL_CODE_SECRET`, e config SMTP (host, porta, user, senha, from). Em dev o modo `log` apenas imprime o código no terminal — útil para testar sem servidor SMTP.
- [desktop/src/lib/components/LoginForm.svelte](../desktop/src/lib/components/LoginForm.svelte) — fluxo de cadastro vira de 2 passos: o usuário preenche os dados, recebe o código por email, digita o código, e só então a conta Firebase é criada e o `POST /auth/register` é chamado com o token de verificação.

---

## Apêndice — Pastas que ainda não viraram código

Algumas pastas existem como esqueleto mas não têm implementação completa no estado atual do branch `main`:

- [mobile/](../mobile/) — o esqueleto Flutter (telas, providers Riverpod, channels Kotlin, AndroidManifest) foi gerado no Marco 1, mas a integração real (Firebase Android, VPN service funcional, SQLCipher Dart) ainda não foi concluída. Plano completo em [docs/DEVELOPMENT_GUIDE.md → Trilha Mobile](DEVELOPMENT_GUIDE.md) (Fases M1–M3).
- [infra/](../infra/) — [compose.yml](../infra/compose.yml) sobe o backend em container, [firebase.json](../infra/firebase.json) e [firestore.rules](../infra/firestore.rules) descrevem a config Firebase, mas o deploy de produção (Azure Container Apps, listeners real-time do Firestore) ficou para a Fase B6 do guia de desenvolvimento.

Para o escopo planejado dessas pastas, consulte:
- [docs/PROTOTYPE.md](PROTOTYPE.md) — o que entra no v0.1 e o que fica para depois.
- [docs/ARCHITECTURE.md](ARCHITECTURE.md) — fluxo de dados cross-platform.
- [docs/GAPS.md](GAPS.md) — débitos técnicos conhecidos.
