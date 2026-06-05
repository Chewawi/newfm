use aide::transform::TransformOpenApi;
use aide::{
    axum::{
        routing::{delete, get, post}, ApiRouter,
        IntoApiResponse,
    },
    openapi::OpenApi,
    scalar::Scalar,
};
use axum::{middleware, routing::get as axum_get, Extension, Json, Router};
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
        .api_route("/v1/scrobble", post(scrobbles::scrobble))
        .api_route("/v1/now-playing", post(scrobbles::update_now_playing))
        // Social
        .api_route("/v1/user/{username}/follow", post(users::follow))
        .api_route("/v1/user/{username}/follow", delete(users::unfollow))
        // API tokens
        .api_route("/v1/auth/tokens", post(auth::create_api_token))
        .api_route("/v1/auth/tokens", get(auth::list_api_tokens))
        .api_route("/v1/auth/tokens/{id}", delete(auth::delete_api_token))
        // Logout
        .api_route("/v1/auth/logout", post(auth::logout))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // Public routes
    let public = ApiRouter::new()
        // Auth
        .api_route("/v1/auth/register", post(auth::register))
        .api_route("/v1/auth/login", post(auth::login))
        // User profiles (visibility checked inside handler)
        .api_route("/v1/user/{username}", get(users::get_profile))
        .api_route("/v1/user/{username}/friends", get(users::get_friends))
        .api_route(
            "/v1/user/{username}/recent",
            get(scrobbles::recent_scrobbles),
        )
        // SSE endpoint — registered as a plain route (aide does not support streaming response types)
        .route(
            "/v1/user/{username}/live",
            axum_get(scrobbles::live_now_playing),
        )
        .api_route(
            "/v1/user/{username}/top-artists",
            get(scrobbles::top_artists),
        )
        .api_route("/v1/user/{username}/top-tracks", get(scrobbles::top_tracks))
        .api_route(
            "/v1/user/{username}/heatmap",
            get(scrobbles::activity_heatmap),
        )
        // Catalog
        .api_route("/v1/track/{id}", get(tracks::get_track))
        .api_route("/v1/artist/{id}", get(tracks::get_artist))
        .api_route("/v1/search", get(tracks::search))
        // Health
        .api_route("/health", get(health));

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
