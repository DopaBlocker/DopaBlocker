use rusqlite::params;
use tokio_rusqlite::Connection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{AdultFilterSettings, BlockedItem, BlockedType, CreateBlockedItemRequest};

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

pub async fn add_item(
    db: &Connection,
    user_id: String,
    payload: CreateBlockedItemRequest,
) -> Result<BlockedItem, AppError> {
    let id = Uuid::new_v4().to_string();
    let item_type = item_type_to_str(&payload.item_type).to_string();
    let value = payload.value.trim().to_lowercase();

    if value.is_empty() {
        return Err(AppError::BadRequest("value não pode ser vazio".into()));
    }

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
