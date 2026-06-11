use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");

    tracing::info!("worker: connecting to database...");
    let db = db::pool::connect(&database_url).await?;

    tracing::info!("worker: starting background loops");

    // ── Cleanup loop ──────────────────────────────────────────────────────
    // Runs every 5 minutes and purges expired sessions + now_playing rows.
    let db_cleanup = db.clone();
    let cleanup_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            match cleanup_expired(&db_cleanup).await {
                Ok((sessions, now_playing)) => {
                    tracing::info!(
                        "cleanup: removed {sessions} expired sessions, {now_playing} stale now_playing rows"
                    );
                }
                Err(e) => tracing::error!("cleanup error: {e}"),
            }
        }
    });

    // ── Wait for tasks (they loop forever unless cancelled) ───────────────
    tokio::select! {
        _ = cleanup_handle => tracing::warn!("cleanup task exited unexpectedly"),
        _ = tokio::signal::ctrl_c() => tracing::info!("received Ctrl-C, shutting down"),
    }

    Ok(())
}

/// Purges expired `user_sessions` and stale `now_playing` rows.
/// Returns `(sessions_deleted, now_playing_deleted)`.
async fn cleanup_expired(db: &sqlx::PgPool) -> Result<(u64, u64), sqlx::Error> {
    let sessions = db::queries::auth::delete_expired_sessions(db).await?;

    let np_result = sqlx::query!("DELETE FROM now_playing WHERE expires_at <= NOW()")
        .execute(db)
        .await?;

    Ok((sessions, np_result.rows_affected()))
}
