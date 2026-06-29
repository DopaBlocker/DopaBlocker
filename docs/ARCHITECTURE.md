# DopaBlocker — Arquitetura

> **Escopo deste documento:** descreve **apenas o que está implementado hoje**. Propostas,
> alternativas e roadmap ficam em [DECISOES_E_ROADMAP.md](DECISOES_E_ROADMAP.md). Referência de
> endpoints em [API.md](API.md); como rodar/testar em [RUNBOOK.md](RUNBOOK.md).

## Visão geral

DopaBlocker é um bloqueador de distrações para **Windows** (desktop) e **Android** (mobile), com
dois modos na mesma conta: **pessoal** (o usuário controla os próprios bloqueios) e **controle
parental** (o pai gerencia a blocklist dos dispositivos filhos).

Monorepo com 4 sub-projetos:

| Pasta | Stack | Estado |
|---|---|---|
| `shared/` | Crate Rust (modelos, bloom filter, domain matcher, regra parental) | Funcional, testado |
| `backend/` | API REST Rust/Axum + SQLCipher | Funcional |
| `desktop/` | Tauri 2 (Rust) + SvelteKit + Tailwind v4 | Funcional |
| `mobile/` | Flutter + Kotlin | Funcional: **sites (DNS)**, **apps** (overlay + seletor visual) e **conteúdo adulto** (Cloudflare for Families); pendente: cache SQLCipher Dart |

`shared`, `backend` e `desktop/src-tauri` formam **um único workspace Cargo**.

## Fluxo de dados

```
[Desktop (Tauri)]  --HTTP: Firebase JWT ou dt_-->  [Backend (Axum)] --SQLCipher--> dopablocker.db
[Mobile (Flutter)] --HTTP: Firebase JWT ou dt_-->  [Backend (Axum)]
[Firebase Auth]    --JWT validado via JWKS------->  [Backend (Axum)]
```

O **backend (SQLCipher) é a fonte de verdade** dos dados de negócio. O Firebase é usado **só para
autenticação** das contas Pessoal/Pais — não é banco da blocklist e não há sincronização via
Firestore. Cada cliente mantém um **cache local SQLCipher** do qual o engine de bloqueio lê.

## Onboarding (3 opções → 2 modos)

A tela inicial oferece **Pessoal**, **Pais** e **Filhos**, mas o banco só tem dois modos:

| Opção UI | `User.mode` | Cria conta Firebase? | Cria User local? | Credencial |
|---|---|---|---|---|
| Pessoal | `personal` | Sim | Sim | Firebase JWT |
| Pais | `parental` | Sim | Sim | Firebase JWT |
| Filhos | — | **Não** | **Não** | Device Token (`dt_…`) |

No fluxo Filhos não há cadastro: o usuário digita um **código de 6 dígitos** gerado pelo pai; o
backend cria um `Device` sob o `user_id` do pai com `is_child=true` e emite um Device Token.

O `mode` (`personal`/`parental`) **pode ser trocado depois** nas Configurações via `PUT /auth/me`,
**sem recriar a conta** (ver [API.md](API.md)). A troca não mexe em devices/blocklist/vínculos — só
muda como a regra "pai imune" é computada no próximo sync.

**Criação de conta resiliente à identidade.** A conta vive em dois sistemas não transacionais
(Firebase = identidade; backend = registro). Por isso `POST /auth/register` é **idempotente** e faz
**reclaim**: se já existe conta para o `firebase_uid`, retorna-a; se existe uma conta órfã com o
mesmo **email** (presa a um UID antigo), **reassocia-a** ao UID atual (provando posse do email pelo
mesmo mecanismo do cadastro). Isso elimina o antigo beco sem saída em que o `email UNIQUE` prendia o
recadastro. Detalhes em [API.md](API.md).

## Autenticação dual

O cluster de auth ([backend/src/core/auth/](../backend/src/core/auth/) — `middleware.rs` +
`jwt.rs` + `jwks.rs` + `device_token.rs`) inspeciona o prefixo do header `Authorization: Bearer`:

- **Firebase JWT** (sem prefixo) — contas Pessoal/Pais. Valida assinatura via **JWKS do Google**
  (cacheado), checa `iss`/`aud`/`exp`, resolve `user_id` por `firebase_uid`.
- **Device Token** (`dt_…`) — devices filhos. Gerado uma vez no link/confirm, salvo como **hash
  SHA-256** em `device_tokens`. Escopo **read-only**: qualquer método ≠ `GET`/`HEAD` é rejeitado
  com **403 no middleware** (por método HTTP, não por handler — toda rota nova herda a regra).

Ambos resolvem para `AuthUser { user_id, source, device_id }`.

### Máquina de estados de auth (contrato cross-platform)

Desktop e mobile implementam a **mesma** máquina de estados. Referência:
[desktop/src/lib/stores/auth.ts](../desktop/src/lib/stores/auth.ts); o mobile a espelha em Riverpod
([mobile/lib/providers/auth_provider.dart](../mobile/lib/providers/auth_provider.dart)) — **incluindo
`AuthBackendUnavailable`**, então há paridade entre as plataformas.

Fases: `booting` → (`child_session` | `signed_out` | `authenticating`); `authenticating` →
(`authenticated` | `pending_local_registration` | `backend_unavailable` | `child_session` |
`signed_out`); `pending_local_registration` → `authenticated`; `backend_unavailable` →
`authenticating` (retry); `authenticated`/`child_session` → `signed_out` (logout/revogação).

`pending_local_registration` é **transitório** (Firebase ok, falta concluir/recuperar o registro
local): com o `register` idempotente + reclaim no backend, concluir o cadastro **cria ou recupera** a
conta — não é mais um beco sem saída. A UI mostra só "finalizar cadastro / tentar de novo".

Invariantes: `authenticated` ⇒ tem user+firebase e não tem child; `child_session` ⇒ tem child e
**nunca** convive com Firebase. O header `Authorization` é determinado exclusivamente pela fase
(via um `AuthProvider`: firebase / child / anônimo).

A sessão de filho é um **singleton** numa tabela `child_session` no SQLCipher local (um device não
pode estar vinculado a dois pais ao mesmo tempo).

## Modelo de dados e armazenamento

Modelos em [shared/src/models.rs](../shared/src/models.rs) (reusados por backend e desktop; o mobile
espelha em Dart): `User`, `Device`, `BlockedItem`, `ParentalLink`, `AdultFilterSettings`,
`DeviceToken`; enums `BlockMode`, `Platform`, `BlockedType` (`domain`/`app`/`keyword`), `LinkStatus`.

**SQLCipher** (SQLite + AES-256 via `PRAGMA key`, que é sempre o **primeiro** comando após abrir a
conexão) em toda parte:

- **Backend** (fonte de verdade) — `rusqlite` + `bundled-sqlcipher`. Migrations:
  `001_initial` (users, devices, blocked_items, parental_links, adult_filter_settings),
  `002_parental_fixes` (device_tokens + índices), `003_email_verification` (email_verifications),
  `004_device_events` (eventos de adulteração reportados pelos filhos — C2.1/C2.2).
  `UNIQUE(user_id, item_type, value)` garante idempotência da blocklist.
- **Desktop** (cache local) — migrations próprias: `001_local_cache` (blocked_items_cache,
  blocking_state) e `002_child_session`. A chave do SQLCipher é gerada no primeiro boot e guardada
  no **Windows Credential Manager** (não embarcada no binário).
- **Mobile** — cache SQLCipher Dart (`sqflite_sqlcipher`) ainda **não** implementado; hoje o estado
  nativo de bloqueio é persistido em `SharedPreferences` (ver engine mobile).

## Sincronização

Backend como intermediário via REST; cache local em cada device. Além do fetch em
load/mutação de telas, **todas as sessões** mantêm o cache/engine em dia por **polling periódico**
de `GET /blocklist` (~30–45s): o **device-filho** (recebe edições do pai) e o **modo pessoal/pai**
(recebe mudanças feitas em outro device da mesma conta). É isso que faz, p.ex., um site bloqueado
no celular pessoal aparecer no PC pessoal sem reiniciar o app. O poll detecta revogação (401 →
logout **apenas no filho**) **e aplica** a blocklist atualizada ao cache/engine local — no desktop
via `blockingStore.startAutoSync`/`tauri-bridge.saveBlocklist` (ligado no `+layout.svelte`; o filho
tem o poll próprio em `/child-blocked`), no mobile via `_startPollIfNeeded`/`_syncNative`. Para
baratear o poll, `GET /blocklist` suporta **ETag/`If-None-Match`** → `304 Not Modified` quando
nada mudou. **Não há** listeners realtime/Firestore.

> **Nota:** o **desktop não bloqueia apps** (o engine só carrega domínios); itens `app` existem na
> blocklist mas só são aplicados no mobile. Logo, na sync Mobile→PC do modo pessoal só **sites**
> se propagam de fato.

## Engine de bloqueio — Desktop (Windows)

Orquestrado por [engine.rs](../desktop/src-tauri/src/blocking/engine.rs). **Ordem de start importa**
(fecha a janela de bypass): **(1) WFP** → (2) CA local → (3) block page HTTP `:80` → (4) block page
HTTPS `:443` → (5) **DNS proxy `:53`**. Stop é na ordem inversa.

- **DNS proxy** ([dns_proxy.rs](../desktop/src-tauri/src/blocking/dns_proxy.rs)) — escuta em
  `127.0.0.1:53` **e `::1:53`** (IPv4 **e IPv6**), UDP **e** TCP. Domínio bloqueado → responde
  **`A = 127.0.0.1` (TTL 5s)**, **`AAAA` vazio (NoError)**, demais tipos **NXDOMAIN**; caso
  contrário consulta cache e encaminha ao upstream. Há **cache de respostas** com TTL
  ([dns_cache.rs](../desktop/src-tauri/src/blocking/dns_cache.rs)).
- **WFP (anti-bypass, kernel)** ([wfp.rs](../desktop/src-tauri/src/blocking/wfp.rs)) — instalado
  **antes** do proxy. Redireciona/bloqueia DNS direto (UDP/TCP 53 exceto loopback), **DoT** (TCP
  853), **DoH/DoQ** (TCP **e** UDP 443 para uma lista curada de IPs em `shared/data/doh-ipv4.txt` e
  `doh-ipv6.txt`), espelhado em **IPv4 e IPv6** (`FWPM_LAYER_ALE_AUTH_CONNECT_V4`/`_V6`). O próprio
  app é auto-excluído por app_id.
- **Bloqueio de FQDN de DoH** ([block_reason.rs](../desktop/src-tauri/src/blocking/block_reason.rs))
  — variant `BlockReason::DohEndpoint` checa `shared/data/doh-fqdns.txt` **antes** da lista do
  usuário (resolver o FQDN do provedor DoH cai no próprio proxy e é bloqueado).
- **Block page** ([block_page.rs](../desktop/src-tauri/src/blocking/block_page.rs) + CA local em
  [ca.rs](../desktop/src-tauri/src/blocking/ca.rs)) — CA auto-assinada instalada no Windows Root
  Store; certs por hostname gerados on-demand via SNI ([tls_resolver.rs](../desktop/src-tauri/src/blocking/tls_resolver.rs)).
- **Filtro de conteúdo adulto** ([adult_filter.rs](../desktop/src-tauri/src/blocking/adult_filter.rs))
  — **Bloom filter** (crate `shared`) populado da lista Steven Black (`alternates/porn/hosts`,
  ~100k domínios), com cache binário (bincode) revalidado a cada 7 dias; construído em background no
  boot.
- **Restauração de DNS é crítica** ([lib.rs](../desktop/src-tauri/src/lib.rs)) — panic hook,
  `SetConsoleCtrlHandler`, `RunEvent::ExitRequested` e self-heal de DNS órfão no boot restauram o
  DNS do sistema em qualquer saída (crash, logoff, shutdown). Logs com rotação diária via
  `tracing-appender`.

## Engine de bloqueio — Mobile (Android)

- **Bloqueio de sites por DNS — IMPLEMENTADO.** `VpnService` com TUN
  ([DnsVpnService.kt](../mobile/android/app/src/main/kotlin/com/dopablocker/dopablocker_mobile/vpn/DnsVpnService.kt))
  roteando **apenas o DNS virtual** (`addRoute("10.0.0.1", 32)` — sinkhole DNS-only; o resto do
  tráfego segue normal). O `packetLoop` lê pacotes, faz parse de IPv4/UDP/DNS
  ([DnsPacket.kt](../mobile/android/app/src/main/kotlin/com/dopablocker/dopablocker_mobile/vpn/DnsPacket.kt)),
  decide via [DomainMatcher.kt](../mobile/android/app/src/main/kotlin/com/dopablocker/dopablocker_mobile/vpn/DomainMatcher.kt)
  (mesma semântica de `shared/src/domain_matcher.rs`) e responde **`127.0.0.1`/AAAA vazio/NXDOMAIN**
  (igual ao desktop) ou encaminha ao upstream via socket **`protect()`**-ado
  ([DnsForwarder.kt](../mobile/android/app/src/main/kotlin/com/dopablocker/dopablocker_mobile/vpn/DnsForwarder.kt),
  falha → SERVFAIL). A blocklist e a flag `blocking_active` são persistidas em `SharedPreferences`
  (`dopablocker_prefs`); `startVpn()` as recarrega no boot/restart; `onRevoke()` limpa o estado.
  Cobertura: testes JVM (matcher + parser) e instrumentados no emulador (persistência + E2E de
  bloqueio).
- **Bloqueio de apps — IMPLEMENTADO (C3).** O caminho de dados foi fechado: `_syncNative` envia
  os itens `app` → método de canal `updateBlockedApps` →
  [AppBlockerService.kt](../mobile/android/app/src/main/kotlin/com/dopablocker/dopablocker_mobile/accessibility/AppBlockerService.kt)
  (persiste em `SharedPreferences`, recarrega em `onServiceConnected`). Ao abrir um app bloqueado,
  o serviço lança a **`BlockOverlayActivity`** (overlay full-screen — técnica Cold Turkey/AppBlock),
  em vez do antigo "trazer pra frente" (contornável). O **seletor visual de apps** na UI Flutter
  (`AppPickerSheet`) lista os apps instalados via `InstalledAppsProvider` (canal `getInstalledApps`,
  `<queries>` MAIN/LAUNCHER no Manifest), com fallback de digitação manual do package name.
- **Filtro de conteúdo adulto — IMPLEMENTADO via resolver filtrado (C4).** Com o toggle ligado, o
  [DnsForwarder.kt](../mobile/android/app/src/main/kotlin/com/dopablocker/dopablocker_mobile/vpn/DnsForwarder.kt)
  troca o upstream por **Cloudflare for Families** (`1.1.1.3`/`1.0.0.3`, malware + adulto), sem
  manter ~100k domínios no device. A flag é persistida e aplicada no boot/restart.
- **Detector de adulteração (C2.1/C2.2).** Quando o filho desliga a VPN (`onRevoke`) ou abre as
  Configs de VPN/DNS (heurística em `SettingsTamperDetector`), o `TamperReporter` faz
  `POST /devices/tamper` ao backend e o pai vê no painel de **Alertas**. Sem root é dissuasão
  observável, não cofre (ver [DECISOES_E_ROADMAP.md](DECISOES_E_ROADMAP.md)).
- **Boot** — [BootReceiver.kt](../mobile/android/app/src/main/kotlin/com/dopablocker/dopablocker_mobile/receivers/BootReceiver.kt)
  relê a flag persistida e religa a VPN.
- **Ativação obrigatória no filho.** A tela do filho
  ([child_blocked_screen.dart](../mobile/lib/screens/child_blocked_screen.dart)) é um **muro de
  configuração**: enquanto faltar consentimento de VPN, acessibilidade ou overlay (rastreados em
  [permissions_provider.dart](../mobile/lib/providers/permissions_provider.dart), com o passo de VPN
  via `isVpnPrepared`), ela guia o filho passo a passo e **não** mostra "proteção ativa" enganosa.
  Concedidas as permissões, o engine **sobe sozinho** (`ensureEngineRunning`: VPN + sync da lista do
  pai) e reaplica no `resumed`. Sem isto, o dispositivo do filho exibia "bloqueio ativo" mas nunca
  iniciava a VPN — agora o bloqueio definido pelo pai realmente vale.

## Regra do "pai imune"

Função pura testada em [shared/src/parental.rs](../shared/src/parental.rs):

```
personal              → aplica tudo
parental + is_child   → aplica tudo
parental + !is_child  → lista vazia (pai imune)
```

- **Desktop:** o frontend deriva `ParentalContext { mode, is_child }` e envia em toda chamada Tauri
  que afeta o engine; o Rust decide via `effective_strategy`
  ([commands.rs::effective_rules](../desktop/src-tauri/src/commands.rs)).
- **Mobile:** o `_syncNative()` do `blocking_provider.dart` aplica a mesma regra — no device do pai
  em modo parental envia lista vazia ao nativo.

A UI sempre mostra a lista cheia (o pai precisa **gerenciar**); quem decide aplicar ou não é o engine.

## Controle parental

Uma conta, múltiplos devices. Vinculação por **código de 6 dígitos (TTL 5 min)**; um índice único
parcial (`WHERE status='pending'`) evita colisão. O filho não cria conta — usa o Device Token gerado
no confirm. Blocklist **única por conta** (compartilhada por todos os filhos). O pai revoga pela UI
(`POST /devices/{id}/revoke`).

## Limites por plataforma (resumo)

O anti-bypass do **desktop** é forte (WFP fecha DNS direto, DoH/DoT/DoQ por IP+FQDN, IPv4+IPv6); o
**mobile sem root** é intrinsecamente mais fraco (o usuário pode desligar a VPN, usar DoH no Chrome,
trocar o DNS). O modelo de ameaça por plataforma e as alternativas estão detalhados em
[DECISOES_E_ROADMAP.md](DECISOES_E_ROADMAP.md).
