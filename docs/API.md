# DopaBlocker — API REST

Este documento reflete a implementação atual do backend em `backend/src`
(`main.rs`, `routes/*`, `middleware.rs`, `models.rs` e `services/*`).

## Base URL

```text
Development: http://localhost:3000
Production:  https://api.dopablocker.com (planejado/Azure)
```

## Convenções

### JSON e erros

Rotas de negócio recebem e retornam JSON. Erros gerados por handlers usam o
formato:

```json
{ "error": "mensagem legível" }
```

Exceção prática: erros barrados diretamente pelo middleware de autenticação em
rotas protegidas podem voltar apenas com status `401` ou `403`.

### Valores aceitos

Enums são serializados em lowercase:

| Campo | Valores |
|---|---|
| `mode` | `personal`, `parental` |
| `platform` | `windows`, `android` |
| `item_type` | `domain`, `app`, `keyword` |

### Resposta de sucesso simples

Algumas operações destrutivas retornam:

```json
{ "message": "..." }
```

---

## Autenticação dual

O backend aceita dois tipos de token no header `Authorization`:

| Tipo | Prefixo | Quem usa | Formato |
|---|---|---|---|
| **Firebase JWT** | sem prefixo | Contas Pessoal e Pais | `Authorization: Bearer eyJhbGci...` |
| **Device Token** | `dt_` | Devices filhos sem conta Firebase | `Authorization: Bearer dt_<plain_token>` |

O Firebase JWT é obtido pelo frontend via Firebase Auth SDK. O backend valida
assinatura, `iss`, `aud` e expiração, depois resolve o usuário local por
`firebase_uid`.

O Device Token é emitido uma única vez em `POST /devices/link/confirm`. O
response contém `dt_<plain_token>`, mas o banco guarda apenas o SHA-256 da parte
plain, sem o prefixo `dt_`. Em cada request, o middleware remove `dt_`, calcula
o hash e busca `device_tokens.revoked_at IS NULL`.

Device Tokens são **read-only** nas rotas protegidas: qualquer método diferente
de `GET`/`HEAD` com Device Token é rejeitado com `403` antes de chegar ao
handler.

### Matriz de autenticação

| Método | Rota | Autenticação implementada |
|---|---|---|
| GET | `/health` | Pública |
| POST | `/auth/email-code/start` | Pública; `Authorization` é ignorado |
| POST | `/auth/email-code/verify` | Pública; `Authorization` é ignorado |
| POST | `/auth/register` | Firebase JWT obrigatório; user local pode não existir |
| POST | `/auth/login` | Firebase JWT obrigatório; user local precisa existir |
| GET | `/auth/me` | Firebase JWT ou Device Token |
| DELETE | `/auth/me` | Firebase JWT apenas |
| GET | `/blocklist` | Firebase JWT ou Device Token |
| POST | `/blocklist` | Firebase JWT apenas |
| DELETE | `/blocklist/{id}` | Firebase JWT apenas |
| PUT | `/blocklist/adult-filter` | Firebase JWT apenas |
| POST | `/devices/register` | Firebase JWT apenas |
| GET | `/devices` | Firebase JWT ou Device Token |
| POST | `/devices/link/generate` | Firebase JWT apenas |
| POST | `/devices/link/confirm` | Pública |
| POST | `/devices/{id}/revoke` | Firebase JWT apenas |

---

## Health

| Método | Rota | Descrição |
|---|---|---|
| GET | `/health` | Healthcheck simples |

**Response 200:**

```text
OK
```

---

## Auth

Cadastro por email/senha usa verificação por código antes de criar o usuário
local:

1. `POST /auth/email-code/start`
2. `POST /auth/email-code/verify`
3. Cadastro no Firebase pelo frontend
4. `POST /auth/register` com Firebase JWT e `email_verification_token`

Para providers que não são `password` (ex: Google), o backend não exige código,
mas exige que o email venha verificado nas claims do Firebase.

| Método | Rota | Descrição |
|---|---|---|
| POST | `/auth/email-code/start` | Enviar código de 6 dígitos para validar email |
| POST | `/auth/email-code/verify` | Validar código e emitir token curto de verificação |
| POST | `/auth/register` | Criar usuário local para uma conta Firebase |
| POST | `/auth/login` | Buscar/sincronizar usuário local a partir do JWT Firebase |
| GET | `/auth/me` | Dados do usuário autenticado |
| DELETE | `/auth/me` | Excluir a conta local e dados associados |

`firebase_uid` nunca vem no body; é extraído do JWT validado. A senha também
não transita pelo backend.

### POST /auth/email-code/start

Body:

```json
{ "email": "pai@example.com" }
```

Response 200:

```json
{
  "expires_at": "2026-04-10T12:10:00Z",
  "resend_after_seconds": 60
}
```

Regras implementadas:

- Email é normalizado com `trim().to_lowercase()` e validado.
- Código tem 6 dígitos e expira em 10 minutos.
- Cooldown de reenvio: 60 segundos.
- Limite por email: 5 envios por hora.
- Em `EMAIL_DELIVERY_MODE=log`, o código é escrito nos logs do backend.

### POST /auth/email-code/verify

Body:

```json
{
  "email": "pai@example.com",
  "code": "123456"
}
```

Response 200:

```json
{ "email_verification_token": "token-opaco" }
```

Regras implementadas:

- O código deve ter exatamente 6 dígitos.
- Máximo de 5 tentativas por código.
- O token de verificação emitido expira em 15 minutos.
- O backend guarda HMAC-SHA256 do código e do token, não os valores puros.

### POST /auth/register

Requer:

```text
Authorization: Bearer <firebase_jwt>
```

Body:

```json
{
  "email": "pai@example.com",
  "display_name": "João",
  "mode": "parental",
  "email_verification_token": "token-opaco"
}
```

`email_verification_token` é opcional no DTO, mas obrigatório quando o provider
Firebase é `password`. Para Google/outros providers, o backend aceita apenas se
`email_verified=true` nas claims.

Identidade usada pelo backend:

- Se o JWT traz `email`, ele vence o body.
- Se o body traz email diferente do JWT, retorna `400`.
- Se o JWT traz `name`, ele vence `display_name`.
- Se não há nome no JWT nem no body, o backend deriva do prefixo do email.

Response 200:

```json
{
  "id": "uuid-do-user",
  "firebase_uid": "firebase-uid",
  "email": "pai@example.com",
  "display_name": "João",
  "mode": "parental",
  "created_at": "2026-04-10T12:00:00Z"
}
```

Erros comuns:

- `400` se email do body divergir do email autenticado.
- `400` se provider `password` não tiver verificação de email.
- `400` se provider externo não trouxer email verificado.
- `409` se o Firebase UID já tiver usuário local.

### POST /auth/login

Requer:

```text
Authorization: Bearer <firebase_jwt>
```

Body: vazio.

Response 200: `User`.

Erros comuns:

- `404` se o Firebase UID ainda não tem registro local. O cliente deve chamar
  `/auth/register`.

### GET /auth/me

Requer Firebase JWT ou Device Token.

Response 200: `User`.

Quando chamado com Device Token, retorna o `User` do pai, porque o filho não tem
conta própria.

### DELETE /auth/me

Requer Firebase JWT. Device Token é rejeitado.

Response 200:

```json
{ "message": "Conta excluida" }
```

O backend apaga o usuário local e os dados dependentes por cascade
(`devices`, `blocked_items`, `parental_links`, `adult_filter_settings` e
`device_tokens`). O frontend ainda é responsável por excluir a conta no Firebase;
o backend não usa Firebase Admin SDK no v0.1.

---

## Blocklist

| Método | Rota | Descrição | Device Token |
|---|---|---|---|
| GET | `/blocklist` | Listar itens do usuário | Sim |
| POST | `/blocklist` | Adicionar item | Não |
| DELETE | `/blocklist/{id}` | Remover item | Não |
| PUT | `/blocklist/adult-filter` | Toggle do filtro adulto | Não |

### GET /blocklist

Response 200:

```json
[
  {
    "id": "uuid-do-item",
    "user_id": "uuid-do-user",
    "item_type": "domain",
    "value": "instagram.com",
    "is_active": true,
    "created_at": "2026-04-10T12:00:00Z"
  }
]
```

Os itens vêm ordenados por `created_at DESC`.

### POST /blocklist

Body:

```json
{
  "item_type": "domain",
  "value": "https://www.Instagram.com/reels"
}
```

Response 200:

```json
{
  "id": "uuid-do-item",
  "user_id": "uuid-do-user",
  "item_type": "domain",
  "value": "instagram.com",
  "is_active": true,
  "created_at": "2026-04-10T12:00:00Z"
}
```

Regras implementadas:

- `domain` passa por `normalize_domain`.
- `app` e `keyword` recebem `trim().to_lowercase()`.
- `value` vazio retorna `400`.
- Domínio sem ponto retorna `400`.
- Duplicata por `(user_id, item_type, value)` retorna `409`.

### DELETE /blocklist/{id}

Response 200:

```json
{ "message": "Item removido" }
```

Retorna `404` se o item não existir ou não pertencer ao usuário autenticado.

### PUT /blocklist/adult-filter

Body:

```json
{ "enabled": true }
```

Response 200:

```json
{
  "id": "uuid-da-config",
  "user_id": "uuid-do-user",
  "is_enabled": true,
  "last_list_update": null
}
```

O backend faz upsert em `adult_filter_settings` por `user_id`.

---

## Devices

| Método | Rota | Descrição | Autenticação |
|---|---|---|---|
| POST | `/devices/register` | Registrar device do titular/pai | Firebase JWT |
| GET | `/devices` | Listar devices da conta/família | Firebase JWT ou Device Token |
| POST | `/devices/link/generate` | Gerar código de vinculação | Firebase JWT |
| POST | `/devices/link/confirm` | Confirmar código e emitir Device Token | Pública |
| POST | `/devices/{id}/revoke` | Revogar/desvincular device filho | Firebase JWT |

### POST /devices/register

Body:

```json
{
  "device_name": "PC do João",
  "platform": "windows"
}
```

Response 200:

```json
{
  "id": "uuid-do-device",
  "user_id": "uuid-do-user",
  "device_name": "PC do João",
  "platform": "windows",
  "is_child": false,
  "created_at": "2026-04-10T12:00:00Z"
}
```

Esta rota sempre cria device com `is_child=false`. Device filho só nasce por
`POST /devices/link/confirm`.

### GET /devices

Response 200:

```json
[
  {
    "id": "uuid-do-device",
    "user_id": "uuid-do-user",
    "device_name": "PC do João",
    "platform": "windows",
    "is_child": false,
    "created_at": "2026-04-10T12:00:00Z"
  }
]
```

Com Device Token, retorna os devices do `user_id` do pai. A lista é ordenada por
`created_at ASC`.

### POST /devices/link/generate

Response 200:

```json
{
  "code": "123456",
  "expires_at": "2026-04-10T12:05:00Z"
}
```

Regras implementadas:

- O usuário precisa ter pelo menos um device `is_child=false` registrado antes.
- O backend usa o primeiro device não-filho do usuário como `parent_device_id`.
- O código tem 6 dígitos, pode começar com zero e expira em 5 minutos.
- Se houver colisão com outro código `pending`, retorna `409`; o cliente pode tentar de novo.

### POST /devices/link/confirm

Rota pública.

Body:

```json
{
  "code": "123456",
  "device_name": "Celular do João Jr.",
  "platform": "android"
}
```

Response 200:

```json
{
  "device_token": "dt_a1b2c3d4e5f6...",
  "device_id": "uuid-do-novo-device",
  "user_id": "uuid-do-user-pai",
  "parent_device_id": "uuid-do-device-pai"
}
```

O backend, em uma transação:

1. Valida código `pending` e não expirado.
2. Resolve `user_id` pelo `parent_device_id`.
3. Cria device filho com `is_child=true`.
4. Marca o link como `active`.
5. Salva o hash do token em `device_tokens`.

Erros comuns:

- `400` para código inválido, expirado ou já utilizado.
- `500` para falhas inesperadas de persistência.

O `device_token` deve ser salvo pelo app do filho em storage seguro. Não há
endpoint para recuperá-lo depois.

### POST /devices/{id}/revoke

Revoga um device filho vinculado.

Response 200:

```json
{ "message": "Dispositivo desvinculado" }
```

Regras implementadas:

- Apenas Firebase JWT.
- O `{id}` precisa ser um device do mesmo `user_id`.
- O device precisa ter `is_child=true`.
- Marca `device_tokens.revoked_at` e atualiza o link parental `active` para `revoked`.
- Retorna `404` se o filho não existir, não pertencer ao usuário, não for filho ou já estiver revogado.

---

## Fluxo completo de vinculação parental

```text
Device do Pai                           Device do Filho
──────────────────────────────────      ─────────────────────────────────
1. Login Firebase
2. POST /auth/login
3. POST /devices/register
   → parent device is_child=false
4. POST /devices/link/generate
   → code "123456"
                                        5. Usuário digita "123456"
                                        6. POST /devices/link/confirm
                                           → device_token + device_id
                                        7. Guarda device_token
                                        8. GET /blocklist
                                           Authorization: Bearer dt_...

9. POST /blocklist
   → adiciona youtube.com
                                        10. Polling GET /blocklist
                                            → vê youtube.com
                                            → atualiza DNS Proxy/VPN
```
