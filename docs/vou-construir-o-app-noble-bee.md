# DopaBlocker Desktop v0.1 — Plano de Conclusão

## Contexto

O projeto tem o backend (Axum + SQLCipher + Firebase JWT + Device Tokens) essencialmente pronto, com rotas de auth, blocklist e devices funcionais (ver [backend/src/main.rs](backend/src/main.rs)). A biblioteca compartilhada [shared/src/](shared/src/) tem models, bloom_filter e domain_matcher implementados. O que falta é **toda a camada desktop**:

- [desktop/src-tauri/src/](desktop/src-tauri/src/) — comandos IPC, SQLCipher local e engine de bloqueio (DNS + WFP + Adult Filter) estão como stubs-comentário.
- [desktop/src/](desktop/src/) — rotas SvelteKit existem mas os componentes, stores e services são apenas comentários.

O objetivo desta entrega é fechar o **modo Pessoal** do desktop v0.1:

1. Sistema de contas funcional (Firebase Auth email/senha + Google) integrado ao backend.
2. Blocklist CRUD via backend com cache local em SQLCipher.
3. Motor de bloqueio em duas camadas: DNS proxy local (127.0.0.1:53) + WFP para blindar DoH/IP direto.
4. Filtro adulto com Bloom Filter populado a partir da lista Steven Black/OISD.
5. UI SvelteKit completa: login/cadastro, dashboard, blocklist, settings.

Modo Parental, sync Firestore real-time e mobile ficam fora deste ciclo.

---

## Decisões de escopo

| Item | Decisão |
|------|---------|
| Bloqueio | DNS proxy local + WFP kernel-level |
| Auth providers | Email/senha + Google (Firebase) |
| Parental | Fora do v0.1 — UI mostra as 3 opções (Pessoal / Pais / Filhos), mas só "Pessoal" avança; "Pais" e "Filhos" ficam visíveis e clicáveis sem efeito (placeholder) |
| Adult filter | Incluído (lista baixada on-demand, cacheada) |
| Sync cloud | Apenas backend REST (Firestore polling fica para v0.2) |
| Admin elevation | App pede UAC na primeira execução (porta 53 + WFP + DNS do sistema) |

---

## Arquivos críticos a modificar

### Shared lib — verificação rápida
- [shared/src/bloom_filter.rs](shared/src/bloom_filter.rs) — confirmar `insert()` e `contains()` completos; adicionar método `len()` e serialização opcional.
- [shared/src/domain_matcher.rs](shared/src/domain_matcher.rs) — confirmar `is_domain_blocked()` com matching de subdomínio.
- Adicionar `#[cfg(test)] mod tests` cobrindo normalização, subdomínios e Bloom Filter.

### Desktop Tauri — backend Rust do desktop

[desktop/src-tauri/Cargo.toml](desktop/src-tauri/Cargo.toml) — adicionar:
```toml
tokio = { version = "1", features = ["full"] }
tokio-rusqlite = "0.6"
rusqlite = { version = "0.32", features = ["bundled-sqlcipher"] }
hickory-server = "0.24"      # DNS proxy
hickory-proto  = "0.24"
hickory-resolver = "0.24"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
uuid = { version = "1", features = ["v4"] }
thiserror = "1"
dopablocker-shared = { path = "../../shared" }
windows = { version = "0.58", features = [
    "Win32_Foundation", "Win32_NetworkManagement_WindowsFilteringPlatform",
    "Win32_Security", "Win32_System_Registry", "Win32_NetworkManagement_IpHelper"
] }
```

[desktop/src-tauri/tauri.conf.json](desktop/src-tauri/tauri.conf.json) — habilitar `windows.requireAdministrator = true` no bundle, setar `identifier` e permissões CSP.

[desktop/src-tauri/src/db.rs](desktop/src-tauri/src/db.rs):
- `init(app_handle) -> Result<Connection>` — resolve `app_data_dir`, abre SQLCipher, executa `PRAGMA key` (chave derivada de `machine-id + user-salt` armazenada no Windows Credential Store), roda migrations locais (tabela `blocked_items_cache`, `blocking_state`, `adult_filter_cache`).
- CRUD: `list_blocked_items`, `upsert_blocked_item`, `delete_blocked_item`, `set_blocking_enabled`, `get_blocking_enabled`, `set_adult_filter_enabled`.

[desktop/src-tauri/src/commands.rs](desktop/src-tauri/src/commands.rs) — `#[tauri::command]` handlers que validam JWT do frontend e delegam a services:
- `get_blocklist() -> Vec<BlockedItem>`
- `add_blocked_item(item_type, value) -> BlockedItem`
- `remove_blocked_item(id)`
- `toggle_blocking(enabled)` — chama `engine::start/stop`
- `toggle_adult_filter(enabled)` — chama `adult_filter::enable/disable`
- `get_blocking_status() -> { enabled, adult_filter_enabled, item_count }`
- `sync_with_backend(jwt)` — puxa blocklist do backend e reconcilia com cache local
- `get_app_version()`
Cada comando recebe `State<AppState>` com `Arc<Mutex<Engine>>` e `Connection`.

[desktop/src-tauri/src/lib.rs](desktop/src-tauri/src/lib.rs) — no `setup`:
- Inicializar DB (`db::init`) e armazenar como `state`.
- Instanciar `Engine` em estado parado; `app.manage(engine)`.
- Registrar todos os comandos via `invoke_handler(tauri::generate_handler![...])`.
- Registrar plugin `tauri-plugin-single-instance` (evita dois DNS proxies simultâneos).

[desktop/src-tauri/src/blocking/engine.rs](desktop/src-tauri/src/blocking/engine.rs):
- Struct `Engine { dns: Option<DnsProxy>, wfp: Option<WfpFilter>, adult: AdultFilter, rules: Arc<RwLock<HashSet<String>>> }`.
- `start(rules)` — popula `rules`, sobe `DnsProxy::spawn(rules.clone(), adult.clone())`, sobe `WfpFilter::install(rules.clone())`, troca DNS do sistema para `127.0.0.1` via `netsh interface ipv4 set dns ...`, salva DNS anterior em registry para restore.
- `stop()` — mata DNS proxy, remove filtros WFP, restaura DNS original.
- `update_rules(new_rules)` — hot-reload sem parar o engine.

[desktop/src-tauri/src/blocking/dns_proxy.rs](desktop/src-tauri/src/blocking/dns_proxy.rs):
- Servidor DNS baseado em `hickory-server` em `127.0.0.1:53` (UDP + TCP).
- `RequestHandler`: para cada query, normaliza domínio, consulta `rules` (HashSet) e `adult_filter.contains()`; se bloqueado, responde `NXDOMAIN`; senão forwarda via `hickory-resolver` para `1.1.1.1`/`8.8.8.8`.
- Logging estruturado de hits/misses (tracing).

[desktop/src-tauri/src/blocking/wfp.rs](desktop/src-tauri/src/blocking/wfp.rs):
- Wrapper sobre `windows::Win32::NetworkManagement::WindowsFilteringPlatform` (`FwpmEngineOpen`, `FwpmFilterAdd`).
- `install(rules)` — adiciona filtros UDP/TCP bloqueando portas 53 (exceto para nosso proxy) e 853 (DoT). Para DoH, bloqueia conexões TCP/443 para IPs de resolvers conhecidos (Cloudflare 1.1.1.1, Google 8.8.8.8, etc.) quando o hostname corresponde via SNI — v0.1 usa lista estática de IPs DoH e um provider de SNI inspection básico.
- `uninstall()` — remove todos os filtros instalados pelo app (identificados por `providerKey` único).
- Implementar com cuidado: engine handle deve ser `FwpmEngineClose`-d em Drop.

[desktop/src-tauri/src/blocking/adult_filter.rs](desktop/src-tauri/src/blocking/adult_filter.rs):
- `AdultFilter { bloom: BloomFilter, enabled: bool }`.
- `fetch_and_build(cache_dir)` — se `cache_dir/adult_list.txt` é mais velho que 7 dias, baixa de `https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/porn/hosts` via `reqwest`. Parse linhas `0.0.0.0 dominio.com`, insere no Bloom Filter com `expected_items=100_000`, `fp_rate=0.001`. Serializa o estado do filtro para `cache_dir/adult_filter.bin` (bincode) para próximas inicializações.
- `contains(domain)` delega para `BloomFilter::contains` do shared.

### Frontend SvelteKit

[desktop/src/lib/services/firebase.ts](desktop/src/lib/services/firebase.ts):
- Ler config de `VITE_FIREBASE_*` env vars; `initializeApp`, `getAuth`.
- Exportar: `signInEmail(email, pass)`, `signUpEmail(email, pass, displayName)`, `signInGoogle()` (usa `signInWithPopup(new GoogleAuthProvider())` — no Tauri 2, popup funciona dentro da webview; se falhar, fallback para `signInWithRedirect`).
- `onAuthChange(cb)` wrap de `onAuthStateChanged`.
- `getIdToken(force = false)` → sempre `await user.getIdToken(force)` antes de cada request (resolve refresh automático).

[desktop/src/lib/services/api.ts](desktop/src/lib/services/api.ts):
- Client HTTP tipado com `baseUrl = import.meta.env.VITE_API_URL || 'http://localhost:3000'`.
- `request<T>(method, path, body?)` injeta `Authorization: Bearer ${await getIdToken()}`, trata 401 com um retry após `getIdToken(true)`, deserializa JSON ou lança `ApiError`.
- Métodos tipados: `register(payload)`, `login(payload)`, `me()`, `listBlocklist()`, `createBlockedItem(payload)`, `deleteBlockedItem(id)`, `toggleAdultFilter(enabled)`.
- Tipos derivados de [backend/src/models.rs](backend/src/models.rs) e [shared/src/models.rs](shared/src/models.rs) — definir em [desktop/src/lib/types.ts](desktop/src/lib/types.ts) espelhando os DTOs.

[desktop/src/lib/services/tauri-bridge.ts](desktop/src/lib/services/tauri-bridge.ts):
- Wrappers tipados de `invoke('get_blocklist')`, `invoke('add_blocked_item', ...)`, etc.
- `getBlockingStatus()`, `toggleBlocking(enabled)`, `toggleAdultFilter(enabled)`, `syncWithBackend(jwt)`.

[desktop/src/lib/stores/auth.ts](desktop/src/lib/stores/auth.ts):
- Store Svelte 5 (rune-based ou `writable` — usar `writable` por compatibilidade).
- Estado `{ user: User|null, loading: boolean, error: string|null }`.
- `initAuth()` — chamado no `+layout.ts`, registra `onAuthChange`; ao autenticar, chama backend `/auth/login` para obter `User` local e guarda; ao deslogar, zera.
- `login`, `loginGoogle`, `register`, `logout` delegam a `firebase.ts` e depois `api.register`/`api.login`.

[desktop/src/lib/stores/blocking.ts](desktop/src/lib/stores/blocking.ts):
- Estado `{ items: BlockedItem[], enabled: bool, adultFilterEnabled: bool, loading: bool }`.
- `loadBlocklist()` — backend + `syncWithBackend` no Tauri.
- `addItem(type, value)`, `removeItem(id)` — otimista, com rollback em erro.
- `toggleBlocking()`, `toggleAdultFilter()` — chamam tauri-bridge.

[desktop/src/routes/+layout.ts](desktop/src/routes/+layout.ts):
- `export const ssr = false; export const prerender = false;` (SvelteKit rodando no Tauri webview, sem servidor).
- Inicializa `authStore.init()` e aguarda o primeiro estado.

[desktop/src/routes/+layout.svelte](desktop/src/routes/+layout.svelte):
- Guard: se `!$authStore.user && route !== '/login'` → `goto('/login')`.
- Sidebar com links Dashboard / Bloqueios / Configurações + botão logout.

[desktop/src/routes/+page.svelte](desktop/src/routes/+page.svelte):
- Dashboard: card de status (on/off com toggle), contador de itens, botão grande "Ativar Bloqueio".

[desktop/src/routes/login/+page.svelte](desktop/src/routes/login/+page.svelte) + [LoginForm.svelte](desktop/src/lib/components/LoginForm.svelte):
- Tabs: Entrar / Cadastrar. Inputs email/senha, botão "Entrar com Google", tratamento de erro.
- Na tela de **cadastro** (e/ou num passo de onboarding pós-primeiro-login), o usuário escolhe o modo através de [ModeSelector.svelte](desktop/src/lib/components/ModeSelector.svelte) com três cards: **Pessoal**, **Pais**, **Filhos**.
  - "Pessoal" → submete `mode: "personal"` ao backend `/auth/register` e segue o fluxo normal.
  - "Pais" e "Filhos" → ficam visíveis e clicáveis, mas ao clicar exibem apenas um toast/badge "Em breve — disponível na v0.2" e não avançam. Nenhuma chamada de API, nenhuma mudança de estado.
  - Implementação: `on:click` em cada card verifica `mode === 'personal'` antes de prosseguir; caso contrário dispara `showToast('Em breve')` e retorna.

[desktop/src/routes/blocking/+page.svelte](desktop/src/routes/blocking/+page.svelte) + [BlockList.svelte](desktop/src/lib/components/BlockList.svelte) + [AddBlockModal.svelte](desktop/src/lib/components/AddBlockModal.svelte):
- Tabela (tipo, valor, data, ação remover), botão "Adicionar" abre modal (radio site/app + input + validação com `normalize_domain` via tauri-bridge).

[desktop/src/routes/settings/+page.svelte](desktop/src/routes/settings/+page.svelte):
- Toggles: ligar/desligar bloqueio, ligar/desligar filtro adulto. Info de conta. Logout.

[desktop/src/routes/parental/+page.svelte](desktop/src/routes/parental/+page.svelte):
- Placeholder "Em breve — v0.2" para não quebrar nav. Acessível apenas se o usuário estiver em modo Pessoal curioso — não é navegação principal.

### Configuração

- [desktop/.env.example](desktop/.env.example) novo arquivo com `VITE_FIREBASE_API_KEY`, `VITE_FIREBASE_AUTH_DOMAIN`, `VITE_FIREBASE_PROJECT_ID`, `VITE_FIREBASE_APP_ID`, `VITE_API_URL`.
- Instalar pacote npm `firebase` no [desktop/package.json](desktop/package.json) (`pnpm add firebase`).
- Instalar `@tauri-apps/api` e `@tauri-apps/plugin-log` (deve já estar implícito pelo scaffold — conferir).

---

## Passos de execução (ordem sugerida)

1. **Shared lib — testes e verificação** (meio dia)
   Rodar `cargo test -p dopablocker-shared`; adicionar testes faltantes; fix se necessário.

2. **Desktop Tauri — DB local** (1 dia)
   Implementar `db.rs` (init SQLCipher + CRUD + chave via Windows Credential Store via crate `keyring`).

3. **Desktop Tauri — comandos sem bloqueio** (1 dia)
   Implementar `commands.rs` para blocklist CRUD local e `sync_with_backend`. Registrar no `lib.rs`. Validar com `cargo tauri dev` chamando de um botão teste.

4. **Frontend — auth e api** (1-2 dias)
   `firebase.ts`, `api.ts`, `auth.ts` store, tela de login funcional com conta real criada via backend `/auth/register`. Guard de rota.

5. **Frontend — blocklist UI** (1 dia)
   `blocking.ts` store, tela `/blocking` com add/remove funcional (via backend + Tauri cache).

6. **Desktop Tauri — DNS proxy** (2 dias)
   `blocking/dns_proxy.rs` com hickory; testar com `nslookup instagram.com 127.0.0.1` retornando NXDOMAIN quando bloqueado.

7. **Desktop Tauri — engine + integração de DNS do sistema** (1 dia)
   `engine.rs` start/stop; troca DNS via `netsh`; restauração no shutdown. Toggle na UI.

8. **Desktop Tauri — Adult filter** (1 dia)
   Download da lista, popular Bloom Filter, integrar ao DNS proxy. Toggle UI.

9. **Desktop Tauri — WFP** (2-3 dias)
   `blocking/wfp.rs`. Primeiro bloqueio de porta 53 não autorizada (força uso do proxy). Depois bloqueio de DoT (853). Depois inspeção SNI/IPs DoH conhecidos.

10. **Polimento** (1 dia)
    Mensagens de UAC, tela de primeira execução explicando que requer admin, ícone/bundle, testes manuais de golden path.

**Estimativa total:** ~11-13 dias de trabalho focado.

---

## Verificação end-to-end

### Golden path manual
1. `cargo run -p dopablocker-backend` — backend sobe em `:3000`.
2. `pnpm --filter desktop tauri dev` — app abre, pede UAC.
3. Tela de login aparece. No cadastro, os três cards de modo aparecem (Pessoal/Pais/Filhos); clicar em "Pais" ou "Filhos" mostra toast "Em breve" e não avança. Clicar em "Pessoal" + email+senha → backend cria `User`, frontend recebe JWT.
4. Adicionar `instagram.com` na blocklist. Botão "Ativar Bloqueio".
5. Abrir browser, acessar `https://instagram.com` → falha de DNS. `nslookup instagram.com` retorna NXDOMAIN.
6. Tentar trocar DNS manualmente no Windows para `1.1.1.1` → WFP derruba a conexão (valida kernel-level).
7. Ligar filtro adulto → `nslookup` em domínio conhecido da lista Steven Black também retorna NXDOMAIN.
8. "Desativar Bloqueio" → DNS do sistema volta ao anterior; `nslookup instagram.com` volta a resolver.
9. Fechar e reabrir o app → blocklist persiste (cache local + sync com backend).
10. Logout → redireciona para /login; relogin com Google funciona.

### Testes automatizados
- `cargo test --workspace` — unitários do shared + backend + desktop (partes puras).
- `pnpm --filter desktop check` — `svelte-check` sem erros.
- Smoke test dos comandos Tauri via `#[cfg(test)]` com `tauri::test::mock_builder`.

### O que NÃO está coberto nesta entrega
- Mobile (Flutter) — intocado.
- Firestore real-time sync — fica em polling manual via "sync" button para dev.
- Rotação de device tokens / revogação UI.
- Múltiplas blocklists por device-filho (requer Parental).
- macOS/Linux — só Windows nesta fase (WFP é Windows-only).

---

## Riscos e mitigações

| Risco | Mitigação |
|-------|-----------|
| WFP mais complexo que estimado | Passo 9 é o último — se atrasar, release sem WFP e adicionar em v0.1.1. DNS proxy sozinho já resolve ~80% dos casos. |
| `signInWithPopup` do Firebase falha em Tauri webview | Fallback para `signInWithRedirect` ou fluxo customizado via `@tauri-apps/plugin-shell::open` para OAuth URL + deeplink de callback. |
| Troca de DNS quebra se app crashar sem limpar | Salvar DNS original em registry ao iniciar engine; ao subir, verificar se há `previous_dns` pendente e restaurar antes de trocar de novo. |
| Porta 53 ocupada | Tentativa de bind com erro claro pedindo ao usuário desabilitar o DNS Client service temporariamente. |
| Chave SQLCipher no disco | Armazenar via crate `keyring` (Windows Credential Manager) — nunca em arquivo texto. |
