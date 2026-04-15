use crate::errors::AppError;
use crate::models::{BlockMode, CreateUserRequest, User};

pub async fn create_user(payload: CreateUserRequest) -> Result<User, AppError> {
    // MOCK: Futuramente aqui entra SQLx/Firebase.
    // Inserir registro no Banco.

    let new_user = User {
        id: "mock-user-id-123".into(),
        firebase_uid: payload.firebase_uid,
        email: payload.email,
        display_name: payload.display_name,
        mode: BlockMode::Personal,
        created_at: "2026-04-14T12:00:00Z".into(),
    };

    Ok(new_user)
}
