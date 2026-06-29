// =============================================================================
// device_event_service — registro e leitura de eventos de adulteração (tamper).
// =============================================================================
// Suporta o controle parental "observável" no mobile (C2.1/C2.2): sem root, o
// filho consegue desligar a VPN / abrir as Configs de VPN/DNS. Não dá para
// IMPEDIR, mas dá para o app nativo REPORTAR o evento ao backend, e o pai vê.
//
// Autenticação invertida de propósito: o filho manda o próprio Device Token NO
// CORPO (rota pública `POST /devices/tamper`), e validamos aqui. Assim a regra
// "Device Token só faz GET/HEAD" do middleware continua valendo sem exceções —
// esta rota fica fora do `require_auth`, exatamente como `/devices/link/confirm`.
// =============================================================================

use rusqlite::{params, OptionalExtension};
use tokio_rusqlite::Connection;
use uuid::Uuid;

use crate::core::auth::hash_device_token;
use crate::core::errors::AppError;
use crate::core::models::DeviceEvent;
use crate::core::util::iso_now;

/// Tipos de evento aceitos. Mantido em sincronia com o `CHECK` da migration 004
/// e com os `kind` que o app nativo Android envia.
const VALID_KINDS: &[&str] = &["vpn_revoked", "vpn_settings_opened", "dns_settings_opened"];

fn is_valid_kind(kind: &str) -> bool {
    VALID_KINDS.contains(&kind)
}

/// Remove espaços e o prefixo opcional `dt_` do token recebido no corpo —
/// o app pode mandar com ou sem o prefixo. O hash é calculado sobre o plain
/// SEM prefixo, alinhado com o que o middleware faz no `strip_prefix("dt_")`.
fn normalize_device_token(raw: &str) -> String {
    let trimmed = raw.trim();
    trimmed.strip_prefix("dt_").unwrap_or(trimmed).to_string()
}

/// Registra um evento de adulteração reportado pelo device do filho. Valida o
/// Device Token (hash → lookup ativo) e o `kind`. Token inválido/revogado → 401.
pub async fn record_tamper(
    db: &Connection,
    device_token_raw: &str,
    kind: &str,
) -> Result<(), AppError> {
    let kind = kind.trim().to_string();
    if !is_valid_kind(&kind) {
        return Err(AppError::BadRequest("tipo de evento inválido".into()));
    }

    let hash = hash_device_token(&normalize_device_token(device_token_raw));
    let id = Uuid::new_v4().to_string();
    let now = iso_now();

    let inserted = db
        .call(move |c| {
            // Resolve user_id/device_id a partir do token ativo (não revogado).
            let row: Option<(String, String)> = c
                .query_row(
                    "SELECT user_id, device_id FROM device_tokens
                     WHERE token_hash = ?1 AND revoked_at IS NULL",
                    params![hash],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                )
                .optional()?;

            let (user_id, device_id) = match row {
                Some(values) => values,
                None => return Ok(false),
            };

            c.execute(
                "INSERT INTO device_events(id, user_id, device_id, kind, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, user_id, device_id, kind, now],
            )?;
            Ok(true)
        })
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    if !inserted {
        return Err(AppError::Unauthorized(
            "Device token inválido ou revogado".into(),
        ));
    }
    Ok(())
}

/// Lista os eventos de tamper de um pai (todos os filhos da conta), mais
/// recentes primeiro. Limita a 100 para a tela de alertas não crescer sem fim.
pub async fn list_events(db: &Connection, user_id: String) -> Result<Vec<DeviceEvent>, AppError> {
    db.call(move |c| {
        let mut stmt = c.prepare(
            "SELECT id, user_id, device_id, kind, created_at, acknowledged_at
             FROM device_events WHERE user_id = ?1
             ORDER BY created_at DESC LIMIT 100",
        )?;
        let rows = stmt
            .query_map(params![user_id], |row| {
                Ok(DeviceEvent {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    device_id: row.get(2)?,
                    kind: row.get(3)?,
                    created_at: row.get(4)?,
                    acknowledged_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{is_valid_kind, normalize_device_token};

    #[test]
    fn validates_known_event_kinds() {
        assert!(is_valid_kind("vpn_revoked"));
        assert!(is_valid_kind("vpn_settings_opened"));
        assert!(is_valid_kind("dns_settings_opened"));
        assert!(!is_valid_kind("bogus"));
        assert!(!is_valid_kind(""));
    }

    #[test]
    fn strips_dt_prefix_and_whitespace_from_token() {
        assert_eq!(normalize_device_token("  dt_abc123 "), "abc123");
        assert_eq!(normalize_device_token("abc123"), "abc123");
    }
}
