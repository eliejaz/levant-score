pub mod api;
pub mod features;
pub mod infrastructure;

use sqlx::PgPool;
use std::sync::Arc;

pub use api::router as app;
pub use infrastructure::database as db;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: Arc<str>,
    pub jwt_expiration_hours: u64,
}
