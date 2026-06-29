// =============================================================================
// Feature de autenticação / conta.
// =============================================================================
// Rotas `/auth/*` + verificação de email + CRUD de user. Módulos:
//   - routes         : handlers HTTP (register/login/me/email-code/...).
//   - service        : criação/reclaim de conta (transações).
//   - email_code     : códigos/tokens de verificação de email (HMAC + DB).
//   - email_delivery : entrega dos códigos (SMTP real ou log de dev).
//   - user           : CRUD da tabela `users`.
//
// A validação de Firebase JWT em si vive em `crate::core::auth`; aqui mora a
// verificação por código que precede o `POST /auth/register`.
// =============================================================================

pub mod email_code;
pub mod email_delivery;
pub mod routes;
pub mod service;
pub mod user;

pub use routes::{protected_router, public_router};
