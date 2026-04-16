// =============================================================================
// device_service — CRUD de devices + fluxo completo de vinculação parental.
// =============================================================================
// Este é o service mais complexo do backend. Contém três responsabilidades:
//
//   1. CRUD básico de devices (register, list).
//   2. Geração de código de vinculação (pai).
//   3. Confirmação do código (filho) — que é uma TRANSAÇÃO com 4 passos:
//        a) valida o código (não consumido, não expirado)
//        b) cria o device filho
//        c) marca o `parental_link` como 'active'
//        d) insere o hash do device_token em `device_tokens`
//      Os quatro passos vivem numa mesma `tx` — se um falha, nenhum outro
//      é persistido. É a única parte do backend que realmente precisa de
//      transação; o resto são queries isoladas.
//
// O código de vinculação tem 6 dígitos decimais (100k combinações) e TTL
// de 5 minutos. É curto de propósito: o pai precisa ditar em voz alta.
// A unicidade entre códigos `pending` é garantida por índice UNIQUE
// PARCIAL na migration 002 (não ficam órfãos depois que viram 'active').
//
// O Device Token devolvido pela `confirm_link` é a ÚNICA vez em que o
// token aparece em plain text. Depois disso o banco só tem o SHA-256 e
// não há como recuperar — perdeu, refaz o fluxo.
// =============================================================================

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

/// TTL do código de vinculação em segundos. 5 min é o meio-termo:
/// tempo suficiente para pai e filho coordenarem presencialmente,
/// curto o bastante para limitar brute-force (100k códigos / 5min =
/// alguém teria que chutar 333 códigos/s para cobrir todo o espaço).
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

/// Registra device do PAI. O `is_child` é hard-coded como 0 aqui — um device
/// filho SÓ nasce via `confirm_link`. Isso elimina a classe inteira de bugs
/// "cliente mandou is_child=true e burlou a hierarquia".
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

/// Lista todos os devices do user (pai + filhos). Ordenação ASC por
/// created_at corresponde à ordem cronológica que o usuário adicionou os
/// devices — útil para a tela "Meus aparelhos" mostrar "meu PC" (criado
/// primeiro) no topo.
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

/// Gera código de 6 dígitos para vincular device filho.
///
/// Pré-condição: o pai precisa ter pelo menos um device registrado — usamos
/// o primeiro (mais antigo) como "parent_device_id" para o registro em
/// `parental_links`. Se não tem nenhum, rejeita 400 pedindo registro prévio.
///
/// Colisão de código: o UNIQUE parcial em `parental_links(link_code) WHERE
/// status='pending'` impede que dois códigos pending coincidam. Se cair na
/// colisão (chance ~1 em 100k - N), devolvemos 409 e o frontend pode tentar
/// de novo. Códigos já `active` não contam — podem ser reusados.
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

    // Código com zero-padding para sempre ter 6 caracteres (ex: "000042").
    // `rand::thread_rng()` é um PRNG rápido, não-cripto, suficiente aqui:
    // o adversário tem 5 min e UNIQUE para lutar contra, não os bits de entropia.
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

/// Consome um código e emite o Device Token do filho. Transação obrigatória:
/// os quatro passos têm que ser atômicos, senão podemos deixar o banco em
/// estado quebrado (ex: device filho criado mas sem token correspondente).
///
/// Trick de propagação de erros com mensagens custom:
/// `tokio_rusqlite::Error::Other` aceita qualquer `Box<dyn Error>`. Usamos
/// strings sentinela ("LINK_NOT_FOUND", "LINK_EXPIRED") para que a camada
/// externa possa distinguir do erro genérico de SQL e traduzir para HTTP
/// mais específico (400 vs 500).
///
/// Geração do plain token: concatenamos dois UUIDs v4 "simple" (sem hífens)
/// → 64 hex chars = 256 bits de entropia. Overkill, mas barato. O prefixo
/// "dt_" é adicionado só no response — o hash gravado no banco é calculado
/// sobre o plain SEM prefixo, alinhado com o que o middleware faz no strip_prefix.
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
                // Transação: se qualquer um dos statements falhar, rollback
                // automático ao sair do escopo sem `commit()`.
                let tx = c.transaction()?;

                // (a) Valida o código: precisa existir e estar 'pending'.
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

                // Comparação lexicográfica de strings ISO-8601 UTC funciona
                // como comparação cronológica — propriedade garantida pelo
                // formato fixed-width com zero-padding (ano, mês, dia…).
                if expires_at.as_str() < now_iso.as_str() {
                    return Err(tokio_rusqlite::Error::Other(
                        Box::<dyn std::error::Error + Send + Sync>::from("LINK_EXPIRED"),
                    ));
                }

                // Precisamos do user_id do pai para amarrar o device filho
                // e o device_token. O filho NUNCA tem user_id próprio —
                // compartilha o do pai via `user_id` em `devices`.
                let parent_user_id: String = tx.query_row(
                    "SELECT user_id FROM devices WHERE id = ?1",
                    params![parent_device_id],
                    |r| r.get(0),
                )?;

                // (b) Cria device filho (is_child = 1, forçado pelo servidor).
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

                // (c) Marca o link como 'active' e registra o filho vinculado.
                // Isso também tira o link do índice UNIQUE parcial de pending,
                // liberando o code para reúso se um dia isso for necessário.
                tx.execute(
                    "UPDATE parental_links
                     SET child_device_id = ?1, status = 'active'
                     WHERE id = ?2",
                    params![child_device_id, link_id],
                )?;

                // (d) Persiste o hash do token. NÃO guardamos o plain —
                // se o banco vazar, o atacante não consegue autenticar.
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

    // Tradução dos sentinelas para HTTP errors claros.
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
        // Prefixo "dt_" é o que o middleware usa para rotear: sem ele,
        // o token seria interpretado como Firebase JWT e falharia.
        device_token: format!("dt_{}", plain_token),
        device_id: child_device_id,
        user_id: parent_user_id,
        parent_device_id,
    })
}
