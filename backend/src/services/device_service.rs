use crate::errors::AppError;
use crate::models::{CreateDeviceRequest, Device, ParentalLink, LinkStatus};

pub async fn register_device(user_id: &str, payload: CreateDeviceRequest) -> Result<Device, AppError> {
    // MOCK
    Ok(Device {
        id: "mock-device-id".into(),
        user_id: user_id.into(),
        device_name: payload.device_name,
        platform: payload.platform,
        is_child: payload.is_child,
        created_at: "2026-04-14T12:00:00Z".into(),
    })
}

pub async fn link_child_device(parent_user_id: &str, child_code: &str) -> Result<ParentalLink, AppError> {
    // MOCK logic to search pending link by code
    Ok(ParentalLink {
        id: "mock-link-id".into(),
        parent_device_id: "parent-dev-id".into(),
        child_device_id: Some("child-dev-id".into()),
        link_code: child_code.into(),
        status: LinkStatus::Active,
        expires_at: "2026-04-14T12:05:00Z".into(),
        created_at: "2026-04-14T12:00:00Z".into(),
    })
}
