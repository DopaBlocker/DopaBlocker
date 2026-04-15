use axum::{
    extract::{Path, State},
    routing::{delete, get, put},
    Extension, Json, Router,
};

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    AdultFilterSettings, AdultFilterToggleRequest, BlockedItem, CreateBlockedItemRequest,
    SuccessResponse,
};
use crate::services::blocklist_service;
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_items).post(add_item))
        .route("/:id", delete(delete_item))
        .route("/adult-filter", put(set_adult_filter))
}

async fn list_items(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<BlockedItem>>, AppError> {
    let items = blocklist_service::list_items(&state.db, auth.user_id).await?;
    Ok(Json(items))
}

async fn add_item(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<CreateBlockedItemRequest>,
) -> Result<Json<BlockedItem>, AppError> {
    let item = blocklist_service::add_item(&state.db, auth.user_id, payload).await?;
    Ok(Json(item))
}

async fn delete_item(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse>, AppError> {
    blocklist_service::delete_item(&state.db, auth.user_id, id).await?;
    Ok(Json(SuccessResponse {
        message: "Item removido".into(),
    }))
}

async fn set_adult_filter(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<AdultFilterToggleRequest>,
) -> Result<Json<AdultFilterSettings>, AppError> {
    let settings =
        blocklist_service::set_adult_filter(&state.db, auth.user_id, payload.enabled).await?;
    Ok(Json(settings))
}
