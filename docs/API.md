# DopaBlocker - API REST

## Base URL

```
Development: http://localhost:3000
Production:  https://api.dopablocker.com (Azure)
```

## Autenticacao

Todas as rotas (exceto /auth/register e /auth/login) requerem header:
```
Authorization: Bearer <firebase_jwt_token>
```

## Endpoints

### Auth

| Metodo | Rota | Descricao |
|--------|------|-----------|
| POST | /auth/register | Criar conta (email, password, displayName) |
| POST | /auth/login | Login (email, password) -> token |
| GET | /auth/me | Dados do usuario autenticado |

### Blocklist

| Metodo | Rota | Descricao |
|--------|------|-----------|
| GET | /blocklist | Listar itens bloqueados do usuario |
| POST | /blocklist | Adicionar item (itemType, value) |
| DELETE | /blocklist/:id | Remover item |
| PUT | /blocklist/adult-filter | Toggle filtro adulto (enabled: bool) |

### Devices

| Metodo | Rota | Descricao |
|--------|------|-----------|
| POST | /devices/register | Registrar dispositivo (deviceName, platform) |
| GET | /devices | Listar dispositivos vinculados |
| POST | /devices/link/generate | Gerar codigo 6 digitos (TTL 5 min) |
| POST | /devices/link/confirm | Confirmar vinculacao (code) |
