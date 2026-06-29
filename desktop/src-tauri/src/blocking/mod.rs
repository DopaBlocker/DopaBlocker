// Módulo de bloqueio — agrupado por responsabilidade:
//   - dns/      : data plane do DNS proxy (proxy, cache, upstream)
//   - page/     : página de bloqueio HTTP/HTTPS + CA local + resolver SNI
//   - policy/   : decisão de bloqueio (block_reason) e filtro adulto
//   - os/       : enforcement no SO (WFP, DNS do sistema)
//   - engine    : sobe/derruba o stack in-process (WFP, CA, páginas, DNS proxy)
//   - lifecycle : dono único da orquestração (engine + DNS do sistema + flag no DB)

pub mod dns;
pub mod engine;
pub mod lifecycle;
pub mod os;
pub mod page;
pub mod policy;
pub(crate) mod util;
