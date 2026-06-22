# DopaBlocker — Runbook (rodar, testar, validar)

> Roteiro operacional do que existe hoje. Setup completo do ambiente (Rust, OpenSSL/vcpkg, Node,
> Flutter, Android SDK, Firebase) está no [README.md](../README.md). Arquitetura em
> [ARCHITECTURE.md](ARCHITECTURE.md); limitações conhecidas no fim deste doc.

## Pré-requisitos (resumo)

- **Windows 10/11 x64**; Rust + Tauri + C++ Build Tools + WebView2; **OpenSSL via vcpkg**
  (`bundled-sqlcipher` linka contra ele — sem isso o `cargo build` falha com `OPENSSL_DIR`).
- Node + pnpm (frontend desktop); Flutter + Android SDK (mobile).
- `.env` em `backend/` (e raiz). Para validar o engine desktop (porta 53, WFP, `netsh`, CA), rode
  o terminal/app **como Administrador**.
- O build Android exige `mobile/android/app/google-services.json` (do Firebase Console; está no
  `.gitignore`). Sem ele, qualquer `gradlew` falha.

## Como rodar

| Alvo | Comando |
|---|---|
| Backend (porta 3000) | `cd backend && cargo run` |
| Desktop (dev: sobe backend + Vite) | `pnpm tauri:dev` (na raiz) |
| Desktop (build) | `pnpm tauri:build` |
| Mobile | `cd mobile && flutter run` |
| Backend via Docker | `cd infra && docker compose up --build` *(Dockerfile é placeholder — ver roadmap)* |

## Verificações automatizadas

| O quê | Comando |
|---|---|
| Rust (shared + backend + desktop) | `cargo test` (na raiz) |
| Type-check do frontend | `cd desktop && pnpm check` |
| Análise estática mobile | `cd mobile && flutter analyze` |
| Testes unitários nativos do mobile (JVM) | `cd mobile/android && ./gradlew :app:testDebugUnitTest` |
| Testes instrumentados do mobile (emulador/device) | `cd mobile/android && ./gradlew :app:connectedDebugAndroidTest` |
| CI (GitHub Actions) | `.github/workflows/ci.yml` roda os três acima (Rust shared+backend, `pnpm check`, `flutter analyze`) em cada push/PR |

Cobertura atual relevante: a crate `shared`, o backend e o desktop têm testes `#[test]`/`#[tokio::test]`
inline. O engine DNS do mobile tem testes JVM (matcher + parser de pacotes DNS) e instrumentados
(persistência da blocklist + E2E: domínio bloqueado resolve `127.0.0.1`, permitido resolve normal).
**WFP não é coberto por testes** (interação direta com o kernel) — exige validação manual.

## Golden path — Desktop (manual, como Admin)

1. **Backend sobe**: migrations `001/002/003` aplicadas; `GET /health` → `OK`.
2. **Desktop abre** em `/welcome` com 3 cards (Pessoal/Pais/Filhos).
3. **Cadastro Pessoal**: envia código → digita código → cria conta Firebase → `auth/register` →
   dashboard. Código inválido não cria user.
4. **Login/Logout**: logout pede confirmação e volta a `/welcome`.
5. **Pais + vinculação**: em `/parental`, "Gerar código" registra o device titular e chama
   `devices/link/generate`; em `/onboarding/child` o código válido cria a `child_session` e cai em
   `/child-blocked`.
6. **Blocklist**: `https://www.Instagram.com/reels` normaliza para `instagram.com`; domínio sem ponto
   → 400; duplicata → 409.
7. **Ativar bloqueio** (Admin): DNS do sistema aponta para loopback; `nslookup instagram.com` →
   `127.0.0.1`; `nslookup google.com` resolve real; `netsh wfp show state` lista filtros DopaBlocker.
8. **Block page/CA**: `certutil -store Root | findstr DopaBlocker` acha a CA; `https://instagram.com`
   mostra a página local (Firefox pode dar erro de certificado por usar NSS).
9. **Filtro adulto**: toggle liga; mostra "Construindo…"; depois bloqueia domínio adulto conhecido.
10. **Hot reload / crash**: adicionar/remover sem reiniciar o engine; após kill, reabrir restaura o
    DNS órfão / reativa o engine.
11. **Pai imune**: pai em modo parental com bloqueio ativo → `nslookup instagram.com` resolve normal.
12. **Exclusão de conta**: `/settings` → confirmação por texto → apaga Firebase + backend → `/welcome`.
13. **Trocar de modo**: `/settings` → "Mudar para Pais/Pessoal" (`PUT /auth/me`); o badge atualiza sem
    recriar a conta. Saindo do parental com filhos vinculados, a UI confirma antes.
14. **Recuperar conta órfã**: apague a linha do user no backend (ou simule um `firebase_uid` novo) e
    entre de novo com o mesmo email → o `register` **recupera** a conta (reclaim), sem o antigo erro
    "email já tem conta / use outro email".

## Golden path — Mobile (DNS)

Verificável via teste instrumentado (`DnsBlockingInstrumentedTest`) ou manualmente: com a VPN ativa
e `instagram.com` na lista, `nslookup instagram.com` no device → `127.0.0.1`; `example.com` → IP
real. Desligar a VPN nas Configurações dispara `onRevoke`, zera a flag **e** reporta
`vpn_revoked` ao backend (no device do filho) — o pai vê no painel de Alertas.

- **Apps (C3):** adicionar um item `app` (pacote, ex.: `com.instagram.android`); com o
  AppBlockerService ativo, abrir o app dispara a `BlockOverlayActivity` full-screen.
- **Filtro adulto (C4):** com o toggle ligado, o upstream vira Cloudflare for Families
  (`1.1.1.3`); um domínio adulto conhecido passa a falhar/ser filtrado.
- **Tamper (C2.2):** abrir Configs de VPN/DNS traz o app à frente e reporta o evento.
- **Filho — ativação obrigatória:** ao vincular o celular do filho (pai gera o código no desktop), a
  tela do filho exige **VPN → acessibilidade → overlay** passo a passo; concedidas, o engine sobe
  sozinho. Pai adiciona um site no desktop → em ~45s o filho bloqueia e o overlay aparece.

## Smoke tests obrigatórios (ao mexer em auth/bloqueio)

Conta Pessoal completa (criar → item → bloquear → confirmar → pausar); conta Pais (gerar código →
vincular filho sem Firebase); pai adiciona item e o filho recebe; **pai imune**; revogar filho derruba
o Device Token; reabrir desktop/mobile restaura sessão+lista+estado.

## Limitações conhecidas (aceitas para o protótipo)

- **Anti-bypass desktop:** resolvers DoH self-hosted (IP+FQDN próprios), VPNs com DNS embarcado e
  DNS-over-Tor podem escapar (inerente a soluções sem driver kernel-mode). Cert pinning de alguns
  apps contorna a block page.
- **Anti-bypass mobile:** sem root, o usuário pode desligar a VPN, usar DoH no Chrome ou trocar o
  DNS. DNS-sinkhole only. Mitigação: o filho **reporta** desligar a VPN / abrir Configs de VPN/DNS
  ao backend, e o pai vê (C2.1/C2.2) — dissuasão observável, não cofre.
- **Mobile pendente:** **cache SQLCipher Dart** ausente (estado nativo ainda em
  `SharedPreferences`) e **fila de escrita offline**. Bloqueio de sites/apps/adulto e o **seletor
  visual de apps** (`AppPickerSheet`) já funcionam.
- **Produção:** Dockerfile e hospedagem (deploy) ainda são roadmap; auto-updater/assinatura de
  binário pendentes. Já implementados: **CI** (`.github/workflows/ci.yml`), **`/healthz`**,
  **rate-limit + CORS por allowlist** nas rotas públicas. Detalhes em
  [DECISOES_E_ROADMAP.md](DECISOES_E_ROADMAP.md).
- **Firefox** não confia na CA local automaticamente (usa NSS).
