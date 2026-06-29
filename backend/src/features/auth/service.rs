// =============================================================================
// service — criação e reclaim de conta (orquestração + transações).
// =============================================================================
// Consome o `email_verification_token` (prova de posse do email, emitido por
// `email_code`) e cria/reassocia a conta numa transação. A normalização de
// email e o hash do token vêm de `email_code`.
// =============================================================================

use rusqlite::{params, OptionalExtension};
use tokio_rusqlite::Connection;
use uuid::Uuid;

use crate::core::config::AppConfig;
use crate::core::errors::AppError;
use crate::core::models::{BlockMode, User};
use crate::core::util::{block_mode_to_sql, iso_now, map_sqlite_error, parse_block_mode, ServiceError};

use super::email_code::{email_token_hash, normalize_email};

/// Especialização do `map_sqlite_error` para `create_user_with_email_verification`:
/// erros de UNIQUE constraint (firebase_uid duplicado) viram 409 Conflict, e os
/// demais (ServiceError tipado ou SQL puro) seguem o mapeamento padrão.
fn map_register_error(err: tokio_rusqlite::Error) -> AppError {
    let msg = err.to_string();
    if ServiceError::from_message(&msg).is_none() && msg.contains("UNIQUE") {
        AppError::Conflict(format!("Falha ao criar usuario: {msg}"))
    } else {
        map_sqlite_error(err)
    }
}

pub async fn create_user_with_email_verification(
    db: &Connection,
    config: &AppConfig,
    firebase_uid: String,
    email: String,
    display_name: String,
    mode: BlockMode,
    email_verification_token: String,
) -> Result<User, AppError> {
    let normalized_email = normalize_email(&email)?;
    let token = email_verification_token.trim();
    if token.is_empty() {
        return Err(AppError::BadRequest(
            "Verificacao de email ausente ou invalida".into(),
        ));
    }

    let token_hash = email_token_hash(&config.email_code_secret, token);
    let id = Uuid::new_v4().to_string();
    let now = iso_now();
    let mode_str = block_mode_to_sql(&mode).to_string();
    // Persistimos sempre a versao normalizada — mesmas regras (trim+lowercase)
    // aplicadas pelo `normalize_email`. Isto evita que `User@Example.COM` e
    // `user@example.com` virem registros distintos depois de uma alteracao
    // de provedor (Firebase pode mandar com case diferente em casos de borda).
    let email = normalized_email.clone();
    let display_name = display_name.trim().to_string();

    db.call(move |c| {
        let tx = c.transaction()?;

        let verification: Option<(String, String)> = tx
            .query_row(
                "SELECT id, token_expires_at
                 FROM email_verifications
                 WHERE email = ?1
                   AND token_hash = ?2
                   AND status = 'verified'
                   AND consumed_at IS NULL
                 ORDER BY verified_at DESC LIMIT 1",
                params![normalized_email, token_hash],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        let (verification_id, token_expires_at) =
            verification.ok_or_else(|| ServiceError::EmailTokenInvalid.into_sqlite())?;

        if token_expires_at.as_str() < now.as_str() {
            tx.execute(
                "UPDATE email_verifications
                 SET status = 'expired'
                 WHERE id = ?1",
                params![verification_id],
            )?;
            return Err(ServiceError::EmailTokenExpired.into_sqlite());
        }

        tx.execute(
            "INSERT INTO users(id, firebase_uid, email, display_name, mode)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, firebase_uid, email, display_name, mode_str],
        )?;

        tx.execute(
            "UPDATE email_verifications
             SET status = 'consumed', consumed_at = ?1
             WHERE id = ?2",
            params![now, verification_id],
        )?;

        let user = tx.query_row(
            "SELECT id, firebase_uid, email, display_name, mode, created_at
             FROM users WHERE id = ?1",
            params![id],
            |row| {
                Ok(User {
                    id: row.get(0)?,
                    firebase_uid: row.get(1)?,
                    email: row.get(2)?,
                    display_name: row.get(3)?,
                    mode: parse_block_mode(&row.get::<_, String>(4)?),
                    created_at: row.get(5)?,
                })
            },
        )?;

        tx.commit()?;

        Ok(user)
    })
    .await
    .map_err(map_register_error)
}

/// Reassocia (reclaim) uma conta existente — identificada pelo `email` já
/// verificado — a um novo `firebase_uid`. Usado pelo `/auth/register` quando o
/// vínculo `firebase_uid`→linha quebrou (ex.: conta Firebase recriada/apagada)
/// e o `email UNIQUE` prenderia o recadastro num beco sem saída.
///
/// Consome o MESMO `email_verification_token` exigido no cadastro como prova de
/// posse do email — não há vetor de roubo: o Firebase só emite token verificado
/// para quem controla o email, e se o `firebase_uid` original ainda existisse o
/// dono entraria por ele (login 200) sem cair aqui.
///
/// Preserva `mode`, `id` e `created_at` da conta original; troca apenas o
/// `firebase_uid`. Espelha `create_user_with_email_verification`, mas faz UPDATE
/// em vez de INSERT.
pub async fn reclaim_user_with_email_verification(
    db: &Connection,
    config: &AppConfig,
    new_firebase_uid: String,
    email: String,
    email_verification_token: String,
) -> Result<User, AppError> {
    let normalized_email = normalize_email(&email)?;
    let token = email_verification_token.trim();
    if token.is_empty() {
        return Err(AppError::BadRequest(
            "Verificacao de email ausente ou invalida".into(),
        ));
    }

    let token_hash = email_token_hash(&config.email_code_secret, token);
    let now = iso_now();
    let email = normalized_email.clone();

    db.call(move |c| {
        let tx = c.transaction()?;

        let verification: Option<(String, String)> = tx
            .query_row(
                "SELECT id, token_expires_at
                 FROM email_verifications
                 WHERE email = ?1
                   AND token_hash = ?2
                   AND status = 'verified'
                   AND consumed_at IS NULL
                 ORDER BY verified_at DESC LIMIT 1",
                params![normalized_email, token_hash],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        let (verification_id, token_expires_at) =
            verification.ok_or_else(|| ServiceError::EmailTokenInvalid.into_sqlite())?;

        if token_expires_at.as_str() < now.as_str() {
            tx.execute(
                "UPDATE email_verifications SET status = 'expired' WHERE id = ?1",
                params![verification_id],
            )?;
            return Err(ServiceError::EmailTokenExpired.into_sqlite());
        }

        let updated = tx.execute(
            "UPDATE users SET firebase_uid = ?1 WHERE email = ?2",
            params![new_firebase_uid, email],
        )?;
        // A conta sumiu entre a checagem do caller e este UPDATE (corrida).
        // Trata como token inválido para não consumir o token à toa.
        if updated == 0 {
            return Err(ServiceError::EmailTokenInvalid.into_sqlite());
        }

        tx.execute(
            "UPDATE email_verifications
             SET status = 'consumed', consumed_at = ?1
             WHERE id = ?2",
            params![now, verification_id],
        )?;

        let user = tx.query_row(
            "SELECT id, firebase_uid, email, display_name, mode, created_at
             FROM users WHERE email = ?1",
            params![email],
            |row| {
                Ok(User {
                    id: row.get(0)?,
                    firebase_uid: row.get(1)?,
                    email: row.get(2)?,
                    display_name: row.get(3)?,
                    mode: parse_block_mode(&row.get::<_, String>(4)?),
                    created_at: row.get(5)?,
                })
            },
        )?;

        tx.commit()?;

        Ok(user)
    })
    .await
    .map_err(map_register_error)
}
