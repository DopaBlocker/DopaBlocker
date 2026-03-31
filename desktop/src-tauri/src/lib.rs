// Builder do app Tauri com registro de comandos IPC.
// Implementar: registrar todos os comandos (get_blocklist, toggle_blocking,
// add_blocked_item, remove_blocked_item, link_device, get_devices, toggle_adult_filter),
// inicializar o SQLite local e o engine de bloqueio (WFP + DNS proxy).

mod commands;
mod db;
mod blocking;

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
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
