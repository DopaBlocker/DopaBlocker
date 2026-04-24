# Plano: Página de bloqueio universal (HTTP + HTTPS)

## Context

Hoje o DopaBlocker só serve a página de bloqueio em **HTTP (porta 80)**. Como praticamente todos os sites modernos forçam HTTPS via HSTS (Instagram, Twitter, Facebook, Google…), o browser tenta TCP em `127.0.0.1:443`, onde nada escuta, e mostra `ERR_CONNECTION_REFUSED` em vez da nossa página bonita. O screenshot do usuário evidencia: `xvideos.com` (HTTP) mostra a página; `instagram.com` (HTTPS) mostra erro do browser.

Isso é o gap 🔴 C6 documentado em [docs/GAPS.md:24](docs/GAPS.md#L24). O objetivo aqui é fechá-lo: **qualquer site bloqueado — por lista manual ou pelo filtro adulto — deve mostrar a página do DopaBlocker**, com o domínio e a origem do bloqueio.

O DNS proxy já redireciona o domínio pra `127.0.0.1` ([dns_proxy.rs:344-373](desktop/src-tauri/src/blocking/dns_proxy.rs#L344-L373)), então o caminho de rede está resolvido. Falta um servidor TLS em `127.0.0.1:443` que o browser confie — caminho padrão: **CA local auto-instalada + certificados leaf dinâmicos por SNI** (mesma técnica do Little Snitch, Fiddler, Charles Proxy, Pi-hole-with-block-page).

Decisões já tomadas com o usuário:
- CA instalada em `LocalMachine\Root` (app já roda como admin pro WFP).
- Firefox (NSS) fica como limitação conhecida documentada — escopo Chrome/Edge/Brave.
- Página mostra **domínio bloqueado + razão** (Lista pessoal / Filtro adulto).

---

## Arquivos a modificar

**Criar:**
- [desktop/src-tauri/src/blocking/ca.rs](desktop/src-tauri/src/blocking/ca.rs) — geração, persistência e install da CA + assinatura de leafs
- [desktop/src-tauri/src/blocking/tls_resolver.rs](desktop/src-tauri/src/blocking/tls_resolver.rs) — `ResolvesServerCert` do rustls com cache por SNI
- [desktop/src-tauri/src/blocking/block_reason.rs](desktop/src-tauri/src/blocking/block_reason.rs) — enum `BlockReason` + função `check()` compartilhada

**Modificar:**
- [desktop/src-tauri/Cargo.toml](desktop/src-tauri/Cargo.toml) — deps `rcgen`, `rustls`, `tokio-rustls`
- [desktop/src-tauri/src/blocking/mod.rs](desktop/src-tauri/src/blocking/mod.rs) — registrar módulos novos
- [desktop/src-tauri/src/blocking/block_page.rs](desktop/src-tauri/src/blocking/block_page.rs) — extrair `render_page(domain, reason)`, adicionar `run_https(...)`
- [desktop/src-tauri/src/blocking/block_page.html](desktop/src-tauri/src/blocking/block_page.html) — placeholders `{{DOMAIN}}` e `{{REASON_TEXT}}`
- [desktop/src-tauri/src/blocking/dns_proxy.rs](desktop/src-tauri/src/blocking/dns_proxy.rs) — usar `block_reason::check()` em vez do `is_blocked`+`adult_blocks` inline
- [desktop/src-tauri/src/blocking/engine.rs](desktop/src-tauri/src/blocking/engine.rs) — inicializar CA + instalar + subir HTTPS
- [desktop/src-tauri/src/lib.rs](desktop/src-tauri/src/lib.rs) — expor `install_ca_root` command (retry manual)
- [desktop/src-tauri/src/commands.rs](desktop/src-tauri/src/commands.rs) — command wrapper
- [docs/GAPS.md](docs/GAPS.md) — marcar C6 como fechado (parcial); adicionar Firefox + pinning Chromium
- [docs/GOLDEN_PATH.md](docs/GOLDEN_PATH.md) — passos de validação HTTPS

---

## Implementação

### 1. Dependências
Em [Cargo.toml:31](desktop/src-tauri/Cargo.toml#L31):
```toml
rcgen         = { version = "0.13", default-features = false, features = ["crypto", "pem", "ring"] }
rustls        = { version = "0.23", default-features = false, features = ["std", "ring"] }
tokio-rustls  = { version = "0.26", default-features = false, features = ["ring"] }
```
`reqwest` já usa `rustls-tls` (ok, compatível).

### 2. Helper compartilhado — `block_reason.rs`
```rust
pub enum BlockReason { UserList, AdultFilter }
pub async fn check(domain: &str, rules: &Arc<RwLock<HashSet<String>>>,
                   adult: &Arc<AdultFilter>) -> Option<BlockReason>
```
- Reusa a lógica atual de [dns_proxy.rs:293-321](desktop/src-tauri/src/blocking/dns_proxy.rs#L293-L321) (`is_blocked` + `adult_blocks`), mas retornando a origem.
- `dns_proxy::handle_query_bytes()` passa a chamar `check(...)` → se `Some(_)`, chama `build_block_redirect()` (comportamento idêntico). Regressão zero.
- O HTTPS/HTTP server usa a mesma função no momento de renderizar a página → decide `REASON_TEXT`.

### 3. CA — `ca.rs`
- `struct LocalCa { cert_der: Vec<u8>, key_pair: rcgen::KeyPair, cert: rcgen::Certificate }`.
- `LocalCa::load_or_create(app_data_dir) -> Result<Self>`:
  - Procura `ca.pem` + `ca.key` no dir do app (mesmo dir do SQLCipher).
  - Se faltar: gera ECDSA P-256, CN `"DopaBlocker Local CA"`, validade 10 anos, `isCA=true`, `BasicConstraints`, `KeyUsage=keyCertSign|cRLSign`. Persiste no disco (perm `600` via `fs::set_permissions` + ACL Windows — NT AUTHORITY\SYSTEM + admin only).
- `LocalCa::install_in_windows_root(&self) -> Result<InstallStatus>`:
  - Computa thumbprint SHA-1 do DER.
  - Checa via `certutil -store Root <thumbprint>` (exit 0 = já instalado). Short-circuit.
  - Instala: escreve DER em `%TEMP%\dopablocker_ca.cer`, roda `certutil -addstore Root <path>`. Como o app já sobe elevado (WFP exige), UAC extra não aparece.
  - Retorna `Installed | AlreadyPresent | Failed(reason)`. Falha não aborta o engine — apenas loga; HTTPS ainda sobe, só que browsers mostram aviso/erro até user instalar manual.
- `LocalCa::sign_leaf(hostname: &str) -> Result<CertifiedKey>`:
  - Leaf ECDSA P-256, CN = hostname, SAN `DNS:hostname` e `DNS:*.hostname` se aplicável, validade 30 dias, assinado pela CA.
  - Retorna estrutura rustls pronta (`CertifiedKey { cert_chain: [leaf.der], key: PrivateKeyDer }`).

### 4. Resolver TLS — `tls_resolver.rs`
```rust
pub struct SniCertResolver {
    ca: Arc<LocalCa>,
    cache: Mutex<HashMap<String, Arc<CertifiedKey>>>,
}
impl rustls::server::ResolvesServerCert for SniCertResolver { ... }
```
- Extrai SNI de `ClientHello::server_name()`. Sem SNI → `None` (browser aborta, ok).
- Cache hit → devolve `Arc`. Miss → `ca.sign_leaf(sni)` e insere.
- Capacidade do cache: simples `HashMap` (limite implícito pelo número de domínios bloqueados que o user visita; dezenas/centenas no máximo). Eviction fica pra depois se virar problema.

### 5. Block page — `block_page.rs`
- Extrair função pura `fn render_page(domain: Option<&str>, reason: Option<BlockReason>) -> String` que preenche `{{DOMAIN}}`, `{{REASON_TEXT}}`, `{{QUOTE}}` no template.
- Mantem `run()` atual (HTTP/80) — passa a extrair `Host:` do request e chamar `render_page(host, block_reason::check(host).await)`.
- Adicionar `run_https(rules, adult, ca) -> JoinHandle`:
  - `ServerConfig::builder().with_no_client_auth().with_cert_resolver(Arc::new(SniCertResolver::new(ca)))`.
  - `TcpListener::bind("127.0.0.1:443")` + loop `accept()`.
  - Para cada conexão: `TlsAcceptor::accept(tcp)` → read primeiro request line/headers → extrai `Host:` → renderiza → responde `HTTP/1.1 200 OK` com o HTML.
  - Se bind falha (outro processo em 443): loga erro claro com hint, HTTPS fica off, HTTP continua funcionando.
- Versão IPv6: também bindar `[::1]:443` quando C2 (IPv6 DNS) for atacado. **Fora do escopo aqui** — C6 é HTTP-vs-HTTPS, IPv6 é C1/C2/C3.

### 6. Template — `block_page.html`
Inserir antes do blockquote:
```html
<p class="domain">{{DOMAIN}}</p>
<p class="reason">{{REASON_TEXT}}</p>
```
Mapeamento de razão:
- `UserList` → "Na sua lista de bloqueios"
- `AdultFilter` → "Filtro de conteúdo adulto"
- `None` (fallback, ex: browser bateu em 127.0.0.1 via HSTS sem SNI blockando) → "Este site está bloqueado pelo DopaBlocker"

### 7. Engine — ordem de start ([engine.rs:61-115](desktop/src-tauri/src/blocking/engine.rs#L61-L115))
```
1. LocalCa::load_or_create (gera se preciso)
2. LocalCa::install_in_windows_root (idempotente; falha → warn, segue)
3. block_page HTTP :80  (existente)
4. block_page HTTPS :443 (novo; com rules+adult+ca refs)
5. dns_proxy :53 (existente)
6. WFP (existente)
```
`stop()` desmonta na ordem inversa, com `shutdown` channel pros dois servers de block page.

### 8. Command Tauri
`install_ca_root()` em [commands.rs](desktop/src-tauri/src/commands.rs) — chama `LocalCa::install_in_windows_root()` e retorna status. Útil pra botão "Reinstalar certificado" na tela de settings (UI em follow-up, fora deste plano).

### 9. Persistência
Flags opcionais em `app_data_dir/persistence.json`:
- `ca_thumbprint: String` — pra mostrar ao user qual CA está ativa
- `ca_installed_at: Timestamp`

---

## Verificação (end-to-end)

### Teste unitário (cargo test)
- `ca.rs`: gera CA, assina leaf pra `example.com`, verifica cadeia com `rustls::server::ServerConfig` dummy.
- `tls_resolver.rs`: cache hit na segunda chamada com mesmo SNI; miss gera novo `CertifiedKey`.
- `block_reason.rs`: domínio só em rules → `UserList`; só no adulto → `AdultFilter`; em nenhum → `None`; em ambos → `UserList` (prioridade da lista manual, igual hoje).
- Não quebrar os 18 testes existentes (`cargo test --workspace`).

### Golden path manual (atualizar [docs/GOLDEN_PATH.md](docs/GOLDEN_PATH.md))
Novos passos após "9. Ativar bloqueio":
```
### 9.1 Instalação da CA
- [ ] Primeiro ativar: log "CA raiz instalada (thumbprint=XXX)" ou "CA já presente"
- [ ] `certutil -store Root` | findstr DopaBlocker → mostra o cert

### 9.2 Bloqueio HTTPS
- [ ] https://instagram.com no Chrome/Edge/Brave → página bonita do DopaBlocker
- [ ] Página mostra "instagram.com" + "Na sua lista de bloqueios"
- [ ] Barra de URL não mostra aviso de cert (vermelho)

### 9.3 HTTPS via filtro adulto
- [ ] Toggle filtro adulto ON
- [ ] https://pornhub.com → página do DopaBlocker
- [ ] Razão exibida: "Filtro de conteúdo adulto"

### 9.4 HTTP ainda funciona
- [ ] http://xvideos.com → página bonita (regressão da v0.1)

### 9.5 Firefox (limitação conhecida)
- [ ] Firefox → ERR_CONNECTION_REFUSED (documentado, fora do escopo)
```

### Checks adicionais
- `nslookup instagram.com` devolve `127.0.0.1` (inalterado).
- `netstat -an | findstr :443` mostra `127.0.0.1:443 LISTENING` quando engine ativo.
- Parar engine → bind de 443 solto; `curl https://127.0.0.1` recusa.
- Reativar → bind volta sem warm-up extra.

### Update [docs/GAPS.md](docs/GAPS.md)
- **C6**: mover pra seção "histórico de cortes fechados" com nota "parcial — Firefox ainda aberto".
- **Novo gap 🟠**: "Firefox não confia na CA do DopaBlocker (usa NSS); página HTTPS só aparece em browsers baseados em Chromium. Fix: `certutil` do NSS por perfil detectado."
- **Novo gap 🟡**: "Domínios com built-in pinning do Chromium (Google properties, alguns do Facebook) ainda mostram erro de cert mesmo com CA user-installed. Lista é pequena — maioria dos sites afetados são raros."
- **Atualizar H12**: a chave privada da CA no disco herda a mesma superfície — reforçar mitigação via DPAPI/ACL.

---

## Ordem de implementação sugerida

1. Bump Cargo.toml + `cargo check` pra garantir compile das deps novas
2. `block_reason.rs` + refactor do `dns_proxy.rs` — roda todos os testes, garante regressão zero
3. `ca.rs` com testes unitários isolados (geração/assinatura apenas, sem mexer em store real)
4. `tls_resolver.rs` com teste unitário de cache
5. Install no Windows store — testar idempotência manualmente (rodar 2x, checar via certutil)
6. `block_page.rs`: extrair `render_page`, atualizar HTTP pra usar reason; adicionar `run_https`
7. Atualizar `block_page.html`
8. Integrar em `engine.rs`: ordem de start/stop
9. Command Tauri `install_ca_root` (thin wrapper)
10. Golden path manual E2E em Chrome, Edge, Brave
11. Atualizar docs (GAPS + GOLDEN_PATH)

---

## Riscos / trade-offs

- **Bind 443 em conflito**: se IIS/dev server já escuta em 127.0.0.1:443, bind falha. Log explícito com comando de diagnóstico (`netstat -ano | findstr :443`), HTTPS fica off mas HTTP e DNS continuam.
- **Chromium pinning**: ~10-20 domínios com built-in pins (Google, alguns Facebook) vão mostrar cert error em vez da nossa página. Fora do alcance de qualquer CA local — limitação inerente.
- **Chave privada da CA no disco**: malware rodando como o user tem acesso → pode emitir certs válidos pro sistema. Mitigação parcial via ACL; DPAPI é upgrade futuro (alinha com GAPS H12).
- **Primeira instalação da CA**: depende do app estar elevado (já é o caso em uso normal — WFP exige). Se rodar como user comum, install falha graciosamente, HTTPS sobe sem confiança, user vê aviso de cert no browser (em vez da página) — degradação aceitável vs estado atual (ERR_CONNECTION_REFUSED).
- **Escopo IPv6**: HTTPS em `[::1]:443` fica de fora — hoje o DNS proxy só bindas `127.0.0.1:53` e AAAA volta vazio, então o cliente sempre cai em IPv4. Quando C2 for atacado, bindar `[::1]:443` também fica trivial.
