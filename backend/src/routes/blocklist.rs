use axum::{routing::{get, post}, Extension, Json, Router};
use crate::errors::AppError;
use crate::middleware::CurrentUser;
use crate::models::{BlockedItem, CreateBlockedItemRequest};
use crate::services::blocklist_service;

pub fn router() -> Router {
    Router::new()
        .route("/", post(add_item_handler))
        .route("/", get(list_items_handler))
}

async fn add_item_handler(
    Extension(user): Extension<CurrentUser>,
    Json(payload): Json<CreateBlockedItemRequest>,
) -> Result<Json<BlockedItem>, AppError> {
    let item = blocklist_service::add_item(&user.id, payload).await?;
    Ok(Json(item))
}

async fn list_items_handler(
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<Vec<BlockedItem>>, AppError> {
    let items = blocklist_service::list_items(&user.id).await?;
    Ok(Json(items))
}
