// Montagem do router principal.
// Implementar: função create_router() que faz merge dos sub-routers
// de auth, blocklist e devices, aplicando middleware de autenticação
// nas rotas protegidas.

pub mod auth;
pub mod blocklist;
pub mod devices;
