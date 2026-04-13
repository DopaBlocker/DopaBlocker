-- =============================================================================
-- DopaBlocker — Migration 002: Ajustes de Controle Parental
-- =============================================================================
-- Esta migration é aditiva: não altera tabelas existentes, apenas adiciona
-- índices faltantes e uma nova tabela para suportar o fluxo "Filhos sem conta".
-- =============================================================================

-- -----------------------------------------------------------------------------
-- ÍNDICE: idx_parental_links_parent
-- Acelera queries do tipo "listar todos os filhos vinculados a este pai",
-- que até aqui faziam full table scan.
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_parental_links_parent
    ON parental_links(parent_device_id);

-- -----------------------------------------------------------------------------
-- ÍNDICE UNIQUE PARCIAL: idx_parental_links_code_active
-- Impede que dois pais tenham simultaneamente o mesmo link_code com status
-- 'pending'. Sem este índice, uma colisão aleatória poderia fazer um filho
-- vincular ao pai errado (10^6 combinações não é tanto).
-- SQLite suporta partial indexes desde a versão 3.8.
-- -----------------------------------------------------------------------------
CREATE UNIQUE INDEX IF NOT EXISTS idx_parental_links_code_active
    ON parental_links(link_code)
    WHERE status = 'pending';

-- -----------------------------------------------------------------------------
-- TABELA: device_tokens
-- Tokens de acesso dos devices filhos, que não têm conta Firebase.
-- Gerado no momento da confirmação do código de vinculação:
--   POST /devices/link/confirm → retorna um token novo → salvo aqui (hashed).
--
-- IMPORTANTE: `token_hash` armazena o SHA-256 do token, NUNCA o plain text.
-- Se o banco vazar, os tokens hasheados são inúteis para um atacante.
--
-- O middleware recebe `Authorization: Bearer dt_<plain_token>`, remove o
-- prefixo `dt_`, calcula o SHA-256, e busca por `token_hash`.
--
-- `revoked_at` permite "desvincular" um filho sem deletar o histórico:
--   - NULL  = token ativo
--   - !NULL = revogado (data/hora em ISO 8601)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS device_tokens (
    token_hash TEXT PRIMARY KEY,
    device_id  TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    revoked_at TEXT  -- NULL = ativo
);

-- -----------------------------------------------------------------------------
-- ÍNDICE: idx_device_tokens_device
-- Acelera a query "listar/revogar tokens deste device".
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_device_tokens_device
    ON device_tokens(device_id);
