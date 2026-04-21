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
    #[error("Forbidden: {0}")]
    Forbidden(String),
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
            Self::NotFound => {
                tracing::warn!(event = "error.not_found", "Resource not found");
                (StatusCode::NOT_FOUND, self.to_string())
            }
            Self::Validation(m) => {
                tracing::warn!(event = "error.validation", detail = %m, "Validation error");
                (StatusCode::BAD_REQUEST, m.clone())
            }
            Self::Unauthorized => {
                tracing::warn!(event = "error.unauthorized", "Unauthorized access attempt");
                (StatusCode::UNAUTHORIZED, self.to_string())
            }
            Self::Forbidden(m) => {
                tracing::warn!(event = "error.forbidden", detail = %m, "Forbidden access attempt");
                (StatusCode::FORBIDDEN, m.clone())
            }
            Self::Conflict(m) => {
                tracing::warn!(event = "error.conflict", detail = %m, "Resource conflict");
                (StatusCode::CONFLICT, m.clone())
            }
            Self::BadRequest(m) => {
                tracing::warn!(event = "error.bad_request", detail = %m, "Bad request");
                (StatusCode::BAD_REQUEST, m.clone())
            }
            Self::Internal(e) => {
                tracing::error!(event = "error.internal", error = ?e, "Internal error");
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
                let message = db_err.message();

                if code.as_ref() == "1062"
                    || message.contains("1062")
                    || message.contains("Duplicate entry")
                {
                    AppError::Conflict("Duplicate entry".into())
                } else if code.as_ref() == "1451"
                    || message.contains("1451")
                    || message.contains("Cannot delete or update a parent row")
                {
                    AppError::Conflict("Referenced by other records".into())
                } else if code.as_ref() == "1452"
                    || message.contains("1452")
                    || message.contains("Cannot add or update a child row")
                {
                    AppError::BadRequest("Referenced record not found".into())
                } else {
                    AppError::Internal(anyhow::anyhow!(e))
                }
            }
            sqlx::Error::RowNotFound => AppError::NotFound,
            _ => AppError::Internal(anyhow::anyhow!(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppError;
    use axum::{http::StatusCode, response::IntoResponse};

    fn status_of(error: AppError) -> StatusCode {
        error.into_response().status()
    }

    #[test]
    fn test_not_found_status() {
        assert_eq!(status_of(AppError::NotFound), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_validation_status() {
        assert_eq!(
            status_of(AppError::Validation("invalid".into())),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn test_unauthorized_status() {
        assert_eq!(status_of(AppError::Unauthorized), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_forbidden_status() {
        assert_eq!(
            status_of(AppError::Forbidden("not owner".into())),
            StatusCode::FORBIDDEN,
        );
    }

    #[test]
    fn test_conflict_status() {
        assert_eq!(
            status_of(AppError::Conflict("duplicate".into())),
            StatusCode::CONFLICT
        );
    }
}
