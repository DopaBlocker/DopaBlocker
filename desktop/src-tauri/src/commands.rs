// =============================================================================
// Handlers de comandos IPC do Tauri — chamados pelo frontend via invoke().
// =============================================================================
// Divisão de responsabilidades:
//   - O **backend REST** (Axum) é a fonte-da-verdade de auth e blocklist.
//   - O **frontend** fala HTTP com o backend (via `api.ts` usando Firebase
//     JWT) e espelha as mudanças no cache local via estes comandos.
//   - O **engine** (DNS proxy) lê do cache local. Então toda mutação no
//     cache precisa, se o engine estiver rodando, propagar as novas regras
//     via `engine.update_rules` — senão o bloqueio fica stale.
// =============================================================================

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;
use tokio_rusqlite::Connection;

use dopablocker_shared::models::BlockedItem;

use crate::blocking::engine::Engine;
use crate::db;

#[derive(Debug, Serialize)]
pub struct BlockingStatus {
    pub enabled: bool,
    pub adult_filter_enabled: bool,
    pub item_count: usize,
}

fn stringify<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// Recarrega as regras ativas do DB para o engine — chamar sempre que o
/// cache local muda e o engine estiver rodando. Se não estiver, no-op.
async fn refresh_engine_rules(
    conn: &Connection,
    engine: &Mutex<Engine>,
    user_id: String,
) -> Result<(), String> {
    let eng = engine.lock().await;
    if !eng.is_running() {
        return Ok(());
    }
    let rules = db::list_active_domains(conn, user_id)
        .await
        .map_err(stringify)?;
    eng.update_rules(rules).await;
    Ok(())
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
/// frontend fazer GET /blocklist no backend.
#[tauri::command]
pub async fn save_blocklist(
    conn: State<'_, Connection>,
    engine: State<'_, Arc<Mutex<Engine>>>,
    user_id: String,
    items: Vec<BlockedItem>,
) -> Result<(), String> {
    db::replace_all_for_user(&conn, user_id.clone(), items)
        .await
        .map_err(stringify)?;
    refresh_engine_rules(&conn, &engine, user_id).await
}

#[tauri::command]
pub async fn cache_add_item(
    conn: State<'_, Connection>,
    engine: State<'_, Arc<Mutex<Engine>>>,
    item: BlockedItem,
) -> Result<(), String> {
    let user_id = item.user_id.clone();
    db::upsert_blocked_item(&conn, item).await.map_err(stringify)?;
    refresh_engine_rules(&conn, &engine, user_id).await
}

#[tauri::command]
pub async fn cache_remove_item(
    conn: State<'_, Connection>,
    engine: State<'_, Arc<Mutex<Engine>>>,
    id: String,
    user_id: String,
) -> Result<(), String> {
    db::delete_blocked_item(&conn, id).await.map_err(stringify)?;
    refresh_engine_rules(&conn, &engine, user_id).await
}

/// Liga/desliga o engine de bloqueio (DNS proxy).
///
/// Requer `user_id` porque, ao ligar, carregamos as regras do DB filtradas
/// pelo dono. Idempotente: chamar com engine já rodando re-aplica as regras
/// sem derrubar a task.
#[tauri::command]
pub async fn set_blocking_enabled(
    conn: State<'_, Connection>,
    engine: State<'_, Arc<Mutex<Engine>>>,
    user_id: String,
    enabled: bool,
) -> Result<(), String> {
    {
        let mut eng = engine.lock().await;
        if enabled {
            let rules = db::list_active_domains(&conn, user_id.clone())
                .await
                .map_err(stringify)?;
            if eng.is_running() {
                eng.update_rules(rules).await;
            } else {
                eng.start(rules).await.map_err(stringify)?;
            }
        } else {
            eng.stop().await;
        }
    }
    // Persiste flag + user ativo para reativação no próximo boot do app.
    db::set_blocking_enabled(&conn, enabled)
        .await
        .map_err(stringify)?;
    db::set_last_active_user_id(&conn, user_id)
        .await
        .map_err(stringify)?;
    Ok(())
}

#[tauri::command]
pub async fn set_adult_filter_enabled(
    conn: State<'_, Connection>,
    enabled: bool,
) -> Result<(), String> {
    // TODO(etapa 8): integrar com adult_filter (Bloom Filter de domínios).
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
