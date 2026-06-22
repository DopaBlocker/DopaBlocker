// =============================================================================
// Rotas de devices — /devices/register, /devices, /devices/link/*
// =============================================================================
// Esta é a parte mais delicada do backend, porque mistura rotas protegidas
// com uma rota PÚBLICA: `/devices/link/confirm`.
//
// Por que `confirm` é pública? Porque quem chama é o app do FILHO, que ainda
// não tem credencial nenhuma. O fluxo é:
//
//   1. Pai (logado via Firebase) chama POST /devices/link/generate e recebe
//      um código de 6 dígitos válido por 5 minutos.
//   2. Pai lê o código em voz alta (ou mostra na tela) para o filho.
//   3. Filho digita o código no app do celular/desktop dele e chama
//      POST /devices/link/confirm com (code, device_name, platform).
//   4. Backend valida o código, cria um device filho, gera um Device Token
//      e devolve em PLAIN TEXT no response.
//   5. Filho salva o token em secure storage. Dali em diante, suas
//      requisições carregam `Authorization: Bearer dt_<token>`.
//
// Sem essa rota pública não haveria como o filho autenticar a primeira vez.
// Por isso ela está em `public_router()` separado, que `main.rs` monta fora
// do grupo coberto por `require_auth`.
// =============================================================================

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Extension, Json, Router,
};

use crate::errors::AppError;
use crate::middleware::{AuthSource, AuthUser};
use crate::models::{
    ConfirmLinkRequest, ConfirmLinkResponse, Device, DeviceEvent, GenerateLinkCodeResponse,
    RegisterDeviceRequest, SuccessResponse, TamperReportRequest,
};
use crate::services::{device_event_service, device_service};
use crate::AppState;

/// Rotas que exigem JWT Firebase OU Device Token (GET apenas).
/// Montadas em `main.rs` via `.nest("/devices", ...)`.
pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register_device))
        .route("/", get(list_devices))
        .route("/events", get(list_events))
        .route("/link/generate", post(generate_link_code))
        .route("/{id}/revoke", post(revoke_device))
}

/// Rota pública — apenas `/devices/link/confirm`. Note que aqui o path é
/// ABSOLUTO (não faz nest), porque `main.rs` faz `.merge()` deste router
/// no Router raiz, e precisamos do path completo.
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/devices/link/confirm", post(confirm_link))
        .route("/devices/tamper", post(report_tamper))
}

/// `POST /devices/register` — registra um device DO PAI (is_child=false,
/// forçado no service). Um pai pode ter múltiplos devices (notebook de casa,
/// celular, PC do trabalho). Device do filho NUNCA é criado por esta rota —
/// só via `confirm_link`.
async fn register_device(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<RegisterDeviceRequest>,
) -> Result<Json<Device>, AppError> {
    let device = device_service::register_device(&state.db, auth.user_id, payload).await?;
    Ok(Json(device))
}

/// `GET /devices` — lista devices do user. Como `user_id` no banco é sempre
/// o do pai (mesmo para device filho), isso retorna TODOS os devices da
/// "família" — úteis para a tela "meus aparelhos vinculados".
async fn list_devices(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<Device>>, AppError> {
    let devices = device_service::list_devices(&state.db, auth.user_id).await?;
    Ok(Json(devices))
}

/// `POST /devices/link/generate` — gera um código de 6 dígitos para vincular
/// device filho. Só aceita Firebase (pai real). Se um device filho (Device
/// Token) tentar chamar, o middleware já barra com 403 — aqui é redundância
/// defensiva em caso de refactor futuro que afrouxe a regra geral.
async fn generate_link_code(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<GenerateLinkCodeResponse>, AppError> {
    if auth.source != AuthSource::Firebase {
        return Err(AppError::Forbidden(
            "Apenas contas Firebase podem gerar códigos de vinculação".into(),
        ));
    }
    let resp = device_service::generate_link_code(&state.db, auth.user_id).await?;
    Ok(Json(resp))
}

/// `POST /devices/link/confirm` — ROTA PÚBLICA. Input: (code, device_name,
/// platform). Output: um device filho criado + um Device Token em plain text.
/// O token só aparece AQUI, uma vez na vida: depois, o banco guarda só o
/// hash SHA-256. Se o app do filho perder o token, precisa recomeçar o
/// fluxo (pai gera novo código).
async fn confirm_link(
    State(state): State<AppState>,
    Json(payload): Json<ConfirmLinkRequest>,
) -> Result<Json<ConfirmLinkResponse>, AppError> {
    let resp = device_service::confirm_link(&state.db, payload).await?;
    Ok(Json(resp))
}

/// `POST /devices/:id/revoke` — pai desvincula um filho. Apenas Firebase JWT
/// (mesma justificativa de `/link/generate`). O service confere que o device
/// pertence ao `user_id` autenticado E que é `is_child=true`. Após revogar,
/// qualquer request com o token antigo cai em 401.
async fn revoke_device(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(device_id): Path<String>,
) -> Result<Json<SuccessResponse>, AppError> {
    if auth.source != AuthSource::Firebase {
        return Err(AppError::Forbidden(
            "Apenas contas Firebase podem revogar dispositivos vinculados".into(),
        ));
    }
    device_service::revoke_child_device(&state.db, auth.user_id, device_id).await?;
    Ok(Json(SuccessResponse {
        message: "Dispositivo desvinculado".into(),
    }))
}

/// `POST /devices/tamper` — ROTA PÚBLICA. O device do filho reporta um evento
/// de adulteração (VPN desligada, Configs de VPN/DNS abertas). Autentica-se
/// pelo Device Token NO CORPO (ver `device_event_service::record_tamper`), e
/// não pelo header — assim a regra read-only do middleware fica intocada.
/// Token inválido/revogado → 401; `kind` desconhecido → 400.
async fn report_tamper(
    State(state): State<AppState>,
    Json(payload): Json<TamperReportRequest>,
) -> Result<Json<SuccessResponse>, AppError> {
    device_event_service::record_tamper(&state.db, &payload.device_token, &payload.kind).await?;
    Ok(Json(SuccessResponse {
        message: "Evento registrado".into(),
    }))
}

/// `GET /devices/events` — o pai lê os alertas de adulteração dos seus filhos
/// (mais recentes primeiro). Apenas Firebase JWT: um Device Token (filho) não
/// deve enxergar os próprios alertas, então rejeitamos com 403 mesmo sendo GET.
async fn list_events(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<DeviceEvent>>, AppError> {
    if auth.source != AuthSource::Firebase {
        return Err(AppError::Forbidden(
            "Apenas contas Firebase podem ver alertas de dispositivos".into(),
        ));
    }
    let events = device_event_service::list_events(&state.db, auth.user_id).await?;
    Ok(Json(events))
}
