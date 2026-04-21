#![allow(dead_code)]

use std::time::Duration;

use blog_api::{route::create_router, AppState};
use reqwest::{Client, Response, StatusCode};
use serde_json::{json, Value};
use sqlx::mysql::MySqlPoolOptions;
use tokio::net::TcpListener;
use tokio::sync::OnceCell;
use uuid::Uuid;

static DB_SETUP: OnceCell<()> = OnceCell::const_new();

pub struct TestApp {
    pub base_url: String,
    pub client: Client,
}

pub struct AuthTokens {
    pub access: String,
    pub refresh: String,
}

pub async fn spawn_app() -> TestApp {
    dotenvy::dotenv().ok();

    // Default rate limiting to disabled for tests so parallel runs don't hit
    // spurious 429s. Individual tests (e.g., rate_limit_test) may override
    // this by setting the env var before calling spawn_app().
    if std::env::var("RATE_LIMIT_ENABLED").is_err() {
        std::env::set_var("RATE_LIMIT_ENABLED", "false");
    }

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    ensure_test_database_ready(&database_url).await;

    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("DB connect");

    let jwt_secret =
        std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-in-production".into());
    let state = AppState {
        db: pool,
        jwt_secret,
    };

    let app = create_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let test_app = TestApp {
        base_url: format!("http://127.0.0.1:{}", addr.port()),
        client: Client::new(),
    };

    wait_for_health(&test_app).await;

    test_app
}

async fn ensure_test_database_ready(database_url: &str) {
    DB_SETUP
        .get_or_init(|| async move {
            let pool = MySqlPoolOptions::new()
                .max_connections(5)
                .connect(database_url)
                .await
                .expect("DB connect");

            sqlx::migrate!().run(&pool).await.expect("migrations");
        })
        .await;
}

async fn wait_for_health(app: &TestApp) {
    for _ in 0..20 {
        if let Ok(response) = app.client.get(format!("{}/health", app.base_url)).send().await {
            if response.status() == StatusCode::OK {
                return;
            }
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    panic!("server failed to become healthy");
}

pub fn unique_email(prefix: &str) -> String {
    let suffix = Uuid::new_v4().simple().to_string();
    let short_prefix: String = prefix.chars().take(8).collect();
    format!("{}{}@t.io", short_prefix, &suffix[..8])
}

fn parse_auth_tokens(body: &Value) -> AuthTokens {
    let access = body["access_token"]
        .as_str()
        .expect("access_token should be present")
        .to_string();
    let refresh = body["refresh_token"]
        .as_str()
        .expect("refresh_token should be present")
        .to_string();
    assert!(!access.is_empty(), "access_token should be non-empty");
    assert!(!refresh.is_empty(), "refresh_token should be non-empty");
    AuthTokens { access, refresh }
}

pub async fn register_user(
    app: &TestApp,
    name: &str,
    email: &str,
    password: &str,
) -> (AuthTokens, Value) {
    let response = app
        .client
        .post(format!("{}/api/auth/register", app.base_url))
        .json(&json!({
            "name": name,
            "email": email,
            "password": password,
        }))
        .send()
        .await
        .expect("register request should succeed");

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = response.json().await.expect("register body should be json");
    let tokens = parse_auth_tokens(&body);
    (tokens, body)
}

pub async fn login_user(app: &TestApp, email: &str, password: &str) -> AuthTokens {
    let response = app
        .client
        .post(format!("{}/api/auth/login", app.base_url))
        .json(&json!({
            "email": email,
            "password": password,
        }))
        .send()
        .await
        .expect("login request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("login body should be json");
    parse_auth_tokens(&body)
}

pub async fn refresh_tokens(app: &TestApp, refresh: &str) -> AuthTokens {
    let response = app
        .client
        .post(format!("{}/api/auth/refresh", app.base_url))
        .json(&json!({
            "refresh_token": refresh,
        }))
        .send()
        .await
        .expect("refresh request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("refresh body should be json");
    parse_auth_tokens(&body)
}

pub async fn create_test_post(app: &TestApp, token: &str, title: &str, content: &str) -> Value {
    let response = app
        .client
        .post(format!("{}/api/posts", app.base_url))
        .bearer_auth(token)
        .json(&json!({
            "title": title,
            "content": content,
            "status": "draft",
        }))
        .send()
        .await
        .expect("create post request should succeed");

    assert_eq!(response.status(), StatusCode::CREATED);
    response.json().await.expect("post body should be json")
}

pub async fn create_test_comment(app: &TestApp, token: &str, post_id: i32, comment: &str) -> Value {
    let response = app
        .client
        .post(format!("{}/api/posts/{post_id}/comments", app.base_url))
        .bearer_auth(token)
        .json(&json!({
            "comment": comment,
        }))
        .send()
        .await
        .expect("create comment request should succeed");

    assert_eq!(response.status(), StatusCode::CREATED);
    response.json().await.expect("comment body should be json")
}

pub async fn assert_error_message(response: Response, status: StatusCode) -> Value {
    assert_eq!(response.status(), status);
    let body: Value = response.json().await.expect("error body should be json");
    assert!(body.get("error").and_then(Value::as_str).is_some());
    body
}
