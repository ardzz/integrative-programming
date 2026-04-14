use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum AppError {
    #[error("Not found")]
    NotFound,
    #[error("Validation: {0}")]
    Validation(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            Self::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            Self::Validation(m) => (StatusCode::BAD_REQUEST, m.clone()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            Self::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            Self::Internal(e) => {
                tracing::error!(error = ?e, "Internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Unexpected error".into())
            }
        };
        (status, Json(json!({"error": msg}))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::Database(db_err) => {
                let code = db_err.code().unwrap_or_default();
                match code.as_ref() {
                    "1062" => AppError::Conflict("Duplicate entry".into()),
                    "1451" => AppError::Conflict("Referenced by other records".into()),
                    "1452" => AppError::BadRequest("Referenced record not found".into()),
                    _ => AppError::Internal(anyhow::anyhow!(e)),
                }
            }
            sqlx::Error::RowNotFound => AppError::NotFound,
            _ => AppError::Internal(anyhow::anyhow!(e)),
        }
    }
}
