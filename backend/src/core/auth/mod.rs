// =============================================================================
// core::auth — autenticação dual: Firebase JWT + Device Token.
// =============================================================================
// Cluster que resolve um único problema: transformar o header
// `Authorization: Bearer <algo>` num `AuthUser { user_id, source, device_id }`.
// Dividido por responsabilidade (estilo "por cluster" do engine desktop):
//
//   - jwks         : cache das chaves públicas do Firebase (JWKS).
//   - jwt          : claims + validação de assinatura RS256 do Firebase JWT.
//   - device_token : hash + validação do Device Token (filhos, read-only).
//   - middleware   : `require_auth` (orquestração) + `AuthUser`/`AuthSource`.
//
// A superfície pública é reexportada aqui para o resto do crate importar via
// `crate::core::auth::X` (ergonomia preservada do antigo `crate::middleware::X`).
// =============================================================================

mod device_token;
mod jwks;
mod jwt;
mod middleware;

pub use device_token::hash_device_token;
pub use jwks::JwksCache;
pub use jwt::{extract_bearer_token, verify_firebase_jwt_token, FirebaseClaims};
// `FirebaseAuthInfo` só é referenciado pelos testes de `features::auth::routes`.
#[cfg(test)]
pub use jwt::FirebaseAuthInfo;
pub use middleware::{require_auth, AuthSource, AuthUser};
