// =============================================================================
// user_service — CRUD mínimo da tabela `users`.
// =============================================================================
// Responsabilidades:
//   - Criar user local correspondente a um Firebase UID recém-registrado.
//   - Buscar user pelo `firebase_uid` (usado em /auth/login e no middleware).
//   - Buscar user pelo `id` interno (usado em /auth/me e outros handlers).
//
// Convenções deste módulo:
//   - Recebemos parâmetros como `String` (consumindo o dado) porque as
//     closures passadas a `db.call()` precisam ser `'static` — ou seja,
//     não podem capturar referências ao escopo do handler. A cópia é barata
//     em comparação com a round-trip ao SQLite.
//   - `BlockMode` é persistido como texto ("personal"/"parental") para
//     legibilidade direta no banco (inspeção via CLI). As conversões
//     ficam nos helpers `*_to_str`/`str_to_*` neste arquivo.
//   - Erros de SQL (incluindo UNIQUE constraint do `firebase_uid`) viram
//     `AppError::Conflict` quando vêm de `create_user` — o caller sabe
//     traduzir para HTTP 409.
// =============================================================================

use rusqlite::{params, OptionalExtension};
use tokio_rusqlite::Connection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{BlockMode, User};
use crate::services::auth_service::normalize_email;
use crate::services::util::{block_mode_to_sql, parse_block_mode};

// Conversões enum ↔ string vivem em `services/util.rs` (block_mode_to_sql /
// parse_block_mode) — antes desta refatoração estavam duplicadas em quatro
// services diferentes.

/// INSERT + SELECT: primeiro insere, depois lê de volta para garantir que
/// os defaults do banco (como `created_at DEFAULT CURRENT_TIMESTAMP`)
/// estão refletidos no struct retornado. Alternativa seria usar RETURNING,
/// mas o SQLite só suporta a partir de 3.35 e queremos máxima portabilidade.
pub async fn create_user(
    db: &Connection,
    firebase_uid: String,
    email: String,
    display_name: String,
    mode: BlockMode,
) -> Result<User, AppError> {
    let id = Uuid::new_v4().to_string();
    let mode_str = block_mode_to_sql(&mode).to_string();
    // Normaliza o email antes de gravar para garantir consistencia em todo o
    // app — `auth_service::create_user_with_email_verification` ja faz isso;
    // este caminho (Google login direto) tambem precisa.
    let normalized_email = normalize_email(&email)?;
    let display_name = display_name.trim().to_string();

    // Clones porque o closure precisa ser `move` e `'static` — o original
    // pode ser reutilizado depois do `.await` (ex: no get_user_by_firebase_uid).
    let id_clone = id.clone();
    let firebase_uid_clone = firebase_uid.clone();
    let email_clone = normalized_email;
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
    // Se `firebase_uid` já existe, o UNIQUE dá erro aqui e vira 409.
    // (O handler de /auth/register faz a checagem antes, então isso só
    // aciona em corrida de requests concorrentes.)
    .map_err(|e| AppError::Conflict(format!("Falha ao criar usuário: {e}")))?;

    get_user_by_firebase_uid(db, firebase_uid)
        .await?
        .ok_or_else(|| AppError::InternalServerError("User recém-criado não encontrado".into()))
}

/// Usado por /auth/login e pelo middleware para resolver Firebase UID → user_id.
/// Retorna `Ok(None)` quando não existe — o caller decide se isso é 404 ou não.
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
                        mode: parse_block_mode(&row.get::<_, String>(4)?),
                        created_at: row.get(5)?,
                    })
                },
            )
            // `.optional()` converte "NoRows" em `Ok(None)` — queremos
            // distinguir "erro real" de "não achou" no caller.
            .optional()?;
        Ok(r)
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))
}

/// Apaga a conta do usuário e tudo que depende dela. Usado por `DELETE /auth/me`.
///
/// As FKs `ON DELETE CASCADE` cuidam de `devices`, `blocked_items`,
/// `parental_links`, `adult_filter_settings` e `device_tokens`. Mas
/// `email_verifications` referencia o user **por email** (sem FK — a tabela
/// existe antes do user, durante o fluxo de cadastro), então precisa ser
/// apagada manualmente para não vazar dados antigos do email entre contas.
///
/// Tudo em uma transação: ou apaga tudo, ou nada. Se o `email_verifications`
/// falhar, o `users` volta atrás.
pub async fn delete_user(db: &Connection, user_id: String) -> Result<(), AppError> {
    db.call(move |c| {
        let tx = c.transaction()?;

        // Pega o email para limpar `email_verifications` antes de apagar o user.
        let email: Option<String> = tx
            .query_row(
                "SELECT email FROM users WHERE id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .optional()?;

        let Some(email) = email else {
            // User não existe — tratado como sucesso idempotente. O caller
            // (handler) recebeu `auth.user_id` válido, então isso só rola se
            // duas requisições concorrentes chegarem ao mesmo tempo.
            return Ok(());
        };

        tx.execute("DELETE FROM users WHERE id = ?1", params![user_id])?;
        tx.execute(
            "DELETE FROM email_verifications WHERE email = ?1",
            params![email],
        )?;

        tx.commit()?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))
}

/// Usado por /auth/me. Diferente do `by_firebase_uid`, aqui "não achou" é
/// sempre um erro 404 — se o middleware injetou `auth.user_id`, o user
/// deveria existir; se sumiu, algo muito errado aconteceu.
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
                        mode: parse_block_mode(&row.get::<_, String>(4)?),
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
