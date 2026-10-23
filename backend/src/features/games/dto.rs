use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::domain::{ScoreDirection, ScoreMode, Status, ThresholdRule};

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerInput {
    pub name: String,
    pub team_number: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGameRequest {
    pub title: String,
    pub preset_code: String,
    pub custom_preset_name: Option<String>,
    pub team_mode: Option<bool>,
    pub score_direction: Option<ScoreDirection>,
    pub target_score: Option<i32>,
    pub threshold_rule: Option<ThresholdRule>,
    pub score_mode: Option<ScoreMode>,
    pub round_limit: Option<i32>,
    pub players: Vec<PlayerInput>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScoreInput {
    pub player_id: Option<Uuid>,
    pub team_number: Option<i32>,
    pub points: Option<i32>,
    pub bid: Option<i32>,
    pub made: Option<bool>,
    pub king_hearts: Option<i32>,
    pub queens: Option<i32>,
    pub diamonds: Option<i32>,
    pub tricks: Option<i32>,
    pub finish_position: Option<i32>,
    pub card_points: Option<i32>,
    pub bonus_points: Option<i32>,
    pub basras: Option<i32>,
    pub majority: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddRoundRequest {
    pub note: Option<String>,
    pub rule_code: Option<String>,
    pub bidder_team: Option<i32>,
    pub bid: Option<i32>,
    pub tricks: Option<i32>,
    pub scores: Vec<ScoreInput>,
}

#[derive(Debug, Deserialize)]
pub struct StatusRequest {
    pub status: Status,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerView {
    pub id: Uuid,
    pub name: String,
    pub team_number: Option<i32>,
    pub seat_order: i32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StandingView {
    pub key: String,
    pub name: String,
    pub player_ids: Vec<Uuid>,
    pub team_number: Option<i32>,
    pub total: i32,
    pub position: i32,
}

#[derive(Debug, Serialize)]
pub struct RoundScoreView {
    pub key: String,
    pub name: String,
    pub points: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundView {
    pub id: Uuid,
    pub round_number: i32,
    pub rule_code: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub scores: Vec<RoundScoreView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSummary {
    pub id: Uuid,
    pub title: String,
    pub preset_code: String,
    pub preset_name: String,
    pub team_mode: bool,
    pub score_direction: ScoreDirection,
    pub target_score: Option<i32>,
    pub threshold_rule: ThresholdRule,
    pub status: Status,
    pub score_mode: ScoreMode,
    pub round_limit: Option<i32>,
    pub round_count: usize,
    pub standings: Vec<StandingView>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameView {
    pub id: Uuid,
    pub title: String,
    pub preset_code: String,
    pub preset_name: String,
    pub team_mode: bool,
    pub score_direction: ScoreDirection,
    pub target_score: Option<i32>,
    pub threshold_rule: ThresholdRule,
    pub status: Status,
    pub score_mode: ScoreMode,
    pub round_limit: Option<i32>,
    pub players: Vec<PlayerView>,
    pub standings: Vec<StandingView>,
    pub rounds: Vec<RoundView>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
