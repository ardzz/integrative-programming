use axum::{
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::handler::{auth, comment, post as post_handler, user};
use crate::AppState;

pub fn create_router(state: AppState) -> Router {
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
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({"status": "ok"}))
}
