// =============================================================================
// Feature de blocklist.
// =============================================================================
// Rotas `/blocklist/*`: itens bloqueados (domain/app/keyword), filtro adulto e
// ETag/304 para o poll periódico.
//   - routes  : handlers HTTP.
//   - service : CRUD de itens + filtro adulto + cálculo de ETag.
// =============================================================================

pub mod routes;
pub mod service;

pub use routes::router;
