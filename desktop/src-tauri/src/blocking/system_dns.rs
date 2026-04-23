// =============================================================================
// Controle do DNS do sistema Windows via `netsh`.
// =============================================================================
// Por que netsh e não a API Win32 (IpHelper)? A API nativa devolve os IPs de
// DNS mas só permite alterar via `NotifyRoute...` + registry writes — ~150
// linhas de unsafe FFI. `netsh` é estável há 25 anos, presente em todo
// Windows, e os comandos de DNS que usamos (set static / set dhcp) são
// garantidos idempotentes. Custa um process spawn (~50ms) por operação,
// aceitável porque chamamos só no toggle.
//
// Fluxo "usuário liga bloqueio":
//   1. `capture_current()` — roda `netsh show dnsservers`, parseia blocos
//      por interface. IPv4 apenas (IPv6 é gap documentado — a maioria do
//      tráfego segue por A, mas DoH via IPv6 escapa).
//   2. Persiste a snapshot em `blocking_state` como JSON — serve de rede de
//      segurança contra crash (se o app morrer, o próximo boot restaura).
//   3. `apply_proxy_dns` aponta todas as interfaces para `127.0.0.1`.
//
// Fluxo "desliga":
//   1. `restore_all` aplica a snapshot. Se a interface estava em DHCP,
//      volta pra `source=dhcp`; se estava em static, seta os IPs saved.
//   2. Remove a snapshot do DB.
//
// Fluxo "app subiu após crash":
//   1. `restore_if_any` — se tem snapshot, restaura antes de qualquer coisa.
//      Isso garante que nunca re-capturamos 127.0.0.1 como "original".
//   2. Depois a lógica normal de resume do engine.
//
// Parsing é locale-independente: detecta DHCP por case-insensitive "dhcp"
// (acrônimo, igual em PT e EN), nomes de interface vêm sempre entre aspas,
// IPs são IPv4 standard. Testado com outputs EN e PT BR.
// =============================================================================

use std::net::{IpAddr, Ipv4Addr};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio_rusqlite::Connection;

use crate::db;

const PROXY_IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const STATE_KEY: &str = "original_dns_config";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DnsSource {
    Dhcp,
    Static,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceDnsConfig {
    pub name: String,
    pub source: DnsSource,
    pub servers: Vec<IpAddr>,
}

// -------- high-level orchestration -----------------------------------------

/// Captura o DNS atual + persiste no DB + aponta todas as interfaces pro
/// proxy. Operação pensada pra ser atômica do ponto de vista do usuário:
/// ou completa tudo ou volta atrás.
pub async fn apply_and_remember(conn: &Connection) -> Result<()> {
    if !cfg!(target_os = "windows") {
        return Ok(()); // no-op em outros SOs — útil pra dev local
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
        // Rollback do DB — sem a snapshot, não há "estado pendente" que o
        // próximo boot vá tentar restaurar.
        let _ = db::clear_state(conn, STATE_KEY).await;
        return Err(apply_err).context("aplicar proxy DNS");
    }
    Ok(())
}

/// Restaura qualquer snapshot pendente (se houver) e limpa o slot no DB.
/// Idempotente: sem snapshot, no-op.
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
    Ok(())
}

// -------- capture ----------------------------------------------------------

/// Lista as interfaces ativas (não-loopback) com suas configs atuais de DNS.
pub async fn capture_current() -> Result<Vec<InterfaceDnsConfig>> {
    let output = netsh(&["interface", "ipv4", "show", "dnsservers"]).await?;
    Ok(parse_dnsservers_output(&output))
}

// -------- apply ------------------------------------------------------------

/// Aplica `127.0.0.1` como único DNS em todas as interfaces. Erros por
/// interface são logados; se *todas* falharem, retorna erro.
pub async fn apply_proxy_dns(interfaces: &[InterfaceDnsConfig]) -> Result<()> {
    let mut applied = 0;
    let mut last_err: Option<anyhow::Error> = None;
    for cfg in interfaces {
        match set_static_primary(&cfg.name, PROXY_IP).await {
            Ok(()) => {
                applied += 1;
                tracing::info!(interface = %cfg.name, "DNS → 127.0.0.1");
            }
            Err(e) => {
                tracing::warn!(interface = %cfg.name, error = %e, "falha ao setar DNS");
                last_err = Some(e);
            }
        }
    }
    if applied == 0 {
        return Err(last_err.unwrap_or_else(|| anyhow::anyhow!("nenhuma interface elegível")));
    }
    Ok(())
}

// -------- restore ----------------------------------------------------------

/// Tenta restaurar cada interface ao snapshot. Best-effort: erros individuais
/// são logados mas não abortam a restauração das outras. Na prática isso é
/// importante — se uma interface foi desconectada entre o capture e o
/// restore, não queremos deixar as outras sem restaurar.
pub async fn restore_all(configs: &[InterfaceDnsConfig]) {
    for cfg in configs {
        if let Err(e) = restore_one(cfg).await {
            tracing::warn!(interface = %cfg.name, error = %e, "falha ao restaurar DNS");
        } else {
            tracing::info!(interface = %cfg.name, "DNS restaurado");
        }
    }
}

async fn restore_one(cfg: &InterfaceDnsConfig) -> Result<()> {
    match cfg.source {
        DnsSource::Dhcp => set_dhcp(&cfg.name).await,
        DnsSource::Static => {
            if cfg.servers.is_empty() {
                // Estava static sem nenhum DNS — caso raro, patológico mesmo.
                // Fallback pra DHCP é mais útil que "sem DNS nenhum" depois
                // do app desligar.
                return set_dhcp(&cfg.name).await;
            }
            // Primeiro seta o primary; `add` acumula os demais.
            set_static_primary(&cfg.name, cfg.servers[0]).await?;
            for (i, ip) in cfg.servers.iter().enumerate().skip(1) {
                netsh(&[
                    "interface",
                    "ipv4",
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

async fn set_static_primary(iface: &str, ip: IpAddr) -> Result<()> {
    netsh(&[
        "interface",
        "ipv4",
        "set",
        "dnsservers",
        &format!("name=\"{iface}\""),
        "static",
        &ip.to_string(),
        "primary",
        // validate=no evita que netsh tente "ping" no DNS novo antes de setar
        // — inútil pra nós (o proxy já está subido) e atrasa uns 5s.
        "validate=no",
    ])
    .await
    .map(|_| ())
}

async fn set_dhcp(iface: &str) -> Result<()> {
    netsh(&[
        "interface",
        "ipv4",
        "set",
        "dnsservers",
        &format!("name=\"{iface}\""),
        "source=dhcp",
    ])
    .await
    .map(|_| ())
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
        // Mensagem específica pra caso clássico — admin faltando.
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

// -------- parser -----------------------------------------------------------

fn parse_dnsservers_output(text: &str) -> Vec<InterfaceDnsConfig> {
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
                source: DnsSource::Static,
                servers: Vec::new(),
            });
            continue;
        }
        let Some(cfg) = current.as_mut() else { continue };
        if trimmed.to_lowercase().contains("dhcp") {
            cfg.source = DnsSource::Dhcp;
        }
        if let Some(ip) = extract_ipv4(trimmed) {
            cfg.servers.push(ip);
        }
    }
    if let Some(last) = current.take() {
        push_if_usable(&mut out, last);
    }
    out
}

/// Extrai o nome entre aspas de uma linha "Configuration for interface "Wi-Fi"".
/// Funciona em EN e PT porque o formato "..." é o mesmo.
fn extract_quoted_name(line: &str) -> Option<String> {
    let first = line.find('"')?;
    let rest = &line[first + 1..];
    let last = rest.find('"')?;
    Some(rest[..last].to_string())
}

/// Pega o primeiro IPv4 que aparecer na linha. Ignora lixo (colon, keywords).
fn extract_ipv4(line: &str) -> Option<IpAddr> {
    line.split_whitespace()
        .filter_map(|tok| tok.trim_end_matches(&[',', ';'][..]).parse::<IpAddr>().ok())
        .find(|ip| ip.is_ipv4())
}

fn push_if_usable(out: &mut Vec<InterfaceDnsConfig>, cfg: InterfaceDnsConfig) {
    let lower = cfg.name.to_lowercase();
    // "Loopback Pseudo-Interface 1" (EN) e "Loopback Pseudo-Interface 1" (PT)
    // — nome é igual, keyword "loopback" é universal.
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
Configuração para interface "Loopback Pseudo-Interface 1"
    Servidores DNS configurados estaticamente:    Nenhum

Configuração para interface "Wi-Fi"
    Servidores DNS configurados por DHCP:  192.168.0.1
    Registrar com qual sufixo:             Somente primário
"#;

    #[test]
    fn parses_english_output() {
        let cfgs = parse_dnsservers_output(SAMPLE_EN);
        assert_eq!(cfgs.len(), 2, "loopback deve ser filtrado");
        assert_eq!(cfgs[0].name, "Wi-Fi");
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
        assert_eq!(cfgs[0].source, DnsSource::Dhcp);
        assert_eq!(cfgs[0].servers, vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]);
    }

    #[test]
    fn extracts_quoted_name() {
        assert_eq!(
            extract_quoted_name(r#"Configuration for interface "Wi-Fi""#),
            Some("Wi-Fi".to_string())
        );
        assert_eq!(extract_quoted_name("no quotes here"), None);
    }
}
