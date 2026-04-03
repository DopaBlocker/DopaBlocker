// Struct de configuração carregada de variáveis de ambiente.
// Implementar: AppConfig com campos port, database_url, firebase_project_id,
// firebase_api_key, sqlcipher_key. Usar dotenvy para carregar .env e std::env para ler variáveis.
// A sqlcipher_key é usada como PRAGMA key ao abrir a conexão com o banco — sem ela, o banco fica ilegível.
