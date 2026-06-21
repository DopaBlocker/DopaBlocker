package com.dopablocker.dopablocker_mobile.vpn

/// Normalização e matching de domínios — porta fiel de
/// shared/src/domain_matcher.rs para manter a MESMA semântica do desktop.
object DomainMatcher {

    /// Remove protocolo, `www.` e path, retornando só o domínio em minúsculas.
    /// Ex.: "https://www.YouTube.com/watch?v=1" → "youtube.com".
    fun normalizeDomain(input: String): String {
        var s = input.lowercase()
        s = s.removePrefix("https://")
        s = s.removePrefix("http://")
        s = s.removePrefix("www.")
        val slash = s.indexOf('/')
        if (slash >= 0) s = s.substring(0, slash)
        s = s.removeSuffix("/")
        return s
    }

    /// Verifica se `domain` está bloqueado. Match exato OU de subdomínio
    /// (`.item`) — o ponto é exigido para `notyoutube.com` não casar com
    /// `youtube.com`.
    fun isDomainBlocked(domain: String, blocklist: Collection<String>): Boolean {
        val normalized = normalizeDomain(domain)
        return blocklist.any { item -> normalized == item || normalized.endsWith(".$item") }
    }
}
