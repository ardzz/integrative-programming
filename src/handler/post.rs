use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use tracing::instrument;
use validator::Validate;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::model::pagination::Paginated;
use crate::model::post::PostResponse;
use crate::schema::pagination::PaginationQuery;
use crate::schema::post::{CreatePost, UpdatePost};
use crate::AppState;

#[derive(sqlx::FromRow)]
struct PostWithUser {
    id: i32,
    title: String,
    status: String,
    content: String,
    user_id: i32,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
    user_name: Option<String>,
}

impl From<PostWithUser> for PostResponse {
    fn from(row: PostWithUser) -> Self {
        PostResponse {
            id: row.id,
            title: row.title,
            status: row.status,
            content: row.content,
            user_id: row.user_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            user_name: row.user_name,
        }
    }
}

#[instrument(skip_all)]
pub async fn list_posts(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<Paginated<PostResponse>>, AppError> {
    pagination
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let page = pagination.page();
    let per_page = pagination.per_page();
    let offset = pagination.offset();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts")
        .fetch_one(&state.db)
        .await?;

    let posts = sqlx::query_as::<_, PostWithUser>(
        "SELECT p.*, u.name as user_name FROM posts p JOIN users u ON p.user_id = u.id LIMIT ? OFFSET ?",
    )
    .bind(per_page as i64)
    .bind(offset as i64)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<PostResponse> = posts.into_iter().map(|p| p.into()).collect();
    tracing::debug!(event = "post.listed", page = %page, per_page = %per_page, total = %total, "Posts listed");
    Ok(Json(Paginated::new(data, page, per_page, total as u64)))
}

#[instrument(skip_all)]
pub async fn create_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreatePost>,
) -> Result<(StatusCode, Json<PostResponse>), AppError> {
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let status = input.status.unwrap_or_else(|| "draft".to_string());
    if status != "draft" && status != "published" {
        return Err(AppError::Validation(
            "Status must be 'draft' or 'published'".into(),
        ));
    }

    let result = sqlx::query(
        "INSERT INTO posts (title, status, content, user_id) VALUES (?, ?, ?, ?)",
    )
    .bind(&input.title)
    .bind(&status)
    .bind(&input.content)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    let new_id = result.last_insert_id() as i32;
    tracing::info!(event = "post.created", post_id = %new_id, author_id = %auth.user_id, "Post created");

    let post = sqlx::query_as::<_, PostWithUser>(
        "SELECT p.*, u.name as user_name FROM posts p JOIN users u ON p.user_id = u.id WHERE p.id = ?",
    )
    .bind(new_id)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(post.into())))
}

#[instrument(skip_all)]
pub async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<PostResponse>, AppError> {
    let post = sqlx::query_as::<_, PostWithUser>(
        "SELECT p.*, u.name as user_name FROM posts p JOIN users u ON p.user_id = u.id WHERE p.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    tracing::debug!(event = "post.retrieved", post_id = %id, "Post retrieved");
    Ok(Json(post.into()))
}

#[instrument(skip_all)]
pub async fn update_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
    Json(input): Json<UpdatePost>,
) -> Result<Json<PostResponse>, AppError> {
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let current = sqlx::query_as::<_, PostWithUser>(
        "SELECT p.*, u.name as user_name FROM posts p JOIN users u ON p.user_id = u.id WHERE p.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    if current.user_id != auth.user_id {
        return Err(AppError::Forbidden(
            "You can only modify your own posts".into(),
        ));
    }

    let title = input.title.unwrap_or(current.title);
    let content = input.content.unwrap_or(current.content);
    let status = input.status.unwrap_or(current.status);

    if status != "draft" && status != "published" {
        return Err(AppError::Validation(
            "Status must be 'draft' or 'published'".into(),
        ));
    }

    sqlx::query("UPDATE posts SET title = ?, content = ?, status = ? WHERE id = ?")
        .bind(&title)
        .bind(&content)
        .bind(&status)
        .bind(id)
        .execute(&state.db)
        .await?;

    tracing::info!(event = "post.updated", post_id = %id, author_id = %auth.user_id, "Post updated");

    let updated = sqlx::query_as::<_, PostWithUser>(
        "SELECT p.*, u.name as user_name FROM posts p JOIN users u ON p.user_id = u.id WHERE p.id = ?",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(updated.into()))
}

#[instrument(skip_all)]
pub async fn delete_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Result<StatusCode, AppError> {
    let post = sqlx::query_as::<_, PostWithUser>(
        "SELECT p.*, u.name as user_name FROM posts p JOIN users u ON p.user_id = u.id WHERE p.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    if post.user_id != auth.user_id {
        return Err(AppError::Forbidden(
            "You can only modify your own posts".into(),
        ));
    }

    sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    tracing::info!(event = "post.deleted", post_id = %id, author_id = %auth.user_id, "Post deleted");
    Ok(StatusCode::NO_CONTENT)
}
