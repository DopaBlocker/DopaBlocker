// =============================================================================
// DNS proxy — escuta 127.0.0.1 (UDP + TCP) e decide: block / cache / forward.
// =============================================================================
// Fluxo de uma query:
//   1. Parse mínimo com hickory-proto para extrair nome/qtype.
//   2. Walk de labels contra a blocklist. Hit → NXDOMAIN sintético.
//   3. Lookup no DnsCache pela tripla (nome, qtype, qclass). Hit → devolve
//      bytes cacheados com o ID reescrito.
//   4. Miss → UpstreamPool.resolve (DoH → UDP failover). Sucesso:
//      armazena no cache e devolve ao cliente. Falha: SERVFAIL.
//
// UDP e TCP rodam em paralelo no mesmo handler `handle_query_bytes`. TCP
// respeita o framing de 2-byte length prefix do RFC 1035 §4.2.2.
//
// Porta 53 exige admin no Windows. Em dev: `DOPABLOCKER_DNS_PORT=5353`.
// =============================================================================

use std::{collections::HashSet, net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use hickory_proto::{
    op::{Message, MessageType, ResponseCode},
    serialize::binary::BinEncodable,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
    sync::{oneshot, RwLock},
};

use super::{dns_cache::DnsCache, dns_upstream::UpstreamPool};

const DEFAULT_PORT: u16 = 53;
const MAX_DNS_PACKET: usize = 4096;
const MAX_TCP_MSG: usize = 65_535; // TCP DNS carrega u16 length prefix.

/// Sobe UDP + TCP em 127.0.0.1:PORT até `shutdown` disparar. Retorna erro
/// se qualquer um dos binds falhar (tipicamente: sem admin para porta 53).
pub async fn run(rules: Arc<RwLock<HashSet<String>>>, shutdown: oneshot::Receiver<()>) -> Result<()> {
    let port: u16 = std::env::var("DOPABLOCKER_DNS_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();

    let udp = Arc::new(
        UdpSocket::bind(addr)
            .await
            .with_context(|| format!("bind UDP {addr} (porta 53 requer admin)"))?,
    );
    let tcp = TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind TCP {addr} (porta 53 requer admin)"))?;
    tracing::info!(%addr, "DNS proxy escutando (UDP + TCP)");

    let cache = DnsCache::new();
    let upstream = UpstreamPool::default_cloudflare_google();

    let mut shutdown = shutdown;
    let udp_for_loop = udp.clone();
    let rules_udp = rules.clone();
    let cache_udp = cache.clone();
    let upstream_udp = upstream.clone();

    tokio::select! {
        biased;
        _ = &mut shutdown => {
            tracing::info!("DNS proxy: shutdown");
            Ok(())
        }
        r = udp_loop(udp_for_loop, rules_udp, cache_udp, upstream_udp) => r,
        r = tcp_loop(tcp, rules, cache, upstream) => r,
    }
}

// ----- UDP -------------------------------------------------------------------

async fn udp_loop(
    socket: Arc<UdpSocket>,
    rules: Arc<RwLock<HashSet<String>>>,
    cache: DnsCache,
    upstream: UpstreamPool,
) -> Result<()> {
    let mut buf = vec![0u8; MAX_DNS_PACKET];
    loop {
        let (n, client) = match socket.recv_from(&mut buf).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "UDP recv_from falhou");
                continue;
            }
        };
        let data = buf[..n].to_vec();
        let socket = socket.clone();
        let rules = rules.clone();
        let cache = cache.clone();
        let upstream = upstream.clone();
        tokio::spawn(async move {
            let response = handle_query_bytes(&data, &rules, &cache, &upstream).await;
            if let Some(bytes) = response {
                if let Err(e) = socket.send_to(&bytes, client).await {
                    tracing::debug!(error = %e, %client, "UDP send_to falhou");
                }
            }
        });
    }
}

// ----- TCP -------------------------------------------------------------------

async fn tcp_loop(
    listener: TcpListener,
    rules: Arc<RwLock<HashSet<String>>>,
    cache: DnsCache,
    upstream: UpstreamPool,
) -> Result<()> {
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "TCP accept falhou");
                continue;
            }
        };
        let rules = rules.clone();
        let cache = cache.clone();
        let upstream = upstream.clone();
        tokio::spawn(async move {
            if let Err(e) = tcp_connection(stream, &rules, &cache, &upstream).await {
                tracing::debug!(error = %e, %peer, "TCP conn encerrou");
            }
        });
    }
}

/// Uma conexão TCP pode transportar múltiplas queries sequenciais. Cada uma
/// tem um prefixo de 2 bytes com o tamanho (big-endian), per RFC 1035.
async fn tcp_connection(
    mut stream: TcpStream,
    rules: &Arc<RwLock<HashSet<String>>>,
    cache: &DnsCache,
    upstream: &UpstreamPool,
) -> Result<()> {
    loop {
        let mut len_buf = [0u8; 2];
        match stream.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let len = u16::from_be_bytes(len_buf) as usize;
        if len == 0 || len > MAX_TCP_MSG {
            return Ok(());
        }
        let mut msg = vec![0u8; len];
        stream.read_exact(&mut msg).await?;

        let Some(response) = handle_query_bytes(&msg, rules, cache, upstream).await else {
            continue;
        };
        let resp_len = (response.len() as u16).to_be_bytes();
        stream.write_all(&resp_len).await?;
        stream.write_all(&response).await?;
    }
}

// ----- lógica comum ----------------------------------------------------------

/// Roteia um pacote DNS: block → NXDOMAIN, hit no cache → resposta cacheada,
/// miss → forward via pool. Retorna `None` só quando o pacote é inutilizável
/// (ex: vazio, sem queries) — aí o chamador simplesmente não responde.
async fn handle_query_bytes(
    data: &[u8],
    rules: &RwLock<HashSet<String>>,
    cache: &DnsCache,
    upstream: &UpstreamPool,
) -> Option<Vec<u8>> {
    let msg = match Message::from_vec(data) {
        Ok(m) => m,
        Err(e) => {
            tracing::debug!(error = %e, "DNS mal-formado, ignorando");
            return None;
        }
    };
    let query = msg.queries().first()?;
    let name = query.name().to_string();
    let normalized = name.trim_end_matches('.').to_lowercase();
    let query_id = msg.id();

    if is_blocked(&normalized, rules).await {
        tracing::info!(query = %normalized, "BLOCK → NXDOMAIN");
        return build_nxdomain(&msg).ok();
    }

    if let Some(cached) = cache.get(&msg, query_id).await {
        tracing::debug!(query = %normalized, "cache HIT");
        return Some(cached);
    }

    match upstream.resolve(data).await {
        Ok(bytes) => {
            cache.put(&msg, &bytes).await;
            tracing::debug!(query = %normalized, "cache MISS → upstream OK");
            Some(bytes)
        }
        Err(e) => {
            tracing::warn!(query = %normalized, error = %e, "upstream falhou — SERVFAIL");
            build_servfail(&msg).ok()
        }
    }
}

/// Walk label-por-label: `m.music.youtube.com` bate em `youtube.com`, mas
/// `notyoutube.com` não (nunca sobe pra `youtube.com`).
async fn is_blocked(domain: &str, rules: &RwLock<HashSet<String>>) -> bool {
    let rules = rules.read().await;
    let mut current = domain;
    loop {
        if rules.contains(current) {
            return true;
        }
        match current.find('.') {
            Some(idx) => current = &current[idx + 1..],
            None => return false,
        }
    }
}

fn build_nxdomain(query_msg: &Message) -> Result<Vec<u8>> {
    build_error_response(query_msg, ResponseCode::NXDomain)
}

fn build_servfail(query_msg: &Message) -> Result<Vec<u8>> {
    build_error_response(query_msg, ResponseCode::ServFail)
}

fn build_error_response(query_msg: &Message, code: ResponseCode) -> Result<Vec<u8>> {
    let mut response = Message::new();
    response.set_id(query_msg.id());
    response.set_message_type(MessageType::Response);
    response.set_op_code(query_msg.op_code());
    response.set_response_code(code);
    response.set_recursion_desired(query_msg.recursion_desired());
    response.set_recursion_available(true);
    response.set_authoritative(false);

    for q in query_msg.queries() {
        response.add_query(q.clone());
    }

    response.to_bytes().context("encode resposta de erro")
}
