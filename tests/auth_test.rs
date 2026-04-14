mod common;

use common::{assert_error_message, register_user, spawn_app, unique_email};
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
    assert!(body["token"].as_str().is_some());
    assert!(body["user"]["id"].as_i64().is_some());
    assert_eq!(body["user"]["name"], "Register User");
    assert_eq!(body["user"]["email"], email);
    assert!(body["user"].get("password").is_none());
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
    assert!(body["token"].as_str().is_some());
    assert_eq!(body["user"]["email"], email);
    assert!(body["user"].get("password").is_none());
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
