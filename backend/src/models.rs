// =============================================================================
// Modelos do backend — re-exports do crate compartilhado + DTOs da API REST.
// =============================================================================
// Esta arquitetura tem duas camadas de tipos:
//
//   1. MODELS (re-exportados de `dopablocker_shared::models`):
//      `User`, `Device`, `BlockedItem`, `ParentalLink`, ...
//      Representam as entidades do domínio. São os MESMOS tipos usados
//      pelo desktop (Tauri) e pelo mobile (Flutter via FFI). Um campo
//      novo em `User` aparece simultaneamente nos três.
//
//   2. DTOs (Data Transfer Objects) definidos aqui:
//      `RegisterRequest`, `CreateBlockedItemRequest`, ...
//      Representam EXCLUSIVAMENTE o payload da API REST — ou seja, o
//      JSON que entra ou sai pela rede. Diferem dos models porque:
//        - Omitem campos gerados pelo servidor (id, created_at).
//        - Omitem campos que vêm de outro canal (firebase_uid vem do JWT,
//          não do body da requisição).
//        - Incluem só os campos que o frontend sabe/deve enviar.
//
// Separar as duas camadas é o que impede bugs como "o frontend mandou um
// user_id falsificado e o backend confiou". O DTO nunca carrega user_id;
// esse campo vem do token validado.
// =============================================================================

use serde::{Deserialize, Serialize};

// Re-exports dos models compartilhados. O backend usa `crate::models::User`
// em vez de `dopablocker_shared::models::User`, ficando mais curto e
// centralizando os imports em um só ponto (se algum dia trocarmos a
// origem, é só aqui).
pub use dopablocker_shared::models::{
    AdultFilterSettings, BlockMode, BlockedItem, BlockedType, Device, Platform, User,
};

// =====================================================================
// DTOs / Payloads da API
// =====================================================================

// ---- Auth ----

/// Body de `POST /auth/register`.
/// Note que NÃO tem `firebase_uid` — o backend extrai das claims do JWT
/// que acompanha a requisição. Também não tem `password`: a senha é
/// registrada diretamente no Firebase pelo frontend, nunca transita aqui.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub display_name: String,
    pub mode: BlockMode, // Personal ou Parental — escolhido na tela inicial.
}

// ---- Blocklist ----

/// Body de `POST /blocklist`. O backend atribui o `user_id` (do AuthUser),
/// o `id` (UUID), o `created_at` (now) e o `is_active=true` automaticamente.
#[derive(Debug, Deserialize)]
pub struct CreateBlockedItemRequest {
    pub item_type: BlockedType, // "domain" | "app" | "keyword"
    pub value: String,          // ex: "instagram.com" ou "com.instagram.android"
}

/// Body de `PUT /blocklist/adult-filter`.
#[derive(Debug, Deserialize)]
pub struct AdultFilterToggleRequest {
    pub enabled: bool,
}

// ---- Devices ----

/// Body de `POST /devices/register`. O `is_child` NÃO é aceito do cliente:
/// um device registrado por essa rota é sempre do PAI (is_child=false).
/// Devices filhos nascem apenas via `POST /devices/link/confirm`, onde
/// o is_child=true é forçado pelo servidor.
#[derive(Debug, Deserialize)]
pub struct RegisterDeviceRequest {
    pub device_name: String,
    pub platform: Platform, // "windows" ou "android"
}

/// Resposta de `POST /devices/link/generate`. O `code` tem exatamente
/// 6 dígitos decimais (com zero-padding). `expires_at` está em ISO-8601
/// UTC e corresponde a now + 5 minutos.
#[derive(Debug, Serialize)]
pub struct GenerateLinkCodeResponse {
    pub code: String,
    pub expires_at: String,
}

/// Body de `POST /devices/link/confirm`. Esta é uma rota PÚBLICA — o
/// filho ainda não tem credencial quando a chama. O trio (code,
/// device_name, platform) é tudo que o backend precisa para criar o
/// device do filho e emitir o primeiro token.
#[derive(Debug, Deserialize)]
pub struct ConfirmLinkRequest {
    pub code: String,
    pub device_name: String,
    pub platform: Platform,
}

/// Resposta de `POST /devices/link/confirm`. O `device_token` vem em
/// PLAIN TEXT (única vez na vida — o banco só guarda o SHA-256). O app
/// do filho DEVE salvá-lo em secure storage; se perder, precisa refazer
/// o fluxo de vinculação com novo código.
#[derive(Debug, Serialize)]
pub struct ConfirmLinkResponse {
    /// Formato: "dt_<plain_token>". O prefixo "dt_" é o que o middleware
    /// usa para decidir entre JWT e Device Token.
    pub device_token: String,
    pub device_id: String,         // id do device filho recém-criado
    pub user_id: String,           // id do user do PAI (filhos não têm user próprio)
    pub parent_device_id: String,  // id do device que gerou o código
}

// ---- Respostas utilitárias ----

/// Corpo genérico de sucesso quando não há dado útil pra retornar
/// (ex: DELETE). O HTTP 200 + este JSON é mais amigável para clientes
/// que esperam sempre JSON do que um 204 No Content.
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub message: String,
}
