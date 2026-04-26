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

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex;
use tokio_rusqlite::Connection;

use dopablocker_shared::models::{BlockMode, BlockedItem};
use dopablocker_shared::parental::{effective_strategy, BlocklistStrategy};

use crate::blocking::adult_filter::AdultFilter;
use crate::blocking::ca::{InstallStatus, LocalCa};
use crate::blocking::engine::Engine;
use crate::blocking::system_dns;
use crate::db;
use crate::AppPaths;

/// Contexto da regra do pai imune que o frontend envia em cada operacao do
/// engine. Sem isto, o engine nao tem como distinguir um device pai (que
/// deve ficar imune) de um pessoal (que aplica tudo) — a info nao vive no
/// SQLCipher local, mora no auth store do frontend.
#[derive(Debug, Clone, Deserialize)]
pub struct ParentalContext {
    pub mode: BlockMode,
    pub is_child: bool,
}

impl ParentalContext {
    fn personal() -> Self {
        Self {
            mode: BlockMode::Personal,
            is_child: false,
        }
    }
}

async fn effective_rules(
    conn: &Connection,
    user_id: String,
    ctx: &ParentalContext,
) -> Result<Vec<String>, String> {
    match effective_strategy(ctx.mode.clone(), ctx.is_child) {
        BlocklistStrategy::Empty => Ok(Vec::new()),
        BlocklistStrategy::ApplyAll => db::list_active_domains(conn, user_id)
            .await
            .map_err(stringify),
    }
}

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

/// Recarrega as regras ativas do DB para o engine — chamar sempre que o
/// cache local muda e o engine estiver rodando. Se não estiver, no-op.
///
/// Aplica a "regra do pai imune": no device do pai em modo parental, as regras
/// efetivas sao vazias (lista cheia continua no DB para a UI mostrar, so o
/// engine nao recebe).
async fn refresh_engine_rules(
    conn: &Connection,
    engine: &Mutex<Engine>,
    user_id: String,
    ctx: &ParentalContext,
) -> Result<(), String> {
    let eng = engine.lock().await;
    if !eng.is_running() {
        return Ok(());
    }
    let rules = effective_rules(conn, user_id, ctx).await?;
    eng.update_rules(rules).await;
    Ok(())
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
    refresh_engine_rules(&conn, &engine, user_id, &ctx).await
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
    refresh_engine_rules(&conn, &engine, user_id, &ctx).await
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
    refresh_engine_rules(&conn, &engine, user_id, &ctx).await
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
    parental: Option<ParentalContext>,
) -> Result<(), String> {
    let ctx = parental.unwrap_or_else(ParentalContext::personal);
    if enabled {
        // 1. Sobe o proxy. Se falhar no bind (porta 53 sem admin, já ocupada),
        //    nem chegamos a mexer no DNS do sistema.
        //
        // Aplica a regra do pai imune: device do pai em modo parental recebe
        // lista vazia (engine roda mas nao bloqueia nada). Device pessoal e
        // device filho recebem a lista cheia.
        let rules = effective_rules(&conn, user_id.clone(), &ctx).await?;
        {
            let mut eng = engine.lock().await;
            if eng.is_running() {
                eng.update_rules(rules).await;
            } else {
                eng.start(rules).await.map_err(stringify)?;
            }
        }

        // 2. Aponta o DNS do sistema pro proxy. Se falhar (tipicamente admin),
        //    rollback no engine — melhor desligado do que meio-configurado.
        if let Err(e) = system_dns::apply_and_remember(&conn).await {
            let mut eng = engine.lock().await;
            eng.stop().await;
            return Err(format!("falha ao trocar DNS do sistema: {e}"));
        }
    } else {
        // Ordem importa: restaura o DNS ANTES de matar o proxy. Se matasse
        // primeiro, haveria uma janela de segundos em que o sistema ainda
        // aponta pra 127.0.0.1:53 mas ninguém está escutando → DNS quebrado.
        if let Err(e) = system_dns::restore_if_any(&conn).await {
            tracing::warn!(error = %e, "falha ao restaurar DNS — seguindo pra parar engine");
        }
        let mut eng = engine.lock().await;
        eng.stop().await;
    }

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
