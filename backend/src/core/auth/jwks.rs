// -----------------------------------------------------------------------------
// JWKS cache — chaves públicas do Firebase
// -----------------------------------------------------------------------------
// O Firebase assina JWTs com chaves RSA que rotacionam periodicamente.
// Para validar a assinatura, precisamos da chave pública correspondente
// ao `kid` que o JWT carrega no header.
//
// Este endpoint retorna um JSON `{ "kid1": "PEM...", "kid2": "PEM..." }`:

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::DecodingKey;
use tokio::sync::RwLock;

use crate::core::errors::AppError;

const JWKS_URL: &str =
    "https://www.googleapis.com/robot/v1/metadata/x509/securetoken@system.gserviceaccount.com";
const JWKS_FETCH_TIMEOUT: Duration = Duration::from_secs(10);

// Tempo de vida do cache. O Google rotaciona chaves a cada ~24h, mas o
// header `Cache-Control` da resposta é mais curto. 6h é um meio-termo
// seguro: muitas validações cached, sem risco de usar chave muito antiga.
const JWKS_TTL_SECS: u64 = 6 * 60 * 60;

/// Cache de chaves públicas do Firebase, thread-safe via `RwLock`.
/// Criado uma vez no boot e compartilhado via `Arc` no `AppState`.
#[derive(Default)]
pub struct JwksCache {
    inner: RwLock<Option<CachedJwks>>,
}

struct CachedJwks {
    fetched_at: Instant,
    keys: HashMap<String, DecodingKey>,
}

impl JwksCache {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Retorna a chave com o `kid` dado. Se o cache estiver fresco e
    /// contiver o kid, retorna imediatamente. Senão, baixa a lista nova
    /// do Google, atualiza o cache, e tenta de novo.
    ///
    /// Não faz retry se o kid ainda não estiver presente após o refetch
    /// — isso significaria que o JWT usa uma chave desconhecida (JWT forjado
    /// ou deprecado) e deve falhar como Unauthorized.
    pub(crate) async fn get(&self, kid: &str) -> Result<DecodingKey, AppError> {
        // Tentativa 1: ler do cache (lock compartilhado, múltiplos leitores ok).
        {
            let guard = self.inner.read().await;
            if let Some(cached) = guard.as_ref() {
                if cached.fetched_at.elapsed() < Duration::from_secs(JWKS_TTL_SECS) {
                    if let Some(k) = cached.keys.get(kid) {
                        return Ok(k.clone());
                    }
                }
            }
        }

        // Cache frio, expirado ou sem o kid → vai buscar.
        let fresh = fetch_jwks_timed().await?;
        let key = fresh
            .get(kid)
            .cloned()
            .ok_or_else(|| AppError::Unauthorized("kid do JWT não encontrado no JWKS".into()))?;

        // Substitui o cache (lock exclusivo).
        let mut guard = self.inner.write().await;
        *guard = Some(CachedJwks {
            fetched_at: Instant::now(),
            keys: fresh,
        });
        Ok(key)
    }
}

/// Baixa e parseia o JWKS do Google com timeout. Cada PEM é convertido em
/// `DecodingKey` reutilizável para a validação de assinatura.
async fn fetch_jwks_timed() -> Result<HashMap<String, DecodingKey>, AppError> {
    let client = reqwest::Client::builder()
        .timeout(JWKS_FETCH_TIMEOUT)
        .build()
        .map_err(|e| AppError::InternalServerError(format!("Falha ao criar client JWKS: {e}")))?;

    let resp: HashMap<String, String> = client
        .get(JWKS_URL)
        .send()
        .await
        .map_err(|e| AppError::InternalServerError(format!("Falha ao buscar JWKS: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::InternalServerError(format!("JWKS inválido: {e}")))?;

    let mut out = HashMap::new();
    for (kid, pem) in resp {
        let key = DecodingKey::from_rsa_pem(pem.as_bytes())
            .map_err(|e| AppError::InternalServerError(format!("PEM inválido: {e}")))?;
        out.insert(kid, key);
    }
    Ok(out)
}
