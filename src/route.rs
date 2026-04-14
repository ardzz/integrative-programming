use std::time::Duration;

use axum::{
    extract::MatchedPath,
    http::Request,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::{info_span, Span};

use crate::handler::{auth, comment, post as post_handler, user};
use crate::AppState;

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
        .route("/login", post(auth::login));

    let user_routes = Router::new()
        .route("/", get(user::list_users))
        .route(
            "/{id}",
            get(user::get_user)
                .put(user::update_user)
                .delete(user::delete_user),
        );

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

    Router::new()
        .route("/health", get(health_check))
        .nest("/api/auth", auth_routes)
        .nest("/api/users", user_routes)
        .nest("/api/posts", post_routes)
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
