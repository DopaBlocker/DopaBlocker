use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Extension, Json, Router,
};

use crate::errors::AppError;
use crate::middleware::{
    extract_bearer_token, verify_firebase_jwt_token, AuthUser, FirebaseClaims,
};
use crate::models::{RegisterRequest, User};
use crate::services::user_service;
use crate::AppState;

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
}

pub fn protected_router() -> Router<AppState> {
    Router::new().route("/auth/me", get(me))
}

fn resolve_registration_identity(
    claims: &FirebaseClaims,
    payload: &RegisterRequest,
) -> Result<(String, String), AppError> {
    let body_email = payload.email.trim();
    let claim_email = claims
        .email
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let email = match claim_email {
        Some(claim_email) => {
            if !body_email.is_empty() && !body_email.eq_ignore_ascii_case(claim_email) {
                return Err(AppError::BadRequest(
                    "O email informado nao corresponde ao email autenticado no Firebase".into(),
                ));
            }
            claim_email.to_string()
        }
        None if !body_email.is_empty() => body_email.to_string(),
        None => {
            return Err(AppError::BadRequest(
                "Nao foi possivel determinar o email autenticado".into(),
            ));
        }
    };

    let display_name = claims
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            let body_name = payload.display_name.trim();
            (!body_name.is_empty()).then(|| body_name.to_string())
        })
        .unwrap_or_else(|| fallback_display_name(&email));

    Ok((email, display_name))
}

fn fallback_display_name(email: &str) -> String {
    email
        .split('@')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Usuario")
        .to_string()
}

async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<User>, AppError> {
    let token = extract_bearer_token(&headers)?;
    let claims = verify_firebase_jwt_token(&state, &token).await?;

    if let Some(existing) =
        user_service::get_user_by_firebase_uid(&state.db, claims.sub.clone()).await?
    {
        return Err(AppError::Conflict(format!(
            "Usuario ja registrado: {}",
            existing.email
        )));
    }

    let (email, display_name) = resolve_registration_identity(&claims, &payload)?;

    let user =
        user_service::create_user(&state.db, claims.sub, email, display_name, payload.mode)
            .await?;
    Ok(Json(user))
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<User>, AppError> {
    let token = extract_bearer_token(&headers)?;
    let claims = verify_firebase_jwt_token(&state, &token).await?;

    let user = user_service::get_user_by_firebase_uid(&state.db, claims.sub.clone())
        .await?
        .ok_or_else(|| {
            AppError::NotFound("Usuario nao registrado localmente - chame /auth/register".into())
        })?;

    Ok(Json(user))
}

async fn me(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<User>, AppError> {
    let user = user_service::get_user_by_id(&state.db, auth.user_id).await?;
    Ok(Json(user))
}

#[cfg(test)]
mod tests {
    use super::{fallback_display_name, resolve_registration_identity};
    use crate::middleware::FirebaseClaims;
    use crate::models::{BlockMode, RegisterRequest};

    fn payload(email: &str, display_name: &str) -> RegisterRequest {
        RegisterRequest {
            email: email.into(),
            display_name: display_name.into(),
            mode: BlockMode::Personal,
        }
    }

    fn claims(email: Option<&str>, name: Option<&str>) -> FirebaseClaims {
        FirebaseClaims {
            sub: "firebase-uid".into(),
            email: email.map(str::to_string),
            name: name.map(str::to_string),
        }
    }

    #[test]
    fn rejects_mismatched_email_between_body_and_claims() {
        let result = resolve_registration_identity(
            &claims(Some("firebase@example.com"), Some("Firebase Name")),
            &payload("body@example.com", "Body Name"),
        );

        assert!(matches!(result, Err(crate::errors::AppError::BadRequest(_))));
    }

    #[test]
    fn prefers_claim_identity_when_available() {
        let result = resolve_registration_identity(
            &claims(Some("firebase@example.com"), Some("Firebase Name")),
            &payload("firebase@example.com", "Body Name"),
        )
        .expect("claims should win");

        assert_eq!(result.0, "firebase@example.com");
        assert_eq!(result.1, "Firebase Name");
    }

    #[test]
    fn falls_back_to_body_name_when_claim_name_is_missing() {
        let result = resolve_registration_identity(
            &claims(Some("firebase@example.com"), None),
            &payload("firebase@example.com", "Body Name"),
        )
        .expect("body name should be used");

        assert_eq!(result.1, "Body Name");
    }

    #[test]
    fn derives_display_name_from_email_when_needed() {
        let result = resolve_registration_identity(
            &claims(Some("focus.user@example.com"), None),
            &payload("focus.user@example.com", ""),
        )
        .expect("email fallback should work");

        assert_eq!(result.1, "focus.user");
        assert_eq!(fallback_display_name("focus.user@example.com"), "focus.user");
    }
}
