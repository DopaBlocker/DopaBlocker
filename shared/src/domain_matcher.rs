// Utilitários de normalização e matching de domínios.
// Usado tanto pelo DNS proxy quanto pelo bloom filter.

/// Remove protocolo, `www.` e path, retornando apenas o domínio em minúsculas.
///
/// Exemplos:
/// - `"https://www.YouTube.com/watch?v=123"` → `"youtube.com"`
/// - `"http://facebook.com/"` → `"facebook.com"`
/// - `"sub.example.com"` → `"sub.example.com"`
pub fn normalize_domain(input: &str) -> String {
    let s = input.to_lowercase();
    let s = s.strip_prefix("https://").unwrap_or(&s);
    let s = s.strip_prefix("http://").unwrap_or(s);
    let s = s.strip_prefix("www.").unwrap_or(s);
    let s = match s.find('/') {
        Some(pos) => &s[..pos],
        None => s,
    };
    let s = s.strip_suffix('/').unwrap_or(s);
    s.to_string()
}

/// Extrai o domínio de uma URL, removendo protocolo, `www.`, porta e path.
/// Retorna `None` se a entrada for vazia ou não contiver domínio.
///
/// Exemplos:
/// - `"https://www.example.com:8080/path"` → `Some("example.com")`
/// - `"  "` → `None`
/// - `""` → `None`
pub fn extract_domain(url: &str) -> Option<String> {
    let input = url.trim();
    if input.is_empty() {
        return None;
    }
    let s = input
        .strip_prefix("https://")
        .or_else(|| input.strip_prefix("http://"))
        .unwrap_or(input);
    let s = match s.find('/') {
        Some(pos) => &s[..pos],
        None => s,
    };
    // remove porta (ex: "example.com:8080" → "example.com")
    let s = match s.find(':') {
        Some(pos) => &s[..pos],
        None => s,
    };
    let s = s.strip_prefix("www.").unwrap_or(s);
    let result = s.to_lowercase();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Verifica se `domain` está bloqueado pela `blocklist`.
///
/// A correspondência é feita após normalização e considera subdomínios:
/// bloquear `"youtube.com"` também bloqueia `"music.youtube.com"`, mas
/// **não** bloqueia `"notyoutube.com"`.
pub fn is_domain_blocked(domain: &str, blocklist: &[String]) -> bool {
    let normalized = normalize_domain(domain);
    blocklist
        .iter()
        .any(|item| normalized == *item || normalized.ends_with(&format!(".{}", item)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_protocol_www_and_path() {
        assert_eq!(
            normalize_domain("https://www.YouTube.com/watch?v=123"),
            "youtube.com"
        );
        assert_eq!(normalize_domain("http://facebook.com/"), "facebook.com");
        assert_eq!(normalize_domain("HTTP://Instagram.COM"), "instagram.com");
    }

    #[test]
    fn normalize_preserves_subdomain() {
        assert_eq!(normalize_domain("sub.example.com"), "sub.example.com");
        assert_eq!(normalize_domain("m.youtube.com"), "m.youtube.com");
    }

    #[test]
    fn normalize_idempotent_on_clean_domain() {
        let clean = "reddit.com";
        assert_eq!(normalize_domain(clean), clean);
    }

    #[test]
    fn extract_handles_port_and_path() {
        assert_eq!(
            extract_domain("https://www.example.com:8080/path?q=1"),
            Some("example.com".into()),
        );
    }

    #[test]
    fn extract_empty_returns_none() {
        assert_eq!(extract_domain(""), None);
        assert_eq!(extract_domain("   "), None);
    }

    #[test]
    fn is_blocked_matches_exact() {
        let list = vec!["youtube.com".to_string()];
        assert!(is_domain_blocked("youtube.com", &list));
        assert!(is_domain_blocked("https://www.youtube.com/", &list));
    }

    #[test]
    fn is_blocked_matches_subdomain() {
        let list = vec!["youtube.com".to_string()];
        assert!(is_domain_blocked("m.youtube.com", &list));
        assert!(is_domain_blocked("music.youtube.com", &list));
    }

    #[test]
    fn is_blocked_rejects_similar_domain() {
        // `notyoutube.com` termina com `.com`, não com `.youtube.com`.
        // Esse é o bug clássico de matching ingênuo — o teste existe pra garantir
        // que o ponto é exigido na comparação de sufixo.
        let list = vec!["youtube.com".to_string()];
        assert!(!is_domain_blocked("notyoutube.com", &list));
        assert!(!is_domain_blocked("myyoutube.com", &list));
    }

    #[test]
    fn is_blocked_empty_list_never_blocks() {
        let list: Vec<String> = vec![];
        assert!(!is_domain_blocked("anything.com", &list));
    }
}
