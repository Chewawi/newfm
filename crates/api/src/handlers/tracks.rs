use aide::axum::IntoApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
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
