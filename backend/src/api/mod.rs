pub mod error;

use axum::{
    http::{header, HeaderValue, Method},
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::time::Duration;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

use crate::{
    features::{auth, games},
    AppState,
};

pub fn router(state: AppState, origins: &[String]) -> Router {
    let allowed: Vec<HeaderValue> = origins
        .iter()
        .filter_map(|origin| origin.trim().parse().ok())
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(allowed)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .max_age(Duration::from_secs(60 * 60))
        .allow_credentials(true);

    Router::new()
        .route("/actuator/health", get(health))
        .route("/api/presets", get(presets))
        .route("/api/auth/signup", post(auth::signup))
        .route("/api/auth/login", post(auth::login))
        .route("/api/games", get(games::list).post(games::create))
        .route("/api/games/{id}", get(games::get).delete(games::delete))
        .route(
            "/api/games/{id}/rounds",
            get(games::list_rounds).post(games::add_round),
        )
        .route("/api/games/{id}/rematch", post(games::rematch))
        .route("/api/games/{id}/rounds/latest", delete(games::undo))
        .route("/api/games/{id}/status", patch(games::set_status))
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({"status":"UP"}))
}

async fn presets() -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, "public, max-age=3600")],
        Json(games::presets::all()),
    )
}
