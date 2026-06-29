// =============================================================================
// features — domínios de negócio do backend.
// =============================================================================
// Cada feature agrupa seus handlers HTTP (`routes`) e a lógica de domínio
// (`service` e módulos afins), espelhando o `lib/features/` do mobile. O padrão
// "rotas chamam serviços" continua valendo DENTRO de cada feature: `routes.rs`
// faz parsing/extração de auth e delega a regra de negócio aos demais módulos.
// =============================================================================

pub mod auth;
pub mod blocklist;
pub mod devices;
