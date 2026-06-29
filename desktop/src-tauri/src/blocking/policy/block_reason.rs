// =============================================================================
// Origem do bloqueio — compartilhado entre DNS proxy e block page.
// =============================================================================
// O DNS proxy usa só pra decidir "bloqueio ou não". O block page (HTTP/HTTPS)
// usa pra renderizar a razão ("Na sua lista" vs "Filtro adulto"). Manter a
// lógica aqui garante que as duas camadas concordem sobre quando um domínio
// é bloqueado.
//
// `DohEndpoint` cobre o gap C2 (frente B): mesmo que o WFP nao tenha o IP do
// resolver na lista curada, se o cliente ainda precisar resolver o FQDN do
// provedor (ex: `dns.google`) via DNS plain, este check intercepta e nega.
// Lista bundled em `shared/data/doh-fqdns.txt` via include_str!.
// =============================================================================

use std::collections::HashSet;
use std::sync::OnceLock;

use tokio::sync::RwLock;

use super::adult_filter::AdultFilter;

/// Lista bundled de FQDNs de provedores DoH/DoT conhecidos. Atualizada
/// manualmente. Build-time embed via include_str! — sem I/O em runtime.
const DOH_FQDNS_RAW: &str = include_str!("../../../../../shared/data/doh-fqdns.txt");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    UserList,
    AdultFilter,
    DohEndpoint,
}

impl BlockReason {
    pub fn as_text(self) -> &'static str {
        match self {
            BlockReason::UserList => "Na sua lista de bloqueios",
            BlockReason::AdultFilter => "Filtro de conteúdo adulto",
            BlockReason::DohEndpoint => "Provedor DoH/DoT bloqueado",
        }
    }
}

/// Retorna a origem do bloqueio se houver. Ordem importa:
///   1. DohEndpoint — sempre prioritario, evita que cliente DoH bypasse o proxy.
///   2. UserList — lista do usuario (pessoal ou pai).
///   3. AdultFilter — Bloom Filter do filtro adulto.
pub async fn check(
    domain: &str,
    rules: &RwLock<HashSet<String>>,
    adult: &AdultFilter,
) -> Option<BlockReason> {
    if is_doh_endpoint(domain) {
        return Some(BlockReason::DohEndpoint);
    }
    if is_in_rules(domain, rules).await {
        return Some(BlockReason::UserList);
    }
    if is_in_adult(domain, adult) {
        return Some(BlockReason::AdultFilter);
    }
    None
}

/// True se `domain` (ja normalizado: lowercase, sem trailing dot) for um FQDN
/// DoH conhecido OU subdominio de um. Walk subdomain por subdomain — mesma
/// estrategia de `is_in_rules`.
fn is_doh_endpoint(domain: &str) -> bool {
    let fqdns = doh_fqdns();
    let mut current = domain;
    loop {
        if fqdns.contains(current) {
            return true;
        }
        match current.find('.') {
            Some(idx) => current = &current[idx + 1..],
            None => return false,
        }
    }
}

/// Lazy init do HashSet de FQDNs DoH a partir do `include_str!`. Strings sao
/// `&'static str` pois apontam dentro do binario — zero alloc por entrada.
fn doh_fqdns() -> &'static HashSet<&'static str> {
    static CACHE: OnceLock<HashSet<&'static str>> = OnceLock::new();
    CACHE.get_or_init(|| {
        DOH_FQDNS_RAW
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    None
                } else {
                    Some(trimmed)
                }
            })
            .collect()
    })
}

async fn is_in_rules(domain: &str, rules: &RwLock<HashSet<String>>) -> bool {
    let rules = rules.read().await;
    let mut current = domain;
    loop {
        if rules.contains(current) {
            return true;
        }
        match current.find('.') {
            Some(idx) => current = &current[idx + 1..],
            None => return false,
        }
    }
}

fn is_in_adult(domain: &str, adult: &AdultFilter) -> bool {
    if !adult.is_enabled() {
        return false;
    }
    let mut current = domain;
    loop {
        if adult.contains(current) {
            return true;
        }
        match current.find('.') {
            Some(idx) => current = &current[idx + 1..],
            None => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn setup(rules: &[&str]) -> (RwLock<HashSet<String>>, AdultFilter) {
        let set: HashSet<String> = rules.iter().map(|s| s.to_string()).collect();
        let tmp = std::env::temp_dir().join(format!(
            "dopablocker_block_reason_test_{}",
            uuid::Uuid::new_v4()
        ));
        let af = AdultFilter::new(PathBuf::from(tmp), false);
        (RwLock::new(set), af)
    }

    #[tokio::test]
    async fn matches_user_list_exact() {
        let (rules, adult) = setup(&["instagram.com"]);
        assert_eq!(
            check("instagram.com", &rules, &adult).await,
            Some(BlockReason::UserList)
        );
    }

    #[tokio::test]
    async fn matches_user_list_subdomain() {
        let (rules, adult) = setup(&["instagram.com"]);
        assert_eq!(
            check("www.instagram.com", &rules, &adult).await,
            Some(BlockReason::UserList)
        );
    }

    #[tokio::test]
    async fn returns_none_when_not_blocked() {
        let (rules, adult) = setup(&["instagram.com"]);
        assert_eq!(check("google.com", &rules, &adult).await, None);
    }

    #[tokio::test]
    async fn adult_filter_disabled_returns_none() {
        // AdultFilter.contains() é false quando !is_enabled; adult check pula.
        let (rules, adult) = setup(&[]);
        assert_eq!(check("pornhub.com", &rules, &adult).await, None);
    }

    #[tokio::test]
    async fn matches_doh_fqdn_exact() {
        let (rules, adult) = setup(&[]);
        assert_eq!(
            check("dns.google", &rules, &adult).await,
            Some(BlockReason::DohEndpoint)
        );
    }

    #[tokio::test]
    async fn matches_doh_fqdn_subdomain() {
        let (rules, adult) = setup(&[]);
        assert_eq!(
            check("v2.cloudflare-dns.com", &rules, &adult).await,
            Some(BlockReason::DohEndpoint)
        );
    }

    #[tokio::test]
    async fn doh_check_runs_before_user_list() {
        // Mesmo se o usuario tenha `dns.google` na propria lista, o motivo
        // reportado deve ser DohEndpoint (ordem fixada na funcao check).
        let (rules, adult) = setup(&["dns.google"]);
        assert_eq!(
            check("dns.google", &rules, &adult).await,
            Some(BlockReason::DohEndpoint)
        );
    }

    #[tokio::test]
    async fn doh_list_contains_known_providers() {
        // Sanity check — se a lista bundled mudar, este teste ajuda a pegar
        // remocao acidental dos provedores principais.
        let fqdns = doh_fqdns();
        assert!(fqdns.contains("dns.google"));
        assert!(fqdns.contains("cloudflare-dns.com"));
        assert!(fqdns.contains("dns.quad9.net"));
    }
}
