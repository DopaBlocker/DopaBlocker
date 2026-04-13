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
| **Device Token** | `dt_` | Devices filhos (sem conta) | `Authorization: Bearer dt_<hash>` |

- O **Firebase JWT** é obtido pelo frontend via Firebase Auth SDK (login com email/senha ou Google). É renovado automaticamente pelo SDK.
- O **Device Token** é obtido uma única vez pelo device filho quando ele confirma o código de vinculação (`POST /devices/link/confirm`). Fica válido até o pai revogar manualmente.

Ambos resolvem para um `user_id` no backend (o id da conta do pai, no caso do device token).

### Rotas públicas (sem `Authorization`)

| Método | Rota | Descrição |
|---|---|---|
| POST | `/auth/register` | Criar conta Firebase + user local |
| POST | `/auth/login` | Login (frontend já autenticou no Firebase, sincroniza com backend) |
| POST | `/devices/link/confirm` | **Filho** confirma o código de vinculação e recebe um device token |

Todas as demais rotas exigem um token válido (Firebase JWT ou Device Token).

---

## Endpoints

### Auth

| Método | Rota | Descrição |
|---|---|---|
| POST | `/auth/register` | Criar conta (email, password, display_name, mode) |
| POST | `/auth/login` | Sincronizar user local com o Firebase user (body vazio, JWT no header) |
| GET | `/auth/me` | Dados do usuário autenticado |

**POST /auth/register** — body:
```json
{
  "email": "pai@example.com",
  "display_name": "João",
  "mode": "parental"
}
```
> A senha é cadastrada direto no Firebase pelo frontend, não pelo backend. Este endpoint só cria o registro local do usuário após o Firebase já ter o user criado.

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
