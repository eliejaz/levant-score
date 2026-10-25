ALTER TABLE games ADD COLUMN outcome_data JSONB;

CREATE INDEX idx_games_owner_status_updated
    ON games(owner_id, status, updated_at DESC);
