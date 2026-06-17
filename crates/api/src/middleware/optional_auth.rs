use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{middleware::auth::AuthUser, state::AppState};
use db::queries::{auth as auth_db, users as users_db};
use uuid::Uuid;

pub async fn optional_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    if let Some(token) = extract_bearer_token_optional(&req) {
        if let Ok(session_id) = Uuid::parse_str(&token) {
            if let Ok(session) = auth_db::get_session(&state.db, session_id).await {
                if let Some(session) = session {
                    if let Ok(Some(user)) = users_db::find_by_id(&state.db, session.user_id).await {
                        req.extensions_mut().insert(AuthUser {
                            id: user.id,
                            username: user.username,
                            scopes: vec!["read".into()],
                        });
                    }
                }
            }
        }
    }

    next.run(req).await
}

fn extract_bearer_token_optional(req: &Request) -> Option<String> {
    req.headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(|s| s.to_string()))
}
