// =============================================================================
// Rotas da blocklist: /blocklist (GET/POST), /blocklist/:id (DELETE),
// /blocklist/adult-filter (PUT)
// =============================================================================
// Todas as rotas aqui são "protegidas" — passam pelo middleware global
// `require_auth` que injeta o `AuthUser`. Isso significa que:
//
//   - `GET /blocklist` aceita tanto Firebase JWT quanto Device Token (o app
//     do filho precisa ler a lista para saber o que bloquear).
//   - `POST`, `DELETE` e `PUT` são automaticamente rejeitados com 403 se
//     vierem via Device Token (enforcement em middleware.rs). Ou seja, um
//     filho pode LER a blocklist, mas nunca modificar.
//
// O `auth.user_id` injetado é sempre o id do user LOCAL (do pai, no caso
// de Device Token), então as queries ficam simples: "filtre por user_id".
// =============================================================================

use axum::{
    extract::{Path, State},
    routing::{delete, get, put},
    Extension, Json, Router,
};

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    AdultFilterSettings, AdultFilterToggleRequest, BlockedItem, CreateBlockedItemRequest,
    SuccessResponse,
};
use crate::services::blocklist_service;
use crate::AppState;

/// Router exposto em `main.rs` via `.nest("/blocklist", ...)`. Portanto os
/// paths aqui são relativos a `/blocklist` (ex: `"/"` vira `/blocklist`).
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_items).post(add_item))
        .route("/:id", delete(delete_item))
        .route("/adult-filter", put(set_adult_filter))
}

/// `GET /blocklist` — retorna todos os itens bloqueados do user autenticado,
/// mais recentes primeiro. Funciona tanto via JWT quanto via Device Token.
async fn list_items(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<BlockedItem>>, AppError> {
    let items = blocklist_service::list_items(&state.db, auth.user_id).await?;
    Ok(Json(items))
}

/// `POST /blocklist` — adiciona domínio/app/keyword à blocklist. Como é
/// método de escrita, o middleware já filtrou Device Tokens antes de
/// chegar aqui; o handler só vê usuários Firebase.
async fn add_item(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<CreateBlockedItemRequest>,
) -> Result<Json<BlockedItem>, AppError> {
    let item = blocklist_service::add_item(&state.db, auth.user_id, payload).await?;
    Ok(Json(item))
}

/// `DELETE /blocklist/:id` — remove um item. A query no service usa
/// `WHERE id = ?1 AND user_id = ?2` para impedir que um user delete
/// item de outro, mesmo que adivinhe o UUID.
async fn delete_item(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<SuccessResponse>, AppError> {
    blocklist_service::delete_item(&state.db, auth.user_id, id).await?;
    Ok(Json(SuccessResponse {
        message: "Item removido".into(),
    }))
}

/// `PUT /blocklist/adult-filter` — liga/desliga o filtro de conteúdo adulto
/// do user. Upsert: cria se não existir, atualiza se existir (UNIQUE em user_id).
async fn set_adult_filter(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<AdultFilterToggleRequest>,
) -> Result<Json<AdultFilterSettings>, AppError> {
    let settings =
        blocklist_service::set_adult_filter(&state.db, auth.user_id, payload.enabled).await?;
    Ok(Json(settings))
}
