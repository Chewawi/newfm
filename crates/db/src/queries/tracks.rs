use sqlx::PgPool;

use shared::models::{Artist, Track};

/// Looks up an artist by their normalized name, creating one if none exists.
///
/// On conflict the original casing is preserved — the `DO UPDATE SET name`
/// clause is intentionally a no-op so the first-inserted casing wins.
pub async fn find_or_create_artist(pool: &PgPool, name: &str) -> Result<Artist, sqlx::Error> {
    let normalized = name.trim().to_lowercase();

    sqlx::query_as!(
        Artist,
        r#"
        INSERT INTO artists (name, name_normalized)
        VALUES ($1, $2)
        ON CONFLICT (name_normalized) DO UPDATE
            SET name = artists.name
        RETURNING id, name, name_normalized, mbid, image_url, bio,
                  scrobble_count, listener_count, created_at
        "#,
        name.trim(),
        normalized,
    )
    .fetch_one(pool)
    .await
}

pub async fn find_artist_by_id(pool: &PgPool, id: i64) -> Result<Option<Artist>, sqlx::Error> {
    sqlx::query_as!(
        Artist,
        r#"
        SELECT id, name, name_normalized, mbid, image_url, bio,
               scrobble_count, listener_count, created_at
        FROM artists
        WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(pool)
    .await
}

/// Searches artists by fuzzy name match using pg_trgm similarity.
///
/// Results are ranked by trigram similarity first, then by global popularity
/// (`scrobble_count`) as a tiebreaker. The `%` operator applies a minimum
/// similarity threshold (default 0.3) set via `pg_trgm.similarity_threshold`.
pub async fn search_artists(
    pool: &PgPool,
    query: &str,
    limit: i64,
) -> Result<Vec<Artist>, sqlx::Error> {
    sqlx::query_as!(
        Artist,
        r#"
        SELECT id, name, name_normalized, mbid, image_url, bio,
               scrobble_count, listener_count, created_at
        FROM artists
        WHERE name % $1
        ORDER BY similarity(name, $1) DESC, scrobble_count DESC
        LIMIT $2
        "#,
        query,
        limit,
    )
    .fetch_all(pool)
    .await
}

pub async fn find_or_create_album(
    pool: &PgPool,
    artist_id: i64,
    title: &str,
) -> Result<i64, sqlx::Error> {
    let normalized = title.trim().to_lowercase();

    let row = sqlx::query!(
        r#"
        INSERT INTO albums (artist_id, title, title_normalized)
        VALUES ($1, $2, $3)
        ON CONFLICT (artist_id, title_normalized) DO UPDATE
            SET title = albums.title
        RETURNING id
        "#,
        artist_id,
        title.trim(),
        normalized,
    )
    .fetch_one(pool)
    .await?;

    Ok(row.id)
}

/// Looks up a track by `(artist_id, title_normalized)`, creating one if none exists.
///
/// On conflict, `album_id` and `duration_ms` are filled in only if the existing
/// row has `NULL` in those columns (`COALESCE` keeps the stored value otherwise).
/// This lets callers enrich incomplete records without overwriting good data.
pub async fn find_or_create_track(
    pool: &PgPool,
    artist_id: i64,
    album_id: Option<i64>,
    title: &str,
    duration_ms: Option<i32>,
) -> Result<Track, sqlx::Error> {
    let normalized = title.trim().to_lowercase();

    sqlx::query_as!(
        Track,
        r#"
        INSERT INTO tracks (artist_id, album_id, title, title_normalized, duration_ms)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (artist_id, title_normalized) DO UPDATE
            SET album_id    = COALESCE(tracks.album_id,    EXCLUDED.album_id),
                duration_ms = COALESCE(tracks.duration_ms, EXCLUDED.duration_ms)
        RETURNING id, artist_id, album_id, title, title_normalized, mbid,
                  duration_ms, scrobble_count, created_at
        "#,
        artist_id,
        album_id,
        title.trim(),
        normalized,
        duration_ms,
    )
    .fetch_one(pool)
    .await
}

pub async fn find_track_by_id(pool: &PgPool, id: i64) -> Result<Option<Track>, sqlx::Error> {
    sqlx::query_as!(
        Track,
        r#"
        SELECT id, artist_id, album_id, title, title_normalized, mbid,
               duration_ms, scrobble_count, created_at
        FROM tracks
        WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(pool)
    .await
}

/// See [`search_artists`] — same ranking strategy applied to track titles.
pub async fn search_tracks(
    pool: &PgPool,
    query: &str,
    limit: i64,
) -> Result<Vec<Track>, sqlx::Error> {
    sqlx::query_as!(
        Track,
        r#"
        SELECT id, artist_id, album_id, title, title_normalized, mbid,
               duration_ms, scrobble_count, created_at
        FROM tracks
        WHERE title % $1
        ORDER BY similarity(title, $1) DESC, scrobble_count DESC
        LIMIT $2
        "#,
        query,
        limit,
    )
    .fetch_all(pool)
    .await
}
