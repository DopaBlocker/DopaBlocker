# DopaBlocker — API REST

## Base URL

```
Development: http://localhost:3000
Production:  https://api.dopablocker.com (Azure)
```

## Autenticação dual

O backend aceita **dois tipos** de token no header `Authorization`, e o middleware decide qual usar pelo prefixo:

| Tipo | Prefixo | Quem usa | Formato |
|---|---|---|---|
| **Firebase JWT** | (sem prefixo) | Contas Pessoal e Pais | `Authorization: Bearer eyJhbGci...` |
| **Device Token** | `dt_` | Devices filhos (sem conta) | `Authorization: Bearer dt_<plain_token>` |

- O **Firebase JWT** é obtido pelo frontend via Firebase Auth SDK (login com email/senha ou Google). É renovado automaticamente pelo SDK.
- O **Device Token** é obtido uma única vez pelo device filho quando ele confirma o código de vinculação (`POST /devices/link/confirm`). Fica válido até o pai revogar manualmente.

Ambos resolvem para um `user_id` no backend (o id da conta do pai, no caso do device token).

Somente `/devices/link/confirm` e `/auth/email-code/*` são **totalmente anônimas**. `/auth/register` e `/auth/login` exigem um Firebase JWT no header — eles só não exigem que o `user` já exista localmente no SQLCipher.

### Matriz de autenticação

| Método | Rota | Firebase JWT | Device Token | Anônima |
|---|---|---|---|---|
| POST | `/auth/email-code/start` | opcional | não aceito | **sim** |
| POST | `/auth/email-code/verify` | opcional | não aceito | **sim** |
| POST | `/auth/register` | obrigatório (user pode não existir no banco) | não aceito | — |
| POST | `/auth/login` | obrigatório (user pode não existir no banco) | não aceito | — |
| POST | `/devices/link/confirm` | — | — | **sim** |
| Demais | * | obrigatório (+ user local) | aceito (somente leitura) | — |

Tentativas de escrita (POST/DELETE/PUT) com Device Token retornam **403 Forbidden**. Tentativas de `/devices/link/generate` com Device Token também retornam **403** (apenas contas Firebase geram códigos).

---

## Endpoints

### Auth

Cadastro por email/senha usa verificação por código antes de criar o usuário local: primeiro `POST /auth/email-code/start`, depois `POST /auth/email-code/verify`, depois Firebase Auth e `POST /auth/register` com `email_verification_token`.

| Método | Rota | Descrição |
|---|---|---|
| POST | `/auth/email-code/start` | Enviar código de 6 dígitos para validar email de cadastro |
| POST | `/auth/email-code/verify` | Validar código e emitir token curto de verificação |
| POST | `/auth/register` | Criar registro local para uma conta Firebase recém-criada (email, display_name, mode) |
| POST | `/auth/login` | Sincronizar user local com o Firebase user (body vazio, JWT no header) |
| GET | `/auth/me` | Dados do usuário autenticado |

> **Importante:** `firebase_uid` **nunca** vem do body — é extraído das claims do JWT validado. A senha também não transita pelo backend: é cadastrada no Firebase pelo frontend antes de chamar `/auth/register`.

**POST /auth/email-code/start** — body:
```json
{ "email": "pai@example.com" }
```

**Response 200:**
```json
{
  "expires_at": "2026-04-10T12:10:00Z",
  "resend_after_seconds": 60
}
```

**POST /auth/email-code/verify** — body:
```json
{
  "email": "pai@example.com",
  "code": "123456"
}
```

**Response 200:**
```json
{ "email_verification_token": "token-opaco" }
```

O backend guarda apenas hashes HMAC-SHA256 do código e do token. O código expira em 10 minutos, aceita no máximo 5 tentativas e respeita cooldown de 60 segundos para reenvio.

**POST /auth/register** — body:
```json
{
  "email": "pai@example.com",
  "display_name": "João",
  "mode": "parental",
  "email_verification_token": "token-opaco"
}
```
> A senha é cadastrada direto no Firebase pelo frontend, não pelo backend. Para provider Firebase `password`, `email_verification_token` é obrigatório. Para Google, o backend aceita o email já verificado pelo provider.

**Response 200:**
```json
{
  "id": "uuid-do-user",
  "firebase_uid": "...",
  "email": "pai@example.com",
  "display_name": "João",
  "mode": "parental",
  "created_at": "2026-04-10T12:00:00Z"
}
```

---

### Blocklist

> Requer token (JWT ou Device Token). Device tokens têm acesso **read-only** — tentativas de POST/DELETE/PUT com device token retornam **403 Forbidden**.

| Método | Rota | Descrição | Aceita Device Token |
|---|---|---|---|
| GET | `/blocklist` | Listar itens bloqueados do usuário | Sim (read-only) |
| POST | `/blocklist` | Adicionar item (item_type, value) | Não |
| DELETE | `/blocklist/:id` | Remover item | Não |
| PUT | `/blocklist/adult-filter` | Toggle filtro adulto (enabled: bool) | Não |

**POST /blocklist** — body:
```json
{
  "item_type": "domain",
  "value": "instagram.com"
}
```

**PUT /blocklist/adult-filter** — body:
```json
{ "enabled": true }
```

---

### Devices

> `/devices/link/confirm` é **pública**. As demais exigem token.

| Método | Rota | Descrição | Autenticação |
|---|---|---|---|
| POST | `/devices/register` | Registrar dispositivo (device_name, platform) | JWT ou Device Token |
| GET | `/devices` | Listar dispositivos vinculados à conta | JWT ou Device Token |
| POST | `/devices/link/generate` | Gerar código de 6 dígitos (TTL 5 min) | **JWT apenas** (pai) |
| POST | `/devices/link/confirm` | Confirmar vinculação (code) e receber device_token | **Pública** |

**POST /devices/link/generate** — resposta:
```json
{
  "code": "123456",
  "expires_at": "2026-04-10T12:05:00Z"
}
```

**POST /devices/link/confirm** — body:
```json
{
  "code": "123456",
  "device_name": "Celular do João Jr.",
  "platform": "android"
}
```

**Response 200:**
```json
{
  "device_token": "dt_a1b2c3d4e5f6...",
  "device_id": "uuid-do-novo-device",
  "user_id": "uuid-do-user-pai",
  "parent_device_id": "uuid-do-device-pai"
}
```

**Erros possíveis:**
- `400` — código inválido, expirado, ou já utilizado
- `500` — falha ao gerar device token

> **Importante:** o `device_token` retornado deve ser salvo pelo app do filho em storage seguro (SQLCipher local ou secure storage do sistema). Não há endpoint para recuperá-lo depois — se for perdido, o filho precisa refazer a vinculação com um novo código.

---

## Fluxo completo de vinculação parental

```
┌─ Device do Pai ───────────────────┐   ┌─ Device do Filho ─────────────┐
│                                   │   │                               │
│ 1. Login Firebase (email/Google)  │   │                               │
│ 2. POST /auth/login → user_id     │   │                               │
│ 3. POST /devices/register         │   │                               │
│    → parent_device_id             │   │                               │
│ 4. POST /devices/link/generate    │   │                               │
│    → code "123456"                │   │                               │
│    exibe na tela                  │   │                               │
│                                   │   │ 5. Usuário digita "123456"    │
│                                   │   │ 6. POST /devices/link/confirm │
│                                   │   │    { code, device_name, ... } │
│                                   │   │    → device_token + user_id   │
│                                   │   │ 7. Guarda device_token        │
│                                   │   │ 8. GET /blocklist             │
│                                   │   │    Authorization: Bearer dt_…  │
│                                   │   │    → blocklist do pai         │
│ 9. POST /blocklist                │   │                               │
│    (adiciona youtube.com)         │   │                               │
│                                   │   │ 10. Polling GET /blocklist    │
│                                   │   │     → vê youtube.com          │
│                                   │   │     → atualiza DNS Proxy/VPN  │
└───────────────────────────────────┘   └───────────────────────────────┘
```
