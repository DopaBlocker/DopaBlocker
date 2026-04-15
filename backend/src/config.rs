use dotenvy::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub database_url: String, // Futuro
    pub firebase_project_id: String, // Futuro
}

impl AppConfig {
    pub fn init() -> Self {
        // Carrega variáveis de ambiente do .env, ignorando se falhar em prod
        let _ = dotenv();

        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".into())
            .parse::<u16>()
            .expect("A variável PORT deve ser um número válido");

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://dopablocker.db".into());

        let firebase_project_id = env::var("FIREBASE_PROJECT_ID")
            .unwrap_or_else(|_| "dopablocker-mock".into());

        Self {
            port,
            database_url,
            firebase_project_id,
        }
    }
}
