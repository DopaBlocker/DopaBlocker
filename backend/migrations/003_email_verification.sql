-- =============================================================================
-- DopaBlocker - Migration 003: Verificacao de email por codigo
-- =============================================================================

CREATE TABLE IF NOT EXISTS email_verifications (
    id               TEXT PRIMARY KEY,
    email            TEXT NOT NULL,
    code_hash        TEXT NOT NULL,
    token_hash       TEXT,
    status           TEXT NOT NULL DEFAULT 'pending'
                         CHECK(status IN ('pending', 'verified', 'consumed', 'expired')),
    attempts         INTEGER NOT NULL DEFAULT 0,
    expires_at       TEXT NOT NULL,
    token_expires_at TEXT,
    last_sent_at     TEXT NOT NULL,
    verified_at      TEXT,
    consumed_at      TEXT,
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_email_verifications_email_status
    ON email_verifications(email, status, created_at);

CREATE INDEX IF NOT EXISTS idx_email_verifications_token_hash
    ON email_verifications(token_hash)
    WHERE token_hash IS NOT NULL;
