// Data plane do DNS proxy: escuta no loopback (:53) e decide, por query,
// entre bloquear, servir do cache ou encaminhar ao upstream.

pub mod cache;
pub mod proxy;
pub mod upstream;

/// Tamanho máximo de um pacote DNS lido em buffer (query UDP e resposta UDP do
/// upstream). 4096 cobre EDNS0; acima disso o cliente usa TCP. Compartilhado
/// por `proxy` e `upstream`.
pub(crate) const MAX_DNS_PACKET: usize = 4096;
