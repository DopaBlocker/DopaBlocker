// Acesso ao SQLCipher local via rusqlite + tokio-rusqlite.
// Implementar: init_db(app_handle) que:
//   1. Obtém app_data_dir via app_handle.path().app_data_dir()
//   2. Abre conexão com tokio_rusqlite::Connection::open(path)
//   3. Executa PRAGMA key = '<chave>' como PRIMEIRO comando (obrigatório)
//   4. Roda CREATE TABLE IF NOT EXISTS para as tabelas locais
//   5. Retorna a Connection
// Funções CRUD para blocked_items, devices, user_settings.
// O SQLCipher serve como cache offline criptografado — dados são sincronizados com Firestore via API.
// Sem a chave correta, o arquivo .db é ilegível (proteção contra acesso físico ao disco).
