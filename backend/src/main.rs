use anyhow::{Context, Result};
use levant_score_api::{app, db, AppState};
use std::{env, sync::Arc};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info")),
        )
        .init();
    let pool = db::connect().await?;
    db::migrate(&pool)
        .await
        .context("run database migrations")?;
    let secret = env::var("JWT_SECRET")
        .unwrap_or_else(|_| "dev-only-secret-change-before-production-1234567890".into());
    let expiration = env::var("JWT_EXPIRATION_HOURS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(168);
    let origins = env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:5173,http://localhost:3000".into())
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let port: u16 = env::var("PORT")
        .or_else(|_| env::var("SERVER_PORT"))
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(27310);
    let state = AppState {
        pool,
        jwt_secret: Arc::from(secret),
        jwt_expiration_hours: expiration,
    };
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!(%port, "Levant Score API ready");
    axum::serve(listener, app(state, &origins))
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

async fn shutdown() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler")
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
}
