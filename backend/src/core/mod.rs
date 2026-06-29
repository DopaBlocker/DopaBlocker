// =============================================================================
// core — infraestrutura compartilhada do backend.
// =============================================================================
// Agrupa o que NÃO pertence a uma feature específica e é reusado por todas:
// configuração, banco, erros, modelos/DTOs, utilitários de domínio e o cluster
// de autenticação (middleware dual Firebase JWT + Device Token).
//
// Espelha o `lib/core/` do mobile (infra sem UI): aqui não mora regra de
// negócio de nenhuma feature — só os blocos transversais que elas reutilizam.
// =============================================================================

pub mod auth;
pub mod config;
pub mod db;
pub mod errors;
pub mod models;
pub mod util;
