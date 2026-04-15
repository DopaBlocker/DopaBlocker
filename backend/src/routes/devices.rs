use axum::{
    extract::State,
    routing::{get, post},
    Extension, Json, Router,
};

use crate::errors::AppError;
use crate::middleware::{AuthSource, AuthUser};
use crate::models::{
    ConfirmLinkRequest, ConfirmLinkResponse, Device, GenerateLinkCodeResponse,
    RegisterDeviceRequest,
};
use crate::services::device_service;
use crate::AppState;

/// Rotas de devices que exigem autenticação (JWT ou Device Token).
pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register_device))
        .route("/", get(list_devices))
        .route("/link/generate", post(generate_link_code))
}

/// Rotas públicas: apenas /devices/link/confirm (filho ainda não tem credencial).
pub fn public_router() -> Router<AppState> {
    Router::new().route("/devices/link/confirm", post(confirm_link))
}

async fn register_device(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<RegisterDeviceRequest>,
) -> Result<Json<Device>, AppError> {
    let device = device_service::register_device(&state.db, auth.user_id, payload).await?;
    Ok(Json(device))
}

async fn list_devices(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<Device>>, AppError> {
    let devices = device_service::list_devices(&state.db, auth.user_id).await?;
    Ok(Json(devices))
}

/// Apenas contas Firebase (pai) podem gerar códigos. Device tokens são rejeitados.
async fn generate_link_code(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<GenerateLinkCodeResponse>, AppError> {
    if auth.source != AuthSource::Firebase {
        return Err(AppError::Forbidden(
            "Apenas contas Firebase podem gerar códigos de vinculação".into(),
        ));
    }
    let resp = device_service::generate_link_code(&state.db, auth.user_id).await?;
    Ok(Json(resp))
}

async fn confirm_link(
    State(state): State<AppState>,
    Json(payload): Json<ConfirmLinkRequest>,
) -> Result<Json<ConfirmLinkResponse>, AppError> {
    let resp = device_service::confirm_link(&state.db, payload).await?;
    Ok(Json(resp))
}
