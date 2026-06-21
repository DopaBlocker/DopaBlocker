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
| `mobile/` | Flutter + Kotlin | Parcial — bloqueio de **sites por DNS** funcional; apps e filtro adulto pendentes |

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
  (não há pasta `tests/`). Estão sobretudo em `shared/src/`, `backend/src/services/`,
  `backend/src/routes/` e `desktop/src-tauri/src/blocking/`.
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

Padrão **"rotas chamam serviços"**: handlers em `routes/` validam o JSON e delegam a regra
de negócio para `services/`, que falam com o SQLCipher. Erros propagam via `AppError`
(`errors.rs`), que implementa `IntoResponse` → JSON `{ "error": "..." }`. **Nunca use
`unwrap()` na regra de negócio** — propague com `?` / `AppError`.

- Entry point [backend/src/main.rs](backend/src/main.rs): carrega `.env`, abre o SQLCipher
  (`PRAGMA key` é o **primeiro** comando, senão o banco não descriptografa), roda migrations
  idempotentes, monta `AppState { config, db, jwks }`.
- **Rotas públicas vs. protegidas**: `/health`, `/auth/register`, `/auth/login` e
  `/devices/link/confirm` ficam **fora** do middleware de auth (o filho ainda não tem
  credencial ao confirmar o código). O resto passa por `middleware::require_auth`.

### Autenticação dual

O middleware ([backend/src/middleware.rs](backend/src/middleware.rs)) inspeciona o prefixo
do `Authorization: Bearer`:

- **Firebase JWT** (sem prefixo) — contas Pessoal/Pais. Valida assinatura via JWKS do Google
  (cacheado ~6h em `JwksCache`), checa `iss`/`aud`/`exp`, resolve `user_id` por `firebase_uid`.
- **Device Token** (`dt_...`) — devices filhos, gerado uma vez no link/confirm e salvo como
  **hash SHA-256** em `device_tokens`. Escopo **read-only**: POST/DELETE/PUT são rejeitados
  com 403 no próprio middleware.

Ambos resolvem para `AuthUser { user_id, source, device_id }`; handlers checam `source`
quando a regra depende do tipo de credencial.

### Engine de bloqueio do desktop (`desktop/src-tauri/src/blocking/`)

Orquestrado por [engine.rs](desktop/src-tauri/src/blocking/engine.rs). Ordem de start
importa: **WFP primeiro** (fecha a janela de bypass), CA local → block page HTTP(:80) →
HTTPS(:443) → **DNS proxy(:53)** que devolve `127.0.0.1`/NXDOMAIN para domínios bloqueados.
Filtro de conteúdo adulto via **Bloom filter** (crate `shared`), construído em background no
boot. Camadas de page são best-effort; o bloqueio por DNS funciona mesmo se :443/CA falharem.

**Restauração de DNS é crítica**: [lib.rs](desktop/src-tauri/src/lib.rs) instala panic hook,
`SetConsoleCtrlHandler` e trata `RunEvent::ExitRequested` para **restaurar o DNS do sistema**
em qualquer saída (crash, logoff, shutdown). Há self-heal síncrono no boot que conserta DNS
órfão apontando para loopback de um crash anterior. Ao mexer no ciclo de vida do app ou no
DNS, **preserve esses caminhos de cleanup** — sem eles, um crash deixa o usuário sem internet.

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
backend em `backend/migrations/` (`001_initial`, `002_parental_fixes`, `003_email_verification`).

## Convenções e armadilhas

- **`unwrap()` é proibido na regra de negócio** (backend) — propague erro via `AppError`/`?`.
- **Status real vence o doc**: para o que está implementado vs. aberto, confie em
  [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) (estado atual), [docs/RUNBOOK.md](docs/RUNBOOK.md)
  (limitações conhecidas), [docs/DECISOES_E_ROADMAP.md](docs/DECISOES_E_ROADMAP.md) (o que é
  proposta/roadmap) e no código. `ARCHITECTURE`/`API`/`RUNBOOK` descrevem só o estado atual.
- **`mobile/` é parcial** (v0.2): o **bloqueio de sites por DNS** está implementado e testado
  (engine nativo em `mobile/android/.../vpn/`: `DnsVpnService`, `DnsPacket`, `DomainMatcher`,
  `DnsForwarder`; testes JVM + instrumentados no emulador). Ainda **pendentes**: bloqueio de
  apps (`AppBlockerService` só detecta), filtro de conteúdo adulto, e cache SQLCipher Dart.
  Não assuma que o que está pendente funciona.
- **Mudou um modelo? Mude em `shared/`**: `User`, `Device`, `BlockedItem` etc. vivem na crate
  `shared` e são reusados por backend e desktop; o frontend espelha em `desktop/src/lib/types.ts`.

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
