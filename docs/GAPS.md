# DopaBlocker — Gaps do Protótipo v0.1

Inventário vivo de tudo que **ficou para depois**: simplificações conscientes,
features fora de escopo, hardening adiado e polimento pendente. Organizado por
severidade para facilitar priorização em v0.1.1 / v0.2.

**Contexto:** A decisão editorial desde o início do desenvolvimento foi
"bloqueio e auth prod-grade; resto simplificado". Cinco cortes pré-etapa-7
foram fechados explicitamente (DNS cache com TTL, normalização server-side,
persistência do engine entre boots, TCP + failover, DoH upstream). Os itens
abaixo são **o que continua aberto**.

---

## 🔴 Críticos — afetam a eficácia do bloqueio

| # | Gap | Onde | Impacto | Saída sugerida |
|---|-----|------|---------|----------------|
| C1 | **Sem filtros IPv6** no WFP | [blocking/wfp.rs](../desktop/src-tauri/src/blocking/wfp.rs) só usa `FWPM_LAYER_ALE_AUTH_CONNECT_V4` | Cliente que prefere IPv6 pra DNS/DoH (comum em ISPs modernos) escapa de todos os filtros | Duplicar filtros no layer `_V6`, mapear IPs DoH pra v6 equivalentes |
| C2 | **Sem DNS proxy em `[::1]:53`** | [blocking/dns_proxy.rs](../desktop/src-tauri/src/blocking/dns_proxy.rs) bind só `127.0.0.1:53` | OS que prefere IPv6 na resolução manda query pro DNS v6 original → bypass | Bindar também `[::1]:53` UDP+TCP, adaptar `system_dns.rs` pra `netsh interface ipv6` |
| C3 | **Troca de DNS só IPv4** | [blocking/system_dns.rs](../desktop/src-tauri/src/blocking/system_dns.rs) usa `netsh interface ipv4` | Mesmo se filtros WFP v6 existirem, o OS ainda tem DNS v6 original que pode vazar antes do filtro ser consultado | Espelhar a lógica de capture/apply/restore pra `interface ipv6` |
| C4 | **DoH sem SNI inspection** | [blocking/wfp.rs](../desktop/src-tauri/src/blocking/wfp.rs) bloqueia DoH só por IP estático | NextDNS/ControlD/resolvers self-hosted usam IPs rotativos não cobertos pela lista | Callout driver kernel-mode (DPI no TLS ClientHello) — projeto de semanas, vira v0.2+ |
| C5 | **Sem DoQ (DNS-over-QUIC) explícito** | UDP/443 não é filtrado especificamente | Protocolo emergente — resolver DoQ em IP não-conhecido passa | Adicionar UDP/443 ao filtro DoH junto com lista de IPs |

---

## 🟠 Funcionais — completude do produto

| # | Gap | Onde | Impacto |
|---|-----|------|---------|
| F1 | **Modo Pais/Filhos não implementado** | [LoginForm.svelte](../desktop/src/lib/components/LoginForm.svelte), [routes/parental/](../desktop/src/routes/parental/) | UI mostra cards "Em breve" mas fluxo não existe. Backend tem `devices/link/*` pronto mas frontend não consome |
| F2 | **Sem sync Firestore real-time** | Backend fala só REST — não tem listeners Firestore | Mudança de blocklist em um device não aparece em outro sem polling manual (v0.1 aceitável; v0.2 esperado) |
| F3 | **Sem rotação/revogação de device tokens via UI** | Backend [services/device_service.rs](../backend/src/services/device_service.rs) suporta; falta tela | Pai não consegue desvincular filho sem mexer no DB direto |
| F4 | **Blocklist única por conta** | Schema global por `user_id` | Múltiplos filhos compartilham lista — requer tabela `child_blocklists` pra v0.2 |
| F5 | **Refresh da lista adulta só em boot** | [blocking/adult_filter.rs](../desktop/src-tauri/src/blocking/adult_filter.rs) `build_if_needed` roda uma vez | App aberto por 10 dias usa lista desatualizada; adicionar task periódica |
| F6 | **Sem múltiplas listas de conteúdo adulto** | Só Steven Black `alternates/porn/hosts` | Combinar com OISD/adguard-content-farms aumentaria cobertura |
| F7 | **`sync_with_backend` só unidirecional** | [lib/stores/blocking.ts](../desktop/src/lib/stores/blocking.ts) | Frontend pull, não push — se offline editar não sincroniza de volta. Precisa fila de writes pendentes |
| F8 | **Sem painel de estatísticas** | Nada agregando hits/misses do DNS proxy | Usuário não enxerga quanto foi bloqueado; precisa tabela `block_events` + UI |
| F9 | **Firefox não confia na CA do DopaBlocker** | Firefox usa NSS em vez do Windows Root store por padrão | A página HTTPS funciona em Chrome/Edge/Brave, mas Firefox pode mostrar erro de certificado. Fix futuro: instalar a CA nos perfis NSS detectados |

---

## 🟡 Hardening — produção

### Backend
| # | Gap | Onde |
|---|-----|------|
| H1 | CORS `permissive` — qualquer origem aceita | [backend/src/main.rs:121](../backend/src/main.rs#L121) |
| H2 | Sem rate limiting em `/auth/register` e `/auth/login` | [backend/src/routes/auth.rs](../backend/src/routes/auth.rs) |
| H3 | Sem validação de força de senha server-side | Só Firebase faz min=6 client-side |
| H4 | Sem rotação automática de device tokens | Token emitido nunca expira — só revogação manual via DB |
| H5 | `FIREBASE_API_KEY` em `backend/.env` é linha morta (não lida) | Confunde manutenção |

### Desktop
| # | Gap | Onde |
|---|-----|------|
| H6 | `parse_type` no DB local mapeia valores desconhecidos para `Domain` silenciosamente | [desktop/src-tauri/src/db.rs](../desktop/src-tauri/src/db.rs) — mascara corrupção de schema |
| H7 | API client sem retry além do 401 single-shot | [desktop/src/lib/services/api.ts](../desktop/src/lib/services/api.ts) — 5xx/network errors caem |
| H8 | Refresh-token fail do Firebase não faz auto-logout | [desktop/src/lib/stores/auth.ts](../desktop/src/lib/stores/auth.ts) — user fica em limbo |
| H9 | Engine não detecta crash da task do DNS proxy | [blocking/engine.rs](../desktop/src-tauri/src/blocking/engine.rs) — `is_running()` pode reportar true com task morta |
| H10 | Sem monitor de "DNS do sistema foi trocado fora do app" | Usuário muda DNS em rede → proxy fica órfão sem detectar |

### Segurança
| # | Gap | Nota |
|---|-----|------|
| H11 | Sem 2FA no Firebase Auth | Fluxo opcional no próprio Firebase |
| H12 | SQLCipher key e chave privada da CA ficam acessíveis a qualquer processo rodando como o mesmo usuário | Windows Credential Manager/app data são user-scoped; malware no user teria acesso. Mitigação: DPAPI com flags de process-scoped + ACL mais restrita |
| H13 | Sem assinatura de binário Windows (code signing) | Defender vai marcar "Unknown publisher" e usuário precisa clicar "Run anyway" |
| H14 | Sem proteção contra DLL hijacking / side-loading | Binário não valida DLLs carregadas — relevante se distribuir sem installer |
| H15 | Domínios com pinning embutido no Chromium ainda podem mostrar erro de certificado | Algumas propriedades Google/Facebook recusam CAs locais mesmo instaladas; limitação inerente do navegador |

---

## 🟢 Plataforma — fora de escopo v0.1

| # | Gap | Nota |
|---|-----|------|
| P1 | **macOS / iOS** | O plano original excluiu explicitamente. Porta usável do DNS proxy existe; precisa WFP-equivalent (NEFilterDataProvider) |
| P2 | **Linux** | Sem WFP, blocklist rodaria só no DNS proxy. Precisa systemd-resolved integration + nftables/iptables pra equivalente kernel-level |
| P3 | **Mobile (Flutter)** | Scaffold vazio em [mobile/](../mobile/). v0.2 vai incluir Android VPN Service + Accessibility Service |
| P4 | **Modo Parental cross-device** | Depende de Firestore sync (F2). v0.2 |
| P5 | **Desbloqueio por tarefas / timer** | Plano original menciona "sistema de tasks/checklist pra destravar" — fora do protótipo |
| P6 | **Horários programados** | "Bloquear entre 9h-18h" — fora do protótipo |

---

## 🔵 Polimento UX (etapa 10 do plano original)

| # | Gap |
|---|-----|
| U1 | `tauri.conf.json` com `identifier: "com.tauri.dev"` — não é único; mudar pra `app.dopablocker.desktop` antes do bundle |
| U2 | `requireAdministrator: true` no `bundle.windows` ainda não setado — em produção o app vai perder funcionalidade sem admin |
| U3 | Ícones da janela/tray são os defaults do Tauri scaffold (`icons/`) — precisa identidade visual |
| U4 | Primeira execução não explica por que UAC aparece nem o que o app vai fazer com DNS/WFP |
| U5 | Erros mostrados como banner inline — sem sistema de toast unificado |
| U6 | Sem indicador visual enquanto o Bloom Filter está sendo baixado/populado (primeiro boot leva 5–15s) |
| U7 | Acessibilidade não auditada (contraste, ARIA, navegação por teclado além do óbvio) |
| U8 | Shortcut global pra ligar/desligar bloqueio — ausente |
| U9 | Tray icon + menu "quick toggle" — ausente |
| U10 | Logout no `/settings` não pede confirmação — um click acidental desloga |

---

## ⚫ DevOps / Deploy

| # | Gap |
|---|-----|
| D1 | Sem Dockerfile pro backend — planejado pra v0.2 |
| D2 | Sem CI/CD (GitHub Actions): build, test, release |
| D3 | Sem auto-updater no Tauri (plugin `tauri-plugin-updater` não configurado) |
| D4 | Sem crash reporter (Sentry, Bugsnag) — erros só no console do dev |
| D5 | Sem métricas opt-in de uso |
| D6 | Sem logs persistidos em arquivo — `tracing` escreve só stdout |
| D7 | Sem SBOM / audit trail de dependências (`cargo audit`, `pnpm audit` não rodam em CI) |
| D8 | Backend sem endpoint `/healthz` estruturado (só `/health` text "OK") |

---

## 🔍 Observability & Testing

| # | Gap |
|---|-----|
| O1 | Zero testes E2E (Playwright, Tauri Integration Tests) — só unitários |
| O2 | Sem métricas expostas do DNS proxy (queries/s, cache hit rate, latência p50/p99) |
| O3 | Sem teste de carga — não sabemos como o proxy se comporta com 1k queries/s |
| O4 | Sem fuzz do parser DNS — hickory é maduro, mas nossa camada entre bytes e `Message::from_vec` pode ter bugs em pacotes malformados |
| O5 | Sem teste de integração do fluxo completo (backend + desktop + DB real) |
| O6 | Parser do `netsh interface ipv4 show dnsservers` tem teste só pra EN e PT-BR — outros idiomas não cobertos |

---

## 📝 Histórico de cortes que FORAM fechados

Para registro, antes da etapa 7 foram implementados (saíram da lista de gaps):

- ✅ **DNS cache com TTL** respeitando menor TTL da resposta
- ✅ **Normalização server-side** da blocklist usando `normalize_domain` do shared
- ✅ **Persistência do engine state no boot** + crash recovery do DNS do sistema
- ✅ **TCP DNS** (UDP+TCP listeners)
- ✅ **Failover de upstream** (DoH Cloudflare → DoH Google → UDP 1.1.1.1 → UDP 8.8.8.8)
- ✅ **DoH upstream** (HTTPS pra resolver queries permitidas)
- ✅ **Block page HTTPS** (parcial) — CA local instalada no Windows Root + certificados leaf dinâmicos por SNI; Firefox/NSS e pinning Chromium seguem documentados como gaps

---

## Uso deste documento

- **Antes de fechar v0.1**: revisar 🔴 Críticos. C1/C2/C3 juntos (IPv6 end-to-end) provavelmente justificam uma v0.1.1 dedicada.
- **Para planejar v0.2**: 🟠 Funcionais + 🟢 Plataforma (Parental + Mobile).
- **Antes do primeiro release público**: 🔵 Polimento + 🟡 Hardening H1, H2, H13 (CORS, rate limit, code signing).
- **Mantém vivo**: quando um gap for fechado, mover para a seção histórica no rodapé com link pro PR/commit.
