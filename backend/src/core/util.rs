// =============================================================================
// Utilitários compartilhados pelos services do backend.
// =============================================================================
// Concentra três coisas que antes estavam duplicadas/frágeis:
//
//   1. `ServiceError`  — enum tipado de erros de domínio que viajam por dentro
//      do `tokio_rusqlite::Error::Other`. Antes os services usavam strings
//      sentinela ("LINK_EXPIRED") e os handlers faziam `msg.contains(...)`,
//      o que era frágil: typo silencioso virava 500 em vez de 400.
//
//   2. Helpers de timestamp ISO-8601 (`iso_now`, `iso_after`, `iso_before`)
//      que antes só existiam no `auth_service.rs` enquanto os outros services
//      duplicavam `Utc::now().format("...")` inline.
//
//   3. Conversão `Display`/`FromStr` para os enums persistidos em texto no
//      SQLite (`BlockMode`, `Platform`, `BlockedType`). Antes cada service
//      tinha suas próprias funções `*_to_str` / `str_to_*` duplicadas.
// =============================================================================

use chrono::{Duration as ChronoDuration, Utc};

use crate::core::errors::AppError;
use crate::core::models::{BlockMode, BlockedType, Platform};

// -----------------------------------------------------------------------------
// 1. ServiceError — erros de domínio tipados
// -----------------------------------------------------------------------------

/// Erros de domínio que os services produzem dentro de `tokio_rusqlite::Error::Other`.
///
/// O fluxo é: o service sinaliza o erro com `into_sqlite()`, o handler captura
/// o `tokio_rusqlite::Error` e converte para `AppError` com `to_app_error()`.
/// Erros que não são `ServiceError` (ex: SQL puro) viram `InternalServerError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceError {
    // Vinculação parental
    LinkNotFound,
    LinkExpired,
    // Verificação de email — start
    EmailCodeCooldown,
    EmailCodeRateLimited,
    // Verificação de email — verify / consume
    EmailCodeNotFound,
    EmailCodeExpired,
    EmailCodeTooManyAttempts,
    EmailCodeInvalid,
    EmailTokenInvalid,
    EmailTokenExpired,
}

impl ServiceError {
    /// Tag estável usada como discriminador dentro de `Box<dyn Error>`.
    /// Manter as strings inalteradas — `from_message` faz lookup por elas.
    const fn tag(self) -> &'static str {
        match self {
            ServiceError::LinkNotFound => "LINK_NOT_FOUND",
            ServiceError::LinkExpired => "LINK_EXPIRED",
            ServiceError::EmailCodeCooldown => "EMAIL_CODE_COOLDOWN",
            ServiceError::EmailCodeRateLimited => "EMAIL_CODE_RATE_LIMITED",
            ServiceError::EmailCodeNotFound => "EMAIL_CODE_NOT_FOUND",
            ServiceError::EmailCodeExpired => "EMAIL_CODE_EXPIRED",
            ServiceError::EmailCodeTooManyAttempts => "EMAIL_CODE_TOO_MANY_ATTEMPTS",
            ServiceError::EmailCodeInvalid => "EMAIL_CODE_INVALID",
            ServiceError::EmailTokenInvalid => "EMAIL_TOKEN_INVALID",
            ServiceError::EmailTokenExpired => "EMAIL_TOKEN_EXPIRED",
        }
    }

    /// Embrulha o erro como `tokio_rusqlite::Error::Other` para que ele possa
    /// ser propagado de dentro de `db.call(|c| { ... })`.
    pub fn into_sqlite(self) -> tokio_rusqlite::Error {
        tokio_rusqlite::Error::Other(Box::<dyn std::error::Error + Send + Sync>::from(self.tag()))
    }

    /// Tenta extrair um `ServiceError` da mensagem de um erro genérico
    /// (vinda de `tokio_rusqlite::Error::Other`). Retorna `None` se a mensagem
    /// não corresponde a nenhum tag conhecido.
    pub fn from_message(msg: &str) -> Option<Self> {
        const ALL: &[ServiceError] = &[
            ServiceError::LinkNotFound,
            ServiceError::LinkExpired,
            ServiceError::EmailCodeCooldown,
            ServiceError::EmailCodeRateLimited,
            ServiceError::EmailCodeNotFound,
            ServiceError::EmailCodeExpired,
            ServiceError::EmailCodeTooManyAttempts,
            ServiceError::EmailCodeInvalid,
            ServiceError::EmailTokenInvalid,
            ServiceError::EmailTokenExpired,
        ];
        ALL.iter().copied().find(|e| msg.contains(e.tag()))
    }

    /// Converte para `AppError` (resposta HTTP). Quase todos viram 400; o caller
    /// pode customizar para casos específicos antes de delegar pra cá.
    pub fn to_app_error(self) -> AppError {
        match self {
            ServiceError::LinkNotFound => {
                AppError::BadRequest("Código inválido ou já utilizado".into())
            }
            ServiceError::LinkExpired => AppError::BadRequest("Código expirado".into()),
            ServiceError::EmailCodeCooldown => {
                AppError::BadRequest("Aguarde antes de pedir um novo codigo".into())
            }
            ServiceError::EmailCodeRateLimited => {
                AppError::BadRequest("Limite de codigos por hora atingido".into())
            }
            ServiceError::EmailCodeNotFound => {
                AppError::BadRequest("Codigo invalido ou expirado".into())
            }
            ServiceError::EmailCodeExpired => AppError::BadRequest("Codigo expirado".into()),
            ServiceError::EmailCodeTooManyAttempts => {
                AppError::BadRequest("Muitas tentativas. Peca um novo codigo".into())
            }
            ServiceError::EmailCodeInvalid => AppError::BadRequest("Codigo invalido".into()),
            ServiceError::EmailTokenInvalid => {
                AppError::BadRequest("Verificacao de email ausente ou invalida".into())
            }
            ServiceError::EmailTokenExpired => {
                AppError::BadRequest("Verificacao de email expirada".into())
            }
        }
    }
}

/// Converte um erro de `tokio_rusqlite` para `AppError`. Se a mensagem
/// corresponde a um `ServiceError` conhecido, usa a tradução tipada;
/// caso contrário, vira `InternalServerError`.
pub fn map_sqlite_error(err: tokio_rusqlite::Error) -> AppError {
    let msg = err.to_string();
    if let Some(service_err) = ServiceError::from_message(&msg) {
        service_err.to_app_error()
    } else {
        AppError::InternalServerError(msg)
    }
}

// -----------------------------------------------------------------------------
// 2. Helpers de timestamp ISO-8601
// -----------------------------------------------------------------------------

const ISO_FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

pub fn iso_now() -> String {
    Utc::now().format(ISO_FORMAT).to_string()
}

pub fn iso_after(seconds: i64) -> String {
    (Utc::now() + ChronoDuration::seconds(seconds))
        .format(ISO_FORMAT)
        .to_string()
}

pub fn iso_before(seconds: i64) -> String {
    (Utc::now() - ChronoDuration::seconds(seconds))
        .format(ISO_FORMAT)
        .to_string()
}

// -----------------------------------------------------------------------------
// 3. Conversões enum ↔ texto para persistência SQLite
// -----------------------------------------------------------------------------
//
// Os enums em `shared::models` já têm `#[serde(rename_all = "lowercase")]` para
// JSON. Aqui centralizamos as conversões para os literais que vão pro SQLite.
// Antes da refatoração, cada service tinha sua própria função `*_to_str`/`str_to_*`
// — fonte clássica de divergência se um dia adicionarmos um valor novo.

/// Converte texto vindo do SQLite em `BlockMode`. Qualquer valor inesperado
/// vira `Personal` (mais seguro — modo menos privilegiado). Na prática só
/// acontece se alguém editar o `.db` manualmente.
pub fn parse_block_mode(s: &str) -> BlockMode {
    match s {
        "parental" => BlockMode::Parental,
        _ => BlockMode::Personal,
    }
}

pub fn parse_platform(s: &str) -> Platform {
    match s {
        "android" => Platform::Android,
        _ => Platform::Windows,
    }
}

pub fn parse_blocked_type(s: &str) -> BlockedType {
    match s {
        "app" => BlockedType::App,
        "keyword" => BlockedType::Keyword,
        _ => BlockedType::Domain,
    }
}

/// Atalho `&BlockMode -> String` para uso direto em `params![...]`.
pub fn block_mode_to_sql(mode: &BlockMode) -> &'static str {
    match mode {
        BlockMode::Personal => "personal",
        BlockMode::Parental => "parental",
    }
}

pub fn platform_to_sql(p: &Platform) -> &'static str {
    match p {
        Platform::Windows => "windows",
        Platform::Android => "android",
    }
}

pub fn blocked_type_to_sql(t: &BlockedType) -> &'static str {
    match t {
        BlockedType::Domain => "domain",
        BlockedType::App => "app",
        BlockedType::Keyword => "keyword",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_error_round_trip_through_message() {
        for tag in [
            ServiceError::LinkNotFound,
            ServiceError::LinkExpired,
            ServiceError::EmailCodeCooldown,
            ServiceError::EmailCodeInvalid,
            ServiceError::EmailTokenExpired,
        ] {
            let msg = tag.into_sqlite().to_string();
            assert_eq!(ServiceError::from_message(&msg), Some(tag));
        }
    }

    #[test]
    fn unknown_message_returns_none() {
        assert_eq!(ServiceError::from_message("UNIQUE constraint failed"), None);
    }

    #[test]
    fn enum_display_matches_parse() {
        assert_eq!(parse_block_mode(block_mode_to_sql(&BlockMode::Personal)), BlockMode::Personal);
        assert_eq!(parse_block_mode(block_mode_to_sql(&BlockMode::Parental)), BlockMode::Parental);
        assert_eq!(parse_platform(platform_to_sql(&Platform::Windows)), Platform::Windows);
        assert_eq!(parse_platform(platform_to_sql(&Platform::Android)), Platform::Android);
        assert_eq!(parse_blocked_type(blocked_type_to_sql(&BlockedType::Domain)), BlockedType::Domain);
        assert_eq!(parse_blocked_type(blocked_type_to_sql(&BlockedType::App)), BlockedType::App);
    }

    #[test]
    fn iso_format_is_lexicographically_ordered() {
        let earlier = iso_before(60);
        let now = iso_now();
        let later = iso_after(60);
        assert!(earlier.as_str() < now.as_str());
        assert!(now.as_str() < later.as_str());
    }
}
