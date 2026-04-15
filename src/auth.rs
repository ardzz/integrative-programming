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
}

#[allow(dead_code)]
#[instrument(skip_all)]
pub fn create_token(user_id: i32, secret: &str) -> Result<String, AppError> {
    let now = chrono::Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::hours(24)).timestamp() as usize,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    debug!(event = "auth.token.created", user_id = %user_id, "JWT token created");
    Ok(token)
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
    use super::{create_token, hash_password, verify_password, verify_token};

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
    fn test_create_and_verify_token() {
        let token = create_token(1, "test-secret").expect("token creation should succeed");
        let claims = verify_token(&token, "test-secret").expect("token should verify");

        assert_eq!(claims.sub, "1");
        assert!(claims.exp >= claims.iat);
    }

    #[test]
    fn test_verify_invalid_token() {
        let result = verify_token("garbage-token", "test-secret");

        assert!(result.is_err());
    }
}
