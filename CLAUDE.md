# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> Documentação, código e comentários deste repositório são em **português (PT-BR)**.
> Mantenha esse padrão ao escrever código, comentários e mensagens.

## Visão geral

DopaBlocker é um bloqueador de distrações digitais para **Windows** (desktop) e **Android**
(mobile). Dois modos de uso na mesma conta: **pessoal** (o usuário controla os próprios
bloqueios) e **controle parental** (o pai gerencia a blocklist dos dispositivos filhos).

Monorepo com 4 sub-projetos:

| Pasta | Stack | Status |
|---|---|---|
| `shared/` | Crate Rust (modelos, bloom filter, domain matcher) | Funcional |
| `backend/` | API REST Rust/Axum + SQLCipher | Funcional |
| `desktop/` | Tauri 2 (Rust) + SvelteKit + Tailwind v4 | Funcional |
| `mobile/` | Flutter + Kotlin | Funcional — bloqueio de **sites (DNS)**, **apps** (overlay + seletor visual) e **conteúdo adulto** (Cloudflare for Families); pendente: cache SQLCipher Dart + fila offline |

`shared`, `backend` e `desktop/src-tauri` formam **um único workspace Cargo** (ver
[Cargo.toml](Cargo.toml) na raiz) — comandos `cargo` na raiz operam nos três.

## Comandos

| Ação | Comando |
|---|---|
| Compilar/checar todo o Rust | `cargo check` / `cargo build` (na raiz) |
| Rodar testes Rust | `cargo test` |
| Teste único / por crate | `cargo test -p dopablocker-shared <nome_do_teste>` |
| Rodar backend (porta 3000) | `cd backend && cargo run` |
| Rodar desktop (dev) | `pnpm tauri:dev` (na raiz) |
| Build desktop (produção) | `pnpm tauri:build` |
| Type-check do frontend | `cd desktop && pnpm check` |
| Build do frontend | `cd desktop && pnpm build` |
| Instalar deps do frontend | `cd desktop && pnpm install` |
| Análise estática mobile | `cd mobile && flutter analyze` |
| Backend via Docker | `cd infra && docker compose up --build` |

- **Testes Rust** são módulos `#[test]` / `#[tokio::test]` **inline** nos próprios arquivos
  (não há pasta `tests/`). Estão sobretudo em `shared/src/`, `backend/src/core/`,
  `backend/src/features/<feature>/` e `desktop/src-tauri/src/blocking/`.
- `pnpm tauri:dev` dispara `beforeDevCommand: pnpm dev`, que roda
  [desktop/scripts/dev-with-backend.mjs](desktop/scripts/dev-with-backend.mjs): sobe o
  **backend em :3000** (se ainda não estiver saudável) e o **Vite em :5173**, e encerra
  ambos juntos. Não é preciso subir o backend manualmente para o dev do desktop.
- Não há linter de Rust configurado além de `cargo check`; o frontend valida via
  `svelte-check` (`pnpm check`), não ESLint.

## Pré-requisitos de build não óbvios (Windows)

1. **SQLCipher/OpenSSL** — `backend` e `desktop/src-tauri` usam `rusqlite` com a feature
   `bundled-sqlcipher`, que **compila o SQLCipher do fonte** e linka contra o OpenSSL do
   sistema. Sem OpenSSL, `cargo build` falha com `Missing environment variable OPENSSL_DIR`.
   Setup via vcpkg + variáveis `OPENSSL_DIR` / `OPENSSL_STATIC` / `VCPKGRS_DYNAMIC` está
   detalhado no [README.md](README.md) ("Instalar OpenSSL via vcpkg").
2. **Engine de bloqueio exige admin** — o desktop bind na **porta 53 (DNS proxy)**, instala
   **filtros WFP** e **altera o DNS do sistema**. Rodar `pnpm tauri:dev` sem privilégios de
   administrador faz o boot do app funcionar, mas o `engine.start()` falha (porta 53 / WFP).
3. **WebView2** e **C++ Build Tools (MSVC)** são exigidos pelo Tauri/Rust no Windows.

## Arquitetura

### Backend (`backend/`) — Rust/Axum

Organizado **por feature/domínio** (espelha o mobile `lib/core` + `lib/features`):

- `core/` — infra compartilhada: `config`, `db`, `errors`, `models`, `util` e o cluster
  `auth/` (middleware dual + `jwks`/`jwt`/`device_token`).
- `features/<feature>/` — um diretório por domínio (`auth`, `devices`, `blocklist`), cada um
  com `routes.rs` (handlers) + módulos de domínio (`service.rs`, `email_code.rs`, etc.).
- `app.rs` monta o `Router` (rotas + CORS + rate-limit + health); `bootstrap.rs` inicializa o
  `AppState` (db + migrations); [main.rs](backend/src/main.rs) é o boot fino que costura os dois.

Dentro de cada feature vale o padrão **"rotas chamam serviços"**: `routes.rs` valida o JSON e
delega a regra de negócio para `service.rs`/módulos do domínio, que falam com o SQLCipher.
Erros propagam via `AppError` (`core/errors.rs`), que implementa `IntoResponse` → JSON
`{ "error": "..." }`. **Nunca use `unwrap()` na regra de negócio** — propague com `?` / `AppError`.

- Boot: [backend/src/main.rs](backend/src/main.rs) (fino) → `bootstrap::init_state` abre o
  SQLCipher (`PRAGMA key` é o **primeiro** comando, em `core/db.rs`, senão o banco não
  descriptografa), roda migrations idempotentes e monta `AppState { config, db, jwks }`;
  `app::build_router` monta as rotas.
- **Rotas públicas vs. protegidas** (montadas em `app.rs`): `/health`, `/healthz`,
  `/auth/register`, `/auth/login`, `/auth/email-code/start`, `/auth/email-code/verify`,
  `/devices/link/confirm` e `/devices/tamper` ficam **fora** do middleware de auth (o filho
  ainda não tem credencial ao confirmar o código). O resto passa por `core::auth::require_auth`.

### Autenticação dual

O cluster de auth ([backend/src/core/auth/](backend/src/core/auth/) — `middleware.rs` +
`jwt.rs` + `jwks.rs` + `device_token.rs`) inspeciona o prefixo do `Authorization: Bearer`:

- **Firebase JWT** (sem prefixo) — contas Pessoal/Pais. Valida assinatura via JWKS do Google
  (cacheado ~6h em `JwksCache`), checa `iss`/`aud`/`exp`, resolve `user_id` por `firebase_uid`.
- **Device Token** (`dt_...`) — devices filhos, gerado uma vez no link/confirm e salvo como
  **hash SHA-256** em `device_tokens`. Escopo **read-only**: POST/DELETE/PUT são rejeitados
  com 403 no próprio middleware.

Ambos resolvem para `AuthUser { user_id, source, device_id }`; handlers checam `source`
quando a regra depende do tipo de credencial.

### Engine de bloqueio do desktop (`desktop/src-tauri/src/blocking/`)

Módulo organizado **por cluster**: `dns/` (proxy + cache + upstream pool), `page/` (block page
HTTP/HTTPS + CA local + resolver SNI), `policy/` (decisão de bloqueio + filtro adulto), `os/`
(WFP + DNS do sistema), mais [engine.rs](desktop/src-tauri/src/blocking/engine.rs) e
[lifecycle.rs](desktop/src-tauri/src/blocking/lifecycle.rs) na raiz do módulo (helpers comuns
em `util.rs`).

- **`engine.rs`** sobe/derruba o stack *in-process*. Ordem de start importa: **WFP primeiro**
  (fecha a janela de bypass), CA local → block page HTTP(:80) → HTTPS(:443) → **DNS proxy(:53)**
  que devolve `127.0.0.1`/NXDOMAIN para domínios bloqueados. Filtro de conteúdo adulto via
  **Bloom filter** (crate `shared`), construído em background no boot. Camadas de page são
  best-effort; o bloqueio por DNS funciona mesmo se :443/CA falharem.
- **`lifecycle.rs`** é o **dono único da orquestração completa**: combina o `engine` + a troca do
  **DNS do sistema** (`os::system_dns`) + o flag persistido no DB. Expõe `enable`/`disable`/
  `resume_if_enabled`/`shutdown_and_disable`/`refresh_engine_rules`. [commands.rs](desktop/src-tauri/src/commands.rs)
  (IPC) e [lib.rs](desktop/src-tauri/src/lib.rs) (boot/tray) **só delegam** para ele — não
  reintroduza a coreografia "engine + DNS + rollback + flag" espalhada nesses arquivos. A decisão
  "é bloqueio? por quê?" fica em `policy::block_reason`, compartilhada entre o DNS proxy e a block page.

**Restauração de DNS é crítica**: o restore **síncrono** (sem tokio/SQLCipher, p/ panic e shutdown)
vive em [os/system_dns.rs](desktop/src-tauri/src/blocking/os/system_dns.rs)
(`restore_dns_blocking_global`); [lib.rs](desktop/src-tauri/src/lib.rs) instala panic hook,
`SetConsoleCtrlHandler` e trata `RunEvent::ExitRequested` para **restaurar o DNS do sistema** em
qualquer saída (crash, logoff, shutdown). Há self-heal síncrono no boot (`os::system_dns::heal_orphan_dns`)
que conserta DNS órfão apontando para loopback de um crash anterior. Ao mexer no ciclo de vida do app
ou no DNS, **preserve esses caminhos de cleanup** — sem eles, um crash deixa o usuário sem internet.

### Regra do "pai imune"

Antes de popular a lista de domínios, o engine consulta `mode` do user e `is_child` do device:
`personal` → aplica tudo; `parental` + filho → aplica tudo; `parental` + pai → **lista vazia**
(o proxy/VPN segue ativo, mas nada é bloqueado). Aplicada em todas as plataformas. Ver
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

### Frontend desktop (`desktop/src/`) — SvelteKit + Tailwind v4

SPA com `adapter-static` (sem SSR — roda dentro do WebView do Tauri). Três camadas em `lib/`:

- `services/api.ts` — cliente REST do backend (injeta JWT).
- `services/tauri-bridge.ts` — wrappers tipados sobre `invoke()`; o frontend **nunca chama
  `invoke()` direto**. Espelha o resultado do backend no cache SQLCipher local, de onde o
  engine lê. Os nomes/assinaturas devem casar com [desktop/src-tauri/src/commands.rs](desktop/src-tauri/src/commands.rs)
  e a lista em `tauri::generate_handler![]` em [lib.rs](desktop/src-tauri/src/lib.rs).
- `stores/` — estado reativo (`auth.ts`, `blocking.ts`).

Guard de rota em [+layout.svelte](desktop/src/routes/+layout.svelte): sessão Firebase
(Pessoal/Pais) e sessão de filho (Device Token) são autenticadas; o filho fica **preso em
`/child-blocked`** (read-only); rotas públicas são `/welcome`, `/login`, `/onboarding/child`.

### Armazenamento — SQLCipher em todo lugar

Banco SQLite criptografado (AES-256) via `PRAGMA key`. O **backend é a fonte de verdade**; o
**SQLCipher local do desktop é cache offline** do qual o engine lê a blocklist. Migrations do
backend em `backend/migrations/` (`001_initial`, `002_parental_fixes`, `003_email_verification`,
`004_device_events`).

## Convenções e armadilhas

- **`unwrap()` é proibido na regra de negócio** (backend) — propague erro via `AppError`/`?`.
- **Status real vence o doc**: para o que está implementado vs. aberto, confie em
  [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) (estado atual), [docs/RUNBOOK.md](docs/RUNBOOK.md)
  (limitações conhecidas), [docs/DECISOES_E_ROADMAP.md](docs/DECISOES_E_ROADMAP.md) (o que é
  proposta/roadmap) e no código. `ARCHITECTURE`/`API`/`RUNBOOK` descrevem só o estado atual.
- **`mobile/`** (v0.x): **bloqueio de sites por DNS** (engine nativo em `mobile/android/.../vpn/`:
  `DnsVpnService`, `DnsPacket`, `DomainMatcher`, `DnsForwarder`; testes JVM + instrumentados),
  **bloqueio de apps** (`AppBlockerService` lança o overlay full-screen `BlockOverlayActivity`;
  seletor visual de apps via `InstalledAppsProvider`) e **filtro de conteúdo adulto** (Cloudflare
  for Families `1.1.1.3`) estão **implementados**. Ainda **pendentes**: cache local SQLCipher em
  Dart (`mobile/lib/core/database_service.dart` é stub) e fila de escrita offline. Não assuma que
  o que está pendente funciona.
  - **App-block exige permissões do sistema concedidas pelo usuário**: o AccessibilityService
    (detecta o app aberto) **e** `SYSTEM_ALERT_WINDOW` ("sobrepor a outros apps", p/ o overlay).
    Sem isso o app entra na lista mas **não bloqueia** — há UI pedindo as permissões (banner em
    Bloqueios + tiles na Conta; estado em `mobile/lib/features/blocking/providers/permissions_provider.dart`). Ao detectar app
    bloqueado o serviço também dá `GLOBAL_ACTION_HOME` (o app "abre e fecha sozinho").
  - **No device do filho o bloqueio é obrigatório**: a `child_blocked_screen.dart` vira um **muro de
    setup** (VPN → acessibilidade → overlay; o passo de VPN usa `BlockingChannel.isVpnPrepared`) e só
    mostra "Proteção ativa" quando tudo é concedido — então o engine **sobe sozinho**
    (`blockingProvider.ensureEngineRunning`: VPN + sync) e reaplica no `resumed`. Antes a tela dizia
    "bloqueio ativo" mas nunca iniciava a VPN. Não volte a tratar a sessão de filho como read-only
    passiva sem reativar o engine.
  - **O `BlockOverlayActivity` é compartilhado**: cobre app bloqueado **e** site bloqueado. Para
    site, o `DnsVpnService` sinaliza `AppBlockerService.notifyBlockedDomain` (mesmo processo) e o
    overlay só aparece se um **navegador** estiver em foco (debounce por domínio). Não dá pra
    servir página no navegador sem root (portas 80/443 + HTTPS) — por isso é overlay.
- **Desktop não bloqueia app** (por design — o engine só carrega domínios); na sincronização
  Mobile→PC do modo pessoal, só **sites** se propagam. Bloqueio de app é exclusivo do mobile.
- **Sync por polling em todas as sessões**: o cache local (de onde o engine lê) é mantido em dia
  por polling periódico (~30–45s, com ETag/304) — device-filho **e** modo pessoal/pai. Desktop:
  `blockingStore.startAutoSync` ligado no [+layout.svelte](desktop/src/routes/+layout.svelte); filho em [child-blocked](desktop/src/routes/child-blocked/+page.svelte). Mobile: `_startPollIfNeeded`
  em [blocking_provider.dart](mobile/lib/features/blocking/providers/blocking_provider.dart).
- **Mudou um modelo? Mude em `shared/`**: `User`, `Device`, `BlockedItem` etc. vivem na crate
  `shared` e são reusados por backend e desktop; o frontend espelha em `desktop/src/lib/types.ts`.
- **Excluir conta = backend ANTES do Firebase** (desktop `routes/settings/+page.svelte`; mobile
  `mobile/lib/features/auth/providers/auth_provider.dart`): chame `DELETE /auth/me` **enquanto o token Firebase é válido**
  e só então apague o user do Firebase. Invertendo, após o delete do Firebase o `getIdToken()`
  volta `null`, o `DELETE /auth/me` sai sem token (401) e o user fica **órfão no backend**. Continua
  sendo a ordem correta — não reintroduza Firebase-primeiro. **Mas o órfão não é mais beco sem
  saída:** `POST /auth/register` é **idempotente** e faz **reclaim** (reassocia uma conta órfã do
  mesmo email a um `firebase_uid` novo, provando posse do email pelo mesmo mecanismo do cadastro),
  então o `email UNIQUE` não trava mais o recadastro. Trocar `personal`↔`parental` é via
  `PUT /auth/me` (sem recriar a conta). Ver [docs/API.md](docs/API.md) e A5 em
  [docs/DECISOES_E_ROADMAP.md](docs/DECISOES_E_ROADMAP.md).
- **Frontend mobile e desktop têm paridade**: mesmos design tokens (mobile `lib/shared/theme.dart` ↔
  desktop `src/app.css` — tema escuro azul→roxo, Inter + JetBrains Mono, motion `ease-out`
  Expo) e mesma IA/rótulos (**Início · Bloqueios · Filhos[só parental] · Conta**). Só exiba o
  que o backend/engine suporta de verdade (não há endpoint de estatísticas — nada de dashboard
  de métricas). Ao mexer numa tela, mantenha a paridade entre as duas plataformas.

## Mapa de documentação (`docs/`)

A pasta `docs/` foi consolidada em **4 arquivos**. `ARCHITECTURE.md`, `API.md` e `RUNBOOK.md`
descrevem **apenas o estado atual implementado**; propostas e roadmap ficam em
`DECISOES_E_ROADMAP.md` (rotuladas por status).

| Arquivo | Conteúdo |
|---|---|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Estado atual: fluxo de dados, auth dual + máquina de estados, storage, engine de bloqueio (desktop e mobile), pai imune |
| [API.md](docs/API.md) | Referência dos endpoints REST + auth dual |
| [RUNBOOK.md](docs/RUNBOOK.md) | Como rodar, golden path/smoke tests, verificações e **limitações conhecidas** |
| [DECISOES_E_ROADMAP.md](docs/DECISOES_E_ROADMAP.md) | Decisões revisadas (status: implementado/decidido/proposto), modelo de ameaça e roadmap em ondas |
| [README.md](README.md) | Setup completo do ambiente (Rust, OpenSSL, Node, Flutter, Docker, Firebase) |
