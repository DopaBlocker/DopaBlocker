// =============================================================================
// `AppError` — tipo único de erro da aplicação, conversível em resposta HTTP.
// =============================================================================
// Todo handler do Axum retorna `Result<Json<T>, AppError>`. Quando um
// handler devolve `Err(AppError::...)`, o Axum chama `into_response()` por
// nós e transforma em um JSON com status code apropriado.
//
// Formato de resposta uniforme (body):
//     { "error": "mensagem legível em português" }
//
// Mapeamento status → variant:
//     400 Bad Request       → BadRequest(msg)
//     401 Unauthorized      → Unauthorized(msg)
//     403 Forbidden         → Forbidden(msg)
//     404 Not Found         → NotFound(msg)
//     409 Conflict          → Conflict(msg)
//     500 Internal Error    → InternalServerError(msg)
//
// O `impl<E: Error> From<E> for AppError` no final permite usar `?` em
// qualquer função que retorne `Result<_, ErrorQualquer>`: o erro é
// automaticamente embrulhado como InternalServerError. Isso é útil pra
// IO/SQL/rede — coisas que raramente têm tratamento específico útil.
// Quando precisamos de um erro específico (ex: 409), usamos `.map_err(|e| ...)`
// para traduzir explicitamente.
// =============================================================================

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    InternalServerError(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    ServiceUnavailable(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Destructura em (status, mensagem) e monta o JSON padrão.
        let (status, error_message) = match self {
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            AppError::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
        };

        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}

// Conversão automática de qualquer `std::error::Error` em `InternalServerError`.
// Usar com moderação: quando o erro tem semântica de negócio (duplicata,
// expiração, permissão), prefira `AppError::BadRequest/Conflict/...` explícito.
impl<E> From<E> for AppError
where
    E: std::error::Error,
{
    fn from(err: E) -> Self {
        AppError::InternalServerError(err.to_string())
    }
}
