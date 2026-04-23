// =============================================================================
// blocklist_service — CRUD da blocklist e toggle do filtro adulto.
// =============================================================================
// A "blocklist" é a lista de itens que o user quer bloquear. Cada item tem
// um `item_type` (domain / app / keyword) e um `value`:
//
//   - domain: "instagram.com" → bloqueia via DNS/hosts/proxy no cliente.
//   - app:    "com.instagram.android" → bloqueia via Android VPN.
//   - keyword: "casino" → bloqueia URL se contiver a substring.
//
// Normalização de input:
//   - `value` é trim + lowercase antes de inserir. Evita duplicatas do tipo
//     " Instagram.com " vs "instagram.com" e dá match case-insensitive no
//     domain matcher do cliente sem esforço extra.
//   - Rejeita `value` vazio com 400 (BadRequest).
//
// O UNIQUE (user_id, item_type, value) do schema previne duplicatas; o erro
// é traduzido para 409 Conflict aqui neste módulo.
//
// O "filtro adulto" é separado da blocklist: tem sua própria tabela
// (`adult_filter_settings`) com UNIQUE por user_id. Usamos UPSERT
// (`ON CONFLICT DO UPDATE`) para simplificar "ligar/desligar" em uma query.
// =============================================================================

use rusqlite::params;
use tokio_rusqlite::Connection;
use uuid::Uuid;

use dopablocker_shared::domain_matcher::normalize_domain;

use crate::errors::AppError;
use crate::models::{AdultFilterSettings, BlockedItem, BlockedType, CreateBlockedItemRequest};

// Persistência como texto — mesma justificativa de `block_mode_to_str` em
// user_service: legibilidade via sqlite3 CLI.
fn item_type_to_str(t: &BlockedType) -> &'static str {
    match t {
        BlockedType::Domain => "domain",
        BlockedType::App => "app",
        BlockedType::Keyword => "keyword",
    }
}

fn str_to_item_type(s: &str) -> BlockedType {
    match s {
        "app" => BlockedType::App,
        "keyword" => BlockedType::Keyword,
        _ => BlockedType::Domain,
    }
}

/// Adiciona item à blocklist. Normaliza (trim + lowercase), valida não-vazio,
/// insere, traduz o erro de UNIQUE para 409.
///
/// Retornamos o `BlockedItem` "manualmente" preenchido (sem SELECT de volta)
/// porque todos os campos são conhecidos em código — evita a segunda query.
/// O `created_at` é calculado com `chrono::Utc::now()` em vez de ler o
/// default do banco; isso pode divergir em microssegundos do que o banco
/// gravou, mas para uso prático não faz diferença.
pub async fn add_item(
    db: &Connection,
    user_id: String,
    payload: CreateBlockedItemRequest,
) -> Result<BlockedItem, AppError> {
    let id = Uuid::new_v4().to_string();
    let item_type = item_type_to_str(&payload.item_type).to_string();

    // Normalização dependente do tipo. Domínios passam pela mesma função que
    // o DNS proxy usa na hora de casar a query — `http://www.X.com/path`,
    // `X.COM` e `x.com` viram todos `x.com`. Sem isso, defesa-em-profundidade
    // falha: um POST direto no backend com `Instagram.COM` seria salvo com
    // maiúsculas e o proxy (que lowercase a query) nunca daria match.
    let value = match payload.item_type {
        BlockedType::Domain => normalize_domain(&payload.value),
        _ => payload.value.trim().to_lowercase(),
    };

    if value.is_empty() {
        return Err(AppError::BadRequest("value não pode ser vazio".into()));
    }

    // Sanity check pós-normalização: domínio sem TLD não é domínio.
    if matches!(payload.item_type, BlockedType::Domain) && !value.contains('.') {
        return Err(AppError::BadRequest(
            "domínio deve conter pelo menos um ponto (ex: instagram.com)".into(),
        ));
    }

    // Clones para a closure `'static` (ver comentário em user_service).
    let id_c = id.clone();
    let user_id_c = user_id.clone();
    let value_c = value.clone();
    let item_type_c = item_type.clone();

    db.call(move |c| {
        c.execute(
            "INSERT INTO blocked_items(id, user_id, item_type, value)
             VALUES (?1, ?2, ?3, ?4)",
            params![id_c, user_id_c, item_type_c, value_c],
        )?;
        Ok(())
    })
    .await
    .map_err(|e| {
        // Duck-typing de erros: o SQLite devolve "UNIQUE constraint failed"
        // como texto dentro de `e.to_string()`. É feio, mas é a forma
        // idiomática em `rusqlite` sem parse de error codes.
        let msg = e.to_string();
        if msg.contains("UNIQUE") {
            AppError::Conflict("Item já existe na blocklist".into())
        } else {
            AppError::InternalServerError(msg)
        }
    })?;

    Ok(BlockedItem {
        id,
        user_id,
        item_type: payload.item_type,
        value,
        is_active: true,
        created_at: chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    })
}

/// Lista items do user ordenados por data decrescente (mais recentes primeiro).
/// Ordenação estável importa quando o cliente renderiza "histórico da blocklist".
pub async fn list_items(db: &Connection, user_id: String) -> Result<Vec<BlockedItem>, AppError> {
    db.call(move |c| {
        let mut stmt = c.prepare(
            "SELECT id, user_id, item_type, value, is_active, created_at
             FROM blocked_items WHERE user_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map(params![user_id], |row| {
                Ok(BlockedItem {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    item_type: str_to_item_type(&row.get::<_, String>(2)?),
                    value: row.get(3)?,
                    // SQLite não tem bool nativo — é INTEGER 0/1.
                    is_active: row.get::<_, i64>(4)? != 0,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))
}

/// Deleta item com segurança: o `AND user_id = ?2` é crítico — sem ele,
/// qualquer user poderia deletar items de outros se adivinhasse o UUID.
/// Se o DELETE não afetou nenhuma linha, retornamos 404 (não existe ou
/// não é seu).
pub async fn delete_item(db: &Connection, user_id: String, id: String) -> Result<(), AppError> {
    let changes = db
        .call(move |c| {
            let n = c.execute(
                "DELETE FROM blocked_items WHERE id = ?1 AND user_id = ?2",
                params![id, user_id],
            )?;
            Ok(n)
        })
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    if changes == 0 {
        return Err(AppError::NotFound("Item não encontrado".into()));
    }
    Ok(())
}

/// Liga/desliga o filtro adulto. UPSERT via `ON CONFLICT DO UPDATE` evita
/// precisar de duas queries (existe? → insert ou update?). A cláusula
/// `excluded.is_enabled` refere-se ao valor que SERIA inserido — jeito
/// idiomático SQLite de dizer "use o valor novo no update".
///
/// Depois do upsert fazemos um SELECT para retornar o row completo,
/// incluindo o `last_list_update` (que é gerenciado por trigger/job
/// separado quando há ingestão de lista externa de domínios adultos).
pub async fn set_adult_filter(
    db: &Connection,
    user_id: String,
    enabled: bool,
) -> Result<AdultFilterSettings, AppError> {
    let user_id_c = user_id.clone();
    db.call(move |c| {
        c.execute(
            "INSERT INTO adult_filter_settings(id, user_id, is_enabled)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(user_id) DO UPDATE SET is_enabled = excluded.is_enabled",
            // O `id` gerado aqui só é usado no INSERT inicial — no UPDATE
            // o PK existente é preservado.
            params![Uuid::new_v4().to_string(), user_id_c, enabled as i64],
        )?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    db.call(move |c| {
        let r = c.query_row(
            "SELECT id, user_id, is_enabled, last_list_update
             FROM adult_filter_settings WHERE user_id = ?1",
            params![user_id],
            |row| {
                Ok(AdultFilterSettings {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    is_enabled: row.get::<_, i64>(2)? != 0,
                    last_list_update: row.get(3)?,
                })
            },
        )?;
        Ok(r)
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))
}
