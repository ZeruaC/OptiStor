-- A Market is shared reference data: one per country/jurisdiction's tariff
-- structure, reusable across many organizations and projects rather than
-- duplicating regulatory/pricing setup per project. `tariff_model_key` is
-- the key the engine's tariffs registry looks up (see
-- engine/src/optistor_engine/tariffs/registry.py).
CREATE TABLE markets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    country_code TEXT NOT NULL,
    tariff_model_key TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Nullable: a project isn't required to have a market assigned yet (the
-- provisional flat tariff from Phase 4 still applies until it does).
ALTER TABLE projects ADD COLUMN market_id TEXT REFERENCES markets(id);
