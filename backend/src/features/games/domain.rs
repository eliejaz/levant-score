use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Active,
    Finished,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Finished => "FINISHED",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScoreDirection {
    HighWins,
    LowWins,
}

impl ScoreDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HighWins => "HIGH_WINS",
            Self::LowWins => "LOW_WINS",
        }
    }

    pub fn parse(value: &str) -> Self {
        if value == "LOW_WINS" {
            Self::LowWins
        } else {
            Self::HighWins
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThresholdRule {
    WinAtOrAbove,
    LoseAtOrAbove,
    PlayerWinWithPartnerPositive,
    None,
}

impl ThresholdRule {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WinAtOrAbove => "WIN_AT_OR_ABOVE",
            Self::LoseAtOrAbove => "LOSE_AT_OR_ABOVE",
            Self::PlayerWinWithPartnerPositive => "PLAYER_WIN_WITH_PARTNER_POSITIVE",
            Self::None => "NONE",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "WIN_AT_OR_ABOVE" => Self::WinAtOrAbove,
            "LOSE_AT_OR_ABOVE" => Self::LoseAtOrAbove,
            "PLAYER_WIN_WITH_PARTNER_POSITIVE" => Self::PlayerWinWithPartnerPositive,
            _ => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScoreMode {
    Individual,
    Team,
}

impl ScoreMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Individual => "INDIVIDUAL",
            Self::Team => "TEAM",
        }
    }

    pub fn parse(value: &str) -> Self {
        if value == "TEAM" {
            Self::Team
        } else {
            Self::Individual
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub code: String,
    pub name: String,
    pub name_ar: String,
    pub description: String,
    pub description_ar: String,
    pub team_mode: bool,
    #[serde(default)]
    pub min_players: i32,
    #[serde(default)]
    pub max_players: i32,
    pub allowed_player_counts: Vec<i32>,
    pub direction: ScoreDirection,
    pub default_target: Option<i32>,
    pub threshold_rule: ThresholdRule,
    pub score_mode: ScoreMode,
    pub round_limit: Option<i32>,
    pub rule: String,
    pub rule_ar: String,
    pub accent: String,
}
