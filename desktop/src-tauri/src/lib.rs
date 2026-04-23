// =============================================================================
// Boot do app Tauri.
// =============================================================================
// No setup:
//   1. Registra o plugin de log em debug.
//   2. Inicializa o SQLCipher local (via db::init) e registra a Connection
//      como state — qualquer comando pode pegar via `State<'_, Connection>`.
//   3. O engine de bloqueio (DNS + WFP + adult filter) será inicializado aqui
//      nas etapas 6-9, também como state.
// =============================================================================

mod blocking;
mod commands;
mod db;

use tauri::Manager;

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

            // Boot síncrono do DB — só roda uma vez na subida do app, antes
            // da janela abrir. `block_on` aqui é aceitável porque nada
            // depende de assíncrono concorrente neste momento.
            let handle = app.handle().clone();
            let conn = tauri::async_runtime::block_on(async move { db::init(&handle).await })?;
            app.manage(conn);

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
