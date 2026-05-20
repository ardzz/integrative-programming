mod common;

use common::{
    assert_error_message, create_test_post, login_user, register_user, spawn_app, unique_email,
};
use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn test_list_posts_returns_200() {
    let app = spawn_app().await;

    let response = app.client.get(app.api_path("/posts")).send().await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(
        body["data"].is_array(),
        "paginated envelope must expose data array"
    );
    assert!(
        body["meta"].is_object(),
        "paginated envelope must expose meta object"
    );
}

#[tokio::test]
async fn test_create_post_returns_201() {
    let app = spawn_app().await;
    let email = unique_email("post-create");
    let (tokens, _) = register_user(&app, "Post Author", &email, "qwerty").await;

    let response = app
        .client
        .post(app.api_path("/posts"))
        .bearer_auth(&tokens.access)
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
        .post(app.api_path("/posts"))
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
    let (tokens, _) = register_user(&app, "Post Reader", &email, "qwerty").await;
    let post = create_test_post(&app, &tokens.access, "Readable Post", "Readable content").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .get(app.api_path(&format!("/posts/{post_id}")))
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
        .get(app.api_path("/posts/99999"))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn test_update_own_post_returns_200() {
    let app = spawn_app().await;
    let email = unique_email("post-update-own");
    let (tokens, _) = register_user(&app, "Post Owner", &email, "qwerty").await;
    let post = create_test_post(&app, &tokens.access, "Old Title", "Old content").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .put(app.api_path(&format!("/posts/{post_id}")))
        .bearer_auth(&tokens.access)
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
async fn test_update_others_post_returns_403() {
    let app = spawn_app().await;
    let owner_email = unique_email("post-update-other-owner");
    let other_email = unique_email("post-update-other-actor");
    let (owner_tokens, _) = register_user(&app, "Owner", &owner_email, "qwerty").await;
    let (other_tokens, _) = register_user(&app, "Other", &other_email, "qwerty").await;
    let post = create_test_post(
        &app,
        &owner_tokens.access,
        "Protected Post",
        "Original content",
    )
    .await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .put(app.api_path(&format!("/posts/{post_id}")))
        .bearer_auth(&other_tokens.access)
        .json(&json!({
            "title": "Hijacked",
            "content": "Hijacked",
            "status": "draft",
        }))
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::FORBIDDEN).await;
}

#[tokio::test]
async fn test_delete_own_post_returns_204() {
    let app = spawn_app().await;
    let email = unique_email("post-delete-own");
    register_user(&app, "Delete Owner", &email, "qwerty").await;
    let tokens = login_user(&app, &email, "qwerty").await;
    let post = create_test_post(&app, &tokens.access, "Delete Me", "To be deleted").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .delete(app.api_path(&format!("/posts/{post_id}")))
        .bearer_auth(&tokens.access)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(response.text().await.unwrap(), "");
}

#[tokio::test]
async fn test_delete_others_post_returns_403() {
    let app = spawn_app().await;
    let owner_email = unique_email("post-delete-other-owner");
    let other_email = unique_email("post-delete-other-actor");
    let (owner_tokens, _) = register_user(&app, "Delete Owner", &owner_email, "qwerty").await;
    let (other_tokens, _) = register_user(&app, "Delete Other", &other_email, "qwerty").await;
    let post = create_test_post(&app, &owner_tokens.access, "Cant Delete", "Protected").await;
    let post_id = post["id"].as_i64().unwrap();

    let response = app
        .client
        .delete(app.api_path(&format!("/posts/{post_id}")))
        .bearer_auth(&other_tokens.access)
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::FORBIDDEN).await;
}

#[tokio::test]
async fn test_create_post_defaults_status_to_draft() {
    let app = spawn_app().await;
    let email = unique_email("post-default-status");
    let (tokens, _) = register_user(&app, "Draft Author", &email, "qwerty").await;

    let response = app
        .client
        .post(app.api_path("/posts"))
        .bearer_auth(&tokens.access)
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

#[tokio::test]
async fn test_list_posts_paginated() {
    let app = spawn_app().await;
    let email = unique_email("post-paginate");
    let (tokens, _) = register_user(&app, "Paginator", &email, "qwerty").await;

    for idx in 0..3 {
        create_test_post(
            &app,
            &tokens.access,
            &format!("Page Post {idx}"),
            "paginated body",
        )
        .await;
    }

    let response = app
        .client
        .get(app.api_path("/posts?per_page=2&page=1"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(
        body["data"].as_array().unwrap().len(),
        2,
        "per_page=2 must return exactly 2 items"
    );
    assert!(
        body["meta"]["total"].as_u64().unwrap() >= 3,
        "total must reflect all inserted posts"
    );
    assert_eq!(body["meta"]["page"].as_u64().unwrap(), 1);
    assert_eq!(body["meta"]["per_page"].as_u64().unwrap(), 2);
}

#[tokio::test]
async fn test_list_posts_meta_total_correct() {
    let app = spawn_app().await;
    let email = unique_email("post-meta-total");
    let (tokens, _) = register_user(&app, "Meta Total", &email, "qwerty").await;

    let before: serde_json::Value = app
        .client
        .get(app.api_path("/posts?per_page=1&page=1"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let baseline = before["meta"]["total"].as_u64().unwrap();

    let inserts: u64 = 4;
    for idx in 0..inserts {
        create_test_post(
            &app,
            &tokens.access,
            &format!("Meta Post {idx}"),
            "meta body",
        )
        .await;
    }

    let response = app
        .client
        .get(app.api_path("/posts?per_page=1&page=1"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(
        body["meta"]["total"].as_u64().unwrap(),
        baseline + inserts,
        "meta.total must grow by exactly the number of inserts"
    );
}
