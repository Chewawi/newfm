use aide::axum::IntoApiResponse;
use aide::transform::TransformOperation;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    errors::{ApiResult, AppError},
    state::AppState,
};
use db::queries::tracks as tracks_db;
use shared::models::{Artist, Track};

/// GET /v1/track/:id
pub async fn get_track(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<Track>> {
    tracks_db::find_track_by_id(&state.db, id)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

pub fn _get_track_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Get a track")
        .description("Returns catalog metadata for a single track by its internal ID, including title, artist, album, and duration.")
        .tag("Catalog")
        .response::<200, Json<Track>>()
        .response_with::<404, (), _>(|r| r.description("Track not found"))
}

/// GET /v1/artist/:id
pub async fn get_artist(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<Artist>> {
    tracks_db::find_artist_by_id(&state.db, id)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

pub fn _get_artist_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Get an artist")
        .description("Returns catalog metadata for a single artist by its internal ID.")
        .tag("Catalog")
        .response::<200, Json<Artist>>()
        .response_with::<404, (), _>(|r| r.description("Artist not found"))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchQuery {
    pub q: String,
    pub r#type: Option<String>, // "track" | "artist" | "all" (default)
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SearchResponse {
    pub artists: Vec<Artist>,
    pub tracks: Vec<Track>,
}

/// GET /v1/search
pub async fn search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<impl IntoApiResponse> {
    if q.q.trim().is_empty() {
        return Err(AppError::BadRequest("search query cannot be empty".into()));
    }

    let limit = q.limit.unwrap_or(10).min(30);
    let kind = q.r#type.as_deref().unwrap_or("all");

    let artists = if kind == "all" || kind == "artist" {
        tracks_db::search_artists(&state.db, &q.q, limit).await?
    } else {
        vec![]
    };

    let tracks = if kind == "all" || kind == "track" {
        tracks_db::search_tracks(&state.db, &q.q, limit).await?
    } else {
        vec![]
    };

    Ok(Json(SearchResponse { artists, tracks }))
}

pub fn _search_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Search catalog")
        .description("Full-text search across the music catalog. Use the `type` parameter to restrict results to `track`, `artist`, or `all` (default). Returns up to 30 results per type. Query must not be empty.")
        .tag("Catalog")
        .response::<200, Json<SearchResponse>>()
        .response_with::<400, (), _>(|r| r.description("Empty search query"))
}
