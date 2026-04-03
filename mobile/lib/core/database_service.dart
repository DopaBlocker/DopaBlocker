// Serviço de banco de dados local usando SQLCipher (sqflite_sqlcipher).
// Implementar: classe DatabaseService (singleton) com:
//   - initDatabase() → abre banco com openDatabase(path, password: sqlcipherKey)
//     A senha DEVE ser passada no parâmetro password ao abrir — sem ela o banco fica ilegível.
//   - Criar tabelas locais: blocked_items, devices, adult_filter_settings (cache offline)
//   - Funções CRUD: getBlocklist, addBlockedItem, removeBlockedItem, getDevices, etc.
//   - close() para fechar conexão ao sair do app
// O banco local é um espelho do backend para funcionar offline.
// O sqflite_sqlcipher é drop-in replacement do sqflite — mesma API, com criptografia AES-256.
