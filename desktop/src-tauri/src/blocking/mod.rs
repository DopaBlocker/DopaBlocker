// Módulo de bloqueio — orquestra o DNS proxy (cache + upstream pool),
// WFP (etapa 9) e filtro adulto (etapa 8).

pub mod adult_filter;
pub mod block_page;
pub mod block_reason;
pub mod ca;
pub mod dns_cache;
pub mod dns_proxy;
pub mod dns_upstream;
pub mod engine;
pub mod system_dns;
pub mod tls_resolver;
pub mod wfp;
