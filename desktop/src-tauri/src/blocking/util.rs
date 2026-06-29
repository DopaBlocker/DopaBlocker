// Helpers pequenos compartilhados entre os submódulos de bloqueio.

/// Lê uma porta de uma variável de ambiente, caindo no `default` se ausente
/// ou inválida. Usado pelos listeners (DNS :53, block page :80/:443) para
/// permitir override em testes/dev sem privilégio de administrador.
pub(crate) fn env_port(name: &str, default: u16) -> u16 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
