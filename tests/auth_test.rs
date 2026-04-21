mod common;

use common::{assert_error_message, login_user, register_user, spawn_app, unique_email};
use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn test_register_returns_201_with_token() {
    let app = spawn_app().await;
    let email = unique_email("register-success");

    let response = app
        .client
        .post(format!("{}/api/auth/register", app.base_url))
        .json(&json!({
            "name": "Register User",
            "email": email,
            "password": "qwerty",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["access_token"].as_str().is_some());
    assert!(body["refresh_token"].as_str().is_some());
    assert!(body["user"]["id"].as_i64().is_some());
    assert_eq!(body["user"]["name"], "Register User");
    assert_eq!(body["user"]["email"], email);
    assert!(body["user"].get("password").is_none());
}

#[tokio::test]
async fn test_register_returns_both_tokens() {
    let app = spawn_app().await;
    let email = unique_email("register-both");

    let response = app
        .client
        .post(format!("{}/api/auth/register", app.base_url))
        .json(&json!({
            "name": "Register Both Tokens",
            "email": email,
            "password": "qwerty",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["access_token"].as_str().is_some_and(|token| token.len() > 20));
    assert!(body["refresh_token"].as_str().is_some_and(|token| token.len() > 20));
}

#[tokio::test]
async fn test_register_duplicate_email_returns_409() {
    let app = spawn_app().await;
    let email = unique_email("register-duplicate");

    register_user(&app, "First User", &email, "qwerty").await;

    let response = app
        .client
        .post(format!("{}/api/auth/register", app.base_url))
        .json(&json!({
            "name": "Second User",
            "email": email,
            "password": "qwerty",
        }))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::CONFLICT).await;
}

#[tokio::test]
async fn test_login_returns_200_with_token() {
    let app = spawn_app().await;
    let email = unique_email("login-success");
    register_user(&app, "Login User", &email, "qwerty").await;

    let response = app
        .client
        .post(format!("{}/api/auth/login", app.base_url))
        .json(&json!({
            "email": email,
            "password": "qwerty",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["access_token"].as_str().is_some());
    assert!(body["refresh_token"].as_str().is_some());
    assert_eq!(body["user"]["email"], email);
    assert!(body["user"].get("password").is_none());
}

#[tokio::test]
async fn test_login_returns_both_tokens() {
    let app = spawn_app().await;
    let email = unique_email("login-both");
    register_user(&app, "Login Both Tokens", &email, "qwerty").await;

    let tokens = login_user(&app, &email, "qwerty").await;

    assert!(tokens.access.len() > 20);
    assert!(tokens.refresh.len() > 20);
}

#[tokio::test]
async fn test_refresh_with_valid_refresh_token_returns_new_pair() {
    let app = spawn_app().await;
    let email = unique_email("refresh-valid");
    let (tokens, _) = register_user(&app, "Refresh Valid", &email, "qwerty").await;

    let response = app
        .client
        .post(format!("{}/api/auth/refresh", app.base_url))
        .json(&json!({
            "refresh_token": tokens.refresh,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let access_token = body["access_token"].as_str().expect("access_token should be present");
    let refresh_token = body["refresh_token"].as_str().expect("refresh_token should be present");

    assert!(access_token.len() > 20);
    assert!(refresh_token.len() > 20);
    assert_ne!(access_token, tokens.access);
    assert_ne!(refresh_token, tokens.refresh);
}

#[tokio::test]
async fn test_refresh_with_access_token_returns_401() {
    let app = spawn_app().await;
    let email = unique_email("refresh-access");
    let (tokens, _) = register_user(&app, "Refresh Access", &email, "qwerty").await;

    let response = app
        .client
        .post(format!("{}/api/auth/refresh", app.base_url))
        .json(&json!({
            "refresh_token": tokens.access,
        }))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn test_refresh_with_empty_string_returns_400() {
    let app = spawn_app().await;

    let response = app
        .client
        .post(format!("{}/api/auth/refresh", app.base_url))
        .json(&json!({
            "refresh_token": "",
        }))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn test_login_invalid_credentials_returns_401() {
    let app = spawn_app().await;
    let email = unique_email("login-invalid");
    register_user(&app, "Login Invalid", &email, "qwerty").await;

    let response = app
        .client
        .post(format!("{}/api/auth/login", app.base_url))
        .json(&json!({
            "email": email,
            "password": "wrong-password",
        }))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn test_register_invalid_email_returns_400() {
    let app = spawn_app().await;

    let response = app
        .client
        .post(format!("{}/api/auth/register", app.base_url))
        .json(&json!({
            "name": "Bad Email",
            "email": "not-an-email",
            "password": "qwerty",
        }))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn test_register_short_password_returns_400() {
    let app = spawn_app().await;
    let email = unique_email("register-short-password");

    let response = app
        .client
        .post(format!("{}/api/auth/register", app.base_url))
        .json(&json!({
            "name": "Short Password",
            "email": email,
            "password": "abc",
        }))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::BAD_REQUEST).await;
}
