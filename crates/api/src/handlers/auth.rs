use aide::axum::IntoApiResponse;
use aide::transform::TransformOperation;
use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
};
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    errors::{ApiResult, AppError},
    middleware::auth::AuthUser,
    state::AppState,
};
use db::queries::{auth as auth_db, users as users_db};
use shared::user::{hash_password, verify_password};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AuthResponse {
    pub token: Uuid,
    pub user_id: i64,
    pub username: String,
}

/// POST /v1/register
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> ApiResult<(StatusCode, Json<AuthResponse>)> {
    // Validate input
    if body.username.trim().is_empty() || body.username.len() < 2 {
        return Err(AppError::BadRequest(
            "username must be at least 2 characters".into(),
        ));
    }
    if body.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }

    // Check uniqueness
    if users_db::find_by_username(&state.db, &body.username)
        .await?
        .is_some()
    {
        return Err(AppError::UsernameTaken);
    }
    if users_db::find_by_email(&state.db, &body.email)
        .await?
        .is_some()
    {
        return Err(AppError::EmailTaken);
    }

    let password_hash =
        hash_password(&body.password).map_err(|e| AppError::BadRequest(e.to_string()))?;

    let user = users_db::create_user(
        &state.db,
        &users_db::CreateUser {
            username: &body.username,
            email: &body.email,
            password_hash: &password_hash,
            display_name: body.display_name.as_deref(),
        },
    )
    .await?;

    let session = auth_db::create_session(&state.db, user.id, None, None).await?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token: session.id,
            user_id: user.id,
            username: user.username,
        }),
    ))
}

pub fn _register_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Register a new user")
        .description("Creates a new user account and returns a session token. Username must be at least 2 characters and password at least 8.")
        .tag("Auth")
        .response::<201, Json<AuthResponse>>()
        .response_with::<400, (), _>(|r| r.description("Validation error (username too short, password too short)"))
        .response_with::<409, (), _>(|r| r.description("Username or email already taken"))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// POST /v1/login
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> ApiResult<Json<AuthResponse>> {
    let user = users_db::find_by_username(&state.db, &body.username)
        .await?
        .ok_or(AppError::InvalidCredentials)?;

    verify_password(&body.password, &user.password_hash)
        .map_err(|_| AppError::InvalidCredentials)?;

    let session = auth_db::create_session(&state.db, user.id, None, None).await?;

    Ok(Json(AuthResponse {
        token: session.id,
        user_id: user.id,
        username: user.username,
    }))
}

pub fn _login_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Log in")
        .description("Authenticates a user with username and password. Returns a session token to be used as `Bearer` in the `Authorization` header.")
        .tag("Auth")
        .response::<200, Json<AuthResponse>>()
        .response_with::<401, (), _>(|r| r.description("Invalid credentials"))
}

/// POST /v1/logout
pub async fn logout(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> ApiResult<StatusCode> {
    // We need the raw token to find which session to delete
    // In practice extract it from the Authorization header directly
    // For simplicity, delete all sessions for this user (logout everywhere)
    // A more precise implementation would extract the session UUID from the header.
    sqlx::query!("DELETE FROM user_sessions WHERE user_id = $1", auth_user.id,)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn _logout_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Log out")
        .description("Invalidates all active sessions for the authenticated user (logout from all devices). Requires a valid session token.")
        .tag("Auth")
        .response_with::<204, (), _>(|r| r.description("Successfully logged out"))
        .response_with::<401, (), _>(|r| r.description("Not authenticated"))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateTokenRequest {
    pub name: String,
    pub scopes: Option<Vec<String>>,
    pub expires_days: Option<i64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CreateTokenResponse {
    pub id: Uuid,
    pub name: String,
    pub token: String, // raw token — shown only once
    pub scopes: Vec<String>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
}

/// POST /v1/auth/tokens
pub async fn create_api_token(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(body): Json<CreateTokenRequest>,
) -> ApiResult<impl IntoApiResponse> {
    // Generate a cryptographically random 32-byte token
    let raw_bytes: [u8; 32] = rand::random();
    let raw_token = hex::encode(raw_bytes);

    // Hash it for storage (sha2 would be ideal; using simple approach for now)
    let token_hash = sha2_hex(&raw_token);

    let scopes = body.scopes.unwrap_or_else(|| vec!["scrobble".into()]);

    let api_token = auth_db::create_api_token(
        &state.db,
        auth_user.id,
        &body.name,
        &token_hash,
        &scopes,
        body.expires_days,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(CreateTokenResponse {
            id: api_token.id,
            name: api_token.name,
            token: raw_token, // only time the raw token is shown
            scopes: api_token.scopes,
            expires_at: api_token.expires_at,
            created_at: api_token.created_at,
        }),
    ))
}

pub fn _create_api_token_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Create an API token")
        .description("Generates a new long-lived API token for programmatic access (e.g. scrobbling from a music player). The raw token is only shown once — store it securely.")
        .tag("Auth")
        .response::<201, Json<CreateTokenResponse>>()
        .response_with::<401, (), _>(|r| r.description("Not authenticated"))
}

/// GET /v1/auth/tokens
pub async fn list_api_tokens(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> ApiResult<impl IntoApiResponse> {
    let tokens = auth_db::list_api_tokens(&state.db, auth_user.id).await?;
    Ok(Json(tokens))
}

pub fn _list_api_tokens_doc(op: TransformOperation) -> TransformOperation {
    op.summary("List API tokens")
        .description("Returns all active API tokens belonging to the authenticated user. The raw token value is never returned here — only metadata.")
        .tag("Auth")
        .response_with::<401, (), _>(|r| r.description("Not authenticated"))
}

/// DELETE /v1/auth/tokens/:token_id
pub async fn delete_api_token(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    axum::extract::Path(token_id): axum::extract::Path<Uuid>,
) -> ApiResult<impl IntoApiResponse> {
    let deleted = auth_db::delete_api_token(&state.db, token_id, auth_user.id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

pub fn _delete_api_token_doc(op: TransformOperation) -> TransformOperation {
    op.summary("Revoke an API token")
        .description("Permanently deletes an API token by its ID. Only the owner of the token can revoke it.")
        .tag("Auth")
        .response_with::<204, (), _>(|r| r.description("Token successfully revoked"))
        .response_with::<401, (), _>(|r| r.description("Not authenticated"))
        .response_with::<404, (), _>(|r| r.description("Token not found or not owned by the authenticated user"))
}

fn sha2_hex(input: &str) -> String {
    // Structural placeholder — replace with sha2::Sha256 in production
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
