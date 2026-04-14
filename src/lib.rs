pub mod auth;
pub mod error;
pub mod handler;
pub mod model;
pub mod route;
pub mod schema;

/// Shared application state — NOT wrapped in Arc (Axum handles it internally).
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::MySqlPool,
    pub jwt_secret: String,
}
