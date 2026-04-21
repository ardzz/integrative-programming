use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tracing::{info, instrument, warn};
use validator::Validate;

use crate::auth::{create_access_token, create_refresh_token, hash_password, verify_password};
use crate::error::AppError;
use crate::model::user::{UserResponse, UserRow};
use crate::schema::user::{AuthResponse, CreateUser, LoginUser};
use crate::AppState;

fn auth_token_ttl_u32(var_name: &str, default: u32) -> u32 {
    std::env::var(var_name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

#[instrument(skip_all)]
pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let hashed = hash_password(&input.password)?;

    let result = sqlx::query("INSERT INTO users (name, email, password) VALUES (?, ?, ?)")
        .bind(&input.name)
        .bind(&input.email)
        .bind(&hashed)
        .execute(&state.db)
        .await?;

    let new_id = result.last_insert_id() as i32;

    let user = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?")
        .bind(new_id)
        .fetch_one(&state.db)
        .await?;

    info!(event = "auth.register.success", user_id = %user.id, "User registered");

    let access_token = create_access_token(
        user.id,
        &state.jwt_secret,
        auth_token_ttl_u32("ACCESS_TOKEN_TTL_MINUTES", 15),
    )?;
    let refresh_token = create_refresh_token(
        user.id,
        &state.jwt_secret,
        auth_token_ttl_u32("REFRESH_TOKEN_TTL_DAYS", 7),
    )?;
    let user_response: UserResponse = user.into();

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            access_token,
            refresh_token,
            user: user_response,
        }),
    ))
}

#[instrument(skip_all)]
pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginUser>,
) -> Result<Json<AuthResponse>, AppError> {
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user = match sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE email = ?")
        .bind(&input.email)
        .fetch_optional(&state.db)
        .await?
    {
        Some(user) => user,
        None => {
            warn!(event = "auth.login.failure", reason = "user_not_found", "Login failed");
            return Err(AppError::Unauthorized);
        }
    };

    let valid = verify_password(&input.password, &user.password)?;
    if !valid {
        warn!(event = "auth.login.failure", reason = "invalid_password", "Login failed");
        return Err(AppError::Unauthorized);
    }

    let access_token = create_access_token(
        user.id,
        &state.jwt_secret,
        auth_token_ttl_u32("ACCESS_TOKEN_TTL_MINUTES", 15),
    )?;
    let refresh_token = create_refresh_token(
        user.id,
        &state.jwt_secret,
        auth_token_ttl_u32("REFRESH_TOKEN_TTL_DAYS", 7),
    )?;
    info!(event = "auth.login.success", user_id = %user.id, "User logged in");
    let user_response: UserResponse = user.into();

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        user: user_response,
    }))
}
