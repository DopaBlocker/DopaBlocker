use chrono::Utc;
use rand::Rng;
use rusqlite::{params, OptionalExtension};
use tokio_rusqlite::Connection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::hash_device_token;
use crate::models::{
    ConfirmLinkRequest, ConfirmLinkResponse, Device, GenerateLinkCodeResponse, Platform,
    RegisterDeviceRequest,
};

const LINK_CODE_TTL_SECS: i64 = 5 * 60;

fn platform_to_str(p: &Platform) -> &'static str {
    match p {
        Platform::Windows => "windows",
        Platform::Android => "android",
    }
}

fn str_to_platform(s: &str) -> Platform {
    match s {
        "android" => Platform::Android,
        _ => Platform::Windows,
    }
}

pub async fn register_device(
    db: &Connection,
    user_id: String,
    payload: RegisterDeviceRequest,
) -> Result<Device, AppError> {
    let id = Uuid::new_v4().to_string();
    let platform_str = platform_to_str(&payload.platform).to_string();
    let id_c = id.clone();
    let user_id_c = user_id.clone();
    let device_name_c = payload.device_name.clone();
    let platform_c = platform_str.clone();

    db.call(move |c| {
        c.execute(
            "INSERT INTO devices(id, user_id, device_name, platform, is_child)
             VALUES (?1, ?2, ?3, ?4, 0)",
            params![id_c, user_id_c, device_name_c, platform_c],
        )?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(Device {
        id,
        user_id,
        device_name: payload.device_name,
        platform: payload.platform,
        is_child: false,
        created_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    })
}

pub async fn list_devices(db: &Connection, user_id: String) -> Result<Vec<Device>, AppError> {
    db.call(move |c| {
        let mut stmt = c.prepare(
            "SELECT id, user_id, device_name, platform, is_child, created_at
             FROM devices WHERE user_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map(params![user_id], |row| {
                Ok(Device {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    device_name: row.get(2)?,
                    platform: str_to_platform(&row.get::<_, String>(3)?),
                    is_child: row.get::<_, i64>(4)? != 0,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))
}

pub async fn generate_link_code(
    db: &Connection,
    parent_user_id: String,
) -> Result<GenerateLinkCodeResponse, AppError> {
    let parent_device_id: Option<String> = db
        .call({
            let uid = parent_user_id.clone();
            move |c| {
                let r = c
                    .query_row(
                        "SELECT id FROM devices WHERE user_id = ?1 AND is_child = 0
                         ORDER BY created_at ASC LIMIT 1",
                        params![uid],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()?;
                Ok(r)
            }
        })
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let parent_device_id = parent_device_id.ok_or_else(|| {
        AppError::BadRequest(
            "Nenhum device do pai registrado — chame /devices/register primeiro".into(),
        )
    })?;

    let code = {
        let mut rng = rand::thread_rng();
        format!("{:06}", rng.gen_range(0..1_000_000))
    };
    let expires_at = (Utc::now() + chrono::Duration::seconds(LINK_CODE_TTL_SECS))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let id = Uuid::new_v4().to_string();
    let code_c = code.clone();
    let expires_c = expires_at.clone();
    db.call(move |c| {
        c.execute(
            "INSERT INTO parental_links(id, parent_device_id, link_code, status, expires_at)
             VALUES (?1, ?2, ?3, 'pending', ?4)",
            params![id, parent_device_id, code_c, expires_c],
        )?;
        Ok(())
    })
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("UNIQUE") {
            AppError::Conflict("Código já em uso — tente novamente".into())
        } else {
            AppError::InternalServerError(msg)
        }
    })?;

    Ok(GenerateLinkCodeResponse { code, expires_at })
}

pub async fn confirm_link(
    db: &Connection,
    payload: ConfirmLinkRequest,
) -> Result<ConfirmLinkResponse, AppError> {
    let code = payload.code;
    let device_name = payload.device_name;
    let platform_str = platform_to_str(&payload.platform).to_string();
    let now_iso = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let plain_token = format!(
        "{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let token_hash = hash_device_token(&plain_token);

    let result = db
        .call({
            let code = code.clone();
            let device_name = device_name.clone();
            let platform_str = platform_str.clone();
            let now_iso = now_iso.clone();
            let token_hash = token_hash.clone();
            move |c| {
                let tx = c.transaction()?;

                let link: Option<(String, String, String)> = tx
                    .query_row(
                        "SELECT id, parent_device_id, expires_at FROM parental_links
                         WHERE link_code = ?1 AND status = 'pending'",
                        params![code],
                        |r| {
                            Ok((
                                r.get::<_, String>(0)?,
                                r.get::<_, String>(1)?,
                                r.get::<_, String>(2)?,
                            ))
                        },
                    )
                    .optional()?;

                let (link_id, parent_device_id, expires_at) = match link {
                    Some(v) => v,
                    None => {
                        return Err(tokio_rusqlite::Error::Other(
                            Box::<dyn std::error::Error + Send + Sync>::from("LINK_NOT_FOUND"),
                        ));
                    }
                };

                if expires_at.as_str() < now_iso.as_str() {
                    return Err(tokio_rusqlite::Error::Other(
                        Box::<dyn std::error::Error + Send + Sync>::from("LINK_EXPIRED"),
                    ));
                }

                let parent_user_id: String = tx.query_row(
                    "SELECT user_id FROM devices WHERE id = ?1",
                    params![parent_device_id],
                    |r| r.get(0),
                )?;

                let child_device_id = Uuid::new_v4().to_string();
                tx.execute(
                    "INSERT INTO devices(id, user_id, device_name, platform, is_child)
                     VALUES (?1, ?2, ?3, ?4, 1)",
                    params![
                        child_device_id,
                        parent_user_id,
                        device_name,
                        platform_str
                    ],
                )?;

                tx.execute(
                    "UPDATE parental_links
                     SET child_device_id = ?1, status = 'active'
                     WHERE id = ?2",
                    params![child_device_id, link_id],
                )?;

                tx.execute(
                    "INSERT INTO device_tokens(token_hash, device_id, user_id)
                     VALUES (?1, ?2, ?3)",
                    params![token_hash, child_device_id, parent_user_id],
                )?;

                tx.commit()?;

                Ok((child_device_id, parent_user_id, parent_device_id))
            }
        })
        .await;

    let (child_device_id, parent_user_id, parent_device_id) = result.map_err(|e| {
        let msg = e.to_string();
        if msg.contains("LINK_NOT_FOUND") {
            AppError::BadRequest("Código inválido ou já utilizado".into())
        } else if msg.contains("LINK_EXPIRED") {
            AppError::BadRequest("Código expirado".into())
        } else {
            AppError::InternalServerError(msg)
        }
    })?;

    Ok(ConfirmLinkResponse {
        device_token: format!("dt_{}", plain_token),
        device_id: child_device_id,
        user_id: parent_user_id,
        parent_device_id,
    })
}
