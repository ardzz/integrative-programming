use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use validator::Validate;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::model::comment::CommentResponse;
use crate::schema::comment::{CreateComment, UpdateComment};
use crate::AppState;

#[derive(sqlx::FromRow)]
struct CommentWithUser {
    id: i32,
    comment: String,
    post_id: i32,
    user_id: i32,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
    user_name: Option<String>,
}

impl From<CommentWithUser> for CommentResponse {
    fn from(row: CommentWithUser) -> Self {
        CommentResponse {
            id: row.id,
            comment: row.comment,
            post_id: row.post_id,
            user_id: row.user_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            user_name: row.user_name,
        }
    }
}

async fn ensure_post_exists(pool: &sqlx::MySqlPool, post_id: i32) -> Result<(), AppError> {
    let result = sqlx::query("SELECT id FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_optional(pool)
        .await?;
    if result.is_none() {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn list_comments(
    State(state): State<AppState>,
    Path(post_id): Path<i32>,
) -> Result<Json<Vec<CommentResponse>>, AppError> {
    ensure_post_exists(&state.db, post_id).await?;

    let comments = sqlx::query_as::<_, CommentWithUser>(
        "SELECT c.*, u.name as user_name FROM comments c JOIN users u ON c.user_id = u.id WHERE c.post_id = ?",
    )
    .bind(post_id)
    .fetch_all(&state.db)
    .await?;

    let responses: Vec<CommentResponse> = comments.into_iter().map(|c| c.into()).collect();
    Ok(Json(responses))
}

pub async fn create_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<i32>,
    Json(input): Json<CreateComment>,
) -> Result<(StatusCode, Json<CommentResponse>), AppError> {
    ensure_post_exists(&state.db, post_id).await?;
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let result = sqlx::query("INSERT INTO comments (comment, post_id, user_id) VALUES (?, ?, ?)")
        .bind(&input.comment)
        .bind(post_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    let new_id = result.last_insert_id() as i32;

    let comment = sqlx::query_as::<_, CommentWithUser>(
        "SELECT c.*, u.name as user_name FROM comments c JOIN users u ON c.user_id = u.id WHERE c.id = ?",
    )
    .bind(new_id)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(comment.into())))
}

pub async fn get_comment(
    State(state): State<AppState>,
    Path((post_id, comment_id)): Path<(i32, i32)>,
) -> Result<Json<CommentResponse>, AppError> {
    let comment = sqlx::query_as::<_, CommentWithUser>(
        "SELECT c.*, u.name as user_name FROM comments c JOIN users u ON c.user_id = u.id WHERE c.id = ? AND c.post_id = ?",
    )
    .bind(comment_id)
    .bind(post_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(comment.into()))
}

pub async fn update_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((post_id, comment_id)): Path<(i32, i32)>,
    Json(input): Json<UpdateComment>,
) -> Result<Json<CommentResponse>, AppError> {
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let current = sqlx::query_as::<_, CommentWithUser>(
        "SELECT c.*, u.name as user_name FROM comments c JOIN users u ON c.user_id = u.id WHERE c.id = ? AND c.post_id = ?",
    )
    .bind(comment_id)
    .bind(post_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    if current.user_id != auth.user_id {
        return Err(AppError::Unauthorized);
    }

    sqlx::query("UPDATE comments SET comment = ? WHERE id = ?")
        .bind(&input.comment)
        .bind(comment_id)
        .execute(&state.db)
        .await?;

    let updated = sqlx::query_as::<_, CommentWithUser>(
        "SELECT c.*, u.name as user_name FROM comments c JOIN users u ON c.user_id = u.id WHERE c.id = ?",
    )
    .bind(comment_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(updated.into()))
}

pub async fn delete_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((post_id, comment_id)): Path<(i32, i32)>,
) -> Result<StatusCode, AppError> {
    let current = sqlx::query_as::<_, CommentWithUser>(
        "SELECT c.*, u.name as user_name FROM comments c JOIN users u ON c.user_id = u.id WHERE c.id = ? AND c.post_id = ?",
    )
    .bind(comment_id)
    .bind(post_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    if current.user_id != auth.user_id {
        return Err(AppError::Unauthorized);
    }

    sqlx::query("DELETE FROM comments WHERE id = ?")
        .bind(comment_id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
