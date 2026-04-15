use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Enum de erros centralizado da Aplicação.
/// Qualquer serviço pode retornar um `AppError` e ele será convertido automaticamente em Resposta HTTP.
#[derive(Debug)]
pub enum AppError {
    InternalServerError(String),
    BadRequest(String),
    Unauthorized(String),
    NotFound(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
        };

        let body = Json(json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}

// Permite a conversão automática de erros genéricos (como io::Error) em AppError interno
impl<E> From<E> for AppError
where
    E: std::error::Error,
{
    fn from(err: E) -> Self {
        AppError::InternalServerError(err.to_string())
    }
}
