# DopaBlocker — API REST

> Reflete o backend atual em `backend/src` (`main.rs`, `routes/*`, `middleware.rs`, `models.rs`,
> `services/*`). Arquitetura geral em [ARCHITECTURE.md](ARCHITECTURE.md).

## Base URL

```
Development: http://localhost:3000
Production:  ainda não publicado (deploy é roadmap — ver DECISOES_E_ROADMAP.md)
```

## Convenções

- Erros de handlers: `{ "error": "mensagem legível" }`. Erros barrados pelo middleware podem voltar
  só com status `401`/`403`.
- Sucesso de operações destrutivas: `{ "message": "..." }`.
- Enums em **lowercase**: `mode` ∈ {`personal`,`parental`}; `platform` ∈ {`windows`,`android`};
  `item_type` ∈ {`domain`,`app`,`keyword`}.

## Autenticação dual

| Tipo | Prefixo | Quem usa |
|---|---|---|
| **Firebase JWT** | sem prefixo | Contas Pessoal/Pais |
| **Device Token** | `dt_` | Devices filhos (sem conta Firebase) |

O Firebase JWT é validado por assinatura (JWKS), `iss`, `aud` e expiração; resolve o user por
`firebase_uid`. O Device Token é emitido uma vez em `POST /devices/link/confirm`; o banco guarda só o
**SHA-256** da parte plain. Device Tokens são **read-only**: qualquer método ≠ `GET`/`HEAD` é
rejeitado com **403** antes do handler.

### Matriz de autenticação

| Método | Rota | Auth |
|---|---|---|
| GET | `/health` | Pública (liveness) |
| GET | `/healthz` | Pública (readiness — valida o SQLCipher) |
| POST | `/auth/email-code/start` | Pública |
| POST | `/auth/email-code/verify` | Pública |
| POST | `/auth/register` | Firebase JWT (idempotente: cria, recupera ou retorna a conta) |
| POST | `/auth/login` | Firebase JWT (user local precisa existir) |
| GET | `/auth/me` | Firebase JWT **ou** Device Token |
| PUT | `/auth/me` | Firebase JWT (troca o modo da conta) |
| DELETE | `/auth/me` | Firebase JWT |
| GET | `/blocklist` | Firebase JWT **ou** Device Token |
| POST | `/blocklist` | Firebase JWT |
| DELETE | `/blocklist/{id}` | Firebase JWT |
| PUT | `/blocklist/adult-filter` | Firebase JWT |
| POST | `/devices/register` | Firebase JWT |
| GET | `/devices` | Firebase JWT **ou** Device Token |
| GET | `/devices/events` | Firebase JWT (só o pai) |
| POST | `/devices/link/generate` | Firebase JWT |
| POST | `/devices/link/confirm` | Pública |
| POST | `/devices/tamper` | Pública (Device Token **no corpo**) |
| POST | `/devices/{id}/revoke` | Firebase JWT |

> **CORS + rate-limit (A4):** as rotas públicas de auth (`/auth/*`) e
> `/devices/link/confirm` / `/devices/tamper` passam por **rate-limit por IP**
> (GCRA via `tower_governor`); origens são restritas por **allowlist**
> (`CORS_ALLOWED_ORIGINS`, default cobre o dev). Mobile (HTTP nativo) não usa CORS.

## Auth

Cadastro por email/senha exige verificação por código **antes** de criar o user local:
`email-code/start` → `email-code/verify` → signup no Firebase (frontend) → `auth/register` com o
Firebase JWT e o `email_verification_token`. Para providers não-`password` (ex.: Google), não há
código, mas o email precisa vir verificado nas claims.

- **POST `/auth/email-code/start`** — body `{ "email": "…" }` → `{ expires_at, resend_after_seconds }`.
  Código de 6 dígitos, expira em 10 min, cooldown de 60s, máx. 5 envios/hora. Em
  `EMAIL_DELIVERY_MODE=log`, o código vai para os logs.
- **POST `/auth/email-code/verify`** — body `{ email, code }` → `{ email_verification_token }`.
  Máx. 5 tentativas; token expira em 15 min. O backend guarda **HMAC-SHA256** do código e do token.
- **POST `/auth/register`** — header Firebase JWT; body `{ email, display_name, mode,
  email_verification_token? }` → `User`. **Idempotente e resiliente à identidade** (o nome é
  histórico; na prática "garante que a conta deste login exista"):
    1. Já existe conta para o `firebase_uid` → retorna-a (**não é mais 409**).
    2. Não existe para o UID, mas existe conta com este **email** (presa a um UID antigo — conta
       órfã) → **reclaim**: reassocia a conta a este UID (preserva `mode`/`id`/`created_at`). Destrava
       o email, que antes ficava preso pelo `UNIQUE`.
    3. Nada existe → cria a conta.
  A prova de posse do email é a mesma do cadastro: `password` exige `email_verification_token`;
  provider verificado (Google) exige `email_verified` no claim. O email do JWT vence o do body
  (divergência → 400). Erros: 400 (sem verificação / email divergente / provider externo sem email
  verificado).
- **POST `/auth/login`** — header Firebase JWT, body vazio → `User`; 404 se ainda não há user local
  (cliente deve chamar `/auth/register`).
- **GET `/auth/me`** — Firebase JWT ou Device Token (com token, retorna o `User` do pai).
- **PUT `/auth/me`** — Firebase JWT; body `{ mode }` (`personal`|`parental`) → `User`. Troca o modo da
  conta sem recriá-la — não mexe em devices/blocklist/vínculos; a regra "pai imune" passa a valer (ou
  deixa de valer) no próximo sync. Device Token → 403.
- **DELETE `/auth/me`** — Firebase JWT. Apaga user local + dependentes por cascade. O frontend é
  responsável por apagar a conta no Firebase (não há Firebase Admin SDK no backend).

## Health

- **GET `/health`** → `OK` (liveness simples).
- **GET `/healthz`** → `{ "status": "ok" }` (200) se um `SELECT 1` no SQLCipher
  passa; `{ "status": "error" }` (503) se o banco está inacessível. Usado por
  health-checks de infra.

## Blocklist

- **GET `/blocklist`** → array de `BlockedItem` (ordenado por `created_at DESC`).
  Suporta **ETag/`If-None-Match`** (B2): devolve o header `ETag`; se o cliente
  reenviar o mesmo ETag e a lista não mudou, responde **`304 Not Modified`**
  (sem corpo). O poll do filho (desktop e mobile) usa isto para baratear ~30–60s.
- **POST `/blocklist`** — body `{ item_type, value }` → `BlockedItem`. `domain` passa por
  `normalize_domain`; `app`/`keyword` recebem `trim().lowercase()`. `value` vazio → 400; domínio sem
  ponto → 400; duplicata `(user_id, item_type, value)` → 409.
- **DELETE `/blocklist/{id}`** → `{ message }`; 404 se não existir/não pertencer ao user.
- **PUT `/blocklist/adult-filter`** — body `{ enabled }` → `AdultFilterSettings` (upsert por user).

`BlockedItem`: `{ id, user_id, item_type, value, is_active, created_at }`.

## Devices

- **POST `/devices/register`** — body `{ device_name, platform }` → `Device` (sempre
  `is_child=false`; device filho só nasce no confirm).
- **GET `/devices`** → array de `Device` (com Device Token, retorna os devices do pai; ordenado por
  `created_at ASC`).
- **POST `/devices/link/generate`** → `{ code, expires_at }`. Exige ≥1 device `is_child=false`
  registrado; código de 6 dígitos, expira em 5 min; colisão com outro `pending` → 409.
- **POST `/devices/link/confirm`** (pública) — body `{ code, device_name, platform }` →
  `{ device_token, device_id, user_id, parent_device_id }`. Em transação: valida código pending/não
  expirado, cria device filho, marca link `active`, salva o **hash** do token. O `device_token` é
  retornado **uma única vez** — o app do filho deve guardá-lo em storage seguro.
- **POST `/devices/{id}/revoke`** — Firebase JWT → `{ message }`. Marca `device_tokens.revoked_at` e
  o link como `revoked`; 404 se não for um filho válido do user.
- **POST `/devices/tamper`** (pública) — body `{ device_token, kind }` →
  `{ message }`. O device do filho reporta adulteração (C2.1/C2.2). Autentica-se
  pelo **Device Token no corpo** (com ou sem prefixo `dt_`), não pelo header —
  assim a regra read-only do middleware fica intocada. `kind` ∈
  {`vpn_revoked`,`vpn_settings_opened`,`dns_settings_opened`}. Token inválido/
  revogado → 401; `kind` desconhecido → 400.
- **GET `/devices/events`** — Firebase JWT (só o pai) → array de `DeviceEvent`
  (`{ id, user_id, device_id, kind, created_at, acknowledged_at }`), mais recentes
  primeiro (limite 100). Device Token (filho) → 403.

### Fluxo de vinculação parental (resumo)

Pai: login → `auth/login` → `devices/register` → `devices/link/generate` (code). Filho: digita o
code → `devices/link/confirm` (recebe `device_token`) → `GET /blocklist` com `Bearer dt_…` em polling.
Pai adiciona item → o filho vê na próxima rodada de polling e atualiza o engine.
