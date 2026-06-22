-- =============================================================================
-- DopaBlocker — Migration 004: Eventos de adulteração (tamper) dos devices
-- =============================================================================
-- Aditiva: nova tabela para o controle parental "observável" no mobile (C2.1/
-- C2.2). Sem root, o filho consegue desligar a VPN ou abrir as Configs de
-- VPN/DNS; não dá para IMPEDIR, mas dá para TORNAR VISÍVEL ao responsável.
--
-- O device do filho reporta o evento via `POST /devices/tamper` (rota pública
-- auto-autenticada pelo Device Token no corpo — preserva o invariante
-- "Device Token só faz GET/HEAD" do middleware). O pai lê via `GET /devices/events`.
--
-- `kind` enumera os eventos suportados (espelha o CHECK e os `kind` enviados
-- pelo app nativo Android). `acknowledged_at` fica reservado para o pai marcar
-- um alerta como visto (NULL = não lido).
-- =============================================================================
CREATE TABLE IF NOT EXISTS device_events (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id       TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL
                        CHECK(kind IN (
                            'vpn_revoked',
                            'vpn_settings_opened',
                            'dns_settings_opened'
                        )),
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    acknowledged_at TEXT  -- NULL = alerta ainda não visto pelo pai
);

-- Consulta principal: "listar alertas deste pai, mais recentes primeiro".
CREATE INDEX IF NOT EXISTS idx_device_events_user
    ON device_events(user_id, created_at);
