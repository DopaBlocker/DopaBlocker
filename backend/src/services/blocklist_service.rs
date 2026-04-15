use crate::errors::AppError;
use crate::models::{BlockedItem, CreateBlockedItemRequest};

pub async fn add_item(user_id: &str, payload: CreateBlockedItemRequest) -> Result<BlockedItem, AppError> {
    // MOCK
    Ok(BlockedItem {
        id: "mock-item-id".into(),
        user_id: user_id.into(),
        item_type: payload.item_type,
        value: payload.value,
        is_active: true,
        created_at: "2026-04-14T12:00:00Z".into(),
    })
}

pub async fn list_items(user_id: &str) -> Result<Vec<BlockedItem>, AppError> {
    // MOCK
    Ok(vec![])
}
