// =============================================================================
// Rotas de autenticação: /auth/register, /auth/login, /auth/me
// =============================================================================
// O fluxo do usuário é:
//
//   1. Frontend faz login no Firebase SDK (email/senha ou Google).
//      → Firebase retorna um JWT.
//   2. Primeira vez na vida: frontend chama POST /auth/register enviando
//      email + display_name + mode no body e o JWT no header. O backend
//      valida o JWT, extrai o `firebase_uid` das claims, e cria o user
//      local em `users`.
//   3. Das próximas vezes: frontend chama POST /auth/login com o JWT no
//      header (body vazio). O backend valida, procura o user local e
//      devolve.
//   4. A qualquer momento: GET /auth/me retorna os dados do user
//      autenticado (usa o AuthUser injetado pelo middleware).
//
// Por que `register` e `login` são "públicas" (não passam pelo middleware
// global)? Porque o middleware global também faz lookup do user em
// `users` — se o user não existe ainda, retorna 401. Mas o register é
// justamente a rota que CRIA o user, e o login precisa funcionar até
// para usuários que ainda não completaram o fluxo de register. Então
// ambas validam o JWT inline, sem o lookup forçado.
// =============================================================================

use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Extension, Json, Router,
};

use crate::errors::AppError;
use crate::middleware::{extract_bearer_token, verify_firebase_jwt_token, AuthUser};
use crate::models::{RegisterRequest, User};
use crate::services::user_service;
use crate::AppState;

/// Rotas que NÃO passam pelo middleware global `require_auth`.
/// Ambas validam o Firebase JWT manualmente dentro do próprio handler.
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
}

/// Rotas protegidas: o middleware global já validou o token e injetou
/// `AuthUser`, então o handler só faz trabalho de domínio.
pub fn protected_router() -> Router<AppState> {
    Router::new().route("/auth/me", get(me))
}

/// Cria o user LOCAL correspondente a um user Firebase recém-criado.
///
/// Pré-condições:
///   - Frontend já criou o user no Firebase Auth.
///   - Header `Authorization: Bearer <jwt>` acompanha a requisição.
///   - Body tem email, display_name, mode.
///
/// Comportamento:
///   - Se o firebase_uid das claims já existe em `users`, retorna 409.
///   - Senão, insere e retorna o User criado.
async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<User>, AppError> {
    // Validação do JWT inline — precisamos do `sub` (firebase_uid) para
    // amarrar o user local. O body NÃO carrega firebase_uid: se carregasse,
    // qualquer cliente poderia registrar um user com identidade forjada.
    let token = extract_bearer_token(&headers)?;
    let claims = verify_firebase_jwt_token(&state, &token).await?;

    // Idempotência mal-cabida: se alguém chamar register duas vezes com o
    // mesmo Firebase user, preferimos avisar (409) a silenciosamente retornar
    // o user existente — o frontend deve usar login nesse caso.
    if let Some(existing) =
        user_service::get_user_by_firebase_uid(&state.db, claims.sub.clone()).await?
    {
        return Err(AppError::Conflict(format!(
            "Usuário já registrado: {}",
            existing.email
        )));
    }

    let user = user_service::create_user(
        &state.db,
        claims.sub,
        payload.email,
        payload.display_name,
        payload.mode,
    )
    .await?;
    Ok(Json(user))
}

/// "Sincroniza" o user local com o Firebase. Body vazio, JWT no header.
/// Retorna o User já existente, ou 404 se o frontend ainda não chamou
/// /auth/register para este Firebase UID.
///
/// Optamos por NÃO fazer get-or-create aqui: forçar o register explícito
/// evita que um JWT válido de um user que não completou onboarding crie
/// silenciosamente um registro incompleto (sem `mode` escolhido, etc.).
async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<User>, AppError> {
    let token = extract_bearer_token(&headers)?;
    let claims = verify_firebase_jwt_token(&state, &token).await?;

    let user = user_service::get_user_by_firebase_uid(&state.db, claims.sub.clone())
        .await?
        .ok_or_else(|| {
            AppError::NotFound("Usuário não registrado localmente — chame /auth/register".into())
        })?;
    Ok(Json(user))
}

/// Retorna os dados do user autenticado. Como passa pelo middleware
/// global, o `AuthUser` já vem preenchido — aqui é só fetch pelo id.
async fn me(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<User>, AppError> {
    let user = user_service::get_user_by_id(&state.db, auth.user_id).await?;
    Ok(Json(user))
}
