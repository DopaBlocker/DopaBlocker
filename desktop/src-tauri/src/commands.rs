// =============================================================================
// Handlers de comandos IPC do Tauri — chamados pelo frontend via invoke().
// =============================================================================
// Divisão de responsabilidades nesta arquitetura:
//   - O **backend REST** (Axum) é a fonte-da-verdade de auth e blocklist.
//   - O **frontend** fala HTTP com o backend (via `api.ts` usando Firebase JWT)
//     e espelha as mudanças no cache local via estes comandos.
//   - Os comandos aqui NÃO chamam o backend — só persistem no SQLCipher local
//     e (no futuro) acionam o engine de bloqueio. Isso mantém a JWT/refresh
//     toda no lado JS (Firebase SDK) e evita duplicar HTTP client em Rust.
//
// O engine de bloqueio (DNS proxy + WFP + adult filter) ainda não existe —
// as etapas 6-9 do plano. Por ora, `set_blocking_enabled` e
// `set_adult_filter_enabled` apenas persistem o flag. Quando o engine
// chegar, estes mesmos comandos também farão start/stop.
// =============================================================================

use serde::Serialize;
use tauri::State;
use tokio_rusqlite::Connection;

use dopablocker_shared::models::BlockedItem;

use crate::db;

#[derive(Debug, Serialize)]
pub struct BlockingStatus {
    pub enabled: bool,
    pub adult_filter_enabled: bool,
    pub item_count: usize,
}

// Tauri serializa `String` como erro no lado JS. Nossos erros viram mensagens
// legíveis — o frontend mostra em toast/form-error.
fn stringify<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub async fn list_cached_blocklist(
    conn: State<'_, Connection>,
    user_id: String,
) -> Result<Vec<BlockedItem>, String> {
    db::list_blocked_items(&conn, user_id)
        .await
        .map_err(stringify)
}

/// Substitui o cache inteiro de um usuário atomicamente — chamado após o
/// frontend fazer GET /blocklist no backend. Alternativa mais robusta que
/// tentar reconciliar diff no JS.
#[tauri::command]
pub async fn save_blocklist(
    conn: State<'_, Connection>,
    user_id: String,
    items: Vec<BlockedItem>,
) -> Result<(), String> {
    db::replace_all_for_user(&conn, user_id, items)
        .await
        .map_err(stringify)
}

/// Espelha no cache local um item que o frontend acabou de criar via POST
/// /blocklist. O `item` vem com `id` e `created_at` preenchidos pelo backend.
#[tauri::command]
pub async fn cache_add_item(
    conn: State<'_, Connection>,
    item: BlockedItem,
) -> Result<(), String> {
    db::upsert_blocked_item(&conn, item).await.map_err(stringify)
}

#[tauri::command]
pub async fn cache_remove_item(conn: State<'_, Connection>, id: String) -> Result<(), String> {
    db::delete_blocked_item(&conn, id).await.map_err(stringify)
}

#[tauri::command]
pub async fn set_blocking_enabled(
    conn: State<'_, Connection>,
    enabled: bool,
) -> Result<(), String> {
    // TODO(etapa 7): quando engine existir, chamar engine::start/stop aqui.
    db::set_blocking_enabled(&conn, enabled)
        .await
        .map_err(stringify)
}

#[tauri::command]
pub async fn set_adult_filter_enabled(
    conn: State<'_, Connection>,
    enabled: bool,
) -> Result<(), String> {
    // TODO(etapa 8): quando adult_filter existir, chamar adult_filter::enable/disable.
    db::set_adult_filter_enabled(&conn, enabled)
        .await
        .map_err(stringify)
}

#[tauri::command]
pub async fn get_blocking_status(
    conn: State<'_, Connection>,
    user_id: String,
) -> Result<BlockingStatus, String> {
    let enabled = db::get_blocking_enabled(&conn).await.map_err(stringify)?;
    let adult_filter_enabled = db::get_adult_filter_enabled(&conn)
        .await
        .map_err(stringify)?;
    let items = db::list_blocked_items(&conn, user_id)
        .await
        .map_err(stringify)?;
    Ok(BlockingStatus {
        enabled,
        adult_filter_enabled,
        item_count: items.len(),
    })
}
