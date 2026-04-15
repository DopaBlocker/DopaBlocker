use rusqlite::{params, OptionalExtension};
use tokio_rusqlite::Connection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{BlockMode, User};

fn block_mode_to_str(mode: &BlockMode) -> &'static str {
    match mode {
        BlockMode::Personal => "personal",
        BlockMode::Parental => "parental",
    }
}

fn str_to_block_mode(s: &str) -> BlockMode {
    match s {
        "parental" => BlockMode::Parental,
        _ => BlockMode::Personal,
    }
}

pub async fn create_user(
    db: &Connection,
    firebase_uid: String,
    email: String,
    display_name: String,
    mode: BlockMode,
) -> Result<User, AppError> {
    let id = Uuid::new_v4().to_string();
    let mode_str = block_mode_to_str(&mode).to_string();
    let id_clone = id.clone();
    let firebase_uid_clone = firebase_uid.clone();
    let email_clone = email.clone();
    let display_name_clone = display_name.clone();

    db.call(move |c| {
        c.execute(
            "INSERT INTO users(id, firebase_uid, email, display_name, mode)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id_clone,
                firebase_uid_clone,
                email_clone,
                display_name_clone,
                mode_str
            ],
        )?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::Conflict(format!("Falha ao criar usuário: {e}")))?;

    get_user_by_firebase_uid(db, firebase_uid)
        .await?
        .ok_or_else(|| AppError::InternalServerError("User recém-criado não encontrado".into()))
}

pub async fn get_user_by_firebase_uid(
    db: &Connection,
    firebase_uid: String,
) -> Result<Option<User>, AppError> {
    db.call(move |c| {
        let r = c
            .query_row(
                "SELECT id, firebase_uid, email, display_name, mode, created_at
                 FROM users WHERE firebase_uid = ?1",
                params![firebase_uid],
                |row| {
                    Ok(User {
                        id: row.get(0)?,
                        firebase_uid: row.get(1)?,
                        email: row.get(2)?,
                        display_name: row.get(3)?,
                        mode: str_to_block_mode(&row.get::<_, String>(4)?),
                        created_at: row.get(5)?,
                    })
                },
            )
            .optional()?;
        Ok(r)
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))
}

pub async fn get_user_by_id(db: &Connection, user_id: String) -> Result<User, AppError> {
    db.call(move |c| {
        let r = c
            .query_row(
                "SELECT id, firebase_uid, email, display_name, mode, created_at
                 FROM users WHERE id = ?1",
                params![user_id],
                |row| {
                    Ok(User {
                        id: row.get(0)?,
                        firebase_uid: row.get(1)?,
                        email: row.get(2)?,
                        display_name: row.get(3)?,
                        mode: str_to_block_mode(&row.get::<_, String>(4)?),
                        created_at: row.get(5)?,
                    })
                },
            )
            .optional()?;
        Ok(r)
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
    .ok_or_else(|| AppError::NotFound("Usuário não encontrado".into()))
}
