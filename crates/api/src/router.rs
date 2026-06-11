use aide::transform::TransformOpenApi;
use aide::{
    axum::{
        ApiRouter, IntoApiResponse,
        routing::{delete_with, get, get_with, post_with},
    },
    openapi::OpenApi,
    scalar::Scalar,
};
use axum::{Extension, Json, Router, middleware};
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::{
    handlers::{auth, scrobbles, tracks, users},
    middleware::{auth::require_auth, rate_limit::rate_limit},
    state::AppState,
};

async fn serve_api(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    Json(api)
}
pub fn build(state: AppState) -> Router {
    // Authenticated routes
    let authed = ApiRouter::new()
        // Scrobbling
        .api_route(
            "/v1/scrobble",
            post_with(scrobbles::scrobble, scrobbles::_scrobble_doc),
        )
        .api_route(
            "/v1/now-playing",
            post_with(
                scrobbles::update_now_playing,
                scrobbles::_update_now_playing_doc,
            ),
        )
        // Social
        .api_route(
            "/v1/user/{username}/follow",
            post_with(users::follow, users::_follow_doc),
        )
        .api_route(
            "/v1/user/{username}/follow",
            delete_with(users::unfollow, users::_unfollow_doc),
        )
        // API tokens
        .api_route(
            "/v1/auth/tokens",
            post_with(auth::create_api_token, auth::_create_api_token_doc),
        )
        .api_route(
            "/v1/auth/tokens",
            get_with(auth::list_api_tokens, auth::_list_api_tokens_doc),
        )
        .api_route(
            "/v1/auth/tokens/{id}",
            delete_with(auth::delete_api_token, auth::_delete_api_token_doc),
        )
        // Logout
        .api_route(
            "/v1/auth/logout",
            post_with(auth::logout, auth::_logout_doc),
        )
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // Public routes
    let public = ApiRouter::new()
        // Auth
        .api_route(
            "/v1/auth/register",
            post_with(auth::register, auth::_register_doc),
        )
        .api_route("/v1/auth/login", post_with(auth::login, auth::_login_doc))
        // User profiles (visibility checked inside handler)
        .api_route(
            "/v1/user/{username}",
            get_with(users::get_profile, users::_get_profile_doc),
        )
        .api_route(
            "/v1/user/{username}/friends",
            get_with(users::get_friends, users::_get_friends_doc),
        )
        .api_route(
            "/v1/user/{username}/recent",
            get_with(
                scrobbles::recent_scrobbles,
                scrobbles::_recent_scrobbles_doc,
            ),
        )
        // SSE endpoint — registered as a plain route (aide does not support streaming response types)
        .api_route(
            "/v1/user/{username}/live",
            get_with(
                scrobbles::live_now_playing,
                scrobbles::_live_now_playing_doc,
            ),
        )
        .api_route(
            "/v1/user/{username}/top-artists",
            get_with(scrobbles::top_artists, scrobbles::_top_artists_doc),
        )
        .api_route(
            "/v1/user/{username}/top-tracks",
            get_with(scrobbles::top_tracks, scrobbles::_top_tracks_doc),
        )
        .api_route(
            "/v1/user/{username}/heatmap",
            get_with(
                scrobbles::activity_heatmap,
                scrobbles::_activity_heatmap_doc,
            ),
        )
        // Catalog
        .api_route(
            "/v1/track/{id}",
            get_with(tracks::get_track, tracks::_get_track_doc),
        )
        .api_route(
            "/v1/artist/{id}",
            get_with(tracks::get_artist, tracks::_get_artist_doc),
        )
        .api_route("/v1/search", get_with(tracks::search, tracks::_search_doc))
        // Health
        .api_route(
            "/health",
            get_with(health, |r| r.hidden(true).description("Health check xD")),
        );

    let mut api = OpenApi::default();

    ApiRouter::new()
        .route("/docs", Scalar::new("/api.json").axum_route())
        .merge(authed)
        .merge(public)
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods(Any),
        )
        .route("/api.json", get(serve_api))
        .finish_api_with(&mut api, api_docs)
        .layer(Extension(Arc::new(api)))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

fn api_docs(api: TransformOpenApi) -> TransformOpenApi {
    api.title("Aide axum Open API")
        .summary("An example Todo application")
        .description(include_str!("../../../README.md"))
        .security_scheme(
            "ApiKey",
            aide::openapi::SecurityScheme::ApiKey {
                location: aide::openapi::ApiKeyLocation::Header,
                name: "X-Auth-Key".into(),
                description: Some("A key that is ignored.".into()),
                extensions: Default::default(),
            },
        )
}
