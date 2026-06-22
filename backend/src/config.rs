// =============================================================================
// Configuração do backend — lê variáveis de ambiente em `AppConfig::init()`.
// =============================================================================
// Este módulo centraliza TODA a configuração que vem do ambiente externo.
// Nenhum outro módulo deve chamar `std::env::var` diretamente — sempre
// passa por aqui. Isso facilita:
//   - Testar com configs custom (bastaria um construtor alternativo).
//   - Saber em um só lugar quais env vars o backend lê.
//   - Trocar a fonte de config (AWS Parameter Store, Vault, etc.) sem
//     mexer no resto do código.
//
// Env vars lidas:
//   PORT                  → porta TCP (default 3000)
//   DATABASE_PATH         → caminho do arquivo .db (default "dopablocker.db")
//   SQLCIPHER_KEY         → chave AES do SQLCipher (default inseguro em dev)
//   FIREBASE_PROJECT_ID   → usado na validação de `iss` e `aud` do Firebase JWT
//   CORS_ALLOWED_ORIGINS  → allowlist de origens (CSV). Default: origens de dev.
// =============================================================================

use dotenvy::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    /// Caminho no disco do arquivo .db do SQLCipher. Relativo ao cwd por padrão.
    pub database_path: String,
    /// Chave AES-256 usada pelo `PRAGMA key`. Se vazia ou errada, o banco
    /// não descriptografa e qualquer query retorna erro.
    pub database_key: String,
    /// Project ID do Firebase (ex: "dopablocker-prod"). Usado para validar:
    ///   iss = "https://securetoken.google.com/<project_id>"
    ///   aud = "<project_id>"
    /// dos JWTs emitidos pelo Firebase Auth.
    pub firebase_project_id: String,
    /// Secret usado para HMAC dos cÃ³digos/tokens de verificaÃ§Ã£o de email.
    pub email_code_secret: String,
    /// Modo de entrega dos codigos: SMTP real ou log local de desenvolvimento.
    pub email_delivery_mode: EmailDeliveryMode,
    /// ConfiguraÃ§Ã£o SMTP opcional. Se ausente, o endpoint de envio retorna 503.
    pub smtp: Option<SmtpConfig>,
    /// Origens permitidas no CORS (allowlist). Em dev, default cobre Vite e o
    /// WebView do Tauri; em prod, definir `CORS_ALLOWED_ORIGINS` com o domínio
    /// real do frontend. O mobile (HTTP nativo) não passa por CORS.
    pub cors_allowed_origins: Vec<String>,
}

/// Origens liberadas por padrão quando `CORS_ALLOWED_ORIGINS` não é definida.
/// Cobre o dev server do Vite e os esquemas de origem do WebView do Tauri.
const DEFAULT_CORS_ORIGINS: &[&str] = &[
    "http://localhost:5173",
    "http://localhost:1420",
    "http://tauri.localhost",
    "https://tauri.localhost",
    "tauri://localhost",
];

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailDeliveryMode {
    Smtp,
    Log,
}

impl AppConfig {
    pub fn init() -> Self {
        // `dotenv()` carrega `.env` no `std::env` se o arquivo existir.
        // O `let _ = ...` silencia o erro caso o arquivo não exista —
        // isso é esperado em produção, onde as vars vêm do ambiente real.
        let _ = dotenv();

        // `.expect(...)` é intencional: se PORT for lixo, queremos falhar
        // ALTO e rápido, não iniciar um servidor numa porta errada.
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".into())
            .parse::<u16>()
            .expect("A variável PORT deve ser um número válido");

        let database_path = env::var("DATABASE_PATH").unwrap_or_else(|_| "dopablocker.db".into());

        // ATENÇÃO: o default "dev-only-unsafe-key" existe só para conveniência
        // local. Em produção, SQLCIPHER_KEY DEVE vir de um secret manager
        // (K8s secret, Vault, etc.). Rodar com a chave default é equivalente
        // a rodar sem criptografia.
        let database_key =
            env::var("SQLCIPHER_KEY").unwrap_or_else(|_| "dev-only-unsafe-key".into());

        let firebase_project_id =
            env::var("FIREBASE_PROJECT_ID").unwrap_or_else(|_| "dopablocker-dev".into());

        let email_code_secret = env::var("EMAIL_CODE_SECRET")
            .unwrap_or_else(|_| "dev-only-unsafe-email-code-secret".into());

        let email_delivery_mode_value = env::var("EMAIL_DELIVERY_MODE").ok();
        let email_delivery_mode = parse_email_delivery_mode(email_delivery_mode_value.as_deref())
            .expect("EMAIL_DELIVERY_MODE deve ser 'smtp' ou 'log'");

        let smtp = read_smtp_config();

        let cors_allowed_origins =
            parse_cors_origins(env::var("CORS_ALLOWED_ORIGINS").ok().as_deref());

        Self {
            port,
            database_path,
            database_key,
            firebase_project_id,
            email_code_secret,
            email_delivery_mode,
            smtp,
            cors_allowed_origins,
        }
    }
}

/// Faz o parsing da allowlist de CORS a partir do CSV em `CORS_ALLOWED_ORIGINS`.
/// Vazio/ausente → cai no default de desenvolvimento (`DEFAULT_CORS_ORIGINS`).
fn parse_cors_origins(value: Option<&str>) -> Vec<String> {
    match value {
        Some(raw) if !raw.trim().is_empty() => raw
            .split(',')
            .map(|origin| origin.trim().to_string())
            .filter(|origin| !origin.is_empty())
            .collect(),
        _ => DEFAULT_CORS_ORIGINS
            .iter()
            .map(|origin| origin.to_string())
            .collect(),
    }
}

fn parse_email_delivery_mode(value: Option<&str>) -> Result<EmailDeliveryMode, String> {
    match value.unwrap_or("smtp").trim().to_ascii_lowercase().as_str() {
        "" | "smtp" => Ok(EmailDeliveryMode::Smtp),
        "log" => Ok(EmailDeliveryMode::Log),
        other => Err(format!("modo de entrega de email invalido: {other}")),
    }
}

fn read_smtp_config() -> Option<SmtpConfig> {
    let host = env::var("SMTP_HOST").ok()?.trim().to_string();
    let username = env::var("SMTP_USERNAME").ok()?.trim().to_string();
    let password = env::var("SMTP_PASSWORD").ok()?;
    let from = env::var("SMTP_FROM").ok()?.trim().to_string();
    let port = env::var("SMTP_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(587);

    if host.is_empty() || username.is_empty() || password.is_empty() || from.is_empty() {
        return None;
    }

    Some(SmtpConfig {
        host,
        port,
        username,
        password,
        from,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_cors_origins, parse_email_delivery_mode, EmailDeliveryMode};

    #[test]
    fn parses_cors_origins_from_csv() {
        assert_eq!(
            parse_cors_origins(Some("http://a.com, https://b.com ,")),
            vec!["http://a.com".to_string(), "https://b.com".to_string()]
        );
    }

    #[test]
    fn cors_origins_fall_back_to_dev_defaults() {
        let from_none = parse_cors_origins(None);
        let from_blank = parse_cors_origins(Some("   "));
        assert!(from_none.contains(&"http://localhost:5173".to_string()));
        assert!(from_blank.contains(&"http://localhost:5173".to_string()));
    }

    #[test]
    fn parses_email_delivery_mode_for_smtp_and_log() {
        assert_eq!(
            parse_email_delivery_mode(None).expect("default mode"),
            EmailDeliveryMode::Smtp
        );
        assert_eq!(
            parse_email_delivery_mode(Some("smtp")).expect("smtp mode"),
            EmailDeliveryMode::Smtp
        );
        assert_eq!(
            parse_email_delivery_mode(Some(" log ")).expect("log mode"),
            EmailDeliveryMode::Log
        );
        assert!(parse_email_delivery_mode(Some("outlook")).is_err());
    }
}
