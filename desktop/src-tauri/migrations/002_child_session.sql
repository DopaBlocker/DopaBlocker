-- =============================================================================
-- Migration 002: sessao do filho (sem conta Firebase).
-- =============================================================================
-- O fluxo "Filhos" nao cria conta — o device do filho recebe um Device Token
-- (`dt_<plain>`) ao confirmar o codigo de vinculacao, e usa esse token em
-- todas as requisicoes ao backend.
--
-- O token precisa persistir entre boots (senao o filho teria que digitar o
-- codigo toda vez). Como o SQLCipher ja criptografa o arquivo .db inteiro
-- com a chave do Windows Credential Manager, e seguro guardar o token em
-- texto puro aqui — quem nao tem a chave nao consegue ler.
--
-- Singleton: so existe UMA child_session por instalacao do app (id = 1
-- forcado por CHECK). Trocar de filho exige clear_child_session +
-- save_child_session.
--
-- O mobile (sqflite_sqlcipher) usara EXATAMENTE este mesmo schema — copia e
-- cola. Mantendo identico, fica trivial portar a logica de auth para Dart.
-- =============================================================================

CREATE TABLE IF NOT EXISTS child_session (
    id               INTEGER PRIMARY KEY CHECK (id = 1),  -- singleton
    user_id          TEXT NOT NULL,                       -- user_id do PAI
    device_id        TEXT NOT NULL,                       -- id deste device filho
    device_token     TEXT NOT NULL,                       -- "dt_<plain>"
    parent_device_id TEXT NOT NULL,                       -- device do pai que gerou o codigo
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);
