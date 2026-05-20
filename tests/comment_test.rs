mod common;

use common::{
    assert_error_message, create_test_comment, create_test_post, register_user, spawn_app,
    unique_email,
};
use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn test_list_comments_returns_200() {
    let app = spawn_app().await;
    let email = unique_email("comment-list");
    let (tokens, _) = register_user(&app, "Comment Lister", &email, "qwerty").await;
    let post = create_test_post(&app, &tokens.access, "Commented Post", "Comment target").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .get(app.api_path(&format!("/posts/{post_id}/comments")))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["data"].is_array());
    assert!(body["meta"].is_object());
}

#[tokio::test]
async fn test_create_comment_returns_201() {
    let app = spawn_app().await;
    let email = unique_email("comment-create");
    let (tokens, _) = register_user(&app, "Comment Creator", &email, "qwerty").await;
    let post = create_test_post(&app, &tokens.access, "Comment Post", "Post body").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .post(app.api_path(&format!("/posts/{post_id}/comments")))
        .bearer_auth(&tokens.access)
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
    let (tokens, _) = register_user(&app, "Comment Getter", &email, "qwerty").await;
    let post = create_test_post(&app, &tokens.access, "Comment Fetch Post", "Post body").await;
    let post_id = post["id"].as_i64().unwrap() as i32;
    let comment = create_test_comment(&app, &tokens.access, post_id, "Fetch me").await;
    let comment_id = comment["id"].as_i64().unwrap();

    let response = app
        .client
        .get(app.api_path(&format!("/posts/{post_id}/comments/{comment_id}")))
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
    let (tokens, _) = register_user(&app, "Comment Owner", &email, "qwerty").await;
    let post = create_test_post(&app, &tokens.access, "Own Comment Post", "Post body").await;
    let post_id = post["id"].as_i64().unwrap() as i32;
    let comment = create_test_comment(&app, &tokens.access, post_id, "Before update").await;
    let comment_id = comment["id"].as_i64().unwrap();

    let response = app
        .client
        .put(app.api_path(&format!("/posts/{post_id}/comments/{comment_id}")))
        .bearer_auth(&tokens.access)
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
async fn test_update_others_comment_returns_403() {
    let app = spawn_app().await;
    let owner_email = unique_email("comment-update-other-owner");
    let other_email = unique_email("comment-update-other-actor");
    let (owner_tokens, _) = register_user(&app, "Comment Owner", &owner_email, "qwerty").await;
    let (other_tokens, _) = register_user(&app, "Comment Intruder", &other_email, "qwerty").await;
    let post = create_test_post(
        &app,
        &owner_tokens.access,
        "Protected Comment Post",
        "Post body",
    )
    .await;
    let post_id = post["id"].as_i64().unwrap() as i32;
    let comment =
        create_test_comment(&app, &owner_tokens.access, post_id, "Protected comment").await;
    let comment_id = comment["id"].as_i64().unwrap();

    let response = app
        .client
        .put(app.api_path(&format!("/posts/{post_id}/comments/{comment_id}")))
        .bearer_auth(&other_tokens.access)
        .json(&json!({
            "comment": "Hijacked"
        }))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::FORBIDDEN).await;
}

#[tokio::test]
async fn test_delete_others_comment_returns_403() {
    let app = spawn_app().await;
    let owner_email = unique_email("comment-delete-other-owner");
    let other_email = unique_email("comment-delete-other-actor");
    let (owner_tokens, _) = register_user(&app, "Comment Owner", &owner_email, "qwerty").await;
    let (other_tokens, _) = register_user(&app, "Comment Intruder", &other_email, "qwerty").await;
    let post = create_test_post(
        &app,
        &owner_tokens.access,
        "Protected Delete Post",
        "Post body",
    )
    .await;
    let post_id = post["id"].as_i64().unwrap() as i32;
    let comment = create_test_comment(&app, &owner_tokens.access, post_id, "Protected").await;
    let comment_id = comment["id"].as_i64().unwrap();

    let response = app
        .client
        .delete(app.api_path(&format!("/posts/{post_id}/comments/{comment_id}")))
        .bearer_auth(&other_tokens.access)
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::FORBIDDEN).await;
}

#[tokio::test]
async fn test_delete_own_comment_returns_204() {
    let app = spawn_app().await;
    let email = unique_email("comment-delete-own");
    let (tokens, _) = register_user(&app, "Comment Deleter", &email, "qwerty").await;
    let post = create_test_post(&app, &tokens.access, "Comment Delete Post", "Post body").await;
    let post_id = post["id"].as_i64().unwrap() as i32;
    let comment = create_test_comment(&app, &tokens.access, post_id, "Delete me").await;
    let comment_id = comment["id"].as_i64().unwrap();

    let response = app
        .client
        .delete(app.api_path(&format!("/posts/{post_id}/comments/{comment_id}")))
        .bearer_auth(&tokens.access)
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
        .get(app.api_path("/posts/99999/comments"))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn test_create_comment_without_auth_returns_401() {
    let app = spawn_app().await;
    let email = unique_email("comment-no-auth");
    let (tokens, _) = register_user(&app, "Comment Poster", &email, "qwerty").await;
    let post = create_test_post(&app, &tokens.access, "No Auth Comment Post", "Post body").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .post(app.api_path(&format!("/posts/{post_id}/comments")))
        .json(&json!({
            "comment": "Anonymous"
        }))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn test_list_comments_paginated() {
    let app = spawn_app().await;
    let author_email = unique_email("cmt-page-author");
    let commenter_a_email = unique_email("cmt-page-a");
    let commenter_b_email = unique_email("cmt-page-b");
    let (author_tokens, _) = register_user(&app, "Post Author", &author_email, "qwerty").await;
    let (a_tokens, _) = register_user(&app, "Commenter A", &commenter_a_email, "qwerty").await;
    let (b_tokens, _) = register_user(&app, "Commenter B", &commenter_b_email, "qwerty").await;

    let post = create_test_post(&app, &author_tokens.access, "Paginated Post", "Body").await;
    let post_id = post["id"].as_i64().unwrap() as i32;

    create_test_comment(&app, &author_tokens.access, post_id, "first").await;
    create_test_comment(&app, &a_tokens.access, post_id, "second").await;
    create_test_comment(&app, &b_tokens.access, post_id, "third").await;

    let response = app
        .client
        .get(app.api_path(&format!("/posts/{post_id}/comments?per_page=2")))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let data = body["data"].as_array().expect("data should be an array");
    assert_eq!(data.len(), 2);
    assert!(body["meta"]["total"].as_u64().unwrap() >= 3);
    assert_eq!(body["meta"]["per_page"].as_u64().unwrap(), 2);
}

#[tokio::test]
async fn test_list_comments_for_empty_post_returns_empty_data_total_0() {
    let app = spawn_app().await;
    let email = unique_email("cmt-empty");
    let (tokens, _) = register_user(&app, "Empty Post Author", &email, "qwerty").await;
    let post = create_test_post(&app, &tokens.access, "Empty Comments Post", "Body").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .get(app.api_path(&format!("/posts/{post_id}/comments")))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let data = body["data"].as_array().expect("data should be an array");
    assert!(data.is_empty());
    assert_eq!(body["meta"]["total"].as_u64().unwrap(), 0);
}
