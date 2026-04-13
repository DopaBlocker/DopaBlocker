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
    blocklist.iter().any(|item| {
        normalized == *item || normalized.ends_with(&format!(".{}", item))
    })
}