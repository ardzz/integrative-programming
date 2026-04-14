use chrono::NaiveDateTime;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct PostRow {
    pub id: i32,
    pub title: String,
    pub status: String,
    pub content: String,
    pub user_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct PostResponse {
    pub id: i32,
    pub title: String,
    pub status: String,
    pub content: String,
    pub user_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user_name: Option<String>,
}

impl From<PostRow> for PostResponse {
    fn from(row: PostRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            status: row.status,
            content: row.content,
            user_id: row.user_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            user_name: None,
        }
    }
}
