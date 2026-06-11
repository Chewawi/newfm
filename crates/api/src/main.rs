mod errors;
mod handlers;
mod middleware;
mod router;
mod state;

use fred::{interfaces::ClientLike, types::Builder as RedisBuilder};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    // Database
    tracing::info!("connecting to database...");
    let db = db::pool::connect(&database_url).await?;

    // tracing::info!("running migrations...");
    // sqlx::migrate!("../../migrations").run(&db).await?;

    // Redis
    tracing::info!("connecting to redis...");
    let redis =
        RedisBuilder::from_config(fred::types::config::Config::from_url(&redis_url)?).build()?;
    redis.init().await?;

    // Axum
    let state = state::AppState { db, redis };
    let app = router::build(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("newfm api listening on {bind_addr}");

    axum::serve(listener, app).await?;
    Ok(())
}
