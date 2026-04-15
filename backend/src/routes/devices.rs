use axum::{routing::post, Extension, Json, Router};
use crate::errors::AppError;
use crate::middleware::CurrentUser;
use crate::models::{CreateDeviceRequest, Device, LinkChildRequest, ParentalLink};
use crate::services::device_service;

pub fn router() -> Router {
    Router::new()
        .route("/", post(register_device_handler))
        .route("/link", post(link_child_handler))
}

async fn register_device_handler(
    Extension(user): Extension<CurrentUser>,
    Json(payload): Json<CreateDeviceRequest>,
) -> Result<Json<Device>, AppError> {
    let device = device_service::register_device(&user.id, payload).await?;
    Ok(Json(device))
}

async fn link_child_handler(
    Extension(user): Extension<CurrentUser>,
    Json(payload): Json<LinkChildRequest>,
) -> Result<Json<ParentalLink>, AppError> {
    let link = device_service::link_child_device(&user.id, &payload.link_code).await?;
    Ok(Json(link))
}
