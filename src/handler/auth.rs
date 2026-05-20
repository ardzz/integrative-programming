use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tracing::{info, instrument, warn};
use validator::Validate;

use crate::auth::{
    create_access_token, create_refresh_token, hash_password, verify_password, verify_refresh_token,
};
use crate::error::AppError;
use crate::model::user::{UserResponse, UserRow};
use crate::schema::user::{AuthResponse, CreateUser, LoginUser, RefreshRequest};
use crate::AppState;

fn auth_token_ttl_u32(var_name: &str, default: u32) -> u32 {
    std::env::var(var_name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn ttl_minutes_from_env() -> u32 {
    auth_token_ttl_u32("ACCESS_TOKEN_TTL_MINUTES", 15)
}

fn ttl_days_from_env() -> u32 {
    auth_token_ttl_u32("REFRESH_TOKEN_TTL_DAYS", 7)
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

    let access_token = create_access_token(user.id, &state.jwt_secret, ttl_minutes_from_env())?;
    let refresh_token = create_refresh_token(user.id, &state.jwt_secret, ttl_days_from_env())?;
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
            warn!(
                event = "auth.login.failure",
                reason = "user_not_found",
                "Login failed"
            );
            return Err(AppError::Unauthorized);
        }
    };

    let valid = verify_password(&input.password, &user.password)?;
    if !valid {
        warn!(
            event = "auth.login.failure",
            reason = "invalid_password",
            "Login failed"
        );
        return Err(AppError::Unauthorized);
    }

    let access_token = create_access_token(user.id, &state.jwt_secret, ttl_minutes_from_env())?;
    let refresh_token = create_refresh_token(user.id, &state.jwt_secret, ttl_days_from_env())?;
    info!(event = "auth.login.success", user_id = %user.id, "User logged in");
    let user_response: UserResponse = user.into();

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        user: user_response,
    }))
}

#[instrument(skip_all)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(input): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let claims = verify_refresh_token(&input.refresh_token, &state.jwt_secret)?;
    let user_id: i32 = claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
    let user = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let access = create_access_token(user_id, &state.jwt_secret, ttl_minutes_from_env())?;
    let refresh = create_refresh_token(user_id, &state.jwt_secret, ttl_days_from_env())?;

    info!(event = "auth.refresh.success", user_id = %user_id, "Token refreshed");

    Ok(Json(AuthResponse {
        access_token: access,
        refresh_token: refresh,
        user: user.into(),
    }))
}
