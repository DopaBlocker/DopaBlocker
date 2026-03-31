// Handlers de comandos IPC do Tauri — chamados pelo frontend via invoke().
// Implementar: #[tauri::command] para cada operação:
// - get_blocklist() -> Vec<BlockedItem>
// - add_blocked_item(item_type, value) -> Result
// - remove_blocked_item(id) -> Result
// - toggle_blocking(enabled: bool) -> Result (liga/desliga o engine)
// - toggle_adult_filter(enabled: bool) -> Result
// - generate_link_code() -> String (código 6 dígitos)
// - confirm_link_code(code: String) -> Result
// - get_linked_devices() -> Vec<Device>
