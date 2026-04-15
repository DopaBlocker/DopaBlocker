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

use std::sync::Arc;

use axum::{middleware as axum_mw, routing::get, Router};
use tokio_rusqlite::Connection;
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
    let public_routes = Router::new()
        .merge(routes::auth::public_router())
        .merge(routes::devices::public_router());

    let protected_routes = Router::new()
        .merge(routes::auth::protected_router())
        .nest("/blocklist", routes::blocklist::router())
        .nest("/devices", routes::devices::protected_router())
        .route_layer(axum_mw::from_fn_with_state(
            state.clone(),
            middleware::require_auth,
        ));

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .merge(public_routes)
        .merge(protected_routes)
        // CORS permissivo só para desenvolvimento. Em prod, restringir ao
        // domínio do frontend.
        .layer(CorsLayer::permissive())
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

    axum::serve(listener, app).await.expect("Servidor falhou");
}
