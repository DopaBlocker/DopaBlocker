// -----------------------------------------------------------------------------
// Device Token — hash e validação
// -----------------------------------------------------------------------------

use rusqlite::params;
use sha2::{Digest, Sha256};

use crate::core::errors::AppError;
use crate::AppState;

use super::middleware::{AuthSource, AuthUser};

/// Calcula o SHA-256 hex-encoded de um token plain. Usado TANTO na criação
/// (para armazenar o hash) QUANTO na validação (para comparar). Exposta
/// publicamente porque `features::devices::service::confirm_link` também precisa.
pub fn hash_device_token(plain: &str) -> String {
    let mut h = Sha256::new();
    h.update(plain.as_bytes());
    format!("{:x}", h.finalize())
}

/// Caminho "Device Token" do middleware. Lookup por `token_hash` filtrando
/// `revoked_at IS NULL` — um token revogado fica no banco (para auditoria)
/// mas não é aceito mais.
pub(crate) async fn validate_device_token(
    state: &AppState,
    plain: &str,
) -> Result<AuthUser, AppError> {
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
