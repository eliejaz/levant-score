use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::{domain::*, dto::*, entity::*, presets, rules};
use crate::{
    api::error::{bad, require, ApiError, ApiResult},
    features::auth::AuthUser,
    AppState,
};

pub async fn list(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
) -> ApiResult<Json<Vec<GameSummary>>> {
    // Aggregate the dashboard in PostgreSQL instead of loading every historic
    // round and score for every game. These three queries execute concurrently,
    // so dashboard latency is a single database network round trip.
    let (games, player_totals, round_counts) = tokio::try_join!(
        sqlx::query_as::<_, DbGame>("SELECT id,title,preset_code,preset_name,team_mode,score_direction,target_score,threshold_rule,score_mode,round_limit,status,created_at,updated_at FROM games WHERE owner_id=$1 ORDER BY updated_at DESC")
            .bind(owner).fetch_all(&state.pool),
        sqlx::query_as::<_, (Uuid, Uuid, String, Option<i32>, i32, i64)>("SELECT p.id,p.game_id,p.name,p.team_number,p.seat_order,COALESCE(SUM(se.points),0)::bigint FROM players p JOIN games g ON g.id=p.game_id LEFT JOIN score_entries se ON se.player_id=p.id WHERE g.owner_id=$1 GROUP BY p.id,p.game_id,p.name,p.team_number,p.seat_order,g.updated_at ORDER BY g.updated_at DESC,p.seat_order ASC")
            .bind(owner).fetch_all(&state.pool),
        sqlx::query_as::<_, (Uuid, i64)>("SELECT r.game_id,COUNT(*)::bigint FROM rounds r JOIN games g ON g.id=r.game_id WHERE g.owner_id=$1 GROUP BY r.game_id")
            .bind(owner).fetch_all(&state.pool),
    )?;

    let mut players_by_game: HashMap<Uuid, Vec<DbPlayer>> = HashMap::new();
    let mut totals_by_player = HashMap::new();
    for (id, game_id, name, team_number, seat_order, total) in player_totals {
        let player = DbPlayer {
            id,
            game_id,
            name,
            team_number,
            seat_order,
        };
        players_by_game
            .entry(player.game_id)
            .or_default()
            .push(player);
        totals_by_player.insert(id, total as i32);
    }
    let round_counts: HashMap<Uuid, usize> = round_counts
        .into_iter()
        .map(|(id, count)| (id, count as usize))
        .collect();
    let result = games
        .into_iter()
        .map(|game| {
            let id = game.id;
            summary_from_totals(
                &game,
                &players_by_game.remove(&id).unwrap_or_default(),
                &totals_by_player,
                round_counts.get(&id).copied().unwrap_or_default(),
            )
        })
        .collect();
    Ok(Json(result))
}

pub async fn get(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<GameView>> {
    Ok(Json(view(&load_game(&state.pool, id, owner).await?)))
}

pub async fn delete(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let result = sqlx::query("DELETE FROM games WHERE id=$1 AND owner_id=$2")
        .bind(id)
        .bind(owner)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Game not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Json(request): Json<CreateGameRequest>,
) -> ApiResult<Json<GameView>> {
    validate_create(&request)?;
    let preset = presets::get(&request.preset_code);
    let custom = request.preset_code == "custom";
    let team_mode = if custom {
        request.team_mode.unwrap_or(preset.team_mode)
    } else {
        preset.team_mode
    };
    if !preset
        .allowed_player_counts
        .contains(&(request.players.len() as i32))
    {
        return Err(bad(player_count_message(&preset)));
    }
    if team_mode && request.players.iter().any(|p| p.team_number.is_none()) {
        return Err(bad("Every player needs a team"));
    }
    if !custom && team_mode {
        validate_preset_teams(&request.players)?;
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    let preset_name = if custom {
        request
            .custom_preset_name
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(str::trim)
            .unwrap_or(preset.name)
    } else {
        preset.name
    };
    let direction = if custom {
        request.score_direction.unwrap_or(preset.direction)
    } else {
        preset.direction
    };
    let threshold = if custom {
        request.threshold_rule.unwrap_or(preset.threshold_rule)
    } else {
        preset.threshold_rule
    };
    let mode = if custom {
        request.score_mode.unwrap_or(preset.score_mode)
    } else {
        preset.score_mode
    };
    let round_limit = if custom {
        request.round_limit.or(preset.round_limit)
    } else {
        preset.round_limit
    };
    let target_score = request.target_score.or(preset.default_target);
    let players: Vec<DbPlayer> = request
        .players
        .iter()
        .enumerate()
        .map(|(seat, player)| DbPlayer {
            id: Uuid::new_v4(),
            game_id: id,
            name: player.name.trim().to_owned(),
            team_number: if team_mode { player.team_number } else { None },
            seat_order: seat as i32,
        })
        .collect();
    let mut tx = state.pool.begin().await?;
    sqlx::query("INSERT INTO games (id,owner_id,title,preset_code,preset_name,team_mode,score_direction,target_score,threshold_rule,score_mode,round_limit,status,created_at,updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'ACTIVE',$12,$12)")
        .bind(id).bind(owner).bind(request.title.trim()).bind(preset.code).bind(preset_name).bind(team_mode)
        .bind(direction.as_str()).bind(target_score).bind(threshold.as_str())
        .bind(mode.as_str()).bind(round_limit).bind(now).execute(&mut *tx).await?;
    insert_players(&mut tx, &players).await?;
    tx.commit().await?;
    Ok(Json(view(&GameData {
        game: DbGame {
            id,
            title: request.title.trim().to_owned(),
            preset_code: preset.code.to_owned(),
            preset_name: preset_name.to_owned(),
            team_mode,
            score_direction: direction.as_str().to_owned(),
            target_score,
            threshold_rule: threshold.as_str().to_owned(),
            score_mode: mode.as_str().to_owned(),
            round_limit,
            status: Status::Active.as_str().to_owned(),
            created_at: now,
            updated_at: now,
        },
        players,
        rounds: Vec::new(),
        scores: Vec::new(),
    })))
}

pub async fn rematch(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Path(source_id): Path<Uuid>,
) -> ApiResult<Json<GameView>> {
    let source = load_game(&state.pool, source_id, owner).await?;
    let id = Uuid::new_v4();
    let now = Utc::now();
    let players: Vec<DbPlayer> = source
        .players
        .iter()
        .map(|player| DbPlayer {
            id: Uuid::new_v4(),
            game_id: id,
            name: player.name.clone(),
            team_number: player.team_number,
            seat_order: player.seat_order,
        })
        .collect();
    let mut tx = state.pool.begin().await?;
    sqlx::query("INSERT INTO games (id,owner_id,title,preset_code,preset_name,team_mode,score_direction,target_score,threshold_rule,score_mode,round_limit,status,created_at,updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'ACTIVE',$12,$12)")
        .bind(id).bind(owner).bind(&source.game.title).bind(&source.game.preset_code).bind(&source.game.preset_name)
        .bind(source.game.team_mode).bind(&source.game.score_direction).bind(source.game.target_score)
        .bind(&source.game.threshold_rule).bind(&source.game.score_mode).bind(source.game.round_limit).bind(now)
        .execute(&mut *tx).await?;
    insert_players(&mut tx, &players).await?;
    tx.commit().await?;
    let mut game = source.game;
    game.id = id;
    game.status = Status::Active.as_str().to_owned();
    game.created_at = now;
    game.updated_at = now;
    Ok(Json(view(&GameData {
        game,
        players,
        rounds: Vec::new(),
        scores: Vec::new(),
    })))
}

pub async fn add_round(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<AddRoundRequest>,
) -> ApiResult<Json<GameView>> {
    require(!request.scores.is_empty(), "scores must not be empty")?;
    if request
        .note
        .as_ref()
        .is_some_and(|n| n.chars().count() > 240)
    {
        return Err(bad("note must be at most 240 characters"));
    }
    if request
        .rule_code
        .as_ref()
        .is_some_and(|n| n.chars().count() > 40)
    {
        return Err(bad("ruleCode must be at most 40 characters"));
    }
    let mut game = load_game(&state.pool, id, owner).await?;
    if game.game.game_status() == Status::Finished {
        return Err(ApiError::Conflict(
            "Reopen the game before adding a round".into(),
        ));
    }
    let players_by_id: HashMap<Uuid, DbPlayer> =
        game.players.iter().cloned().map(|p| (p.id, p)).collect();
    let mut used = HashSet::new();
    let mut entries = Vec::new();
    let mut points = Vec::new();
    for input in &request.scores {
        let player = if game.game.mode() == ScoreMode::Team && input.team_number.is_some() {
            game.players
                .iter()
                .find(|p| p.team_number == input.team_number)
                .cloned()
                .ok_or_else(|| bad("Unknown team"))?
        } else {
            input
                .player_id
                .and_then(|pid| players_by_id.get(&pid).cloned())
                .ok_or_else(|| bad("Unknown player"))?
        };
        if !used.insert(player.id) {
            return Err(bad("Duplicate score"));
        }
        let calculated = rules::calculate(&game, input, &request, &player)?;
        entries.push((player, calculated));
        points.push(calculated);
    }
    let expected = if game.game.mode() == ScoreMode::Team {
        game.players
            .iter()
            .map(|p| p.team_number)
            .collect::<HashSet<_>>()
            .len()
    } else {
        game.players.len()
    };
    require(
        used.len() == expected,
        "Add one score for every player or team",
    )?;
    rules::validate(&game, &request, &points)?;

    let round_id = Uuid::new_v4();
    let now = Utc::now();
    let round_number = game.rounds.len() as i32 + 1;
    let note = request
        .note
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_owned);
    let proposed_round = DbRound {
        id: round_id,
        round_number,
        rule_code: request.rule_code.clone(),
        note: note.clone(),
        created_at: now,
    };
    game.rounds.insert(0, proposed_round);
    for (player, value) in &entries {
        game.scores.push(DbScore {
            round_id,
            player_id: player.id,
            points: *value,
        });
    }
    let slam = game.game.preset_code == "400" && request.scores.iter().any(|s| s.bid == Some(13));
    let new_status = if slam || game_ended(&game) {
        Status::Finished
    } else {
        Status::Active
    };
    let player_ids: Vec<Uuid> = entries.iter().map(|(player, _)| player.id).collect();
    let score_values: Vec<i32> = entries.iter().map(|(_, value)| *value).collect();

    sqlx::query("WITH inserted_round AS (INSERT INTO rounds (id,game_id,round_number,rule_code,note,created_at) VALUES ($1,$2,$3,$4,$5,$6) RETURNING id), inserted_scores AS (INSERT INTO score_entries (round_id,player_id,points) SELECT inserted_round.id,scores.player_id,scores.points FROM inserted_round CROSS JOIN UNNEST($7::uuid[],$8::int[]) AS scores(player_id,points) RETURNING 1) UPDATE games SET status=$9,updated_at=$6 WHERE id=$2 AND owner_id=$10 AND EXISTS (SELECT 1 FROM inserted_scores)")
        .bind(round_id)
        .bind(id)
        .bind(round_number)
        .bind(&request.rule_code)
        .bind(note)
        .bind(now)
        .bind(&player_ids)
        .bind(&score_values)
        .bind(new_status.as_str())
        .bind(owner)
        .execute(&state.pool)
        .await?;
    game.game.status = new_status.as_str().to_owned();
    game.game.updated_at = now;
    Ok(Json(view(&game)))
}

pub async fn undo(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<GameView>> {
    let mut game = load_game(&state.pool, id, owner).await?;
    let latest = game
        .rounds
        .first()
        .map(|round| round.id)
        .ok_or_else(|| ApiError::Conflict("There is no round to undo".into()))?;
    let now = Utc::now();
    sqlx::query("WITH deleted AS (DELETE FROM rounds WHERE id=$1 RETURNING id) UPDATE games SET updated_at=$2 WHERE id=$3 AND owner_id=$4 AND EXISTS (SELECT 1 FROM deleted)")
        .bind(latest)
        .bind(now)
        .bind(id)
        .bind(owner)
        .execute(&state.pool)
        .await?;
    game.rounds.retain(|round| round.id != latest);
    game.scores.retain(|score| score.round_id != latest);
    game.game.updated_at = now;
    Ok(Json(view(&game)))
}

pub async fn set_status(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<StatusRequest>,
) -> ApiResult<Json<GameView>> {
    let game = sqlx::query_as::<_, DbGame>("UPDATE games SET status=$1,updated_at=now() WHERE id=$2 AND owner_id=$3 RETURNING id,title,preset_code,preset_name,team_mode,score_direction,target_score,threshold_rule,score_mode,round_limit,status,created_at,updated_at")
        .bind(request.status.as_str()).bind(id).bind(owner).fetch_optional(&state.pool).await?
        .ok_or_else(|| ApiError::NotFound("Game not found".into()))?;
    let (players, rounds, scores) = load_related(&state.pool, id).await?;
    Ok(Json(view(&GameData {
        game,
        players,
        rounds,
        scores,
    })))
}

async fn load_game(pool: &PgPool, id: Uuid, owner: Uuid) -> ApiResult<GameData> {
    let game = sqlx::query_as::<_, DbGame>("SELECT id,title,preset_code,preset_name,team_mode,score_direction,target_score,threshold_rule,score_mode,round_limit,status,created_at,updated_at FROM games WHERE id=$1 AND owner_id=$2")
        .bind(id).bind(owner).fetch_optional(pool).await?.ok_or_else(|| ApiError::NotFound("Game not found".into()))?;
    let (players, rounds, scores) = load_related(pool, id).await?;
    Ok(GameData {
        game,
        players,
        rounds,
        scores,
    })
}

async fn load_related(
    pool: &PgPool,
    id: Uuid,
) -> Result<(Vec<DbPlayer>, Vec<DbRound>, Vec<DbScore>), sqlx::Error> {
    tokio::try_join!(
        sqlx::query_as::<_, DbPlayer>("SELECT id,game_id,name,team_number,seat_order FROM players WHERE game_id=$1 ORDER BY seat_order ASC")
            .bind(id).fetch_all(pool),
        sqlx::query_as::<_, DbRound>("SELECT id,round_number,rule_code,note,created_at FROM rounds WHERE game_id=$1 ORDER BY round_number DESC")
            .bind(id).fetch_all(pool),
        sqlx::query_as::<_, DbScore>("SELECT se.round_id,se.player_id,se.points FROM score_entries se JOIN rounds r ON r.id=se.round_id JOIN players p ON p.id=se.player_id WHERE r.game_id=$1 ORDER BY r.round_number DESC,p.seat_order ASC")
            .bind(id).fetch_all(pool),
    )
}

async fn insert_players(
    tx: &mut Transaction<'_, Postgres>,
    players: &[DbPlayer],
) -> Result<(), sqlx::Error> {
    let ids: Vec<Uuid> = players.iter().map(|player| player.id).collect();
    let game_ids: Vec<Uuid> = players.iter().map(|player| player.game_id).collect();
    let names: Vec<String> = players.iter().map(|player| player.name.clone()).collect();
    let teams: Vec<Option<i32>> = players.iter().map(|player| player.team_number).collect();
    let seats: Vec<i32> = players.iter().map(|player| player.seat_order).collect();
    sqlx::query("INSERT INTO players (id,game_id,name,team_number,seat_order) SELECT * FROM UNNEST($1::uuid[],$2::uuid[],$3::text[],$4::int[],$5::int[])")
        .bind(&ids)
        .bind(&game_ids)
        .bind(&names)
        .bind(&teams)
        .bind(&seats)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

fn validate_create(request: &CreateGameRequest) -> ApiResult<()> {
    require(!request.title.trim().is_empty(), "title must not be blank")?;
    require(
        request.title.chars().count() <= 120,
        "title must be at most 120 characters",
    )?;
    require(
        !request.preset_code.trim().is_empty(),
        "presetCode must not be blank",
    )?;
    require(!request.players.is_empty(), "players must not be empty")?;
    for player in &request.players {
        require(
            !player.name.trim().is_empty(),
            "Player name must not be blank",
        )?;
        require(
            player.name.chars().count() <= 60,
            "Player name must be at most 60 characters",
        )?;
    }
    Ok(())
}
fn validate_preset_teams(players: &[PlayerInput]) -> ApiResult<()> {
    require(
        players.iter().all(|p| matches!(p.team_number, Some(1 | 2))),
        "This game requires exactly two teams",
    )?;
    require(
        players.iter().filter(|p| p.team_number == Some(1)).count()
            == players.iter().filter(|p| p.team_number == Some(2)).count(),
        "Both teams must have the same number of players",
    )
}
fn player_count_message(preset: &Preset) -> String {
    if preset.allowed_player_counts.len() == 1 {
        format!(
            "{} requires exactly {} players",
            preset.name, preset.min_players
        )
    } else {
        format!(
            "Choose {} players for {}",
            preset
                .allowed_player_counts
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" or "),
            preset.name
        )
    }
}

fn standings(data: &GameData) -> Vec<StandingView> {
    let mut player_totals: HashMap<Uuid, i32> = HashMap::new();
    for score in &data.scores {
        *player_totals.entry(score.player_id).or_default() += score.points;
    }
    standings_from_totals(&data.game, &data.players, &player_totals)
}

fn standings_from_totals(
    game: &DbGame,
    players: &[DbPlayer],
    player_totals: &HashMap<Uuid, i32>,
) -> Vec<StandingView> {
    let mut groups: Vec<(String, Vec<&DbPlayer>)> = Vec::new();
    for player in players {
        let key = if game.mode() == ScoreMode::Team {
            format!("team-{}", player.team_number.unwrap_or_default())
        } else {
            player.id.to_string()
        };
        if let Some((_, members)) = groups.iter_mut().find(|(k, _)| *k == key) {
            members.push(player);
        } else {
            groups.push((key, vec![player]));
        }
    }
    let mut totals: Vec<StandingView> = groups
        .into_iter()
        .map(|(key, members)| {
            let ids: Vec<Uuid> = members.iter().map(|p| p.id).collect();
            let total = ids
                .iter()
                .map(|id| player_totals.get(id).copied().unwrap_or_default())
                .sum();
            let name = if game.mode() == ScoreMode::Team {
                members
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" & ")
            } else {
                members[0].name.clone()
            };
            StandingView {
                key,
                name,
                player_ids: ids,
                team_number: members[0].team_number,
                total,
                position: 0,
            }
        })
        .collect();
    match game.direction() {
        ScoreDirection::HighWins => totals.sort_by_key(|a| std::cmp::Reverse(a.total)),
        ScoreDirection::LowWins => totals.sort_by_key(|a| a.total),
    }
    for (index, item) in totals.iter_mut().enumerate() {
        item.position = index as i32 + 1;
    }
    totals
}

fn game_ended(data: &GameData) -> bool {
    if data
        .game
        .round_limit
        .is_some_and(|limit| data.rounds.len() as i32 >= limit)
    {
        return true;
    }
    let Some(target) = data.game.target_score else {
        return false;
    };
    if data.game.threshold() == ThresholdRule::None || data.rounds.is_empty() {
        return false;
    }
    let current = standings(data);
    if data.game.threshold() == ThresholdRule::PlayerWinWithPartnerPositive {
        let mut threshold = target;
        loop {
            let teams: HashSet<i32> = current
                .iter()
                .filter(|s| s.total >= threshold)
                .filter_map(|s| s.team_number.map(|team| (s, team)))
                .filter(|(s, team)| {
                    current
                        .iter()
                        .filter(|p| p.team_number == Some(*team) && p.key != s.key)
                        .all(|p| p.total > 0)
                })
                .map(|(_, team)| team)
                .collect();
            if teams.len() == 1 {
                return true;
            }
            if teams.len() < 2 {
                return false;
            }
            threshold += 10;
        }
    }
    current.iter().any(|s| s.total >= target)
}

fn summary_from_totals(
    game: &DbGame,
    players: &[DbPlayer],
    player_totals: &HashMap<Uuid, i32>,
    round_count: usize,
) -> GameSummary {
    GameSummary {
        id: game.id,
        title: game.title.clone(),
        preset_code: game.preset_code.clone(),
        preset_name: game.preset_name.clone(),
        team_mode: game.team_mode,
        score_direction: game.direction(),
        target_score: game.target_score,
        threshold_rule: game.threshold(),
        status: game.game_status(),
        score_mode: game.mode(),
        round_limit: game.round_limit,
        round_count,
        standings: standings_from_totals(game, players, player_totals),
        updated_at: game.updated_at,
    }
}

fn view(data: &GameData) -> GameView {
    let player_views = data
        .players
        .iter()
        .map(|p| PlayerView {
            id: p.id,
            name: p.name.clone(),
            team_number: p.team_number,
            seat_order: p.seat_order,
        })
        .collect();
    let round_views = data
        .rounds
        .iter()
        .map(|round| {
            let scores = data
                .scores
                .iter()
                .filter(|s| s.round_id == round.id)
                .filter_map(|entry| {
                    let player = data.players.iter().find(|p| p.id == entry.player_id)?;
                    if data.game.mode() == ScoreMode::Individual {
                        Some(RoundScoreView {
                            key: player.id.to_string(),
                            name: player.name.clone(),
                            points: entry.points,
                        })
                    } else {
                        let team = player.team_number.unwrap_or_default();
                        Some(RoundScoreView {
                            key: format!("team-{team}"),
                            name: data
                                .players
                                .iter()
                                .filter(|p| p.team_number == Some(team))
                                .map(|p| p.name.as_str())
                                .collect::<Vec<_>>()
                                .join(" & "),
                            points: entry.points,
                        })
                    }
                })
                .collect();
            RoundView {
                id: round.id,
                round_number: round.round_number,
                rule_code: round.rule_code.clone(),
                note: round.note.clone(),
                created_at: round.created_at,
                scores,
            }
        })
        .collect();
    GameView {
        id: data.game.id,
        title: data.game.title.clone(),
        preset_code: data.game.preset_code.clone(),
        preset_name: data.game.preset_name.clone(),
        team_mode: data.game.team_mode,
        score_direction: data.game.direction(),
        target_score: data.game.target_score,
        threshold_rule: data.game.threshold(),
        status: data.game.game_status(),
        score_mode: data.game.mode(),
        round_limit: data.game.round_limit,
        players: player_views,
        standings: standings(data),
        rounds: round_views,
        created_at: data.game.created_at,
        updated_at: data.game.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_player_count_message_matches_existing_api() {
        assert_eq!(
            player_count_message(&presets::get("tarneeb")),
            "Tarneeb requires exactly 4 players"
        );
    }
    #[test]
    fn rejects_unbalanced_teams() {
        let players = vec![
            PlayerInput {
                name: "1".into(),
                team_number: Some(1),
            },
            PlayerInput {
                name: "2".into(),
                team_number: Some(1),
            },
            PlayerInput {
                name: "3".into(),
                team_number: Some(1),
            },
            PlayerInput {
                name: "4".into(),
                team_number: Some(2),
            },
        ];
        assert_eq!(
            validate_preset_teams(&players).unwrap_err().to_string(),
            "Both teams must have the same number of players"
        );
    }

    #[test]
    fn team_standings_aggregate_partners_but_individual_mode_does_not() {
        let mut data = game_data(
            "tarneeb",
            ScoreMode::Team,
            ThresholdRule::WinAtOrAbove,
            Some(31),
            None,
        );
        add_players_and_scores(&mut data, &[(1, 10), (1, 0), (2, 6), (2, 0)]);
        let result = standings(&data);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].total, 10);
        assert_eq!(result[0].name, "P1 & P2");
        data.game.score_mode = "INDIVIDUAL".into();
        assert_eq!(standings(&data).len(), 4);
    }

    #[test]
    fn leekha_and_round_limit_end_conditions_match_existing_behavior() {
        let mut leekha = game_data(
            "leekha-team",
            ScoreMode::Individual,
            ThresholdRule::LoseAtOrAbove,
            Some(101),
            None,
        );
        add_players_and_scores(&mut leekha, &[(1, 101), (1, 0), (2, 0), (2, 0)]);
        assert!(game_ended(&leekha));
        let mut hand = game_data(
            "hand",
            ScoreMode::Individual,
            ThresholdRule::None,
            None,
            Some(1),
        );
        add_players_and_scores(&mut hand, &[(0, 0), (0, 10)]);
        assert!(game_ended(&hand));
    }

    #[test]
    fn four_hundred_requires_a_positive_partner() {
        let mut data = game_data(
            "400",
            ScoreMode::Individual,
            ThresholdRule::PlayerWinWithPartnerPositive,
            Some(41),
            None,
        );
        add_players_and_scores(&mut data, &[(1, 41), (1, 0), (2, 10), (2, 10)]);
        assert!(!game_ended(&data));
        let partner = data.players[1].id;
        data.scores
            .iter_mut()
            .find(|s| s.player_id == partner)
            .unwrap()
            .points = 1;
        assert!(game_ended(&data));
    }

    fn game_data(
        code: &str,
        mode: ScoreMode,
        threshold: ThresholdRule,
        target: Option<i32>,
        round_limit: Option<i32>,
    ) -> GameData {
        let now = Utc::now();
        let id = Uuid::new_v4();
        GameData {
            game: DbGame {
                id,
                title: "Game".into(),
                preset_code: code.into(),
                preset_name: code.into(),
                team_mode: true,
                score_direction: "HIGH_WINS".into(),
                target_score: target,
                threshold_rule: threshold.as_str().into(),
                score_mode: mode.as_str().into(),
                round_limit,
                status: "ACTIVE".into(),
                created_at: now,
                updated_at: now,
            },
            players: vec![],
            rounds: vec![],
            scores: vec![],
        }
    }

    fn add_players_and_scores(data: &mut GameData, values: &[(i32, i32)]) {
        let round = DbRound {
            id: Uuid::new_v4(),
            round_number: 1,
            rule_code: None,
            note: None,
            created_at: Utc::now(),
        };
        for (seat, (team, total)) in values.iter().enumerate() {
            let player = DbPlayer {
                id: Uuid::new_v4(),
                game_id: data.game.id,
                name: format!("P{}", seat + 1),
                team_number: Some(*team),
                seat_order: seat as i32,
            };
            data.scores.push(DbScore {
                round_id: round.id,
                player_id: player.id,
                points: *total,
            });
            data.players.push(player);
        }
        data.rounds.push(round);
    }
}
