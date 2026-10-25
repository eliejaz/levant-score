use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sqlx::PgPool;
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
    Query(query): Query<ListGamesQuery>,
) -> ApiResult<Json<GamePage>> {
    let status = query.status.unwrap_or(Status::Active);
    let page = query.page.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(6).clamp(1, 24);
    let offset = (page - 1) * size;
    let (games, stats): (Vec<DbGame>, (i64, i64, i64)) = tokio::try_join!(
        sqlx::query_as::<_, DbGame>("SELECT id,title,preset_code,preset_name,team_mode,score_direction,target_score,threshold_rule,score_mode,round_limit,round_count,status,created_at,updated_at FROM games WHERE owner_id=$1 AND status=$2 ORDER BY updated_at DESC LIMIT $3 OFFSET $4")
            .bind(owner).bind(status.as_str()).bind(size).bind(offset).fetch_all(&state.pool),
        sqlx::query_as("SELECT COALESCE(SUM(round_count),0)::bigint,COUNT(*) FILTER (WHERE status='ACTIVE')::bigint,COUNT(*) FILTER (WHERE status='FINISHED')::bigint FROM games WHERE owner_id=$1")
            .bind(owner).fetch_one(&state.pool),
    )?;
    let game_ids: Vec<Uuid> = games.iter().map(|game| game.id).collect();
    let players = if game_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, DbPlayer>("SELECT id,game_id,name,team_number,seat_order,total_score FROM players WHERE game_id=ANY($1) ORDER BY game_id,seat_order")
            .bind(&game_ids).fetch_all(&state.pool).await?
    };
    let mut players_by_game: HashMap<Uuid, Vec<DbPlayer>> = HashMap::new();
    for player in players {
        players_by_game
            .entry(player.game_id)
            .or_default()
            .push(player);
    }
    let items = games
        .into_iter()
        .map(|game| {
            let id = game.id;
            summary(&game, &players_by_game.remove(&id).unwrap_or_default())
        })
        .collect();
    let total_items = if status == Status::Active {
        stats.1
    } else {
        stats.2
    };
    Ok(Json(GamePage {
        items,
        page,
        size,
        total_items,
        total_pages: (total_items + size - 1) / size,
        stats: GameStats {
            all_time_rounds: stats.0,
            active_games: stats.1,
            finished_games: stats.2,
        },
    }))
}

pub async fn get(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<GameView>> {
    Ok(Json(view(&load_game(&state.pool, id, owner, 20).await?)))
}

pub async fn list_rounds(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<RoundPageQuery>,
) -> ApiResult<Json<RoundPage>> {
    let limit = query.limit.unwrap_or(20).clamp(1, 50);
    let before = query.before_round.unwrap_or(i32::MAX);
    let (game, players, (mut rounds, scores)) = tokio::try_join!(
        fetch_game(&state.pool, id, owner),
        load_players(&state.pool, id),
        load_rounds(&state.pool, id, before, limit + 1),
    )?;
    let game = game.ok_or_else(|| ApiError::NotFound("Game not found".into()))?;
    let has_more = rounds.len() as i64 > limit;
    rounds.truncate(limit as usize);
    let data = GameData {
        game,
        players,
        rounds,
        scores,
    };
    Ok(Json(RoundPage {
        items: round_views(&data),
        has_more,
    }))
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
            total_score: 0,
        })
        .collect();
    let game = DbGame {
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
        round_count: 0,
        status: Status::Active.as_str().to_owned(),
        created_at: now,
        updated_at: now,
    };
    insert_game(&state.pool, owner, &game, &players).await?;
    Ok(Json(view(&GameData {
        game,
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
    let source = load_game_state(&state.pool, source_id, owner, 0).await?;
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
            total_score: 0,
        })
        .collect();
    let mut game = source.game;
    game.id = id;
    game.round_count = 0;
    game.status = Status::Active.as_str().to_owned();
    game.created_at = now;
    game.updated_at = now;
    insert_game(&state.pool, owner, &game, &players).await?;
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
) -> ApiResult<Json<GameMutation>> {
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
    let mut game = load_game_state(&state.pool, id, owner, 20).await?;
    if game.game.game_status() == Status::Finished {
        return Err(ApiError::Conflict(
            "Reopen the game before adding a round".into(),
        ));
    }
    let rules::Evaluation { entries, finished } = rules::evaluate(&game, &request)?;

    let round_id = Uuid::new_v4();
    let now = Utc::now();
    let round_number = game.game.round_count + 1;
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
        input_data: Some(
            serde_json::to_value(&request)
                .map_err(|error| ApiError::Internal(error.to_string()))?,
        ),
        created_at: now,
    };
    game.rounds.insert(0, proposed_round.clone());
    for (player, value) in &entries {
        if let Some(current) = game.players.iter_mut().find(|item| item.id == player.id) {
            current.total_score += *value;
        }
        game.scores.push(DbScore {
            round_id,
            player_id: player.id,
            points: *value,
        });
    }
    game.game.round_count += 1;
    let new_status = if finished {
        Status::Finished
    } else {
        Status::Active
    };
    let player_ids: Vec<Uuid> = entries.iter().map(|(player, _)| player.id).collect();
    let score_values: Vec<i32> = entries.iter().map(|(_, value)| *value).collect();

    let inserted_round_number = sqlx::query_scalar::<_, i32>("WITH bumped_game AS (UPDATE games SET round_count=round_count+1,status=$9,updated_at=$6 WHERE id=$2 AND owner_id=$10 AND status='ACTIVE' RETURNING round_count), inserted_round AS (INSERT INTO rounds (id,game_id,round_number,rule_code,note,input_data,created_at) SELECT $1,$2,bumped_game.round_count,$3,$4,$5,$6 FROM bumped_game RETURNING id,round_number), inserted_scores AS (INSERT INTO score_entries (round_id,player_id,points) SELECT inserted_round.id,scores.player_id,scores.points FROM inserted_round CROSS JOIN UNNEST($7::uuid[],$8::int[]) AS scores(player_id,points) RETURNING player_id,points), updated_players AS (UPDATE players p SET total_score=p.total_score+scores.points FROM inserted_scores scores WHERE p.id=scores.player_id AND p.game_id=$2 RETURNING p.id) SELECT round_number FROM inserted_round")
        .bind(round_id)
        .bind(id)
        .bind(&request.rule_code)
        .bind(note)
        .bind(&proposed_round.input_data)
        .bind(now)
        .bind(&player_ids)
        .bind(&score_values)
        .bind(new_status.as_str())
        .bind(owner)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::Conflict("The game changed; refresh and try again".into()))?;
    game.rounds[0].round_number = inserted_round_number;
    game.game.round_count = inserted_round_number;
    game.game.status = new_status.as_str().to_owned();
    game.game.updated_at = now;
    Ok(Json(GameMutation {
        status: new_status,
        standings: standings(&game),
        round_count: game.game.round_count,
        updated_at: now,
        round: round_views(&game).into_iter().next(),
        removed_round_id: None,
    }))
}

pub async fn update_round(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Path((id, round_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<AddRoundRequest>,
) -> ApiResult<Json<GameView>> {
    require(!request.scores.is_empty(), "scores must not be empty")?;
    if request
        .note
        .as_ref()
        .is_some_and(|note| note.chars().count() > 240)
    {
        return Err(bad("note must be at most 240 characters"));
    }
    if request
        .rule_code
        .as_ref()
        .is_some_and(|code| code.chars().count() > 40)
    {
        return Err(bad("ruleCode must be at most 40 characters"));
    }

    let mut source = load_game_state(&state.pool, id, owner, i64::from(i32::MAX)).await?;
    if !source.rounds.iter().any(|round| round.id == round_id) {
        return Err(ApiError::NotFound("Round not found".into()));
    }
    let previous_updated_at = source.game.updated_at;
    let mut original_rounds = source.rounds.clone();
    original_rounds.sort_by_key(|round| round.round_number);

    for player in &mut source.players {
        player.total_score = 0;
    }
    source.game.round_count = 0;
    source.game.status = Status::Active.as_str().to_owned();
    source.rounds.clear();
    source.scores.clear();

    let total_rounds = original_rounds.len();
    for (index, original) in original_rounds.into_iter().enumerate() {
        let round_request = if original.id == round_id {
            request.clone()
        } else {
            original
                .input_data
                .as_ref()
                .ok_or_else(|| ApiError::Conflict("An older round cannot be replayed. Undo it and enter it again before editing earlier rounds.".into()))
                .and_then(|value| serde_json::from_value(value.clone()).map_err(|_| ApiError::Conflict("A saved round is not editable. Undo it and enter it again before editing earlier rounds.".into())))?
        };
        let rules::Evaluation { entries, finished } = rules::evaluate(&source, &round_request)?;
        if finished && index + 1 < total_rounds {
            return Err(ApiError::Conflict(
                "This change would end the game before a later round. Undo the later rounds first."
                    .into(),
            ));
        }
        let stored_request = serde_json::to_value(&round_request)
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        let replayed_round = DbRound {
            id: original.id,
            round_number: original.round_number,
            rule_code: round_request.rule_code.clone(),
            note: round_request
                .note
                .as_deref()
                .map(str::trim)
                .filter(|note| !note.is_empty())
                .map(str::to_owned),
            input_data: Some(stored_request),
            created_at: original.created_at,
        };
        for (player, points) in entries {
            if let Some(current) = source.players.iter_mut().find(|item| item.id == player.id) {
                current.total_score += points;
            }
            source.scores.push(DbScore {
                round_id: original.id,
                player_id: player.id,
                points,
            });
        }
        source.rounds.insert(0, replayed_round);
        source.game.round_count += 1;
        source.game.status = if finished {
            Status::Finished.as_str().to_owned()
        } else {
            Status::Active.as_str().to_owned()
        };
    }

    let now = Utc::now();
    let player_ids: Vec<Uuid> = source.players.iter().map(|player| player.id).collect();
    let player_totals: Vec<i32> = source
        .players
        .iter()
        .map(|player| player.total_score)
        .collect();
    let score_round_ids: Vec<Uuid> = source.scores.iter().map(|score| score.round_id).collect();
    let score_player_ids: Vec<Uuid> = source.scores.iter().map(|score| score.player_id).collect();
    let score_values: Vec<i32> = source.scores.iter().map(|score| score.points).collect();
    let edited_round = source
        .rounds
        .iter()
        .find(|round| round.id == round_id)
        .expect("edited round exists after replay");

    let mut transaction = state.pool.begin().await?;
    let updated = sqlx::query(
        "UPDATE games SET status=$1,updated_at=$2 WHERE id=$3 AND owner_id=$4 AND updated_at=$5",
    )
    .bind(&source.game.status)
    .bind(now)
    .bind(id)
    .bind(owner)
    .bind(previous_updated_at)
    .execute(&mut *transaction)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::Conflict(
            "The game changed; refresh and try again".into(),
        ));
    }
    sqlx::query("UPDATE rounds SET rule_code=$1,note=$2,input_data=$3 WHERE id=$4 AND game_id=$5")
        .bind(&edited_round.rule_code)
        .bind(&edited_round.note)
        .bind(&edited_round.input_data)
        .bind(round_id)
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("UPDATE players AS player SET total_score=score.total FROM UNNEST($1::uuid[],$2::int[]) AS score(id,total) WHERE player.id=score.id AND player.game_id=$3")
        .bind(&player_ids)
        .bind(&player_totals)
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "DELETE FROM score_entries WHERE round_id IN (SELECT id FROM rounds WHERE game_id=$1)",
    )
    .bind(id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query("INSERT INTO score_entries (round_id,player_id,points) SELECT score.round_id,score.player_id,score.points FROM UNNEST($1::uuid[],$2::uuid[],$3::int[]) AS score(round_id,player_id,points)")
        .bind(&score_round_ids)
        .bind(&score_player_ids)
        .bind(&score_values)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;

    source.game.updated_at = now;
    Ok(Json(view(&source)))
}

pub async fn undo(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<GameMutation>> {
    let mut game = load_game_state(&state.pool, id, owner, 1).await?;
    let latest = game
        .rounds
        .first()
        .cloned()
        .ok_or_else(|| ApiError::Conflict("There is no round to undo".into()))?;
    let removed_scores = sqlx::query_as::<_, DbScore>(
        "SELECT round_id,player_id,points FROM score_entries WHERE round_id=$1",
    )
    .bind(latest.id)
    .fetch_all(&state.pool)
    .await?;
    let player_ids: Vec<Uuid> = removed_scores.iter().map(|score| score.player_id).collect();
    let values: Vec<i32> = removed_scores.iter().map(|score| score.points).collect();
    let now = Utc::now();
    let round_count = sqlx::query_scalar::<_, i32>("WITH score_values AS (SELECT * FROM UNNEST($5::uuid[],$6::int[]) AS scores(player_id,points)), updated_players AS (UPDATE players p SET total_score=p.total_score-scores.points FROM score_values scores WHERE p.id=scores.player_id AND p.game_id=$3 RETURNING p.id), deleted AS (DELETE FROM rounds WHERE id=$1 AND game_id=$3 RETURNING id), updated_game AS (UPDATE games SET round_count=GREATEST(round_count-1,0),updated_at=$2 WHERE id=$3 AND owner_id=$4 AND EXISTS (SELECT 1 FROM deleted) RETURNING round_count) SELECT round_count FROM updated_game")
        .bind(latest.id)
        .bind(now)
        .bind(id)
        .bind(owner)
        .bind(&player_ids)
        .bind(&values)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::Conflict("There is no round to undo".into()))?;
    for removed in &removed_scores {
        if let Some(player) = game
            .players
            .iter_mut()
            .find(|item| item.id == removed.player_id)
        {
            player.total_score -= removed.points;
        }
    }
    game.rounds.retain(|round| round.id != latest.id);
    game.game.round_count = round_count;
    game.game.updated_at = now;
    Ok(Json(GameMutation {
        status: game.game.game_status(),
        standings: standings(&game),
        round_count,
        updated_at: now,
        round: None,
        removed_round_id: Some(latest.id),
    }))
}

pub async fn set_status(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<StatusRequest>,
) -> ApiResult<Json<GameMutation>> {
    let mut game = load_game_state(&state.pool, id, owner, 0).await?;
    let now = Utc::now();
    let result =
        sqlx::query("UPDATE games SET status=$1,updated_at=$2 WHERE id=$3 AND owner_id=$4")
            .bind(request.status.as_str())
            .bind(now)
            .bind(id)
            .bind(owner)
            .execute(&state.pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Game not found".into()));
    }
    game.game.status = request.status.as_str().to_owned();
    game.game.updated_at = now;
    Ok(Json(GameMutation {
        status: request.status,
        standings: standings(&game),
        round_count: game.game.round_count,
        updated_at: now,
        round: None,
        removed_round_id: None,
    }))
}

async fn load_game(pool: &PgPool, id: Uuid, owner: Uuid, limit: i64) -> ApiResult<GameData> {
    let (game, players, (rounds, scores)) = tokio::try_join!(
        fetch_game(pool, id, owner),
        load_players(pool, id),
        load_rounds(pool, id, i32::MAX, limit),
    )?;
    let game = game.ok_or_else(|| ApiError::NotFound("Game not found".into()))?;
    Ok(GameData {
        game,
        players,
        rounds,
        scores,
    })
}

async fn load_game_state(
    pool: &PgPool,
    id: Uuid,
    owner: Uuid,
    history_limit: i64,
) -> ApiResult<GameData> {
    let (game, players, rounds) = tokio::try_join!(
        fetch_game(pool, id, owner),
        load_players(pool, id),
        sqlx::query_as::<_, DbRound>("SELECT id,round_number,rule_code,note,input_data,created_at FROM rounds WHERE game_id=$1 ORDER BY round_number DESC LIMIT $2")
            .bind(id).bind(history_limit).fetch_all(pool),
    )?;
    Ok(GameData {
        game: game.ok_or_else(|| ApiError::NotFound("Game not found".into()))?,
        players,
        rounds,
        scores: Vec::new(),
    })
}

async fn fetch_game(pool: &PgPool, id: Uuid, owner: Uuid) -> Result<Option<DbGame>, sqlx::Error> {
    sqlx::query_as::<_, DbGame>("SELECT id,title,preset_code,preset_name,team_mode,score_direction,target_score,threshold_rule,score_mode,round_limit,round_count,status,created_at,updated_at FROM games WHERE id=$1 AND owner_id=$2")
        .bind(id).bind(owner).fetch_optional(pool).await
}

async fn load_players(pool: &PgPool, id: Uuid) -> Result<Vec<DbPlayer>, sqlx::Error> {
    sqlx::query_as::<_, DbPlayer>("SELECT id,game_id,name,team_number,seat_order,total_score FROM players WHERE game_id=$1 ORDER BY seat_order ASC")
        .bind(id).fetch_all(pool).await
}

async fn load_rounds(
    pool: &PgPool,
    id: Uuid,
    before: i32,
    limit: i64,
) -> Result<(Vec<DbRound>, Vec<DbScore>), sqlx::Error> {
    let (rounds, scores) = tokio::try_join!(
        sqlx::query_as::<_, DbRound>("SELECT id,round_number,rule_code,note,input_data,created_at FROM rounds WHERE game_id=$1 AND round_number<$2 ORDER BY round_number DESC LIMIT $3")
            .bind(id).bind(before).bind(limit).fetch_all(pool),
        sqlx::query_as::<_, DbScore>("SELECT se.round_id,se.player_id,se.points FROM score_entries se JOIN rounds r ON r.id=se.round_id JOIN players p ON p.id=se.player_id WHERE r.id IN (SELECT id FROM rounds WHERE game_id=$1 AND round_number<$2 ORDER BY round_number DESC LIMIT $3) ORDER BY r.round_number DESC,p.seat_order ASC")
            .bind(id).bind(before).bind(limit).fetch_all(pool),
    )?;
    Ok((rounds, scores))
}

async fn insert_game(
    pool: &PgPool,
    owner: Uuid,
    game: &DbGame,
    players: &[DbPlayer],
) -> Result<(), sqlx::Error> {
    let ids: Vec<Uuid> = players.iter().map(|player| player.id).collect();
    let names: Vec<String> = players.iter().map(|player| player.name.clone()).collect();
    let teams: Vec<Option<i32>> = players.iter().map(|player| player.team_number).collect();
    let seats: Vec<i32> = players.iter().map(|player| player.seat_order).collect();
    sqlx::query("WITH inserted_game AS (INSERT INTO games (id,owner_id,title,preset_code,preset_name,team_mode,score_direction,target_score,threshold_rule,score_mode,round_limit,round_count,status,created_at,updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,0,'ACTIVE',$12,$12) RETURNING id) INSERT INTO players (id,game_id,name,team_number,seat_order,total_score) SELECT player.id,inserted_game.id,player.name,player.team_number,player.seat_order,0 FROM inserted_game CROSS JOIN UNNEST($13::uuid[],$14::text[],$15::int[],$16::int[]) AS player(id,name,team_number,seat_order)")
        .bind(game.id)
        .bind(owner)
        .bind(&game.title)
        .bind(&game.preset_code)
        .bind(&game.preset_name)
        .bind(game.team_mode)
        .bind(&game.score_direction)
        .bind(game.target_score)
        .bind(&game.threshold_rule)
        .bind(&game.score_mode)
        .bind(game.round_limit)
        .bind(game.created_at)
        .bind(&ids)
        .bind(&names)
        .bind(&teams)
        .bind(&seats)
        .execute(pool)
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
    standings_from_players(&data.game, &data.players)
}

fn standings_from_players(game: &DbGame, players: &[DbPlayer]) -> Vec<StandingView> {
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
            let total = members.iter().map(|player| player.total_score).sum();
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

fn summary(game: &DbGame, players: &[DbPlayer]) -> GameSummary {
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
        round_count: game.round_count,
        standings: standings_from_players(game, players),
        created_at: game.created_at,
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
    let round_views = round_views(data);
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
        round_count: data.game.round_count,
        has_more_rounds: data.game.round_count as usize > round_views.len(),
        players: player_views,
        standings: standings(data),
        rounds: round_views,
        created_at: data.game.created_at,
        updated_at: data.game.updated_at,
    }
}

fn round_views(data: &GameData) -> Vec<RoundView> {
    let players_by_id: HashMap<Uuid, &DbPlayer> = data
        .players
        .iter()
        .map(|player| (player.id, player))
        .collect();
    let mut scores_by_round: HashMap<Uuid, Vec<&DbScore>> = HashMap::new();
    for score in &data.scores {
        scores_by_round
            .entry(score.round_id)
            .or_default()
            .push(score);
    }
    let team_names: HashMap<i32, String> = data
        .players
        .iter()
        .filter_map(|player| player.team_number)
        .collect::<HashSet<_>>()
        .into_iter()
        .map(|team| {
            let name = data
                .players
                .iter()
                .filter(|player| player.team_number == Some(team))
                .map(|player| player.name.as_str())
                .collect::<Vec<_>>()
                .join(" & ");
            (team, name)
        })
        .collect();
    data.rounds
        .iter()
        .map(|round| {
            let scores = scores_by_round
                .get(&round.id)
                .into_iter()
                .flatten()
                .filter_map(|entry| {
                    let player = players_by_id.get(&entry.player_id)?;
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
                            name: team_names.get(&team).cloned().unwrap_or_default(),
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
                input_data: round
                    .input_data
                    .as_ref()
                    .and_then(|value| serde_json::from_value(value.clone()).ok()),
                scores,
            }
        })
        .collect()
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
                round_count: 0,
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
            input_data: None,
            created_at: Utc::now(),
        };
        for (seat, (team, total)) in values.iter().enumerate() {
            let player = DbPlayer {
                id: Uuid::new_v4(),
                game_id: data.game.id,
                name: format!("P{}", seat + 1),
                team_number: Some(*team),
                seat_order: seat as i32,
                total_score: *total,
            };
            data.scores.push(DbScore {
                round_id: round.id,
                player_id: player.id,
                points: *total,
            });
            data.players.push(player);
        }
        data.rounds.push(round);
        data.game.round_count = 1;
    }
}
