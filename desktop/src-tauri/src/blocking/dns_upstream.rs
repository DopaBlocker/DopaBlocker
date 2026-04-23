// =============================================================================
// Pool de resolvers upstream com failover.
// =============================================================================
// Ordem tentada (primeira que responder vence):
//   1. DoH Cloudflare  — HTTPS encryptado, sem bisbilhoteiro no meio.
//   2. DoH Google      — mesmo, backup independente.
//   3. UDP 1.1.1.1     — fallback simples caso HTTPS esteja bloqueado/caído.
//   4. UDP 8.8.8.8     — fallback final.
//
// Por que DoH primeiro, e UDP como último recurso? O app é de foco/privacidade
// — não faz sentido o provedor de internet ver toda query permitida. Mas em
// redes restritivas (hotel, corporate) o 443 pode estar bloqueado pra DNS;
// nesses casos o UDP salva a pátria.
//
// Cada tentativa tem timeout individual. Se todas falharem, devolve erro;
// o caller escolhe o que fazer (tipicamente: SERVFAIL pro cliente).
// =============================================================================

use std::{net::SocketAddr, time::Duration};

use anyhow::{bail, Context, Result};
use tokio::{net::UdpSocket, time::timeout};

const DOH_TIMEOUT: Duration = Duration::from_secs(3);
const UDP_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_DNS_PACKET: usize = 4096;

#[derive(Clone)]
enum Upstream {
    /// URL `https://.../dns-query` falando RFC 8484 (application/dns-message).
    Doh(String),
    /// Resolver plaintext na porta 53. Quase sempre `1.1.1.1:53` / `8.8.8.8:53`.
    Udp(SocketAddr),
}

#[derive(Clone)]
pub struct UpstreamPool {
    upstreams: Vec<Upstream>,
    http: reqwest::Client,
}

impl UpstreamPool {
    pub fn default_cloudflare_google() -> Self {
        let http = reqwest::Client::builder()
            .timeout(DOH_TIMEOUT)
            // HTTP/2 com keep-alive para reusar conexão entre queries. DoH
            // sem isso faz TLS handshake a cada query e fica inviável.
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .expect("reqwest::Client deveria construir com defaults");

        Self {
            upstreams: vec![
                Upstream::Doh("https://cloudflare-dns.com/dns-query".into()),
                Upstream::Doh("https://dns.google/dns-query".into()),
                Upstream::Udp("1.1.1.1:53".parse().unwrap()),
                Upstream::Udp("8.8.8.8:53".parse().unwrap()),
            ],
            http,
        }
    }

    /// Tenta cada upstream em ordem. Retorna a primeira resposta com status
    /// HTTP 2xx (DoH) ou que chegou antes do timeout (UDP). Se todos falharem,
    /// retorna erro com o motivo do último.
    pub async fn resolve(&self, query: &[u8]) -> Result<Vec<u8>> {
        let mut last_err: Option<anyhow::Error> = None;
        for up in &self.upstreams {
            let result = match up {
                Upstream::Doh(url) => self.resolve_doh(url, query).await,
                Upstream::Udp(addr) => resolve_udp(*addr, query).await,
            };
            match result {
                Ok(bytes) => return Ok(bytes),
                Err(e) => {
                    tracing::warn!(upstream = ?up, error = %e, "upstream falhou");
                    last_err = Some(e);
                }
            }
        }
        match last_err {
            Some(e) => Err(e),
            None => bail!("nenhum upstream configurado"),
        }
    }

    async fn resolve_doh(&self, url: &str, query: &[u8]) -> Result<Vec<u8>> {
        let resp = self
            .http
            .post(url)
            .header("content-type", "application/dns-message")
            .header("accept", "application/dns-message")
            .body(query.to_vec())
            .send()
            .await
            .context("DoH: erro de rede")?
            .error_for_status()
            .context("DoH: status HTTP ruim")?;
        let bytes = resp.bytes().await.context("DoH: falha lendo body")?;
        Ok(bytes.to_vec())
    }
}

async fn resolve_udp(upstream: SocketAddr, query: &[u8]) -> Result<Vec<u8>> {
    let sock = UdpSocket::bind("0.0.0.0:0")
        .await
        .context("UDP: bind efêmero")?;
    sock.send_to(query, upstream).await.context("UDP: send")?;
    let mut buf = vec![0u8; MAX_DNS_PACKET];
    let (n, _) = timeout(UDP_TIMEOUT, sock.recv_from(&mut buf))
        .await
        .context("UDP: timeout")??;
    buf.truncate(n);
    Ok(buf)
}

impl std::fmt::Debug for Upstream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Upstream::Doh(url) => write!(f, "DoH({url})"),
            Upstream::Udp(addr) => write!(f, "UDP({addr})"),
        }
    }
}
