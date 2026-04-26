// DopaBlocker Shared Crate
// Módulos compartilhados entre backend, desktop e mobile (via FFI).
// Contém modelos de dados, bloom filter e utilitários de matching de domínios.

pub mod bloom_filter;
pub mod domain_matcher;
pub mod models;
pub mod parental;
