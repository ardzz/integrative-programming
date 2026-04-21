mod common;

use common::{create_test_post, register_user, spawn_app, unique_email};
use reqwest::StatusCode;

#[tokio::test]
async fn test_pagination_envelope_shape() {
    let app = spawn_app().await;
    let email = unique_email("page-shape");
    let (tokens, _) = register_user(&app, "Pagination Shape", &email, "qwerty").await;
    create_test_post(&app, &tokens.access, "Shape Post", "Shape Content").await;

    let response = app
        .client
        .get(format!("{}/api/posts", app.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["meta"]["page"].is_number());
    assert!(body["meta"]["per_page"].is_number());
    assert!(body["meta"]["total"].is_number());
    assert!(body["meta"]["total_pages"].is_number());
    assert!(body["data"].is_array());
}

#[tokio::test]
async fn test_pagination_defaults_to_page_1_per_page_10() {
    let app = spawn_app().await;

    let response = app
        .client
        .get(format!("{}/api/posts", app.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["meta"]["page"], 1);
    assert_eq!(body["meta"]["per_page"], 10);
}

#[tokio::test]
async fn test_pagination_page_beyond_total_returns_empty_data() {
    let app = spawn_app().await;
    let email = unique_email("page-high");
    let (tokens, _) = register_user(&app, "Pagination High", &email, "qwerty").await;
    create_test_post(&app, &tokens.access, "Beyond Post", "Beyond Content").await;

    let response = app
        .client
        .get(format!("{}/api/posts?per_page=5&page=999", app.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["data"], serde_json::json!([]));
    assert!(body["meta"]["total_pages"].as_u64().unwrap() < 999);
}
