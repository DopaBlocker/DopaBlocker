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
| GET | `/health` | Pública |
| POST | `/auth/email-code/start` | Pública |
| POST | `/auth/email-code/verify` | Pública |
| POST | `/auth/register` | Firebase JWT (user local pode não existir ainda) |
| POST | `/auth/login` | Firebase JWT (user local precisa existir) |
| GET | `/auth/me` | Firebase JWT **ou** Device Token |
| DELETE | `/auth/me` | Firebase JWT |
| GET | `/blocklist` | Firebase JWT **ou** Device Token |
| POST | `/blocklist` | Firebase JWT |
| DELETE | `/blocklist/{id}` | Firebase JWT |
| PUT | `/blocklist/adult-filter` | Firebase JWT |
| POST | `/devices/register` | Firebase JWT |
| GET | `/devices` | Firebase JWT **ou** Device Token |
| POST | `/devices/link/generate` | Firebase JWT |
| POST | `/devices/link/confirm` | Pública |
| POST | `/devices/{id}/revoke` | Firebase JWT |

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
  email_verification_token? }` → `User`. `email_verification_token` é obrigatório quando o provider é
  `password`. O email do JWT vence o do body (divergência → 400). Erros: 400 (sem verificação /
  email divergente / provider externo sem email verificado), 409 (UID já tem user local).
- **POST `/auth/login`** — header Firebase JWT, body vazio → `User`; 404 se ainda não há user local
  (cliente deve chamar `/auth/register`).
- **GET `/auth/me`** — Firebase JWT ou Device Token (com token, retorna o `User` do pai).
- **DELETE `/auth/me`** — Firebase JWT. Apaga user local + dependentes por cascade. O frontend é
  responsável por apagar a conta no Firebase (não há Firebase Admin SDK no backend).

## Blocklist

- **GET `/blocklist`** → array de `BlockedItem` (ordenado por `created_at DESC`).
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

### Fluxo de vinculação parental (resumo)

Pai: login → `auth/login` → `devices/register` → `devices/link/generate` (code). Filho: digita o
code → `devices/link/confirm` (recebe `device_token`) → `GET /blocklist` com `Bearer dt_…` em polling.
Pai adiciona item → o filho vê na próxima rodada de polling e atualiza o engine.
