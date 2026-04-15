use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};

#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub id: String,
    pub firebase_uid: String,
}

/// Middleware MOCK para validação de Autenticação.
/// No futuro aqui validaremos o Bearer token usando a Lib do Firebase.
pub async fn require_auth(
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req.headers().get(header::AUTHORIZATION);

    if let Some(header_value) = auth_header {
        if let Ok(auth_str) = header_value.to_str() {
            if auth_str.starts_with("Bearer ") {
                let _token = &auth_str[7..];
                // TODO: Firebase verify_id_token(token)

                // Mock payload user
                let user = CurrentUser {
                    id: "mock-user-id-123".to_string(),
                    firebase_uid: "mock-firebase-uid".to_string(),
                };

                // Injeta o usuário validado na request usando Extensions
                req.extensions_mut().insert(user);
                return Ok(next.run(req).await);
            }
        }
    }

    // Se chegar aqui, requisição falhou, não tinha token ou era inválido.
    Err(StatusCode::UNAUTHORIZED)
}
