-- Cache local do desktop. Reflete os dados que o backend possui, mas existe
-- para que o engine de bloqueio (DNS proxy + WFP) consulte as regras sem
-- depender de rede. O servidor é a fonte-da-verdade; sincronização é feita
-- em intervalos via sync_with_backend.

CREATE TABLE IF NOT EXISTS blocked_items_cache (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL,
    item_type  TEXT NOT NULL CHECK(item_type IN ('domain','app','keyword')),
    value      TEXT NOT NULL,
    is_active  INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    synced_at  TEXT
);

CREATE INDEX IF NOT EXISTS idx_blocked_items_cache_user
    ON blocked_items_cache(user_id);

CREATE INDEX IF NOT EXISTS idx_blocked_items_cache_value
    ON blocked_items_cache(value);

-- Key-value simples para estado do engine entre boots:
--   "blocking_enabled"     -> "true" | "false"
--   "adult_filter_enabled" -> "true" | "false"
--   "previous_dns_v4"      -> "1.1.1.1,8.8.8.8" (restaurado ao desligar)
--   "adult_list_updated_at"-> ISO 8601
--   "last_user_id"         -> último usuário logado (para recarregar cache)
CREATE TABLE IF NOT EXISTS blocking_state (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
