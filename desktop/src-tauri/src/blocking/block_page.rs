// =============================================================================
// Servidor da página de bloqueio — HTTP (:80) e HTTPS (:443).
// =============================================================================
// Quando o DNS proxy bloqueia um domínio, devolve A=127.0.0.1. O browser
// então tenta conectar aqui. Servimos a mesma página nas duas portas —
// HTTPS usa a LocalCa do módulo `ca` com resolver SNI dinâmico do módulo
// `tls_resolver`.
//
// Layout:
//   - `render_page(domain, reason)` — único ponto de renderização, injeta
//     {{DOMAIN}}, {{REASON_TEXT}}, {{QUOTE}} no template.
//   - `run_http(...)` — porta 80, parse mínimo da request, read-Host-header
//     pra decidir domínio/razão.
//   - `run_https(...)` — porta 443 com TLS via tokio_rustls. Mesmo parser
//     depois do handshake.
//
// Ambos aceitam shutdown via oneshot. Falha de bind loga e sai graciosa —
// a camada DNS continua bloqueando, só fica sem página bonita.
// =============================================================================

use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::Arc,
};

use anyhow::{anyhow, Context, Result};
use rand::seq::SliceRandom;
use rustls::ServerConfig;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{oneshot, RwLock},
};
use tokio_rustls::TlsAcceptor;

use super::{
    adult_filter::AdultFilter,
    block_reason::{self, BlockReason},
    ca::LocalCa,
    tls_resolver::SniCertResolver,
};

const HTTP_DEFAULT_PORT: u16 = 80;
const HTTPS_DEFAULT_PORT: u16 = 443;
const BLOCK_PAGE_HTML: &str = include_str!("block_page.html");
const GENERIC_REASON_TEXT: &str = "Este site está bloqueado pelo DopaBlocker";

const QUOTES: &[&str] = &[
    "Você não pode esgotar a criatividade. Quanto mais usa, mais tem. —Maya Angelou",
    "Você perde 100% dos chutes que não dá. —Wayne Gretzky",
    "O único modo de fazer um trabalho excelente é amar o que você faz. —Steve Jobs",
    "A disciplina é a ponte entre metas e realizações. —Jim Rohn",
    "Não espere. O tempo nunca será o momento certo. —Napoleon Hill",
    "O foco é dizer não. —Steve Jobs",
    "Faça o que você pode, com o que você tem, onde estiver. —Theodore Roosevelt",
    "A maneira de começar é parar de falar e começar a fazer. —Walt Disney",
    "Acorde com determinação. Durma com satisfação.",
    "O futuro depende do que você faz hoje. —Mahatma Gandhi",
    "Pequenos progressos diários geram grandes resultados.",
    "Quem tem um porquê forte aguenta qualquer como. —Nietzsche",
    "Não é sobre ter tempo. É sobre dar prioridade.",
    "A única saída é através. —Robert Frost",
    "Hábito é corda. Tecemos um fio dela por dia. —Horace Mann",
    "Se você quer mudar o mundo, comece arrumando sua cama. —Almirante McRaven",
    "O que obtemos realizando nossos objetivos não é tão importante quanto o que nos tornamos. —Zig Ziglar",
    "A persistência é o caminho do êxito. —Charles Chaplin",
    "Trinta minutos de silêncio te fazem mais produtivo que trinta minutos de distração.",
    "A atenção é a moeda mais valiosa do século. Gaste bem.",
];

// ---------- HTTP (porta 80) -------------------------------------------------

pub async fn run_http(
    rules: Arc<RwLock<HashSet<String>>>,
    adult: Arc<AdultFilter>,
    shutdown: oneshot::Receiver<()>,
) -> Result<()> {
    let port: u16 = std::env::var("DOPABLOCKER_BLOCK_PAGE_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(HTTP_DEFAULT_PORT);
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind TCP {addr} (porta 80 requer admin)"))?;
    tracing::info!(%addr, "Block page HTTP escutando");

    let mut shutdown = shutdown;
    loop {
        tokio::select! {
            biased;
            _ = &mut shutdown => {
                tracing::info!("Block page HTTP: shutdown");
                return Ok(());
            }
            accept = listener.accept() => {
                let (stream, peer) = match accept {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!(error = %e, "HTTP accept falhou");
                        continue;
                    }
                };
                let rules = rules.clone();
                let adult = adult.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_http_conn(stream, rules, adult).await {
                        tracing::debug!(error = %e, %peer, "HTTP conn encerrou");
                    }
                });
            }
        }
    }
}

async fn handle_http_conn(
    mut stream: TcpStream,
    rules: Arc<RwLock<HashSet<String>>>,
    adult: Arc<AdultFilter>,
) -> Result<()> {
    let host = read_and_parse_host(&mut stream).await;
    let reason = resolve_reason(host.as_deref(), &rules, &adult).await;
    let response = build_http_response(host.as_deref(), reason);
    stream.write_all(response.as_bytes()).await?;
    let _ = stream.shutdown().await;
    Ok(())
}

// ---------- HTTPS (porta 443) -----------------------------------------------

pub async fn run_https(
    rules: Arc<RwLock<HashSet<String>>>,
    adult: Arc<AdultFilter>,
    ca: Arc<LocalCa>,
    shutdown: oneshot::Receiver<()>,
) -> Result<()> {
    let port: u16 = std::env::var("DOPABLOCKER_BLOCK_PAGE_HTTPS_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(HTTPS_DEFAULT_PORT);
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let resolver = Arc::new(SniCertResolver::new(ca));
    let server_config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| anyhow!("rustls versions: {e}"))?
        .with_no_client_auth()
        .with_cert_resolver(resolver);
    let acceptor = TlsAcceptor::from(Arc::new(server_config));

    let listener = TcpListener::bind(addr).await.with_context(|| {
        format!(
            "bind TCP {addr} — outro processo em 443? (diagnóstico: `netstat -ano | findstr :443`)"
        )
    })?;
    tracing::info!(%addr, "Block page HTTPS escutando");

    let mut shutdown = shutdown;
    loop {
        tokio::select! {
            biased;
            _ = &mut shutdown => {
                tracing::info!("Block page HTTPS: shutdown");
                return Ok(());
            }
            accept = listener.accept() => {
                let (stream, peer) = match accept {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!(error = %e, "HTTPS accept falhou");
                        continue;
                    }
                };
                let acceptor = acceptor.clone();
                let rules = rules.clone();
                let adult = adult.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_https_conn(stream, acceptor, rules, adult).await {
                        tracing::debug!(error = %e, %peer, "HTTPS conn encerrou");
                    }
                });
            }
        }
    }
}

async fn handle_https_conn(
    stream: TcpStream,
    acceptor: TlsAcceptor,
    rules: Arc<RwLock<HashSet<String>>>,
    adult: Arc<AdultFilter>,
) -> Result<()> {
    let mut tls = acceptor.accept(stream).await.context("TLS handshake")?;
    let host = read_and_parse_host(&mut tls).await;
    let reason = resolve_reason(host.as_deref(), &rules, &adult).await;
    let response = build_http_response(host.as_deref(), reason);
    tls.write_all(response.as_bytes()).await?;
    let _ = tls.shutdown().await;
    Ok(())
}

// ---------- shared helpers --------------------------------------------------

/// Lê até 2KB da stream, extrai o valor do header `Host:`. Qualquer request
/// (método/path) devolve a mesma página, então só precisamos do host.
async fn read_and_parse_host<S: AsyncRead + Unpin>(stream: &mut S) -> Option<String> {
    let mut buf = [0u8; 2048];
    let n = stream.read(&mut buf).await.ok()?;
    if n == 0 {
        return None;
    }
    let text = std::str::from_utf8(&buf[..n]).ok()?;
    for line in text.lines() {
        let lower = line.trim_start();
        let low = lower.to_ascii_lowercase();
        if let Some(rest) = low.strip_prefix("host:") {
            let original_start = lower.len() - rest.len();
            let value = lower[original_start..].trim();
            // devolve só o host, descartando :porta se houver
            let host_only = value.split(':').next().unwrap_or(value);
            return Some(host_only.to_string());
        }
    }
    None
}

async fn resolve_reason(
    host: Option<&str>,
    rules: &Arc<RwLock<HashSet<String>>>,
    adult: &Arc<AdultFilter>,
) -> Option<BlockReason> {
    let h = host?.trim().to_lowercase();
    if h.is_empty() {
        return None;
    }
    block_reason::check(&h, rules, adult).await
}

fn render_page(domain: Option<&str>, reason: Option<BlockReason>) -> String {
    let domain_text = domain.unwrap_or("");
    let reason_text = reason.map(BlockReason::as_text).unwrap_or(GENERIC_REASON_TEXT);
    let quote = {
        let mut rng = rand::thread_rng();
        QUOTES.choose(&mut rng).copied().unwrap_or(QUOTES[0])
    };
    BLOCK_PAGE_HTML
        .replace("{{DOMAIN}}", &html_escape(domain_text))
        .replace("{{REASON_TEXT}}", &html_escape(reason_text))
        .replace("{{QUOTE}}", &html_escape(quote))
}

fn build_http_response(domain: Option<&str>, reason: Option<BlockReason>) -> String {
    let body = render_page(domain, reason);
    format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         Cache-Control: no-store\r\n\
         \r\n\
         {body}",
        body.len(),
    )
}

/// Escape mínimo pro template (domínio vem do browser → pode conter
/// caracteres perigosos; razão e quote são fixas, mas custa zero escapar).
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

// Compat wrapper: o `run` antigo ficou como `run_http`. Mantemos o símbolo
// pra se algum código externo ainda chamar — mas não existe uso fora do
// engine, então é só defensivo.
#[allow(dead_code)]
pub async fn run(
    rules: Arc<RwLock<HashSet<String>>>,
    adult: Arc<AdultFilter>,
    shutdown: oneshot::Receiver<()>,
) -> Result<()> {
    run_http(rules, adult, shutdown).await
}

// Make write trait used on TlsStream
trait _Placeholder: AsyncWrite {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_uses_reason_text() {
        let html = render_page(Some("instagram.com"), Some(BlockReason::UserList));
        assert!(html.contains("instagram.com"));
        assert!(html.contains("Na sua lista de bloqueios"));
    }

    #[test]
    fn render_falls_back_to_generic_reason() {
        let html = render_page(Some("exemplo.com"), None);
        assert!(html.contains("exemplo.com"));
        assert!(html.contains("Este site está bloqueado pelo DopaBlocker"));
    }

    #[test]
    fn escape_prevents_html_injection_in_domain() {
        let html = render_page(Some("<script>.com"), Some(BlockReason::AdultFilter));
        assert!(!html.contains("<script>.com"));
        assert!(html.contains("&lt;script&gt;.com"));
    }
}
