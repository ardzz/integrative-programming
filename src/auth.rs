use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use tracing::{debug, instrument, warn};

use crate::error::AppError;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub token_type: String,
    pub jti: String,
}

#[allow(dead_code)]
#[instrument(skip_all)]
pub fn create_access_token(
    user_id: i32,
    secret: &str,
    ttl_minutes: u32,
) -> Result<String, AppError> {
    let now = chrono::Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::seconds((ttl_minutes as i64) * 60)).timestamp() as usize,
        token_type: "access".to_string(),
        jti: uuid::Uuid::new_v4().to_string(),
    };

    let token = encode_jwt(&claims, secret)?;
    debug!(
        event = "auth.token.created",
        user_id = %user_id,
        token_type = %claims.token_type,
        "JWT token created"
    );
    Ok(token)
}

#[allow(dead_code)]
#[instrument(skip_all)]
pub fn create_refresh_token(user_id: i32, secret: &str, ttl_days: u32) -> Result<String, AppError> {
    let now = chrono::Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::seconds((ttl_days as i64) * 86_400)).timestamp() as usize,
        token_type: "refresh".to_string(),
        jti: uuid::Uuid::new_v4().to_string(),
    };

    let token = encode_jwt(&claims, secret)?;
    debug!(
        event = "auth.token.created",
        user_id = %user_id,
        token_type = %claims.token_type,
        "JWT token created"
    );
    Ok(token)
}

#[instrument(skip_all)]
fn encode_jwt(claims: &Claims, secret: &str) -> Result<String, AppError> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
}

#[allow(dead_code)]
#[instrument(skip_all)]
pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| {
        warn!(event = "auth.token.invalid", "Token verification failed");
        AppError::Unauthorized
    })?;
    debug!(event = "auth.token.verified", user_id = %token_data.claims.sub, "Token verified");
    Ok(token_data.claims)
}

#[allow(dead_code)]
#[instrument(skip_all)]
pub fn verify_refresh_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let claims = verify_token(token, secret)?;

    if claims.token_type != "refresh" {
        warn!(
            event = "auth.token.wrong_type",
            expected = "refresh",
            actual = %claims.token_type,
            "Token type mismatch"
        );
        return Err(AppError::Unauthorized);
    }

    Ok(claims)
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct AuthUser {
    pub user_id: i32,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;

        let claims = verify_token(token, &state.jwt_secret)?;
        if claims.token_type != "access" {
            warn!(
                event = "auth.token.wrong_type",
                expected = "access",
                actual = %claims.token_type,
                "Token type mismatch"
            );
            return Err(AppError::Unauthorized);
        }
        let user_id: i32 = claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
        Ok(AuthUser { user_id })
    }
}

#[allow(dead_code)]
#[instrument(skip_all)]
pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(hash.to_string())
}

#[allow(dead_code)]
#[instrument(skip_all)]
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::{
        create_access_token, create_refresh_token, hash_password, verify_password,
        verify_refresh_token, verify_token, AuthUser,
    };
    use crate::{error::AppError, AppState};
    use axum::extract::FromRequestParts;
    use axum::http::header::AUTHORIZATION;
    use axum::http::Request;
    use sqlx::MySqlPool;

    #[test]
    fn test_hash_and_verify_password() {
        let hash = hash_password("qwerty").expect("hash should succeed");
        let verified = verify_password("qwerty", &hash).expect("verify should succeed");

        assert!(verified);
    }

    #[test]
    fn test_verify_wrong_password() {
        let hash = hash_password("qwerty").expect("hash should succeed");
        let verified = verify_password("wrong", &hash).expect("verify should succeed");

        assert!(!verified);
    }

    #[test]
    fn test_create_access_token_has_access_type() {
        let token =
            create_access_token(1, "test-secret", 15).expect("token creation should succeed");
        let claims = verify_token(&token, "test-secret").expect("token should verify");

        assert_eq!(claims.sub, "1");
        assert!(claims.exp >= claims.iat);
        assert_eq!(claims.token_type, "access");
    }

    #[test]
    fn test_create_refresh_token_has_refresh_type() {
        let token =
            create_refresh_token(1, "test-secret", 7).expect("token creation should succeed");
        let claims = verify_token(&token, "test-secret").expect("token should verify");

        assert_eq!(claims.sub, "1");
        assert!(claims.exp >= claims.iat);
        assert_eq!(claims.token_type, "refresh");
    }

    #[test]
    fn test_access_token_rejected_by_refresh_verifier() {
        let token =
            create_access_token(1, "test-secret", 15).expect("token creation should succeed");

        let result = verify_refresh_token(&token, "test-secret");

        assert!(matches!(result, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn test_refresh_token_rejected_by_authuser() {
        let token =
            create_refresh_token(1, "test-secret", 7).expect("token creation should succeed");
        let state = AppState {
            db: MySqlPool::connect_lazy("mysql://test:test@localhost/test")
                .expect("lazy pool should build"),
            jwt_secret: "test-secret".to_string(),
        };
        let request = Request::builder()
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(())
            .expect("request should build");
        let (mut parts, _) = request.into_parts();

        let result = AuthUser::from_request_parts(&mut parts, &state).await;

        assert!(matches!(result, Err(AppError::Unauthorized)));
    }

    #[test]
    fn test_verify_invalid_token() {
        let result = verify_token("garbage-token", "test-secret");

        assert!(result.is_err());
    }
}
