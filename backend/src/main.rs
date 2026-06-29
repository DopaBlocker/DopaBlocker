// =============================================================================
// DopaBlocker — Ponto de entrada do backend (Rust/Axum)
// =============================================================================
// `main.rs` é o "boot" do servidor, mantido fino: inicializa logs, lê a
// configuração, constrói o `AppState` (via `bootstrap::init_state`), monta o
// router (via `app::build_router`) e sobe o listener TCP. A coreografia de
// inicialização vive em `bootstrap.rs` e a montagem das rotas em `app.rs`.
//
// Organização do crate (espelha o mobile `lib/core` + `lib/features`):
//   - core/        → infra compartilhada (config, db, errors, models, util, auth)
//   - features/    → domínios de negócio (auth, devices, blocklist)
//   - app.rs       → build do Router (rotas + CORS + rate-limit + health)
//   - bootstrap.rs → init do AppState (db + migrations)
// =============================================================================

use std::net::SocketAddr;
use std::sync::Arc;

use tokio_rusqlite::Connection;

mod app;
mod bootstrap;
mod core;
mod features;

use crate::core::auth::JwksCache;
use crate::core::config::AppConfig;

/// Estado compartilhado da aplicação. Todo handler recebe um clone disto via
/// `State<AppState>`. É `Clone` porque o Axum precisa poder clonar o state
/// para injetá-lo em cada request — e os campos internos são cheap-to-clone
/// (`tokio_rusqlite::Connection` é um `Arc` por dentro; `Arc<JwksCache>` é
/// um ponteiro reference-counted; `AppConfig` são apenas strings/ints).
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: Connection,
    pub jwks: Arc<JwksCache>,
}

#[tokio::main]
async fn main() {
    // Logs estruturados. `tracing::info!` abaixo só tem efeito após isto.
    tracing_subscriber::fmt::init();
    tracing::info!("Starting DopaBlocker Backend...");

    // Configuração — lê PORT, DATABASE_PATH, SQLCIPHER_KEY, FIREBASE_PROJECT_ID, etc.
    // Em dev, defaults inseguros são aceitos; em prod, todas devem vir do
    // ambiente (principalmente SQLCIPHER_KEY).
    let config = AppConfig::init();
    // Captura a porta ANTES de mover `config` para dentro do AppState.
    let port = config.port;

    // Estado compartilhado: abre o SQLCipher (PRAGMA key primeiro), roda as
    // migrations idempotentes e monta o AppState. Ver `bootstrap::init_state`.
    let state = bootstrap::init_state(config).await;

    // Montagem das rotas (públicas + protegidas + CORS + rate-limit + health).
    let app = app::build_router(state);

    // Sobe o servidor. `0.0.0.0` para aceitar conexões de qualquer interface
    // (necessário em container/Docker; localhost bastaria para dev puro local).
    let address = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .expect("Falha ao bindar porta");
    tracing::info!("Listening on {}", address);

    // `into_make_service_with_connect_info` injeta o `SocketAddr` do peer em
    // cada request — necessário para o `SmartIpKeyExtractor` do rate-limiter
    // chavear por IP quando não há header de proxy.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("Servidor falhou");
}
