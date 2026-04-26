# DopaBlocker — Auth State Machine (cross-platform)

Este documento é o **contrato** que define como qualquer cliente do DopaBlocker (desktop Svelte/Tauri ou mobile Flutter) deve gerenciar o estado de autenticação. O backend e os endpoints são compartilhados; a máquina de estados também precisa ser, porque qualquer divergência aparece como bug de borda em produção.

A implementação de referência vive em [desktop/src/lib/stores/auth.ts](../desktop/src/lib/stores/auth.ts). Quando o mobile for implementado em Riverpod, ele deve **espelhar** este contrato — só muda a sintaxe (sealed class Dart vs união discriminada TypeScript), não a semântica.

---

## 1. Os três fluxos de conta

O DopaBlocker tem três opções na tela inicial — mas só **dois modos** de conta no banco. A diferença é que "Filho" não cria conta:

| Opção UI | `User.mode` | Cria User Firebase? | Cria User local? | Credencial usada |
|---|---|---|---|---|
| **Pessoal** | `Personal` | Sim | Sim | Firebase JWT |
| **Pais** | `Parental` | Sim | Sim | Firebase JWT |
| **Filhos** | — | **Não** | **Não** | Device Token (`dt_<plain>`) |

No fluxo Filhos, o device é registrado sob o `user_id` do **pai** com `Device.is_child = true`. O backend gera um Device Token, que o cliente envia no header `Authorization: Bearer dt_<plain>`. Toda a lógica de auth depende dessa distinção.

---

## 2. Fases do `AuthState`

```
                  ┌──────────┐
                  │ booting  │  ← inicialização do app
                  └────┬─────┘
                       │ tenta restaurar do storage
            ┌──────────┴──────────────┐
            │                         │
            v                         v
   ┌────────────────┐         ┌──────────────────┐
   │ child_session  │         │   signed_out     │
   │  (filho)       │         │  (sem credencial)│
   └───┬────────────┘         └────────┬─────────┘
       │                               │ login Firebase / código de filho
       │ logout                        │
       │                               v
       │                      ┌────────────────────┐
       │                      │  authenticating    │
       │                      └────────┬───────────┘
       │                               │
       │           ┌───────────────────┼──────────────────────┐
       │           │                   │                      │
       │           v                   v                      v
       │   ┌──────────────┐    ┌─────────────┐      ┌──────────────────────┐
       │   │ authenticated│    │backend_     │      │pending_local_        │
       │   │  (Firebase)  │    │unavailable  │      │registration          │
       │   └──────┬───────┘    └──────┬──────┘      │  (Firebase OK,       │
       │          │ logout            │ retry       │   /auth/register     │
       │          │                   │             │   pendente)          │
       │          │                   │             └──────────┬───────────┘
       │          │                   v                        │
       │          │         (retry hydrateFromFirebase)         │
       │          │                                             v
       │          │                                  (após register/login OK)
       │          │                                             │
       └──────────┴─────────────────────────────────────────────┘
                                                                │
                                                                v
                                                         authenticated
```

### Tabela de transições

| De → Para | Evento que dispara |
|---|---|
| `booting → child_session` | `loadChildSession()` retorna sessão válida; `GET /blocklist` com `dt_` responde 200 |
| `booting → signed_out` | sem `child_session` no SQLCipher e Firebase responde `null` |
| `booting → authenticating` | Firebase responde com `User` (sessão persistida) |
| `signed_out → authenticating` | usuário clica "Entrar" (Firebase) ou "Confirmar código" (filho) |
| `authenticating → authenticated` | `POST /auth/login` retorna 200 |
| `authenticating → pending_local_registration` | `POST /auth/login` retorna 404 |
| `authenticating → backend_unavailable` | `POST /auth/login` falha (timeout, rede, 5xx) |
| `authenticating → child_session` | `POST /devices/link/confirm` retorna 200 + token persistido |
| `authenticating → signed_out` | Firebase signin/signup falha; ou `link/confirm` falha |
| `pending_local_registration → authenticated` | `POST /auth/register` retorna 200 |
| `backend_unavailable → authenticating` | usuário clica "Tentar novamente" (`retryBackendSync`) |
| `authenticated → signed_out` | `logout()` ou `DELETE /auth/me` (exclusão de conta) |
| `child_session → signed_out` | `logout()` (limpa SQLCipher), pai revogou (`401`), **OU polling do /child-blocked** detecta revogação a cada 30s |
| qualquer → `signed_out` | sessão Firebase expira/é invalidada |

### Invariantes

- `authenticated` ⇒ `user !== null && firebase_user !== null && child === null`
- `child_session` ⇒ `user === null && firebase_user === null && child !== null`
- `pending_local_registration` ⇒ `user === null && firebase_user !== null` (logado no Firebase, sem registro local)
- `child_session` **NUNCA** convive com Firebase. Se o filho fizer login no Firebase por engano, a máquina ignora os eventos do Firebase via `authSyncVersion`.
- `Authorization` header é determinado **exclusivamente** pelo `phase` corrente, via `AuthProvider` (ver §4).

---

## 3. Schema de armazenamento (cross-platform)

### Sessão Firebase (Pessoal/Pais)
- **Onde**: o próprio Firebase SDK persiste em IndexedDB (web) ou em arquivos próprios (Flutter/iOS/Android).
- **Não fazemos cache local** do JWT — ele é renovado automaticamente pelo SDK.

### Sessão de filho
- **Onde**: tabela `child_session` no SQLCipher local. Schema **idêntico** entre desktop (`desktop/src-tauri/migrations/002_child_session.sql`) e mobile (`mobile/lib/core/database_service.dart`):

```sql
CREATE TABLE IF NOT EXISTS child_session (
    id               INTEGER PRIMARY KEY CHECK (id = 1),  -- singleton
    user_id          TEXT NOT NULL,
    device_id        TEXT NOT NULL,
    device_token     TEXT NOT NULL,                       -- "dt_<plain>"
    parent_device_id TEXT NOT NULL,
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);
```

- **Por que singleton**: um device físico não pode estar vinculado a duas contas de pai simultaneamente. Trocar de pai = `clearChildSession()` + novo `confirmChildCode()`.
- **Por que texto puro**: o SQLCipher já cifra o arquivo `.db` inteiro com a chave do Credential Manager (Windows) / Keychain (macOS) / Keystore (Android). Cifrar de novo dentro seria dupla criptografia inútil.

---

## 4. Interface `AuthProvider`

A escolha "qual token mandar nesta request?" é abstraída por um `AuthProvider`. O cliente HTTP (`api.ts` / `api_client.dart`) não toca em Firebase nem em Device Token diretamente — ele pergunta ao provider.

### TypeScript (desktop)
[desktop/src/lib/services/auth-provider.ts](../desktop/src/lib/services/auth-provider.ts)

```ts
export interface AuthProvider {
    getAuthHeader(): Promise<string | null>; // "Bearer ..." ou null
    refresh(): Promise<boolean>;             // true = consegui refrescar; false = não tem como
}

export const firebaseAuthProvider: AuthProvider;        // usa getIdToken()
export function childAuthProvider(token: string): AuthProvider;
export const anonymousAuthProvider: AuthProvider;       // rotas públicas
```

### Dart (mobile — implementação esperada)
```dart
abstract class AuthProvider {
    Future<String?> getAuthHeader();
    Future<bool> refresh();
}

class FirebaseAuthProvider implements AuthProvider { ... }
class ChildAuthProvider implements AuthProvider {
    final String deviceToken;
    ChildAuthProvider(this.deviceToken);
    @override Future<String?> getAuthHeader() async => 'Bearer $deviceToken';
    @override Future<bool> refresh() async => false;
}
class AnonymousAuthProvider implements AuthProvider { ... }
```

### Roteamento por fase

| `phase` | Provider corrente |
|---|---|
| `booting`, `signed_out` | `anonymousAuthProvider` |
| `authenticating`, `authenticated`, `pending_local_registration`, `backend_unavailable` | `firebaseAuthProvider` |
| `child_session` | `childAuthProvider(snapshot.child.device_token)` |

A regra é aplicada uma vez, dentro do `commit()` do auth store, via função `syncAuthProvider`.

---

## 5. Regra do pai imune

Implementação em [shared/src/parental.rs](../shared/src/parental.rs):

```rust
pub fn effective_strategy(mode: BlockMode, is_child: bool) -> BlocklistStrategy {
    match (mode, is_child) {
        (BlockMode::Personal, _)        => BlocklistStrategy::ApplyAll,
        (BlockMode::Parental, true)     => BlocklistStrategy::ApplyAll,
        (BlockMode::Parental, false)    => BlocklistStrategy::Empty,  // pai imune
    }
}
```

### Quem aplica e quando

- **Desktop**: o frontend deriva `ParentalContext { mode, is_child }` do auth store e envia em **toda** chamada Tauri que afeta o engine (`set_blocking_enabled`, `cache_add_item`, `cache_remove_item`, `save_blocklist`). O Rust chama `effective_strategy` e decide entre carregar a lista cheia do SQLCipher ou passar uma lista vazia ao engine. Ver [desktop/src-tauri/src/commands.rs::effective_rules](../desktop/src-tauri/src/commands.rs).
- **Mobile**: o `blocking_provider.dart` (Riverpod) deriva o mesmo contexto e replica a decisão em Dart antes de chamar `BlockingChannel.updateBlocklist`. As 4 linhas da função são copiadas literalmente do Rust — não vale o custo de FFI.

### Por que a UI mostra a lista mesmo no pai imune
Pai precisa **gerenciar** a lista (ver, adicionar, remover) — ela apenas não se aplica ao próprio device dele. O cache local sempre tem a lista cheia; quem decide se aplica ou não é o engine.

---

## 6. Endpoints e quem chama

| Rota | Quem chama | Auth necessária |
|---|---|---|
| `POST /auth/email-code/start` | Pessoal/Pais durante signup | Pública |
| `POST /auth/email-code/verify` | Pessoal/Pais durante signup | Pública |
| `POST /auth/register` | Pessoal/Pais após Firebase signup | Firebase JWT (mas user pode não existir local ainda) |
| `POST /auth/login` | Pessoal/Pais a cada `hydrateFromFirebase` | Firebase JWT |
| `GET /auth/me` | Pessoal/Pais para validar sessão | Firebase JWT (user precisa existir) |
| `DELETE /auth/me` | Pessoal/Pais para excluir conta permanentemente (LGPD) | Firebase JWT (rejeita Device Token com 403) |
| `POST /devices/link/generate` | Pais (gera código de 6 dígitos) | Firebase JWT (rejeita Device Token com 403) |
| `POST /devices/link/confirm` | Filho (consome código, recebe token) | **Pública** (filho não tem credencial ainda) |
| `POST /devices/:id/revoke` | Pais (desvincula filho) | Firebase JWT (rejeita Device Token com 403) |
| `GET /blocklist`, `GET /devices` | Todos | JWT ou Device Token (read-only para token) |
| `POST/DELETE/PUT` em `/blocklist` | Pais e Pessoal | JWT (Device Token rejeitado com 403 pelo middleware) |

Detalhes em [docs/API.md](API.md). A regra "Device Token é read-only" é centralizada em [backend/src/middleware.rs::require_auth](../backend/src/middleware.rs) — qualquer rota nova herda automaticamente.

---

## 7. Smoke tests obrigatórios (golden path)

Quando alterar qualquer parte da auth state machine, validar manualmente:

### Fluxo de entrada (welcome → modo)

1. **Boot limpo**: app abre direto em `/welcome` com 3 cards (Pessoal, Pais, Filhos).
2. **Pessoal**: card Pessoal → `/login?mode=personal` → cadastra (envia código → digita código) → cai em `/` com `mode=personal`.
3. **Pais**: card Pais → `/login?mode=parental` → cadastra → cai em `/` com `mode=parental` → `/parental` aparece na sidebar com botão "Gerar código".
4. **Filhos**: card Filhos → `/onboarding/child` (sem login, só input de 6 dígitos) → digita código gerado por uma sessão Pais → cai em `/child-blocked`.
5. **Tela "Bloqueado"**: nenhum botão, nenhum link, sem sidebar. Logo + "Bloqueado" + texto explicativo + `v0.1.0` discreto no rodapé.

### Persistência e revogação

6. **Reload**: fechar e reabrir o app — `child_session` cai em `/child-blocked` direto; `authenticated` cai em `/`.
7. **Logout** em qualquer fase volta para `/welcome`.
8. **Pai revoga filho**: pai (em outra máquina) clica "Desvincular" → em até 30s o filho cai em `/welcome` (polling do `/child-blocked` detecta 401 e dispara `logout`).

### Regras de bloqueio

9. **Pai imune**: pai em modo parental ativa o bloqueio; `nslookup instagram.com` resolve normalmente (lista vazia no engine).
10. **Filho do mesmo pai**: bloqueio ativo; `nslookup instagram.com` retorna `127.0.0.1`.

### Exclusão de conta

11. **Excluir conta**: em `/settings` → botão "Excluir conta" → modal "Tem certeza?" → "Continuar" → input "Digite EXCLUIR" → "Excluir conta" → conta apagada (Firebase + backend) → app cai em `/welcome`.
12. **Sessão antiga (recent login)**: se o último login foi há muito tempo, ao tentar excluir aparece modal "Sessão antiga. Faça login de novo" → botão desloga e leva para `/welcome` → após relogin, repete o fluxo.
