// =============================================================================
// Boot do app Tauri.
// =============================================================================
// No setup:
//   1. Registra o plugin de log em debug.
//   2. Inicializa o SQLCipher local (db::init) e registra a Connection
//      como state — qualquer comando pega via `State<'_, Connection>`.
//   3. Cria o Engine de bloqueio em estado parado e registra como state
//      (dentro de Arc<Mutex<_>> pra permitir o resume assíncrono).
//   4. Se `blocking_enabled=true` estava persistido, spawna uma task que
//      reativa o engine com as regras do último user — UX "abriu o app e
//      continua bloqueando sem ter que clicar de novo".
// =============================================================================

mod blocking;
mod commands;
mod db;

use std::sync::Arc;

use tauri::Manager;
use tokio::sync::Mutex;

use blocking::engine::Engine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Boot síncrono do DB — só roda uma vez, antes da janela abrir.
            let handle = app.handle().clone();
            let conn = tauri::async_runtime::block_on(async move { db::init(&handle).await })?;

            let engine = Arc::new(Mutex::new(Engine::new()));

            // Resume do engine em background. Não bloqueia o setup — se falhar
            // (sem admin pra porta 53, por ex.), o usuário vê a UI com estado
            // correto e pode tentar de novo. Usamos clones pra não prender a
            // conexão/engine originais aqui.
            let conn_resume = conn.clone();
            let engine_resume = engine.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = resume_engine_if_enabled(&conn_resume, &engine_resume).await {
                    tracing::warn!(error = %e, "não foi possível reativar engine no boot");
                }
            });

            app.manage(conn);
            app.manage(engine);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_version,
            commands::list_cached_blocklist,
            commands::save_blocklist,
            commands::cache_add_item,
            commands::cache_remove_item,
            commands::set_blocking_enabled,
            commands::set_adult_filter_enabled,
            commands::get_blocking_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn resume_engine_if_enabled(
    conn: &tokio_rusqlite::Connection,
    engine: &Mutex<Engine>,
) -> anyhow::Result<()> {
    if !db::get_blocking_enabled(conn).await? {
        return Ok(());
    }
    let Some(user_id) = db::get_last_active_user_id(conn).await? else {
        tracing::info!("flag blocking_enabled=true mas sem last_active_user_id — pulando resume");
        return Ok(());
    };
    let rules = db::list_active_domains(conn, user_id.clone()).await?;
    let mut eng = engine.lock().await;
    if eng.is_running() {
        return Ok(());
    }
    eng.start(rules)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    tracing::info!(%user_id, "engine reativado no boot");
    Ok(())
}
