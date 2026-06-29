// =============================================================================
// email_delivery — entrega dos códigos de verificação de email.
// =============================================================================
// Duas estratégias, escolhidas por `EMAIL_DELIVERY_MODE`:
//   - Smtp : envia o código por SMTP real (StartTLS/587 ou WrapperTLS/465).
//   - Log  : apenas loga o código (desenvolvimento; sem credenciais SMTP).
// O `email_code` chama estas funções; aqui não há lógica de código/HMAC.
// =============================================================================

use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::core::config::{AppConfig, EmailDeliveryMode, SmtpConfig};
use crate::core::errors::AppError;

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

pub(crate) fn resolve_smtp_delivery(config: &AppConfig) -> Result<Option<SmtpConfig>, AppError> {
    match config.email_delivery_mode {
        EmailDeliveryMode::Smtp => config.smtp.clone().map(Some).ok_or_else(|| {
            AppError::ServiceUnavailable("Envio de email nao configurado no backend".into())
        }),
        EmailDeliveryMode::Log => Ok(None),
    }
}

pub(crate) fn log_verification_code(email: &str, code: &str, expires_at: &str) {
    tracing::warn!(
        email = %email,
        code = %code,
        expires_at = %expires_at,
        "EMAIL_DELIVERY_MODE=log; codigo de verificacao gerado sem envio SMTP"
    );
}

pub(crate) async fn send_verification_email(
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
    use crate::core::config::{AppConfig, EmailDeliveryMode};

    use super::{resolve_smtp_delivery, smtp_tls_mode_for_port, SmtpTlsMode};

    fn config_with_delivery_mode(email_delivery_mode: EmailDeliveryMode) -> AppConfig {
        AppConfig {
            port: 3000,
            database_path: "dopablocker.db".into(),
            database_key: "test-key".into(),
            firebase_project_id: "dopablocker-test".into(),
            email_code_secret: "test-secret".into(),
            email_delivery_mode,
            smtp: None,
            cors_allowed_origins: vec!["http://localhost:5173".into()],
        }
    }

    #[test]
    fn uses_starttls_for_submission_port_and_wrapper_tls_for_smtps() {
        assert_eq!(smtp_tls_mode_for_port(587), SmtpTlsMode::StartTls);
        assert_eq!(smtp_tls_mode_for_port(465), SmtpTlsMode::WrapperTls);
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
