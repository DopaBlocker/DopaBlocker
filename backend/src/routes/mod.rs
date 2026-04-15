// Agrupa os módulos de rotas por domínio de negócio. Cada submódulo
// expõe um ou mais `Router<AppState>` que `main.rs` compõe.
pub mod auth;
pub mod blocklist;
pub mod devices;
