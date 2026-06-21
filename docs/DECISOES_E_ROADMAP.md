# DopaBlocker — Decisões & Roadmap

> Revisão crítica das decisões de arquitetura e o roadmap de evolução. **Estado atual de fato** está
> em [ARCHITECTURE.md](ARCHITECTURE.md) — aqui ficam as **decisões revisadas e propostas**, cada uma
> rotulada para o doc não voltar a "mentir".
>
> **Status:** `[IMPLEMENTADO]` já no código · `[DECIDIDO]` escolha consciente, sem trabalho · `[PROPOSTO]` mudança ainda não feita.
> **Veredito:** **MANTER** · **TROCAR** · **ADICIONAR** · **ADIAR**.
> **Contexto da análise:** protótipo solo, pré-lançamento (breaking changes OK). Dimensões avaliadas:
> anti-bypass, segurança/privacidade, custo/velocidade, simplicidade/manutenção.

## Modelo de ameaça (em camadas, por plataforma)

O DopaBlocker atende três perfis, e o nível de proteção viável **depende da plataforma**:

1. **Autocontrole** (usuário não-adversarial, modo pessoal) — quer se ajudar; basta atrito.
2. **Filho leigo** (controle parental) — tenta o óbvio; precisa ser barrado.
3. **Adversário técnico** — sabe usar VPN/DoH/trocar DNS; mitigar **até onde a plataforma permite**.

| Plataforma | Autocontrole | Filho leigo | Adversário técnico |
|---|---|---|---|
| **Desktop (Windows)** | ✅ | ✅ | **Forte** — WFP fecha DNS direto, DoH/DoT/DoQ (IP+FQDN), IPv4+IPv6. Escapam só DoH self-hosted, VPN com DNS embarcado, DNS-over-Tor. |
| **Android (sem root)** | ✅ | ⚠️ parcial | **Fraco e inerente** — dá para desligar a VPN, usar DoH no Chrome, trocar o DNS. Sem driver/root não há como igualar o desktop. |

**Decisão honesta `[DECIDIDO]`:** no Android, mirar autocontrole + filho leigo, e contra o técnico
investir no que **move o ponteiro sem root** (detectar/avisar adulteração) em vez de prometer um
cofre. Documentar isso é parte do produto.

## A. Auth & contas

### A1. Firebase Auth + backend Rust + SQLCipher simultâneos — **TROCAR** `[PROPOSTO]`
São **três** sistemas de identidade/dado. O Firebase aqui paga pouco e cobra caro: o backend já
valida JWT (JWKS) **e já reimplementou** verificação de email por código (HMAC, TTL, cooldown,
rate por hora, comparação constante) — ou seja, o que o Firebase faria de graça já está feito à mão.
O Firebase só entrega, hoje, **login Google + storage de senha**, ao custo de dependência de
privacidade do Google, SDK pesado (Flutter + Svelte) e a confusão JWKS↔HMAC. `FIREBASE_API_KEY` no
`.env` nem é lida (gap morto).

- **Recomendado:** consolidar auth self-hosted no Axum — `argon2` para senha, JWT próprio (HS256 com
  `jsonwebtoken`, já em uso) e "Login com Google" via OAuth code flow direto. Reaproveita o canal de
  email por código (vira reset de senha também).
- **Trade-offs:** você passa a guardar senhas (argon2 mitiga); OAuth manual ~1 dia a mais que o SDK;
  **quebra logins existentes** — de graça agora (sem usuários), caríssimo depois. **Fazer antes de
  ter usuários.**
- **Esforço médio · Impacto alto · Risco médio.**
- **Descartado:** migrar tudo para um BaaS único (ex.: Supabase) — exigiria mexer no engine que lê do
  SQLCipher local e jogar fora um backend Rust testado (a parte boa); troca a dependência do Google
  pela de outro fornecedor. Pior ROI para 1 dev.

### A2. Device Token (`dt_`, SHA-256, read-only no middleware) — **MANTER** `[IMPLEMENTADO]`
Melhor decisão de auth do projeto: read-only forçado por **método HTTP** no middleware (impossível
esquecer numa rota nova), hash em vez de plaintext. Follow-up de rotação automática (H4) → **ADIAR**.

### A3. Verificação de email por código (HMAC) — **MANTER / reposicionar** `[IMPLEMENTADO]`
Bem feita. Após A1, deixa de ser "complemento do Firebase" e vira o **núcleo de identidade**.

### A4. CORS + rate limiting + política de senha (H1/H2/H3) — **ADICIONAR** `[PROPOSTO]`
Hoje `CorsLayer::permissive()` e sem rate-limit em `/auth/*` (rotas públicas). Convite a abuso —
pior ainda após A1 (passa a guardar senha). `tower_governor` + CORS por allowlist.
- **Esforço baixo · Impacto alto · Risco baixo.** Quick win pré-release.

## B. Armazenamento & sync

### B1. SQLCipher (backend = verdade; cache local) — **MANTER** `[IMPLEMENTADO]`
Coerente: o engine precisa ler a blocklist offline; SQLCipher protege contra acesso físico ao disco
(relevante no device do filho). Não mexer.

### B2. Sync por polling, sem realtime — **MANTER + completar** `[IMPLEMENTADO/PROPOSTO]`
Para controle parental, propagação em segundos é aceitável; realtime (SSE/WebSocket) adicionaria
infra de conexão persistente que 1 dev não quer manter. Falta o **poll periódico** no device do
filho (gap F2) — hoje só sincroniza em load/mutação.
- **Recomendado:** poll a cada ~30–60s com `ETag`/`updated_at` → 304 barato. **Esforço baixo ·
  Impacto médio · Risco baixo.**

### B3. Fila offline de writes (F7) — **ADIAR** `[DECIDIDO]`
O filho é read-only; pai/pessoal normalmente estão online ao editar. Ganho pequeno vs. complexidade
de reconciliação. Por ora, **erro claro + retry manual**.

### B4. Blocklist única por conta (F4/P5) — **MANTER (por ora)** `[DECIDIDO]`
Listas por filho é produto, não fundação. Adiar explicitamente.

## C. Engine & anti-bypass

### C1. Engine desktop (DNS proxy + WFP + CA + block pages + cleanup órfão) — **MANTER (intocado)** `[IMPLEMENTADO]`
É a joia da coroa: ordem de start correta, WFP cobrindo DoH/DoT/DoQ (IP+FQDN, IPv4+IPv6), cleanup de
DNS órfão robusto. Gaps residuais (DoH self-hosted, SNI inspection) estão **corretamente aceitos**.
Follow-ups baratos: detectar crash da task do proxy (H8) e DNS alterado externamente (H9) → ADIAR.

### C2. Assimetria desktop forte × mobile frágil — **ADICIONAR (mitigações realistas)** `[PROPOSTO]`
Sem root, nenhuma engenharia iguala o WFP no Android. O que **de fato** move o ponteiro, por ROI:
1. **Avisar o pai quando a VPN cai** (`onRevoke` → notificação/registro no backend). Não impede o
   bypass, mas o **torna visível ao responsável** — é o que controle parental real faz. **Maior ROI
   anti-bypass do mobile, por pouquíssimo código.**
2. **Accessibility como detector de tamper:** detectar abertura da tela de VPN/DNS das Configurações
   e trazer o app/avisar. Transforma o ponto fraco num evento observável.
3. **Honestidade no produto:** documentar que Android sem root é dissuasão (autocontrole/filho
   leigo), não cofre.
- **Esforço médio · Impacto alto (no contexto Android) · Risco médio.**

### C3. Bloqueio de apps no mobile — **ADICIONAR (terminar)** `[PROPOSTO]`
Hoje o caminho de dados está **rompido em 3 camadas**: o provider só envia `domain`; não há método de
canal `updateBlockedPackages`; o `AppBlockerService` nunca recebe a lista. Terminar: enviar `app`
items do provider → método no `MainActivity`/channel → `AppBlockerService` aplica **overlay
full-screen** (técnica padrão de Cold Turkey/AppBlock) em vez do "trazer pra frente" (contornável).
- **Esforço médio · Impacto alto · Risco médio.** Robusto vs. filho leigo; o técnico desativa o
  Accessibility → daí C2.2.

### C4. Filtro adulto no mobile — **ADICIONAR via resolver upstream filtrado** `[PROPOSTO]`
O `DnsForwarder` **já encaminha** o que não está bloqueado — trocar o upstream para um **resolver de
família** é praticamente uma linha e dá cobertura adulta sempre atualizada, sem 100k entradas no
device, sem FFI, sem cache de 7 dias.
- **Trade-off (privacidade):** expõe os domínios ao resolver. Aceitável **no device do filho**
  (privacidade deliberadamente reduzida); no modo pessoal, deixar **opcional**.
- **Alternativa "zero terceiros":** lista server-side baixada e cacheada em SQLCipher local (mais
  privada, mais trabalho). **Esforço baixo→médio · Impacto médio · Risco baixo.**

### C5. Reuso da crate `shared` no mobile via FFI — **NÃO (manter reimplementação)** `[DECIDIDO]`
`DomainMatcher`/regra parental são ~4 linhas reimplementadas em Dart/Kotlin. UniFFI traria toolchain
NDK + binários por ABI por ganho mínimo. Decisão consciente, não dívida.

## D. Backend, deploy & CI (maior ROI de custo/velocidade)

### D1. Dockerfile real + `infra/compose.yml` — **ADICIONAR** `[PROPOSTO]`
O Dockerfile é um comentário; o compose depende dele. Multi-stage (builder com OpenSSL → runtime
enxuto) é ~meia hora. Sem isso, não há deploy.
- **Esforço baixo · Impacto alto · Risco baixo.**

### D2. Hospedagem single-instance com volume persistente — **ADICIONAR** `[PROPOSTO]`
A fonte de verdade é **um arquivo SQLCipher**. Isso prende a **1 instância** (sem scale-out) e exige
**backup do volume** — perfeito para pré-lançamento. Fly.io/Render (disk) ou VPS (Hetzner, mais
barato/controlável). Escalar no futuro = migrar para Postgres (libsql/Turso como ponte).
- **Esforço baixo · Impacto alto · Risco baixo.**

### D3. CI mínimo (GitHub Actions) — **ADICIONAR** `[PROPOSTO]`
`cargo test` + `pnpm check` + `flutter analyze` (depois `cargo audit`/`pnpm audit`, D8). Em
`ubuntu-latest` o OpenSSL é `apt install` — elimina o atrito de build do Windows. **Melhor ROI
individual do projeto para 1 dev.** Esforço baixo · Impacto alto · Risco baixo.

### D4. `/healthz` estruturado (D9) — **ADICIONAR** `[PROPOSTO]`
Health-check que valide a conexão/`PRAGMA key` do SQLCipher (necessário para o health-check do
Fly/Render). Esforço baixo.

### D5. Auto-updater Tauri + assinatura de binário (D4/H12) — **ADIAR** `[DECIDIDO]`
Pré-requisito de distribuição pública (sem isso: "Unknown publisher" + sem update), mas não bloqueia
pré-lançamento. Assinatura custa $ (cert). Agendar para a onda de "primeiro release".

## Decisões "já verdadeiras" a registrar (não são trabalho)

`[DECIDIDO]` — devem aparecer como decisões conscientes, não como dívida:
- Paridade da máquina de estados de auth no mobile (inclui `AuthBackendUnavailable`).
- Reimplementação trivial em Dart/Kotlin em vez de FFI da crate `shared`.
- Backend single-instance preso a um arquivo SQLite (limitação aceita; exige backup de volume).
- Device Token read-only forçado por método HTTP no middleware.
- Limite anti-bypass do Android sem root (dissuasão, não cofre).

## Norte recomendado

Um **backend Axum auto-suficiente** (identidade própria, sem Firebase) como **única** fonte de
verdade em SQLCipher, **deployável e com CI verde**; **engine desktop intocado**; **mobile honesto**
sobre seus limites (DNS-sinkhole + apps via Accessibility + detector de tamper que avisa o pai;
adulto via resolver filtrado). Regra: **consolidar identidade, terminar o que está rompido no mobile,
tornar o backend deployável/observável — sem reescrever o que já é bom.**

**Não tocar:** engine desktop, crate `shared`, regra pai imune, Device Token, SQLCipher como verdade,
sync por polling, reimplementação sem FFI, backend single-file SQLite (por ora).

## Roadmap (em ondas)

**Onda 1 — quick wins (alto ROI, baixo esforço)**
1. Dockerfile real + deploy + `/healthz` (D1/D2/D4) — sem isso não há produto rodando.
2. CI no GitHub Actions (D3) — elimina atrito de build, protege contra regressão.
3. Rate-limit + CORS allowlist (A4) — fecha abuso da auth pública.
4. Avisar o pai quando a VPN cai no device do filho (C2.1) — maior ganho anti-bypass mobile por linha.
5. Poll periódico de blocklist no filho (B2) — fecha o golden path parental.

**Onda 2 — médio prazo**
6. Auth self-hosted no Axum, removendo Firebase (A1) — **antes de ter usuários**.
7. Terminar bloqueio de apps mobile (C3).
8. Accessibility como detector de tamper (C2.2).
9. Filtro adulto mobile via resolver filtrado (C4).

**Onda 3 — apostas maiores**
10. Auto-updater Tauri + assinatura de binário (D5).
11. Telemetria/stats opt-in (F8) + métricas do DNS proxy (O2).
12. Testes E2E (Tauri/Playwright) e integração backend+desktop+DB real (O1/O5).
13. Sob demanda: listas por filho (P5), fila offline (F7).

## Limitações aceitas (residuais)

- **Anti-bypass desktop:** DoH self-hosted (IP+FQDN próprios), VPN com DNS embarcado e DNS-over-Tor
  escapam — inerente a soluções sem driver kernel-mode. SNI inspection via driver fica para v1.0+ se
  houver sinal de bypass relevante. Cert pinning de alguns apps contorna a block page.
- **Anti-bypass mobile (sem root):** desligar a VPN, DoH no Chrome, trocar DNS. Mitigar via aviso ao
  pai + detector de tamper (C2), não prometer cofre.
- **Plataformas:** macOS/iOS (P1) e Linux (P2) fora de escopo; horários/tarefas/relatórios (P3/P4)
  são produto futuro.
