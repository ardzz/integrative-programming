use std::{sync::Arc, time::Duration};

use axum::{
    extract::MatchedPath,
    http::Request,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tower::ServiceBuilder;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::cors::CorsLayer;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::{info_span, Span};

use crate::handler::{auth, comment, post as post_handler, user};
use crate::AppState;

fn rate_limit_enabled() -> bool {
    std::env::var("RATE_LIMIT_ENABLED")
        .unwrap_or_else(|_| "true".into())
        == "true"
}

fn with_rate_limit<S>(router: Router<S>, per_second: u64, burst_size: u32) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if rate_limit_enabled() {
        let config = GovernorConfigBuilder::default()
            .per_second(per_second)
            .burst_size(burst_size)
            .finish()
            .expect("valid rate limit config");

        router.layer(GovernorLayer { config: Arc::new(config) })
    } else {
        router
    }
}

pub fn create_router(state: AppState) -> Router {
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|request: &Request<_>| {
            let matched_path = request
                .extensions()
                .get::<MatchedPath>()
                .map(MatchedPath::as_str)
                .unwrap_or(request.uri().path());
            let request_id = request
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("");

            info_span!(
                "http_request",
                method = %request.method(),
                path = %matched_path,
                request_id = %request_id,
                status = tracing::field::Empty,
                latency_ms = tracing::field::Empty,
            )
        })
        .on_request(|_request: &Request<_>, _span: &Span| {
            tracing::debug!("request started");
        })
        .on_response(|response: &Response, latency: Duration, span: &Span| {
            span.record("status", response.status().as_u16());
            span.record("latency_ms", latency.as_millis() as u64);
            tracing::info!("request completed");
        })
        .on_failure(|error: ServerErrorsFailureClass, latency: Duration, span: &Span| {
            span.record("latency_ms", latency.as_millis() as u64);
            tracing::error!(error = %error, "request failed");
        });

    let auth_routes = Router::new()
        .route("/register", post(auth::register))
        .route("/login", post(auth::login))
        .route("/refresh", post(auth::refresh));

    let auth_routes = with_rate_limit(auth_routes, 12, 5);

    let user_routes = Router::new()
        .route("/", get(user::list_users))
        .route(
            "/me",
            get(user::get_me)
                .put(user::update_me)
                .delete(user::delete_me),
        )
        .route("/{id}", get(user::get_user));

    let post_routes = Router::new()
        .route(
            "/",
            get(post_handler::list_posts).post(post_handler::create_post),
        )
        .route(
            "/{id}",
            get(post_handler::get_post)
                .put(post_handler::update_post)
                .delete(post_handler::delete_post),
        )
        .route(
            "/{post_id}/comments",
            get(comment::list_comments).post(comment::create_comment),
        )
        .route(
            "/{post_id}/comments/{comment_id}",
            get(comment::get_comment)
                .put(comment::update_comment)
                .delete(comment::delete_comment),
        );

    let api_routes = Router::new()
        .nest("/auth", auth_routes)
        .nest("/users", user_routes)
        .nest("/posts", post_routes);

    let api_routes = with_rate_limit(api_routes, 1, 60);

    Router::new()
        .route("/health", get(health_check))
        .nest("/api", api_routes)
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(trace_layer)
                .layer(PropagateRequestIdLayer::x_request_id())
                .layer(CorsLayer::permissive()),
        )
        .with_state(state)
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({"status": "ok"}))
}
