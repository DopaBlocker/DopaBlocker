-- =============================================================================
-- DopaBlocker — Migration 001: Schema Inicial
-- =============================================================================
-- -----------------------------------------------------------------------------
-- TABELA: users
-- Uma linha por conta de usuário.
-- firebase_uid: vem do Firebase Auth e é o elo entre o backend e o Google.
-- mode: como o usuário opera o app (pessoal ou parental).
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS users (
    id           TEXT PRIMARY KEY,
    firebase_uid TEXT NOT NULL UNIQUE,
    email        TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    mode         TEXT NOT NULL DEFAULT 'personal'
                     CHECK(mode IN ('personal', 'parental')),
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- -----------------------------------------------------------------------------
-- TABELA: devices
-- Uma linha por instalação do app (desktop ou mobile).
-- is_child: INTEGER 0/1 — SQLite não tem tipo BOOLEAN nativo.
-- Rust: bool → rusqlite converte automaticamente entre bool e 0/1.
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS devices (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_name TEXT NOT NULL,
    platform    TEXT NOT NULL
                    CHECK(platform IN ('windows', 'android')),
    is_child    INTEGER NOT NULL DEFAULT 0
                    CHECK(is_child IN (0, 1)),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- -----------------------------------------------------------------------------
-- TABELA: blocked_items
-- Um item por domínio ou app que o usuário quer bloquear.
-- item_type: diferencia o mecanismo de bloqueio.
--   "domain"  → bloqueado via DNS Proxy (desktop) ou VPN (mobile)
--   "app"     → bloqueado via Accessibility Service (Android)
--   "keyword" → reservado para future use
-- is_active: permite pausar o bloqueio sem deletar o item.
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS blocked_items (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_type  TEXT NOT NULL
                   CHECK(item_type IN ('domain', 'app', 'keyword')),
    value      TEXT NOT NULL,
    is_active  INTEGER NOT NULL DEFAULT 1
                   CHECK(is_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),

    -- Impede duplicatas: um usuário não pode bloquear o mesmo valor duas vezes
    UNIQUE(user_id, item_type, value)
);

-- -----------------------------------------------------------------------------
-- TABELA: parental_links
-- Representa o vínculo pai → filho.
-- Ciclo de vida do status:
--   "pending" → pai gerou o código, aguardando o filho digitar
--   "active"  → filho confirmou, vínculo estabelecido
--   "revoked" → pai ou filho desfez o vínculo
-- child_device_id: NULL até o filho confirmar o código.
-- expires_at: o código expira 5 minutos após a criação.
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS parental_links (
    id               TEXT PRIMARY KEY,
    parent_device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    child_device_id  TEXT REFERENCES devices(id) ON DELETE SET NULL,  -- nullable
    link_code        TEXT NOT NULL,
    status           TEXT NOT NULL DEFAULT 'pending'
                         CHECK(status IN ('pending', 'active', 'revoked')),
    expires_at       TEXT NOT NULL,
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- -----------------------------------------------------------------------------
-- TABELA: adult_filter_settings
-- Uma linha por usuário (UNIQUE em user_id).
-- Usa INSERT OR REPLACE para atualizar sem checar se o registro existe.
-- last_list_update: NULL se a lista ainda não foi baixada.
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS adult_filter_settings (
    id               TEXT PRIMARY KEY,
    user_id          TEXT NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    is_enabled       INTEGER NOT NULL DEFAULT 0
                         CHECK(is_enabled IN (0, 1)),
    last_list_update TEXT  -- NULL = lista nunca foi baixada
);

-- -----------------------------------------------------------------------------
-- ÍNDICES — aceleram as queries mais comuns
-- -----------------------------------------------------------------------------

-- Buscar todos os itens de um usuário (query mais frequente da blocklist)
CREATE INDEX IF NOT EXISTS idx_blocked_items_user_id
    ON blocked_items(user_id);

-- Buscar todos os devices de um usuário
CREATE INDEX IF NOT EXISTS idx_devices_user_id
    ON devices(user_id);

-- Buscar um link pelo código digitado pelo filho (precisa ser rápido)
CREATE INDEX IF NOT EXISTS idx_parental_links_code
    ON parental_links(link_code);
