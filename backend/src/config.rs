// =============================================================================
// Configuração do backend — lê variáveis de ambiente em `AppConfig::init()`.
// =============================================================================
// Este módulo centraliza TODA a configuração que vem do ambiente externo.
// Nenhum outro módulo deve chamar `std::env::var` diretamente — sempre
// passa por aqui. Isso facilita:
//   - Testar com configs custom (bastaria um construtor alternativo).
//   - Saber em um só lugar quais env vars o backend lê.
//   - Trocar a fonte de config (AWS Parameter Store, Vault, etc.) sem
//     mexer no resto do código.
//
// Env vars lidas:
//   PORT                → porta TCP (default 3000)
//   DATABASE_PATH       → caminho do arquivo .db (default "dopablocker.db")
//   SQLCIPHER_KEY       → chave AES do SQLCipher (default inseguro em dev)
//   FIREBASE_PROJECT_ID → usado na validação de `iss` e `aud` do Firebase JWT
// =============================================================================

use dotenvy::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    /// Caminho no disco do arquivo .db do SQLCipher. Relativo ao cwd por padrão.
    pub database_path: String,
    /// Chave AES-256 usada pelo `PRAGMA key`. Se vazia ou errada, o banco
    /// não descriptografa e qualquer query retorna erro.
    pub database_key: String,
    /// Project ID do Firebase (ex: "dopablocker-prod"). Usado para validar:
    ///   iss = "https://securetoken.google.com/<project_id>"
    ///   aud = "<project_id>"
    /// dos JWTs emitidos pelo Firebase Auth.
    pub firebase_project_id: String,
}

impl AppConfig {
    pub fn init() -> Self {
        // `dotenv()` carrega `.env` no `std::env` se o arquivo existir.
        // O `let _ = ...` silencia o erro caso o arquivo não exista —
        // isso é esperado em produção, onde as vars vêm do ambiente real.
        let _ = dotenv();

        // `.expect(...)` é intencional: se PORT for lixo, queremos falhar
        // ALTO e rápido, não iniciar um servidor numa porta errada.
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".into())
            .parse::<u16>()
            .expect("A variável PORT deve ser um número válido");

        let database_path = env::var("DATABASE_PATH")
            .unwrap_or_else(|_| "dopablocker.db".into());

        // ATENÇÃO: o default "dev-only-unsafe-key" existe só para conveniência
        // local. Em produção, SQLCIPHER_KEY DEVE vir de um secret manager
        // (K8s secret, Vault, etc.). Rodar com a chave default é equivalente
        // a rodar sem criptografia.
        let database_key = env::var("SQLCIPHER_KEY")
            .unwrap_or_else(|_| "dev-only-unsafe-key".into());

        let firebase_project_id = env::var("FIREBASE_PROJECT_ID")
            .unwrap_or_else(|_| "dopablocker-dev".into());

        Self {
            port,
            database_path,
            database_key,
            firebase_project_id,
        }
    }
}
