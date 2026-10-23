use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use super::domain::{ScoreDirection, ScoreMode, Status, ThresholdRule};

#[derive(Clone, Debug, FromRow)]
pub struct DbGame {
    pub id: Uuid,
    pub title: String,
    pub preset_code: String,
    pub preset_name: String,
    pub team_mode: bool,
    pub score_direction: String,
    pub target_score: Option<i32>,
    pub threshold_rule: String,
    pub score_mode: String,
    pub round_limit: Option<i32>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, FromRow)]
pub struct DbPlayer {
    pub id: Uuid,
    pub game_id: Uuid,
    pub name: String,
    pub team_number: Option<i32>,
    pub seat_order: i32,
}

#[derive(Clone, Debug, FromRow)]
pub struct DbRound {
    pub id: Uuid,
    pub round_number: i32,
    pub rule_code: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, FromRow)]
pub struct DbScore {
    pub round_id: Uuid,
    pub player_id: Uuid,
    pub points: i32,
}

#[derive(Clone, Debug)]
pub struct GameData {
    pub game: DbGame,
    pub players: Vec<DbPlayer>,
    pub rounds: Vec<DbRound>,
    pub scores: Vec<DbScore>,
}

impl DbGame {
    pub fn direction(&self) -> ScoreDirection {
        ScoreDirection::parse(&self.score_direction)
    }

    pub fn threshold(&self) -> ThresholdRule {
        ThresholdRule::parse(&self.threshold_rule)
    }

    pub fn mode(&self) -> ScoreMode {
        ScoreMode::parse(&self.score_mode)
    }

    pub fn game_status(&self) -> Status {
        if self.status == "FINISHED" {
            Status::Finished
        } else {
            Status::Active
        }
    }
}
