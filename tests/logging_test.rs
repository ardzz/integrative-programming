mod common;

use common::{create_test_post, register_user, spawn_app, unique_email};
use serde_json::json;
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn test_login_success_emits_event() {
    let app = spawn_app().await;
    let email = unique_email("log-logok");
    register_user(&app, "LogTest", &email, "qwerty").await;

    app.client
        .post(format!("{}/api/auth/login", app.base_url))
        .json(&json!({"email": email, "password": "qwerty"}))
        .send()
        .await
        .unwrap();

    assert!(logs_contain("auth.login.success"));
}

#[tokio::test]
#[traced_test]
async fn test_login_failure_emits_event() {
    let app = spawn_app().await;

    app.client
        .post(format!("{}/api/auth/login", app.base_url))
        .json(&json!({"email": "nonexistent@test.com", "password": "wrong"}))
        .send()
        .await
        .unwrap();

    assert!(logs_contain("auth.login.failure"));
}

#[tokio::test]
#[traced_test]
async fn test_post_created_emits_event() {
    let app = spawn_app().await;
    let email = unique_email("log-post");
    let (tokens, _) = register_user(&app, "PostLogger", &email, "qwerty").await;

    create_test_post(&app, &tokens.access, "Smoke Test Post", "Content").await;

    assert!(logs_contain("post.created"));
}

#[tokio::test]
#[traced_test]
async fn test_not_found_emits_warn() {
    let app = spawn_app().await;

    app.client
        .get(format!("{}/api/posts/99999", app.base_url))
        .send()
        .await
        .unwrap();

    assert!(logs_contain("error.not_found"));
}

#[tokio::test]
#[traced_test]
async fn test_register_emits_event() {
    let app = spawn_app().await;
    let email = unique_email("log-regis");

    register_user(&app, "RegisterLogger", &email, "qwerty").await;

    assert!(logs_contain("auth.register.success"));
}
