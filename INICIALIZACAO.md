# Inicialização e Execução do DopaBlocker

Este guia descreve o passo a passo manual para executar o **Backend**, o **App Mobile (Android)** e o **App Desktop (Windows)** em ambiente de desenvolvimento.

---

## 1. Preparação e Dependências

Antes de rodar os projetos pela primeira vez, certifique-se de instalar as dependências de cada plataforma:

1. **Variáveis do Backend:** Garanta que o arquivo `backend/.env` esteja configurado (veja os detalhes no [README.md](README.md)).
2. **Dependências do Desktop:** 
   ```bash
   cd desktop
   pnpm install
   cd ..
   ```
3. **Dependências do Mobile:** 
   ```bash
   cd mobile
   flutter pub get
   cd ..
   ```

---

## 2. Passo a Passo para Execução

Siga os passos na ordem recomendada abaixo, abrindo terminais separados para cada componente:

### Passo 1 — Iniciar o Backend
O backend serve como a API central de sincronização e autenticação.
1. Abra um terminal na pasta raiz do projeto.
2. Navegue até a pasta `backend` e execute o servidor:
   ```bash
   cd backend
   cargo run
   ```
3. O console indicará que o servidor está compilado e escutando em `http://localhost:3000`. Mantenha este terminal aberto.

### Passo 2 — Iniciar o Emulador ou Conectar Dispositivo (Mobile)
Para rodar o aplicativo Android:
1. Abra o **Android Studio**.
2. Vá em **Device Manager** e inicialize um emulador configurado (ou conecte um celular físico com a *Depuração USB* ativada).
3. Aguarde o sistema Android inicializar completamente.

### Passo 3 — Executar o App Mobile (Flutter)
Com o emulador ou celular conectado e ativo:
1. Abra um novo terminal.
2. Navegue até a pasta `mobile/` e execute o app:
   ```bash
   cd mobile
   flutter run
   ```
3. Se houver mais de um dispositivo conectado, o terminal solicitará que você escolha em qual deles deseja iniciar o app.

### Passo 4 — Executar o App Desktop (Tauri)
A engine de bloqueio do Windows exige privilégios de Administrador para interceptar o tráfego de rede (WFP) e rodar o proxy de DNS local na porta 53.

1. Procure por **PowerShell** ou **Prompt de Comando** no menu iniciar do Windows.
2. Clique com o botão direito e selecione **"Executar como Administrador"**.
3. Navegue até a pasta raiz do projeto no terminal elevado:
   ```cmd
   cd "C:\caminho\para\seu\projeto\DopaBlocker"
   ```
4. Inicie o ambiente de desenvolvimento do Tauri:
   ```bash
   pnpm tauri:dev
   ```
5. Isso compilará os arquivos Rust do Tauri, iniciará o servidor frontend do Vite (SvelteKit) e abrirá a janela visível do DopaBlocker Desktop.

---

## 3. Resolução de Problemas Comuns

### O bloqueio de sites não funciona no Desktop
- **Causa:** O terminal onde você executou `pnpm tauri:dev` não possui permissões administrativas.
- **Solução:** Feche o aplicativo, abra um terminal elevado (Executar como Administrador), navegue até a pasta e execute o comando novamente.

### Falha ao iniciar na porta 53 (Port 53 already in use)
- **Causa:** Outro serviço de DNS local (como WSL2 ativo, Docker Desktop rodando ou DNS do Windows Server) está usando a porta 53.
- **Solução:** Identifique e pare temporariamente o serviço que está usando a porta (pode utilizar `netstat -ano | findstr :53` no cmd de administrador para encontrar o PID do processo).
