use serde::Deserialize;
use serde::Serialize;
use validator::Validate;

use crate::model::user::UserResponse;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUser {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(email, length(max = 50))]
    pub email: String,
    #[validate(length(min = 6))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUser {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    #[validate(email, length(max = 50))]
    pub email: Option<String>,
    #[validate(length(min = 6))]
    pub password: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginUser {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RefreshRequest {
    #[validate(length(min = 1))]
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserResponse,
}
