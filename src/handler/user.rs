use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use tracing::{debug, info, instrument};
use validator::Validate;

use crate::auth::{hash_password, AuthUser};
use crate::error::AppError;
use crate::model::user::{UserResponse, UserRow};
use crate::schema::user::UpdateUser;
use crate::AppState;

#[instrument(skip_all)]
pub async fn list_users(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    let users = sqlx::query_as::<_, UserRow>("SELECT * FROM users")
        .fetch_all(&state.db)
        .await?;
    let responses: Vec<UserResponse> = users.into_iter().map(|u| u.into()).collect();
    debug!(event = "user.listed", "Users listed");
    Ok(Json(responses))
}

#[instrument(skip_all)]
pub async fn get_user(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i32>,
) -> Result<Json<UserResponse>, AppError> {
    let user = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    debug!(event = "user.retrieved", user_id = %id, "User retrieved");
    Ok(Json(user.into()))
}

#[instrument(skip_all)]
pub async fn update_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
    Json(input): Json<UpdateUser>,
) -> Result<Json<UserResponse>, AppError> {
    if auth.user_id != id {
        return Err(AppError::Unauthorized);
    }

    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let current = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    let name = input.name.unwrap_or(current.name);
    let email = input.email.unwrap_or(current.email);
    let password = match input.password {
        Some(pw) => hash_password(&pw)?,
        None => current.password,
    };

    sqlx::query("UPDATE users SET name = ?, email = ?, password = ? WHERE id = ?")
        .bind(&name)
        .bind(&email)
        .bind(&password)
        .bind(id)
        .execute(&state.db)
        .await?;

    let updated = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    info!(event = "user.updated", user_id = %id, "User updated");
    Ok(Json(updated.into()))
}

#[instrument(skip_all)]
pub async fn delete_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Result<StatusCode, AppError> {
    if auth.user_id != id {
        return Err(AppError::Unauthorized);
    }

    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    info!(event = "user.deleted", user_id = %id, "User deleted");
    Ok(StatusCode::NO_CONTENT)
}
