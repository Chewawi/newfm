use aide::axum::IntoApiResponse;
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    errors::{ApiResult, AppError},
    middleware::auth::AuthUser,
    state::AppState,
};
use db::queries::users as users_db;
use shared::models::UserProfile;

#[derive(Debug, Serialize, JsonSchema)]
pub struct ProfileResponse {
    #[serde(flatten)]
    pub profile: UserProfile,
    pub is_following: Option<bool>, // None if not authenticated
}

/// GET /v1/user/:username
pub async fn get_profile(
    State(state): State<AppState>,
    Path(username): Path<String>,
    auth_user: Option<Extension<AuthUser>>,
) -> ApiResult<impl IntoApiResponse> {
    let user = users_db::find_by_username(&state.db, &username)
        .await?
        .ok_or(AppError::NotFound)?;

    if user.is_private {
        // Private profile — only visible to followers or the user themselves
        let viewer_id = auth_user.as_ref().map(|a| a.id);
        let is_owner = viewer_id == Some(user.id);
        if !is_owner {
            return Err(AppError::Forbidden);
        }
    }

    let is_following = if let Some(Extension(viewer)) = &auth_user {
        if viewer.id != user.id {
            Some(users_db::is_following(&state.db, viewer.id, user.id).await?)
        } else {
            None
        }
    } else {
        None
    };

    Ok(Json(ProfileResponse {
        profile: user.into(),
        is_following,
    }))
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FriendsResponse {
    pub followers: Vec<UserProfile>,
    pub following: Vec<UserProfile>,
}

/// GET /v1/user/:username/friends
pub async fn get_friends(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> ApiResult<impl IntoApiResponse> {
    let user = users_db::find_by_username(&state.db, &username)
        .await?
        .ok_or(AppError::NotFound)?;

    let followers = users_db::get_followers(&state.db, user.id)
        .await?
        .into_iter()
        .map(UserProfile::from)
        .collect();

    let following = users_db::get_following(&state.db, user.id)
        .await?
        .into_iter()
        .map(UserProfile::from)
        .collect();

    Ok(Json(FriendsResponse {
        followers,
        following,
    }))
}

/// POST /v1/user/:username/follow
pub async fn follow(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(username): Path<String>,
) -> ApiResult<StatusCode> {
    let target = users_db::find_by_username(&state.db, &username)
        .await?
        .ok_or(AppError::NotFound)?;

    if target.id == auth_user.id {
        return Err(AppError::BadRequest("cannot follow yourself".into()));
    }

    users_db::follow_user(&state.db, auth_user.id, target.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /v1/user/:username/follow
pub async fn unfollow(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(username): Path<String>,
) -> ApiResult<StatusCode> {
    let target = users_db::find_by_username(&state.db, &username)
        .await?
        .ok_or(AppError::NotFound)?;

    users_db::unfollow_user(&state.db, auth_user.id, target.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
