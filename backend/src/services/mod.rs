// =============================================================================
// Camada de serviços — toda a lógica de domínio que os handlers chamam.
// =============================================================================
// A arquitetura do backend segue três camadas bem definidas:
//
//   routes/*    → parsing de request/response, extração de auth, validações
//                 superficiais de payload. NÃO conversa com o banco.
//   services/*  → regras de negócio + SQL. Recebem tipos já parseados e
//                 retornam os models do domínio. É aqui que mora a lógica
//                 que seria idêntica se trocássemos Axum por qualquer outro
//                 framework web.
//   db.rs       → abertura da conexão + migrations. Não tem lógica de negócio.
//
// Cada submódulo agrupa operações de uma entidade:
//   - user_service      → CRUD de User.
//   - blocklist_service → items bloqueados + filtro adulto.
//   - device_service    → devices + fluxo completo de vinculação parental.
//   - auth_service      → fluxo de verificação de email (start/verify/consume)
//                         + envio SMTP. A validação de Firebase JWT em si vive
//                         em `middleware.rs`; aqui mora apenas a verificação
//                         por código que precede o `POST /auth/register`.
//   - util              → helpers compartilhados (ServiceError tipado, ISO-8601,
//                         conversões enum ↔ texto SQLite).
// =============================================================================

pub mod auth_service;
pub mod blocklist_service;
pub mod device_service;
pub mod user_service;
pub mod util;
