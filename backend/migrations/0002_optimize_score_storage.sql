-- The optimized Rust schema removes the unused score UUID and makes the
-- natural (round, player) key the primary key. Existing games are intentionally
-- discarded because this deployment is allowed to reset pre-optimization data.
TRUNCATE TABLE games CASCADE;

DROP TABLE score_entries;

CREATE TABLE score_entries (
    round_id UUID NOT NULL REFERENCES rounds(id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    points INTEGER NOT NULL,
    PRIMARY KEY(round_id, player_id)
);

CREATE INDEX idx_scores_player ON score_entries(player_id);
