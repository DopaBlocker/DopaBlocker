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

use axum::{
    extract::{Request, State},
    http::{header, Method, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::AppState;

use super::device_token::validate_device_token;
use super::jwt::validate_firebase_jwt;

/// Qual foi a fonte da autenticação. Alguns handlers precisam distinguir
/// (ex: `/devices/link/generate` só aceita Firebase — ver features/devices/routes.rs).
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
