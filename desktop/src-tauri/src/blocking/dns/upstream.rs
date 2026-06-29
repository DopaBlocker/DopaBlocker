// =============================================================================
// Pool de resolvers upstream com failover.
// =============================================================================
// Precisamos evitar duas armadilhas:
//   1. O proxy nao pode depender do proprio DNS local para resolver os hosts
//      DoH, senao criamos recursao.
//   2. O fallback UDP continua existindo para redes que derrubam DoH.
// =============================================================================

use std::{net::SocketAddr, time::Duration};

use anyhow::{bail, Context, Result};
use tokio::{net::UdpSocket, time::timeout};

const DOH_TIMEOUT: Duration = Duration::from_secs(3);
const UDP_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_DNS_PACKET: usize = 4096;

#[derive(Clone)]
enum Upstream {
    Doh(String),
    Udp(SocketAddr),
}

#[derive(Clone)]
pub struct UpstreamPool {
    upstreams: Vec<Upstream>,
    http: reqwest::Client,
}

impl UpstreamPool {
    pub fn default_cloudflare_google() -> Self {
        let http = build_doh_http_client();

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

fn build_doh_http_client() -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .timeout(DOH_TIMEOUT)
        .pool_idle_timeout(Duration::from_secs(90));

    for host in ["cloudflare-dns.com", "dns.google"] {
        let addrs = bootstrap_addrs_for_host(host);
        if !addrs.is_empty() {
            builder = builder.resolve_to_addrs(host, &addrs);
        }
    }

    builder
        .build()
        .expect("reqwest::Client deveria construir com defaults")
}

fn bootstrap_addrs_for_host(host: &str) -> Vec<SocketAddr> {
    match host {
        "cloudflare-dns.com" => vec![
            SocketAddr::from(([1, 1, 1, 1], 0)),
            SocketAddr::from(([1, 0, 0, 1], 0)),
        ],
        "dns.google" => vec![
            SocketAddr::from(([8, 8, 8, 8], 0)),
            SocketAddr::from(([8, 8, 4, 4], 0)),
        ],
        _ => Vec::new(),
    }
}

async fn resolve_udp(upstream: SocketAddr, query: &[u8]) -> Result<Vec<u8>> {
    let sock = UdpSocket::bind("0.0.0.0:0")
        .await
        .context("UDP: bind efemero")?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_addrs_cover_builtin_doh_hosts() {
        assert_eq!(
            bootstrap_addrs_for_host("cloudflare-dns.com"),
            vec![
                SocketAddr::from(([1, 1, 1, 1], 0)),
                SocketAddr::from(([1, 0, 0, 1], 0)),
            ]
        );
        assert_eq!(
            bootstrap_addrs_for_host("dns.google"),
            vec![
                SocketAddr::from(([8, 8, 8, 8], 0)),
                SocketAddr::from(([8, 8, 4, 4], 0)),
            ]
        );
        assert!(bootstrap_addrs_for_host("example.com").is_empty());
    }
}
