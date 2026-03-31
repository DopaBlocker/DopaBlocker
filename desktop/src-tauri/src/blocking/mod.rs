// Módulo de bloqueio — orquestra WFP, DNS proxy e filtro adulto.
// Sub-módulos: engine (orquestrador), wfp (Windows Filtering Platform),
// dns_proxy (DNS resolver local), adult_filter (bloom filter de domínios).

pub mod engine;
pub mod wfp;
pub mod dns_proxy;
pub mod adult_filter;
