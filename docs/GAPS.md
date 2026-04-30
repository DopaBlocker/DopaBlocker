# DopaBlocker — Gaps do Protótipo Atual / Rumo v0.2

Inventário vivo do que ainda não bate com a meta do [PROTOTYPE.md](PROTOTYPE.md).
Este documento reflete o código atual: backend/shared/desktop estão funcionais
em várias partes, o mobile ainda é esqueleto, e Docker/produção seguem pendentes.

---

## Críticos — eficácia do bloqueio

| # | Gap | Onde | Impacto | Saída sugerida |
|---|-----|------|---------|----------------|
| C1 | **WFP ainda só instala filtros IPv4** | [blocking/wfp.rs](../desktop/src-tauri/src/blocking/wfp.rs) usa `FWPM_LAYER_ALE_AUTH_CONNECT_V4` | DNS/DoH via IPv6 pode escapar dos filtros kernel-level | Adicionar filtros `_V6`, loopback `::1` e IPs DoH IPv6 |
| C2 | **DoH sem SNI inspection** | [blocking/wfp.rs](../desktop/src-tauri/src/blocking/wfp.rs) bloqueia DoH só por IP estático | Resolvers com IP rotativo/self-hosted podem escapar | Callout driver kernel-mode ou outra estratégia de inspeção TLS |
| C3 | **Sem DoQ explícito** | UDP/443 não é filtrado especificamente | DNS-over-QUIC para IP não conhecido pode passar | Adicionar regra UDP/443 para resolvers conhecidos e avaliar falso positivo |

---

## Funcionais — completude do produto

| # | Gap | Onde | Impacto |
|---|-----|------|---------|
| F1 | **Mobile ainda não implementado** | [mobile/](../mobile/) contém placeholders Dart/Kotlin | A meta v0.2 exige Android funcional, mas Firebase, providers, SQLCipher, VPN e Accessibility ainda não existem de verdade |
| F2 | **Sync cross-device periódico incompleto** | [blocking.ts](../desktop/src/lib/stores/blocking.ts), [child-blocked](../desktop/src/routes/child-blocked/+page.svelte) | O desktop sincroniza em load/mutação; não há polling periódico completo da blocklist em todos os estados |
| F3 | **Modo Filhos desktop precisa de golden path de bloqueio** | [child-blocked/+page.svelte](../desktop/src/routes/child-blocked/+page.svelte), [blocking.ts](../desktop/src/lib/stores/blocking.ts) | A sessão `child_session` existe, mas falta validar/documentar o ciclo completo: restaurar sessão, carregar regras, ligar engine e atualizar regras do pai |
| F4 | **Blocklist única por conta** | Schema global por `user_id` | Decisão aceita para v0.2; regras diferentes por filho ficam para depois |
| F5 | **Refresh adulto só ocorre no build/boot do filtro** | [adult_filter.rs](../desktop/src-tauri/src/blocking/adult_filter.rs) | App aberto por muitos dias pode manter lista antiga até reiniciar/reconstruir |
| F6 | **Sem múltiplas listas de conteúdo adulto** | Apenas Steven Black `alternates/porn/hosts` | Cobertura menor que uma combinação Steven Black + OISD/AdGuard |
| F7 | **Sem fila offline de writes** | [blocking.ts](../desktop/src/lib/stores/blocking.ts) | Se backend estiver offline, adicionar/remover item falha em vez de reconciliar depois |
| F8 | **Sem painel de estatísticas** | Sem tabela `block_events`/telemetria local | Usuário não vê histórico, contagem de bloqueios ou cache hit rate |
| F9 | **Firefox não confia automaticamente na CA local** | Firefox usa NSS, não Windows Root Store | Página HTTPS pode exibir erro de certificado no Firefox |

---

## Hardening — produção

### Backend

| # | Gap | Onde |
|---|-----|------|
| H1 | CORS `permissive` — qualquer origem aceita | [backend/src/main.rs](../backend/src/main.rs) |
| H2 | Sem rate limiting em auth/email-code/login | [backend/src/routes/auth.rs](../backend/src/routes/auth.rs), [auth_service.rs](../backend/src/services/auth_service.rs) |
| H3 | Sem validação forte de senha server-side | Firebase impõe mínimo; backend não aplica política própria |
| H4 | Sem rotação automática de Device Tokens | Token vale até revogação manual |
| H5 | `FIREBASE_API_KEY` em `.env` não é lida pelo backend | Confunde manutenção se estiver presente |

### Desktop

| # | Gap | Onde |
|---|-----|------|
| H6 | `parse_type` local mapeia valor desconhecido para `Domain` | [desktop/src-tauri/src/db.rs](../desktop/src-tauri/src/db.rs) |
| H7 | API client só faz retry automático em 401 | [desktop/src/lib/services/api.ts](../desktop/src/lib/services/api.ts) |
| H8 | Engine não detecta crash interno da task do DNS proxy | [blocking/engine.rs](../desktop/src-tauri/src/blocking/engine.rs) |
| H9 | Sem monitor para DNS alterado fora do app enquanto engine roda | Usuário pode mexer em DNS manualmente e deixar proxy órfão |

### Segurança

| # | Gap | Nota |
|---|-----|------|
| H10 | Sem 2FA no Firebase Auth | Opcional via configuração Firebase |
| H11 | SQLCipher key e chave privada da CA são user-scoped | Processo malicioso no mesmo usuário pode tentar acesso |
| H12 | Sem assinatura de binário Windows | Distribuição pública mostrará "Unknown publisher" |
| H13 | Sem proteção contra DLL side-loading | Relevante antes de distribuir installer |
| H14 | Cert pinning de alguns domínios pode contornar página local | Limitação inerente de interceptação HTTPS com CA local |

---

## Plataforma — fora da v0.2 imediata

| # | Gap | Nota |
|---|-----|------|
| P1 | macOS / iOS | Requer Network Extension/NEFilterDataProvider |
| P2 | Linux | Requer integração com systemd-resolved + nftables/iptables |
| P3 | Desbloqueio por tarefas/timer | Ideia de produto, fora do protótipo atual |
| P4 | Horários programados | Fora do protótipo atual |
| P5 | Blocklists por filho | Fora da v0.2, a menos que vire requisito explícito |

---

## Polimento UX

| # | Gap |
|---|-----|
| U1 | `requireAdministrator`/manifest UAC ainda não configurado no bundle Windows |
| U2 | Ícones ainda parecem os defaults do scaffold Tauri |
| U3 | Acessibilidade não auditada em contraste, ARIA e navegação por teclado |
| U4 | Shortcut global para ligar/desligar bloqueio ausente |
| U5 | Tray icon + menu de quick toggle ausentes |

---

## DevOps / Deploy

| # | Gap |
|---|-----|
| D1 | `backend/Dockerfile` existe, mas é placeholder comentado e não buildável |
| D2 | `infra/compose.yml` aponta para o Dockerfile, mas depende de D1 |
| D3 | Sem CI/CD (GitHub Actions): build, test, release |
| D4 | Sem auto-updater Tauri |
| D5 | Sem crash reporter |
| D6 | Sem métricas opt-in |
| D7 | Sem logs persistidos em arquivo |
| D8 | Sem SBOM/audit trail (`cargo audit`, `pnpm audit`) em CI |
| D9 | Backend só tem `/health` textual, sem `/healthz` estruturado |

---

## Observability & Testing

| # | Gap |
|---|-----|
| O1 | Zero testes E2E (Playwright/Tauri integration) |
| O2 | Sem métricas do DNS proxy (queries/s, cache hit rate, p50/p99) |
| O3 | Sem teste de carga do proxy |
| O4 | Sem fuzz do parser DNS |
| O5 | Sem teste de integração backend + desktop + DB real |
| O6 | Parser de `netsh` cobre EN/PT-BR e alguns casos IPv6, mas não outros idiomas |

---

## Fechado Desde a Lista v0.1

- DNS proxy agora escuta IPv4 e IPv6 (`127.0.0.1` e `::1`) em UDP/TCP.
- Troca/restauração de DNS do sistema agora cobre IPv4 e IPv6.
- Modo Pais/Filhos no desktop saiu do "Em breve": `/welcome`, `/login?mode=parental`, `/parental`, `/onboarding/child` e `/child-blocked` existem.
- Revogação de filhos pela UI existe em `ParentalDashboard.svelte`.
- Registro do device titular foi adicionado no desktop antes de gerar código parental.
- `tauri.conf.json` usa `identifier: "app.dopablocker.desktop"`.
- Sistema de toast existe.
- Onboarding inicial explica permissões de admin, DNS/WFP e SQLCipher.
- UI já mostra estado de build do filtro adulto.
- Logout no settings usa confirmação.
- Email/senha agora passa por verificação de código antes do cadastro local.

---

## Uso Deste Documento

- Para fechar a v0.2, priorize F1, F2 e F3 junto com os smoke tests de [PROTOTYPE.md](PROTOTYPE.md).
- Antes de release público, priorize C1, H1, H2, H12, U1 e D1/D3.
- Quando um gap for fechado, mova para "Fechado Desde a Lista v0.1" com referência aos arquivos alterados.
