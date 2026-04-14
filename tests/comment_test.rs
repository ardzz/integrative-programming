mod common;

use common::{assert_error_message, create_test_comment, create_test_post, register_user, spawn_app, unique_email};
use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn test_list_comments_returns_200() {
    let app = spawn_app().await;
    let email = unique_email("comment-list");
    let (token, _) = register_user(&app, "Comment Lister", &email, "qwerty").await;
    let post = create_test_post(&app, &token, "Commented Post", "Comment target").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .get(format!("{}/api/posts/{post_id}/comments", app.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
async fn test_create_comment_returns_201() {
    let app = spawn_app().await;
    let email = unique_email("comment-create");
    let (token, _) = register_user(&app, "Comment Creator", &email, "qwerty").await;
    let post = create_test_post(&app, &token, "Comment Post", "Post body").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .post(format!("{}/api/posts/{post_id}/comments", app.base_url))
        .bearer_auth(&token)
        .json(&json!({
            "comment": "Nice post"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["comment"], "Nice post");
    assert_eq!(body["post_id"].as_i64().unwrap(), post_id);
}

#[tokio::test]
async fn test_get_comment_returns_200() {
    let app = spawn_app().await;
    let email = unique_email("comment-get");
    let (token, _) = register_user(&app, "Comment Getter", &email, "qwerty").await;
    let post = create_test_post(&app, &token, "Comment Fetch Post", "Post body").await;
    let post_id = post["id"].as_i64().unwrap() as i32;
    let comment = create_test_comment(&app, &token, post_id, "Fetch me").await;
    let comment_id = comment["id"].as_i64().unwrap();

    let response = app
        .client
        .get(format!("{}/api/posts/{post_id}/comments/{comment_id}", app.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["id"].as_i64().unwrap(), comment_id);
    assert_eq!(body["comment"], "Fetch me");
}

#[tokio::test]
async fn test_update_own_comment_returns_200() {
    let app = spawn_app().await;
    let email = unique_email("comment-update-own");
    let (token, _) = register_user(&app, "Comment Owner", &email, "qwerty").await;
    let post = create_test_post(&app, &token, "Own Comment Post", "Post body").await;
    let post_id = post["id"].as_i64().unwrap() as i32;
    let comment = create_test_comment(&app, &token, post_id, "Before update").await;
    let comment_id = comment["id"].as_i64().unwrap();

    let response = app
        .client
        .put(format!("{}/api/posts/{post_id}/comments/{comment_id}", app.base_url))
        .bearer_auth(&token)
        .json(&json!({
            "comment": "After update"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["comment"], "After update");
}

#[tokio::test]
async fn test_update_others_comment_returns_401() {
    let app = spawn_app().await;
    let owner_email = unique_email("comment-update-other-owner");
    let other_email = unique_email("comment-update-other-actor");
    let (owner_token, _) = register_user(&app, "Comment Owner", &owner_email, "qwerty").await;
    let (other_token, _) = register_user(&app, "Comment Intruder", &other_email, "qwerty").await;
    let post = create_test_post(&app, &owner_token, "Protected Comment Post", "Post body").await;
    let post_id = post["id"].as_i64().unwrap() as i32;
    let comment = create_test_comment(&app, &owner_token, post_id, "Protected comment").await;
    let comment_id = comment["id"].as_i64().unwrap();

    let response = app
        .client
        .put(format!("{}/api/posts/{post_id}/comments/{comment_id}", app.base_url))
        .bearer_auth(&other_token)
        .json(&json!({
            "comment": "Hijacked"
        }))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn test_delete_own_comment_returns_204() {
    let app = spawn_app().await;
    let email = unique_email("comment-delete-own");
    let (token, _) = register_user(&app, "Comment Deleter", &email, "qwerty").await;
    let post = create_test_post(&app, &token, "Comment Delete Post", "Post body").await;
    let post_id = post["id"].as_i64().unwrap() as i32;
    let comment = create_test_comment(&app, &token, post_id, "Delete me").await;
    let comment_id = comment["id"].as_i64().unwrap();

    let response = app
        .client
        .delete(format!("{}/api/posts/{post_id}/comments/{comment_id}", app.base_url))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(response.text().await.unwrap(), "");
}

#[tokio::test]
async fn test_comments_on_nonexistent_post_returns_404() {
    let app = spawn_app().await;

    let response = app
        .client
        .get(format!("{}/api/posts/99999/comments", app.base_url))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn test_create_comment_without_auth_returns_401() {
    let app = spawn_app().await;
    let email = unique_email("comment-no-auth");
    let (token, _) = register_user(&app, "Comment Poster", &email, "qwerty").await;
    let post = create_test_post(&app, &token, "No Auth Comment Post", "Post body").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .post(format!("{}/api/posts/{post_id}/comments", app.base_url))
        .json(&json!({
            "comment": "Anonymous"
        }))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::UNAUTHORIZED).await;
}
