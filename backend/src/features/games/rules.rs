use super::{dto::*, entity::*};
use crate::api::error::{bad, require, ApiResult};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub fn calculate(
    game: &GameData,
    input: &ScoreInput,
    request: &AddRoundRequest,
    player: &DbPlayer,
) -> ApiResult<i32> {
    match game.game.preset_code.as_str() {
        "400" => calculate_400(game, input, player),
        "tarneeb" => calculate_tarneeb(input, request),
        "trex" | "trex-complex" => calculate_trex(input, request.rule_code.as_deref()),
        "basra" => Ok(required(input.card_points, "Enter captured card points")?
            + required(input.basras, "Enter the number of Basras")? * 10
            + if input.majority == Some(true) { 3 } else { 0 }),
        "belote" => Ok(required(input.card_points, "Enter card points")?
            + required(input.bonus_points, "Enter declarations and bonuses")?),
        "hand" => required(input.card_points, "Enter each player's cards left"),
        _ => required(input.points, "Enter a score"),
    }
}

pub fn validate(game: &GameData, request: &AddRoundRequest, points: &[i32]) -> ApiResult<()> {
    match game.game.preset_code.as_str() {
        "leekha-solo" | "leekha-team" => {
            require(
                points.iter().all(|p| (0..=36).contains(p)),
                "Leekha scores must be between 0 and 36",
            )?;
            require(
                points.iter().sum::<i32>() == 36,
                "A Leekha hand must distribute exactly 36 points",
            )
        }
        "400" => validate_400(game, request),
        "tarneeb" => validate_tarneeb(request),
        "trex" => validate_trex(game, request, false),
        "trex-complex" => validate_trex(game, request, true),
        "basra" => validate_basra(request),
        "belote" => validate_belote(request),
        "hand" => {
            require(
                points.iter().all(|p| (0..=300).contains(p)),
                "Hand penalties must be between 0 and 300",
            )?;
            require(
                points.contains(&0),
                "At least one player must finish the hand with 0 points",
            )
        }
        _ => require(
            points.iter().all(|p| (-9999..=9999).contains(p)),
            "Score is outside the supported range",
        ),
    }
}

fn calculate_400(game: &GameData, input: &ScoreInput, player: &DbPlayer) -> ApiResult<i32> {
    let bid = required(input.bid, "Enter every player's bid")?;
    require((2..=13).contains(&bid), "400 bids must be between 2 and 13")?;
    if bid == 13 {
        return Ok(if input.made == Some(true) { 41 } else { -41 });
    }
    let values = HashMap::from([
        (2, 2),
        (3, 3),
        (4, 4),
        (5, 10),
        (6, 12),
        (7, 14),
        (8, 16),
        (9, 27),
        (10, 30),
        (11, 33),
        (12, 36),
    ]);
    let current = current_player_total(game, player.id);
    let value = if (5..=6).contains(&bid) && current >= 30 {
        bid
    } else {
        *values.get(&bid).unwrap()
    };
    Ok(if input.made == Some(true) {
        value
    } else {
        -value
    })
}

fn validate_400(game: &GameData, request: &AddRoundRequest) -> ApiResult<()> {
    let mut total_bids = 0;
    for score in &request.scores {
        total_bids += required(score.bid, "Enter every player's bid")?;
    }
    let high_score = game
        .players
        .iter()
        .map(|p| current_player_total(game, p.id))
        .max()
        .unwrap_or(0);
    let required_total = if high_score >= 50 {
        14
    } else if high_score >= 40 {
        13
    } else if high_score >= 30 {
        12
    } else {
        11
    };
    require(
        total_bids >= required_total,
        format!("The bids must total at least {required_total} at the current table score"),
    )?;
    for input in &request.scores {
        let player = game
            .players
            .iter()
            .find(|p| Some(p.id) == input.player_id)
            .ok_or_else(|| bad("Unknown player"))?;
        let total = current_player_total(game, player.id);
        let minimum = if total >= 50 {
            5
        } else if total >= 40 {
            4
        } else if total >= 30 {
            3
        } else {
            2
        };
        require(
            input.bid.unwrap_or_default() >= minimum,
            format!(
                "{} must bid at least {minimum} at their current score",
                player.name
            ),
        )?;
        require(
            input.made.is_some(),
            "Choose made or missed for every 400 bid",
        )?;
    }
    Ok(())
}

fn calculate_tarneeb(input: &ScoreInput, request: &AddRoundRequest) -> ApiResult<i32> {
    let bidder = required(request.bidder_team, "Choose the team that won the bid")?;
    let bid = required(request.bid, "Enter the winning bid")?;
    let tricks = required(request.tricks, "Enter the bidder's tricks")?;
    let team = required(input.team_number, "Missing team")?;
    Ok(if team == bidder {
        if tricks >= bid {
            tricks
        } else {
            -bid
        }
    } else if tricks >= bid {
        0
    } else {
        13 - tricks
    })
}

fn validate_tarneeb(request: &AddRoundRequest) -> ApiResult<()> {
    let bidder = required(request.bidder_team, "Choose the bidder team")?;
    let bid = required(request.bid, "Enter the bid")?;
    let tricks = required(request.tricks, "Enter tricks won")?;
    require(
        request.scores.iter().any(|s| s.team_number == Some(bidder)),
        "Unknown bidder team",
    )?;
    require((7..=13).contains(&bid), "Tarneeb bids must be from 7 to 13")?;
    require((0..=13).contains(&tricks), "Tricks must be from 0 to 13")
}

fn calculate_trex(input: &ScoreInput, code: Option<&str>) -> ApiResult<i32> {
    Ok(match code.ok_or_else(|| bad("Choose a Trex contract"))? {
        "KING" => -75 * value(input.king_hearts),
        "QUEENS" => -25 * value(input.queens),
        "DIAMONDS" => -10 * value(input.diamonds),
        "COLLECTIONS" => -15 * value(input.tricks),
        "TREX" => match required(input.finish_position, "Choose each finishing position")? {
            1 => 200,
            2 => 150,
            3 => 100,
            4 => 50,
            _ => return Err(bad("Trex positions must be 1 to 4")),
        },
        "COMPLEX" => {
            -75 * value(input.king_hearts)
                - 25 * value(input.queens)
                - 10 * value(input.diamonds)
                - 15 * value(input.tricks)
        }
        _ => return Err(bad("Unknown Trex contract")),
    })
}

fn validate_trex(game: &GameData, request: &AddRoundRequest, complex: bool) -> ApiResult<()> {
    let code = request.rule_code.as_deref().unwrap_or("");
    let allowed: HashSet<&str> = if complex {
        ["COMPLEX", "TREX"].into_iter().collect()
    } else {
        ["KING", "QUEENS", "DIAMONDS", "COLLECTIONS", "TREX"]
            .into_iter()
            .collect()
    };
    require(
        allowed.contains(code),
        "Choose a valid contract for this Trex mode",
    )?;
    require(
        game.rounds
            .iter()
            .filter(|r| r.rule_code.as_deref() == Some(code))
            .count()
            < 4,
        "That contract has already been played in all four kingdoms",
    )?;
    match code {
        "KING" => total(
            &request.scores,
            |s| value(s.king_hearts),
            1,
            "Exactly one player must capture the King of Hearts",
        )?,
        "QUEENS" => total(
            &request.scores,
            |s| value(s.queens),
            4,
            "The four queens must all be assigned",
        )?,
        "DIAMONDS" => total(
            &request.scores,
            |s| value(s.diamonds),
            13,
            "The 13 diamonds must all be assigned",
        )?,
        "COLLECTIONS" => total(
            &request.scores,
            |s| value(s.tricks),
            13,
            "The 13 tricks must all be assigned",
        )?,
        "COMPLEX" => {
            total(
                &request.scores,
                |s| value(s.king_hearts),
                1,
                "Exactly one King of Hearts must be assigned",
            )?;
            total(
                &request.scores,
                |s| value(s.queens),
                4,
                "The four queens must all be assigned",
            )?;
            total(
                &request.scores,
                |s| value(s.diamonds),
                13,
                "The 13 diamonds must all be assigned",
            )?;
            total(
                &request.scores,
                |s| value(s.tricks),
                13,
                "The 13 tricks must all be assigned",
            )?;
        }
        "TREX" => {
            let positions: HashSet<Option<i32>> =
                request.scores.iter().map(|s| s.finish_position).collect();
            require(
                positions == [Some(1), Some(2), Some(3), Some(4)].into_iter().collect(),
                "Use each Trex finish position exactly once",
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_basra(request: &AddRoundRequest) -> ApiResult<()> {
    let mut card_total = 0;
    for s in &request.scores {
        card_total += required(s.card_points, "Enter card points")?;
    }
    require(
        card_total == 13,
        "Lebanese Basra has exactly 13 special-card points per hand",
    )?;
    require(
        request
            .scores
            .iter()
            .all(|s| (0..=52).contains(&value(s.basras))),
        "Basra count is invalid",
    )?;
    require(
        request
            .scores
            .iter()
            .filter(|s| s.majority == Some(true))
            .count()
            <= 1,
        "Only one side can receive the 3-point card majority",
    )
}

fn validate_belote(request: &AddRoundRequest) -> ApiResult<()> {
    let mut card_total = 0;
    for s in &request.scores {
        card_total += required(s.card_points, "Enter card points")?;
    }
    require(
        card_total == 162,
        "Belote card points must total 162 before bonuses",
    )?;
    require(
        request
            .scores
            .iter()
            .all(|s| (0..=500).contains(&value(s.bonus_points))),
        "Belote bonuses must be from 0 to 500",
    )
}

fn current_player_total(game: &GameData, player_id: Uuid) -> i32 {
    game.players
        .iter()
        .find(|player| player.id == player_id)
        .map(|player| player.total_score)
        .unwrap_or_default()
}
fn total<F: Fn(&ScoreInput) -> i32>(
    scores: &[ScoreInput],
    get: F,
    expected: i32,
    message: &str,
) -> ApiResult<()> {
    require(scores.iter().map(get).sum::<i32>() == expected, message)
}
fn value(value: Option<i32>) -> i32 {
    value.unwrap_or(0)
}
fn required(value: Option<i32>, message: &str) -> ApiResult<i32> {
    value.ok_or_else(|| bad(message))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leekha_requires_exactly_36() {
        let game = sample("leekha-solo");
        let request = empty_request();
        assert!(validate(&game, &request, &[0, 10, 10, 16]).is_ok());
        assert_eq!(
            validate(&game, &request, &[13, 24, 0, 0])
                .unwrap_err()
                .to_string(),
            "A Leekha hand must distribute exactly 36 points"
        );
    }
    #[test]
    fn tarneeb_contract_calculation_matches_java() {
        let request = AddRoundRequest {
            bidder_team: Some(1),
            bid: Some(8),
            tricks: Some(7),
            ..empty_request()
        };
        let one = ScoreInput {
            team_number: Some(1),
            ..empty_score()
        };
        let two = ScoreInput {
            team_number: Some(2),
            ..empty_score()
        };
        assert_eq!(calculate_tarneeb(&one, &request).unwrap(), -8);
        assert_eq!(calculate_tarneeb(&two, &request).unwrap(), 6);
    }
    #[test]
    fn four_hundred_uses_table_values_and_score_based_values() {
        let mut game = sample("400");
        let player = DbPlayer {
            id: Uuid::new_v4(),
            game_id: game.game.id,
            name: "Elie".into(),
            team_number: Some(1),
            seat_order: 0,
            total_score: 0,
        };
        game.players.push(player.clone());
        let request = empty_request();
        let mut bid = empty_score();
        bid.bid = Some(5);
        bid.made = Some(true);
        bid.player_id = Some(player.id);
        assert_eq!(calculate(&game, &bid, &request, &player).unwrap(), 10);
        let round = Uuid::new_v4();
        game.rounds.push(DbRound {
            id: round,
            round_number: 1,
            rule_code: None,
            note: None,
            created_at: chrono::Utc::now(),
        });
        game.players[0].total_score = 30;
        assert_eq!(calculate(&game, &bid, &request, &player).unwrap(), 5);
        bid.bid = Some(13);
        bid.made = Some(false);
        assert_eq!(calculate(&game, &bid, &request, &player).unwrap(), -41);
    }
    #[test]
    fn trex_contracts_and_positions_match_the_score_table() {
        let mut score = empty_score();
        score.king_hearts = Some(1);
        score.queens = Some(2);
        score.diamonds = Some(3);
        score.tricks = Some(4);
        assert_eq!(calculate_trex(&score, Some("KING")).unwrap(), -75);
        assert_eq!(calculate_trex(&score, Some("COMPLEX")).unwrap(), -215);
        score.finish_position = Some(1);
        assert_eq!(calculate_trex(&score, Some("TREX")).unwrap(), 200);
        let mut request = empty_request();
        request.rule_code = Some("TREX".into());
        request.scores = (1..=4)
            .map(|position| {
                let mut s = empty_score();
                s.finish_position = Some(position);
                s
            })
            .collect();
        assert!(validate_trex(&sample("trex"), &request, false).is_ok());
    }
    #[test]
    fn basra_belote_and_hand_enforce_full_hand_totals() {
        let mut basra = empty_request();
        let mut a = empty_score();
        a.card_points = Some(7);
        a.basras = Some(1);
        a.majority = Some(true);
        let mut b = empty_score();
        b.card_points = Some(6);
        b.basras = Some(0);
        b.majority = Some(false);
        basra.scores = vec![a, b];
        assert!(validate_basra(&basra).is_ok());
        basra.scores[1].card_points = Some(5);
        assert!(validate_basra(&basra).is_err());

        let mut belote = empty_request();
        let mut a = empty_score();
        a.card_points = Some(100);
        a.bonus_points = Some(20);
        let mut b = empty_score();
        b.card_points = Some(62);
        b.bonus_points = Some(0);
        belote.scores = vec![a, b];
        assert!(validate_belote(&belote).is_ok());

        assert!(validate(&sample("hand"), &empty_request(), &[0, 20, 35, 50]).is_ok());
        assert!(validate(&sample("hand"), &empty_request(), &[10, 20, 35, 50]).is_err());
    }
    fn sample(code: &str) -> GameData {
        GameData {
            game: DbGame {
                id: Uuid::nil(),
                title: "x".into(),
                preset_code: code.into(),
                preset_name: "x".into(),
                team_mode: false,
                score_direction: "HIGH_WINS".into(),
                target_score: None,
                threshold_rule: "NONE".into(),
                score_mode: "INDIVIDUAL".into(),
                round_limit: None,
                round_count: 0,
                status: "ACTIVE".into(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
            players: vec![],
            rounds: vec![],
            scores: vec![],
        }
    }
    fn empty_request() -> AddRoundRequest {
        AddRoundRequest {
            note: None,
            rule_code: None,
            bidder_team: None,
            bid: None,
            tricks: None,
            scores: vec![],
        }
    }
    fn empty_score() -> ScoreInput {
        ScoreInput {
            player_id: None,
            team_number: None,
            points: None,
            bid: None,
            made: None,
            king_hearts: None,
            queens: None,
            diamonds: None,
            tricks: None,
            finish_position: None,
            card_points: None,
            bonus_points: None,
            basras: None,
            majority: None,
        }
    }
}
