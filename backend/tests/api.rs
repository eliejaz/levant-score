use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use levant_score_api::{app, AppState};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

#[sqlx::test(migrations = "./migrations")]
async fn auth_game_round_validation_and_delete_work_end_to_end(pool: sqlx::PgPool) {
    let app = app(
        AppState {
            pool,
            jwt_secret: Arc::from("integration-test-secret-at-least-32-characters"),
            jwt_expiration_hours: 168,
        },
        &["http://localhost:5173".into()],
    );

    let signup = send(
        &app,
        "POST",
        "/api/auth/signup",
        None,
        Some(json!({
            "displayName":"Elie", "email":"elie@example.com", "password":"password123"
        })),
    )
    .await;
    assert_eq!(signup.0, StatusCode::OK);
    let token = signup.1["token"].as_str().unwrap().to_owned();

    let created = send(
        &app,
        "POST",
        "/api/games",
        Some(&token),
        Some(json!({
            "title":"Friday Tarneeb", "presetCode":"tarneeb",
            "players":[
                {"name":"A","teamNumber":1},{"name":"B","teamNumber":2},
                {"name":"C","teamNumber":1},{"name":"D","teamNumber":2}
            ]
        })),
    )
    .await;
    assert_eq!(created.0, StatusCode::OK);
    assert_eq!(created.1["targetScore"], 31);
    let id = created.1["id"].as_str().unwrap();

    let round = send(
        &app,
        "POST",
        &format!("/api/games/{id}/rounds"),
        Some(&token),
        Some(json!({
            "bidderTeam":1, "bid":8, "tricks":7,
            "scores":[{"teamNumber":1},{"teamNumber":2}]
        })),
    )
    .await;
    assert_eq!(round.0, StatusCode::OK);
    assert_eq!(round.1["rounds"][0]["scores"][0]["points"], -8);
    assert_eq!(round.1["rounds"][0]["scores"][1]["points"], 6);

    let games = send(&app, "GET", "/api/games", Some(&token), None).await;
    assert_eq!(games.0, StatusCode::OK);
    assert_eq!(games.1.as_array().unwrap().len(), 1);
    assert_eq!(games.1[0]["roundCount"], 1);
    assert_eq!(games.1[0]["standings"][0]["total"], 6);
    assert_eq!(games.1[0]["standings"][1]["total"], -8);

    let deleted = send(
        &app,
        "DELETE",
        &format!("/api/games/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(deleted.0, StatusCode::NO_CONTENT);
    let missing = send(&app, "GET", &format!("/api/games/{id}"), Some(&token), None).await;
    assert_eq!(missing.0, StatusCode::NOT_FOUND);
    assert_eq!(missing.1["message"], "Game not found");
}

#[sqlx::test(migrations = "./migrations")]
async fn leekha_rejects_a_hand_that_does_not_distribute_36_points(pool: sqlx::PgPool) {
    let app = app(
        AppState {
            pool,
            jwt_secret: Arc::from("integration-test-secret-at-least-32-characters"),
            jwt_expiration_hours: 168,
        },
        &["http://localhost:5173".into()],
    );
    let signup = send(
        &app,
        "POST",
        "/api/auth/signup",
        None,
        Some(json!({"displayName":"Player","email":"player@example.com","password":"password123"})),
    )
    .await;
    let token = signup.1["token"].as_str().unwrap();
    let created = send(
        &app,
        "POST",
        "/api/games",
        Some(token),
        Some(json!({
            "title":"Leekha", "presetCode":"leekha-solo",
            "players":[{"name":"A"},{"name":"B"},{"name":"C"},{"name":"D"}]
        })),
    )
    .await;
    let id = created.1["id"].as_str().unwrap();
    let players = created.1["players"].as_array().unwrap();
    let scores: Vec<Value> = players
        .iter()
        .enumerate()
        .map(|(i, p)| json!({"playerId":p["id"],"points":if i==0 {13} else if i==1 {24} else {0}}))
        .collect();
    let invalid = send(
        &app,
        "POST",
        &format!("/api/games/{id}/rounds"),
        Some(token),
        Some(json!({"scores":scores})),
    )
    .await;
    assert_eq!(invalid.0, StatusCode::BAD_REQUEST);
    assert_eq!(
        invalid.1["message"],
        "A Leekha hand must distribute exactly 36 points"
    );
}

async fn send(
    app: &axum::Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(
            builder
                .body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, value)
}
