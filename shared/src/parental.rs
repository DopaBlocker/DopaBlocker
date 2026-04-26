// =============================================================================
// Regra do "pai imune" como pure function tipada.
// =============================================================================
// O DopaBlocker tem dois modos: Pessoal (a propria pessoa) e Parental (pai
// gerencia os filhos). No modo Parental, a blocklist NAO se aplica ao
// dispositivo do pai — so aos dispositivos dos filhos. Caso contrario, o
// pai bloquearia a si mesmo ao adicionar um site, o que nao faz sentido na
// proposta do produto.
//
// Esta funcao encapsula essa decisao em UM LUGAR SO, para que o engine de
// bloqueio (DNS proxy do desktop, VPN service do mobile) decida igual,
// sem reimplementar a logica em cada plataforma.
//
// O mobile (Flutter/Dart) reimplementa em 4 linhas — nao vale o custo de
// FFI Rust→Dart so para isso. O importante e que o `effective_strategy`
// permanece como contrato de referencia + os testes garantem que qualquer
// mudanca de regra atualiza ambos os lados.
// =============================================================================

use crate::models::BlockMode;

/// O que o engine deve fazer com a blocklist do usuario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlocklistStrategy {
    /// Aplicar todos os items: bloquear o que estiver na lista. Comportamento
    /// padrao para uso pessoal e para devices filhos em modo parental.
    ApplyAll,
    /// Nao bloquear nada (lista efetiva vazia). Usado no device do pai em
    /// modo parental — ele e quem GERENCIA a lista mas nao a sofre.
    Empty,
}

/// Decide a estrategia de bloqueio com base no modo da conta e se o device
/// atual e de filho.
///
/// | mode      | is_child | resultado    |
/// |-----------|----------|--------------|
/// | Personal  | false    | ApplyAll     |
/// | Personal  | true     | ApplyAll (*) |
/// | Parental  | false    | Empty        |
/// | Parental  | true     | ApplyAll     |
///
/// (*) Combinacao tecnicamente impossivel — `is_child=true` so existe sob
/// um `User` com `mode=Parental`. Mas tratamos como `ApplyAll` por seguranca:
/// se algum dia o invariante for quebrado, o resultado mais conservador e
/// aplicar a lista (e nao deixar o filho sem bloqueio).
pub fn effective_strategy(mode: BlockMode, is_child: bool) -> BlocklistStrategy {
    match (mode, is_child) {
        (BlockMode::Personal, _) => BlocklistStrategy::ApplyAll,
        (BlockMode::Parental, true) => BlocklistStrategy::ApplyAll,
        (BlockMode::Parental, false) => BlocklistStrategy::Empty,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personal_mode_always_applies_blocklist() {
        assert_eq!(
            effective_strategy(BlockMode::Personal, false),
            BlocklistStrategy::ApplyAll,
        );
        // Combinacao impossivel mas testada por seguranca.
        assert_eq!(
            effective_strategy(BlockMode::Personal, true),
            BlocklistStrategy::ApplyAll,
        );
    }

    #[test]
    fn parental_parent_device_is_immune() {
        // Device do pai (is_child=false) em modo parental NAO recebe bloqueios.
        assert_eq!(
            effective_strategy(BlockMode::Parental, false),
            BlocklistStrategy::Empty,
        );
    }

    #[test]
    fn parental_child_device_applies_full_blocklist() {
        // Device do filho recebe a lista que o pai definiu.
        assert_eq!(
            effective_strategy(BlockMode::Parental, true),
            BlocklistStrategy::ApplyAll,
        );
    }
}
