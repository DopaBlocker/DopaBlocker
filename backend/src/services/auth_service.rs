use hmac::{Hmac, Mac};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use rand::{Rng, RngCore};
use rusqlite::{params, OptionalExtension};
use sha2::Sha256;
use tokio_rusqlite::Connection;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

use crate::config::{AppConfig, EmailDeliveryMode, SmtpConfig};
use crate::errors::AppError;
use crate::models::{BlockMode, EmailCodeStartResponse, EmailCodeVerifyResponse, User};
use crate::services::util::{
    block_mode_to_sql, iso_after, iso_before, iso_now, map_sqlite_error, parse_block_mode,
    ServiceError,
};

const CODE_TTL_SECS: i64 = 10 * 60;
const TOKEN_TTL_SECS: i64 = 15 * 60;
const RESEND_COOLDOWN_SECS: i64 = 60;
const MAX_ATTEMPTS: i64 = 5;
const MAX_SENDS_PER_HOUR: i64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SmtpTlsMode {
    StartTls,
    WrapperTls,
}

fn smtp_tls_mode_for_port(port: u16) -> SmtpTlsMode {
    if port == 465 {
        SmtpTlsMode::WrapperTls
    } else {
        SmtpTlsMode::StartTls
    }
}

pub fn normalize_email(input: &str) -> Result<String, AppError> {
    let email = input.trim().to_lowercase();
    let mut parts = email.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();

    if email.len() > 254
        || local.is_empty()
        || domain.is_empty()
        || parts.next().is_some()
        || domain.starts_with('.')
        || domain.ends_with('.')
        || !domain.contains('.')
    {
        return Err(AppError::BadRequest("Email invalido".into()));
    }

    Ok(email)
}

pub fn format_numeric_code(value: u32) -> String {
    format!("{:06}", value % 1_000_000)
}

fn generate_numeric_code() -> String {
    format_numeric_code(rand::thread_rng().gen_range(0..1_000_000))
}

pub fn validate_code_shape(code: &str) -> Result<String, AppError> {
    let code = code.trim();
    if code.len() == 6 && code.bytes().all(|b| b.is_ascii_digit()) {
        Ok(code.to_string())
    } else {
        Err(AppError::BadRequest("Codigo deve ter 6 digitos".into()))
    }
}

pub fn email_code_hash(secret: &str, email: &str, code: &str) -> String {
    hmac_sha256_hex(secret, &format!("email-code:{email}:{code}"))
}

pub fn email_token_hash(secret: &str, token: &str) -> String {
    hmac_sha256_hex(secret, &format!("email-token:{token}"))
}

pub fn constant_time_eq(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    // `subtle` exige slices de mesmo tamanho — strings com tamanhos
    // diferentes nunca podem ser iguais, e essa checagem é segura mesmo
    // sob otimização agressiva do compilador.
    if a.len() != b.len() {
        return false;
    }
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

fn hmac_sha256_hex(secret: &str, message: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC aceita chaves de qualquer tamanho");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn generate_verification_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

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

pub async fn start_email_verification(
    db: &Connection,
    config: &AppConfig,
    email: String,
) -> Result<EmailCodeStartResponse, AppError> {
    let smtp = resolve_smtp_delivery(config)?;
    let email = normalize_email(&email)?;
    let code = generate_numeric_code();
    let code_hash = email_code_hash(&config.email_code_secret, &email, &code);
    let id = Uuid::new_v4().to_string();
    let now = iso_now();
    let expires_at = iso_after(CODE_TTL_SECS);
    let cooldown_cutoff = iso_before(RESEND_COOLDOWN_SECS);
    let hourly_cutoff = iso_before(60 * 60);

    db.call({
        let id = id.clone();
        let email = email.clone();
        let code_hash = code_hash.clone();
        let now = now.clone();
        let expires_at = expires_at.clone();
        move |c| {
            let latest_sent_at: Option<String> = c
                .query_row(
                    "SELECT last_sent_at FROM email_verifications
                     WHERE email = ?1
                     ORDER BY last_sent_at DESC LIMIT 1",
                    params![email],
                    |row| row.get(0),
                )
                .optional()?;

            if latest_sent_at
                .as_deref()
                .is_some_and(|sent_at| sent_at > cooldown_cutoff.as_str())
            {
                return Err(ServiceError::EmailCodeCooldown.into_sqlite());
            }

            let recent_sends: i64 = c.query_row(
                "SELECT COUNT(*) FROM email_verifications
                 WHERE email = ?1 AND created_at >= ?2",
                params![email, hourly_cutoff],
                |row| row.get(0),
            )?;

            if recent_sends >= MAX_SENDS_PER_HOUR {
                return Err(ServiceError::EmailCodeRateLimited.into_sqlite());
            }

            c.execute(
                "UPDATE email_verifications
                 SET status = 'expired'
                 WHERE email = ?1
                   AND status IN ('pending', 'verified')
                   AND consumed_at IS NULL",
                params![email],
            )?;

            c.execute(
                "INSERT INTO email_verifications(
                    id, email, code_hash, status, attempts, expires_at, last_sent_at
                 )
                 VALUES (?1, ?2, ?3, 'pending', 0, ?4, ?5)",
                params![id, email, code_hash, expires_at, now],
            )?;

            Ok(())
        }
    })
    .await
    .map_err(map_sqlite_error)?;

    if let Some(smtp) = smtp {
        if let Err(err) = send_verification_email(smtp, email.clone(), code).await {
            expire_email_verification(db, id).await;
            return Err(err);
        }
    } else {
        log_verification_code(&email, &code, &expires_at);
    }

    Ok(EmailCodeStartResponse {
        expires_at,
        resend_after_seconds: RESEND_COOLDOWN_SECS,
    })
}

fn resolve_smtp_delivery(config: &AppConfig) -> Result<Option<SmtpConfig>, AppError> {
    match config.email_delivery_mode {
        EmailDeliveryMode::Smtp => config.smtp.clone().map(Some).ok_or_else(|| {
            AppError::ServiceUnavailable("Envio de email nao configurado no backend".into())
        }),
        EmailDeliveryMode::Log => Ok(None),
    }
}

fn log_verification_code(email: &str, code: &str, expires_at: &str) {
    tracing::warn!(
        email = %email,
        code = %code,
        expires_at = %expires_at,
        "EMAIL_DELIVERY_MODE=log; codigo de verificacao gerado sem envio SMTP"
    );
}

pub async fn verify_email_code(
    db: &Connection,
    config: &AppConfig,
    email: String,
    code: String,
) -> Result<EmailCodeVerifyResponse, AppError> {
    let email = normalize_email(&email)?;
    let code = validate_code_shape(&code)?;
    let submitted_hash = email_code_hash(&config.email_code_secret, &email, &code);
    let token = generate_verification_token();
    let token_hash = email_token_hash(&config.email_code_secret, &token);
    let now = iso_now();
    let token_expires_at = iso_after(TOKEN_TTL_SECS);

    db.call({
        let email = email.clone();
        move |c| {
            let record: Option<(String, String, i64, String)> = c
                .query_row(
                    "SELECT id, code_hash, attempts, expires_at
                     FROM email_verifications
                     WHERE email = ?1 AND status = 'pending'
                     ORDER BY created_at DESC LIMIT 1",
                    params![email],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()?;

            let (id, stored_hash, attempts, expires_at) =
                record.ok_or_else(|| ServiceError::EmailCodeNotFound.into_sqlite())?;

            if expires_at.as_str() < now.as_str() {
                c.execute(
                    "UPDATE email_verifications SET status = 'expired' WHERE id = ?1",
                    params![id],
                )?;
                return Err(ServiceError::EmailCodeExpired.into_sqlite());
            }

            if attempts >= MAX_ATTEMPTS {
                return Err(ServiceError::EmailCodeTooManyAttempts.into_sqlite());
            }

            if !constant_time_eq(&stored_hash, &submitted_hash) {
                c.execute(
                    "UPDATE email_verifications
                     SET attempts = attempts + 1
                     WHERE id = ?1",
                    params![id],
                )?;
                return Err(ServiceError::EmailCodeInvalid.into_sqlite());
            }

            c.execute(
                "UPDATE email_verifications
                 SET status = 'verified',
                     token_hash = ?1,
                     token_expires_at = ?2,
                     verified_at = ?3
                 WHERE id = ?4",
                params![token_hash, token_expires_at, now, id],
            )?;

            Ok(())
        }
    })
    .await
    .map_err(map_sqlite_error)?;

    Ok(EmailCodeVerifyResponse {
        email_verification_token: token,
    })
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
    let email = email.trim().to_string();
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

async fn expire_email_verification(db: &Connection, id: String) {
    let _ = db
        .call(move |c| {
            c.execute(
                "UPDATE email_verifications SET status = 'expired' WHERE id = ?1",
                params![id],
            )?;
            Ok(())
        })
        .await;
}

async fn send_verification_email(
    smtp: SmtpConfig,
    email: String,
    code: String,
) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || send_verification_email_blocking(smtp, email, code))
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Falha ao enviar email: {e}")))?
}

fn send_verification_email_blocking(
    smtp: SmtpConfig,
    email: String,
    code: String,
) -> Result<(), AppError> {
    let from: Mailbox = smtp
        .from
        .parse()
        .map_err(|e| AppError::ServiceUnavailable(format!("SMTP_FROM invalido: {e}")))?;
    let to: Mailbox = email
        .parse()
        .map_err(|e| AppError::BadRequest(format!("Email invalido para envio: {e}")))?;
    let body = format!(
        "Seu codigo DopaBlocker e {code}.\n\nEle expira em 10 minutos. Se voce nao pediu este cadastro, ignore este email."
    );

    let message = Message::builder()
        .from(from)
        .to(to)
        .subject("Seu codigo DopaBlocker")
        .body(body)
        .map_err(|e| AppError::ServiceUnavailable(format!("Email invalido: {e}")))?;

    let credentials = Credentials::new(smtp.username, smtp.password);
    let builder = match smtp_tls_mode_for_port(smtp.port) {
        SmtpTlsMode::WrapperTls => SmtpTransport::relay(&smtp.host),
        SmtpTlsMode::StartTls => SmtpTransport::starttls_relay(&smtp.host),
    }
    .map_err(|e| AppError::ServiceUnavailable(format!("SMTP_HOST invalido: {e}")))?;

    let mailer = builder.port(smtp.port).credentials(credentials).build();

    mailer
        .send(&message)
        .map_err(|e| AppError::ServiceUnavailable(format!("Falha ao enviar email: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::config::{AppConfig, EmailDeliveryMode};

    use super::{
        constant_time_eq, email_code_hash, email_token_hash, format_numeric_code, normalize_email,
        resolve_smtp_delivery, smtp_tls_mode_for_port, validate_code_shape,
    };

    fn config_with_delivery_mode(email_delivery_mode: EmailDeliveryMode) -> AppConfig {
        AppConfig {
            port: 3000,
            database_path: "dopablocker.db".into(),
            database_key: "test-key".into(),
            firebase_project_id: "dopablocker-test".into(),
            email_code_secret: "test-secret".into(),
            email_delivery_mode,
            smtp: None,
        }
    }

    #[test]
    fn normalizes_email_for_verification() {
        assert_eq!(
            normalize_email("  Focus.User@Example.COM  ").expect("valid email"),
            "focus.user@example.com"
        );
    }

    #[test]
    fn rejects_invalid_verification_email() {
        assert!(normalize_email("not-an-email").is_err());
        assert!(normalize_email("  ").is_err());
    }

    #[test]
    fn formats_numeric_code_with_zero_padding() {
        assert_eq!(format_numeric_code(42), "000042");
        assert_eq!(format_numeric_code(999_999), "999999");
    }

    #[test]
    fn accepts_only_six_digit_codes() {
        assert!(validate_code_shape("000042").is_ok());
        assert!(validate_code_shape("123456").is_ok());
        assert!(validate_code_shape("12345").is_err());
        assert!(validate_code_shape("1234567").is_err());
        assert!(validate_code_shape("12a456").is_err());
    }

    #[test]
    fn hashes_code_and_token_with_distinct_domains() {
        let secret = "test-secret";
        let email = "focus@example.com";

        let code_hash = email_code_hash(secret, email, "123456");
        let same_code_hash = email_code_hash(secret, email, "123456");
        let token_hash = email_token_hash(secret, "123456");

        assert_eq!(code_hash, same_code_hash);
        assert_ne!(code_hash, token_hash);
    }

    #[test]
    fn compares_hashes_in_constant_time_style() {
        assert!(constant_time_eq("abc123", "abc123"));
        assert!(!constant_time_eq("abc123", "abc124"));
        assert!(!constant_time_eq("abc123", "abc1234"));
    }

    #[test]
    fn uses_starttls_for_submission_port_and_wrapper_tls_for_smtps() {
        assert_eq!(smtp_tls_mode_for_port(587), super::SmtpTlsMode::StartTls);
        assert_eq!(smtp_tls_mode_for_port(465), super::SmtpTlsMode::WrapperTls);
    }

    #[test]
    fn log_delivery_mode_does_not_require_smtp_config() {
        let config = config_with_delivery_mode(EmailDeliveryMode::Log);

        assert!(resolve_smtp_delivery(&config).expect("log mode").is_none());
    }

    #[test]
    fn smtp_delivery_mode_requires_smtp_config() {
        let config = config_with_delivery_mode(EmailDeliveryMode::Smtp);

        assert!(resolve_smtp_delivery(&config).is_err());
    }
}
