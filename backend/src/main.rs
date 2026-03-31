// Entry point do servidor Axum.
// Implementar: carregar config de .env, inicializar tracing/logging,
// montar o router com todas as rotas (auth, blocklist, devices),
// aplicar middleware CORS e de autenticação, bind na porta configurada.

mod config;
mod errors;
mod middleware;
mod models;
mod routes;
mod services;

fn main() {
    // TODO: substituir por #[tokio::main] async fn main()
    println!("DopaBlocker Backend");
}
