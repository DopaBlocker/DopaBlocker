# DopaBlocker

DopaBlocker e um bloqueador de distrações digitais. Ele bloqueia o acesso a redes sociais, sites de entretenimento e conteudo adulto nos seus dispositivos Windows e Android, e so libera o acesso quando voce decide. O app funciona em dois modos: **pessoal** (voce controla seus proprios bloqueios, sincronizados entre desktop e celular) e **controle parental** (o pai gerencia os bloqueios no dispositivo do filho, tudo dentro do mesmo app e da mesma conta).

---

## Requisitos do Sistema

- **Sistema Operacional:** Windows 10/11 (64-bit)
- **RAM:** 8 GB minimo (16 GB recomendado — Android Emulator consome bastante)
- **Disco:** ~15 GB livres (SDKs + emuladores + builds)
- **Conexao com a internet** para baixar dependencias

---

## Passo 1 — Instalar Rust

Rust e usado no backend (API), no app desktop (Tauri) e na crate compartilhada.

1. Baixar e executar o instalador: https://rustup.rs
2. Na instalacao, escolher **"1) Proceed with standard installation"**
3. Quando terminar, **fechar e reabrir o terminal**
4. Verificar:

```bash
rustc --version
# Esperado: rustc 1.77+ (qualquer versao recente funciona)

cargo --version
```

### Instalar o Tauri CLI

```bash
cargo install tauri-cli
```

Isso demora alguns minutos na primeira vez. Verificar:

```bash
cargo tauri --version
# Esperado: tauri-cli 2.x
```

### Instalar C++ Build Tools (obrigatorio no Windows)

O Rust precisa de um linker C++ para compilar no Windows.

1. Baixar o **Visual Studio Build Tools**: https://visualstudio.microsoft.com/visual-cpp-build-tools/
2. No instalador, marcar **"Desktop development with C++"**
3. Instalar e reiniciar o computador

### Instalar WebView2 (obrigatorio para Tauri)

O Tauri usa o WebView2 para renderizar a interface. No Windows 10/11 ele ja vem instalado na maioria dos casos. Para verificar:

1. Abrir **Configuracoes > Apps > Apps instalados**
2. Procurar por **"Microsoft Edge WebView2 Runtime"**
3. Se nao estiver instalado, baixar em: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

---

## Passo 2 — Instalar Node.js e pnpm

Node.js e pnpm sao usados no frontend do desktop (SvelteKit + Tailwind).

1. Baixar e instalar o Node.js LTS: https://nodejs.org
2. Fechar e reabrir o terminal
3. Instalar o pnpm globalmente:

```bash
npm install -g pnpm
```

4. Verificar:

```bash
node --version
# Esperado: v20+ ou v22+

pnpm --version
# Esperado: 9+ ou 10+
```

---

## Passo 3 — Instalar Flutter e Android SDK

Flutter e usado no app mobile. O Android Studio fornece o SDK, emulador e build tools.

### 3.1 — Instalar Android Studio

1. Baixar e instalar: https://developer.android.com/studio
2. Abrir o Android Studio e completar o **Setup Wizard** (baixa SDK automaticamente)
3. Ir em **Settings > Languages & Frameworks > Android SDK**
4. Na aba **SDK Platforms**: marcar **Android 14 (API 35 e 36)**
5. Na aba **SDK Tools**: marcar:
   - **Android SDK Build-Tools** (versao mais recente)
   - **Android SDK Command-line Tools**
   - **Android SDK Platform-Tools**
   - **Android NDK** (necessario para plugins Flutter nativos)
6. Clicar em **Apply** e aguardar o download

### 3.2 — Instalar Flutter SDK

1. Baixar o Flutter SDK: https://docs.flutter.dev/get-started/install/windows/mobile
2. Extrair o zip em uma pasta **sem espacos no caminho** (ex: `C:\flutter`)
3. Adicionar `C:\flutter\bin` ao PATH do sistema:
   - Abrir **Configuracoes > Sistema > Sobre > Configuracoes avancadas do sistema**
   - Clicar em **Variaveis de Ambiente**
   - Na variavel **Path** do usuario, clicar em **Editar > Novo**
   - Colar o caminho: `C:\flutter\bin`
   - Clicar OK em tudo

4. Fechar e reabrir o terminal
5. Verificar:

```bash
flutter --version
# Esperado: Flutter 3.x

flutter doctor
```

### 3.3 — Aceitar licencas do Android

```bash
flutter doctor --android-licenses
```

Digitar **y** para aceitar todas.

### 3.4 — Ativar Modo Desenvolvedor no Windows

Necessario para o Flutter funcionar corretamente:

1. Abrir **Configuracoes > Para Desenvolvedores** (ou **Privacidade e seguranca > Para desenvolvedores**)
2. Ativar **Modo do Desenvolvedor**

### 3.5 — Criar um emulador Android (opcional, para testar sem celular)

1. Abrir Android Studio
2. Ir em **Device Manager > Create Virtual Device**
3. Escolher um modelo (ex: Pixel 7)
4. Selecionar uma imagem de sistema (ex: API 35)
5. Finalizar e iniciar o emulador

### 3.6 — Testar com celular fisico (alternativa ao emulador)

1. No celular Android, ativar **Opcoes do desenvolvedor** (tocar 7 vezes no "Numero da versao" em Configuracoes > Sobre o telefone)
2. Ativar **Depuracao USB** nas Opcoes do desenvolvedor
3. Conectar o celular via USB e aceitar o prompt de depuracao
4. Verificar: `flutter devices` deve listar o celular

---

## Passo 4 — Instalar Docker Desktop

Docker e usado para rodar o backend em container.

1. Baixar e instalar: https://www.docker.com/products/docker-desktop/
2. Durante a instalacao, aceitar a opcao de usar **WSL 2** como backend
3. Reiniciar o computador se pedido
4. Abrir o Docker Desktop e aguardar ele iniciar
5. Verificar:

```bash
docker --version
# Esperado: Docker version 24+ ou superior
```

### 4.1 — Instalar WSL 2 (se nao tiver)

O Docker Desktop precisa do WSL 2. Se ele pedir para instalar:

```bash
wsl --install
```

Reiniciar o computador e definir um usuario/senha para o Ubuntu.

---

## Passo 5 — Instalar Firebase CLI

Firebase e usado para autenticacao (Google + email/senha) e Firestore (sincronizacao de dados).

```bash
npm install -g firebase-tools
```

Fazer login na conta Google que tem o projeto Firebase:

```bash
firebase login
```

Verificar:

```bash
firebase --version
# Esperado: 13+ ou superior
```

---

## Passo 6 — Clonar e Configurar o Projeto

### 6.1 — Clonar o repositorio

```bash
git clone <url-do-repositorio> DopaBlocker
cd DopaBlocker
```

### 6.2 — Configurar variaveis de ambiente

Copiar os arquivos de exemplo:

```bash
cp .env.example .env
cp backend/.env.example backend/.env
```

Editar o arquivo `backend/.env` com os valores reais:

```env
PORT=3000
DATABASE_URL=sqlite://dopablocker.db
FIREBASE_PROJECT_ID=dopablocker-b8425
FIREBASE_API_KEY=<sua-api-key-do-firebase>
RUST_LOG=info
```

Para encontrar a Firebase API Key:
1. Ir no **Firebase Console** (https://console.firebase.google.com)
2. Selecionar o projeto **dopablocker-b8425**
3. Ir em **Configuracoes do projeto** (engrenagem)
4. A API Key esta na secao **"Seus apps"**

### 6.3 — Instalar dependencias do desktop (Node/pnpm)

```bash
cd desktop
pnpm install
cd ..
```

### 6.4 — Instalar dependencias do mobile (Flutter/Dart)

```bash
cd mobile
flutter pub get
cd ..
```

### 6.5 — Compilar o workspace Rust (shared + backend + desktop)

```bash
cargo build
```

A primeira compilacao demora varios minutos. As seguintes sao rapidas.

---

## Passo 7 — Rodar o Projeto

### Rodar o Backend

```bash
cd backend
cargo run
```

O servidor inicia em `http://localhost:3000`.

### Rodar o Desktop (Tauri + SvelteKit)

Em outro terminal, na raiz do projeto:

```bash
pnpm tauri:dev
```

Isso inicia o Vite dev server (SvelteKit) e abre a janela do Tauri. Hot reload funciona automaticamente — ao editar arquivos `.svelte`, a interface atualiza na hora.

### Rodar o Mobile (Flutter)

Em outro terminal:

```bash
cd mobile
flutter run
```

Se tiver um emulador rodando ou celular conectado, o app abre automaticamente. Se tiver varios dispositivos conectados, o Flutter pede para escolher.

### Rodar o Backend via Docker (alternativa)

```bash
cd infra
docker compose up --build
```

---

## Passo 8 — Verificar que tudo funciona

Rodar estes comandos para garantir que o ambiente esta correto:

```bash
# Rust — compila os 3 crates (shared, backend, desktop)
cargo check

# Desktop — build de producao do SvelteKit
cd desktop && pnpm build && cd ..

# Mobile — analise estatica do Dart
cd mobile && flutter analyze && cd ..

# Checklist geral de dependencias
flutter doctor
```

Se todos passarem sem erro, o ambiente esta pronto.

---

## Estrutura do Projeto

```
DopaBlocker/
├── backend/           # API REST em Rust/Axum
│   ├── src/
│   │   ├── main.rs            # Entry point do servidor
│   │   ├── config.rs          # Configuracao via .env
│   │   ├── errors.rs          # Tipos de erro da API
│   │   ├── middleware.rs       # Validacao de Firebase JWT
│   │   ├── models.rs          # Modelos de request/response
│   │   ├── routes/            # Endpoints (auth, blocklist, devices)
│   │   └── services/          # Logica de negocio
│   ├── migrations/            # SQL schemas
│   └── Dockerfile
│
├── desktop/           # App desktop — Tauri 2 + SvelteKit + Tailwind
│   ├── src/
│   │   ├── routes/            # Paginas (login, blocking, parental, settings)
│   │   └── lib/
│   │       ├── components/    # Componentes Svelte (LoginForm, BlockList, etc.)
│   │       ├── stores/        # Estado reativo (auth, blocking)
│   │       └── services/      # API client, Firebase, Tauri bridge
│   └── src-tauri/
│       └── src/
│           ├── commands.rs    # Comandos IPC (frontend -> Rust)
│           ├── db.rs          # SQLite local
│           └── blocking/      # Engine de bloqueio (WFP, DNS proxy, adult filter)
│
├── mobile/            # App mobile — Flutter + Kotlin
│   ├── lib/
│   │   ├── screens/           # Telas (login, home, blocking, parental, settings)
│   │   ├── providers/         # Estado com Riverpod
│   │   ├── models/            # Modelos de dados Dart
│   │   ├── core/              # API client, Firebase, constantes
│   │   ├── widgets/           # Componentes reutilizaveis
│   │   └── channels/          # Bridge Flutter <-> Kotlin nativo
│   └── android/.../kotlin/
│       ├── vpn/               # VPN service para bloqueio DNS
│       ├── accessibility/     # Bloqueio de abertura de apps
│       └── receivers/         # Reinicio automatico no boot
│
├── shared/            # Crate Rust compartilhada
│   └── src/
│       ├── models.rs          # Modelos compartilhados (User, Device, BlockedItem)
│       ├── bloom_filter.rs    # Filtro de conteudo adulto
│       └── domain_matcher.rs  # Matching e normalizacao de dominios
│
├── infra/             # Configuracoes de infraestrutura
│   ├── firebase.json          # Config do Firebase
│   ├── firestore.rules        # Regras de seguranca do Firestore
│   └── compose.yml            # Docker Compose para dev local
│
└── docs/              # Documentacao
    ├── ARCHITECTURE.md        # Arquitetura e fluxo de dados
    ├── PROTOTYPE.md           # Escopo do prototipo v0.1
    └── API.md                 # Documentacao da API REST
```

---

## Resumo de Comandos

| Acao | Comando |
|---|---|
| Compilar Rust (verificacao) | `cargo check` |
| Compilar Rust (build) | `cargo build` |
| Rodar backend | `cd backend && cargo run` |
| Rodar desktop (dev) | `pnpm tauri:dev` |
| Rodar desktop (build producao) | `pnpm tauri:build` |
| Rodar mobile | `cd mobile && flutter run` |
| Instalar deps desktop | `cd desktop && pnpm install` |
| Instalar deps mobile | `cd mobile && flutter pub get` |
| Docker backend | `cd infra && docker compose up --build` |
