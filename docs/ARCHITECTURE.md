# DopaBlocker — Arquitetura

## Visão Geral

Monorepo com 4 sub-projetos: backend, desktop, mobile e shared.

## Fluxo de Dados

```
[Desktop App (Tauri)]  --HTTP/JWT ou dt_--> [Backend API (Axum)] --SQLCipher--> [dopablocker.db]
[Mobile App (Flutter)] --HTTP/JWT ou dt_--> [Backend API (Axum)] --SQLCipher--> [dopablocker.db]
[Firebase Auth]        --JWT/JWKS----------> [Backend API (Axum)]
```

O backend aceita dois tipos de token no header `Authorization` (ver "Autenticação dual" abaixo). A fonte de verdade dos dados de negócio hoje é o SQLCipher do backend; Firebase é usado para autenticação das contas Pessoal/Pais, não como banco da blocklist.

**Estado atual:** backend, shared e desktop têm implementação funcional. O projeto mobile existe com estrutura Flutter/Kotlin, mas os providers, models, Firebase, SQLCipher Dart e serviços nativos ainda são placeholders para a v0.2.

---

## Fluxo de Onboarding

Ao abrir o app pela primeira vez, o usuário vê **três opções** e cada uma dispara um fluxo distinto:

```
                 ┌──────────────────────┐
                 │ Tela inicial         │
                 │  [Pessoal] [Pais]    │
                 │       [Filhos]       │
                 └──┬──────┬────────┬───┘
                    │      │        │
          ┌─────────┘      │        └────────┐
          │                │                 │
          v                v                 v
    ┌──────────┐     ┌──────────┐    ┌──────────────┐
    │ Pessoal  │     │   Pais   │    │    Filhos    │
    └────┬─────┘     └────┬─────┘    └──────┬───────┘
         │                │                 │
         v                v                 v
   ┌────────────┐  ┌────────────┐   ┌────────────────┐
   │ Cadastro   │  │ Cadastro   │   │ Input 6 díg.   │
   │ Firebase   │  │ Firebase   │   │ (sem cadastro) │
   └─────┬──────┘  └─────┬──────┘   └────────┬───────┘
         │               │                   │
         v               v                   v
  ┌────────────┐ ┌────────────────┐ ┌─────────────────┐
  │ User       │ │ User           │ │ Só cria Device  │
  │ mode=      │ │ mode=          │ │ sob user_id do  │
  │ personal   │ │ parental       │ │ pai c/ is_child │
  │ Device     │ │ Device         │ │ = 1             │
  │ is_child=0 │ │ is_child=0     │ │                 │
  └─────┬──────┘ └───────┬────────┘ └────────┬────────┘
        │                │                   │
        v                v                   v
  ┌────────────┐ ┌────────────────┐ ┌─────────────────┐
  │ Firebase   │ │ Firebase JWT + │ │ Device Token    │
  │ JWT        │ │ Gera códigos   │ │ (dt_xxx)        │
  └────────────┘ └────────────────┘ └─────────────────┘
```

**Detalhes de cada opção** estão documentados em [PROTOTYPE.md](PROTOTYPE.md) → "Fluxo de Onboarding".

---

## Autenticação dual

O backend aceita **dois tipos** de token no header `Authorization`. O middleware inspeciona o prefixo para decidir qual caminho seguir:

### Firebase JWT (contas Pessoal e Pais)

```
Authorization: Bearer eyJhbGci...   ← sem prefixo "dt_"
```

- O frontend (Svelte ou Flutter) faz login no Firebase Auth (email/senha ou Google).
- Firebase retorna um JWT assinado pelas chaves públicas do Google.
- O backend valida a assinatura, o issuer, o audience e a expiração.
- Resolve o `user_id` local via `firebase_uid` na tabela `users`.

### Device Token (devices filhos, sem conta Firebase)

```
Authorization: Bearer dt_a1b2c3d4...   ← prefixo "dt_"
```

- Gerado uma única vez pelo backend ao confirmar o código de vinculação (`POST /devices/link/confirm`, rota pública).
- Salvo na tabela `device_tokens` como **hash** (SHA-256), não em plain text.
- Validado em cada requisição: lookup pelo hash, verificar que `revoked_at IS NULL`, resolver o `user_id` e o `device_id`.
- Escopo **read-only**: rotas de escrita (POST/DELETE/PUT em blocklist, gerar código) são proibidas para device tokens e retornam 403.

### Middleware

O middleware extrai o header `Authorization`, inspeciona o prefixo, e chama a validação correta. Ambos os caminhos resolvem para um `AuthUser { user_id, source }`, onde `source` indica qual tipo de token foi usado. Handlers que precisam diferenciar (ex: "só pai pode adicionar à blocklist") checam o `source`.

---

## Técnicas de Bloqueio

### Windows (Desktop)
- **WFP (Windows Filtering Platform)**: filtros de rede a nível de kernel
- **DNS Proxy**: resolver local que retorna NXDOMAIN para domínios bloqueados
- **Bloom Filter**: lookup rápido de domínios adultos (Steven Black / OISD)

### Android (Mobile)
- **VPN Service**: intercepta tráfego DNS via TUN interface (alvo v0.2; arquivo Kotlin ainda é placeholder)
- **Accessibility Service**: detecta e bloqueia abertura de apps (alvo v0.2)
- **Boot Receiver**: reinicia VPN automaticamente após reboot (alvo v0.2)

### Regra do "Pai imune"

Em ambas as plataformas, o blocking engine consulta o modo do user e o papel do device **antes** de popular a lista de domínios a bloquear:

```
se user.mode == 'personal':
    aplica todos os blocked_items da conta
senão se user.mode == 'parental':
    se device.is_child == true:
        aplica todos os blocked_items da conta
    senão (device do pai):
        NÃO aplica nada (lista vazia)
```

Consequência: no device do pai em modo parental, o DNS Proxy (desktop atual) e
o VPN Service (mobile v0.2) devem permanecer ativos com blocklist vazia —
deixando passar todo o tráfego. A blocklist é aplicada **apenas nos devices filhos**.

---

## Armazenamento Local (SQLCipher)

- **SQLCipher** em vez de SQLite puro — criptografia AES-256 transparente no banco local
- Cada dispositivo tem um banco `.db` criptografado que serve como cache offline
- Sem a chave (`PRAGMA key`), o arquivo é ilegível — protege contra acesso físico ao disco
- Backend (Rust): `rusqlite` com feature `bundled-sqlcipher`
- Desktop (Tauri): mesmo `rusqlite` com `bundled-sqlcipher`
- Mobile (Flutter): planejado com `sqflite_sqlcipher`, ainda não implementado no `pubspec.yaml`

### Tabelas principais
- `users`, `devices`, `blocked_items`, `parental_links`, `adult_filter_settings` (migration `001_initial.sql`)
- `device_tokens` (migration `002_parental_fixes.sql`) — tokens de acesso dos devices filhos
- `email_verifications` (migration `003_email_verification.sql`) — códigos e tokens curtos do cadastro por email/senha

No device do filho, o backend não é consultado para auth (o device token é local). Mas as chamadas à API (blocklist, devices) continuam passando pelo backend normalmente, usando o device token como credencial.

---

## Sincronização

- SQLCipher local como cache offline criptografado em cada dispositivo
- SQLCipher do backend como fonte de verdade para sincronização cross-device
- Backend API como intermediário para validação, lógica de negócios e auth dual
- Desktop atual sincroniza por chamadas REST em carregamento/mutação de telas; polling periódico completo de blocklist entre dispositivos ainda é gap
- Listeners real-time/Firestore não fazem parte da implementação atual

---

## Controle Parental

- **Uma conta, múltiplos dispositivos** (pai + filhos vinculados)
- **Vinculação via código de 6 dígitos** com TTL de 5 minutos
- Pai gerencia a blocklist que propaga para os dispositivos filhos
- **Filho não cria conta** — usa device token gerado no momento da vinculação
- **Pai fica imune** aos próprios blocks (ver "Regra do Pai imune" acima)
- Única blocklist compartilhada entre todos os filhos de uma conta (decisão mantida para o alvo v0.2)
