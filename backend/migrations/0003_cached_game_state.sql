-- Derived state makes dashboard and round mutations constant-time. Existing
-- games are intentionally cleared so every row starts with consistent totals.
TRUNCATE TABLE games CASCADE;

ALTER TABLE games
    ADD COLUMN round_count INTEGER NOT NULL DEFAULT 0 CHECK (round_count >= 0);

ALTER TABLE players
    ADD COLUMN total_score INTEGER NOT NULL DEFAULT 0;

DROP INDEX IF EXISTS idx_scores_player;
