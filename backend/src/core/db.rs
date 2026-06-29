// =============================================================================
// Camada de banco — abre conexão SQLCipher e aplica migrations.
// =============================================================================
// Por que `tokio_rusqlite` e não `sqlx`?
//   - `sqlx` não suporta SQLCipher nativamente.
//   - `rusqlite` + feature "bundled-sqlcipher" compila o SQLCipher junto ao
//     binário, sem depender de libs do sistema. Zero setup na máquina de
//     desenvolvedor/deploy.
//   - `tokio_rusqlite::Connection` envelopa o `rusqlite::Connection`
//     (que é síncrono) em um thread dedicado + canal async. Do nosso lado,
//     chamamos `.call(|conn| {...})` e recebemos o resultado via future.
//
// Migrations ficam em `backend/migrations/*.sql` e são incorporadas ao
// binário via `include_str!` — assim, um `cargo build` produz um executável
// auto-contido que carrega seu próprio schema.
// =============================================================================

use rusqlite::params;
use tokio_rusqlite::Connection;

use crate::errors::AppError;

// Lista ordenada (por nome) das migrations. `include_str!` injeta o conteúdo
// SQL no binário em tempo de compilação — se algum arquivo estiver faltando,
// o build falha, o que é o comportamento desejado.
//
// Para adicionar uma nova migration:
//   1. Crie `backend/migrations/003_<slug>.sql`.
//   2. Adicione `("003_<slug>", include_str!("../migrations/003_<slug>.sql"))`
//      ao final desta lista.
//   3. Nunca edite migrations já aplicadas em produção — crie uma nova.
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial", include_str!("../migrations/001_initial.sql")),
    (
        "002_parental_fixes",
        include_str!("../migrations/002_parental_fixes.sql"),
    ),
    (
        "003_email_verification",
        include_str!("../migrations/003_email_verification.sql"),
    ),
    (
        "004_device_events",
        include_str!("../migrations/004_device_events.sql"),
    ),
];

/// Abre o arquivo `.db` e aplica imediatamente o `PRAGMA key` que
/// descriptografa o banco. Se a chave estiver errada, qualquer query
/// posterior falha com "file is not a database".
///
/// Também liga `foreign_keys = ON` — o SQLite tem FKs desabilitadas por
/// padrão (!) por motivos históricos. Sem isso, os `REFERENCES` das
/// migrations seriam apenas decorativos.
pub async fn connect(path: &str, key: &str) -> Result<Connection, AppError> {
    let conn = Connection::open(path)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Falha ao abrir DB: {e}")))?;

    let key = key.to_string();
    conn.call(move |c| {
        // IMPORTANTE: `PRAGMA key` DEVE ser o primeiro comando. Qualquer
        // outra query antes dele (mesmo um `SELECT 1`) faz o SQLCipher
        // tratar o banco como texto claro e falhar.
        c.pragma_update(None, "key", &key)?;
        c.pragma_update(None, "foreign_keys", &"ON")?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::InternalServerError(format!("Falha ao configurar DB: {e}")))?;

    Ok(conn)
}

/// Aplica todas as migrations ainda não aplicadas, em ordem.
///
/// Usa uma tabela `_migrations` para controlar o que já rodou. É
/// idempotente: rodar esta função N vezes tem o mesmo efeito que 1 vez.
/// Isso permite chamá-la sempre na subida do servidor sem medo.
pub async fn run_migrations(conn: &Connection) -> Result<(), AppError> {
    conn.call(|c| {
        // Tabela de controle. O `IF NOT EXISTS` faz o primeiro boot funcionar.
        c.execute(
            "CREATE TABLE IF NOT EXISTS _migrations (
                name       TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
             )",
            [],
        )?;

        // Para cada migration declarada em MIGRATIONS:
        //   - Se já está em _migrations → pula.
        //   - Se não → roda o SQL inteiro via `execute_batch` (aceita múltiplos
        //     statements separados por ';') e registra na tabela de controle.
        for (name, sql) in MIGRATIONS {
            let already: i64 = c.query_row(
                "SELECT COUNT(*) FROM _migrations WHERE name = ?1",
                params![name],
                |r| r.get(0),
            )?;
            if already == 0 {
                c.execute_batch(sql)?;
                c.execute("INSERT INTO _migrations(name) VALUES (?1)", params![name])?;
                tracing::info!(migration = name, "Migration aplicada");
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| AppError::InternalServerError(format!("Falha em migrations: {e}")))?;

    Ok(())
}
