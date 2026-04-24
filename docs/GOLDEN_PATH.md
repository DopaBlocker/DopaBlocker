# DopaBlocker — Golden Path (v0.1)

Roteiro manual de validação end-to-end. Execute do zero (DB do desktop apagado,
backend limpo) para garantir que cada etapa do plano de construção continua
funcionando. Automatização E2E (Playwright + Tauri WebDriver) é um gap
planejado — ver [GAPS.md](GAPS.md) item O1.

---

## Pré-requisitos

- Windows 10/11 (x64)
- Rust toolchain + `pnpm` instalados
- `.env` configurados em `desktop/` e `backend/` (ver [README.md](../README.md))
- Terminal **como Administrador** para os passos que envolvem porta 53, WFP
  e `netsh` (se rodar como usuário comum, vai ver "executar como
  administrador" nos toasts — as etapas 1–4 e 9 ainda funcionam)

## Limpeza inicial (opcional, para teste do zero)

```powershell
# Remove cache local do desktop
Remove-Item "$env:APPDATA\app.dopablocker.desktop" -Recurse -ErrorAction SilentlyContinue
# Remove DB do backend (apaga usuários locais)
Remove-Item ".\backend\dopablocker.db" -ErrorAction SilentlyContinue
# Limpa localStorage do onboarding — é per-user, basta abrir DevTools e apagar
```

## Passos

### 1. Subir backend
```powershell
cd backend
cargo run
```
**Esperado:**
```
Starting DopaBlocker Backend...
Migration aplicada migration="001_initial"
Migration aplicada migration="002_parental_fixes"
Listening on 0.0.0.0:3000
```
- [ ] Backend em `:3000` sem erro.

### 2. Subir desktop
Em outro terminal (admin):
```powershell
pnpm tauri:dev
```
**Esperado:**
- [ ] Compila sem erro (primeira vez: ~2min)
- [ ] Janela abre com tela de login centralizada
- [ ] Logo mark com gradiente azul→roxo visível

### 3. Cadastro
- [ ] Aba "Cadastrar" exibe os 3 cards (Pessoal, Pais, Filhos)
- [ ] Cards "Pais" e "Filhos" têm badge "Em breve"
- [ ] Clicar em "Pais" ou "Filhos" com form vazio mostra banner "Esse modo chega na v0.2"
- [ ] Preencher nome, email, senha e clicar "Pessoal" → cria conta e redireciona para dashboard
- [ ] No backend, log `INFO User created ...` aparece

### 4. Onboarding de primeira execução
- [ ] Modal "Bem-vindo ao DopaBlocker" aparece automaticamente pós-cadastro
- [ ] Lista os 4 pontos: admin, duas camadas, dados no disco, Firebase mínimo
- [ ] Clicar "Entendi, vamos começar" fecha o modal
- [ ] Reload (Ctrl+R na janela dev) — modal **não** reaparece (localStorage lembrou)

### 5. Sidebar e navegação
- [ ] Sidebar à esquerda, 240px, com logo + 3 links
- [ ] Link ativo tem barra azul vertical à esquerda
- [ ] Card do usuário embaixo mostra nome + email
- [ ] Botão "Sair" no rodapé da sidebar

### 6. Dashboard
- [ ] Saudação dinâmica ("Bom dia", "Boa tarde", "Boa noite") com primeiro nome
- [ ] Card grande de status mostra "Pausado" (cinza)
- [ ] Grid com 3 métricas: Itens bloqueados (0), Filtro adulto (Desligado), Modo (Pessoal)
- [ ] Versão do app no rodapé ("DopaBlocker desktop v0.1.0")

### 7. Adicionar bloqueio
Ir para /blocking → clicar "Adicionar":
- [ ] Modal abre com abas Site / App / Palavra-chave
- [ ] Campo "Valor" focado automaticamente
- [ ] Teste de normalização: digitar `https://www.Instagram.com/reels` → clicar Adicionar → aparece como `instagram.com` na lista
- [ ] Toast verde "Bloqueio adicionado" no canto inferior direito
- [ ] Lista mostra o item com badge "Site" e "agora"
- [ ] Esc fecha o modal; click fora do dialog também fecha

### 8. Validação server-side
No terminal:
```powershell
curl.exe -X POST http://localhost:3000/blocklist `
  -H "Authorization: Bearer <JWT>" `
  -H "Content-Type: application/json" `
  -d "{\"item_type\":\"domain\",\"value\":\"https://WWW.Twitter.COM/path\"}"
```
(pegue o JWT via DevTools do Tauri → `await firebase.auth().currentUser.getIdToken()`)
- [ ] Retorna 200 com `value: "twitter.com"` normalizado
- [ ] POST com `value: "foo"` (sem TLD) retorna 400 "domínio deve conter pelo menos um ponto"

### 9. Ativar bloqueio (requer admin)
- [ ] Clicar "Ativar bloqueio" no card de status
- [ ] Toast "Bloqueio ativado"
- [ ] Status vira verde "Ativo"
- [ ] Em outro terminal: `ipconfig /all` mostra DNS da interface ativa = `127.0.0.1`
- [ ] `nslookup instagram.com` (sem especificar servidor) → NXDOMAIN
- [ ] `nslookup google.com` → resolve IP real
- [ ] `nslookup instagram.com 8.8.8.8` → **timeout** (WFP bloqueia DNS fora do proxy)
- [ ] `netsh wfp show state` — arquivo XML gerado contém "DopaBlocker" nos filtros

### 10. Hot reload da blocklist
- [ ] Com bloqueio ativo, adicionar `youtube.com`
- [ ] `nslookup m.youtube.com` (subdomínio!) → NXDOMAIN imediato (sem precisar toggle off/on)
- [ ] Remover `youtube.com` pela UI → `nslookup m.youtube.com` volta a resolver

### 11. Filtro adulto
- [ ] Toggle "Filtro de conteúdo adulto" → ON
- [ ] Na primeira vez, badge "Construindo…" aparece por alguns segundos
- [ ] Após construir, badge some
- [ ] Toast "Filtro adulto ligado"
- [ ] `nslookup pornhub.com` → NXDOMAIN
- [ ] `nslookup m.pornhub.com` → NXDOMAIN (walk label-por-label funciona)
- [ ] Toggle → OFF → `nslookup pornhub.com` resolve normalmente

### 12. Desativar bloqueio
- [ ] Clicar "Pausar"
- [ ] Toast "Bloqueio pausado"
- [ ] `ipconfig /all` mostra DNS original (não mais 127.0.0.1)
- [ ] `nslookup instagram.com` resolve IP real de novo
- [ ] `nslookup instagram.com 8.8.8.8` funciona (filtros WFP removidos)

### 13. Crash recovery
- [ ] Ativar bloqueio → verificar `ipconfig` mostrando `127.0.0.1`
- [ ] **Kill** o processo `dopablocker-desktop.exe` via Task Manager (End Task)
- [ ] `ipconfig /all` — DNS ainda é `127.0.0.1` (órfão)
- [ ] Reabrir o app (`pnpm tauri:dev`) como admin
- [ ] Log esperado: `"engine reativado no boot"` ou `"falha ao restaurar DNS órfão"` + reconfiguração
- [ ] `ipconfig /all` deve estar em estado consistente: ou original restaurado, ou apontando pro proxy novo

### 14. Logout
- [ ] Clicar "Sair" na sidebar **OU** em /settings → "Sair da conta"
- [ ] Modal de confirmação abre
- [ ] Cancelar → modal fecha, sessão permanece
- [ ] Confirmar "Sair" → redireciona para /login
- [ ] Sem toasts de erro

### 15. Login com conta existente
- [ ] Na tela de login, aba "Entrar"
- [ ] Email + senha corretos → dashboard
- [ ] Onboarding **não** reaparece (localStorage tem flag desse user)
- [ ] Bloqueios persistidos aparecem na lista

### 16. Testes de erro
- [ ] Parar o backend (Ctrl+C no terminal backend)
- [ ] Adicionar item → toast vermelho "Falha ao adicionar" ou similar
- [ ] Re-iniciar backend → ações voltam a funcionar

---

## Testes automatizados (existentes)

```powershell
cargo test --workspace         # 18 testes: shared (13) + desktop (5)
pnpm --filter desktop check     # svelte-check: 0 errors, 0 warnings
```

**Cobertura atual:**
- `shared`: Bloom filter (insert/contains/FP-rate), domain_matcher (normalize/extract/is_blocked)
- `desktop`: system_dns parser (EN/PT), adult_filter parser (hosts file), flag enabled

**Gaps automatizados:** ver [GAPS.md](GAPS.md) seção "Observability & Testing".

---

## Critério de release

Todos os itens acima marcados OK + os testes automatizados verdes = v0.1 pronto
para uso pessoal do dev. Para uso externo, rever antes os hardening items
🟡H1 (CORS), 🟡H2 (rate limiting) e 🔵U2 (UAC manifest / code signing) em
[GAPS.md](GAPS.md).
