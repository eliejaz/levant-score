use sqlx::PgPool;
use uuid::Uuid;

use super::entity::{DbGame, DbPlayer, DbRound, DbScore, GameData};
use crate::api::error::{ApiError, ApiResult};

pub(super) async fn load_game(
    pool: &PgPool,
    id: Uuid,
    owner: Uuid,
    limit: i64,
) -> ApiResult<GameData> {
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

pub(super) async fn load_game_state(
    pool: &PgPool,
    id: Uuid,
    owner: Uuid,
    history_limit: i64,
) -> ApiResult<GameData> {
    let row = sqlx::query_as::<
        _,
        (
            sqlx::types::Json<DbGame>,
            sqlx::types::Json<Vec<DbPlayer>>,
            sqlx::types::Json<Vec<DbRound>>,
        ),
    >(
        "SELECT to_jsonb(game_row),COALESCE((SELECT jsonb_agg(to_jsonb(player_row) ORDER BY player_row.seat_order) FROM (SELECT id,name,team_number,seat_order,total_score FROM players WHERE game_id=$1) player_row),'[]'::jsonb),COALESCE((SELECT jsonb_agg(to_jsonb(round_row) ORDER BY round_row.round_number DESC) FROM (SELECT id,round_number,rule_code,note,input_data,created_at FROM rounds WHERE game_id=$1 ORDER BY round_number DESC LIMIT $3) round_row),'[]'::jsonb) FROM (SELECT id,title,preset_code,preset_name,team_mode,score_direction,target_score,threshold_rule,score_mode,round_limit,round_count,status,outcome_data,created_at,updated_at FROM games WHERE id=$1 AND owner_id=$2) game_row",
    )
    .bind(id)
    .bind(owner)
    .bind(history_limit)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Game not found".into()))?;
    Ok(GameData {
        game: row.0 .0,
        players: row.1 .0,
        rounds: row.2 .0,
        scores: Vec::new(),
    })
}

pub(super) async fn fetch_game(
    pool: &PgPool,
    id: Uuid,
    owner: Uuid,
) -> Result<Option<DbGame>, sqlx::Error> {
    sqlx::query_as::<_, DbGame>("SELECT id,title,preset_code,preset_name,team_mode,score_direction,target_score,threshold_rule,score_mode,round_limit,round_count,status,outcome_data,created_at,updated_at FROM games WHERE id=$1 AND owner_id=$2")
        .bind(id).bind(owner).fetch_optional(pool).await
}

pub(super) async fn load_players(pool: &PgPool, id: Uuid) -> Result<Vec<DbPlayer>, sqlx::Error> {
    sqlx::query_as::<_, DbPlayer>("SELECT id,name,team_number,seat_order,total_score FROM players WHERE game_id=$1 ORDER BY seat_order ASC")
        .bind(id).fetch_all(pool).await
}

pub(super) async fn load_rounds(
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

pub(super) async fn insert_game(
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
