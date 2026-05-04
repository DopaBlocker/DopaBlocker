// =============================================================================
// Windows Filtering Platform — filtros kernel-level contra bypass de DNS.
// =============================================================================
// Por que WFP e não apenas DNS proxy:
//   - Um usuário técnico pode trocar o DNS do sistema manualmente (bypass).
//   - Chrome/Firefox fazem DoH (DNS sobre HTTPS) por padrão em muitos
//     mercados — nossas respostas DNS nunca chegam a ser consultadas.
//   - Algum malware/VPN hardcoda 8.8.8.8:53.
//
// Filtros instalados:
//   1. UDP dst:53 ≠ 127.0.0.1 → BLOCK    (DNS plain-text fora do proxy)
//   2. TCP dst:53 ≠ 127.0.0.1 → BLOCK    (idem, via TCP)
//   3. TCP dst:853             → BLOCK   (DoT — DNS over TLS)
//   4. TCP dst:443 → IPs de resolvers DoH conhecidos → BLOCK
//        Cloudflare, Google, Quad9, AdGuard, CleanBrowsing — cobre ~90%
//        dos casos. Resolvers com IPs rotativos (NextDNS) escapam;
//        pegar esses precisaria SNI inspection via callout driver kernel.
//   5. UDP dst:443 → IPs de resolvers DoH conhecidos → BLOCK
//        HTTP/3 (QUIC) sobre UDP/443. Browsers modernos preferem QUIC quando
//        disponivel; sem este filtro, Chrome/Edge resolvem DoH via QUIC e
//        o bloqueio nao tem efeito ate as conexoes caindo para fallback TCP.
//
// Sessão dinâmica: os filtros vivem enquanto o `engine` HANDLE estiver
// aberto. Quando o processo morre (clean ou crash), o BFE do Windows
// derruba tudo automaticamente — zero estado persistente pra limpar.
//
// Toda FFI aqui é `unsafe`. Padrão seguido:
//   - Structs inicializadas com `zeroed()`, só campos necessários são
//     preenchidos (FWPM_FILTER0 tem ~15 campos, a maioria default-ok).
//   - Códigos de erro WFP (DWORD) são traduzidos em `anyhow::bail` com
//     o valor hex — permite lookup rápido em `FWP_E_*` da doc MS.
//   - `WfpSession` guarda o HANDLE e fecha em Drop — RAII protege mesmo
//     se o caller esquecer de chamar `uninstall`.
// =============================================================================

#![cfg(target_os = "windows")]

use std::mem::zeroed;
use std::net::Ipv4Addr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;

use anyhow::{bail, Context, Result};
use windows::core::{GUID, PCWSTR, PWSTR};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::NetworkManagement::WindowsFilteringPlatform::{
    FwpmEngineClose0, FwpmEngineOpen0, FwpmFilterAdd0, FwpmFreeMemory0, FwpmGetAppIdFromFileName0,
    FwpmProviderAdd0, FwpmSubLayerAdd0, FwpmTransactionAbort0, FwpmTransactionBegin0,
    FwpmTransactionCommit0, FWPM_ACTION0, FWPM_CONDITION_ALE_APP_ID, FWPM_CONDITION_IP_PROTOCOL,
    FWPM_CONDITION_IP_REMOTE_ADDRESS, FWPM_CONDITION_IP_REMOTE_PORT, FWPM_DISPLAY_DATA0,
    FWPM_FILTER0, FWPM_FILTER_CONDITION0, FWPM_LAYER_ALE_AUTH_CONNECT_V4, FWPM_PROVIDER0,
    FWPM_SESSION0, FWPM_SESSION_FLAG_DYNAMIC, FWPM_SUBLAYER0, FWP_ACTION_BLOCK, FWP_BYTE_BLOB,
    FWP_BYTE_BLOB_TYPE, FWP_CONDITION_VALUE0, FWP_CONDITION_VALUE0_0, FWP_EMPTY, FWP_MATCH_EQUAL,
    FWP_MATCH_NOT_EQUAL, FWP_MATCH_TYPE, FWP_UINT16, FWP_UINT32, FWP_UINT8, FWP_VALUE0,
    FWP_VALUE0_0,
};

// GUIDs únicos — geradas uma vez, congeladas. Identificam o provider e o
// sublayer deste app em inspeção (netsh wfp show state).
const PROVIDER_GUID: GUID = GUID::from_u128(0x7b3c9f8a_4f1c_4b8e_9f2e_5a8d6c7e1f2a);
const SUBLAYER_GUID: GUID = GUID::from_u128(0xa1b2c3d4_e5f6_7890_1234_567890abcdef);

// Authn service pra FwpmEngineOpen0. O windows crate não re-exporta de rpcdce.h.
const RPC_C_AUTHN_WINNT: u32 = 10;

// FWP_UINT16 é host byte order — a porta cabe direto sem htons.
const PORT_DNS: u16 = 53;
const PORT_DOT: u16 = 853;
const PORT_HTTPS: u16 = 443;

// IPPROTO — 1 byte no cabeçalho IP. WFP quer FWP_UINT8.
const IPPROTO_TCP_U8: u8 = 6;
const IPPROTO_UDP_U8: u8 = 17;

// Loopback em host byte order: 127.0.0.1 → 0x7F000001. FWP_UINT32 usa
// host byte order (confere MSDN → "A 32-bit unsigned integer that specifies
// an IPv4 address, in host byte order").
const LOOPBACK_V4: u32 = 0x7F00_0001;

/// IPs v4 de resolvers DoH conhecidos. Cobertura boa: Cloudflare, Google,
/// Quad9, AdGuard, CleanBrowsing.
const DOH_IPV4: &[Ipv4Addr] = &[
    Ipv4Addr::new(1, 1, 1, 1),         // Cloudflare primary
    Ipv4Addr::new(1, 0, 0, 1),         // Cloudflare secondary
    Ipv4Addr::new(8, 8, 8, 8),         // Google primary
    Ipv4Addr::new(8, 8, 4, 4),         // Google secondary
    Ipv4Addr::new(9, 9, 9, 9),         // Quad9 primary
    Ipv4Addr::new(149, 112, 112, 112), // Quad9 secondary
    Ipv4Addr::new(94, 140, 14, 14),    // AdGuard primary
    Ipv4Addr::new(94, 140, 15, 15),    // AdGuard secondary
    Ipv4Addr::new(185, 228, 168, 168), // CleanBrowsing
];

pub struct WfpSession {
    engine: HANDLE,
}

struct CurrentAppId {
    blob: *mut FWP_BYTE_BLOB,
}

// HANDLE é `*mut c_void`, que Rust considera não-Send/Sync por default.
// Porém o BFE do Windows garante thread-safety para as operações que usamos
// (MSDN "WFP API thread safety" → "Base Filtering Engine is thread-safe").
// Justificado marcar manualmente; sem isso, Tauri não aceita no State.
unsafe impl Send for WfpSession {}
unsafe impl Sync for WfpSession {}

impl CurrentAppId {
    unsafe fn for_current_process() -> Result<Self> {
        let exe = std::env::current_exe().context("resolver caminho do executavel atual")?;
        let wide: Vec<u16> = exe
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut blob: *mut FWP_BYTE_BLOB = ptr::null_mut();
        check(
            FwpmGetAppIdFromFileName0(PCWSTR(wide.as_ptr()), &mut blob),
            "FwpmGetAppIdFromFileName0",
        )?;
        if blob.is_null() {
            bail!("FwpmGetAppIdFromFileName0 retornou app id nulo");
        }
        Ok(Self { blob })
    }

    fn exclude_condition(&self) -> FWPM_FILTER_CONDITION0 {
        FWPM_FILTER_CONDITION0 {
            fieldKey: FWPM_CONDITION_ALE_APP_ID,
            matchType: FWP_MATCH_NOT_EQUAL,
            conditionValue: FWP_CONDITION_VALUE0 {
                r#type: FWP_BYTE_BLOB_TYPE,
                Anonymous: FWP_CONDITION_VALUE0_0 {
                    byteBlob: self.blob,
                },
            },
        }
    }
}

impl Drop for CurrentAppId {
    fn drop(&mut self) {
        unsafe {
            if self.blob.is_null() {
                return;
            }
            let mut raw = self.blob.cast();
            FwpmFreeMemory0(&mut raw);
            self.blob = ptr::null_mut();
        }
    }
}

impl WfpSession {
    /// Abre o engine, cria provider+sublayer e adiciona todos os filtros
    /// em uma única transação atômica. Se algum passo falhar, aborta
    /// a transação e deixa o Drop fechar o engine.
    pub fn install() -> Result<Self> {
        unsafe {
            let engine = open_engine_dynamic()?;
            let session = WfpSession { engine };

            check(FwpmTransactionBegin0(engine, 0), "FwpmTransactionBegin0")?;

            if let Err(e) = session.populate_all() {
                let _ = FwpmTransactionAbort0(engine);
                return Err(e);
            }

            check(FwpmTransactionCommit0(engine), "FwpmTransactionCommit0")?;

            tracing::info!("WFP: filtros instalados (DNS fora do proxy, DoT, DoH conhecidos)");
            Ok(session)
        }
    }

    unsafe fn populate_all(&self) -> Result<()> {
        let app_id = CurrentAppId::for_current_process()?;
        self.add_provider()?;
        self.add_sublayer()?;

        // Todos são BLOCK no mesmo sublayer — match de qualquer um bloqueia.
        self.add_block_port_except_loopback(IPPROTO_UDP_U8, PORT_DNS, "block-udp-53", &app_id)?;
        self.add_block_port_except_loopback(IPPROTO_TCP_U8, PORT_DNS, "block-tcp-53", &app_id)?;
        self.add_block_port(IPPROTO_TCP_U8, PORT_DOT, "block-dot-853", &app_id)?;
        for ip in DOH_IPV4 {
            let tcp_name = format!("block-doh-tcp-{ip}");
            self.add_block_proto_to_ipv4(IPPROTO_TCP_U8, PORT_HTTPS, *ip, &tcp_name, &app_id)?;
            // QUIC/HTTP3 sobre UDP/443 para os mesmos IPs — sem isso o
            // browser cai pra DoH via QUIC e contorna o filtro TCP.
            let udp_name = format!("block-doh-udp-{ip}");
            self.add_block_proto_to_ipv4(IPPROTO_UDP_U8, PORT_HTTPS, *ip, &udp_name, &app_id)?;
        }
        Ok(())
    }

    // -------- primitives -----------------------------------------------------

    unsafe fn add_provider(&self) -> Result<()> {
        let mut name = to_u16("DopaBlocker");
        let mut desc = to_u16("DNS bypass protection");

        let mut provider: FWPM_PROVIDER0 = zeroed();
        provider.providerKey = PROVIDER_GUID;
        provider.displayData = FWPM_DISPLAY_DATA0 {
            name: PWSTR(name.as_mut_ptr()),
            description: PWSTR(desc.as_mut_ptr()),
        };
        // Sem flag PERSISTENT → some com a sessão dinâmica.

        check(
            FwpmProviderAdd0(self.engine, &provider, None),
            "FwpmProviderAdd0",
        )
    }

    unsafe fn add_sublayer(&self) -> Result<()> {
        let mut name = to_u16("DopaBlocker Filters");
        let mut desc = to_u16("Filtros do DopaBlocker");

        let mut sub: FWPM_SUBLAYER0 = zeroed();
        sub.subLayerKey = SUBLAYER_GUID;
        sub.displayData = FWPM_DISPLAY_DATA0 {
            name: PWSTR(name.as_mut_ptr()),
            description: PWSTR(desc.as_mut_ptr()),
        };
        // `providerKey: *mut GUID` — WFP só lê. Local mut vive até fim do bloco.
        let mut provider_guid = PROVIDER_GUID;
        sub.providerKey = &mut provider_guid as *mut GUID;
        sub.weight = 0x8000; // peso alto pra vencer sublayers default em empate

        check(
            FwpmSubLayerAdd0(self.engine, &sub, None),
            "FwpmSubLayerAdd0",
        )
    }

    /// Filtro `BLOCK` para (protocolo, porta-destino) se o endereço remoto
    /// ≠ loopback. Usado pro DNS tradicional (53 TCP+UDP).
    unsafe fn add_block_port_except_loopback(
        &self,
        proto: u8,
        port: u16,
        name: &str,
        app_id: &CurrentAppId,
    ) -> Result<()> {
        let conditions = [
            cond_uint8(&FWPM_CONDITION_IP_PROTOCOL, FWP_MATCH_EQUAL, proto),
            cond_uint16(&FWPM_CONDITION_IP_REMOTE_PORT, FWP_MATCH_EQUAL, port),
            cond_uint32(
                &FWPM_CONDITION_IP_REMOTE_ADDRESS,
                FWP_MATCH_NOT_EQUAL,
                LOOPBACK_V4,
            ),
            app_id.exclude_condition(),
        ];
        self.add_filter(name, &conditions)
    }

    /// Filtro `BLOCK` bruto: (protocolo, porta-destino). Usado pro DoT (853).
    unsafe fn add_block_port(
        &self,
        proto: u8,
        port: u16,
        name: &str,
        app_id: &CurrentAppId,
    ) -> Result<()> {
        let conditions = [
            cond_uint8(&FWPM_CONDITION_IP_PROTOCOL, FWP_MATCH_EQUAL, proto),
            cond_uint16(&FWPM_CONDITION_IP_REMOTE_PORT, FWP_MATCH_EQUAL, port),
            app_id.exclude_condition(),
        ];
        self.add_filter(name, &conditions)
    }

    /// Filtro `BLOCK` específico: (protocolo, porta) pra um IPv4 conhecido.
    /// Um por IP de resolver DoH × protocolo (TCP para HTTP/2, UDP para QUIC).
    unsafe fn add_block_proto_to_ipv4(
        &self,
        proto: u8,
        port: u16,
        target: Ipv4Addr,
        name: &str,
        app_id: &CurrentAppId,
    ) -> Result<()> {
        let ip_u32: u32 = u32::from(target);
        let conditions = [
            cond_uint8(&FWPM_CONDITION_IP_PROTOCOL, FWP_MATCH_EQUAL, proto),
            cond_uint16(&FWPM_CONDITION_IP_REMOTE_PORT, FWP_MATCH_EQUAL, port),
            cond_uint32(&FWPM_CONDITION_IP_REMOTE_ADDRESS, FWP_MATCH_EQUAL, ip_u32),
            app_id.exclude_condition(),
        ];
        self.add_filter(name, &conditions)
    }

    unsafe fn add_filter(&self, name: &str, conditions: &[FWPM_FILTER_CONDITION0]) -> Result<()> {
        let mut display_name = to_u16(name);
        let mut display_desc = to_u16("Filtro DopaBlocker");

        let mut filter: FWPM_FILTER0 = zeroed();
        filter.displayData = FWPM_DISPLAY_DATA0 {
            name: PWSTR(display_name.as_mut_ptr()),
            description: PWSTR(display_desc.as_mut_ptr()),
        };

        // Associa ao nosso provider — se ele for removido, o filtro vai junto.
        let mut provider_guid = PROVIDER_GUID;
        filter.providerKey = &mut provider_guid as *mut GUID;

        filter.layerKey = FWPM_LAYER_ALE_AUTH_CONNECT_V4;
        filter.subLayerKey = SUBLAYER_GUID;

        // weight type=EMPTY → WFP escolhe peso dentro do sublayer.
        filter.weight = FWP_VALUE0 {
            r#type: FWP_EMPTY,
            Anonymous: FWP_VALUE0_0 { uint8: 0 },
        };

        filter.numFilterConditions = conditions.len() as u32;
        filter.filterCondition = conditions.as_ptr() as *mut FWPM_FILTER_CONDITION0;

        filter.action = FWPM_ACTION0 {
            r#type: FWP_ACTION_BLOCK,
            Anonymous: zeroed(),
        };

        let mut filter_id: u64 = 0;
        check(
            FwpmFilterAdd0(self.engine, &filter, None, Some(&mut filter_id)),
            "FwpmFilterAdd0",
        )?;
        tracing::debug!(%name, %filter_id, "WFP: filtro adicionado");
        Ok(())
    }
}

impl Drop for WfpSession {
    fn drop(&mut self) {
        // Fecha o engine — sessão dinâmica → WFP derruba provider, sublayer
        // e filtros automaticamente. Erros aqui só logam.
        unsafe {
            let rc = FwpmEngineClose0(self.engine);
            if rc != 0 {
                tracing::warn!(code = format!("0x{rc:08x}"), "FwpmEngineClose0 falhou");
            } else {
                tracing::info!("WFP: filtros removidos (engine fechado)");
            }
        }
    }
}

// -------- helpers ----------------------------------------------------------

unsafe fn open_engine_dynamic() -> Result<HANDLE> {
    let mut session: FWPM_SESSION0 = zeroed();
    session.flags = FWPM_SESSION_FLAG_DYNAMIC;
    let mut session_name = to_u16("DopaBlocker Session");
    session.displayData.name = PWSTR(session_name.as_mut_ptr());

    let mut engine = HANDLE(ptr::null_mut());
    let rc = FwpmEngineOpen0(
        PCWSTR::null(),
        RPC_C_AUTHN_WINNT,
        None,
        Some(&session),
        &mut engine,
    );
    if rc != 0 {
        bail!("FwpmEngineOpen0 falhou (0x{rc:08x}) — executar como administrador?");
    }
    Ok(engine)
}

fn check(rc: u32, label: &str) -> Result<()> {
    if rc == 0 {
        Ok(())
    } else {
        bail!("{label} falhou (0x{rc:08x})")
    }
}

fn to_u16(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// -------- condition builders ----------------------------------------------

fn cond_uint8(field_key: &GUID, match_type: FWP_MATCH_TYPE, value: u8) -> FWPM_FILTER_CONDITION0 {
    FWPM_FILTER_CONDITION0 {
        fieldKey: *field_key,
        matchType: match_type,
        conditionValue: FWP_CONDITION_VALUE0 {
            r#type: FWP_UINT8,
            Anonymous: FWP_CONDITION_VALUE0_0 { uint8: value },
        },
    }
}

fn cond_uint16(field_key: &GUID, match_type: FWP_MATCH_TYPE, value: u16) -> FWPM_FILTER_CONDITION0 {
    FWPM_FILTER_CONDITION0 {
        fieldKey: *field_key,
        matchType: match_type,
        conditionValue: FWP_CONDITION_VALUE0 {
            r#type: FWP_UINT16,
            Anonymous: FWP_CONDITION_VALUE0_0 { uint16: value },
        },
    }
}

fn cond_uint32(field_key: &GUID, match_type: FWP_MATCH_TYPE, value: u32) -> FWPM_FILTER_CONDITION0 {
    FWPM_FILTER_CONDITION0 {
        fieldKey: *field_key,
        matchType: match_type,
        conditionValue: FWP_CONDITION_VALUE0 {
            r#type: FWP_UINT32,
            Anonymous: FWP_CONDITION_VALUE0_0 { uint32: value },
        },
    }
}
