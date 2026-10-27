use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

mod presets_json;
pub use presets_json::PRESETS_JSON;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    pub preset_code: String,
    pub team_mode: bool,
    pub score_direction: String,
    pub score_mode: String,
    pub target_score: Option<i32>,
    pub threshold_rule: String,
    pub round_limit: Option<i32>,
    pub round_count: i32,
    pub players: Vec<PlayerState>,
    #[serde(default)]
    pub prior_round_rules: Vec<Option<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub id: String,
    pub name: String,
    pub team_number: Option<i32>,
    pub total_score: i32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundRequest {
    pub rule_code: Option<String>,
    pub bidder_team: Option<i32>,
    pub bid: Option<i32>,
    pub tricks: Option<i32>,
    pub leekha_shown_by_player_id: Option<String>,
    pub leekha_captured_by_player_id: Option<String>,
    pub leekha_ten_diamonds_shown_by_player_id: Option<String>,
    pub leekha_ten_diamonds_captured_by_player_id: Option<String>,
    pub quatorze_winner_player_id: Option<String>,
    pub quatorze_all_at_once: Option<bool>,
    pub scores: Vec<ScoreInput>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreInput {
    pub player_id: Option<String>,
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
    pub cards_left: Option<i32>,
    pub jokers: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScoreDelta {
    pub player_id: String,
    pub points: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoundEvaluation {
    pub scores: Vec<ScoreDelta>,
    pub finished: bool,
    pub outcome: Option<GameOutcome>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameOutcome {
    pub reason: String,
    pub winner_player_ids: Vec<String>,
    pub loser_player_ids: Vec<String>,
    pub trigger_player_id: Option<String>,
}

pub fn evaluate_round(game: &GameState, request: &RoundRequest) -> Result<RoundEvaluation, String> {
    require(!request.scores.is_empty(), "scores must not be empty")?;
    let mut used = HashSet::new();
    let mut resolved = Vec::with_capacity(request.scores.len());
    for input in &request.scores {
        let player = if game.score_mode == "TEAM" && input.team_number.is_some() {
            game.players
                .iter()
                .find(|player| player.team_number == input.team_number)
                .ok_or_else(|| "Unknown team".to_owned())?
        } else {
            let id = input
                .player_id
                .as_deref()
                .ok_or_else(|| "Unknown player".to_owned())?;
            game.players
                .iter()
                .find(|player| player.id == id)
                .ok_or_else(|| "Unknown player".to_owned())?
        };
        require(used.insert(player.id.as_str()), "Duplicate score")?;
        resolved.push((player, input));
    }
    let expected = if game.score_mode == "TEAM" {
        game.players
            .iter()
            .map(|player| player.team_number)
            .collect::<HashSet<_>>()
            .len()
    } else {
        game.players.len()
    };
    require(
        used.len() == expected,
        "Add one score for every player or team",
    )?;

    let points: Vec<i32> = resolved
        .iter()
        .map(|(player, input)| calculate(game, input, request, player))
        .collect::<Result<_, _>>()?;
    validate(game, request, &points)?;
    let scores: Vec<ScoreDelta> = resolved
        .iter()
        .zip(points)
        .map(|((player, _), points)| ScoreDelta {
            player_id: player.id.clone(),
            points,
        })
        .collect();
    let mut proposed = game.clone();
    proposed.round_count += 1;
    for delta in &scores {
        if let Some(player) = proposed
            .players
            .iter_mut()
            .find(|player| player.id == delta.player_id)
        {
            player.total_score += delta.points;
        }
    }
    let outcome = determine_outcome(&proposed, request);
    Ok(RoundEvaluation {
        scores,
        finished: outcome.is_some(),
        outcome,
    })
}

fn calculate(
    game: &GameState,
    input: &ScoreInput,
    request: &RoundRequest,
    player: &PlayerState,
) -> Result<i32, String> {
    match game.preset_code.as_str() {
        "leekha-solo-shown" | "leekha-team-shown" => calculate_shown_leekha(input, request, player),
        "400" => calculate_400(input, player),
        "tarneeb" => calculate_tarneeb(input, request),
        "trex" | "trex-complex" => calculate_trex(input, request.rule_code.as_deref()),
        "basra" => Ok(required(input.card_points, "Enter captured card points")?
            + required(input.basras, "Enter the number of Basras")? * 10
            + i32::from(input.majority == Some(true)) * 3),
        "belote" => Ok(required(input.card_points, "Enter card points")?
            + required(input.bonus_points, "Enter declarations and bonuses")?),
        "hand" => required(input.card_points, "Enter each player's cards left"),
        "chinese" => calculate_chinese(input),
        "quatorze" => calculate_quatorze(input, request, player),
        _ => required(input.points, "Enter a score"),
    }
}

fn validate(game: &GameState, request: &RoundRequest, points: &[i32]) -> Result<(), String> {
    match game.preset_code.as_str() {
        "leekha-solo" | "leekha-team" => validate_leekha(game, request, false),
        "leekha-solo-shown" | "leekha-team-shown" => validate_leekha(game, request, true),
        "400" => validate_400(game, request),
        "tarneeb" => validate_tarneeb(request),
        "trex" => validate_trex(game, request, false),
        "trex-complex" => validate_trex(game, request, true),
        "basra" => validate_basra(request),
        "belote" => validate_belote(request),
        "hand" => {
            require(
                points.iter().all(|point| (0..=300).contains(point)),
                "Hand penalties must be between 0 and 300",
            )?;
            require(
                points.contains(&0),
                "At least one player must finish the hand with 0 points",
            )
        }
        "chinese" => validate_chinese(request),
        "quatorze" => validate_quatorze(game, request),
        _ => require(
            points.iter().all(|point| (-9999..=9999).contains(point)),
            "Score is outside the supported range",
        ),
    }
}

fn calculate_shown_leekha(
    input: &ScoreInput,
    request: &RoundRequest,
    player: &PlayerState,
) -> Result<i32, String> {
    let base = required(input.points, "Enter every player's 36-point Leekha score")?;
    Ok(base
        + shown_card_bonus(
            13,
            player,
            request.leekha_shown_by_player_id.as_deref(),
            request.leekha_captured_by_player_id.as_deref(),
        )
        + shown_card_bonus(
            10,
            player,
            request.leekha_ten_diamonds_shown_by_player_id.as_deref(),
            request.leekha_ten_diamonds_captured_by_player_id.as_deref(),
        ))
}

fn shown_card_bonus(
    card_value: i32,
    player: &PlayerState,
    shown_by: Option<&str>,
    captured_by: Option<&str>,
) -> i32 {
    if captured_by != Some(player.id.as_str()) {
        return 0;
    }
    if shown_by == captured_by {
        card_value * 2
    } else {
        card_value
    }
}

fn validate_leekha(
    game: &GameState,
    request: &RoundRequest,
    shown_variant: bool,
) -> Result<(), String> {
    let base = request
        .scores
        .iter()
        .map(|score| required(score.points, "Enter every player's Leekha score"))
        .collect::<Result<Vec<_>, _>>()?;
    require(
        base.iter().all(|point| (0..=36).contains(point)),
        "Leekha scores must be between 0 and 36",
    )?;
    require(
        base.iter().sum::<i32>() == 36,
        if shown_variant {
            "A shown-Leekha hand must distribute exactly 36 base points"
        } else {
            "A Leekha hand must distribute exactly 36 points"
        },
    )?;
    if !shown_variant {
        return require(
            request.leekha_shown_by_player_id.is_none()
                && request.leekha_captured_by_player_id.is_none()
                && request.leekha_ten_diamonds_shown_by_player_id.is_none()
                && request.leekha_ten_diamonds_captured_by_player_id.is_none(),
            "Shown Leekha bonuses require the shown-card game mode",
        );
    }
    validate_open_leekha_card(
        game,
        "Queen of Spades",
        request.leekha_shown_by_player_id.as_deref(),
        request.leekha_captured_by_player_id.as_deref(),
    )?;
    validate_open_leekha_card(
        game,
        "Ten of Diamonds",
        request.leekha_ten_diamonds_shown_by_player_id.as_deref(),
        request.leekha_ten_diamonds_captured_by_player_id.as_deref(),
    )?;

    let mut required_base_by_captor: HashMap<&str, i32> = HashMap::new();
    if let Some(captor) = request.leekha_captured_by_player_id.as_deref() {
        *required_base_by_captor.entry(captor).or_default() += 13;
    }
    if let Some(captor) = request.leekha_ten_diamonds_captured_by_player_id.as_deref() {
        *required_base_by_captor.entry(captor).or_default() += 10;
    }
    for (captor, required_base) in required_base_by_captor {
        let captured_score = request
            .scores
            .iter()
            .find(|score| score.player_id.as_deref() == Some(captor))
            .and_then(|score| score.points)
            .unwrap_or_default();
        require(
            captured_score >= required_base,
            "A player must receive the base value of every open Leekha card they captured",
        )?;
    }
    Ok(())
}

fn validate_open_leekha_card(
    game: &GameState,
    card_name: &str,
    shown_by: Option<&str>,
    captured_by: Option<&str>,
) -> Result<(), String> {
    match (shown_by, captured_by) {
        (None, None) => Ok(()),
        (Some(shown_by), Some(captured_by)) => {
            require(
                game.players.iter().any(|player| player.id == shown_by),
                &format!("Choose who showed the {card_name}"),
            )?;
            require(
                game.players.iter().any(|player| player.id == captured_by),
                &format!("Choose who captured the {card_name}"),
            )
        }
        _ => Err(format!(
            "Choose both who showed and who captured the {card_name}"
        )),
    }
}

fn calculate_chinese(input: &ScoreInput) -> Result<i32, String> {
    let cards = required(
        input.cards_left,
        "Enter the cards left for every Chinese player",
    )?;
    require(
        (0..=13).contains(&cards),
        "Cards left must be between 0 and 13",
    )?;
    Ok(match cards {
        0 => 0,
        1..=4 => cards,
        5..=9 => cards * 2,
        10..=12 => cards * 3,
        13 => 40,
        _ => unreachable!(),
    })
}

fn validate_chinese(request: &RoundRequest) -> Result<(), String> {
    let cards = request
        .scores
        .iter()
        .map(|score| {
            required(
                score.cards_left,
                "Enter the cards left for every Chinese player",
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    require(
        cards.iter().all(|cards| (0..=13).contains(cards)),
        "Cards left must be between 0 and 13",
    )?;
    require(
        cards.iter().filter(|cards| **cards == 0).count() == 1,
        "Choose exactly one player who finished their cards first",
    )
}

fn calculate_quatorze(
    input: &ScoreInput,
    request: &RoundRequest,
    player: &PlayerState,
) -> Result<i32, String> {
    if request.quatorze_winner_player_id.as_deref() == Some(player.id.as_str()) {
        return Ok(if request.quatorze_all_at_once == Some(true) {
            -60
        } else {
            -30
        });
    }
    let cards = required(
        input.card_points,
        "Enter every opponent's remaining-card value",
    )?;
    let jokers = value(input.jokers);
    let penalty = cards + jokers * 25;
    Ok(if request.quatorze_all_at_once == Some(true) {
        penalty * 2
    } else {
        penalty
    })
}

fn validate_quatorze(game: &GameState, request: &RoundRequest) -> Result<(), String> {
    let winner = request
        .quatorze_winner_player_id
        .as_deref()
        .ok_or_else(|| "Choose who went out first".to_owned())?;
    require(
        game.players.iter().any(|player| player.id == winner),
        "Choose a valid Quatorze winner",
    )?;
    let mut jokers = 0;
    for score in &request.scores {
        if score.player_id.as_deref() == Some(winner) {
            continue;
        }
        let cards = required(
            score.card_points,
            "Enter every opponent's remaining-card value",
        )?;
        let player_jokers = value(score.jokers);
        require(
            (0..=140).contains(&cards),
            "Remaining-card value must be between 0 and 140",
        )?;
        require(
            (0..=2).contains(&player_jokers),
            "A player can hold at most 2 Jokers",
        )?;
        require(
            cards > 0 || player_jokers > 0,
            "Every opponent must have a remaining-card penalty",
        )?;
        jokers += player_jokers;
    }
    require(jokers <= 2, "Quatorze uses at most 2 Jokers in total")
}

fn calculate_400(input: &ScoreInput, player: &PlayerState) -> Result<i32, String> {
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
    let value = if (5..=6).contains(&bid) && player.total_score >= 30 {
        bid
    } else {
        values[&bid]
    };
    Ok(if input.made == Some(true) {
        value
    } else {
        -value
    })
}

fn validate_400(game: &GameState, request: &RoundRequest) -> Result<(), String> {
    let bids = request
        .scores
        .iter()
        .map(|score| required(score.bid, "Enter every player's bid"))
        .collect::<Result<Vec<_>, _>>()?;
    let high_score = game
        .players
        .iter()
        .map(|player| player.total_score)
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
        bids.iter().sum::<i32>() >= required_total,
        format!("The bids must total at least {required_total} at the current table score"),
    )?;
    for input in &request.scores {
        let player = game
            .players
            .iter()
            .find(|player| input.player_id.as_deref() == Some(player.id.as_str()))
            .ok_or_else(|| "Unknown player".to_owned())?;
        let minimum = if player.total_score >= 50 {
            5
        } else if player.total_score >= 40 {
            4
        } else if player.total_score >= 30 {
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

fn calculate_tarneeb(input: &ScoreInput, request: &RoundRequest) -> Result<i32, String> {
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

fn validate_tarneeb(request: &RoundRequest) -> Result<(), String> {
    let bidder = required(request.bidder_team, "Choose the bidder team")?;
    let bid = required(request.bid, "Enter the bid")?;
    let tricks = required(request.tricks, "Enter tricks won")?;
    require(
        request
            .scores
            .iter()
            .any(|score| score.team_number == Some(bidder)),
        "Unknown bidder team",
    )?;
    require((7..=13).contains(&bid), "Tarneeb bids must be from 7 to 13")?;
    require((0..=13).contains(&tricks), "Tricks must be from 0 to 13")
}

fn calculate_trex(input: &ScoreInput, code: Option<&str>) -> Result<i32, String> {
    Ok(
        match code.ok_or_else(|| "Choose a Trex contract".to_owned())? {
            "KING" => -75 * value(input.king_hearts),
            "QUEENS" => -25 * value(input.queens),
            "DIAMONDS" => -10 * value(input.diamonds),
            "COLLECTIONS" => -15 * value(input.tricks),
            "TREX" => match required(input.finish_position, "Choose each finishing position")? {
                1 => 200,
                2 => 150,
                3 => 100,
                4 => 50,
                _ => return Err("Trex positions must be 1 to 4".to_owned()),
            },
            "COMPLEX" => {
                -75 * value(input.king_hearts)
                    - 25 * value(input.queens)
                    - 10 * value(input.diamonds)
                    - 15 * value(input.tricks)
            }
            _ => return Err("Unknown Trex contract".to_owned()),
        },
    )
}

fn validate_trex(game: &GameState, request: &RoundRequest, complex: bool) -> Result<(), String> {
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
        game.prior_round_rules
            .iter()
            .filter(|rule| rule.as_deref() == Some(code))
            .count()
            < 4,
        "That contract has already been played in all four kingdoms",
    )?;
    match code {
        "KING" => total(
            &request.scores,
            |score| value(score.king_hearts),
            1,
            "Exactly one player must capture the King of Hearts",
        )?,
        "QUEENS" => total(
            &request.scores,
            |score| value(score.queens),
            4,
            "The four queens must all be assigned",
        )?,
        "DIAMONDS" => total(
            &request.scores,
            |score| value(score.diamonds),
            13,
            "The 13 diamonds must all be assigned",
        )?,
        "COLLECTIONS" => total(
            &request.scores,
            |score| value(score.tricks),
            13,
            "The 13 tricks must all be assigned",
        )?,
        "COMPLEX" => {
            total(
                &request.scores,
                |score| value(score.king_hearts),
                1,
                "Exactly one King of Hearts must be assigned",
            )?;
            total(
                &request.scores,
                |score| value(score.queens),
                4,
                "The four queens must all be assigned",
            )?;
            total(
                &request.scores,
                |score| value(score.diamonds),
                13,
                "The 13 diamonds must all be assigned",
            )?;
            total(
                &request.scores,
                |score| value(score.tricks),
                13,
                "The 13 tricks must all be assigned",
            )?;
        }
        "TREX" => {
            let positions: HashSet<Option<i32>> = request
                .scores
                .iter()
                .map(|score| score.finish_position)
                .collect();
            require(
                positions == [Some(1), Some(2), Some(3), Some(4)].into_iter().collect(),
                "Use each Trex finish position exactly once",
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_basra(request: &RoundRequest) -> Result<(), String> {
    let cards = request
        .scores
        .iter()
        .map(|score| required(score.card_points, "Enter card points"))
        .collect::<Result<Vec<_>, _>>()?;
    require(
        cards.iter().sum::<i32>() == 13,
        "Lebanese Basra has exactly 13 special-card points per hand",
    )?;
    require(
        request
            .scores
            .iter()
            .all(|score| (0..=52).contains(&value(score.basras))),
        "Basra count is invalid",
    )?;
    require(
        request
            .scores
            .iter()
            .filter(|score| score.majority == Some(true))
            .count()
            <= 1,
        "Only one side can receive the 3-point card majority",
    )
}

fn validate_belote(request: &RoundRequest) -> Result<(), String> {
    let cards = request
        .scores
        .iter()
        .map(|score| required(score.card_points, "Enter card points"))
        .collect::<Result<Vec<_>, _>>()?;
    require(
        cards.iter().sum::<i32>() == 162,
        "Belote card points must total 162 before bonuses",
    )?;
    require(
        request
            .scores
            .iter()
            .all(|score| (0..=500).contains(&value(score.bonus_points))),
        "Belote bonuses must be from 0 to 500",
    )
}

fn determine_outcome(game: &GameState, request: &RoundRequest) -> Option<GameOutcome> {
    if game.preset_code == "400" {
        if let Some(input) = request.scores.iter().find(|score| score.bid == Some(13)) {
            let player = game
                .players
                .iter()
                .find(|player| input.player_id.as_deref() == Some(player.id.as_str()))?;
            let made = input.made == Some(true);
            let winning_team = if made {
                player.team_number
            } else {
                game.players
                    .iter()
                    .find(|candidate| candidate.team_number != player.team_number)
                    .and_then(|candidate| candidate.team_number)
            };
            return Some(outcome_for_team(
                game,
                if made {
                    "400_SLAM_MADE"
                } else {
                    "400_SLAM_MISSED"
                },
                winning_team,
                Some(&player.id),
            ));
        }
    }

    if game
        .round_limit
        .is_some_and(|limit| game.round_count >= limit)
    {
        let standings = standing_values(game);
        let winner = if game.score_direction == "LOW_WINS" {
            standings.iter().min_by_key(|standing| standing.total)
        } else {
            standings.iter().max_by_key(|standing| standing.total)
        }?;
        return Some(outcome_for_standing(game, "ROUND_LIMIT", winner, None));
    }
    let target = game.target_score?;
    if game.threshold_rule == "NONE" || game.round_count == 0 {
        return None;
    }
    let current = standing_values(game);
    if game.threshold_rule == "PLAYER_WIN_WITH_PARTNER_POSITIVE" {
        let mut threshold = target;
        loop {
            let teams: HashSet<i32> = current
                .iter()
                .filter(|standing| standing.total >= threshold)
                .filter_map(|standing| standing.team_number.map(|team| (standing, team)))
                .filter(|(standing, team)| {
                    current
                        .iter()
                        .filter(|partner| {
                            partner.team_number == Some(*team) && partner.key != standing.key
                        })
                        .all(|partner| partner.total > 0)
                })
                .map(|(_, team)| team)
                .collect();
            if teams.len() == 1 {
                return Some(outcome_for_team(
                    game,
                    "PLAYER_WITH_POSITIVE_PARTNER",
                    teams.into_iter().next(),
                    None,
                ));
            }
            if teams.len() < 2 {
                return None;
            }
            threshold += 10;
        }
    }
    if game.threshold_rule == "LOSE_AT_OR_ABOVE" {
        let loser = current
            .iter()
            .filter(|standing| standing.total >= target)
            .max_by_key(|standing| standing.total)?;
        if game.team_mode {
            let winning_team = game
                .players
                .iter()
                .find(|player| player.team_number != loser.team_number)
                .and_then(|player| player.team_number);
            return Some(outcome_for_team(
                game,
                "LOSE_AT_OR_ABOVE",
                winning_team,
                Some(loser.key),
            ));
        }
        let winner = current.iter().min_by_key(|standing| standing.total)?;
        return Some(GameOutcome {
            reason: "LOSE_AT_OR_ABOVE".into(),
            winner_player_ids: player_ids_for_standing(game, winner),
            loser_player_ids: player_ids_for_standing(game, loser),
            trigger_player_id: Some(loser.key.to_owned()),
        });
    }
    let winner = current
        .iter()
        .filter(|standing| standing.total >= target)
        .max_by_key(|standing| standing.total)?;
    Some(outcome_for_standing(
        game,
        "WIN_AT_OR_ABOVE",
        winner,
        Some(winner.key),
    ))
}

fn outcome_for_standing(
    game: &GameState,
    reason: &str,
    winner: &StandingValue<'_>,
    trigger: Option<&str>,
) -> GameOutcome {
    let winner_ids = player_ids_for_standing(game, winner);
    GameOutcome {
        reason: reason.into(),
        winner_player_ids: winner_ids.clone(),
        loser_player_ids: game
            .players
            .iter()
            .filter(|player| !winner_ids.contains(&player.id))
            .map(|player| player.id.clone())
            .collect(),
        trigger_player_id: trigger.map(str::to_owned),
    }
}

fn outcome_for_team(
    game: &GameState,
    reason: &str,
    winning_team: Option<i32>,
    trigger: Option<&str>,
) -> GameOutcome {
    let winner_player_ids = game
        .players
        .iter()
        .filter(|player| player.team_number == winning_team)
        .map(|player| player.id.clone())
        .collect::<Vec<_>>();
    GameOutcome {
        reason: reason.into(),
        loser_player_ids: game
            .players
            .iter()
            .filter(|player| !winner_player_ids.contains(&player.id))
            .map(|player| player.id.clone())
            .collect(),
        winner_player_ids,
        trigger_player_id: trigger.map(str::to_owned),
    }
}

fn player_ids_for_standing(game: &GameState, standing: &StandingValue<'_>) -> Vec<String> {
    if game.score_mode == "TEAM" {
        game.players
            .iter()
            .filter(|player| player.team_number == standing.team_number)
            .map(|player| player.id.clone())
            .collect()
    } else {
        vec![standing.key.to_owned()]
    }
}

struct StandingValue<'a> {
    key: &'a str,
    team_number: Option<i32>,
    total: i32,
}

fn standing_values(game: &GameState) -> Vec<StandingValue<'_>> {
    if game.score_mode != "TEAM" {
        return game
            .players
            .iter()
            .map(|player| StandingValue {
                key: &player.id,
                team_number: player.team_number,
                total: player.total_score,
            })
            .collect();
    }
    let mut result: Vec<StandingValue<'_>> = Vec::new();
    for player in &game.players {
        if let Some(existing) = result
            .iter_mut()
            .find(|standing| standing.team_number == player.team_number)
        {
            existing.total += player.total_score;
        } else {
            result.push(StandingValue {
                key: &player.id,
                team_number: player.team_number,
                total: player.total_score,
            });
        }
    }
    result
}

fn total<F: Fn(&ScoreInput) -> i32>(
    scores: &[ScoreInput],
    get: F,
    expected: i32,
    message: &str,
) -> Result<(), String> {
    require(scores.iter().map(get).sum::<i32>() == expected, message)
}
fn value(value: Option<i32>) -> i32 {
    value.unwrap_or(0)
}
fn required(value: Option<i32>, message: &str) -> Result<i32, String> {
    value.ok_or_else(|| message.to_owned())
}
fn require(condition: bool, message: impl Into<String>) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

#[cfg(feature = "wasm")]
mod wasm {
    use super::*;
    use serde::Serialize;
    use std::{
        mem, slice,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static RESULT_LEN: AtomicUsize = AtomicUsize::new(0);

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct EngineCall {
        game: GameState,
        request: RoundRequest,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct EngineResponse {
        ok: bool,
        result: Option<RoundEvaluation>,
        error: Option<String>,
    }

    #[no_mangle]
    pub extern "C" fn engine_version() -> u32 {
        5
    }

    #[no_mangle]
    pub extern "C" fn alloc(len: usize) -> *mut u8 {
        let mut buffer = Vec::<u8>::with_capacity(len);
        let pointer = buffer.as_mut_ptr();
        mem::forget(buffer);
        pointer
    }

    #[no_mangle]
    pub unsafe extern "C" fn dealloc(pointer: *mut u8, len: usize) {
        drop(Vec::from_raw_parts(pointer, 0, len));
    }

    #[no_mangle]
    pub unsafe extern "C" fn evaluate_round_json(pointer: *const u8, len: usize) -> *mut u8 {
        let response =
            match serde_json::from_slice::<EngineCall>(slice::from_raw_parts(pointer, len)) {
                Ok(call) => match evaluate_round(&call.game, &call.request) {
                    Ok(result) => EngineResponse {
                        ok: true,
                        result: Some(result),
                        error: None,
                    },
                    Err(error) => EngineResponse {
                        ok: false,
                        result: None,
                        error: Some(error),
                    },
                },
                Err(error) => EngineResponse {
                    ok: false,
                    result: None,
                    error: Some(format!("Invalid rules input: {error}")),
                },
            };
        let output = serde_json::to_vec(&response)
            .unwrap_or_else(|_| br#"{"ok":false,"error":"Rules serialization failed"}"#.to_vec())
            .into_boxed_slice();
        RESULT_LEN.store(output.len(), Ordering::Release);
        Box::into_raw(output) as *mut u8
    }

    #[no_mangle]
    pub extern "C" fn presets_json() -> *mut u8 {
        let output = PRESETS_JSON.as_bytes().to_vec().into_boxed_slice();
        RESULT_LEN.store(output.len(), Ordering::Release);
        Box::into_raw(output) as *mut u8
    }

    #[no_mangle]
    pub extern "C" fn result_len() -> usize {
        RESULT_LEN.load(Ordering::Acquire)
    }

    #[no_mangle]
    pub unsafe extern "C" fn result_dealloc(pointer: *mut u8, len: usize) {
        drop(Box::from_raw(slice::from_raw_parts_mut(pointer, len)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leekha_requires_exactly_36() {
        let game = sample(
            "leekha-team",
            "INDIVIDUAL",
            "LOSE_AT_OR_ABOVE",
            Some(101),
            None,
        );
        let request = points(&game, &[0, 10, 10, 16]);
        assert!(evaluate_round(&game, &request).is_ok());
        let invalid = points(&game, &[13, 24, 0, 0]);
        assert_eq!(
            evaluate_round(&game, &invalid).unwrap_err(),
            "A Leekha hand must distribute exactly 36 points"
        );
    }

    #[test]
    fn open_leekha_scores_queen_and_ten_independently() {
        let game = sample(
            "leekha-solo-shown",
            "INDIVIDUAL",
            "LOSE_AT_OR_ABOVE",
            Some(201),
            None,
        );
        let mut double = points(&game, &[5, 13, 10, 8]);
        double.leekha_shown_by_player_id = Some("p0".into());
        double.leekha_captured_by_player_id = Some("p1".into());
        assert_eq!(
            evaluate_round(&game, &double)
                .unwrap()
                .scores
                .iter()
                .map(|score| score.points)
                .collect::<Vec<_>>(),
            vec![5, 26, 10, 8]
        );

        let mut triple = points(&game, &[13, 10, 8, 5]);
        triple.leekha_shown_by_player_id = Some("p0".into());
        triple.leekha_captured_by_player_id = Some("p0".into());
        assert_eq!(evaluate_round(&game, &triple).unwrap().scores[0].points, 39);

        let mut both = points(&game, &[0, 23, 8, 5]);
        both.leekha_shown_by_player_id = Some("p0".into());
        both.leekha_captured_by_player_id = Some("p1".into());
        both.leekha_ten_diamonds_shown_by_player_id = Some("p1".into());
        both.leekha_ten_diamonds_captured_by_player_id = Some("p1".into());
        assert_eq!(evaluate_round(&game, &both).unwrap().scores[1].points, 56);

        let mut impossible = points(&game, &[10, 13, 8, 5]);
        impossible.leekha_shown_by_player_id = Some("p0".into());
        impossible.leekha_captured_by_player_id = Some("p1".into());
        impossible.leekha_ten_diamonds_shown_by_player_id = Some("p2".into());
        impossible.leekha_ten_diamonds_captured_by_player_id = Some("p1".into());
        assert!(evaluate_round(&game, &impossible)
            .unwrap_err()
            .contains("base value of every open Leekha card"));
    }

    #[test]
    fn chinese_scores_cards_left_and_requires_one_winner() {
        let game = sample("chinese", "INDIVIDUAL", "LOSE_AT_OR_ABOVE", Some(100), None);
        let request = RoundRequest {
            scores: game
                .players
                .iter()
                .zip([0, 4, 9, 13])
                .map(|(player, cards_left)| ScoreInput {
                    player_id: Some(player.id.clone()),
                    cards_left: Some(cards_left),
                    ..default_score()
                })
                .collect(),
            ..RoundRequest::default()
        };
        assert_eq!(
            evaluate_round(&game, &request)
                .unwrap()
                .scores
                .iter()
                .map(|score| score.points)
                .collect::<Vec<_>>(),
            vec![0, 4, 18, 40]
        );
        let no_winner = RoundRequest {
            scores: game
                .players
                .iter()
                .map(|player| ScoreInput {
                    player_id: Some(player.id.clone()),
                    cards_left: Some(2),
                    ..default_score()
                })
                .collect(),
            ..RoundRequest::default()
        };
        assert_eq!(
            evaluate_round(&game, &no_winner).unwrap_err(),
            "Choose exactly one player who finished their cards first"
        );
    }

    #[test]
    fn quatorze_scores_winner_jokers_and_all_at_once_finish() {
        let game = sample(
            "quatorze",
            "INDIVIDUAL",
            "LOSE_AT_OR_ABOVE",
            Some(201),
            None,
        );
        let request = RoundRequest {
            quatorze_winner_player_id: Some("p0".into()),
            scores: game
                .players
                .iter()
                .zip([(0, 0), (21, 0), (0, 1), (0, 1)])
                .map(|(player, (card_points, jokers))| ScoreInput {
                    player_id: Some(player.id.clone()),
                    card_points: Some(card_points),
                    jokers: Some(jokers),
                    ..default_score()
                })
                .collect(),
            ..RoundRequest::default()
        };
        assert_eq!(
            evaluate_round(&game, &request)
                .unwrap()
                .scores
                .iter()
                .map(|score| score.points)
                .collect::<Vec<_>>(),
            vec![-30, 21, 25, 25]
        );
        let mut all_at_once = request;
        all_at_once.quatorze_all_at_once = Some(true);
        assert_eq!(
            evaluate_round(&game, &all_at_once)
                .unwrap()
                .scores
                .iter()
                .map(|score| score.points)
                .collect::<Vec<_>>(),
            vec![-60, 42, 50, 50]
        );
    }

    #[test]
    fn tarneeb_contract_scores_both_teams() {
        let game = sample("tarneeb", "TEAM", "WIN_AT_OR_ABOVE", Some(31), None);
        let request = RoundRequest {
            bidder_team: Some(1),
            bid: Some(8),
            tricks: Some(6),
            scores: vec![
                ScoreInput {
                    team_number: Some(1),
                    ..default_score()
                },
                ScoreInput {
                    team_number: Some(2),
                    ..default_score()
                },
            ],
            ..RoundRequest::default()
        };
        assert_eq!(
            evaluate_round(&game, &request)
                .unwrap()
                .scores
                .iter()
                .map(|score| score.points)
                .collect::<Vec<_>>(),
            vec![-8, 7]
        );
    }

    #[test]
    fn four_hundred_uses_score_based_values_and_partner_end_rule() {
        let mut game = sample(
            "400",
            "INDIVIDUAL",
            "PLAYER_WIN_WITH_PARTNER_POSITIVE",
            Some(41),
            None,
        );
        game.players[0].total_score = 36;
        let bids = [5, 3, 2, 2];
        let request = RoundRequest {
            scores: game
                .players
                .iter()
                .zip(bids)
                .map(|(player, bid)| ScoreInput {
                    player_id: Some(player.id.clone()),
                    bid: Some(bid),
                    made: Some(true),
                    ..default_score()
                })
                .collect(),
            ..RoundRequest::default()
        };
        let result = evaluate_round(&game, &request).unwrap();
        assert_eq!(result.scores[0].points, 5);
        assert!(result.finished);
    }

    #[test]
    fn four_hundred_outcome_uses_partner_rule_instead_of_individual_leader() {
        let mut game = sample(
            "400",
            "INDIVIDUAL",
            "PLAYER_WIN_WITH_PARTNER_POSITIVE",
            Some(41),
            None,
        );
        game.round_count = 3;
        game.players[0].total_score = 50;
        game.players[2].total_score = -5;
        game.players[1].total_score = 41;
        game.players[3].total_score = 1;
        let outcome = determine_outcome(&game, &RoundRequest::default()).unwrap();
        assert_eq!(outcome.reason, "PLAYER_WITH_POSITIVE_PARTNER");
        assert_eq!(outcome.winner_player_ids, vec!["p1", "p3"]);
    }

    #[test]
    fn trex_basra_belote_and_hand_match_tables() {
        let trex = sample("trex", "INDIVIDUAL", "NONE", None, Some(20));
        let request = RoundRequest {
            rule_code: Some("KING".into()),
            scores: trex
                .players
                .iter()
                .enumerate()
                .map(|(index, player)| ScoreInput {
                    player_id: Some(player.id.clone()),
                    king_hearts: Some(i32::from(index == 0)),
                    ..default_score()
                })
                .collect(),
            ..RoundRequest::default()
        };
        assert_eq!(
            evaluate_round(&trex, &request).unwrap().scores[0].points,
            -75
        );

        let basra = sample("basra", "TEAM", "WIN_AT_OR_ABOVE", Some(101), None);
        let request = RoundRequest {
            scores: vec![
                ScoreInput {
                    team_number: Some(1),
                    card_points: Some(7),
                    basras: Some(1),
                    majority: Some(true),
                    ..default_score()
                },
                ScoreInput {
                    team_number: Some(2),
                    card_points: Some(6),
                    basras: Some(0),
                    majority: Some(false),
                    ..default_score()
                },
            ],
            ..RoundRequest::default()
        };
        assert_eq!(
            evaluate_round(&basra, &request)
                .unwrap()
                .scores
                .iter()
                .map(|score| score.points)
                .collect::<Vec<_>>(),
            vec![20, 6]
        );

        let belote = sample("belote", "TEAM", "WIN_AT_OR_ABOVE", Some(501), None);
        let request = RoundRequest {
            scores: vec![
                ScoreInput {
                    team_number: Some(1),
                    card_points: Some(100),
                    bonus_points: Some(20),
                    ..default_score()
                },
                ScoreInput {
                    team_number: Some(2),
                    card_points: Some(62),
                    bonus_points: Some(0),
                    ..default_score()
                },
            ],
            ..RoundRequest::default()
        };
        assert_eq!(
            evaluate_round(&belote, &request).unwrap().scores[0].points,
            120
        );

        let hand = sample("hand", "INDIVIDUAL", "NONE", None, Some(1));
        let request = RoundRequest {
            scores: hand
                .players
                .iter()
                .enumerate()
                .map(|(index, player)| ScoreInput {
                    player_id: Some(player.id.clone()),
                    card_points: Some(index as i32 * 10),
                    ..default_score()
                })
                .collect(),
            ..RoundRequest::default()
        };
        assert!(evaluate_round(&hand, &request).unwrap().finished);
    }

    fn sample(
        code: &str,
        mode: &str,
        threshold: &str,
        target: Option<i32>,
        limit: Option<i32>,
    ) -> GameState {
        GameState {
            preset_code: code.into(),
            team_mode: matches!(
                code,
                "leekha-team" | "leekha-team-shown" | "400" | "tarneeb" | "basra" | "belote"
            ),
            score_direction: if matches!(
                code,
                "leekha-solo"
                    | "leekha-solo-shown"
                    | "leekha-team"
                    | "leekha-team-shown"
                    | "hand"
                    | "chinese"
                    | "quatorze"
            ) {
                "LOW_WINS".into()
            } else {
                "HIGH_WINS".into()
            },
            score_mode: mode.into(),
            target_score: target,
            threshold_rule: threshold.into(),
            round_limit: limit,
            round_count: 0,
            players: (0..4)
                .map(|index| PlayerState {
                    id: format!("p{index}"),
                    name: format!("P{}", index + 1),
                    team_number: Some(index % 2 + 1),
                    total_score: 0,
                })
                .collect(),
            prior_round_rules: vec![],
        }
    }
    fn points(game: &GameState, values: &[i32]) -> RoundRequest {
        RoundRequest {
            scores: game
                .players
                .iter()
                .zip(values)
                .map(|(player, points)| ScoreInput {
                    player_id: Some(player.id.clone()),
                    points: Some(*points),
                    ..default_score()
                })
                .collect(),
            ..RoundRequest::default()
        }
    }
    fn default_score() -> ScoreInput {
        ScoreInput::default()
    }
}
