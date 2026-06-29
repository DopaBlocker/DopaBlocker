// =============================================================================
// Feature de devices / controle parental.
// =============================================================================
// Rotas `/devices/*` (registro, listagem, vinculação parental, tamper).
//   - routes  : handlers HTTP (públicos e protegidos).
//   - service : devices + fluxo completo de vinculação parental (transação).
//   - events  : registro/leitura de eventos de adulteração (tamper).
// =============================================================================

pub mod events;
pub mod routes;
pub mod service;

pub use routes::{protected_router, public_router};
