use anyhow::{Context, Result};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    PgPool,
};
use std::{env, str::FromStr, time::Duration};

pub async fn connect() -> Result<PgPool> {
    let raw = env::var("DATABASE_URL")
        .or_else(|_| env::var("DB_URL"))
        .unwrap_or_else(|_| "postgresql://localhost:5432/levant_score".into());
    let raw = raw.strip_prefix("jdbc:").unwrap_or(&raw);
    let mut options = PgConnectOptions::from_str(raw).context("invalid PostgreSQL URL")?;
    if let Ok(user) = env::var("DB_USER") {
        options = options.username(&user);
    }
    if let Ok(password) = env::var("DB_PASSWORD") {
        options = options.password(&password);
    }
    let max_connections = env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(5);
    PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(0)
        .acquire_timeout(Duration::from_secs(20))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .connect_with(options)
        .await
        .context("connect to PostgreSQL")
}

pub async fn migrate(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
