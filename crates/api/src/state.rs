use axum::extract::FromRef;
use fred::clients::Client as RedisClient;
use sqlx::PgPool;

/// Shared application state injected into every handler via Axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: RedisClient,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for RedisClient {
    fn from_ref(state: &AppState) -> Self {
        state.redis.clone()
    }
}
