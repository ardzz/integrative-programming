mod common;

use common::{assert_error_message, register_user, spawn_app, unique_email};
use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn test_get_me_returns_current_user() {
    let app = spawn_app().await;
    let email = unique_email("user-me-get");
    let (tokens, body) = register_user(&app, "Me User", &email, "qwerty").await;

    let response = app
        .client
        .get(app.api_path("/users/me"))
        .bearer_auth(&tokens.access)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let me: serde_json::Value = response.json().await.unwrap();
    assert_eq!(me["id"], body["user"]["id"]);
    assert_eq!(me["email"], body["user"]["email"]);
}

#[tokio::test]
async fn test_update_me_updates_own_account() {
    let app = spawn_app().await;
    let email = unique_email("user-me-put");
    let (tokens, _) = register_user(&app, "Before Name", &email, "qwerty").await;

    let response = app
        .client
        .put(app.api_path("/users/me"))
        .bearer_auth(&tokens.access)
        .json(&json!({
            "name": "After Name"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["name"], "After Name");
    assert_eq!(body["email"], email);
}

#[tokio::test]
async fn test_delete_me_hard_deletes_user() {
    let app = spawn_app().await;
    let email = unique_email("user-me-del");
    let (tokens, _) = register_user(&app, "Delete Me", &email, "qwerty").await;

    let delete_response = app
        .client
        .delete(app.api_path("/users/me"))
        .bearer_auth(&tokens.access)
        .send()
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    let get_response = app
        .client
        .get(app.api_path("/users/me"))
        .bearer_auth(&tokens.access)
        .send()
        .await
        .unwrap();

    // After T10 wires /me, the token still authenticates but user lookup misses.
    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_users_paginated() {
    let app = spawn_app().await;
    let email1 = unique_email("user-list-1");
    let email2 = unique_email("user-list-2");
    let email3 = unique_email("user-list-3");
    let email4 = unique_email("user-list-4");

    let (tokens, _) = register_user(&app, "List User 1", &email1, "qwerty").await;
    register_user(&app, "List User 2", &email2, "qwerty").await;
    register_user(&app, "List User 3", &email3, "qwerty").await;
    register_user(&app, "List User 4", &email4, "qwerty").await;

    let response = app
        .client
        .get(app.api_path("/users?per_page=2&page=1"))
        .bearer_auth(&tokens.access)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["meta"]["total"].as_u64().unwrap() >= 3);
    assert_eq!(body["meta"]["page"], 1);
    assert_eq!(body["meta"]["per_page"], 2);
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_list_users_invalid_page_400() {
    let app = spawn_app().await;
    let email = unique_email("user-page-0");
    let (tokens, _) = register_user(&app, "Page Zero", &email, "qwerty").await;

    let response = app
        .client
        .get(app.api_path("/users?page=0"))
        .bearer_auth(&tokens.access)
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn test_list_users_invalid_per_page_400() {
    let app = spawn_app().await;
    let email = unique_email("user-page-101");
    let (tokens, _) = register_user(&app, "Too Many", &email, "qwerty").await;

    let response = app
        .client
        .get(app.api_path("/users?per_page=101"))
        .bearer_auth(&tokens.access)
        .send()
        .await
        .unwrap();

    assert_error_message(response, StatusCode::BAD_REQUEST).await;
}
