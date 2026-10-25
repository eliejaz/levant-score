use levant_score_rules::{
    evaluate_round, GameState as EngineGame, PlayerState as EnginePlayer,
    RoundRequest as EngineRound, ScoreInput as EngineScore,
};

use super::{dto::AddRoundRequest, entity::*};
use crate::api::error::{bad, ApiResult};
use uuid::Uuid;

pub struct Evaluation {
    pub entries: Vec<(DbPlayer, i32)>,
    pub finished: bool,
    pub outcome: Option<levant_score_rules::GameOutcome>,
}

pub fn evaluate(game: &GameData, request: &AddRoundRequest) -> ApiResult<Evaluation> {
    let state = EngineGame {
        preset_code: game.game.preset_code.clone(),
        team_mode: game.game.team_mode,
        score_direction: game.game.score_direction.clone(),
        score_mode: game.game.score_mode.clone(),
        target_score: game.game.target_score,
        threshold_rule: game.game.threshold_rule.clone(),
        round_limit: game.game.round_limit,
        round_count: game.game.round_count,
        players: game
            .players
            .iter()
            .map(|player| EnginePlayer {
                id: player.id.to_string(),
                name: player.name.clone(),
                team_number: player.team_number,
                total_score: player.total_score,
            })
            .collect(),
        prior_round_rules: game
            .rounds
            .iter()
            .map(|round| round.rule_code.clone())
            .collect(),
    };
    let input = EngineRound {
        rule_code: request.rule_code.clone(),
        bidder_team: request.bidder_team,
        bid: request.bid,
        tricks: request.tricks,
        scores: request
            .scores
            .iter()
            .map(|score| EngineScore {
                player_id: score.player_id.map(|id| id.to_string()),
                team_number: score.team_number,
                points: score.points,
                bid: score.bid,
                made: score.made,
                king_hearts: score.king_hearts,
                queens: score.queens,
                diamonds: score.diamonds,
                tricks: score.tricks,
                finish_position: score.finish_position,
                card_points: score.card_points,
                bonus_points: score.bonus_points,
                basras: score.basras,
                majority: score.majority,
            })
            .collect(),
    };
    let result = evaluate_round(&state, &input).map_err(bad)?;
    let entries = result
        .scores
        .into_iter()
        .map(|score| {
            let player_id = Uuid::parse_str(&score.player_id).map_err(|_| bad("Unknown player"))?;
            let player = game
                .players
                .iter()
                .find(|player| player.id == player_id)
                .cloned()
                .ok_or_else(|| bad("Unknown player"))?;
            Ok((player, score.points))
        })
        .collect::<ApiResult<Vec<_>>>()?;
    Ok(Evaluation {
        entries,
        finished: result.finished,
        outcome: result.outcome,
    })
}
