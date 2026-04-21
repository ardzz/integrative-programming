use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use tracing::{debug, info, instrument};
use validator::Validate;

use crate::auth::{hash_password, AuthUser};
use crate::error::AppError;
use crate::model::pagination::Paginated;
use crate::model::user::{UserResponse, UserRow};
use crate::schema::pagination::PaginationQuery;
use crate::schema::user::UpdateUser;
use crate::AppState;

#[instrument(skip_all)]
pub async fn list_users(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
    _auth: AuthUser,
) -> Result<Json<Paginated<UserResponse>>, AppError> {
    pagination
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;

    let users = sqlx::query_as::<_, UserRow>("SELECT * FROM users LIMIT ? OFFSET ?")
        .bind(pagination.per_page())
        .bind(pagination.offset())
        .fetch_all(&state.db)
        .await?;
    let responses: Vec<UserResponse> = users.into_iter().map(|u| u.into()).collect();
    debug!(event = "user.listed", "Users listed");
    Ok(Json(Paginated::new(
        responses,
        pagination.page(),
        pagination.per_page(),
        total.0 as u64,
    )))
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
pub async fn get_me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<UserResponse>, AppError> {
    let user = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?")
        .bind(auth.user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    debug!(event = "user.me.fetched", user_id = %auth.user_id, "Current user fetched");
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
        return Err(AppError::Forbidden(
            "You can only modify your own account".into(),
        ));
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
pub async fn update_me(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<UpdateUser>,
) -> Result<Json<UserResponse>, AppError> {
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let current = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?")
        .bind(auth.user_id)
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
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    let updated = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?")
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

    info!(event = "user.me.updated", user_id = %auth.user_id, "Current user updated");
    Ok(Json(updated.into()))
}

#[instrument(skip_all)]
pub async fn delete_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Result<StatusCode, AppError> {
    if auth.user_id != id {
        return Err(AppError::Forbidden(
            "You can only modify your own account".into(),
        ));
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

#[instrument(skip_all)]
pub async fn delete_me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    info!(event = "user.me.deleted", user_id = %auth.user_id, "Current user deleted");
    Ok(StatusCode::NO_CONTENT)
}
