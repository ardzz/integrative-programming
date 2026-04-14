mod common;

use common::{assert_error_message, create_test_post, login_user, register_user, spawn_app, unique_email};
use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn test_list_posts_returns_200() {
    let app = spawn_app().await;

    let response = app
        .client
        .get(format!("{}/api/posts", app.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
async fn test_create_post_returns_201() {
    let app = spawn_app().await;
    let email = unique_email("post-create");
    let (token, _) = register_user(&app, "Post Author", &email, "qwerty").await;

    let response = app
        .client
        .post(format!("{}/api/posts", app.base_url))
        .bearer_auth(&token)
        .json(&json!({
            "title": "First Post",
            "content": "Hello post body",
            "status": "published",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["title"], "First Post");
    assert_eq!(body["status"], "published");
}

#[tokio::test]
async fn test_create_post_without_auth_returns_401() {
    let app = spawn_app().await;

    let response = app
        .client
        .post(format!("{}/api/posts", app.base_url))
        .json(&json!({
            "title": "No Auth",
            "content": "Denied",
            "status": "draft",
        }))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn test_get_post_returns_200() {
    let app = spawn_app().await;
    let email = unique_email("post-get");
    let (token, _) = register_user(&app, "Post Reader", &email, "qwerty").await;
    let post = create_test_post(&app, &token, "Readable Post", "Readable content").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .get(format!("{}/api/posts/{post_id}", app.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["id"].as_i64().unwrap(), post_id);
    assert_eq!(body["title"], "Readable Post");
}

#[tokio::test]
async fn test_get_nonexistent_post_returns_404() {
    let app = spawn_app().await;

    let response = app
        .client
        .get(format!("{}/api/posts/99999", app.base_url))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn test_update_own_post_returns_200() {
    let app = spawn_app().await;
    let email = unique_email("post-update-own");
    let (token, _) = register_user(&app, "Post Owner", &email, "qwerty").await;
    let post = create_test_post(&app, &token, "Old Title", "Old content").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .put(format!("{}/api/posts/{post_id}", app.base_url))
        .bearer_auth(&token)
        .json(&json!({
            "title": "New Title",
            "content": "New content",
            "status": "published",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["title"], "New Title");
    assert_eq!(body["status"], "published");
}

#[tokio::test]
async fn test_update_others_post_returns_401() {
    let app = spawn_app().await;
    let owner_email = unique_email("post-update-other-owner");
    let other_email = unique_email("post-update-other-actor");
    let (owner_token, _) = register_user(&app, "Owner", &owner_email, "qwerty").await;
    let (other_token, _) = register_user(&app, "Other", &other_email, "qwerty").await;
    let post = create_test_post(&app, &owner_token, "Protected Post", "Original content").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .put(format!("{}/api/posts/{post_id}", app.base_url))
        .bearer_auth(&other_token)
        .json(&json!({
            "title": "Hijacked",
            "content": "Hijacked",
            "status": "draft",
        }))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn test_delete_own_post_returns_204() {
    let app = spawn_app().await;
    let email = unique_email("post-delete-own");
    register_user(&app, "Delete Owner", &email, "qwerty").await;
    let token = login_user(&app, &email, "qwerty").await;
    let post = create_test_post(&app, &token, "Delete Me", "To be deleted").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .delete(format!("{}/api/posts/{post_id}", app.base_url))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(response.text().await.unwrap(), "");
}

#[tokio::test]
async fn test_delete_others_post_returns_401() {
    let app = spawn_app().await;
    let owner_email = unique_email("post-delete-other-owner");
    let other_email = unique_email("post-delete-other-actor");
    let (owner_token, _) = register_user(&app, "Delete Owner", &owner_email, "qwerty").await;
    let (other_token, _) = register_user(&app, "Delete Other", &other_email, "qwerty").await;
    let post = create_test_post(&app, &owner_token, "Cant Delete", "Protected").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .delete(format!("{}/api/posts/{post_id}", app.base_url))
        .bearer_auth(&other_token)
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn test_create_post_defaults_status_to_draft() {
    let app = spawn_app().await;
    let email = unique_email("post-default-status");
    let (token, _) = register_user(&app, "Draft Author", &email, "qwerty").await;

    let response = app
        .client
        .post(format!("{}/api/posts", app.base_url))
        .bearer_auth(&token)
        .json(&json!({
            "title": "Default Status",
            "content": "No status provided"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["status"], "draft");
}
