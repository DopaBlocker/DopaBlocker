// =============================================================================
// Origem do bloqueio — compartilhado entre DNS proxy e block page.
// =============================================================================
// O DNS proxy usa só pra decidir "bloqueio ou não". O block page (HTTP/HTTPS)
// usa pra renderizar a razão ("Na sua lista" vs "Filtro adulto"). Manter a
// lógica aqui garante que as duas camadas concordem sobre quando um domínio
// é bloqueado.
// =============================================================================

use std::collections::HashSet;

use tokio::sync::RwLock;

use super::adult_filter::AdultFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    UserList,
    AdultFilter,
}

impl BlockReason {
    pub fn as_text(self) -> &'static str {
        match self {
            BlockReason::UserList => "Na sua lista de bloqueios",
            BlockReason::AdultFilter => "Filtro de conteúdo adulto",
        }
    }
}

/// Retorna a origem do bloqueio se houver — prioriza lista do usuário
/// (comportamento atual do dns_proxy: checa rules antes do adulto).
pub async fn check(
    domain: &str,
    rules: &RwLock<HashSet<String>>,
    adult: &AdultFilter,
) -> Option<BlockReason> {
    if is_in_rules(domain, rules).await {
        return Some(BlockReason::UserList);
    }
    if is_in_adult(domain, adult) {
        return Some(BlockReason::AdultFilter);
    }
    None
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
}
