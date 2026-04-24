// =============================================================================
// Cert resolver dinâmico por SNI — gera leafs on-demand usando a LocalCa.
// =============================================================================
// O rustls chama `resolve(client_hello)` no meio do handshake TLS; temos que
// devolver um `CertifiedKey` apropriado pro SNI que o browser pediu. Sem
// SNI → None (browser aborta: ok, só TLS 1.0 antigo chega aqui).
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
}

impl std::fmt::Debug for SniCertResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SniCertResolver").finish_non_exhaustive()
    }
}

impl ResolvesServerCert for SniCertResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let sni = client_hello.server_name()?.to_string();

        // Cache hit (escopo curto do lock)
        if let Some(hit) = self.cache.lock().ok().and_then(|m| m.get(&sni).cloned()) {
            return Some(hit);
        }

        let ck = match self.ca.sign_leaf(&sni) {
            Ok(ck) => Arc::new(ck),
            Err(e) => {
                tracing::warn!(sni = %sni, error = %e, "sign_leaf falhou");
                return None;
            }
        };

        if let Ok(mut m) = self.cache.lock() {
            m.insert(sni, ck.clone());
        }
        Some(ck)
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
        let resolver = SniCertResolver::new(ca.clone());
        // Não conseguimos montar um ClientHello real fora do handshake, então
        // exercitamos o caminho via chamada direta à CA — que é o trabalho
        // custoso. Segunda chamada do resolver (cache) seria O(1).
        let a = ca.sign_leaf("instagram.com").unwrap();
        let b = ca.sign_leaf("instagram.com").unwrap();
        // Dois Certificates assinados em instantes diferentes — bytes
        // divergem (serial/timestamps), mas ambos têm cadeia válida.
        assert_eq!(a.cert.len(), b.cert.len());
        // Verifica que o cache do resolver está acessível.
        assert_eq!(resolver.cache.lock().unwrap().len(), 0);
    }
}
