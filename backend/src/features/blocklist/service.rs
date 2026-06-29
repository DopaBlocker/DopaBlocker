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

use crate::core::errors::AppError;
use crate::core::models::{AdultFilterSettings, BlockedItem, BlockedType, CreateBlockedItemRequest};
use crate::core::util::{blocked_type_to_sql, iso_now, parse_blocked_type};

// Conversões enum ↔ string e helpers de timestamp ISO-8601 vivem em
// `core/util.rs` (compartilhados com as features).

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
    let item_type = blocked_type_to_sql(&payload.item_type).to_string();

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
        created_at: iso_now(),
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
                    item_type: parse_blocked_type(&row.get::<_, String>(2)?),
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

/// Calcula um **ETag fraco** da blocklist do user, para o poll periódico do
/// filho (B2) economizar banda via `If-None-Match` → `304 Not Modified`.
///
/// O validador é `COUNT(*)` + `MAX(created_at)`: cobre adições (mudam contagem
/// e/ou o máximo) e remoções (mudam a contagem). Como o backend não tem um
/// toggle server-side de `is_active` (isso é só client-side), esses dois
/// agregados bastam para detectar qualquer mudança na lista efetiva que o
/// cliente recebe. É barato (uma query agregada, sem materializar linhas).
pub async fn blocklist_etag(db: &Connection, user_id: String) -> Result<String, AppError> {
    db.call(move |c| {
        let (count, max_created): (i64, Option<String>) = c.query_row(
            "SELECT COUNT(*), MAX(created_at) FROM blocked_items WHERE user_id = ?1",
            params![user_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok(format_blocklist_etag(count, max_created.as_deref()))
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))
}

/// Formata o ETag (entre aspas, como manda o HTTP) a partir dos agregados.
/// Pura/determinística para ser testável sem banco.
fn format_blocklist_etag(count: i64, max_created: Option<&str>) -> String {
    format!("\"{}-{}\"", count, max_created.unwrap_or_default())
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

#[cfg(test)]
mod tests {
    use super::format_blocklist_etag;

    #[test]
    fn etag_is_quoted_and_combines_count_and_timestamp() {
        let etag = format_blocklist_etag(3, Some("2026-06-21T10:00:00Z"));
        assert_eq!(etag, "\"3-2026-06-21T10:00:00Z\"");
    }

    #[test]
    fn etag_changes_with_count_and_handles_empty_list() {
        // Lista vazia → sem max(created_at).
        assert_eq!(format_blocklist_etag(0, None), "\"0-\"");
        // Mesma data, contagem diferente → ETag diferente (detecta remoção).
        let a = format_blocklist_etag(2, Some("2026-06-21T10:00:00Z"));
        let b = format_blocklist_etag(1, Some("2026-06-21T10:00:00Z"));
        assert_ne!(a, b);
    }
}
