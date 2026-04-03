# DopaBlocker — Guia de Desenvolvimento do Protótipo

Este documento descreve como desenvolver o protótipo DopaBlocker do zero até o produto funcional. O desenvolvimento é dividido em 3 trilhas que podem ser executadas em paralelo, com pontos de convergência definidos.

---

## Visão Geral

O desenvolvimento se divide em 3 trilhas independentes que convergem no final:

```
TRILHA BASE (fazer primeiro — é pré-requisito das outras duas)
  Fase B1: Shared Library
  Fase B2: Backend Core
  Fase B3: Backend Auth
  Fase B4: Backend Blocklist + Devices
       |
       +------ a partir daqui, Desktop e Mobile podem ser feitos em paralelo ------+
       |                                                                            |
  TRILHA DESKTOP                                              TRILHA MOBILE
  Fase D1: Tauri Core (IPC + SQLCipher)                       Fase M1: Flutter Core (Firebase + API + Providers)
  Fase D2: Blocking Engine (DNS, WFP, Adult)                  Fase M2: Kotlin Nativo (VPN, Accessibility, Boot)
  Fase D3: Frontend SvelteKit (UI completa)                   Fase M3: UI Flutter (telas + widgets)
       |                                                                            |
       +------ ambas as trilhas precisam estar prontas para continuar ------+
       |
  TRILHA BASE (continuação)
  Fase B5: Integração Cross-Platform
  Fase B6: Docker, Deploy e Polimento Final
```

**Regra:** As Fases B1–B4 devem ser concluídas antes de iniciar qualquer trilha. Após B4, Desktop e Mobile podem ser desenvolvidos simultaneamente por pessoas ou sessões diferentes. B5 e B6 só começam quando ambas as trilhas estiverem concluídas.

---

# PARTE 1 — TRILHA BASE

Tudo que não é específico de desktop ou mobile: a crate compartilhada, o backend completo, a integração final e o deploy.

---

## Fase B1 — Shared Library (Crate Rust Compartilhada)

### Por que começar aqui

A crate `shared` define os modelos de dados que todas as outras partes do sistema usam. Backend, desktop e mobile dependem dessas structs para serializar/deserializar dados de forma consistente. Se cada parte do sistema definisse seus próprios modelos, qualquer mudança num campo (ex: renomear `is_active` para `active`) teria que ser replicada em 3 lugares — com risco de divergência. Centralizando aqui, uma única mudança se propaga para todo o projeto.

O Bloom Filter e o Domain Matcher também vivem aqui porque são usados tanto no desktop (dentro do DNS Proxy) quanto no backend (validação server-side), e precisam se comportar de forma idêntica em ambos.

### Arquivos

| Arquivo | O que implementar |
|---------|------------------|
| `shared/Cargo.toml` | Já configurado. Pode precisar adicionar dependência para o Bloom Filter |
| `shared/src/lib.rs` | Já declara os módulos. Nenhuma mudança necessária |
| `shared/src/models.rs` | Structs com derive Serialize/Deserialize |
| `shared/src/bloom_filter.rs` | Struct BloomFilter + métodos new/insert/contains |
| `shared/src/domain_matcher.rs` | Funções normalize_domain, extract_domain, is_domain_blocked |

### Detalhamento

**models.rs** — Criar estas structs, todas com `#[derive(Debug, Clone, Serialize, Deserialize)]`. Usar `String` para timestamps (ISO 8601) por simplicidade no protótipo — evita depender de crates de data/hora nesta crate compartilhada:

- `BlockMode` — Enum com variantes `Personal` e `Parental`. Determina como o app se comporta: no modo pessoal, o próprio usuário gerencia seus bloqueios; no modo parental, um dispositivo-pai controla a blocklist dos dispositivos-filhos.
- `User` — Campos: id, firebase_uid, email, display_name, mode (`BlockMode`), created_at. O `firebase_uid` é o identificador que vem do Firebase Auth e vincula o usuário local ao sistema de autenticação.
- `Device` — Campos: id, user_id, device_name, platform, is_child (bool), created_at. Cada instalação do app (desktop ou mobile) é um device. O campo `is_child` indica se este dispositivo é controlado por outro no modo parental.
- `BlockedItem` — Campos: id, user_id, item_type (String: "site" ou "app"), value, is_active (bool), created_at. Representa um domínio ou app que o usuário quer bloquear. O `item_type` diferencia bloqueio de site (via DNS) de bloqueio de app (via Accessibility Service no Android).
- `ParentalLink` — Campos: id, parent_device_id, child_device_id, link_code, status, created_at. Representa a vinculação entre um dispositivo-pai e um dispositivo-filho. O `link_code` é o código de 6 dígitos com validade de 5 minutos.
- `AdultFilterSettings` — Campos: id, user_id, is_enabled (bool), last_list_update. Controla se o filtro de conteúdo adulto (Bloom Filter) está ativo para aquele usuário e quando a lista foi atualizada pela última vez.

**bloom_filter.rs** — Implementar um Bloom Filter básico sem crates externas. A ideia é transformar uma lista enorme de domínios (milhões) numa estrutura compacta que responde "este domínio está na lista?" em tempo constante:

- `BloomFilter::new(expected_items, false_positive_rate)` — Recebe a quantidade esperada de itens e a taxa de falso positivo desejada (ex: 0.001 = 0.1%). Calcula o tamanho ideal do bit array com a fórmula `m = -(n * ln(p)) / (ln(2)^2)` e o número de funções hash com `k = (m/n) * ln(2)`. Inicializa um `Vec<bool>` com `m` posições zeradas. Esses cálculos garantem o equilíbrio entre uso de memória e precisão.
- `insert(&mut self, item: &str)` — Para inserir um domínio, aplica `k` funções hash diferentes sobre a string. Cada hash gera uma posição no bit array, e essa posição é marcada como `true`. A técnica de double hashing (duas sementes diferentes combinadas) gera as `k` posições sem precisar de `k` funções hash independentes.
- `contains(&self, item: &str) -> bool` — Aplica as mesmas `k` funções hash e verifica se **todas** as posições correspondentes estão marcadas. Se alguma estiver em `false`, o item com certeza não está na lista. Se todas estiverem em `true`, o item **provavelmente** está (pode ser falso positivo, mas a taxa é controlada pelo parâmetro `false_positive_rate`).

**domain_matcher.rs** — Três funções puras que padronizam como domínios são comparados em todo o sistema. Sem essa normalização, `Instagram.com`, `https://www.instagram.com/reels`, e `instagram.com` seriam tratados como domínios diferentes:

- `normalize_domain(url)` — Remove protocolo (`http://`, `https://`), prefixo `www.`, trailing `/`, e converte tudo para lowercase. O resultado é sempre o domínio puro (ex: `instagram.com`).
- `extract_domain(url)` — Extrai apenas o domínio de uma URL completa, ignorando path e query string. Ex: `https://youtube.com/watch?v=abc` → `youtube.com`.
- `is_domain_blocked(domain, blocklist)` — Verifica se o domínio normalizado está na lista. Deve verificar subdomínios também: se `youtube.com` está na blocklist, então `m.youtube.com`, `music.youtube.com` e qualquer outro subdomínio deve ser bloqueado. A lógica é: extrair o domínio-raiz e verificar se algum sufixo do domínio completo está na lista.

### Como testar

Escrever testes unitários com `cargo test`. Testar: normalização de URLs com maiúsculas/minúsculas/trailing slashes, Bloom Filter com inserções e consultas (positivas e negativas), subdomínio matching (garantir que `m.youtube.com` é bloqueado quando `youtube.com` está na lista, mas `notyoutube.com` não é).

### Critério de conclusão

- `cargo test` passa com todos os testes
- `cargo check` sem warnings
- Modelos se serializam/deserializam corretamente para JSON

---

## Fase B2 — Backend Core (Servidor Básico + Database)

### Por que esta fase

Antes de implementar qualquer endpoint, o servidor precisa de uma base funcional: carregar configuração do ambiente, ter um sistema de erros padronizado que retorna JSON consistente, conectar ao SQLCipher (banco criptografado), e subir respondendo requests. Sem essa fundação, cada endpoint teria que resolver esses problemas individualmente.

### Dependências a adicionar no backend/Cargo.toml

```toml
rusqlite = { version = "0.32", features = ["bundled-sqlcipher"] }
tokio-rusqlite = "0.6"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
```

> **Por que rusqlite + bundled-sqlcipher em vez de sqlx?** O sqlx não tem suporte nativo a SQLCipher. A feature `bundled-sqlcipher` compila o SQLCipher junto com o binário — sem precisar instalar nada no sistema. O `tokio-rusqlite` fornece o wrapper async necessário para usar com Axum/Tokio.

### Arquivos

| Arquivo | O que implementar |
|---------|------------------|
| `backend/src/config.rs` | Struct AppConfig com port, database_url, firebase_project_id, sqlcipher_key. Função load() que usa dotenvy + std::env com valores default |
| `backend/src/errors.rs` | Enum AppError (Unauthorized, NotFound, BadRequest, Internal). Implementar IntoResponse retornando status code + JSON |
| `backend/src/models.rs` | Re-exportar tudo do shared. Adicionar DTOs: CreateUserRequest, LoginRequest, LoginResponse, AddBlockedItemRequest, GenerateLinkResponse, ConfirmLinkRequest |
| `backend/src/main.rs` | #[tokio::main] async fn main. Carregar config, inicializar tracing, conectar ao SQLCipher (abrir + PRAGMA key), rodar migrations, montar router, aplicar CORS, bind na porta |
| `backend/migrations/001_initial.sql` | CREATE TABLE real para: users, devices, blocked_items, parental_links, adult_filter_settings |

### Detalhamento

**config.rs** — Struct `AppConfig` com os campos: `port` (u16), `database_url` (String — caminho do arquivo .db), `firebase_project_id` (String), `firebase_api_key` (String), `sqlcipher_key` (String — chave de criptografia do banco). A função `load()` usa `dotenvy::dotenv().ok()` para carregar o `.env` (sem falhar se não existir) e depois lê cada variável com `std::env::var("NOME").unwrap_or("valor_default".into())`. A `sqlcipher_key` não deve ter valor default — se estiver vazia, o servidor deve recusar iniciar (panic com mensagem clara), porque rodar sem criptografia é uma falha de segurança.

**errors.rs** — Enum `AppError` com variantes: `Unauthorized(String)`, `NotFound(String)`, `BadRequest(String)`, `Internal(String)`. Implementar o trait `IntoResponse` do Axum para que cada variante retorne o status HTTP correto (401, 404, 400, 500) com corpo JSON no formato `{ "error": "mensagem" }`. Isso permite que qualquer handler retorne `Result<Json<T>, AppError>` e o Axum converte automaticamente os erros em respostas HTTP padronizadas.

**models.rs** — Importar e re-exportar todos os modelos do crate `shared` (para que o backend use `crate::models::User` em vez de `dopablocker_shared::models::User`). Além disso, definir DTOs (Data Transfer Objects) específicos do backend, que representam o que o frontend envia. A diferença entre um DTO e um model é: o model tem todos os campos (id, created_at, etc.), enquanto o DTO tem apenas os campos que o frontend precisa enviar. Exemplo: `CreateUserRequest` tem só email e display_name — o id é gerado pelo backend, o firebase_uid vem do JWT, e o created_at é preenchido automaticamente.

**migrations/001_initial.sql** — Tabelas com tipos SQLite (o arquivo SQL é o mesmo, SQLCipher é compatível com a sintaxe SQLite):

- `users`: id TEXT PK, firebase_uid TEXT UNIQUE, email TEXT UNIQUE, display_name TEXT, mode TEXT default 'personal', created_at TEXT default datetime('now')
- `devices`: id TEXT PK, user_id TEXT FK, device_name TEXT, platform TEXT, is_child INTEGER default 0, created_at TEXT
- `blocked_items`: id TEXT PK, user_id TEXT FK, item_type TEXT CHECK('site','app'), value TEXT, is_active INTEGER default 1, created_at TEXT
- `parental_links`: id TEXT PK, parent_device_id TEXT FK, child_device_id TEXT FK nullable, link_code TEXT, status TEXT default 'pending', expires_at TEXT, created_at TEXT
- `adult_filter_settings`: id TEXT PK, user_id TEXT UNIQUE FK, is_enabled INTEGER default 0, last_list_update TEXT

O `child_device_id` começa como NULL porque, quando o pai gera o código, ainda não se sabe qual dispositivo vai usar esse código. Ele só é preenchido quando o filho confirma a vinculação.

**main.rs** — O fluxo do main é sequencial e cada passo depende do anterior:

1. Carregar configuração com `dotenvy` + `AppConfig::load()`.
2. Inicializar `tracing_subscriber` para logs estruturados no terminal.
3. Abrir conexão SQLCipher com `tokio_rusqlite::Connection::open(&config.database_url)`.
4. Executar `PRAGMA key = '<sqlcipher_key>';` como **primeiro** comando na conexão — isso é obrigatório, sem essa etapa o banco não descriptografa e qualquer query subsequente falha.
5. Rodar os `CREATE TABLE IF NOT EXISTS` do migration inline (ler o arquivo .sql e executar).
6. Compartilhar a conexão como `Arc<tokio_rusqlite::Connection>` no state do Axum via `Extension`.
7. Montar o router chamando `routes::create_router(conn)`.
8. Envolver com `CorsLayer::permissive()` do tower-http (necessário para o frontend SvelteKit acessar a API durante desenvolvimento).
9. Fazer bind com `axum::serve` no `TcpListener` da porta configurada.

> **Nota sobre PRAGMA key:** O `PRAGMA key` DEVE ser o primeiro comando executado após abrir a conexão. Qualquer outro comando antes dele faz o SQLCipher tratar o banco como não-criptografado e falhar. A chave vem da variável de ambiente `SQLCIPHER_KEY`.

### Como testar

`cargo run` deve subir o servidor em localhost:3000. Um curl qualquer deve retornar 404 (nenhuma rota ainda), mas o servidor responde. O arquivo `.db` deve ser criado — e se você tentar abri-lo com um leitor SQLite comum, ele vai mostrar "database is encrypted" (isso confirma que o SQLCipher está funcionando).

### Critério de conclusão

- Servidor sobe sem erros, SQLCipher cria as tabelas, logs do tracing aparecem no terminal
- O arquivo .db NÃO é legível por ferramentas SQLite comuns (está criptografado)

---

## Fase B3 — Backend Auth (Autenticação com Firebase)

### Por que esta fase

Nenhum outro endpoint funciona sem autenticação. Todas as rotas de blocklist e devices precisam saber **qual** usuário está fazendo a requisição — sem isso, qualquer pessoa poderia ler ou modificar os dados de qualquer outra. O middleware de JWT precisa existir antes de proteger essas rotas.

### Dependências a adicionar

```toml
jsonwebtoken = "9"
reqwest = { version = "0.12", features = ["json"] }
```

### Arquivos

| Arquivo | O que implementar |
|---------|------------------|
| `backend/src/middleware.rs` | Extrair e validar Firebase JWT |
| `backend/src/services/auth_service.rs` | Criar usuário no DB, buscar usuário |
| `backend/src/routes/auth.rs` | Endpoints /auth/register, /auth/login, /auth/me |
| `backend/src/routes/mod.rs` | Montar router com rotas de auth |

### Detalhamento

**middleware.rs — Validação de Firebase JWT:**

O fluxo de autenticação funciona assim: o frontend (desktop ou mobile) faz login diretamente no Firebase Auth SDK — o backend **nunca** recebe a senha do usuário. O Firebase valida as credenciais e retorna um JWT (token assinado). O frontend envia esse JWT no header `Authorization: Bearer <token>` de toda requisição. O backend precisa verificar se esse token é legítimo e não foi adulterado.

Para validar o JWT:

1. Buscar as chaves públicas do Google em `https://www.googleapis.com/robot/v1/metadata/x509/securetoken@system.gserviceaccount.com`. Essas chaves mudam periodicamente, então precisam ser cacheadas e atualizadas.
2. Decodificar o header do JWT (a primeira parte antes do ponto) para extrair o `kid` (Key ID) — isso indica qual chave pública foi usada para assinar este token específico.
3. Encontrar a chave pública correspondente ao `kid` no cache.
4. Validar a assinatura digital com `jsonwebtoken::decode()` usando essa chave.
5. Verificar os claims: `iss` (issuer) deve ser `https://securetoken.google.com/<project_id>`, `aud` (audience) deve ser o project_id, `exp` (expiration) não deve estar expirado.

Se qualquer uma dessas verificações falhar, o token é inválido e a requisição deve retornar 401 Unauthorized.

Implementar como um Axum extractor: `AuthUser { uid: String, email: Option<String> }` que implementa o trait `FromRequestParts`. Quando um handler declara `AuthUser` como parâmetro, o Axum automaticamente executa a validação antes de chamar o handler. Se o token for inválido, o handler nem é executado.

Cache das chaves públicas: armazenar em `Arc<RwLock<HashMap<String, DecodingKey>>>` e atualizar quando expirar (o header `Cache-Control` da resposta do Google indica por quanto tempo a chave é válida, geralmente ~24h). Sem esse cache, cada requisição faria um HTTP request para o Google — inaceitável em termos de latência.

**services/auth_service.rs** — Funções que interagem com o banco para gerenciar usuários:

- `create_user(conn, firebase_uid, email, display_name)` — Insere um novo usuário no SQLCipher, gerando um UUID v4 para o campo `id`. O `firebase_uid` vem do JWT validado e vincula o registro local ao sistema de autenticação do Firebase.
- `get_user_by_firebase_uid(conn, firebase_uid)` — Busca um usuário pelo `firebase_uid`. Retorna `Option<User>` porque o usuário pode não existir localmente ainda (primeiro login).
- `get_or_create_user(conn, firebase_uid, email, display_name)` — Combina as duas funções acima: tenta buscar pelo `firebase_uid`; se não encontrar, cria. Isso é útil no endpoint de login, onde o backend precisa garantir que o usuário existe localmente sem exigir um passo de registro separado.

**routes/auth.rs** — Três endpoints de autenticação:

- `POST /auth/register` — Recebe `CreateUserRequest` (email, display_name), cria o usuário no banco e retorna o `User` completo. Este endpoint é chamado apenas uma vez, no primeiro cadastro.
- `POST /auth/login` — O login real acontece no frontend via Firebase SDK. Este endpoint serve para **sincronizar**: o frontend envia o JWT, o backend valida, e usa `get_or_create_user` para garantir que o usuário existe localmente. Retorna os dados do usuário. Isso é necessário porque o Firebase e o banco local são sistemas separados — o login no Firebase não cria automaticamente o registro no SQLCipher.
- `GET /auth/me` — Rota protegida com `AuthUser`. Simplesmente busca e retorna os dados do usuário autenticado. Usada pelo frontend para verificar se a sessão ainda é válida e carregar os dados do usuário no state.

### Como testar

1. Criar um usuário de teste no Firebase Console > Authentication > Users
2. Obter um token via Firebase Auth REST API: POST para `https://identitytoolkit.googleapis.com/v1/accounts:signInWithPassword?key=<API_KEY>` com email/password
3. Testar: `curl -H "Authorization: Bearer <token>" localhost:3000/auth/me` deve retornar os dados do usuário. Sem token deve retornar 401. Token expirado ou adulterado também deve retornar 401.

### Critério de conclusão

- Token Firebase válido retorna 200, sem token retorna 401, token inválido retorna 401
- Usuário criado no SQLCipher no primeiro login
- Cache de chaves públicas funciona (não faz request ao Google a cada chamada)

---

## Fase B4 — Backend Blocklist + Devices

### Por que esta fase

Com a autenticação funcionando, agora é possível implementar as rotas de negócio sabendo **quem** é o usuário de cada requisição. Após esta fase, o backend está completo e funcional — as trilhas Desktop e Mobile podem começar a consumi-lo como API.

### Dependência adicional

```toml
rand = "0.8"
```

### Arquivos

| Arquivo | O que implementar |
|---------|------------------|
| `backend/src/services/blocklist_service.rs` | CRUD de blocked_items + toggle adult filter |
| `backend/src/services/device_service.rs` | Registro de device, gerar/confirmar link code |
| `backend/src/routes/blocklist.rs` | 4 endpoints de blocklist |
| `backend/src/routes/devices.rs` | 4 endpoints de devices |
| `backend/src/routes/mod.rs` | Adicionar blocklist e devices ao router |

### Detalhamento

**services/blocklist_service.rs** — Funções de acesso ao banco para a blocklist. Toda query filtra por `user_id`, garantindo que um usuário nunca acessa os dados de outro:

- `get_user_blocklist(conn, user_id)` — `SELECT * FROM blocked_items WHERE user_id = ?`. Retorna todos os itens bloqueados do usuário, incluindo inativos (o frontend decide se mostra ou filtra).
- `add_blocked_item(conn, user_id, item_type, value)` — Gera um UUID v4 para o `id`, valida que `item_type` é "site" ou "app", normaliza o domínio (se for site) usando `domain_matcher::normalize_domain`, e faz o INSERT. Retorna o item criado com todos os campos preenchidos.
- `remove_blocked_item(conn, user_id, item_id)` — `DELETE FROM blocked_items WHERE id = ? AND user_id = ?`. A cláusula `AND user_id` é uma proteção: mesmo que alguém descubra o id de um item de outro usuário, o DELETE não vai funcionar porque o user_id não bate.
- `toggle_adult_filter(conn, user_id, enabled)` — `INSERT OR REPLACE INTO adult_filter_settings`. Usa INSERT OR REPLACE porque a tabela tem UNIQUE no user_id — se o registro já existe, atualiza; se não, cria. Isso evita a necessidade de verificar se o registro existe antes de decidir entre INSERT e UPDATE.

**services/device_service.rs** — Funções para gerenciar dispositivos e vinculação parental:

- `register_device(conn, user_id, device_name, platform)` — Cria um registro de device com UUID. Cada instalação do app chama essa função uma vez, na primeira inicialização. O `platform` é "windows" ou "android", usado para saber que tipo de bloqueio aplicar.
- `get_user_devices(conn, user_id)` — Lista todos os dispositivos do usuário. No modo parental, o dispositivo-pai usa essa lista para mostrar quais filhos estão vinculados.
- `generate_link_code(conn, device_id)` — Gera um código aleatório de 6 dígitos com `rand::thread_rng().gen_range(100000..=999999)`, salva em `parental_links` com status "pending" e `expires_at = now + 5 minutos`. O código de 6 dígitos é um compromisso entre segurança (1 milhão de combinações possíveis) e usabilidade (fácil de digitar). O TTL de 5 minutos limita a janela de ataque — após expirar, o código é inútil.
- `confirm_link_code(conn, child_device_id, code)` — O dispositivo-filho envia o código digitado pelo usuário. A função busca em `parental_links` um registro com esse código, verifica que o status é "pending" e que `expires_at > now` (não expirou). Se válido, atualiza o `child_device_id` e muda o status para "active". Se o código não existe, já foi usado, ou expirou, retorna erro.

**routes/blocklist.rs** — Todas as rotas são protegidas com o extractor `AuthUser`, ou seja, exigem JWT válido. O `user_id` vem do token, nunca do corpo da requisição (isso impede que um usuário se passe por outro):

- `GET /blocklist` — Lista todos os itens bloqueados do usuário autenticado.
- `POST /blocklist` — Adiciona um item. Recebe `AddBlockedItemRequest` com `item_type` e `value`.
- `DELETE /blocklist/:id` — Remove um item pelo `id`. O service verifica que o item pertence ao usuário.
- `PUT /blocklist/adult-filter` — Liga/desliga o filtro de conteúdo adulto. Recebe `{ "enabled": true/false }`.

**routes/devices.rs** — Também todas protegidas com `AuthUser`:

- `POST /devices/register` — Registra o dispositivo. Recebe `device_name` e `platform`.
- `GET /devices` — Lista os dispositivos do usuário (incluindo filhos vinculados).
- `POST /devices/link/generate` — O dispositivo-pai solicita um código de vinculação. Retorna o código e o tempo de expiração.
- `POST /devices/link/confirm` — O dispositivo-filho envia o código para confirmar a vinculação. Retorna sucesso ou erro.

### Como testar

Usar curl com token Firebase para testar cada endpoint em sequência: registrar device, adicionar site à blocklist, listar, gerar link code, confirmar link de outro device (usando outro token ou simulando).

### Critério de conclusão

- Todos os 8 endpoints retornam respostas corretas
- Dados persistem no SQLCipher
- Link code expira após 5 minutos
- Um usuário não consegue acessar dados de outro

### PONTO DE BIFURCAÇÃO

**A partir daqui, as trilhas Desktop e Mobile podem ser desenvolvidas em paralelo.** O backend está completo e ambas consomem a mesma API.

---

# PARTE 2 — TRILHA DESKTOP

Tudo específico do app Windows: Tauri backend (Rust), blocking engine, e frontend SvelteKit.

**Pré-requisito:** Fases B1–B4 concluídas.

---

## Fase D1 — Tauri Core (Comandos IPC + SQLCipher Local)

### Por que esta fase

O Tauri funciona com uma arquitetura de dois processos: o frontend (SvelteKit rodando no WebView) e o backend nativo (Rust). Eles não compartilham memória — toda comunicação acontece via IPC (Inter-Process Communication) através de comandos Tauri. Nesta fase, definimos essa ponte: o frontend chama uma função JavaScript, que o Tauri traduz para uma chamada Rust, que acessa o banco e retorna o resultado de volta ao frontend.

O SQLCipher local permite que o app funcione offline com dados criptografados no disco — mesmo que alguém copie o arquivo .db, não consegue ler o conteúdo.

### Dependências a adicionar no desktop/src-tauri/Cargo.toml

```toml
rusqlite = { version = "0.32", features = ["bundled-sqlcipher"] }
tokio-rusqlite = "0.6"
uuid = { version = "1", features = ["v4"] }
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
```

### Arquivos

| Arquivo | O que implementar |
|---------|------------------|
| `desktop/src-tauri/src/db.rs` | Inicializar SQLCipher local (open + PRAGMA key), CRUD operations |
| `desktop/src-tauri/src/commands.rs` | Todos os #[tauri::command] handlers |
| `desktop/src-tauri/src/lib.rs` | Registrar comandos no Tauri builder, inicializar DB |

### Detalhamento

**db.rs** — O SQLCipher local é um cache do estado do backend. Ele contém as mesmas tabelas (blocked_items, devices, adult_filter_settings) mas com escopo local — só os dados do usuário logado naquele dispositivo. Esse cache existe por dois motivos: (1) o bloqueio funciona offline (sem internet, o DNS Proxy continua consultando a blocklist local), e (2) a UI carrega instantaneamente dos dados locais, sem esperar resposta do backend.

A função `init_db` deve:
1. Obter o diretório de dados do app via `app_handle.path().app_data_dir()` — isso retorna um caminho como `C:\Users\<user>\AppData\Roaming\com.dopablocker\` onde o Tauri guarda dados persistentes.
2. Criar o diretório se não existir.
3. Abrir conexão com `tokio_rusqlite::Connection::open(path.join("dopablocker.db"))`.
4. Executar `PRAGMA key = '<key>';` como primeiro comando — obrigatório para descriptografar o banco.
5. Rodar `CREATE TABLE IF NOT EXISTS` inline para cada tabela.
6. Retornar a connection.

A chave de criptografia do desktop pode ser derivada de um segredo fixo embarcado no binário ou, idealmente, lida do Windows Credential Store (mais seguro, mas mais complexo — para o protótipo, um segredo fixo basta).

Funções CRUD idênticas ao backend, mas operando no banco local. Todas usam `conn.call(|conn| { ... })` para executar queries síncronas (rusqlite) dentro do wrapper async (tokio-rusqlite).

**commands.rs** — 8 comandos Tauri. Cada um é uma função `async` decorada com `#[tauri::command]` que recebe `State<Arc<tokio_rusqlite::Connection>>` como parâmetro. O Tauri injeta o state automaticamente:

1. `get_blocklist` — Busca todos os itens da blocklist no SQLCipher local e retorna como JSON para o frontend.
2. `add_blocked_item(item_type, value)` — Insere no SQLCipher local (efeito imediato na UI) e em paralelo envia para o backend via HTTP (fire-and-forget). Se o backend estiver offline, o dado fica salvo localmente e será sincronizado depois.
3. `remove_blocked_item(id)` — Remove do banco local e notifica o backend.
4. `toggle_blocking(enabled)` — Liga/desliga o blocking engine (DNS Proxy + WFP). Este comando não mexe no banco — ele controla o engine implementado na Fase D2.
5. `toggle_adult_filter(enabled)` — Atualiza a configuração local e envia para o backend.
6. `generate_link_code` — Chama o backend via `POST /devices/link/generate` e retorna o código para o frontend exibir.
7. `confirm_link_code(code)` — Chama o backend via `POST /devices/link/confirm` com o código digitado pelo usuário.
8. `get_linked_devices` — Chama o backend via `GET /devices` e retorna a lista.

Os comandos 6, 7 e 8 não usam o banco local — são operações de vinculação que só fazem sentido online, pois envolvem comunicação com outro dispositivo.

**lib.rs** — Atualizar o builder do Tauri: no `.setup()`, chamar `db::init_db()` (que abre a conexão SQLCipher e executa PRAGMA key) e armazenar o resultado com `app.manage(Arc::new(conn))`. No `.invoke_handler()`, registrar todos os 8 comandos com `tauri::generate_handler![]`. A ordem de inicialização importa: o DB precisa estar no state antes de qualquer comando ser invocado.

### Como testar

`pnpm tauri dev`, abrir o console F12, e testar com `window.__TAURI__.core.invoke('get_blocklist')` e outros comandos. Cada invoke deve retornar JSON válido ou erro tratado.

### Critério de conclusão

- Todos os 8 comandos registram sem erro
- `invoke()` do frontend retorna dados corretos
- SQLCipher local persiste dados criptografados entre restarts
- O arquivo .db não é legível com ferramentas SQLite comuns

---

## Fase D2 — Blocking Engine (DNS Proxy + WFP + Filtro Adulto)

### Por que esta fase

Esta é a funcionalidade central do DopaBlocker no desktop. Sem o blocking engine, o app é apenas uma lista de sites — não bloqueia nada de verdade. O engine é composto por três camadas que trabalham juntas: o Filtro Adulto (identifica domínios adultos), o DNS Proxy (intercepta e bloqueia resoluções DNS), e o WFP (garante que ninguém contorne o proxy).

### Ordem dentro da fase (do mais simples ao mais complexo)

1. adult_filter.rs — Carregar lista e popular Bloom Filter
2. dns_proxy.rs — DNS resolver local
3. engine.rs — Orquestrador
4. wfp.rs — Windows Filtering Platform (bônus, pode ficar por último)

### Dependências a adicionar

```toml
trust-dns-proto = "0.24"
windows = { version = "0.58", features = [
    "Win32_NetworkManagement_WindowsFilteringPlatform",
    "Win32_Foundation",
    "Win32_Security",
] }
```

### Detalhamento

**blocking/adult_filter.rs** — Responsável por carregar listas públicas de domínios adultos e disponibilizá-las para consulta rápida via Bloom Filter.

Struct `AdultFilter` com um `BloomFilter` do crate shared e flag `is_loaded`. Funções:

- `new()` — Cria a struct com Bloom Filter vazio e `is_loaded = false`.
- `load_from_url(url)` — Baixa uma lista remota de domínios adultos. A lista Steven Black (disponível como raw no GitHub) tem o formato `0.0.0.0 domínio.com` em cada linha. A função deve: fazer GET na URL, iterar sobre as linhas, ignorar linhas que começam com `#` (comentários) e linhas vazias, extrair a segunda coluna (o domínio), e inserir no Bloom Filter. Para evitar baixar a lista toda vez que o app abre, salvar o conteúdo em cache local (arquivo no diretório de dados do app). Nas execuções seguintes, carregar do cache. Atualizar a cada 7 dias comparando a data do cache com a data atual.
- `load_from_file(path)` — Lê um arquivo local de domínios (um por linha). Útil para testes e para carregar o cache salvo.
- `is_adult(domain) -> bool` — Consulta o Bloom Filter. Retorna `true` se o domínio provavelmente está na lista. Lembrar que falsos positivos são possíveis (~0.1%), então o usuário pode adicionar exceções na blocklist.

**blocking/dns_proxy.rs** — Mini servidor DNS local que é o coração do bloqueio. Todo navegador e app no computador faz consultas DNS. Se o DopaBlocker controla o DNS, ele controla quais sites podem ser acessados.

Struct `DnsProxy` com: `blocklist` (`Arc<RwLock<HashSet<String>>>` — lista de domínios bloqueados pelo usuário), `adult_filter` (`Arc<AdultFilter>` — filtro de conteúdo adulto), `upstream_dns` (endereço do DNS real, ex: 8.8.8.8:53), `is_running` (`Arc<AtomicBool>` — flag para parar o loop).

Funções:

- `start(port)` — Abre um socket UDP na porta especificada (idealmente porta 53, a porta padrão DNS). Entra em um loop infinito que: (1) recebe um pacote DNS bruto do socket, (2) parseia o QNAME (o nome do domínio sendo consultado) usando `trust-dns-proto` para decodificar o formato DNS wire, (3) normaliza o domínio extraído usando `domain_matcher::normalize_domain`, (4) verifica se está na blocklist do usuário OU se o `adult_filter.is_adult()` retorna true. Se bloqueado: constrói uma resposta DNS apontando para `0.0.0.0` (o navegador tenta conectar a um endereço que não existe e o site não carrega). Se permitido: encaminha o pacote original para o DNS upstream (8.8.8.8), aguarda a resposta, e repassa de volta ao cliente.
- `stop()` — Seta `is_running = false`, causando o fim do loop.
- `update_blocklist(domains)` — Atualiza o `HashSet` dentro do `RwLock` com a nova lista de domínios. Chamada quando o usuário adiciona/remove itens.

Para que o sistema use o DNS Proxy, é necessário alterar o DNS do adaptador de rede via `netsh interface ip set dns "Ethernet" static 127.0.0.1`. Ao parar, reverter para o DNS original (salvar o valor anterior antes de alterar).

**blocking/engine.rs** — Orquestrador que coordena o DNS Proxy, o Adult Filter e o WFP como uma unidade. O frontend não interage com cada componente individualmente — ele chama `engine.start()` e `engine.stop()`.

Struct `BlockingEngine` com `dns_proxy`, `adult_filter`, e `is_active`. Funções:

- `start(blocklist, adult_filter_enabled)` — Sequência de inicialização: se `adult_filter_enabled` é true, carregar a lista (do cache ou URL); iniciar o DNS Proxy em uma task separada (tokio::spawn); configurar o sistema para usar o proxy como DNS; marcar `is_active = true`. Se algum passo falhar, desfazer os anteriores (rollback).
- `stop()` — Sequência de desligamento: parar o DNS Proxy; restaurar o DNS original do sistema; marcar `is_active = false`. A restauração do DNS é crítica — se o app crashar sem restaurar, o usuário fica sem internet.
- `update_rules(blocklist)` — Atualiza a blocklist no DNS Proxy sem precisar reiniciar o engine inteiro. Chamada quando o usuário adiciona/remove sites.

**blocking/wfp.rs** — Camada extra de segurança usando o Windows Filtering Platform. O DNS Proxy sozinho já bloqueia a maioria dos acessos, mas um usuário técnico poderia mudar o DNS do sistema manualmente para contornar o bloqueio. O WFP impede isso criando regras no firewall do Windows que interceptam o tráfego de rede no nível do kernel.

Struct `WfpFilter` com `engine_handle` e `filter_ids` (lista de IDs dos filtros criados). Funções:

- `new()` — Abre o engine WFP com `FwpmEngineOpen0`. Requer privilégios de administrador.
- `add_block_rule(ip)` — Adiciona um filtro com `FwpmFilterAdd0` na camada `FWPM_LAYER_ALE_AUTH_CONNECT_V4` que bloqueia conexões para o IP especificado. Funciona com IPs (não domínios), servindo como camada anti-bypass: se alguém tentar acessar pelo IP direto em vez do domínio, o WFP bloqueia.
- `remove_block_rule(id)` — Remove um filtro pelo ID.
- `clear_all()` — Remove todos os filtros criados pelo DopaBlocker.
- `drop()` — Fecha o engine WFP e limpa todos os filtros. Implementar via trait `Drop` para garantir limpeza mesmo se o app crashar.

> **Nota importante:** Para o protótipo, o DNS Proxy sozinho já bloqueia a grande maioria dos acessos. O WFP é uma camada extra de segurança. Se for muito complexo, pode ser adiado sem prejudicar a funcionalidade básica.

### Como testar

Com o app rodando, ativar bloqueio via `invoke('toggle_blocking', { enabled: true })`. Tentar acessar um site bloqueado no navegador — deve falhar com `ERR_NAME_NOT_RESOLVED`. Desativar o bloqueio — o site volta a funcionar. Testar também: adicionar um site enquanto o bloqueio está ativo (deve bloquear imediatamente), e verificar que sites não bloqueados continuam acessíveis.

### Critério de conclusão

- DNS Proxy intercepta queries e bloqueia domínios da blocklist
- Adult Filter carrega lista e identifica sites adultos
- Engine liga/desliga corretamente sem deixar resíduos
- DNS do sistema é restaurado ao desligar

---

## Fase D3 — Frontend SvelteKit (UI Completa)

### Por que esta fase

O backend Rust do Tauri funciona, os comandos IPC estão registrados, e o blocking engine bloqueia sites. Agora é preciso criar a interface visual que o usuário vai interagir. Sem ela, o app só funciona via console de desenvolvedor.

### Dependências a instalar

```bash
cd desktop && pnpm add firebase @tauri-apps/api
```

### Ordem de implementação

1. Serviços (firebase.ts, api.ts, tauri-bridge.ts)
2. Tipos (types.ts)
3. Stores (auth.ts, blocking.ts)
4. Layout + login
5. Dashboard (home)
6. Página de bloqueios
7. Página parental
8. Página de configurações

### Detalhamento

**services/firebase.ts** — Inicializar o Firebase Auth SDK com a configuração do projeto dopablocker-b8425. Este módulo encapsula toda a interação com o Firebase para que o resto do app não precise conhecer a API do Firebase diretamente. Exportar funções: `loginEmail(email, password)` que usa `signInWithEmailAndPassword`, `loginGoogle()` que abre popup de login Google via `signInWithPopup` com `GoogleAuthProvider`, `register(email, password)` que usa `createUserWithEmailAndPassword`, `logout()` que chama `signOut`, `getToken()` que retorna o JWT atual via `currentUser.getIdToken()`, e `onAuthChange(callback)` que escuta mudanças de autenticação via `onAuthStateChanged`.

**services/api.ts** — Camada de comunicação com o backend REST. A função genérica `request(method, path, body?)` deve: obter o JWT via `getToken()`, montar os headers com `Content-Type: application/json` e `Authorization: Bearer <token>`, fazer `fetch` para `BACKEND_URL + path`, e tratar erros (status != 2xx). Exportar um objeto `api` com métodos `.get(path)`, `.post(path, body)`, `.put(path, body)`, `.delete(path)`. Essa abstração evita repetir a lógica de autenticação em cada chamada.

**services/tauri-bridge.ts** — Objeto com funções tipadas que chamam `invoke()` do `@tauri-apps/api/core`. Cada função encapsula um comando Tauri específico com tipos corretos de parâmetro e retorno. O frontend nunca chama `invoke()` diretamente — sempre passa pelo bridge, que garante tipagem e centraliza a interface com o Rust.

**types.ts** — Interfaces TypeScript que espelham os modelos Rust do crate shared: `User`, `Device`, `BlockedItem`, `ParentalLink`. Definir também: `BlockMode` como union type `'personal' | 'parental'` e `BlockItemType` como `'site' | 'app'`. Esses tipos garantem que o frontend e o backend concordam sobre a forma dos dados.

**stores/auth.ts** — Svelte writable store com: `user` (`User | null`), `isAuthenticated` (boolean), `isLoading` (boolean). Funções que atualizam o store: `login`, `loginWithGoogle`, `register`, `logout`. No `+layout.svelte`, chamar `onAuthChange` para manter o store sincronizado com o estado do Firebase — se o token expirar e for renovado, ou se o usuário fizer logout em outra aba, o store reflete automaticamente.

**stores/blocking.ts** — Svelte writable store com: `blocklist` (`BlockedItem[]`), `isBlockingActive` (boolean), `isAdultFilterEnabled` (boolean), `blockMode` (`BlockMode`). Funções: `fetchBlocklist` (busca via tauri-bridge e atualiza o store), `addItem` (chama bridge + atualiza store), `removeItem`, `toggleBlocking` (chama bridge que liga/desliga o engine), `toggleAdultFilter`. Cada função chama o tauri-bridge e depois atualiza o store — garantindo que a UI sempre reflete o estado real.

**Componentes** — Implementar com Tailwind CSS:

1. `LoginForm.svelte` — Formulário com campos email/senha com validação, botão de login por email, botão de login com Google (estilizado com ícone), link para registro, e logo do DopaBlocker.
2. `BlockList.svelte` — Lista que itera sobre a blocklist do store, renderizando um `BlockListTile` para cada item. Mostra mensagem "Nenhum site bloqueado" quando a lista está vazia.
3. `AddBlockModal.svelte` — Dialog modal com input para URL ou nome do app, select para escolher tipo (site/app), e botões Cancelar/Adicionar. Valida que o campo não está vazio antes de enviar.
4. `ParentalDashboard.svelte` — Lista de dispositivos-filhos vinculados com status de cada um. Mostra mensagem quando nenhum dispositivo está vinculado.
5. `DeviceLinkCode.svelte` — Dois modos: (1) para o pai, exibe o código de 6 dígitos com countdown de 5 minutos e botão para gerar novo código; (2) para o filho, mostra input para inserir o código com botão de confirmar.
6. `ModeSelector.svelte` — Radio/toggle para alternar entre Pessoal e Parental, com opção pai/filho quando Parental está selecionado.

**Páginas:**

1. `login/+page.svelte` — Usa LoginForm, redireciona para `/` se já autenticado.
2. `+page.svelte` (home) — Card grande com status do bloqueio (ativo/inativo, verde/vermelho), botão on/off central, contador de sites e apps bloqueados, indicador do modo atual.
3. `blocking/+page.svelte` — BlockList + AddBlockModal + toggle de filtro adulto + botão master de bloqueio.
4. `parental/+page.svelte` — ModeSelector + DeviceLinkCode + ParentalDashboard.
5. `settings/+page.svelte` — Info da conta + logout + desvincular dispositivos.

**+layout.svelte** — Adicionar navegação (sidebar ou top nav) com links para `/`, `/blocking`, `/parental`, `/settings`. Esconder a navegação na página de login. Chamar `onAuthChange` aqui para que o listener de autenticação esteja ativo em todas as páginas.

### Como testar

`pnpm tauri dev`. Testar fluxo completo: login, adicionar sites, ativar bloqueio, verificar que sites são bloqueados no navegador, desativar, testar parental (gerar código, vincular).

### Critério de conclusão

- Login/registro funciona com Firebase
- Blocklist sincroniza com backend
- Toggle liga/desliga o engine
- Filtro adulto funciona
- Navegação entre páginas funciona
- Interface estilizada com Tailwind

---

# PARTE 3 — TRILHA MOBILE

Tudo específico do app Android: Flutter core, serviços nativos Kotlin, e UI.

**Pré-requisito:** Fases B1–B4 concluídas.

---

## Fase M1 — Flutter Core (Firebase + API + Modelos + Providers)

### Por que esta fase

Mesmo raciocínio do desktop: primeiro a infraestrutura (Firebase Auth, API client, modelos de dados, gerenciamento de estado), depois a UI e o código nativo. Sem essa base, as telas não teriam de onde buscar dados nem como se comunicar com o backend.

### Dependências a adicionar no mobile/pubspec.yaml

```yaml
dependencies:
  firebase_core: ^3.8.0
  firebase_auth: ^5.3.0
  google_sign_in: ^6.2.0
  http: ^1.2.0
  flutter_riverpod: ^2.6.0
  shared_preferences: ^2.3.0
  sqflite_sqlcipher: ^3.1.0
  path: ^1.9.0
```

> **sqflite_sqlcipher** é um drop-in replacement do sqflite que usa SQLCipher por baixo. A API é idêntica ao sqflite, mas ao abrir o banco você passa um `password` e todos os dados ficam criptografados no disco. No Android, isso protege o banco local mesmo em dispositivos rooteados.

### Configuração Firebase Android (obrigatória antes de codar)

1. No Firebase Console, adicionar app Android com package `com.dopablocker.dopablocker_mobile`
2. Baixar `google-services.json` e colocar em `mobile/android/app/`
3. Editar `mobile/android/build.gradle` para incluir o classpath do Google Services plugin
4. Editar `mobile/android/app/build.gradle` para aplicar o plugin e definir minSdk compatível

Sem isso, o Firebase não inicializa e nada funciona. Esse é o passo que mais causa problemas em projetos Flutter — se o app crashar na inicialização, verificar primeiro se o `google-services.json` está no lugar certo.

### Arquivos

| Arquivo | O que implementar |
|---------|------------------|
| `lib/main.dart` | WidgetsFlutterBinding, Firebase.initializeApp, ProviderScope, rodar App |
| `lib/app.dart` | MaterialApp com ThemeData e rotas |
| `lib/theme.dart` | ThemeData com cores DopaBlocker, estilos de botão/input/card |
| `lib/routes.dart` | Map de rotas nomeadas: /login, /home, /blocking, /parental, /link-device, /settings |
| `lib/core/constants.dart` | BACKEND_URL (dev/prod), nomes dos method channels, chaves SharedPreferences |
| `lib/core/firebase_service.dart` | Wrapper Firebase Auth: signInEmail, signInGoogle, register, signOut, getIdToken, authStateChanges |
| `lib/core/api_client.dart` | Classe HTTP com get/post/put/delete, JWT automático no header, base URL de constants |
| `lib/core/database_service.dart` | Abrir SQLCipher local com openDatabase(password: key), criar tabelas, CRUD para cache offline de blocklist/devices |
| `lib/models/user.dart` | User com fromJson, toJson, copyWith |
| `lib/models/device.dart` | Device com fromJson, toJson, copyWith |
| `lib/models/blocked_item.dart` | BlockedItem com fromJson, toJson, copyWith |
| `lib/providers/auth_provider.dart` | StateNotifierProvider: login, loginWithGoogle, logout, checkAuthState |
| `lib/providers/blocking_provider.dart` | StateNotifierProvider: fetchBlocklist, addItem, removeItem, toggleBlocking, toggleAdultFilter. Chama blocking_channel para controlar VPN |
| `lib/providers/device_provider.dart` | StateNotifierProvider: registerDevice, generateLinkCode, confirmLinkCode, getLinkedDevices |
| `lib/channels/blocking_channel.dart` | MethodChannel 'com.dopablocker/blocking': startVpn, stopVpn, isVpnActive, updateBlocklist |

### Detalhamento

**Modelos Dart** — Cada modelo espelha os campos definidos no `shared/src/models.rs`. A conversão entre Dart e Rust passa por JSON, então os modelos precisam lidar com a diferença de convenções de nomenclatura: o backend Rust usa `snake_case` (ex: `firebase_uid`), enquanto o Dart idiomático usa `camelCase` (ex: `firebaseUid`). Implementar:

- `factory fromJson(Map<String, dynamic> json)` — Mapeia as chaves JSON `snake_case` para campos Dart `camelCase`. Ex: `json['firebase_uid']` → `firebaseUid`.
- `Map<String, dynamic> toJson()` — Faz o inverso: `firebaseUid` → `'firebase_uid'`.
- `copyWith({...})` — Retorna uma nova instância com campos alterados, preservando imutabilidade. Isso é essencial para o Riverpod: em vez de mutar o estado, você cria uma cópia modificada, e o framework detecta a mudança e re-renderiza os widgets afetados.

**database_service.dart** — Classe singleton que gerencia o banco SQLCipher local. Na inicialização, chama `openDatabase(path, password: sqlcipherKey)` do pacote `sqflite_sqlcipher`. A senha criptografa o banco inteiro no disco. Cria as tabelas de cache (blocked_items, devices, adult_filter_settings) com `CREATE TABLE IF NOT EXISTS`. Funções CRUD para ler e escrever dados locais. O banco local é um espelho do backend para funcionar offline: ao adicionar um item, grava localmente primeiro (efeito imediato) e envia para o backend em background.

**Providers Riverpod** — Cada provider é um `StateNotifier` que gerencia um estado imutável. O Riverpod garante que qualquer widget que observe um provider é reconstruído automaticamente quando o estado muda — sem callbacks manuais, sem `setState()`.

- `auth_provider` — Escuta `authStateChanges` do Firebase e atualiza o estado (logado/deslogado/carregando). Quando o usuário faz login, obtém o JWT e chama o backend `/auth/login` para sincronizar. Quando faz logout, limpa o estado e redireciona para a tela de login.
- `blocking_provider` — Gerencia a blocklist e o estado do bloqueio. Faz chamadas ao `api_client` para sincronizar com o backend e ao `blocking_channel` para controlar o bloqueio nativo (VPN). Quando o usuário adiciona um site, o provider: salva no banco local (via `database_service`), atualiza o state (UI reflete), envia para o backend (sync), e atualiza a blocklist na VPN (via `blocking_channel`).
- `device_provider` — Comunica exclusivamente com o backend via `api_client`. Gerencia a lista de dispositivos e a vinculação parental.

**blocking_channel.dart** — Define a ponte Flutter ↔ Kotlin. Declara um `MethodChannel` com nome `'com.dopablocker/blocking'` e expõe métodos estáticos que chamam `invokeMethod`. Os métodos são: `startVpn()`, `stopVpn()`, `isVpnActive()`, `updateBlocklist(List<String> domains)`. Nesta fase, o lado Kotlin ainda não responde — o channel só vai funcionar de verdade na Fase M2. Chamadas vão lançar `MissingPluginException`, o que é esperado.

### Como testar

`flutter run`. Testar login com Firebase (precisa do `google-services.json`). Verificar que os providers carregam dados do backend. O MethodChannel vai lançar `MissingPluginException` até a Fase M2 — isso é esperado e não é um erro.

### Critério de conclusão

- Firebase inicializa sem erro
- Login/registro funciona (email e Google)
- API client comunica com o backend e retorna dados
- Providers armazenam e atualizam estado corretamente
- Banco SQLCipher local cria e persiste dados

---

## Fase M2 — Kotlin Nativo (VPN + Accessibility + Boot)

### Por que esta fase

O bloqueio no Android depende de serviços nativos que não existem em Dart. O Flutter roda numa camada de abstração — ele não tem acesso direto às APIs de rede e sistema do Android. Por isso, três serviços precisam ser implementados em Kotlin nativo: o `VpnService` intercepta todo o tráfego DNS, o `AccessibilityService` detecta e bloqueia abertura de apps, e o `BootReceiver` garante que o bloqueio sobrevive a reinicializações do dispositivo.

### Ordem de implementação

1. MainActivity.kt — Registrar MethodChannel e conectar com Flutter
2. VpnManager.kt — Gerenciar ciclo de vida da VPN
3. DnsVpnService.kt — VPN real com bloqueio DNS
4. AppBlockerService.kt — Detectar e bloquear abertura de apps
5. BootReceiver.kt — Auto-restart no boot

### Detalhamento

**MainActivity.kt** — Ponto de entrada da comunicação Flutter ↔ Kotlin. Sobrescrever `configureFlutterEngine` para criar um `MethodChannel` com nome `'com.dopablocker/blocking'` (o mesmo nome declarado no `blocking_channel.dart` do Flutter). No handler, rotear por `call.method`: `"startVpn"` chama `VpnManager.start()`, `"stopVpn"` chama `VpnManager.stop()`, `"isVpnActive"` retorna booleano, `"updateBlocklist"` extrai a lista de domínios do argumento (`call.argument<List<String>>("domains")`) e repassa para `DnsVpnService`. Cada chamada deve retornar `result.success(valor)` ou `result.error(código, mensagem)`.

**vpn/VpnManager.kt** — Objeto singleton que gerencia o ciclo de vida da VPN. O Android exige várias etapas para iniciar uma VPN, e encapsulá-las aqui simplifica o uso. Funções:

- `prepare(activity)` — Chama `VpnService.prepare(activity)`, que retorna um `Intent` se o sistema precisa pedir permissão ao usuário (o Android exibe um dialog "Permitir VPN?"). Retorna `true` se a VPN já está autorizada, `false` se precisa de permissão. O flutter exibe a UI de permissão antes de chamar `start`.
- `start(context)` — Lança o `DnsVpnService` via `startForegroundService(intent)`. Usa `startForegroundService` (não `startService`) porque o Android 8+ exige foreground service para serviços de longa duração.
- `stop(context)` — Para o serviço, que dispara o `onDestroy` do `DnsVpnService`.
- `isActive()` — Retorna uma flag booleana atualizada pelo `DnsVpnService`.

**vpn/DnsVpnService.kt** — A parte mais técnica do projeto inteiro. Este serviço cria uma VPN local que intercepta todo o tráfego de rede do dispositivo e filtra os pacotes DNS. Estende `android.net.VpnService`. O fluxo no `onStartCommand`:

1. **Criar interface TUN** — A TUN (tunnel) é uma interface de rede virtual. Usar o `Builder` do VpnService: `addAddress("10.0.0.2", 32)` define o IP da interface, `addDnsServer("10.0.0.1")` define o DNS da VPN, `addRoute("0.0.0.0", 0)` captura todo o tráfego IPv4, `establish()` ativa a interface e retorna um FileDescriptor.
2. **Criar notificação persistente** — Obrigatório para foreground service no Android. Chamar `startForeground(id, notification)`. Sem isso, o sistema mata o serviço em segundos.
3. **Iniciar loop de leitura** — Em uma thread/coroutine separada, ler pacotes da interface TUN em loop infinito. Cada pacote lido é um pacote IP completo (cabeçalho IP + cabeçalho UDP/TCP + payload).
4. **Filtrar pacotes DNS** — Identificar pacotes com porta destino 53 (DNS). Parsear o QNAME (nome do domínio sendo consultado) do payload DNS. Verificar se o domínio está na blocklist.
5. **Se bloqueado** — Construir uma resposta DNS "fake" com o IP `0.0.0.0` (formato: copiar o header da query, setar flag de resposta, adicionar answer record com A=0.0.0.0), e escrever de volta na interface TUN. O app que fez a query recebe `0.0.0.0` como resposta e não consegue conectar.
6. **Se permitido** — Abrir um socket UDP real (fora da VPN, usando `protect(socket)` para evitar loop) para o DNS upstream (8.8.8.8:53), encaminhar a query original, receber a resposta legítima, e escrevê-la na interface TUN.

Manter a blocklist como variável estática (`companion object`) que pode ser atualizada via `updateBlocklist(domains)` chamado do MethodChannel — sem precisar reiniciar a VPN inteira.

Registrar no `AndroidManifest.xml` como `<service>` com `android:permission="android.permission.BIND_VPN_SERVICE"` e `<intent-filter>` para `android.net.VpnService`.

**accessibility/AppBlockerService.kt** — Estende `AccessibilityService`. Este serviço recebe eventos do sistema sempre que uma janela muda (app aberto, tela trocada). No `onAccessibilityEvent`, verificar se `eventType == TYPE_WINDOW_STATE_CHANGED`. Extrair o `packageName` do evento — esse é o identificador único do app que acabou de abrir (ex: `com.instagram.android`). Se o `packageName` está na lista de apps bloqueados, criar um `Intent` para a `MainActivity` do DopaBlocker com `FLAG_ACTIVITY_NEW_TASK` — isso traz o DopaBlocker para frente, efetivamente impedindo o uso do app bloqueado.

Registrar no `AndroidManifest.xml` com `android:permission="android.permission.BIND_ACCESSIBILITY_SERVICE"`, `<intent-filter>` para `AccessibilityService`, e `<meta-data>` apontando para `res/xml/accessibility_config.xml`. Criar esse XML com `accessibilityEventTypes="typeWindowStateChanged"` e `canRetrieveWindowContent="false"` (não precisamos ler o conteúdo das telas, apenas detectar troca de app).

**receivers/BootReceiver.kt** — Estende `BroadcastReceiver`. O Android dispara o broadcast `BOOT_COMPLETED` quando o dispositivo termina de inicializar. No `onReceive`, verificar se a action é `BOOT_COMPLETED`, ler `SharedPreferences` para saber se a VPN estava ativa antes do reboot. Se sim, chamar `VpnManager.start(context)`. Isso garante que o bloqueio sobrevive a reinicializações — sem isso, bastaria reiniciar o celular para desativar o bloqueio.

Registrar no `AndroidManifest.xml` com `<intent-filter>` para `BOOT_COMPLETED`.

**Permissões no AndroidManifest.xml:**

```xml
<uses-permission android:name="android.permission.RECEIVE_BOOT_COMPLETED" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_SPECIAL_USE" />
```

### Como testar

1. `flutter run` no emulador ou celular físico
2. Ativar bloqueio pela UI — sistema pede permissão VPN (aceitar)
3. Abrir navegador e tentar acessar site bloqueado — deve falhar
4. Abrir app bloqueado — deve redirecionar para o DopaBlocker
5. Reiniciar dispositivo — VPN deve reiniciar automaticamente

### Critério de conclusão

- VPN intercepta DNS e bloqueia domínios da lista
- Accessibility detecta apps bloqueados e redireciona
- Boot Receiver reinicia VPN após reboot
- Permissões solicitadas corretamente ao usuário

---

## Fase M3 — UI Flutter (Telas + Widgets)

### Por que esta fase

Com providers, services e código nativo funcionando, todas as camadas invisíveis estão prontas. Agora é preciso criar as telas que o usuário realmente vai ver e interagir. Cada tela consome dados dos providers (via Riverpod) e dispara ações que percorrem toda a stack: UI → Provider → API/Channel → Backend/Nativo.

### Ordem de implementação

1. theme.dart — Cores e estilos
2. routes.dart — Mapa de rotas
3. app.dart — MaterialApp com theme e routes
4. login_screen.dart
5. home_screen.dart
6. block_list_tile.dart + add_block_dialog.dart
7. blocking_screen.dart
8. mode_selector.dart
9. link_device_screen.dart
10. parental_screen.dart
11. settings_screen.dart

### Detalhamento

Todas as telas devem ser `ConsumerWidget` (Riverpod) em vez de `StatelessWidget`. Isso dá acesso ao `ref`, que permite observar providers com `ref.watch()` (a tela re-renderiza quando o provider muda) e disparar ações com `ref.read().método()`.

**login_screen.dart** — Campos de email e senha com validação (email válido, senha não vazia). Botão de login por email que chama `ref.read(authProvider.notifier).login(email, password)`. Botão de login com Google estilizado com ícone do Google. Link "Criar conta" que alterna para modo registro. Logo do DopaBlocker no topo. Redirecionar para `/home` após login bem-sucedido. Mostrar `SnackBar` em caso de erro (credenciais inválidas, rede indisponível).

**home_screen.dart** — Dashboard principal, primeira tela após login. Card grande com status do bloqueio: fundo verde e texto "Bloqueio ativo" quando ligado, fundo vermelho e "Bloqueio inativo" quando desligado. Botão on/off central que chama `toggleBlocking`. Contador de sites e apps bloqueados (lido do `blocking_provider`). Indicador do modo atual (Pessoal/Parental). Usar `BottomNavigationBar` com 4 itens: Home, Bloqueios, Parental, Config.

**blocking_screen.dart** — Tela de gerenciamento da blocklist. Mostra a lista de itens bloqueados usando `block_list_tile` para cada item. FAB (Floating Action Button) no canto inferior que abre `add_block_dialog`. Switch para ligar/desligar o filtro de conteúdo adulto. Botão master de bloqueio (bloquear/desbloquear tudo). No modo parental-filho, a lista é exibida como read-only — o filho não pode editar os bloqueios definidos pelo pai.

**block_list_tile.dart** — Widget reutilizável que recebe um `BlockedItem`. Mostra ícone (globo para site, ícone de app para app), nome/URL, e status ativo/inativo com cor diferente. Gesture de swipe para remover ou ícone de lixeira que chama `removeItem`.

**add_block_dialog.dart** — `AlertDialog` com `TextField` para digitar URL ou nome do app. `SegmentedButton` para escolher o tipo (site/app). Botões Cancelar e Adicionar. Validar que o campo não está vazio antes de enviar. Ao confirmar, chama `addItem` no provider e fecha o dialog.

**parental_screen.dart** — `ModeSelector` no topo para escolher entre Pai e Filho. Se o modo é Pai: lista de dispositivos-filhos vinculados (vindos do `device_provider`), botão para gerar código de vinculação, e opção de gerenciar a blocklist dos filhos. Se o modo é Filho: mensagem informando que os bloqueios são gerenciados pelo pai, e exibição dos bloqueios ativos como lista read-only.

**link_device_screen.dart** — Tela separada (sem bottom nav) para o processo de vinculação. Dois modos:
- Gerando (pai): exibe o código de 6 dígitos em fonte grande e legível, countdown de 5 minutos mostrando o tempo restante, e botão para gerar um novo código se o atual expirar.
- Inserindo (filho): 6 campos de input separados (um por dígito) que avançam automaticamente o foco, botão Confirmar, e feedback visual de sucesso ou erro.

**settings_screen.dart** — Informações da conta (avatar, email, nome), botão de logout com dialog de confirmação ("Tem certeza?"), opção de desvincular dispositivos, e versão do app no rodapé.

**Navegação** — Usar `BottomNavigationBar` persistente nas telas principais (home, blocking, parental, settings). As telas de login e link_device são telas separadas sem bottom nav, acessadas via `Navigator.push`.

### Como testar

`flutter run`. Testar o fluxo completo: login, navegar entre telas, adicionar/remover bloqueios, ativar/desativar VPN, gerar e inserir código parental, logout.

### Critério de conclusão

- Todas as 6 telas renderizam sem erro
- Navegação entre telas funciona
- Login/logout atualiza a UI
- Blocklist exibe, adiciona e remove itens
- Toggle de bloqueio ativa/desativa VPN
- Modo parental permite gerar/inserir código

---

# CONVERGÊNCIA — De Volta à Trilha Base

Quando ambas as trilhas (Desktop e Mobile) estiverem concluídas, continuar com as fases finais.

---

## Fase B5 — Integração Cross-Platform

### Por que esta fase

Desktop e mobile funcionam individualmente. Agora é preciso garantir que funcionam **juntos**: mesma conta, mesma blocklist, controle parental entre dispositivos diferentes. Problemas de sincronização só aparecem quando os dois apps estão rodando ao mesmo tempo, então não podem ser testados antes.

### Cenários de teste obrigatórios

**Cenário 1 — Sync de blocklist (modo pessoal):**
1. Logar na mesma conta no desktop e no mobile
2. No desktop, adicionar "instagram.com" à blocklist
3. No mobile, verificar que "instagram.com" aparece
4. No mobile, adicionar "tiktok.com"
5. No desktop, verificar que "tiktok.com" aparece

**Cenário 2 — Controle parental cross-device:**
1. No desktop, selecionar modo Parental > Pai
2. Gerar código de vinculação
3. No mobile, selecionar modo Parental > Filho
4. Inserir o código e confirmar vinculação
5. No desktop (pai), adicionar "youtube.com" à blocklist
6. No mobile (filho), verificar que youtube.com está bloqueado
7. No mobile (filho), tentar desbloquear — não deve ser possível

**Cenário 3 — Filtro adulto cross-device:**
1. No desktop, ativar filtro de conteúdo adulto
2. No mobile (mesma conta), verificar que o filtro está ativo
3. Tentar acessar site adulto em ambos — deve ser bloqueado

**Cenário 4 — Offline e reconexão:**
1. Desconectar da internet em um dispositivo
2. A blocklist local deve continuar funcionando, bloqueio ativo
3. Reconectar — deve sincronizar mudanças pendentes

### Implementação da sincronização

Se os cenários falharem, é porque a sincronização precisa de ajustes. O fluxo correto é:

```
Ação do usuário (add/remove)
  → Salva no SQLCipher local (imediato, para funcionar offline)
  → Envia para Backend API (async)
  → Backend salva no SQLCipher do server + Firestore

Outro dispositivo:
  → Ao abrir o app OU a cada 30 segundos: GET /blocklist
  → Atualiza SQLCipher local + UI
```

Para o protótipo, polling simples é suficiente. Listeners real-time do Firestore podem ser adicionados no futuro.

### Critério de conclusão

- Os 4 cenários acima passam sem erros
- Dados sincronizam em menos de 30 segundos entre dispositivos

---

## Fase B6 — Docker, Deploy e Polimento Final

### Docker

Implementar o `backend/Dockerfile` real com build multi-stage:

- **Stage builder:** usar `rust:latest`, copiar source (shared/ e backend/), rodar `cargo build --release`. A feature `bundled-sqlcipher` compila o SQLCipher dentro do binário, então não é necessário instalar dependências externas no container.
- **Stage runtime:** usar `debian:bookworm-slim`, copiar apenas o binário compilado do stage anterior, expor porta 3000, definir `CMD` para rodar o servidor.

O Dockerfile está em `backend/` mas precisa do `shared/` para compilar (é uma dependência via Cargo workspace). Rodar `docker build` a partir da raiz do monorepo com `-f backend/Dockerfile .` ou copiar `shared/` no Dockerfile.

Testar: `cd infra && docker compose up --build`. A variável `SQLCIPHER_KEY` é passada via env no `compose.yml`.

### Deploy na Azure

1. Criar Azure Container Registry (ACR)
2. Push da imagem Docker para o ACR
3. Criar Azure Container App ou App Service
4. Configurar variáveis de ambiente (PORT, DATABASE_URL, FIREBASE_PROJECT_ID, SQLCIPHER_KEY)
5. Atualizar URL do backend nos clients (desktop e mobile) para URL de produção

### Builds de produção

- Desktop: `pnpm tauri:build` — gera instalador .msi/.exe em `desktop/src-tauri/target/release/bundle/`
- Mobile: `flutter build apk --release` — gera APK em `mobile/build/app/outputs/flutter-apk/`

### Polimento

- Substituir ícones placeholder (Tauri e Flutter) pelo ícone real do DopaBlocker
- Splash screen no mobile
- Tratar estados de erro na UI: sem internet, token expirado, permissão negada, servidor fora
- Garantir que permissões (VPN, Accessibility) são pedidas com explicação clara ao usuário
- Testar em dispositivos reais (não apenas emulador)

### Critério de conclusão

- Docker build funciona e container roda
- Backend acessível via URL pública
- Desktop gera instalador funcional
- Mobile gera APK funcional
- Fluxo completo end-to-end funciona em ambiente de produção

---

## Resumo Geral

### Dependências por Fase

| Fase | Adicionar |
|------|----------|
| B1 - Shared | (já configurado) |
| B2 - Backend Core | rusqlite (bundled-sqlcipher), tokio-rusqlite, uuid, chrono |
| B3 - Backend Auth | jsonwebtoken, reqwest |
| B4 - Backend Blocklist/Devices | rand |
| D1 - Desktop Tauri Core | rusqlite (bundled-sqlcipher), tokio-rusqlite, uuid, tokio, reqwest |
| D2 - Desktop Blocking | trust-dns-proto, windows |
| D3 - Desktop Frontend | firebase, @tauri-apps/api (pnpm) |
| M1 - Mobile Core | firebase_core/auth, google_sign_in, http, flutter_riverpod, shared_preferences, sqflite_sqlcipher, path |
| M2 - Mobile Kotlin | (nenhuma — APIs nativas Android) |
| M3 - Mobile UI | (nenhuma) |
| B5 - Integração | (nenhuma) |
| B6 - Deploy | (nenhuma) |

### Complexidade por Fase

| Fase | Complexidade | Nota |
|------|-------------|------|
| B1 - Shared | Baixa | Structs + funções puras |
| B2 - Backend Core | Baixa | Boilerplate Axum + SQLCipher |
| B3 - Backend Auth | Média | Firebase JWT é a parte mais técnica |
| B4 - Backend Blocklist/Devices | Baixa | CRUD padrão |
| D1 - Desktop Tauri Core | Baixa | Comandos Tauri são diretos |
| D2 - Desktop Blocking | **Alta** | DNS Proxy e WFP requerem conhecimento de rede |
| D3 - Desktop Frontend | Média | Muitos componentes Svelte + Tailwind |
| M1 - Mobile Core | Média | Firebase setup Android pode dar trabalho |
| M2 - Mobile Kotlin | **Alta** | VPN Service é a parte mais complexa do projeto |
| M3 - Mobile UI | Média | Muitas telas Flutter |
| B5 - Integração | Média | Debug de sync cross-platform |
| B6 - Deploy | Baixa | Docker + build commands |

As Fases D2 (DNS Proxy + WFP) e M2 (VPN Android) são as mais desafiadoras tecnicamente. Reserve mais tempo para elas.
