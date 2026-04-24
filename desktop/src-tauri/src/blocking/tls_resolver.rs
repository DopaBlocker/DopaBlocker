// =============================================================================
// Cert resolver dinâmico por SNI — gera leafs on-demand usando a LocalCa.
// =============================================================================
// O rustls chama `resolve(client_hello)` no meio do handshake TLS; temos que
// devolver um `CertifiedKey` apropriado pro SNI que o browser pediu. Sem
// SNI -> None (browser aborta: ok, só TLS 1.0 antigo chega aqui).
//
// Cache: HashMap por SNI. Tamanho implícito (dezenas/centenas de hosts por
// sessão). Se virar problema, trocar por LRU; por ora, simples.
// =============================================================================

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;

use super::ca::LocalCa;

pub struct SniCertResolver {
    ca: Arc<LocalCa>,
    cache: Mutex<HashMap<String, Arc<CertifiedKey>>>,
}

impl SniCertResolver {
    pub fn new(ca: Arc<LocalCa>) -> Self {
        Self {
            ca,
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn resolve_sni(&self, sni: &str) -> Option<Arc<CertifiedKey>> {
        if let Some(hit) = self.cache.lock().ok().and_then(|m| m.get(sni).cloned()) {
            return Some(hit);
        }

        let ck = match self.ca.sign_leaf(sni) {
            Ok(ck) => Arc::new(ck),
            Err(e) => {
                tracing::warn!(sni, error = %e, "sign_leaf falhou");
                return None;
            }
        };

        if let Ok(mut m) = self.cache.lock() {
            m.insert(sni.to_string(), ck.clone());
        }
        Some(ck)
    }

    #[cfg(test)]
    fn cache_len(&self) -> usize {
        self.cache.lock().map(|m| m.len()).unwrap_or(0)
    }
}

impl std::fmt::Debug for SniCertResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SniCertResolver").finish_non_exhaustive()
    }
}

impl ResolvesServerCert for SniCertResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let sni = client_hello.server_name()?.to_string();
        self.resolve_sni(&sni)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "dopablocker_tls_resolver_test_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn cache_returns_same_arc_for_same_sni() {
        let ca = Arc::new(LocalCa::load_or_create(&tmp_dir()).unwrap());
        let resolver = SniCertResolver::new(ca);
        let a = resolver.resolve_sni("instagram.com").unwrap();
        let b = resolver.resolve_sni("instagram.com").unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(resolver.cache_len(), 1);
    }

    #[test]
    fn cache_miss_creates_new_entry_for_new_sni() {
        let ca = Arc::new(LocalCa::load_or_create(&tmp_dir()).unwrap());
        let resolver = SniCertResolver::new(ca);
        let a = resolver.resolve_sni("instagram.com").unwrap();
        let b = resolver.resolve_sni("youtube.com").unwrap();
        assert!(!Arc::ptr_eq(&a, &b));
        assert_eq!(resolver.cache_len(), 2);
    }
}
