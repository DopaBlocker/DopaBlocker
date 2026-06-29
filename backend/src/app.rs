// =============================================================================
// app — montagem do Router do Axum.
// =============================================================================
// Função pura de montagem (testável sem rede/DB): compõe as rotas públicas e
// protegidas das features, aplica o rate-limit (GCRA por IP) nas públicas de
// auth, o middleware `require_auth` nas protegidas, o CORS por allowlist e os
// health-checks. Tudo o que estava inline no `main.rs` antes da modularização.
//
// A separação entre `public_routes` e `protected_routes` é essencial para
// permitir que `/devices/link/confirm` seja acessada sem credencial — afinal,
// o filho ainda não tem nem Firebase JWT nem Device Token quando confirma o
// código de vinculação pela primeira vez.
// =============================================================================

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
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};
use tower_http::cors::CorsLayer;

use crate::core::auth::require_auth;
use crate::core::config::AppConfig;
use crate::features::{auth, blocklist, devices};
use crate::AppState;

/// Parâmetros do rate-limit das rotas públicas de auth (GCRA por IP).
/// `BURST` = quantas requisições um IP pode disparar de imediato; depois disso,
/// repõe 1 permissão a cada `REPLENISH_SECS` (≈ 12/min sustentado após o burst).
const AUTH_RATE_BURST: u32 = 10;
const AUTH_RATE_REPLENISH_SECS: u64 = 5;

/// Monta o `Router` completo da aplicação com o `state` já injetado.
///
/// `public_routes` = rotas que o middleware global NÃO valida (auth público +
/// `/devices/link/confirm` + `/devices/tamper`), com rate-limit por IP.
/// `protected_routes` = tudo o mais, sob `route_layer(require_auth)` (auth dual
/// Firebase JWT OU Device Token; escrita rejeita Device Token com 403).
/// `/health` e `/healthz` ficam fora de tudo para health-checks de infra.
pub fn build_router(state: AppState) -> Router {
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
        .merge(auth::public_router())
        .merge(devices::public_router())
        .layer(GovernorLayer::new(governor_conf));

    let protected_routes = Router::new()
        .merge(auth::protected_router())
        .nest("/blocklist", blocklist::router())
        .nest("/devices", devices::protected_router())
        .route_layer(axum_mw::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        // `/health` = liveness simples; `/healthz` = readiness que valida o
        // banco SQLCipher (ver `healthz`). Ambos públicos para health-checks.
        .route("/health", get(|| async { "OK" }))
        .route("/healthz", get(healthz))
        .merge(public_routes)
        .merge(protected_routes)
        // CORS por allowlist (config). Em prod, definir `CORS_ALLOWED_ORIGINS`.
        .layer(build_cors(&state.config))
        .with_state(state)
}

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
fn build_cors(config: &AppConfig) -> CorsLayer {
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
