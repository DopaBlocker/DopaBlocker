use axum::{
    extract::State,
    http::HeaderMap,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};

use crate::core::auth::{
    extract_bearer_token, verify_firebase_jwt_token, AuthSource, AuthUser, FirebaseClaims,
};
use crate::core::errors::AppError;
use crate::core::models::{
    EmailCodeStartRequest, EmailCodeStartResponse, EmailCodeVerifyRequest, EmailCodeVerifyResponse,
    RegisterRequest, SuccessResponse, UpdateModeRequest, User,
};
use crate::AppState;

use super::{email_code, service, user};

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/auth/email-code/start", post(start_email_code))
        .route("/auth/email-code/verify", post(verify_email_code))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
}

pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/auth/me", get(me))
        .route("/auth/me", put(update_me))
        .route("/auth/me", delete(delete_me))
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

fn requires_email_verification_code(claims: &FirebaseClaims) -> bool {
    claims.sign_in_provider() == Some("password")
}

fn provider_is_already_verified(claims: &FirebaseClaims) -> bool {
    !requires_email_verification_code(claims) && claims.email_verified.unwrap_or(false)
}

/// Extrai o `email_verification_token` do body (provider `password`), exigindo
/// que esteja presente e não vazio. É a prova de posse do email usada tanto na
/// criação quanto no reclaim de conta.
fn require_email_verification_token(payload: &RegisterRequest) -> Result<String, AppError> {
    payload
        .email_verification_token
        .clone()
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
        .ok_or_else(|| {
            AppError::BadRequest("Verifique seu email antes de concluir o cadastro".into())
        })
}

async fn start_email_code(
    State(state): State<AppState>,
    Json(payload): Json<EmailCodeStartRequest>,
) -> Result<Json<EmailCodeStartResponse>, AppError> {
    let response =
        email_code::start_email_verification(&state.db, &state.config, payload.email).await?;
    Ok(Json(response))
}

async fn verify_email_code(
    State(state): State<AppState>,
    Json(payload): Json<EmailCodeVerifyRequest>,
) -> Result<Json<EmailCodeVerifyResponse>, AppError> {
    let response =
        email_code::verify_email_code(&state.db, &state.config, payload.email, payload.code)
            .await?;
    Ok(Json(response))
}

/// `POST /auth/register` — **idempotente e resiliente à identidade**. O nome é
/// histórico; na prática "garante que a conta deste login exista". Três casos:
///
///   1. Já existe conta para este `firebase_uid` → retorna-a (não é mais 409).
///      Torna o cadastro repetível sem erro e simplifica o retry do frontend.
///   2. Não existe para o uid, mas existe uma conta com este **email** (presa a
///      um `firebase_uid` antigo/diferente — conta órfã) → **reclaim**: reassocia
///      a conta a este uid, com a mesma prova de posse de email já exigida no
///      cadastro. Isso destrava o email (antes ele ficava preso pelo UNIQUE).
///   3. Nada existe → cria a conta normalmente.
///
/// A prova de posse do email é: provider `password` → `email_verification_token`
/// (código de 6 dígitos); provider verificado (Google) → claim `email_verified`.
async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<User>, AppError> {
    let token = extract_bearer_token(&headers)?;
    let claims = verify_firebase_jwt_token(&state, &token).await?;

    // (1) Idempotência: conta já existe para este firebase_uid.
    if let Some(existing) =
        user::get_user_by_firebase_uid(&state.db, claims.sub.clone()).await?
    {
        return Ok(Json(existing));
    }

    let (email, display_name) = resolve_registration_identity(&claims, &payload)?;

    // (2) Reclaim: existe conta com este email, presa a outro firebase_uid.
    if user::get_user_by_email(&state.db, email.clone())
        .await?
        .is_some()
    {
        let user = if requires_email_verification_code(&claims) {
            let email_verification_token = require_email_verification_token(&payload)?;
            service::reclaim_user_with_email_verification(
                &state.db,
                &state.config,
                claims.sub,
                email,
                email_verification_token,
            )
            .await?
        } else {
            if !provider_is_already_verified(&claims) {
                return Err(AppError::BadRequest(
                    "Email do provedor de login nao esta verificado".into(),
                ));
            }
            user::rebind_firebase_uid(&state.db, email, claims.sub).await?
        };
        return Ok(Json(user));
    }

    // (3) Conta nova.
    let user = if requires_email_verification_code(&claims) {
        let email_verification_token = require_email_verification_token(&payload)?;
        service::create_user_with_email_verification(
            &state.db,
            &state.config,
            claims.sub,
            email,
            display_name,
            payload.mode,
            email_verification_token,
        )
        .await?
    } else {
        if !provider_is_already_verified(&claims) {
            return Err(AppError::BadRequest(
                "Email do provedor de login nao esta verificado".into(),
            ));
        }

        user::create_user(&state.db, claims.sub, email, display_name, payload.mode).await?
    };
    Ok(Json(user))
}

async fn login(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<User>, AppError> {
    let token = extract_bearer_token(&headers)?;
    let claims = verify_firebase_jwt_token(&state, &token).await?;

    let user = user::get_user_by_firebase_uid(&state.db, claims.sub.clone())
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
    let user = user::get_user_by_id(&state.db, auth.user_id).await?;
    Ok(Json(user))
}

/// `DELETE /auth/me` — exclusao definitiva da conta. Apenas Firebase JWT;
/// Device Tokens (filhos) sao rejeitados. O service apaga `users` e cascateia
/// para `devices`, `blocked_items`, `parental_links`, `adult_filter_settings`
/// e `device_tokens`. Tambem limpa `email_verifications` pelo email do user.
///
/// O frontend e responsavel por tambem chamar `firebase.deleteUser()` —
/// o backend nao integra com Firebase Admin SDK no v0.1.
async fn delete_me(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<SuccessResponse>, AppError> {
    if auth.source != AuthSource::Firebase {
        return Err(AppError::Forbidden(
            "Apenas o titular da conta Firebase pode excluir".into(),
        ));
    }
    user::delete_user(&state.db, auth.user_id).await?;
    Ok(Json(SuccessResponse {
        message: "Conta excluida".into(),
    }))
}

/// `PUT /auth/me` — troca o modo da conta (personal↔parental) sem recriá-la.
/// Apenas Firebase JWT (o titular); Device Tokens (filhos) são rejeitados — o
/// middleware já barra PUT de Device Token com 403, e aqui reforçamos por
/// defesa em profundidade. Trocar de modo NÃO mexe nos vínculos de filhos nem na
/// blocklist; a regra "pai imune" passa a valer (ou deixa de valer) no próximo
/// sync. Permitido mesmo com filhos vinculados — a UI confirma quando há filhos.
async fn update_me(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<UpdateModeRequest>,
) -> Result<Json<User>, AppError> {
    if auth.source != AuthSource::Firebase {
        return Err(AppError::Forbidden(
            "Apenas o titular da conta Firebase pode trocar o modo".into(),
        ));
    }
    let user = user::update_mode(&state.db, auth.user_id, payload.mode).await?;
    Ok(Json(user))
}

#[cfg(test)]
mod tests {
    use super::{
        fallback_display_name, provider_is_already_verified, require_email_verification_token,
        requires_email_verification_code, resolve_registration_identity,
    };
    use crate::core::auth::{FirebaseAuthInfo, FirebaseClaims};
    use crate::core::models::{BlockMode, RegisterRequest};

    fn payload(email: &str, display_name: &str) -> RegisterRequest {
        RegisterRequest {
            email: email.into(),
            display_name: display_name.into(),
            mode: BlockMode::Personal,
            email_verification_token: None,
        }
    }

    fn claims(email: Option<&str>, name: Option<&str>) -> FirebaseClaims {
        FirebaseClaims {
            sub: "firebase-uid".into(),
            email: email.map(str::to_string),
            name: name.map(str::to_string),
            email_verified: Some(true),
            firebase: None,
        }
    }

    fn claims_with_provider(provider: &str, email_verified: bool) -> FirebaseClaims {
        FirebaseClaims {
            sub: "firebase-uid".into(),
            email: Some("firebase@example.com".into()),
            name: Some("Firebase Name".into()),
            email_verified: Some(email_verified),
            firebase: Some(FirebaseAuthInfo {
                sign_in_provider: Some(provider.into()),
            }),
        }
    }

    #[test]
    fn rejects_mismatched_email_between_body_and_claims() {
        let result = resolve_registration_identity(
            &claims(Some("firebase@example.com"), Some("Firebase Name")),
            &payload("body@example.com", "Body Name"),
        );

        assert!(matches!(
            result,
            Err(crate::core::errors::AppError::BadRequest(_))
        ));
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
        assert_eq!(
            fallback_display_name("focus.user@example.com"),
            "focus.user"
        );
    }

    #[test]
    fn require_email_verification_token_extracts_and_trims() {
        let mut p = payload("focus@example.com", "Focus");
        p.email_verification_token = Some("  tok-123  ".into());
        assert_eq!(
            require_email_verification_token(&p).expect("token presente"),
            "tok-123"
        );
    }

    #[test]
    fn require_email_verification_token_rejects_missing_or_blank() {
        let mut p = payload("focus@example.com", "Focus");
        p.email_verification_token = None;
        assert!(require_email_verification_token(&p).is_err());

        p.email_verification_token = Some("   ".into());
        assert!(require_email_verification_token(&p).is_err());
    }

    #[test]
    fn password_provider_requires_email_code() {
        let claims = claims_with_provider("password", false);

        assert!(requires_email_verification_code(&claims));
        assert!(!provider_is_already_verified(&claims));
    }

    #[test]
    fn verified_google_provider_does_not_require_email_code() {
        let claims = claims_with_provider("google.com", true);

        assert!(!requires_email_verification_code(&claims));
        assert!(provider_is_already_verified(&claims));
    }
}
