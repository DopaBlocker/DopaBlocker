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
use crate::blocking::lifecycle::{self, ParentalContext};
use crate::blocking::page::ca::{InstallStatus, LocalCa};
use crate::blocking::policy::adult_filter::AdultFilter;
use crate::db;
use crate::AppPaths;

#[derive(Debug, Serialize)]
pub struct BlockingStatus {
    pub enabled: bool,
    pub adult_filter_enabled: bool,
    /// True quando o filtro está enabled mas o Bloom ainda não foi
    /// construído (download + populate em background após o boot).
    pub adult_filter_building: bool,
    pub item_count: usize,
}

#[derive(Debug, Serialize)]
pub struct CaInstallResult {
    pub status: String,
    pub thumbprint: String,
}

fn stringify<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub async fn install_ca_root(paths: State<'_, AppPaths>) -> Result<CaInstallResult, String> {
    let ca = LocalCa::load_or_create(&paths.data_dir).map_err(stringify)?;
    let status = match ca.install_in_windows_root() {
        InstallStatus::Installed => "installed",
        InstallStatus::AlreadyPresent => "already_present",
        InstallStatus::Failed => "failed",
    };
    Ok(CaInstallResult {
        status: status.to_string(),
        thumbprint: ca.thumbprint().to_string(),
    })
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
    parental: Option<ParentalContext>,
) -> Result<(), String> {
    let ctx = parental.unwrap_or_else(ParentalContext::personal);
    db::replace_all_for_user(&conn, user_id.clone(), items)
        .await
        .map_err(stringify)?;
    lifecycle::refresh_engine_rules(&conn, &engine, user_id, &ctx).await
}

#[tauri::command]
pub async fn cache_add_item(
    conn: State<'_, Connection>,
    engine: State<'_, Arc<Mutex<Engine>>>,
    item: BlockedItem,
    parental: Option<ParentalContext>,
) -> Result<(), String> {
    let ctx = parental.unwrap_or_else(ParentalContext::personal);
    let user_id = item.user_id.clone();
    db::upsert_blocked_item(&conn, item)
        .await
        .map_err(stringify)?;
    lifecycle::refresh_engine_rules(&conn, &engine, user_id, &ctx).await
}

#[tauri::command]
pub async fn cache_remove_item(
    conn: State<'_, Connection>,
    engine: State<'_, Arc<Mutex<Engine>>>,
    id: String,
    user_id: String,
    parental: Option<ParentalContext>,
) -> Result<(), String> {
    let ctx = parental.unwrap_or_else(ParentalContext::personal);
    db::delete_blocked_item(&conn, id)
        .await
        .map_err(stringify)?;
    lifecycle::refresh_engine_rules(&conn, &engine, user_id, &ctx).await
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
    paths: State<'_, AppPaths>,
    user_id: String,
    enabled: bool,
    parental: Option<ParentalContext>,
) -> Result<(), String> {
    let ctx = parental.unwrap_or_else(ParentalContext::personal);
    if enabled {
        lifecycle::enable(&engine, &conn, &paths.data_dir, user_id, &ctx)
            .await
            .map_err(stringify)
    } else {
        lifecycle::disable(&engine, &conn, &paths.data_dir, user_id)
            .await
            .map_err(stringify)
    }
}

#[tauri::command]
pub async fn set_adult_filter_enabled(
    conn: State<'_, Connection>,
    adult_filter: State<'_, Arc<AdultFilter>>,
    enabled: bool,
) -> Result<(), String> {
    // AtomicBool primeiro (o DNS proxy vê o toggle imediatamente na próxima
    // query); persistência depois, pra que o próximo boot recarregue o mesmo
    // estado no AdultFilter::new.
    adult_filter.set_enabled(enabled);
    db::set_adult_filter_enabled(&conn, enabled)
        .await
        .map_err(stringify)
}

#[tauri::command]
pub async fn get_blocking_status(
    conn: State<'_, Connection>,
    adult_filter: State<'_, Arc<AdultFilter>>,
    user_id: String,
) -> Result<BlockingStatus, String> {
    let enabled = db::get_blocking_enabled(&conn).await.map_err(stringify)?;
    let adult_filter_enabled = db::get_adult_filter_enabled(&conn)
        .await
        .map_err(stringify)?;
    let adult_filter_building = adult_filter_enabled && !adult_filter.is_built();
    let items = db::list_blocked_items(&conn, user_id)
        .await
        .map_err(stringify)?;
    Ok(BlockingStatus {
        enabled,
        adult_filter_enabled,
        adult_filter_building,
        item_count: items.len(),
    })
}

// -------- child_session ------------------------------------------------------
//
// Persistencia da sessao do filho. O frontend chama save apos confirmar o
// codigo de vinculacao, load no boot (para restaurar a sessao), e clear no
// logout. Ver `desktop/src/lib/stores/auth.ts`.

#[tauri::command]
pub async fn save_child_session(
    conn: State<'_, Connection>,
    user_id: String,
    device_id: String,
    device_token: String,
    parent_device_id: String,
) -> Result<(), String> {
    db::save_child_session(
        &conn,
        db::ChildSession {
            user_id,
            device_id,
            device_token,
            parent_device_id,
        },
    )
    .await
    .map_err(stringify)
}

#[tauri::command]
pub async fn load_child_session(
    conn: State<'_, Connection>,
) -> Result<Option<db::ChildSession>, String> {
    db::load_child_session(&conn).await.map_err(stringify)
}

#[tauri::command]
pub async fn clear_child_session(conn: State<'_, Connection>) -> Result<(), String> {
    db::clear_child_session(&conn).await.map_err(stringify)
}
