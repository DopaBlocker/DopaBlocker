// =============================================================================
// DopaBlocker — Ponto de entrada do backend (Rust/Axum)
// =============================================================================
// Este arquivo é o "boot" do servidor. Ele executa, em ordem:
//   1. Carrega variáveis de ambiente (.env + sistema).
//   2. Inicializa o sistema de logs (tracing).
//   3. Abre a conexão com o SQLCipher (com a chave passada via env) e
//      aplica as migrations que ainda não foram aplicadas.
//   4. Monta o `AppState` que é compartilhado entre todos os handlers.
//   5. Separa as rotas em dois grupos (públicas vs. protegidas) e as une.
//   6. Inicia o listener TCP e delega ao Axum o tratamento das requisições.
//
// A separação entre `public_routes` e `protected_routes` é essencial para
// permitir que `/devices/link/confirm` seja acessada sem credencial —
// afinal, o filho ainda não tem nem Firebase JWT nem Device Token quando
// confirma o código de vinculação pela primeira vez.
// =============================================================================

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::State,
    http::{header, Method, StatusCode},
    middleware as axum_mw,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use tokio_rusqlite::Connection;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};
use tower_http::cors::CorsLayer;

mod config;
mod db;
mod errors;
mod middleware;
mod models;
mod routes;
mod services;

use middleware::JwksCache;

/// Estado compartilhado da aplicação. Todo handler recebe um clone disto via
/// `State<AppState>`. É `Clone` porque o Axum precisa poder clonar o state
/// para injetá-lo em cada request — e os campos internos são cheap-to-clone
/// (`tokio_rusqlite::Connection` é um `Arc` por dentro; `Arc<JwksCache>` é
/// um ponteiro reference-counted; `AppConfig` são apenas strings/ints).
#[derive(Clone)]
pub struct AppState {
    pub config: config::AppConfig,
    pub db: Connection,
    pub jwks: Arc<JwksCache>,
}

#[tokio::main]
async fn main() {
    // -------------------------------------------------------------------------
    // 1. Logs estruturados. `tracing::info!` abaixo só tem efeito após isto.
    // -------------------------------------------------------------------------
    tracing_subscriber::fmt::init();
    tracing::info!("Starting DopaBlocker Backend...");

    // -------------------------------------------------------------------------
    // 2. Configuração — lê PORT, DATABASE_PATH, SQLCIPHER_KEY, FIREBASE_PROJECT_ID.
    //    Em dev, defaults inseguros são aceitos; em prod, todas devem vir do
    //    ambiente (principalmente SQLCIPHER_KEY).
    // -------------------------------------------------------------------------
    let app_config = config::AppConfig::init();

    // -------------------------------------------------------------------------
    // 3. Banco — abre o arquivo .db, aplica `PRAGMA key = '<chave>'`
    //    IMEDIATAMENTE (é isso que descriptografa o SQLCipher), e só então
    //    roda as migrations. Sem o PRAGMA key, qualquer query falharia com
    //    "file is not a database" porque o conteúdo continua cifrado.
    // -------------------------------------------------------------------------
    let db = db::connect(&app_config.database_path, &app_config.database_key)
        .await
        .expect("Falha ao conectar ao SQLCipher");
    db::run_migrations(&db)
        .await
        .expect("Falha ao aplicar migrations");

    // -------------------------------------------------------------------------
    // 4. State compartilhado. `JwksCache::new()` cria um cache vazio — as
    //    chaves públicas do Firebase são baixadas na primeira validação de
    //    JWT (lazy), e depois cacheadas por 6h (ver middleware.rs).
    // -------------------------------------------------------------------------
    let state = AppState {
        config: app_config.clone(),
        db,
        jwks: JwksCache::new(),
    };

    // -------------------------------------------------------------------------
    // 5. Montagem das rotas.
    //
    //    `public_routes` = rotas que o middleware global NÃO valida:
    //      - POST /auth/register    → valida JWT manualmente inline
    //      - POST /auth/login       → valida JWT manualmente inline
    //      - POST /devices/link/confirm → totalmente anônima (filho ainda não
    //        tem credencial). Gera o device_token que será usado dali em diante.
    //
    //    `protected_routes` = tudo o mais. O `route_layer(require_auth)` aplica
    //    o middleware de auth dual (Firebase JWT OU Device Token) a TODAS as
    //    rotas dentro deste grupo. Rotas read-only (GET) aceitam ambos; rotas
    //    de escrita rejeitam Device Tokens com 403.
    //
    //    O `/health` é público (sem auth) para health-checks de infra.
    // -------------------------------------------------------------------------
    // Rate-limit por IP (GCRA via `tower_governor`) aplicado SÓ às rotas
    // públicas de auth (`/auth/*`) e ao `/devices/link/confirm` — o ponto de
    // abuso (spam de código por email, brute-force de login/código). Rotas
    // protegidas ficam de fora (o poll do filho passa por elas). O
    // `SmartIpKeyExtractor` usa `x-forwarded-for`/`x-real-ip` atrás de proxy e
    // cai no IP do peer (exige `into_make_service_with_connect_info`).
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(AUTH_RATE_REPLENISH_SECS)
            .burst_size(AUTH_RATE_BURST)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("configuração de rate-limit inválida"),
    );
    // Limpeza periódica do storage do rate-limiter (entradas antigas).
    let governor_limiter = governor_conf.limiter().clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(60));
        governor_limiter.retain_recent();
    });

    let public_routes = Router::new()
        .merge(routes::auth::public_router())
        .merge(routes::devices::public_router())
        .layer(GovernorLayer::new(governor_conf));

    let protected_routes = Router::new()
        .merge(routes::auth::protected_router())
        .nest("/blocklist", routes::blocklist::router())
        .nest("/devices", routes::devices::protected_router())
        .route_layer(axum_mw::from_fn_with_state(
            state.clone(),
            middleware::require_auth,
        ));

    let app = Router::new()
        // `/health` = liveness simples; `/healthz` = readiness que valida o
        // banco SQLCipher (ver `healthz`). Ambos públicos para health-checks.
        .route("/health", get(|| async { "OK" }))
        .route("/healthz", get(healthz))
        .merge(public_routes)
        .merge(protected_routes)
        // CORS por allowlist (config). Em prod, definir `CORS_ALLOWED_ORIGINS`.
        .layer(build_cors(&app_config))
        .with_state(state);

    // -------------------------------------------------------------------------
    // 6. Sobe o servidor. `0.0.0.0` para aceitar conexões de qualquer
    //    interface (necessário em container/Docker; localhost bastaria
    //    para desenvolvimento puro local).
    // -------------------------------------------------------------------------
    let address = format!("0.0.0.0:{}", app_config.port);
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

/// Parâmetros do rate-limit das rotas públicas de auth (GCRA por IP).
/// `BURST` = quantas requisições um IP pode disparar de imediato; depois disso,
/// repõe 1 permissão a cada `REPLENISH_SECS` (≈ 12/min sustentado após o burst).
const AUTH_RATE_BURST: u32 = 10;
const AUTH_RATE_REPLENISH_SECS: u64 = 5;

/// `GET /healthz` — readiness estruturada. Roda um `SELECT 1` para provar que a
/// conexão SQLCipher está aberta e que a `PRAGMA key` descriptografou o banco
/// (chave errada falha já aqui). 200 = saudável; 503 = banco inacessível.
async fn healthz(State(state): State<AppState>) -> Response {
    let probe = state
        .db
        .call(|c| Ok(c.query_row("SELECT 1", [], |r| r.get::<_, i64>(0))?))
        .await;

    match probe {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "ok" }))).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "healthz: banco inacessível");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "error" })),
            )
                .into_response()
        }
    }
}

/// Monta o `CorsLayer` a partir da allowlist da config. Origens que não
/// parseiam como `HeaderValue` são ignoradas (defesa contra config malformada).
fn build_cors(config: &config::AppConfig) -> CorsLayer {
    let origins: Vec<axum::http::HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}
