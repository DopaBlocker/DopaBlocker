// =============================================================================
// bootstrap — inicialização do estado da aplicação.
// =============================================================================
// Concentra o I/O de subida (os pontos que dão `panic`/`expect`): abre o
// SQLCipher (com `PRAGMA key` primeiro), roda as migrations idempotentes e
// monta o `AppState` compartilhado. Mantido separado de `app.rs` (montagem do
// router, pura) e de `main.rs` (boot fino).
// =============================================================================

use crate::core::auth::JwksCache;
use crate::core::config::AppConfig;
use crate::core::db;
use crate::AppState;

/// Abre o arquivo .db, aplica `PRAGMA key = '<chave>'` IMEDIATAMENTE (é isso que
/// descriptografa o SQLCipher) e só então roda as migrations. `JwksCache::new()`
/// cria um cache vazio — as chaves do Firebase são baixadas na primeira
/// validação de JWT (lazy) e depois cacheadas por 6h.
///
/// Falhas críticas (DB inacessível, chave errada) causam `panic` via `.expect`
/// — comportamento desejado: fail fast no boot, não subir num estado quebrado.
pub async fn init_state(config: AppConfig) -> AppState {
    let db = db::connect(&config.database_path, &config.database_key)
        .await
        .expect("Falha ao conectar ao SQLCipher");
    db::run_migrations(&db)
        .await
        .expect("Falha ao aplicar migrations");

    AppState {
        config,
        db,
        jwks: JwksCache::new(),
    }
}
