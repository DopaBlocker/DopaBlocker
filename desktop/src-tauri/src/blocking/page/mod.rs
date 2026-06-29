// Página de bloqueio servida quando o DNS proxy redireciona um domínio
// bloqueado para 127.0.0.1. O `server` atende HTTP(:80) e HTTPS(:443); o
// HTTPS usa a `ca` local com o resolver SNI dinâmico (`tls_resolver`).

pub mod ca;
pub mod server;
pub mod tls_resolver;
