// =============================================================================
// Cache local criptografado do desktop — SQLCipher via rusqlite + tokio-rusqlite.
// =============================================================================
// O desktop mantém uma cópia local da blocklist para que o engine de bloqueio
// (DNS proxy + WFP) consulte regras sem depender de rede. Fonte-da-verdade é
// o backend; aqui é cache. Criptografia (SQLCipher) protege contra leitura do
// arquivo .db por quem tem acesso ao disco mas não à chave.
//
// A chave SQLCipher é gerada na primeira execução (32 bytes aleatórios, hex)
// e guardada no **Windows Credential Manager** via `keyring`. Nunca fica em
// texto plano no disco. Se o usuário reinstalar o app mas mantiver a
// credencial, o banco permanece legível; se perder a credencial, o .db vira
// lixo (esperado — não queremos recuperação sem a chave).
// =============================================================================

use std::path::PathBuf;

use rand::RngCore;
use rusqlite::params;
use tauri::{AppHandle, Manager};
use thiserror::Error;
use tokio_rusqlite::Connection;

use dopablocker_shared::models::{BlockedItem, BlockedType};

const KEYRING_SERVICE: &str = "DopaBlocker";
const KEYRING_USER: &str = "sqlcipher-db-key";
const DB_FILENAME: &str = "dopablocker-local.db";

// Embutidas no binário — um `cargo build` gera um executável auto-contido
// que carrega seu próprio schema. Para adicionar migration nova, criar
// 002_xxx.sql e acrescentar aqui, nunca editar uma já em produção.
const MIGRATIONS: &[(&str, &str)] = &[(
    "001_local_cache",
    include_str!("../migrations/001_local_cache.sql"),
)];

#[derive(Debug, Error)]
pub enum DbError {
    #[error("falha ao resolver app data dir: {0}")]
    AppDataDir(String),
    #[error("falha ao criar diretório: {0}")]
    Io(#[from] std::io::Error),
    #[error("keyring: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("sqlite: {0}")]
    Sqlite(#[from] tokio_rusqlite::Error),
}

pub type DbResult<T> = Result<T, DbError>;

// -------- boot ---------------------------------------------------------------

/// Abre (ou cria) o banco local criptografado e aplica migrations. Deve ser
/// chamada uma única vez no `setup` do Tauri. A `Connection` retornada é
/// `Clone` (wrap de Arc) — guardar como `State` e clonar nos comandos.
pub async fn init(app: &AppHandle) -> DbResult<Connection> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| DbError::AppDataDir(e.to_string()))?;
    std::fs::create_dir_all(&data_dir)?;
    let db_path = data_dir.join(DB_FILENAME);

    let key = load_or_create_key()?;
    let conn = Connection::open(db_path_string(&db_path)).await?;

    // PRAGMA key é obrigatoriamente o PRIMEIRO comando. Qualquer outro
    // antes (mesmo um SELECT 1) faz o SQLCipher tratar o arquivo como
    // texto plano e falhar todas as queries com "file is not a database".
    conn.call(move |c| {
        c.pragma_update(None, "key", &key)?;
        c.pragma_update(None, "foreign_keys", &"ON")?;
        Ok(())
    })
    .await?;

    apply_migrations(&conn).await?;
    tracing::info!(path = %db_path.display(), "DB local pronto");
    Ok(conn)
}

fn db_path_string(p: &PathBuf) -> String {
    p.to_string_lossy().into_owned()
}

/// Lê a chave do Credential Manager; se não existir, gera 32 bytes aleatórios,
/// serializa em hex e persiste. Idempotente após a primeira execução.
fn load_or_create_key() -> DbResult<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    match entry.get_password() {
        Ok(existing) => Ok(existing),
        Err(keyring::Error::NoEntry) => {
            let mut bytes = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut bytes);
            let hex = to_hex(&bytes);
            entry.set_password(&hex)?;
            tracing::info!("Chave SQLCipher gerada e salva no Credential Manager");
            Ok(hex)
        }
        Err(e) => Err(e.into()),
    }
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}

async fn apply_migrations(conn: &Connection) -> DbResult<()> {
    conn.call(|c| {
        c.execute(
            "CREATE TABLE IF NOT EXISTS _migrations (
                 name       TEXT PRIMARY KEY,
                 applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
             )",
            [],
        )?;
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
    .await?;
    Ok(())
}

// -------- BlockedItem CRUD ---------------------------------------------------

pub async fn list_blocked_items(conn: &Connection, user_id: String) -> DbResult<Vec<BlockedItem>> {
    let items = conn
        .call(move |c| {
            let mut stmt = c.prepare(
                "SELECT id, user_id, item_type, value, is_active, created_at
                 FROM blocked_items_cache
                 WHERE user_id = ?1
                 ORDER BY created_at DESC",
            )?;
            let rows = stmt.query_map(params![user_id], |row| {
                Ok(BlockedItem {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    item_type: parse_type(&row.get::<_, String>(2)?),
                    value: row.get(3)?,
                    is_active: row.get::<_, i64>(4)? != 0,
                    created_at: row.get(5)?,
                })
            })?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            Ok(out)
        })
        .await?;
    Ok(items)
}

/// Lista só os domínios ativos — forma que o DNS proxy espera. Evita carregar
/// BlockedItem inteiro quando o engine só precisa de `Vec<String>`.
pub async fn list_active_domains(conn: &Connection, user_id: String) -> DbResult<Vec<String>> {
    let domains = conn
        .call(move |c| {
            let mut stmt = c.prepare(
                "SELECT value FROM blocked_items_cache
                 WHERE user_id = ?1 AND item_type = 'domain' AND is_active = 1",
            )?;
            let rows = stmt.query_map(params![user_id], |r| r.get::<_, String>(0))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            Ok(out)
        })
        .await?;
    Ok(domains)
}

pub async fn upsert_blocked_item(conn: &Connection, item: BlockedItem) -> DbResult<()> {
    conn.call(move |c| {
        c.execute(
            "INSERT INTO blocked_items_cache
                 (id, user_id, item_type, value, is_active, created_at, synced_at)
             VALUES
                 (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%dT%H:%M:%SZ','now'))
             ON CONFLICT(id) DO UPDATE SET
                 item_type = excluded.item_type,
                 value     = excluded.value,
                 is_active = excluded.is_active,
                 synced_at = excluded.synced_at",
            params![
                item.id,
                item.user_id,
                type_to_str(&item.item_type),
                item.value,
                item.is_active as i64,
                item.created_at,
            ],
        )?;
        Ok(())
    })
    .await?;
    Ok(())
}

pub async fn delete_blocked_item(conn: &Connection, id: String) -> DbResult<()> {
    conn.call(move |c| {
        c.execute(
            "DELETE FROM blocked_items_cache WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    })
    .await?;
    Ok(())
}

/// Substitui o cache inteiro de um usuário atomicamente — usado pelo
/// `sync_with_backend` para reconciliar adições/remoções remotas em um passo
/// só, sem precisar diff client-side.
pub async fn replace_all_for_user(
    conn: &Connection,
    user_id: String,
    items: Vec<BlockedItem>,
) -> DbResult<()> {
    conn.call(move |c| {
        let tx = c.transaction()?;
        tx.execute(
            "DELETE FROM blocked_items_cache WHERE user_id = ?1",
            params![user_id],
        )?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO blocked_items_cache
                     (id, user_id, item_type, value, is_active, created_at, synced_at)
                 VALUES
                     (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
            )?;
            for item in &items {
                stmt.execute(params![
                    item.id,
                    item.user_id,
                    type_to_str(&item.item_type),
                    item.value,
                    item.is_active as i64,
                    item.created_at,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    })
    .await?;
    Ok(())
}

// -------- state key-value ----------------------------------------------------

pub async fn set_state(conn: &Connection, key: &'static str, value: String) -> DbResult<()> {
    conn.call(move |c| {
        c.execute(
            "INSERT INTO blocking_state(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    })
    .await?;
    Ok(())
}

pub async fn get_state(conn: &Connection, key: &'static str) -> DbResult<Option<String>> {
    let v = conn
        .call(move |c| {
            let mut stmt = c.prepare("SELECT value FROM blocking_state WHERE key = ?1")?;
            let mut rows = stmt.query(params![key])?;
            match rows.next()? {
                Some(row) => Ok(Some(row.get::<_, String>(0)?)),
                None => Ok(None),
            }
        })
        .await?;
    Ok(v)
}

pub async fn set_blocking_enabled(conn: &Connection, enabled: bool) -> DbResult<()> {
    set_state(conn, "blocking_enabled", enabled.to_string()).await
}

pub async fn get_blocking_enabled(conn: &Connection) -> DbResult<bool> {
    Ok(get_state(conn, "blocking_enabled")
        .await?
        .map(|v| v == "true")
        .unwrap_or(false))
}

pub async fn set_adult_filter_enabled(conn: &Connection, enabled: bool) -> DbResult<()> {
    set_state(conn, "adult_filter_enabled", enabled.to_string()).await
}

pub async fn get_adult_filter_enabled(conn: &Connection) -> DbResult<bool> {
    Ok(get_state(conn, "adult_filter_enabled")
        .await?
        .map(|v| v == "true")
        .unwrap_or(false))
}

/// Lembra qual usuário estava ativo na última vez que o engine foi ligado.
/// O `lib.rs::setup` usa isso para reativar o engine no boot — sem esse
/// pointer, a gente não saberia de qual user carregar as regras.
pub async fn set_last_active_user_id(conn: &Connection, user_id: String) -> DbResult<()> {
    set_state(conn, "last_active_user_id", user_id).await
}

pub async fn get_last_active_user_id(conn: &Connection) -> DbResult<Option<String>> {
    get_state(conn, "last_active_user_id").await
}

// -------- helpers ------------------------------------------------------------

fn type_to_str(t: &BlockedType) -> &'static str {
    match t {
        BlockedType::Domain => "domain",
        BlockedType::App => "app",
        BlockedType::Keyword => "keyword",
    }
}

// Se o DB contiver um valor fora do CHECK constraint, algo já quebrou em outro
// lugar — mas mapeamos para Domain como fallback conservador em vez de panicar.
fn parse_type(s: &str) -> BlockedType {
    match s {
        "app" => BlockedType::App,
        "keyword" => BlockedType::Keyword,
        _ => BlockedType::Domain,
    }
}
