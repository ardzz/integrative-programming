use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use tracing::{debug, info, instrument};
use validator::Validate;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::model::comment::CommentResponse;
use crate::model::pagination::Paginated;
use crate::schema::comment::{CreateComment, UpdateComment};
use crate::schema::pagination::PaginationQuery;
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

#[instrument(skip_all)]
async fn ensure_post_exists(pool: &sqlx::MySqlPool, post_id: i32) -> Result<(), AppError> {
    debug!(event = "comment.post_check", post_id = %post_id, "Checking post exists");
    let result = sqlx::query("SELECT id FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_optional(pool)
        .await?;
    if result.is_none() {
        return Err(AppError::NotFound);
    }
    Ok(())
}

#[instrument(skip_all)]
pub async fn list_comments(
    State(state): State<AppState>,
    Path(post_id): Path<i32>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<Paginated<CommentResponse>>, AppError> {
    ensure_post_exists(&state.db, post_id).await?;

    pagination
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let page = pagination.page();
    let per_page = pagination.per_page();
    let offset = pagination.offset();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE post_id = ?")
        .bind(post_id)
        .fetch_one(&state.db)
        .await?;

    let comments = sqlx::query_as::<_, CommentWithUser>(
        "SELECT c.*, u.name as user_name FROM comments c JOIN users u ON c.user_id = u.id WHERE c.post_id = ? LIMIT ? OFFSET ?",
    )
    .bind(post_id)
    .bind(per_page as i64)
    .bind(offset as i64)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<CommentResponse> = comments.into_iter().map(|c| c.into()).collect();
    debug!(event = "comment.listed", post_id = %post_id, page = %page, per_page = %per_page, total = %total, "Comments listed");
    Ok(Json(Paginated::new(data, page, per_page, total as u64)))
}

#[instrument(skip_all)]
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
    info!(event = "comment.created", comment_id = %new_id, post_id = %post_id, author_id = %auth.user_id, "Comment created");

    let comment = sqlx::query_as::<_, CommentWithUser>(
        "SELECT c.*, u.name as user_name FROM comments c JOIN users u ON c.user_id = u.id WHERE c.id = ?",
    )
    .bind(new_id)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(comment.into())))
}

#[instrument(skip_all)]
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

    debug!(event = "comment.retrieved", comment_id = %comment_id, "Comment retrieved");
    Ok(Json(comment.into()))
}

#[instrument(skip_all)]
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
        return Err(AppError::Forbidden(
            "You can only modify your own comments".into(),
        ));
    }

    sqlx::query("UPDATE comments SET comment = ? WHERE id = ?")
        .bind(&input.comment)
        .bind(comment_id)
        .execute(&state.db)
        .await?;

    info!(event = "comment.updated", comment_id = %comment_id, author_id = %auth.user_id, "Comment updated");

    let updated = sqlx::query_as::<_, CommentWithUser>(
        "SELECT c.*, u.name as user_name FROM comments c JOIN users u ON c.user_id = u.id WHERE c.id = ?",
    )
    .bind(comment_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(updated.into()))
}

#[instrument(skip_all)]
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
        return Err(AppError::Forbidden(
            "You can only modify your own comments".into(),
        ));
    }

    sqlx::query("DELETE FROM comments WHERE id = ?")
        .bind(comment_id)
        .execute(&state.db)
        .await?;

    info!(event = "comment.deleted", comment_id = %comment_id, author_id = %auth.user_id, "Comment deleted");
    Ok(StatusCode::NO_CONTENT)
}
