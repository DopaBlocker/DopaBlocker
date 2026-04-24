// =============================================================================
// DNS proxy - escuta no loopback e decide: block / cache / forward.
// =============================================================================
// O app agora precisa atender consultas em 127.0.0.1 e ::1. Antes o Windows
// podia continuar consultando DNS IPv6 fora do proxy, mesmo com o bloqueio
// "ligado". Este modulo passa a bindar ambas as familias.
// =============================================================================

use std::{
    collections::HashSet,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
};

use anyhow::{Context, Result};
use hickory_proto::{
    op::{Message, MessageType, ResponseCode},
    rr::{rdata::A, RData, Record, RecordType},
    serialize::binary::BinEncodable,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
    sync::{oneshot, RwLock},
    task::JoinSet,
};

use super::{
    adult_filter::AdultFilter, block_reason, dns_cache::DnsCache, dns_upstream::UpstreamPool,
};

const DEFAULT_PORT: u16 = 53;
const MAX_DNS_PACKET: usize = 4096;
const MAX_TCP_MSG: usize = 65_535;

pub async fn run(
    rules: Arc<RwLock<HashSet<String>>>,
    adult: Arc<AdultFilter>,
    shutdown: oneshot::Receiver<()>,
) -> Result<()> {
    let port: u16 = std::env::var("DOPABLOCKER_DNS_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let addrs = listener_addrs(port);

    let udp_sockets = bind_udp_sockets(&addrs).await?;
    let tcp_listeners = bind_tcp_listeners(&addrs).await?;

    let cache = DnsCache::new();
    let upstream = UpstreamPool::default_cloudflare_google();
    let mut shutdown = shutdown;
    let mut tasks = JoinSet::new();

    for socket in udp_sockets {
        tasks.spawn(udp_loop(
            socket,
            rules.clone(),
            adult.clone(),
            cache.clone(),
            upstream.clone(),
        ));
    }
    for listener in tcp_listeners {
        tasks.spawn(tcp_loop(
            listener,
            rules.clone(),
            adult.clone(),
            cache.clone(),
            upstream.clone(),
        ));
    }

    tokio::select! {
        biased;
        _ = &mut shutdown => {
            tracing::info!("DNS proxy: shutdown");
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
            Ok(())
        }
        res = tasks.join_next() => {
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
            match res {
                Some(Ok(inner)) => inner,
                Some(Err(join_err)) => Err(anyhow::anyhow!("task do DNS proxy abortou: {join_err}")),
                None => Err(anyhow::anyhow!("DNS proxy sem listeners ativos")),
            }
        }
    }
}

fn listener_addrs(port: u16) -> [SocketAddr; 2] {
    [
        SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port),
        SocketAddr::new(Ipv6Addr::LOCALHOST.into(), port),
    ]
}

async fn bind_udp_sockets(addrs: &[SocketAddr]) -> Result<Vec<Arc<UdpSocket>>> {
    let mut sockets = Vec::new();
    let mut last_err = None;

    for addr in addrs {
        match UdpSocket::bind(*addr)
            .await
            .with_context(|| format!("bind UDP {addr} (porta 53 requer admin)"))
        {
            Ok(socket) => {
                tracing::info!(%addr, "DNS proxy escutando em UDP");
                sockets.push(Arc::new(socket));
            }
            Err(e) => {
                tracing::warn!(%addr, error = %e, "falha ao bindar UDP do DNS proxy");
                last_err = Some(e);
            }
        }
    }

    if sockets.is_empty() {
        return Err(last_err.unwrap_or_else(|| anyhow::anyhow!("nenhum socket UDP disponivel")));
    }
    Ok(sockets)
}

async fn bind_tcp_listeners(addrs: &[SocketAddr]) -> Result<Vec<TcpListener>> {
    let mut listeners = Vec::new();
    let mut last_err = None;

    for addr in addrs {
        match TcpListener::bind(*addr)
            .await
            .with_context(|| format!("bind TCP {addr} (porta 53 requer admin)"))
        {
            Ok(listener) => {
                tracing::info!(%addr, "DNS proxy escutando em TCP");
                listeners.push(listener);
            }
            Err(e) => {
                tracing::warn!(%addr, error = %e, "falha ao bindar TCP do DNS proxy");
                last_err = Some(e);
            }
        }
    }

    if listeners.is_empty() {
        return Err(last_err.unwrap_or_else(|| anyhow::anyhow!("nenhum listener TCP disponivel")));
    }
    Ok(listeners)
}

// ----- UDP -----------------------------------------------------------------

async fn udp_loop(
    socket: Arc<UdpSocket>,
    rules: Arc<RwLock<HashSet<String>>>,
    adult: Arc<AdultFilter>,
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
        let adult = adult.clone();
        let cache = cache.clone();
        let upstream = upstream.clone();
        tokio::spawn(async move {
            let response = handle_query_bytes(&data, &rules, &adult, &cache, &upstream).await;
            if let Some(bytes) = response {
                if let Err(e) = socket.send_to(&bytes, client).await {
                    tracing::debug!(error = %e, %client, "UDP send_to falhou");
                }
            }
        });
    }
}

// ----- TCP -----------------------------------------------------------------

async fn tcp_loop(
    listener: TcpListener,
    rules: Arc<RwLock<HashSet<String>>>,
    adult: Arc<AdultFilter>,
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
        let adult = adult.clone();
        let cache = cache.clone();
        let upstream = upstream.clone();
        tokio::spawn(async move {
            if let Err(e) = tcp_connection(stream, &rules, &adult, &cache, &upstream).await {
                tracing::debug!(error = %e, %peer, "TCP conn encerrou");
            }
        });
    }
}

async fn tcp_connection(
    mut stream: TcpStream,
    rules: &Arc<RwLock<HashSet<String>>>,
    adult: &Arc<AdultFilter>,
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

        let Some(response) = handle_query_bytes(&msg, rules, adult, cache, upstream).await else {
            continue;
        };
        let resp_len = (response.len() as u16).to_be_bytes();
        stream.write_all(&resp_len).await?;
        stream.write_all(&response).await?;
    }
}

// ----- common logic --------------------------------------------------------

async fn handle_query_bytes(
    data: &[u8],
    rules: &RwLock<HashSet<String>>,
    adult: &AdultFilter,
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

    if let Some(reason) = block_reason::check(&normalized, rules, adult).await {
        tracing::info!(query = %normalized, reason = ?reason, "BLOCK -> 127.0.0.1");
        return build_block_redirect(&msg).ok();
    }

    if let Some(cached) = cache.get(&msg, query_id).await {
        tracing::debug!(query = %normalized, "cache HIT");
        return Some(cached);
    }

    match upstream.resolve(data).await {
        Ok(bytes) => {
            cache.put(&msg, &bytes).await;
            tracing::debug!(query = %normalized, "cache MISS -> upstream OK");
            Some(bytes)
        }
        Err(e) => {
            tracing::warn!(query = %normalized, error = %e, "upstream falhou -> SERVFAIL");
            build_servfail(&msg).ok()
        }
    }
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

fn build_block_redirect(query_msg: &Message) -> Result<Vec<u8>> {
    let mut response = Message::new();
    response.set_id(query_msg.id());
    response.set_message_type(MessageType::Response);
    response.set_op_code(query_msg.op_code());
    response.set_response_code(ResponseCode::NoError);
    response.set_recursion_desired(query_msg.recursion_desired());
    response.set_recursion_available(true);
    response.set_authoritative(false);

    let Some(query) = query_msg.queries().first().cloned() else {
        return response.to_bytes().context("encode redirect sem queries");
    };
    let qtype = query.query_type();
    let qname = query.name().clone();
    response.add_query(query);

    match qtype {
        RecordType::A => {
            let record = Record::from_rdata(qname, 60, RData::A(A(Ipv4Addr::LOCALHOST)));
            response.add_answer(record);
        }
        RecordType::AAAA => {}
        _ => {
            response.set_response_code(ResponseCode::NXDomain);
        }
    }

    response.to_bytes().context("encode block redirect")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    fn listener_addrs_cover_ipv4_and_ipv6_loopback() {
        let addrs = listener_addrs(53);
        assert_eq!(addrs.len(), 2);
        assert_eq!(addrs[0].ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(addrs[1].ip(), IpAddr::V6(Ipv6Addr::LOCALHOST));
    }
}
