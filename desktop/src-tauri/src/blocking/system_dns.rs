// =============================================================================
// Controle do DNS do sistema Windows via `netsh`.
// =============================================================================
// O motor de bloqueio vive no loopback local, entao precisamos apontar o DNS
// do sistema para ele. O bug real observado em Windows 11 era que trocavamos
// apenas o DNS IPv4; o sistema continuava com DNS IPv6 ativo e resolvia por
// fora do proxy. Este modulo agora captura/restaura IPv4 + IPv6.
// =============================================================================

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio_rusqlite::Connection;

use crate::db;

const PROXY_IPV4: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const PROXY_IPV6: IpAddr = IpAddr::V6(Ipv6Addr::LOCALHOST);
const STATE_KEY: &str = "original_dns_config";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DnsFamily {
    V4,
    V6,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DnsSource {
    Dhcp,
    Static,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceDnsConfig {
    pub name: String,
    #[serde(default = "default_dns_family")]
    pub family: DnsFamily,
    pub source: DnsSource,
    pub servers: Vec<IpAddr>,
}

fn default_dns_family() -> DnsFamily {
    DnsFamily::V4
}

// -------- high-level orchestration -----------------------------------------

pub async fn apply_and_remember(conn: &Connection) -> Result<()> {
    if !cfg!(target_os = "windows") {
        return Ok(());
    }
    let current = capture_current().await.context("capturar DNS atual")?;
    if current.is_empty() {
        bail!("nenhuma interface de rede ativa");
    }

    let json = serde_json::to_string(&current).context("serializar snapshot")?;
    db::set_state(conn, STATE_KEY, json)
        .await
        .context("persistir snapshot de DNS")?;

    if let Err(apply_err) = apply_proxy_dns(&current).await {
        let _ = db::clear_state(conn, STATE_KEY).await;
        return Err(apply_err).context("aplicar proxy DNS");
    }
    Ok(())
}

pub async fn restore_if_any(conn: &Connection) -> Result<()> {
    if !cfg!(target_os = "windows") {
        return Ok(());
    }
    let Some(json) = db::get_state(conn, STATE_KEY).await? else {
        return Ok(());
    };
    if json.is_empty() {
        return Ok(());
    }
    let snapshot: Vec<InterfaceDnsConfig> =
        serde_json::from_str(&json).context("deserializar snapshot")?;
    restore_all(&snapshot).await;
    db::clear_state(conn, STATE_KEY).await?;
    flush_resolver_cache().await;
    Ok(())
}

// -------- capture ----------------------------------------------------------

pub async fn capture_current() -> Result<Vec<InterfaceDnsConfig>> {
    let ipv4 = netsh(&["interface", "ipv4", "show", "dnsservers"]).await?;
    let ipv6 = netsh(&["interface", "ipv6", "show", "dnsservers"]).await?;

    let mut out = parse_dnsservers_output_for_family(&ipv4, DnsFamily::V4);
    out.extend(parse_dnsservers_output_for_family(&ipv6, DnsFamily::V6));
    Ok(out)
}

// -------- apply ------------------------------------------------------------

pub async fn apply_proxy_dns(interfaces: &[InterfaceDnsConfig]) -> Result<()> {
    let mut applied = 0;
    let mut last_err: Option<anyhow::Error> = None;

    for cfg in interfaces {
        let proxy_ip = match cfg.family {
            DnsFamily::V4 => PROXY_IPV4,
            DnsFamily::V6 => PROXY_IPV6,
        };

        match set_static_primary(cfg.family, &cfg.name, proxy_ip).await {
            Ok(()) => {
                applied += 1;
                tracing::info!(
                    interface = %cfg.name,
                    family = ?cfg.family,
                    dns = %proxy_ip,
                    "DNS apontado para o proxy"
                );
            }
            Err(e) => {
                tracing::warn!(
                    interface = %cfg.name,
                    family = ?cfg.family,
                    error = %e,
                    "falha ao setar DNS"
                );
                last_err = Some(e);
            }
        }
    }

    if applied == 0 {
        return Err(last_err.unwrap_or_else(|| anyhow::anyhow!("nenhuma interface elegivel")));
    }

    flush_resolver_cache().await;
    Ok(())
}

// -------- restore ----------------------------------------------------------

pub async fn restore_all(configs: &[InterfaceDnsConfig]) {
    for cfg in configs {
        if let Err(e) = restore_one(cfg).await {
            tracing::warn!(
                interface = %cfg.name,
                family = ?cfg.family,
                error = %e,
                "falha ao restaurar DNS"
            );
        } else {
            tracing::info!(interface = %cfg.name, family = ?cfg.family, "DNS restaurado");
        }
    }
}

async fn restore_one(cfg: &InterfaceDnsConfig) -> Result<()> {
    match cfg.source {
        DnsSource::Dhcp => set_dhcp(cfg.family, &cfg.name).await,
        DnsSource::Static => {
            if cfg.servers.is_empty() {
                return set_dhcp(cfg.family, &cfg.name).await;
            }

            set_static_primary(cfg.family, &cfg.name, cfg.servers[0]).await?;
            for (i, ip) in cfg.servers.iter().enumerate().skip(1) {
                netsh(&[
                    "interface",
                    family_label(cfg.family),
                    "add",
                    "dnsservers",
                    &format!("name=\"{}\"", cfg.name),
                    &ip.to_string(),
                    &format!("index={}", i + 1),
                ])
                .await?;
            }
            Ok(())
        }
    }
}

// -------- netsh wrappers ---------------------------------------------------

async fn set_static_primary(family: DnsFamily, iface: &str, ip: IpAddr) -> Result<()> {
    netsh(&[
        "interface",
        family_label(family),
        "set",
        "dnsservers",
        &format!("name=\"{iface}\""),
        "static",
        &ip.to_string(),
        "primary",
        "validate=no",
    ])
    .await
    .map(|_| ())
}

async fn set_dhcp(family: DnsFamily, iface: &str) -> Result<()> {
    netsh(&[
        "interface",
        family_label(family),
        "set",
        "dnsservers",
        &format!("name=\"{iface}\""),
        "source=dhcp",
    ])
    .await
    .map(|_| ())
}

fn family_label(family: DnsFamily) -> &'static str {
    match family {
        DnsFamily::V4 => "ipv4",
        DnsFamily::V6 => "ipv6",
    }
}

async fn netsh(args: &[&str]) -> Result<String> {
    let out = Command::new("netsh")
        .args(args)
        .output()
        .await
        .context("spawn netsh")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let msg = if !stderr.is_empty() { stderr } else { stdout };
        let hint = if msg.to_lowercase().contains("access")
            || msg.contains("negado")
            || msg.contains("Elevation")
        {
            " (executar o app como administrador)"
        } else {
            ""
        };
        bail!("netsh falhou{hint}: {msg}");
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

async fn flush_resolver_cache() {
    match Command::new("ipconfig").args(["/flushdns"]).output().await {
        Ok(out) if out.status.success() => {
            tracing::info!("cache DNS do Windows limpo");
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let msg = if !stderr.is_empty() { stderr } else { stdout };
            tracing::warn!(error = %msg, "falha ao limpar cache DNS do Windows");
        }
        Err(e) => {
            tracing::warn!(error = %e, "nao foi possivel executar ipconfig /flushdns");
        }
    }
}

// -------- parser -----------------------------------------------------------

#[cfg_attr(not(test), allow(dead_code))]
fn parse_dnsservers_output(text: &str) -> Vec<InterfaceDnsConfig> {
    let family = if text.contains("::") || text.contains('%') {
        DnsFamily::V6
    } else {
        DnsFamily::V4
    };
    parse_dnsservers_output_for_family(text, family)
}

fn parse_dnsservers_output_for_family(text: &str, family: DnsFamily) -> Vec<InterfaceDnsConfig> {
    let mut out = Vec::new();
    let mut current: Option<InterfaceDnsConfig> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(name) = extract_quoted_name(trimmed) {
            if let Some(prev) = current.take() {
                push_if_usable(&mut out, prev);
            }
            current = Some(InterfaceDnsConfig {
                name,
                family,
                source: DnsSource::Static,
                servers: Vec::new(),
            });
            continue;
        }

        let Some(cfg) = current.as_mut() else {
            continue;
        };
        if trimmed.to_lowercase().contains("dhcp") {
            cfg.source = DnsSource::Dhcp;
        }
        if let Some(ip) = extract_ip(trimmed, family) {
            cfg.servers.push(ip);
        }
    }

    if let Some(last) = current.take() {
        push_if_usable(&mut out, last);
    }
    out
}

fn extract_quoted_name(line: &str) -> Option<String> {
    let first = line.find('"')?;
    let rest = &line[first + 1..];
    let last = rest.find('"')?;
    Some(rest[..last].to_string())
}

fn extract_ip(line: &str, family: DnsFamily) -> Option<IpAddr> {
    line.split_whitespace()
        .filter_map(normalize_ip_token)
        .filter_map(|tok| tok.parse::<IpAddr>().ok())
        .find(|ip| match family {
            DnsFamily::V4 => ip.is_ipv4(),
            DnsFamily::V6 => ip.is_ipv6(),
        })
}

fn normalize_ip_token(tok: &str) -> Option<String> {
    let trimmed = tok.trim_end_matches(&[',', ';'][..]);
    if trimmed.is_empty() {
        return None;
    }
    let without_zone = match trimmed.find('%') {
        Some(idx) => &trimmed[..idx],
        None => trimmed,
    };
    Some(without_zone.to_string())
}

fn push_if_usable(out: &mut Vec<InterfaceDnsConfig>, cfg: InterfaceDnsConfig) {
    let lower = cfg.name.to_lowercase();
    if lower.contains("loopback") {
        return;
    }
    out.push(cfg);
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_EN: &str = r#"
Configuration for interface "Loopback Pseudo-Interface 1"
    Statically Configured DNS Servers:    None
    Register with which suffix:           None

Configuration for interface "Wi-Fi"
    DNS servers configured through DHCP:  192.168.1.1
                                           192.168.1.2
    Register with which suffix:           Primary only

Configuration for interface "Ethernet 3"
    Statically Configured DNS Servers:    8.8.8.8
                                           1.1.1.1
    Register with which suffix:           Primary only
"#;

    const SAMPLE_PT: &str = r#"
Configuracao para interface "Loopback Pseudo-Interface 1"
    Servidores DNS configurados estaticamente:    Nenhum

Configuracao para interface "Wi-Fi"
    Servidores DNS configurados por DHCP:  192.168.0.1
    Registrar com qual sufixo:             Somente primario
"#;

    #[test]
    fn parses_english_output() {
        let cfgs = parse_dnsservers_output(SAMPLE_EN);
        assert_eq!(cfgs.len(), 2, "loopback deve ser filtrado");
        assert_eq!(cfgs[0].name, "Wi-Fi");
        assert_eq!(cfgs[0].family, DnsFamily::V4);
        assert_eq!(cfgs[0].source, DnsSource::Dhcp);
        assert_eq!(cfgs[0].servers.len(), 2);
        assert_eq!(cfgs[1].name, "Ethernet 3");
        assert_eq!(cfgs[1].source, DnsSource::Static);
        assert_eq!(cfgs[1].servers.len(), 2);
    }

    #[test]
    fn parses_portuguese_output() {
        let cfgs = parse_dnsservers_output(SAMPLE_PT);
        assert_eq!(cfgs.len(), 1);
        assert_eq!(cfgs[0].name, "Wi-Fi");
        assert_eq!(cfgs[0].family, DnsFamily::V4);
        assert_eq!(cfgs[0].source, DnsSource::Dhcp);
        assert_eq!(
            cfgs[0].servers,
            vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]
        );
    }

    #[test]
    fn extracts_quoted_name() {
        assert_eq!(
            extract_quoted_name(r#"Configuration for interface "Wi-Fi""#),
            Some("Wi-Fi".to_string())
        );
        assert_eq!(extract_quoted_name("no quotes here"), None);
    }

    #[test]
    fn parses_ipv6_dns_servers_and_strips_zone_ids() {
        let sample = r#"
Configuracao da interface "Ethernet"
    Servidores DNS configurados por DHCP:  fe80::860b:bbff:fe1b:2288%6
                                           2606:4700:4700::1111
    Registrar com o sufixo:           Somente principal
"#;

        let cfgs = parse_dnsservers_output(sample);
        assert_eq!(cfgs.len(), 1);
        assert_eq!(cfgs[0].name, "Ethernet");
        assert_eq!(cfgs[0].family, DnsFamily::V6);
        assert_eq!(cfgs[0].source, DnsSource::Dhcp);
        assert_eq!(
            cfgs[0].servers,
            vec![
                IpAddr::V6("fe80::860b:bbff:fe1b:2288".parse().unwrap()),
                IpAddr::V6("2606:4700:4700::1111".parse().unwrap()),
            ]
        );
    }
}
