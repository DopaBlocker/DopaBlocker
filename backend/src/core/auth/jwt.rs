// -----------------------------------------------------------------------------
// Claims do Firebase JWT + validação de assinatura
// -----------------------------------------------------------------------------

use axum::http::header;
use jsonwebtoken::{decode, decode_header, Algorithm, Validation};
use rusqlite::params;
use serde::Deserialize;

use crate::core::errors::AppError;
use crate::AppState;

use super::middleware::{AuthSource, AuthUser};

/// Subset dos claims que o Firebase emite. O único campo obrigatório
/// pra gente é o `sub` (Firebase UID); `email` e `name` vêm junto mas
/// são opcionais e não são usados hoje — mantidos para eventual uso
/// em auto-preenchimento de registro.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct FirebaseClaims {
    pub sub: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email_verified: Option<bool>,
    #[serde(default)]
    pub firebase: Option<FirebaseAuthInfo>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct FirebaseAuthInfo {
    #[serde(default)]
    pub sign_in_provider: Option<String>,
}

impl FirebaseClaims {
    pub fn sign_in_provider(&self) -> Option<&str> {
        self.firebase
            .as_ref()
            .and_then(|firebase| firebase.sign_in_provider.as_deref())
    }
}

/// Valida um Firebase JWT e retorna os claims. NÃO faz lookup em `users` —
/// use quando o user pode ainda não existir localmente (register/login).
///
/// Validações aplicadas:
///   - assinatura RS256 com a chave pública do Google (via JWKS cache)
///   - iss == "https://securetoken.google.com/<project_id>"
///   - aud == "<project_id>"
///   - exp > agora
pub async fn verify_firebase_jwt_token(
    state: &AppState,
    token: &str,
) -> Result<FirebaseClaims, AppError> {
    // `decode_header` não valida assinatura — só extrai o JSON do header
    // (primeira parte do JWT, antes do ponto) para pegar o `kid`.
    let header = decode_header(token)
        .map_err(|e| AppError::Unauthorized(format!("JWT header inválido: {e}")))?;
    let kid = header
        .kid
        .ok_or_else(|| AppError::Unauthorized("JWT sem kid".into()))?;
    let key = state.jwks.get(&kid).await?;

    // Validador configurado com as regras do Firebase. `jsonwebtoken`
    // checa `exp` por padrão; aqui adicionamos `aud` e `iss`.
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[&state.config.firebase_project_id]);
    validation.set_issuer(&[&format!(
        "https://securetoken.google.com/{}",
        state.config.firebase_project_id
    )]);

    let data = decode::<FirebaseClaims>(token, &key, &validation)
        .map_err(|e| AppError::Unauthorized(format!("JWT inválido: {e}")))?;

    Ok(data.claims)
}

/// Extrai o token do header `Authorization: Bearer <...>`. Usado pelos
/// handlers públicos (register/login) que precisam validar o JWT
/// manualmente (o middleware global não roda neles).
pub fn extract_bearer_token(req: &axum::http::HeaderMap) -> Result<String, AppError> {
    req.get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Unauthorized("Authorization ausente".into()))
}

/// Caminho "Firebase JWT" do middleware: valida o token + faz lookup
/// do user local. Se o Firebase UID não tiver registro em `users`,
/// rejeita pedindo para chamar /auth/register antes.
pub(crate) async fn validate_firebase_jwt(
    state: &AppState,
    token: &str,
) -> Result<AuthUser, AppError> {
    let claims = verify_firebase_jwt_token(state, token).await?;
    let uid = claims.sub.clone();

    // Lookup em `users` para obter o id local. `.ok()` trata "não achou"
    // como `None` em vez de erro.
    let user_id: Option<String> = state
        .db
        .call(move |c| {
            let r = c
                .query_row(
                    "SELECT id FROM users WHERE firebase_uid = ?1",
                    params![uid],
                    |r| r.get::<_, String>(0),
                )
                .ok();
            Ok(r)
        })
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let user_id = user_id.ok_or_else(|| {
        AppError::Unauthorized("Usuário não registrado localmente — chame /auth/register".into())
    })?;

    Ok(AuthUser {
        user_id,
        source: AuthSource::Firebase,
        device_id: None,
    })
}
