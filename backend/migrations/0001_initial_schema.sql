-- First Rust deployment intentionally replaces the legacy Spring/Flyway data.
DROP TABLE IF EXISTS score_entries CASCADE;
DROP TABLE IF EXISTS rounds CASCADE;
DROP TABLE IF EXISTS players CASCADE;
DROP TABLE IF EXISTS games CASCADE;
DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS flyway_schema_history CASCADE;

CREATE TABLE users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    display_name VARCHAR(80) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE games (
    id UUID PRIMARY KEY,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(120) NOT NULL,
    preset_code VARCHAR(40) NOT NULL,
    preset_name VARCHAR(80) NOT NULL,
    team_mode BOOLEAN NOT NULL DEFAULT FALSE,
    score_direction VARCHAR(16) NOT NULL,
    target_score INTEGER,
    threshold_rule VARCHAR(32) NOT NULL DEFAULT 'NONE',
    score_mode VARCHAR(24) NOT NULL DEFAULT 'INDIVIDUAL',
    round_limit INTEGER,
    status VARCHAR(16) NOT NULL DEFAULT 'ACTIVE',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE players (
    id UUID PRIMARY KEY,
    game_id UUID NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    name VARCHAR(60) NOT NULL,
    team_number INTEGER,
    seat_order INTEGER NOT NULL
);
CREATE TABLE rounds (
    id UUID PRIMARY KEY,
    game_id UUID NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    round_number INTEGER NOT NULL,
    rule_code VARCHAR(40),
    note VARCHAR(240),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(game_id, round_number)
);
CREATE TABLE score_entries (
    id UUID PRIMARY KEY,
    round_id UUID NOT NULL REFERENCES rounds(id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    points INTEGER NOT NULL,
    UNIQUE(round_id, player_id)
);
CREATE INDEX idx_games_owner_updated ON games(owner_id, updated_at DESC);
CREATE INDEX idx_players_game_seat ON players(game_id, seat_order);
CREATE INDEX idx_rounds_game_number ON rounds(game_id, round_number DESC);
CREATE INDEX idx_scores_round ON score_entries(round_id);
