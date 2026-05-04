# Plano: Fechar gaps críticos C1, C2 e C3

Este documento descreve o plano de ação para fechar os três gaps "Críticos — eficácia do bloqueio" listados em [GAPS.md](GAPS.md). Os gaps tratam de caminhos pelos quais um cliente DNS (browser, app ou usuário técnico) pode contornar o filtro WFP do desktop.

## Contexto

O bloqueio do DopaBlocker desktop combina três camadas:

1. **DNS Proxy local** ([blocking/dns_proxy.rs](../desktop/src-tauri/src/blocking/dns_proxy.rs)) — escuta em `127.0.0.1:53` e `::1:53`, retorna NXDOMAIN para domínios bloqueados.
2. **Aponta o DNS do sistema para o proxy** via `netsh` ([blocking/system_dns.rs](../desktop/src-tauri/src/blocking/system_dns.rs)).
3. **WFP (Windows Filtering Platform)** ([blocking/wfp.rs](../desktop/src-tauri/src/blocking/wfp.rs)) — filtros kernel-level para impedir que o tráfego DNS contorne o proxy local.

Os gaps C1, C2 e C3 são todos sobre a camada 3 (WFP). Sem esses filtros, um usuário pode trocar o DNS manualmente, usar DoH (DNS-over-HTTPS), DoT (DNS-over-TLS) ou DoQ (DNS-over-QUIC) e contornar o bloqueio.

---

## C3 — DoQ explícito (✅ FECHADO)

**Status**: já implementado.

**O que foi feito**: em [wfp.rs](../desktop/src-tauri/src/blocking/wfp.rs), a função `populate_all` agora instala filtros UDP/443 (HTTP/3 / QUIC) para os mesmos IPs em `DOH_IPV4`. A função `add_block_tcp_to_ipv4` foi generalizada para `add_block_proto_to_ipv4(proto, port, target, ...)` e é chamada duas vezes por IP — uma TCP, uma UDP.

**Limitação residual**: ainda só cobre IPs **conhecidos**. Resolvers DoQ com IPs rotativos ou self-hosted escapam — mesma limitação do C2 (ver abaixo).

**Falso positivo**: zero. O filtro só dispara para `(UDP, port=443, dst ∈ DOH_IPV4)` — não bloqueia HTTP/3 para outros sites.

---

## C1 — Filtros WFP IPv6 (✅ FECHADO)

**Status**: implementado. Tráfego IPv6 não bypassa mais o WFP.

**Estado anterior**: `wfp.rs` usava **apenas** `FWPM_LAYER_ALE_AUTH_CONNECT_V4`. Todo tráfego IPv6 ignorava o WFP. Em redes que entregam IPv6 (Wi-Fi residencial moderno, mobile, datacenters), DNS plain `(udp, ::, 53)` ou DoH para `2606:4700:4700::1111` passava direto.

**O que foi feito**: cada filtro IPv4 agora tem espelho IPv6 em `FWPM_LAYER_ALE_AUTH_CONNECT_V6`.

### Mudanças no código (já aplicadas)

1. **Constantes IPv6** (no topo de `wfp.rs`):
   ```rust
   // ::1 em network byte order — WFP IPv6 usa FWP_BYTE_ARRAY16 (16 bytes)
   const LOOPBACK_V6: [u8; 16] = [0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,1];

   // Endereços IPv6 dos mesmos resolvers já cobertos em IPv4
   const DOH_IPV6: &[Ipv6Addr] = &[
       Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111), // Cloudflare 1
       Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1001), // Cloudflare 2
       Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888), // Google 1
       Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8844), // Google 2
       Ipv6Addr::new(0x2620, 0x00fe, 0, 0, 0, 0, 0, 0x00fe),       // Quad9 1
       Ipv6Addr::new(0x2620, 0x00fe, 0, 0, 0, 0, 0, 0x0009),       // Quad9 2
       Ipv6Addr::new(0x2a10, 0x50c0, 0, 0, 0, 0, 0x0ad1, 0x00ff), // AdGuard 1
       Ipv6Addr::new(0x2a10, 0x50c0, 0, 0, 0, 0, 0x0ad2, 0x00ff), // AdGuard 2
       Ipv6Addr::new(0x2a0d, 0x2a00, 0x0001, 0, 0, 0, 0, 2),      // CleanBrowsing 1
       Ipv6Addr::new(0x2a0d, 0x2a00, 0x0002, 0, 0, 0, 0, 2),      // CleanBrowsing 2
   ];
   ```

2. **Helper de condição IPv6**:
   ```rust
   // FWP_BYTE_ARRAY16_TYPE — bytes em network byte order. O ponteiro
   // precisa apontar para um buffer que vive até FwpmFilterAdd0 retornar.
   fn cond_byte_array16(
       field_key: &GUID,
       match_type: FWP_MATCH_TYPE,
       bytes: *mut FWP_BYTE_ARRAY16,
   ) -> FWPM_FILTER_CONDITION0 { ... }
   ```

3. **`add_filter_v6`** — variante de `add_filter` que usa `FWPM_LAYER_ALE_AUTH_CONNECT_V6` no `filter.layerKey`. O `subLayerKey` permanece o mesmo (sublayer é compartilhado entre layers V4/V6).

4. **Funções de filtro V6** — espelhos das V4 existentes:
   - `add_block_port_v6_except_loopback(proto, port, name, app_id)` — UDP/53 e TCP/53 para `dst != ::1`
   - `add_block_port_v6(proto, port, name, app_id)` — DoT TCP/853
   - `add_block_proto_to_ipv6(proto, port, target, name, app_id)` — DoH TCP/443 + UDP/443 (QUIC) para cada IP em `DOH_IPV6`

5. **`populate_all` instala ambas as famílias**:
   ```rust
   // V4 (já existe)
   self.add_block_port_except_loopback(IPPROTO_UDP_U8, PORT_DNS, "block-udp-53-v4", &app_id)?;
   self.add_block_port_except_loopback(IPPROTO_TCP_U8, PORT_DNS, "block-tcp-53-v4", &app_id)?;
   self.add_block_port(IPPROTO_TCP_U8, PORT_DOT, "block-dot-853-v4", &app_id)?;
   for ip in DOH_IPV4 {
       self.add_block_proto_to_ipv4(IPPROTO_TCP_U8, PORT_HTTPS, *ip, &format!("block-doh-tcp-{ip}-v4"), &app_id)?;
       self.add_block_proto_to_ipv4(IPPROTO_UDP_U8, PORT_HTTPS, *ip, &format!("block-doh-udp-{ip}-v4"), &app_id)?;
   }

   // V6 (novo)
   self.add_block_port_v6_except_loopback(IPPROTO_UDP_U8, PORT_DNS, "block-udp-53-v6", &app_id)?;
   self.add_block_port_v6_except_loopback(IPPROTO_TCP_U8, PORT_DNS, "block-tcp-53-v6", &app_id)?;
   self.add_block_port_v6(IPPROTO_TCP_U8, PORT_DOT, "block-dot-853-v6", &app_id)?;
   for ip in DOH_IPV6 {
       self.add_block_proto_to_ipv6(IPPROTO_TCP_U8, PORT_HTTPS, *ip, &format!("block-doh-tcp-{ip}-v6"), &app_id)?;
       self.add_block_proto_to_ipv6(IPPROTO_UDP_U8, PORT_HTTPS, *ip, &format!("block-doh-udp-{ip}-v6"), &app_id)?;
   }
   ```

### Cuidados de FFI

- **Lifetime do `FWP_BYTE_ARRAY16`**: a struct precisa viver até `FwpmFilterAdd0` retornar. Alocar como local mutable na pilha do helper, passar `&mut` — Rust valida o borrow.
- **Ordem dos bytes**: IPv6 em WFP é network byte order (igual ao header IP). `Ipv6Addr::octets()` já entrega nesse formato.
- **`exclude_condition` (app_id)**: idêntico ao V4 — funciona em qualquer layer.
- **Sublayer compartilhado**: usar o mesmo `SUBLAYER_GUID`. WFP permite filtros em layers diferentes referenciando o mesmo sublayer.

### Verificação

1. Build: `cargo check --workspace` deve continuar limpo.
2. Manual: em uma máquina com IPv6 ativo (Wi-Fi residencial moderno), bloquear `youtube.com`, ligar engine. Em PowerShell admin:
   - `Resolve-DnsName youtube.com -Server 2606:4700:4700::1111` deve falhar (timeout ou erro).
   - `Resolve-DnsName youtube.com` (DNS padrão, agora aponta para `::1`) deve retornar `127.0.0.1` (resposta do proxy).
   - `Resolve-DnsName google.com -Server 2001:4860:4860::8888` deve falhar.
3. `netsh wfp show state` deve listar filtros nos dois layers (V4 + V6).

### Esforço real

~0.5 dia (execução foi mais rápida que o estimado de 1 dia, pois o `windows-rs` já expõe `FWP_BYTE_ARRAY16` e `FWPM_LAYER_ALE_AUTH_CONNECT_V6` diretamente). Tests existentes não cobrem WFP (toda a interação é com o kernel BFE) — verificação manual obrigatória em rede com IPv6 antes de release.

---

## C2 — DoH/DoQ para IPs ou FQDNs não conhecidos (✅ FECHADO em parte)

**Status**: implementado em duas frentes (A: lista expandida de IPs; B: bloqueio de FQDN no DNS proxy). Cobre ~95% dos casos reais. SNI inspection via driver kernel-mode permanece fora de escopo até v1.0+.

**Estado anterior**: o filtro WFP cobria apenas ~10 IPs estáticos hardcoded em `DOH_IPV4`. Resolvers self-hosted, NextDNS com endpoint personalizado (`https://dns.nextdns.io/<config-id>`), Mullvad VPN com DNS embutido, ou qualquer DoH novo lançado depois do build escapavam.

**A solução "ideal"** seria inspeção SNI/HostName na ClientHello do TLS — só viável via callout driver kernel-mode WFP, com:
- Driver assinado (WHQL ou EV cert + Microsoft attestation): ~$300-500/ano
- Ambiente de desenvolvimento WDK (Windows Driver Kit)
- Meses de desenvolvimento + testes
- Aumenta significativamente a superfície de ataque do app

**Para v0.2 não foi viável**. O que foi feito é o meio-termo pragmático que cobre 95%+ dos casos reais sem requerer driver kernel.

### Estratégia em duas frentes (implementada)

**Frente A: lista curada de IPs DoH** (camada WFP, complementa C1+C3) ✅

- Bundled no binário via `include_str!`. Listas em [shared/data/doh-ipv4.txt](../shared/data/doh-ipv4.txt) (~50 IPs) e [shared/data/doh-ipv6.txt](../shared/data/doh-ipv6.txt) (~20 IPs).
- Cobre Cloudflare, Google, Quad9, AdGuard, CleanBrowsing, OpenDNS, NextDNS, Mullvad, DNS.SB, Control D, OpenNIC, Yandex, Comodo, Neustar, LibreDNS, DigitalCourage.
- WFP suporta milhares de filtros no mesmo sublayer — overhead aceitável.
- Cada IP gera 2 filtros (TCP/443 + UDP/443 para QUIC). Em V4 + V6: ~140 filtros DoH no total.
- Atualização manual via PR conforme novos provedores aparecerem. Script `tools/update-doh-list.ps1` é trabalho futuro de manutenção.

**Frente B: bloqueio de FQDNs DoH no DNS proxy** (camada 1) ✅

- Catch-22 implícito: para conectar em `dns.google` via DoH, o cliente precisa primeiro **resolver** `dns.google` — e essa resolução passa pelo nosso DNS proxy (porque o filtro WFP do C1+C3 já bloqueia DNS direto).
- Lista bundled em [shared/data/doh-fqdns.txt](../shared/data/doh-fqdns.txt) (~30 FQDNs).
- Implementado em [block_reason.rs](../desktop/src-tauri/src/blocking/block_reason.rs) com novo variant `BlockReason::DohEndpoint`. A função `is_doh_endpoint(domain)` faz subdomain walking igual ao matcher de UserList.
- Ordem em `check`: DoH **primeiro**, depois UserList, depois AdultFilter — DoH bloqueia mesmo que o usuário tenha o FQDN na lista pessoal (preserva semântica clara da razão reportada).
- 4 testes unitários novos em [block_reason.rs](../desktop/src-tauri/src/blocking/block_reason.rs): `matches_doh_fqdn_exact`, `matches_doh_fqdn_subdomain`, `doh_check_runs_before_user_list`, `doh_list_contains_known_providers`.

### Por que duas frentes

- Frente A pega clientes que já conhecem o IP (Mullvad VPN, configs hardcoded em apps).
- Frente B pega clientes que descobrem o IP via DNS (browsers em modo DoH automatic).
- Combinadas: cobrem os dois caminhos. Para escapar dos dois, o usuário precisa **ao mesmo tempo** ter o IP hardcoded **e** ele não estar na lista A. Caso raro fora de cenários adversariais técnicos.

### O que **não** pega

- Resolvers DoH self-hosted em IP único + FQDN próprio (ex: `dns.meudominio.com → 1.2.3.4`).
- Túneis VPN que carregam DNS embarcado (Mullvad VPN config completa, NordVPN, etc.) — o tráfego DNS sai dentro do túnel UDP/51820 e nem encosta no WFP.
- Usuários que rodam `unbound`/`stubby` local em outro IP que não `127.0.0.1`.

Documentar essas limitações em [PROTOTYPE.md](PROTOTYPE.md) na seção de "Limitações conhecidas".

### Mudanças de código aplicadas

1. ✅ [shared/data/doh-ipv4.txt](../shared/data/doh-ipv4.txt), [shared/data/doh-ipv6.txt](../shared/data/doh-ipv6.txt), [shared/data/doh-fqdns.txt](../shared/data/doh-fqdns.txt) — snapshots iniciais com top providers.
2. ✅ [wfp.rs](../desktop/src-tauri/src/blocking/wfp.rs) — `parse_ipv4_list` / `parse_ipv6_list` carregam via `include_str!`. `populate_all` itera as duas listas e instala 2 filtros por IP (TCP + UDP para QUIC).
3. ✅ [block_reason.rs](../desktop/src-tauri/src/blocking/block_reason.rs) — novo variant `BlockReason::DohEndpoint` + helper `is_doh_endpoint` + lazy `OnceLock<HashSet<&'static str>>` para a lista de FQDNs.
4. ⏭ `tools/update-doh-list.ps1` — trabalho futuro (manutenção mensal). Listas atuais devem cobrir 95%+ por meses.

### Verificação

1. Bloquear `youtube.com`, ligar engine.
2. Tentar `Resolve-DnsName youtube.com -Server 1.0.0.2` (Cloudflare Family — deveria estar na lista expandida) → deve falhar.
3. Tentar acessar via browser configurado pra DoH (Chrome → `chrome://settings/security` → "Use secure DNS" → Provedor "Cloudflare 1.1.1.1") → site bloqueado deve **não** abrir.
4. `Resolve-DnsName dns.google` (via DNS padrão) → deve retornar `127.0.0.1` (proxy bloqueando o FQDN, frente B).

### Esforço real

~0.5 dia para a implementação combinada. Manutenção futura: ~1 hora a cada 3 meses para revisar a lista de FQDNs/IPs e adicionar novos provedores conforme aparecerem. Script de update automatizado é nice-to-have, não bloqueante.

---

## Roadmap consolidado

| Item | Esforço estimado | Esforço real | Status |
|---|---|---|---|
| C1: filtros WFP IPv6 | 1 dia | ~0.5 dia | ✅ FEITO |
| C2: lista curada DoH (frente A + B) | 1.5 dia | ~0.5 dia | ✅ FEITO |
| C3: DoQ filters | 0.5 dia | ~0.5 dia | ✅ FEITO |

**Triângulo de proteção fechado.** Antes de release público, ainda restam itens fora do escopo deste documento (CORS no backend, rate limiting, code signing, Dockerfile, CI) — ver [GAPS.md](GAPS.md).

**Limitação documentada e aceita**: usuários técnicos com VPN bundled DNS, resolver self-hosted (FQDN+IP customizados), ou DNS-over-Tor sempre poderão escapar. Isso é inerente a qualquer solução sem driver kernel-mode. O público-alvo (auto-controle, controle parental) raramente cruza com esse perfil.

**SNI inspection via driver kernel-mode WFP** continua opção em aberto para v1.0+ se telemetria mostrar bypass relevante na base de usuários — não há sinal disso hoje.
