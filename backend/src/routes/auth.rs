use axum::{routing::post, Json, Router};
use crate::errors::AppError;
use crate::models::{CreateUserRequest, UserResponse};
use crate::services::user_service;

pub fn router() -> Router {
    Router::new().route("/register", post(register_handler))
}

async fn register_handler(
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let user = user_service::create_user(payload).await?;

    Ok(Json(UserResponse {
        message: "Usuário registrado com sucesso".into(),
        user,
    }))
}
