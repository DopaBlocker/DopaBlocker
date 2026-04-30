# DopaBlocker — Conceitos e Tecnologias

Este documento explica, de forma acessivel, os conceitos e tecnologias mais importantes usados no DopaBlocker. O objetivo e que qualquer pessoa consiga entender **o que cada coisa faz** e **por que escolhemos usar ela** no projeto.

---

## Indice

1. [Bloom Filter — Filtro de conteudo adulto](#1-bloom-filter)
2. [DNS Proxy — Bloqueio de sites por dominio](#2-dns-proxy)
3. [Windows Filtering Platform (WFP) — Bloqueio no nivel do sistema](#3-windows-filtering-platform-wfp)
4. [Firebase JWT — Autenticacao segura](#4-firebase-jwt)
5. [Riverpod — Gerenciamento de estado no Flutter](#5-riverpod)
6. [Docker — Ambiente isolado para o backend](#6-docker)
7. [SQLCipher — Banco de dados criptografado](#7-sqlcipher)

---

## 1. Bloom Filter

### O que e?

Um Bloom Filter e uma estrutura de dados que responde a uma pergunta simples: **"esse item esta no conjunto?"**. Ele e extremamente rapido e usa muito pouca memoria, mas tem um detalhe: ele pode dar **falsos positivos** (dizer "sim" quando a resposta e "nao"), porem **nunca da falsos negativos** (se ele diz "nao", e "nao" com certeza).

### Analogia

Imagine que voce tem uma lista de 2 milhoes de sites adultos. Guardar essa lista inteira na memoria seria pesado — cada URL ocupa espaco, e buscar nela seria lento.

O Bloom Filter funciona como um "carimbo digital" dessa lista. Em vez de guardar todos os 2 milhoes de enderecos, ele cria uma tabela compacta de bits (zeros e uns). Quando voce quer saber se `site-adulto.com` esta na lista, ele aplica algumas funcoes matematicas (chamadas de **hash**) no dominio e verifica se os bits correspondentes estao marcados.

```
Dominio: "site-adulto.com"
         |
         v
   Hash 1 → posicao 42    → bit = 1 ✓
   Hash 2 → posicao 1087  → bit = 1 ✓
   Hash 3 → posicao 5531  → bit = 1 ✓
         |
         v
   Todos marcados → "provavelmente esta na lista"
```

Se **qualquer** bit estiver em 0, o dominio **com certeza** nao esta na lista. Se todos estiverem em 1, ele **provavelmente** esta (pode ser coincidencia — o falso positivo).

### Por que usar no DopaBlocker?

O filtro de conteudo adulto do DopaBlocker usa listas publicas como a **Steven Black** e **OISD**, que contem milhoes de dominios. Precisamos verificar, em tempo real, se cada site que o usuario tenta acessar esta nessas listas.


| Abordagem                | Memoria     | Velocidade de busca                    |
| ------------------------ | ----------- | -------------------------------------- |
| Lista completa (HashSet) | ~150 MB+    | Rapida, mas consome muita RAM          |
| Banco de dados (SQLite)  | Disco       | Lenta (acesso a disco a cada consulta) |
| **Bloom Filter**         | **~2-5 MB** | **Extremamente rapida (nanosegundos)** |


O Bloom Filter e perfeito para esse caso porque:

- **Memoria minima**: 2 milhoes de dominios cabem em poucos megabytes
- **Velocidade**: a checagem e quase instantanea — essencial porque toda requisicao DNS passa por essa verificacao
- **Falso positivo aceitavel**: se um site legitimo for bloqueado por engano (raro, ~0.1%), o usuario pode adicionar uma excecao. Mas um site adulto **nunca** vai escapar do filtro (zero falsos negativos)

### Onde fica no codigo?

- `shared/src/bloom_filter.rs` — implementacao do Bloom Filter
- Usado pelo desktop (no DNS proxy) e pelo mobile (no VPN service)

---

## 2. DNS Proxy

### O que e DNS?

Antes de entender o DNS Proxy, e preciso entender o DNS. Quando voce digita `instagram.com` no navegador, seu computador nao sabe onde fica esse site. Ele precisa perguntar para um **servidor DNS** (Domain Name System) qual e o endereco IP do Instagram.

```
Voce digita: instagram.com
         |
         v
Computador pergunta ao DNS: "Qual o IP de instagram.com?"
         |
         v
DNS responde: "157.240.1.174"
         |
         v
Navegador conecta ao IP 157.240.1.174
```

Isso acontece para **toda pagina** que voce acessa. Sem DNS, a internet nao funciona.

### O que e um DNS Proxy?

Um DNS Proxy e um servidor que fica **entre o seu computador e o servidor DNS real**. Toda pergunta DNS passa por ele primeiro. Isso permite que o proxy **intercepte** e **modifique** as respostas.

```
SEM DNS Proxy (normal):
  Navegador → Servidor DNS → responde o IP → Navegador conecta

COM DNS Proxy (DopaBlocker):
  Navegador → DNS Proxy (DopaBlocker) → checa a blocklist
      |                                        |
      |    Se permitido: repassa ao DNS real → responde o IP
      |    Se bloqueado: responde 0.0.0.0 (nenhum lugar)
      v
  Site bloqueado nao carrega
```

Quando o DNS Proxy responde `0.0.0.0` para um dominio bloqueado, o navegador tenta conectar a um endereco que nao existe — e o site simplesmente nao carrega. Isso funciona para **qualquer navegador e qualquer app** no computador, sem precisar instalar extensao nenhuma.

### Por que usar no DopaBlocker?

O DNS Proxy e o **coracao do sistema de bloqueio** porque:

1. **Funciona em todo o sistema**: bloqueia o site em todos os navegadores (Chrome, Firefox, Edge) e em qualquer aplicativo que tente acessar a internet. Uma extensao de navegador so funciona naquele navegador
2. **Invisivel para o usuario**: nao aparece como extensao, nao pode ser facilmente desativado
3. **Leve**: processar pacotes DNS consome quase nenhum recurso do computador
4. **Compativel com o Bloom Filter**: cada consulta DNS e verificada contra o Bloom Filter — a combinacao dos dois e extremamente eficiente

### Desktop vs Mobile

A implementacao e diferente em cada plataforma:

**Estado atual:** o desktop usa o DNS Proxy real. O mobile ainda e alvo da
v0.2; os arquivos Kotlin/Dart existem como placeholders e precisam ser
implementados antes de considerar o bloqueio Android funcional.


| Plataforma            | Como funciona                                                                                                                                  |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| **Windows (Desktop)** | O DopaBlocker roda um servidor DNS local na porta 53. O WFP (explicado abaixo) redireciona todo o trafego DNS do sistema para esse servidor    |
| **Android (Mobile)**  | Alvo v0.2: o DopaBlocker cria uma VPN local usando `VpnService`. Todo o trafego de rede passa pelo app, que intercepta os pacotes DNS e aplica o bloqueio |


### Onde fica no codigo?

- **Desktop**: `desktop/src-tauri/src/blocking/dns_proxy.rs`
- **Mobile**: `mobile/android/.../vpn/DnsVpnService.kt` (placeholder no estado atual)

---

## 3. Windows Filtering Platform (WFP)

### O que e?

O Windows Filtering Platform e uma API do Windows que permite **inspecionar e controlar o trafego de rede no nivel do kernel** (o nucleo do sistema operacional). Programas de antivirus e firewalls usam o WFP para bloquear conexoes.

### Por que e necessario?

No Windows, o DNS Proxy sozinho nao basta. Mesmo que o DopaBlocker rode um servidor DNS local, nada impede o navegador de usar outro servidor DNS (como o 8.8.8.8 do Google) e ignorar completamente o bloqueio.

O WFP resolve isso criando **regras no firewall do Windows** que:

1. **Redirecionam** todo o trafego DNS (porta 53) para o DNS Proxy do DopaBlocker
2. **Bloqueiam** qualquer tentativa de usar DNS-over-HTTPS (DoH), que seria uma forma de contornar o bloqueio

```
SEM WFP:
  Chrome (usa DNS 8.8.8.8) → ignora o DNS Proxy → site abre normalmente ✗

COM WFP:
  Chrome (tenta usar DNS 8.8.8.8)
      |
      v
  WFP intercepta → redireciona para DNS Proxy local
      |
      v
  DNS Proxy checa blocklist → site bloqueado ✓
```

### Analogia

Pense no WFP como um **guarda de transito** dentro do Windows. Todo pacote de dados que entra ou sai do computador passa por ele. O DopaBlocker pede para esse guarda: "se alguem tentar sair pela porta 53 (DNS), manda ele passar por mim primeiro". O guarda obedece, e agora ninguem consegue resolver nomes de dominio sem passar pelo filtro.

### Nivel de protecao

O WFP opera no **modo kernel**, o que significa que:

- Aplicativos comuns nao conseguem contornar as regras
- Mesmo que o usuario abra outro navegador ou use um app diferente, o bloqueio funciona
- So pode ser desativado por um processo com **privilegios de administrador**

### Onde fica no codigo?

- `desktop/src-tauri/src/blocking/wfp.rs` — integracao com a API do WFP via FFI (Foreign Function Interface — chamadas do Rust para APIs C/C++ do Windows)
- `desktop/src-tauri/src/blocking/engine.rs` — orquestra o WFP junto com o DNS Proxy

---

## 4. Firebase JWT

### O que e Firebase Authentication?

Firebase Authentication e um servico do Google que cuida de todo o sistema de login de um aplicativo: cadastro com email/senha, login com Google, recuperacao de senha, etc. Em vez de implementar tudo isso do zero (o que seria complexo e inseguro), delegamos para o Firebase.

### O que e um JWT?

JWT (JSON Web Token) e um "cartao de identidade digital". Quando o usuario faz login no DopaBlocker (pelo app desktop ou mobile), o Firebase gera um JWT — um texto codificado que contem:

```
eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoiYWJjMTIzIiwiZW1haWwiOiJ1c2VyQGVtYWlsLmNvbSIsImlhdCI6MTcxMDAwMDAwMCwiZXhwIjoxNzEwMDAzNjAwfQ.assinatura_digital
```

Esse texto parece aleatorio, mas na verdade sao **3 partes separadas por pontos**:

```
HEADER.PAYLOAD.ASSINATURA
```


| Parte          | O que contem                                                     |
| -------------- | ---------------------------------------------------------------- |
| **Header**     | Tipo do token e algoritmo de criptografia                        |
| **Payload**    | Dados do usuario (ID, email, quando expira)                      |
| **Assinatura** | Prova matematica de que o token e autentico e nao foi adulterado |


### Como funciona no DopaBlocker?

```
1. Usuario abre o app e faz login (email/senha ou Google)
         |
         v
2. Firebase valida as credenciais e retorna um JWT
         |
         v
3. O app guarda o JWT localmente
         |
         v
4. Toda requisicao para o backend inclui o JWT no header:
   Authorization: Bearer eyJhbGci...
         |
         v
5. O backend recebe a requisicao, extrai o JWT e valida:
   - A assinatura e do Firebase? (nao foi falsificado)
   - O token ainda nao expirou?
   - O usuario existe?
         |
         v
6. Se valido: processa a requisicao
   Se invalido: retorna erro 401 (nao autorizado)
```

### Por que usar Firebase + JWT?

1. **Seguranca sem complexidade**: implementar autenticacao do zero e uma das tarefas mais propensas a falhas de seguranca. Firebase faz isso certo por padrao
2. **Login com Google de graca**: integrar OAuth 2.0 manualmente exige registrar o app no Google, lidar com tokens de refresh, callbacks, etc. Firebase faz tudo com poucas linhas
3. **Funciona offline**: o JWT e um token auto-contido — o backend pode validar ele sem precisar consultar o Firebase a cada requisicao. Basta verificar a assinatura
4. **Cross-platform**: o mesmo sistema de login funciona no desktop (via Firebase JS SDK) e deve ser espelhado no mobile v0.2 (via Firebase Flutter SDK), usando a mesma conta e os mesmos tokens
5. **Expiracao automatica**: JWTs do Firebase expiram em 1 hora. O SDK renova automaticamente em background, sem o usuario perceber

### Onde fica no codigo?

- **Backend (validacao)**: `backend/src/middleware.rs` — middleware que intercepta toda requisicao protegida e valida o JWT
- **Desktop (login)**: `desktop/src/lib/services/firebase.ts` — inicializacao do Firebase Auth e funcoes de login/logout
- **Mobile (login)**: `mobile/lib/core/firebase_service.dart` — placeholder da implementacao Flutter

---

## 5. Riverpod

> **Status atual:** Riverpod e a arquitetura escolhida para o mobile v0.2, mas
> ainda nao esta instalado nem implementado em `mobile/pubspec.yaml`. Os arquivos
> em `mobile/lib/providers/*` sao placeholders.

### O que e gerenciamento de estado?

Em qualquer aplicativo, existem **dados que mudam ao longo do tempo** e que a interface precisa refletir. Exemplos no DopaBlocker:

- O usuario esta logado ou nao?
- Quais sites estao na blocklist?
- O bloqueio esta ativo ou pausado?
- O dispositivo e pai ou filho?

Quando esses dados mudam, a tela precisa atualizar automaticamente. "Gerenciamento de estado" e o nome que se da a como organizar e reagir a essas mudancas.

### O problema que o Riverpod resolve

Sem uma solucao de gerenciamento de estado, o codigo rapidamente vira uma bagunca:

```dart
// SEM gerenciamento de estado — codigo fragil e acoplado
class HomeScreen extends StatefulWidget {
  // Busca dados da API aqui
  // Guarda em variaveis locais
  // Passa dados para widgets filhos via construtor
  // Quando o dado muda, precisa chamar setState()
  // Se outra tela precisa do mesmo dado, busca de novo?
  // Se duas telas mostram o mesmo dado, como manter sincronizado?
}
```

Os problemas:

- **Dados duplicados**: varias telas buscam os mesmos dados independentemente
- **Sincronizacao manual**: se uma tela muda um dado, as outras nao sabem
- **Prop drilling**: passar dados de pai para filho, para neto, para bisneto...

### Como o Riverpod funciona

O Riverpod cria **providers** — fontes centralizadas de dados que qualquer widget pode observar:

```dart
// Provider de autenticacao — uma unica fonte de verdade
final authProvider = StateNotifierProvider<AuthNotifier, AuthState>((ref) {
  return AuthNotifier();
});
```

Qualquer tela que precise saber se o usuario esta logado simplesmente "observa" esse provider:

```dart
// Em qualquer tela do app
final auth = ref.watch(authProvider);
// Se o estado mudar, a tela re-renderiza automaticamente
```

```
                    authProvider
                    (fonte unica)
                   /      |      \
                  /       |       \
          HomeScreen  BlockingScreen  SettingsScreen
          (observa)    (observa)       (observa)

   Quando authProvider muda → todas as telas atualizam
```

### Por que Riverpod e nao outras opcoes?

O Flutter tem varias opcoes de gerenciamento de estado. As mais populares:


| Solucao             | Vantagem                                                   | Desvantagem                                               |
| ------------------- | ---------------------------------------------------------- | --------------------------------------------------------- |
| `setState` (nativo) | Simples                                                    | Nao escala, nao compartilha estado entre telas            |
| Provider (pacote)   | Popular, simples                                           | Limitado, depende da arvore de widgets, dificil de testar |
| BLoC                | Muito estruturado                                          | Verboso demais — muito codigo para coisas simples         |
| GetX                | Facil de comecar                                           | Magico demais, dificil de debugar, comunidade dividida    |
| **Riverpod**        | **Type-safe, testavel, independente da arvore de widgets** | **Curva de aprendizado inicial**                          |


O Riverpod foi escolhido porque:

1. **Compile-safe**: erros de tipo sao pegos em tempo de compilacao, nao em runtime
2. **Independente da arvore**: providers existem fora dos widgets — nao importa onde o widget esta na hierarquia, ele acessa o mesmo provider
3. **Testavel**: cada provider pode ser testado isoladamente, sem precisar montar widgets
4. **Combinacao de providers**: um provider pode depender de outro (ex: `blocklistProvider` depende de `authProvider` para saber de qual usuario buscar os dados)
5. **Padrao da comunidade Flutter**: e a evolucao do Provider (criado pelo mesmo autor) e a recomendacao atual para projetos novos

### Onde fica no codigo?

- `mobile/lib/providers/auth_provider.dart` — estado de autenticacao
- `mobile/lib/providers/blocking_provider.dart` — estado das regras de bloqueio
- `mobile/lib/providers/device_provider.dart` — estado dos dispositivos vinculados

---

## 6. Docker

### O que e?

Docker e uma ferramenta que empacota um aplicativo e **todas as suas dependencias** dentro de um **container** — um ambiente isolado que funciona de forma identica em qualquer maquina.

### Analogia

Imagine que voce quer enviar um aquario com peixes para outra cidade. Voce tem duas opcoes:

1. **Sem Docker**: enviar uma lista de instrucoes — "compre um aquario de 50L, encha com agua filtrada a 25°C, adicione X ml de condicionador, coloque as plantas assim..." — e torcer para que a outra pessoa faca tudo igual
2. **Com Docker**: enviar o aquario inteiro, com agua, peixes, plantas e temperatura — pronto para funcionar. E isso que um container faz com software

```
Sem Docker (instalacao manual):
  "Instale Rust 1.77, SQLite 3.x, configure essas 5 variaveis de ambiente,
   compile com essas flags, rode na porta 3000..."
   → Cada maquina pode dar problema diferente

Com Docker:
  docker compose up --build
  → Funciona igual em qualquer maquina
```

### Container vs Maquina Virtual

Containers sao frequentemente comparados com maquinas virtuais (VMs), mas sao bem diferentes:

```
Maquina Virtual:                    Container Docker:
┌─────────────────┐                 ┌─────────────────┐
│   Seu App        │                 │   Seu App        │
│   Dependencias   │                 │   Dependencias   │
│   Sistema Op.    │  ← SO inteiro   │                  │
│   Hypervisor     │                 │   Docker Engine  │  ← compartilha o SO
│   Hardware virt. │                 │                  │
└─────────────────┘                 └─────────────────┘
     ~2 GB, minutos                    ~50 MB, segundos
```

O container compartilha o kernel do sistema operacional, entao e **muito mais leve e rapido** que uma VM.

### Por que usar Docker no DopaBlocker?

O Docker e usado especificamente para o **backend** (API REST em Rust/Axum). Os motivos:

1. **Setup simplificado**: qualquer desenvolvedor novo no projeto roda `docker compose up --build` e tem o backend funcionando — sem instalar Rust, configurar SQLCipher, ou ajustar variaveis de ambiente manualmente
2. **Ambiente identico**: o container garante que o backend roda com as mesmas versoes de tudo (Rust compiler, libs, SQLCipher) independente do sistema operacional do desenvolvedor
3. **Isolamento**: o backend roda isolado do resto do sistema. Se algo der errado, basta parar o container — nada afeta o computador do desenvolvedor
4. **Preparacao para producao**: quando o DopaBlocker for publicado, o backend vai rodar em um servidor na nuvem (AWS, GCP, etc). O objetivo e ter o mesmo Dockerfile para desenvolvimento e producao — sem surpresas

### Como funciona no DopaBlocker?

O arquivo `infra/compose.yml` define o servico:

```yaml
services:
  backend:
    build: ../backend          # compila o Rust dentro do container
    ports:
      - "3000:3000"            # expoe a porta 3000
    environment:
      - DATABASE_URL=dopablocker.db
      - SQLCIPHER_KEY=${SQLCIPHER_KEY}
```

O `backend/Dockerfile` deve usar um build **multi-stage** (dois estagios). No
estado atual do repositorio ele ainda e um placeholder comentado, entao
`docker compose up --build` nao deve ser tratado como fluxo pronto ate esse
arquivo virar um Dockerfile real:

```
Estagio 1 (builder):
  - Usa imagem pesada com compilador Rust
  - Compila o backend
  - Gera um binario executavel

Estagio 2 (runtime):
  - Usa imagem minima (distroless, ~20 MB)
  - Copia apenas o binario compilado
  - Resultado: imagem final leve e segura
```

Isso significa que a imagem final do container nao tem compilador, codigo fonte, nem ferramentas desnecessarias — so o executavel e o minimo para rodar.

### Onde fica no codigo?

- `backend/Dockerfile` — placeholder atual das instrucoes de build do container
- `infra/compose.yml` — orquestracao planejada do container com Docker Compose

---

## 7. SQLCipher

### O que e?

SQLCipher e uma versao do SQLite com **criptografia AES-256 transparente**. Ele funciona exatamente como o SQLite — mesma sintaxe SQL, mesma API, mesma performance — mas todo o conteudo do arquivo `.db` e criptografado no disco. Sem a chave correta, o arquivo e apenas bytes aleatorios.

### Analogia

Imagine que o SQLite e um caderno onde voce anota informacoes. Qualquer pessoa que pegar esse caderno pode ler tudo. O SQLCipher e o mesmo caderno, mas escrito em codigo secreto — so quem tem a senha consegue decodificar. O caderno funciona identico para quem tem a senha (escrever, ler, apagar paginas), mas e completamente inutil para quem nao tem.

```
SQLite comum:
  Arquivo dopablocker.db → abrir com qualquer leitor SQLite → todos os dados visiveis ✗

SQLCipher:
  Arquivo dopablocker.db → abrir com leitor SQLite → "database is encrypted or is not a database"
  Arquivo dopablocker.db → abrir com PRAGMA key → todos os dados acessiveis ✓
```

### Como funciona?

O SQLCipher intercepta todas as leituras e escritas no arquivo de banco:

1. **Ao escrever**: os dados sao criptografados com AES-256 antes de serem salvos no disco
2. **Ao ler**: os dados sao descriptografados na memoria antes de serem retornados
3. **A chave**: e fornecida via `PRAGMA key = '<sua-chave>';` como o **primeiro comando** apos abrir a conexao

```
Aplicacao → SQL query → SQLCipher Engine
                              |
                    ┌─────────┴─────────┐
                    │  AES-256 encrypt   │
                    │  (ao escrever)     │
                    │                    │
                    │  AES-256 decrypt   │
                    │  (ao ler)          │
                    └─────────┬─────────┘
                              |
                         Arquivo .db
                     (bytes criptografados)
```

> **Importante:** O `PRAGMA key` DEVE ser o primeiro comando executado apos abrir a conexao. Se voce executar qualquer outro comando antes (mesmo um SELECT), o SQLCipher trata o banco como nao-criptografado e falha.

### Por que usar no DopaBlocker?

O DopaBlocker armazena dados sensiveis localmente em cada dispositivo:

- **Blocklist do usuario** — quais sites/apps estao bloqueados
- **Dados da conta** — email, nome, configuracoes
- **Vinculos parentais** — quem e pai, quem e filho, codigos de vinculacao
- **Configuracoes de filtro adulto** — o usuario ativou ou nao

Sem criptografia, qualquer pessoa com acesso ao computador ou celular (ou um malware) poderia:

1. Abrir o arquivo `.db` com um leitor SQLite
2. Ver toda a blocklist e desativar itens diretamente
3. Ver informacoes da conta
4. Em modo parental: o filho poderia modificar o banco para remover bloqueios

Com SQLCipher, o arquivo `.db` e inutil sem a chave:

| Cenario | Sem SQLCipher | Com SQLCipher |
|---------|---------------|---------------|
| Usuario abre o .db com um leitor | Ve tudo | "encrypted or not a database" |
| Malware lê o arquivo | Extrai dados | Bytes sem sentido |
| Filho tenta editar blocklist no disco | Consegue | Impossivel sem a chave |
| Backup do disco cai em maos erradas | Dados expostos | Dados protegidos |

### Implementacao por plataforma

| Plataforma | Biblioteca | Como usar |
|------------|-----------|-----------|
| **Backend (Rust)** | `rusqlite` com feature `bundled-sqlcipher` | Compila o SQLCipher junto com o binario. Chave vem da variavel de ambiente `SQLCIPHER_KEY` |
| **Desktop (Tauri/Rust)** | Mesmo `rusqlite` com `bundled-sqlcipher` | Chave pode ser derivada de um segredo no app ou lida do keystore do Windows |
| **Mobile (Flutter)** | Planejado: `sqflite_sqlcipher` (pacote Dart) | Drop-in replacement do `sqflite`. Passa `password` no `openDatabase()` |

A feature `bundled-sqlcipher` e importante porque ela **compila o SQLCipher inteiro dentro do binario** — nao precisa instalar nada no sistema do usuario. O app ja sai com tudo embutido.

### Onde fica no codigo?

- **Backend**: `backend/src/config.rs` (carrega SQLCIPHER_KEY), conexao aberta em `main.rs` com PRAGMA key
- **Desktop**: `desktop/src-tauri/src/db.rs` — init_db abre SQLCipher com PRAGMA key
- **Mobile**: `mobile/lib/core/database_service.dart` — placeholder; deve abrir SQLCipher com password na v0.2

---

## Resumo: Como tudo se conecta

```
┌─────────────────────────────────────────────────────────────────┐
│                         USUARIO                                  │
│                    (Desktop ou Mobile)                            │
└────────────────┬────────────────────────────┬────────────────────┘
                 │                            │
                 v                            v
    ┌────────────────────┐      ┌──────────────────────────┐
    │  DESKTOP (Windows)  │      │   MOBILE (Android)        │
    │                    │      │                          │
    │  SvelteKit (UI)    │      │  Flutter (UI)            │
    │       │            │      │  Riverpod (estado)       │
    │  Tauri Bridge      │      │       │                  │
    │       │            │      │  Method Channel          │
    │  ┌────┴──────────┐ │      │       │                  │
    │  │ WFP           │ │      │  ┌────┴──────────┐       │
    │  │ (redireciona   │ │      │  │ VPN Service    │       │
    │  │  trafego DNS)  │ │      │  │ (intercepta    │       │
    │  └───────┬───────┘ │      │  │  trafego)      │       │
    │          │         │      │  └───────┬───────┘       │
    │  ┌───────┴───────┐ │      │  ┌───────┴───────┐       │
    │  │ DNS Proxy      │ │      │  │ DNS Resolver   │       │
    │  │ (resolve ou    │ │      │  │ (resolve ou    │       │
    │  │  bloqueia)     │ │      │  │  bloqueia)     │       │
    │  └───────┬───────┘ │      │  └───────┬───────┘       │
    │          │         │      │          │               │
    │  ┌───────┴───────┐ │      │  ┌───────┴───────┐       │
    │  │ Bloom Filter   │ │      │  │ Bloom Filter   │       │
    │  │ (adulto?)      │ │      │  │ (adulto?)      │       │
    │  └───────────────┘ │      │  └───────────────┘       │
    └─────────┬──────────┘      └────────────┬─────────────┘
              │                              │
              │     Firebase JWT             │
              │  ┌──────────────────────┐    │
              └──┤  BACKEND (Rust/Axum) ├────┘
                 │                      │
                 │  Valida JWT          │
                 │  CRUD blocklist      │
                 │  Gestao dispositivos │
                 │  Vinculacao parental │
                 │                      │
                 │  ┌────────────────┐  │
                 │  │SQLCipher (dados)│  │
                 │  └────────────────┘  │
                 │  ┌────────────────┐  │
                 │  │ REST sync      │  │
                 │  │ via backend    │  │
                 │  └────────────────┘  │
                 └──────────────────────┘
                 ┌──────────────────────┐
                 │  Docker (empacota    │
                 │  tudo acima para     │
                 │  dev e producao)     │
                 └──────────────────────┘
```

### Fluxo de uma requisicao de bloqueio (exemplo completo)

1. O usuario adiciona `instagram.com` na blocklist pelo app desktop
2. O SvelteKit chama o Tauri Bridge, que chama o backend via API REST
3. O backend **valida o Firebase JWT** do usuario
4. O backend salva `instagram.com` no SQLCipher (criptografado)
5. Outros clientes da mesma conta recebem a mudanca via sincronizacao REST/polling
6. No desktop: o **WFP** ja esta redirecionando todo DNS para o **DNS Proxy** local
7. Quando o navegador tenta acessar `instagram.com`, o DNS Proxy:
  - Checa a blocklist do usuario → encontra `instagram.com` → bloqueia
  - (Tambem checa o **Bloom Filter** para conteudo adulto, se ativado)
8. O DNS Proxy responde `127.0.0.1` para registros A bloqueados e o navegador cai na pagina local de bloqueio
9. No mobile v0.2: o **VPN Service** deve fazer o mesmo processo quando o celular tentar resolver o dominio
