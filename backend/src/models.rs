use serde::{Deserialize, Serialize};

// Re-exports dos modelos universais criados em dopablocker_shared!
pub use dopablocker_shared::models::{
    AdultFilterSettings, BlockMode, BlockedItem, BlockedType, Device, LinkStatus, ParentalLink,
    Platform, User,
};

// =====================================================================
// DTOs (Data Transfer Objects) / Payloads da API
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub display_name: String,
    pub firebase_uid: String, 
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub message: String,
    pub user: User,
}

#[derive(Debug, Deserialize)]
pub struct CreateDeviceRequest {
    pub device_name: String,
    pub platform: Platform,
    pub is_child: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateBlockedItemRequest {
    pub item_type: BlockedType,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct LinkChildRequest {
    pub link_code: String,
}

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub message: String,
}
