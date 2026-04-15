use axum::{
    middleware,
    routing::get,
    Router,
};
use tower_http::cors::CorsLayer;
use tracing_subscriber;

mod config;
mod errors;
mod middleware;
mod models;
mod routes;
mod services;

#[derive(Clone)]
pub struct AppState {
    pub config: config::AppConfig,
    // Futuro: pub pool: sqlx::SqlitePool ou Firebase Client
}

#[tokio::main]
async fn main() {
    // 1. Inicializa Logs
    tracing_subscriber::fmt::init();
    tracing::info!("Starting DopaBlocker Backend...");

    // 2. Carrega Configurações
    let app_config = config::AppConfig::init();
    
    // 3. Monta o AppState
    let state = AppState {
        config: app_config.clone(),
    };

    // 4. Monta as Rotas Protegidas (aplicam o middleware require_auth)
    let protected_routes = Router::new()
        .nest("/devices", routes::devices::router())
        .nest("/blocklist", routes::blocklist::router())
        .route_layer(middleware::from_fn(crate::middleware::require_auth));

    // 5. Monta o Router Principal
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .nest("/api/auth", routes::auth::router())  // Auth não é protegida (login/register)
        .nest("/api", protected_routes)            // Todo o resto protegido
        .layer(CorsLayer::permissive())            // Habilita requisições de qualquer origem (CORS MOCK)
        .with_state(state);                        // Injeta dependências

    // 6. Inicia o Servidor
    let address = format!("0.0.0.0:{}", app_config.port);
    let listener = tokio::net::TcpListener::bind(&address).await.unwrap();
    tracing::info!("Listening on {}", address);

    axum::serve(listener, app).await.unwrap();
}
