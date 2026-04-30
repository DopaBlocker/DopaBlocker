# DopaBlocker — Golden Path Atual

Roteiro manual para validar o que existe hoje em backend + desktop. A meta v0.2
também inclui mobile Android, mas o mobile ainda está em placeholder; os testes
mobile entram aqui quando `mobile/` deixar de ser esqueleto.

Automação E2E ainda é gap; ver [GAPS.md](GAPS.md) seção "Observability & Testing".

---

## Pré-requisitos

- Windows 10/11 (x64)
- Rust toolchain, `pnpm` e dependências do Tauri instalados
- `.env` configurados em `backend/` e `desktop/`
- Terminal como Administrador para validar porta 53, WFP, `netsh` e CA local

## Limpeza Inicial Opcional

```powershell
Remove-Item "$env:APPDATA\app.dopablocker.desktop" -Recurse -ErrorAction SilentlyContinue
Remove-Item ".\backend\dopablocker.db" -ErrorAction SilentlyContinue
```

Para repetir o onboarding de primeira execução, limpe também o `localStorage`
do WebView/DevTools.

---

## Passos

### 1. Subir backend

```powershell
cd backend
cargo run
```

Esperado:

- Backend escuta em `0.0.0.0:3000`.
- Migrations `001_initial`, `002_parental_fixes` e `003_email_verification`
  são aplicadas em banco limpo.
- `GET http://localhost:3000/health` retorna `OK`.

### 2. Subir desktop

Em outro terminal, na raiz do projeto:

```powershell
pnpm --dir desktop tauri dev
```

Esperado:

- Tauri compila sem erro.
- Janela abre em `/welcome`.
- A tela inicial mostra três opções: Pessoal, Pais e Filhos.

### 3. Cadastro Pessoal

- Clicar em **Pessoal** leva para `/login?mode=personal`.
- Aba **Cadastrar** mostra formulário de nome, email, senha e confirmação.
- Senhas divergentes mostram erro.
- Cadastro email/senha envia código de verificação.
- Código inválido não cria usuário local.
- Código válido cria conta Firebase, chama `POST /auth/register` e abre o dashboard.
- Onboarding "Bem-vindo ao DopaBlocker" aparece uma vez por usuário.

### 4. Login Existente

- Logout abre modal de confirmação.
- Confirmar logout volta para `/welcome`.
- Clicar em **Pessoal** ou **Pais** e usar a aba **Entrar** autentica com conta existente.
- Onboarding não reaparece para o mesmo usuário.

### 5. Cadastro Pais e Vinculação Filho

- Clicar em **Pais** leva para `/login?mode=parental`.
- Cadastro/login parental abre dashboard com modo `Parental`.
- Sidebar mostra link **Filhos**.
- Em `/parental`, clicar **Gerar código de vinculação**.
- O desktop registra o device titular se ainda não existir e então chama
  `POST /devices/link/generate`.
- Código de 6 dígitos aparece com countdown de 5 minutos.
- Em `/welcome`, clicar **Filhos** leva para `/onboarding/child`.
- Digitar o código válido chama `POST /devices/link/confirm`, salva
  `child_session` no SQLCipher local e leva para `/child-blocked`.
- Revogar o filho em `/parental` faz o token antigo deixar de funcionar no
  próximo ciclo de validação.

### 6. Dashboard e Navegação

- Dashboard mostra saudação, status do bloqueio, métricas e modo atual.
- `/blocking` mostra lista de bloqueios e toggles.
- `/settings` mostra conta, modo, versão, logout e exclusão de conta.
- `/parental` só aparece para usuário com `mode=parental`.
- Sessão `child_session` fica presa em `/child-blocked`, sem sidebar.

### 7. Blocklist

- Em `/blocking`, clicar **Adicionar** abre modal.
- Domínio `https://www.Instagram.com/reels` é normalizado para `instagram.com`.
- Item aparece na lista com badge `Site`.
- Remover item atualiza UI e backend.
- POST direto com domínio sem ponto retorna `400`.
- Duplicata retorna `409`.

### 8. Ativar Bloqueio Desktop

Requer terminal/app como administrador.

- Clicar **Ativar bloqueio** em `/blocking`.
- DNS da interface ativa aponta para loopback (`127.0.0.1` e/ou `::1` conforme família).
- `nslookup instagram.com` retorna bloqueio/local.
- `nslookup google.com` resolve IP real.
- DNS externo direto para `8.8.8.8` deve ser bloqueado pelo WFP em IPv4.
- `netsh wfp show state` contém filtros DopaBlocker.

### 9. Página de Bloqueio e CA Local

- Primeira ativação instala ou reutiliza a CA local do DopaBlocker.
- `certutil -store Root | findstr DopaBlocker` encontra a CA no Windows Root Store.
- `https://instagram.com` em Chrome/Edge/Brave mostra página local do DopaBlocker.
- A página exibe domínio e razão do bloqueio.
- Firefox pode mostrar erro de certificado por usar NSS.

### 10. Filtro Adulto

- Toggle **Filtro de conteúdo adulto** liga a configuração.
- Na primeira construção, a UI mostra estado `Construindo...`.
- Após a lista carregar, domínio adulto conhecido bloqueia.
- Desligar o toggle libera o domínio novamente.

### 11. Hot Reload e Crash Recovery

- Com bloqueio ativo, adicionar `youtube.com` bloqueia subdomínios sem reiniciar engine.
- Remover `youtube.com` libera subdomínios.
- Após kill forçado do processo, reabrir o app deve restaurar DNS órfão ou reativar o engine de forma consistente.

### 12. Exclusão de Conta

- Em `/settings`, abrir **Excluir conta permanentemente**.
- Texto de confirmação obrigatório impede exclusão acidental.
- Fluxo tenta apagar Firebase primeiro e backend depois.
- Ao final, logout local volta para `/welcome`.

---

## Verificações Automatizadas

```powershell
cargo test
pnpm --dir desktop check
```

Última auditoria local: `cargo test` passou com 60 testes Rust
(backend 20, desktop 24, shared 16) e `pnpm --dir desktop check` passou com
0 erros/0 warnings. `flutter analyze` foi interrompido anteriormente; mobile
ainda não é critério de pronto porque os arquivos são placeholders.

---

## Critério de Release Interno

Todos os passos aplicáveis acima marcados OK, `cargo test` verde e
`pnpm --dir desktop check` verde. Para v0.2, adicionar golden path mobile e os
smoke tests cross-platform descritos em [PROTOTYPE.md](PROTOTYPE.md).
