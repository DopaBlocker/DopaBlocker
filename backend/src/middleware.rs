// =============================================================================
// Middleware de autenticação dual: Firebase JWT + Device Token
// =============================================================================
// Este é o coração da autenticação do DopaBlocker. Ele resolve um único
// problema: transformar o header `Authorization: Bearer <algo>` em um
// `AuthUser { user_id, source, device_id }` que os handlers possam usar.
//
// Dois tipos de credencial são aceitos, diferenciados pelo prefixo:
//
//   1. Firebase JWT (sem prefixo)
//        Authorization: Bearer eyJhbGci...
//        Usado por: contas Pessoal e Pais.
//        Validação: baixa chaves públicas do Google, verifica assinatura
//                   RS256, checa iss/aud/exp, extrai `sub` (firebase_uid),
//                   faz lookup em `users` para obter o user_id local.
//
//   2. Device Token (prefixo "dt_")
//        Authorization: Bearer dt_a1b2c3d4...
//        Usado por: devices filhos, que não têm conta Firebase.
//        Validação: remove "dt_", calcula SHA-256, faz lookup em
//                   `device_tokens WHERE token_hash=? AND revoked_at IS NULL`.
//
// Regra importante de segurança: Device Tokens são READ-ONLY. Qualquer
// POST/DELETE/PUT feito com um token `dt_` é rejeitado com 403 ANTES
// do handler ser chamado. Isso garante que um filho nunca consegue
// modificar a blocklist ou gerar códigos de vinculação, mesmo que
// algum handler esqueça de checar `source`.
// =============================================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    extract::{Request, State},
    http::{header, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use rusqlite::params;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use crate::errors::AppError;
use crate::AppState;

/// Qual foi a fonte da autenticação. Alguns handlers precisam distinguir
/// (ex: `/devices/link/generate` só aceita Firebase — ver routes/devices.rs).
#[derive(Clone, Debug, PartialEq)]
pub enum AuthSource {
    Firebase,
    DeviceToken,
}

/// Identidade do chamador, já resolvida pelo middleware. Injetada no
/// request via `Extensions` e extraída nos handlers com `Extension<AuthUser>`.
#[derive(Clone, Debug)]
#[allow(dead_code)] // `device_id` é preenchido mas nem todos os handlers usam (ainda).
pub struct AuthUser {
    /// ID local do usuário (UUID em `users.id`). Sempre presente, seja qual
    /// for a origem do token. No caso de Device Token, é o user_id do PAI.
    pub user_id: String,
    pub source: AuthSource,
    /// Preenchido apenas em autenticação via Device Token (o `devices.id`
    /// do filho). `None` em autenticação Firebase.
    pub device_id: Option<String>,
}

// -----------------------------------------------------------------------------
// JWKS cache — chaves públicas do Firebase
// -----------------------------------------------------------------------------
// O Firebase assina JWTs com chaves RSA que rotacionam periodicamente.
// Para validar a assinatura, precisamos da chave pública correspondente
// ao `kid` que o JWT carrega no header.
//
// Este endpoint retorna um JSON `{ "kid1": "PEM...", "kid2": "PEM..." }`:
const JWKS_URL: &str =
    "https://www.googleapis.com/robot/v1/metadata/x509/securetoken@system.gserviceaccount.com";
const JWKS_FETCH_TIMEOUT: Duration = Duration::from_secs(10);

// Tempo de vida do cache. O Google rotaciona chaves a cada ~24h, mas o
// header `Cache-Control` da resposta é mais curto. 6h é um meio-termo
// seguro: muitas validações cached, sem risco de usar chave muito antiga.
const JWKS_TTL_SECS: u64 = 6 * 60 * 60;

/// Cache de chaves públicas do Firebase, thread-safe via `RwLock`.
/// Criado uma vez no `main` e compartilhado via `Arc` no `AppState`.
#[derive(Default)]
pub struct JwksCache {
    inner: RwLock<Option<CachedJwks>>,
}

struct CachedJwks {
    fetched_at: Instant,
    keys: HashMap<String, DecodingKey>,
}

impl JwksCache {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Retorna a chave com o `kid` dado. Se o cache estiver fresco e
    /// contiver o kid, retorna imediatamente. Senão, baixa a lista nova
    /// do Google, atualiza o cache, e tenta de novo.
    ///
    /// Não faz retry se o kid ainda não estiver presente após o refetch
    /// — isso significaria que o JWT usa uma chave desconhecida (JWT forjado
    /// ou deprecado) e deve falhar como Unauthorized.
    async fn get(&self, kid: &str) -> Result<DecodingKey, AppError> {
        // Tentativa 1: ler do cache (lock compartilhado, múltiplos leitores ok).
        {
            let guard = self.inner.read().await;
            if let Some(cached) = guard.as_ref() {
                if cached.fetched_at.elapsed() < Duration::from_secs(JWKS_TTL_SECS) {
                    if let Some(k) = cached.keys.get(kid) {
                        return Ok(k.clone());
                    }
                }
            }
        }

        // Cache frio, expirado ou sem o kid → vai buscar.
        let fresh = fetch_jwks_timed().await?;
        let key = fresh
            .get(kid)
            .cloned()
            .ok_or_else(|| AppError::Unauthorized("kid do JWT não encontrado no JWKS".into()))?;

        // Substitui o cache (lock exclusivo).
        let mut guard = self.inner.write().await;
        *guard = Some(CachedJwks {
            fetched_at: Instant::now(),
            keys: fresh,
        });
        Ok(key)
    }
}

/// Baixa e parseia o JWKS do Google. Cada PEM é convertido em `DecodingKey`
/// reutilizável para a validação de assinatura.
#[allow(dead_code)]
async fn fetch_jwks() -> Result<HashMap<String, DecodingKey>, AppError> {
    let resp: HashMap<String, String> = reqwest::get(JWKS_URL)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Falha ao buscar JWKS: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::InternalServerError(format!("JWKS inválido: {e}")))?;

    let mut out = HashMap::new();
    for (kid, pem) in resp {
        let key = DecodingKey::from_rsa_pem(pem.as_bytes())
            .map_err(|e| AppError::InternalServerError(format!("PEM inválido: {e}")))?;
        out.insert(kid, key);
    }
    Ok(out)
}

async fn fetch_jwks_timed() -> Result<HashMap<String, DecodingKey>, AppError> {
    let client = reqwest::Client::builder()
        .timeout(JWKS_FETCH_TIMEOUT)
        .build()
        .map_err(|e| AppError::InternalServerError(format!("Falha ao criar client JWKS: {e}")))?;

    let resp: HashMap<String, String> = client
        .get(JWKS_URL)
        .send()
        .await
        .map_err(|e| AppError::InternalServerError(format!("Falha ao buscar JWKS: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::InternalServerError(format!("JWKS invÃ¡lido: {e}")))?;

    let mut out = HashMap::new();
    for (kid, pem) in resp {
        let key = DecodingKey::from_rsa_pem(pem.as_bytes())
            .map_err(|e| AppError::InternalServerError(format!("PEM invÃ¡lido: {e}")))?;
        out.insert(kid, key);
    }
    Ok(out)
}

// -----------------------------------------------------------------------------
// Claims do Firebase JWT
// -----------------------------------------------------------------------------

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
    state: &crate::AppState,
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

// -----------------------------------------------------------------------------
// Middleware principal `require_auth`
// -----------------------------------------------------------------------------

/// Roda antes de todo handler protegido. Fluxo:
///   1. Lê o header Authorization; se faltar, 401.
///   2. Inspeciona o prefixo do token:
///      - "dt_..." → valida como Device Token.
///      - qualquer outro → valida como Firebase JWT.
///   3. Se autenticação passou mas é Device Token e o método é de escrita
///      (POST/DELETE/PUT), rejeita com 403 antes do handler.
///   4. Injeta o `AuthUser` no request e chama o próximo layer.
pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_string();

    // Roteamento por prefixo. Nota: o `strip_prefix` já consome o "dt_",
    // então `plain` é o token sem o prefixo — que é o que vamos hashear.
    let auth_user = if let Some(plain) = auth_header.strip_prefix("dt_") {
        validate_device_token(&state, plain)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?
    } else {
        validate_firebase_jwt(&state, &auth_header)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?
    };

    // Enforcement read-only para Device Tokens. Feito aqui (não no handler)
    // para que a regra seja impossível de esquecer. GET e HEAD passam;
    // qualquer outro método é bloqueado.
    if auth_user.source == AuthSource::DeviceToken
        && !matches!(req.method(), &Method::GET | &Method::HEAD)
    {
        return Err(StatusCode::FORBIDDEN);
    }

    // O handler acessa isso via parâmetro `Extension<AuthUser>`.
    req.extensions_mut().insert(auth_user);
    Ok(next.run(req).await)
}

/// Caminho "Firebase JWT" do middleware: valida o token + faz lookup
/// do user local. Se o Firebase UID não tiver registro em `users`,
/// rejeita pedindo para chamar /auth/register antes.
async fn validate_firebase_jwt(state: &AppState, token: &str) -> Result<AuthUser, AppError> {
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

/// Calcula o SHA-256 hex-encoded de um token plain. Usado TANTO na criação
/// (para armazenar o hash) QUANTO na validação (para comparar). Exposta
/// publicamente porque `device_service::confirm_link` também precisa.
pub fn hash_device_token(plain: &str) -> String {
    let mut h = Sha256::new();
    h.update(plain.as_bytes());
    format!("{:x}", h.finalize())
}

/// Caminho "Device Token" do middleware. Lookup por `token_hash` filtrando
/// `revoked_at IS NULL` — um token revogado fica no banco (para auditoria)
/// mas não é aceito mais.
async fn validate_device_token(state: &AppState, plain: &str) -> Result<AuthUser, AppError> {
    let hash = hash_device_token(plain);
    let row: Option<(String, String)> = state
        .db
        .call(move |c| {
            let r = c
                .query_row(
                    "SELECT user_id, device_id FROM device_tokens
                     WHERE token_hash = ?1 AND revoked_at IS NULL",
                    params![hash],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                )
                .ok();
            Ok(r)
        })
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let (user_id, device_id) =
        row.ok_or_else(|| AppError::Unauthorized("Device token inválido ou revogado".into()))?;

    Ok(AuthUser {
        user_id,
        source: AuthSource::DeviceToken,
        device_id: Some(device_id),
    })
}
