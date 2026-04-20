# DopaBlocker Backend

Este é o componente Backend (API REST) do ecossistema DopaBlocker. Ele atua como o servidor central de sincronização, autenticação e gerenciamento da Blocklist para as plataformas Mobile e Desktop. 

A API foi construída em **Rust** utilizando o framework **Axum**, focando em alta performance, robustez e separação clara de responsabilidades através do conceito de *Services*.

---

## 🛠️ Stack Tecnológica

* **Linguagem:** Rust (Edition 2021)
* **Web Framework:** Axum
* **Servidor Assíncrono:** Tokio
* **Serialização:** Serde (`serde_json`)
* **Variáveis de Ambiente:** dotenvy
* **Tratamento de Erros:** Respostas padronizadas JSON (`AppError`)
* **Compartilhamento de Dados:** Utiliza a crate local `dopablocker-shared`

---

## 📁 Estrutura de Arquivos

```text
backend/src/
├── main.rs            # Ponto de entrada (Setup do Server, Router principal e AppState)
├── config.rs          # Inicializa variáveis de ambiente (.env)
├── errors.rs          # Definição e abstração global de Erros (Transforma erro de Rust em HTTP Status)
├── middleware.rs      # Interceptores de chamadas (ex: Validação de Token de Autenticação)
├── models.rs          # DTOs (Request e Response payloads da API) específicos da API
├── routes/            # Definição dos métodos HTTP (GET, POST) dos fluxos principais
│   ├── auth.rs
│   ├── devices.rs
│   └── blocklist.rs
└── services/          # A Lógica de negócio e comunicação com Banco de Dados
    ├── user_service.rs
    ├── device_service.rs
    └── blocklist_service.rs
```

---

## 🏗️ Padrões de Arquitetura

O Backend adota o padrão de "Rotas chamam Serviços". Isso blinda a regra de negócio e torna os testes unitários independentes do servidor web.

1. **A Requisição:** O Frontend chama a API (`/api/auth/register`).
2. **O Middleware (`middleware.rs`):** Se a rota for protegida, o middleware extrai o cabeçalho `Authorization: Bearer <token>`, valida e injeta os dados do Usuário Atual (*Current User*) no contexto da requisição (`Router State / Extension`).
3. **A Rota (`routes/`):** A Rota valida formalmente e processa o JSON (usando as estruturas de `models.rs`). Em seguida, chama o `service` repassando o Payload.
4. **O Serviço (`services/`):** Aplica a regra de negócio (Bloqueios, verificação da Base de Dados) e devolve a Resposta ou levanta um `AppError` na pipeline global se algo falhar.
5. **O Retorno:** A requisição responde o cliente com Status `200 OK` + JSON de Sucesso ou Intercepta e formata um Status de Erro sem explodir o Rust (ex: `400 Bad Request`).

### Exemplo de Fluxo

**Em `routes/auth.rs`**:
```rust
async fn register_handler(Json(payload): Json<CreateUserRequest>) -> Result<Json<UserResponse>, AppError> {
    // A rota apenas recebe e repassa para o service
    let user = user_service::create_user(payload).await?;
    
    Ok(Json(UserResponse { message: "Sucesso".into(), user }))
}
```

**Em `services/user_service.rs`**:
```rust
pub async fn create_user(payload: CreateUserRequest) -> Result<User, AppError> {
    // Realiza transações de banco de dados e validações complexas.
    // Levanta erro globalmente com a sintaxe '?' em caso de falha.
}
```

---

## 🔒 Tratamento de Erros

No DopaBlocker, **nunca usamos `unwrap()`** diretamente dentro da regra de negócio para evitar quebras do servidor (Panic). Ao invés disso, propagamos o erro utilizando o enumerador `AppError` localizado em `errors.rs`.

O `AppError` implementa a Trait `IntoResponse` do Axum. Isso significa que podemos retornar um tipo nativo do Rust e ele "magicamente" o converte para o Frontend no formato JSON de falha correto:

```json
{
  "error": "Descrição clara do que houve de errado"
}
```

---

## 🚀 Como Executar

O Backend requer o pacote da crate interna `dopablocker-shared`. O build resolve isso automaticamente, pois fazemos parte do Workspace raiz do Rust.

1. **Clone das configurações:** Crie um arquivo `.env` na raiz do `backend` (baseado no `.env.example`).
2. **Execute via Cargo**:
```bash
cargo run
# ou apenas para validar a sintaxe e compilação, sem subir o server:
cargo check
```

A API será exposta por padrão na porta `3000` (conforme definido pelo seu `config.rs`), acessível primordialmente via `http://localhost:3000/`.

---

## 🔐 Setup do SQLCipher e OpenSSL (Especial para Windows)

O banco de dados utiliza a crate `rusqlite` com a feature `bundled-sqlcipher` para garantir a criptografia AES-256 do arquivo `.db`. O SQLCipher exige o **OpenSSL** para compilar, o que requer uma configuração especial no ambiente Windows.

### Passo a passo com `vcpkg`:
1. Clone o `vcpkg` dentro da pasta do seu projeto (ou numa pasta geral de ferramentas) e instale-o:
   ```powershell
   git clone https://github.com/microsoft/vcpkg.git
   cd vcpkg
   .\bootstrap-vcpkg.bat
   ```
2. Baixe e compile o OpenSSL via vcpkg:
   ```powershell
   .\vcpkg install openssl:x64-windows
   ```
3. Antes de compilar o backend em Rust, defina as Variáveis de Ambiente no terminal apontando para a instalação do OpenSSL:
   ```powershell
   $env:OPENSSL_DIR="c:\<seu-caminho-ate-o-projeto>\vcpkg\installed\x64-windows"
   $env:OPENSSL_NO_VENDOR="1"
   ```
4. **Lidando com o erro de DLL (STATUS_DLL_NOT_FOUND):** Como a biblioteca compilada é dinâmica, o binário final precisa saber onde as DLLs do OpenSSL estão na hora da execução. Para isso, você pode adicionar a pasta `bin` temporariamente ao seu `PATH`:
   ```powershell
   $env:PATH += ";c:\<seu-caminho-ate-o-projeto>\vcpkg\installed\x64-windows\bin"
   cargo run --bin dopablocker-backend
   ```
   *(Alternativa Permanente: Copiar e colar os arquivos `libcrypto-3-x64.dll` e `libssl-3-x64.dll` diretamente para a sua pasta `target/debug/` do projeto).*

---

## 📝 Changelog Recente

- **Atualização do Axum (Compatibilidade v0.7 / v0.8):** A sintaxe de definição de captura de parâmetros (path variables) sofreu *breaking changes*. Rotas definidas anteriormente como `/:id` foram corrigidas para a nova especificação baseada em chaves `/{id}` no arquivo `src/routes/blocklist.rs` para prevenir panics na inicialização do servidor.
