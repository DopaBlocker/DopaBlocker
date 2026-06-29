// =============================================================================
// Filtro de conteúdo adulto — Bloom Filter populado a partir da lista
// Steven Black (alternates/porn/hosts).
// =============================================================================
// Arquitetura:
//   - O filtro é **uma única instância** (`Arc<AdultFilter>`), criada no boot
//     do app. O build é async em background pra não bloquear a abertura da
//     janela — enquanto não terminou, `contains()` devolve false e o DNS
//     proxy se comporta como se o filtro estivesse desligado.
//   - `enabled` é `AtomicBool` porque o hot-path do DNS (uma query por
//     pacote) consulta isso sem precisar de lock.
//   - `bloom` é `OnceLock`: setado uma vez no fim do build. Leitura
//     subsequente é ponteiro-deref puro, sem RwLock.
//
// Cache em disco:
//   - `app_cache_dir/adult_list.txt` — raw do GitHub, serve de backup
//     offline (se o próximo download falhar, reusamos o último).
//   - `app_cache_dir/adult_filter.bin` — bincode do `BloomFilter` já
//     populado. Carregar isso é muito mais barato que reprocessar 50k+
//     linhas (centenas de ms vs dezenas de segundos).
//   - Revalida a cada 7 dias (mtime do .bin). Se passou, baixa de novo.
//
// Parsing:
//   - Linhas no formato `0.0.0.0 dominio.com` (hosts file).
//   - Comentários com `#` e linhas em branco são puladas.
//   - Domínios "técnicos" (localhost, 0.0.0.0) filtrados.
// =============================================================================

use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        OnceLock,
    },
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result};

use dopablocker_shared::bloom_filter::BloomFilter;

const LIST_URL: &str =
    "https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/porn/hosts";
const MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);
const EXPECTED_DOMAINS: usize = 100_000;
const FP_RATE: f64 = 0.001;

pub struct AdultFilter {
    bloom: OnceLock<BloomFilter>,
    enabled: AtomicBool,
    cache_dir: PathBuf,
}

impl AdultFilter {
    /// Cria a instância sem construir o filtro — chame `build_if_needed`
    /// em background logo depois. Aceita o estado persistido de enabled.
    pub fn new(cache_dir: PathBuf, initially_enabled: bool) -> Self {
        Self {
            bloom: OnceLock::new(),
            enabled: AtomicBool::new(initially_enabled),
            cache_dir,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Verdadeiro depois que o download+populate terminou. Usado pra UI
    /// mostrar "construindo filtro..." enquanto o usuário liga o toggle
    /// mas o Bloom ainda nem foi carregado.
    pub fn is_built(&self) -> bool {
        self.bloom.get().is_some()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Retorna se o `domain` está no filtro. Hot-path do DNS — barato quando
    /// desligado ou ainda não construído (zero locks, só leitura atômica).
    pub fn contains(&self, domain: &str) -> bool {
        if !self.is_enabled() {
            return false;
        }
        match self.bloom.get() {
            Some(b) => b.contains(domain),
            None => false, // ainda construindo
        }
    }

    /// Tenta construir o filtro. Idempotente: se `bloom` já está setado,
    /// sai em O(1). Prioriza carregar o `.bin` em cache — só baixa a lista
    /// se o cache expirou ou não existe.
    pub async fn build_if_needed(&self) -> Result<()> {
        if self.bloom.get().is_some() {
            return Ok(());
        }
        std::fs::create_dir_all(&self.cache_dir).ok();
        let bin_path = self.cache_dir.join("adult_filter.bin");
        let txt_path = self.cache_dir.join("adult_list.txt");

        // Fast path: bin serializado + fresh.
        if is_fresh(&bin_path, MAX_AGE) {
            if let Some(bloom) = try_load_bin(&bin_path) {
                let _ = self.bloom.set(bloom);
                tracing::info!("adult filter: carregado do cache binário");
                return Ok(());
            }
        }

        // Precisa (re)construir. Download, com fallback pro .txt antigo.
        let text = match download_list().await {
            Ok(text) => {
                let _ = std::fs::write(&txt_path, &text);
                text
            }
            Err(e) if txt_path.exists() => {
                tracing::warn!(error = %e, "download falhou, usando lista em cache");
                std::fs::read_to_string(&txt_path).context("ler cache local .txt")?
            }
            Err(e) => return Err(e),
        };

        let domains = parse_hosts(&text);
        tracing::info!(count = domains.len(), "adult filter: populando Bloom");
        let mut bloom = BloomFilter::new(domains.len().max(EXPECTED_DOMAINS), FP_RATE);
        for d in &domains {
            bloom.insert(d);
        }

        // Persiste pro próximo boot. Erro de I/O aqui não é fatal —
        // continuamos com o filtro em memória nesta sessão.
        match bincode::serialize(&bloom) {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(&bin_path, bytes) {
                    tracing::warn!(error = %e, "falha ao salvar adult_filter.bin");
                }
            }
            Err(e) => tracing::warn!(error = %e, "falha ao serializar Bloom"),
        }

        let _ = self.bloom.set(bloom);
        Ok(())
    }
}

// -------- helpers ----------------------------------------------------------

fn is_fresh(path: &Path, max_age: Duration) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    SystemTime::now()
        .duration_since(modified)
        .map(|age| age < max_age)
        .unwrap_or(false)
}

fn try_load_bin(path: &Path) -> Option<BloomFilter> {
    let bytes = std::fs::read(path).ok()?;
    bincode::deserialize::<BloomFilter>(&bytes).ok()
}

async fn download_list() -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(DOWNLOAD_TIMEOUT)
        .build()
        .context("construir http client")?;
    let resp = client
        .get(LIST_URL)
        .send()
        .await
        .context("GET Steven Black hosts")?
        .error_for_status()
        .context("status HTTP ruim")?;
    resp.text().await.context("ler body")
}

/// Parse tolerante do formato hosts:
///   `0.0.0.0 exemplo.com # comentário`
/// Strips comentários, ignora linhas vazias e domínios sem `.`.
fn parse_hosts(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let no_comment = line.split('#').next().unwrap_or("").trim();
            if no_comment.is_empty() {
                return None;
            }
            let mut parts = no_comment.split_whitespace();
            let _ip = parts.next()?;
            let domain = parts.next()?.to_lowercase();
            // Pula entradas "de serviço" do próprio hosts file.
            if domain == "localhost"
                || domain == "localhost.localdomain"
                || domain == "broadcasthost"
                || domain == "0.0.0.0"
                || !domain.contains('.')
            {
                return None;
            }
            Some(domain)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stevenblack_like_input() {
        let sample = r#"
# Comentário
0.0.0.0 localhost
0.0.0.0 broadcasthost
0.0.0.0  pornhub.com
0.0.0.0 xvideos.com   # comentário trailing
::1 localhost

0.0.0.0 invalid_no_dot
0.0.0.0 redtube.com
"#;
        let domains = parse_hosts(sample);
        assert_eq!(domains, vec!["pornhub.com", "xvideos.com", "redtube.com"]);
    }

    #[test]
    fn contains_respects_enabled_flag() {
        let tmp = std::env::temp_dir().join("dopablocker_test_af");
        let af = AdultFilter::new(tmp, false);
        // Sem enabled, nunca bate — mesmo que bloom esteja vazio.
        assert!(!af.contains("qualquer.com"));
        af.set_enabled(true);
        // Ainda assim false porque bloom está None (não construído).
        assert!(!af.contains("qualquer.com"));
    }
}
