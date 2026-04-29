-- Initial schema for ArchettoBot

CREATE TABLE IF NOT EXISTS admins (
    user_id INTEGER PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS bot_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS func_scopes (
    target_type TEXT NOT NULL,
    target_id   INTEGER NOT NULL,
    bili_parse  INTEGER NOT NULL DEFAULT 0,
    competition INTEGER NOT NULL DEFAULT 0,
    welcome     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (target_type, target_id)
);

CREATE TABLE IF NOT EXISTS group_welcome (
    group_id INTEGER PRIMARY KEY,
    message  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS competitions (
    link       TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    start_time INTEGER NOT NULL,
    duration   INTEGER NOT NULL,
    platform   TEXT NOT NULL,
    notified   INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_competition_notify ON competitions(start_time, notified);
