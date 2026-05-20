mod common;

use common::{spawn_app, unique_email};
use reqwest::StatusCode;
use serde_json::json;

// Run in isolation so the process-wide RATE_LIMIT_ENABLED env var does not leak
// across parallel tests: cargo test --test rate_limit_test -- --ignored --test-threads=1

#[tokio::test]
#[ignore = "run in isolation: cargo test --test rate_limit_test -- --ignored --test-threads=1"]
async fn test_login_rate_limit_triggers_429() {
    std::env::set_var("RATE_LIMIT_ENABLED", "true");
    let app = spawn_app().await;
    let email = unique_email("rl-login");

    let mut last_status = StatusCode::OK;

    for _ in 0..6 {
        let response = app
            .client
            .post(app.api_path("/auth/login"))
            .json(&json!({
                "email": email,
                "password": "wrong-password",
            }))
            .send()
            .await
            .unwrap();

        last_status = response.status();

        if last_status == StatusCode::TOO_MANY_REQUESTS {
            break;
        }
    }

    assert_eq!(last_status, StatusCode::TOO_MANY_REQUESTS);
    std::env::set_var("RATE_LIMIT_ENABLED", "false");
}

#[tokio::test]
#[ignore = "run in isolation: cargo test --test rate_limit_test -- --ignored --test-threads=1"]
async fn test_global_rate_limit_triggers_429() {
    std::env::set_var("RATE_LIMIT_ENABLED", "true");
    let app = spawn_app().await;

    let mut saw_429 = false;

    for _ in 0..65 {
        let response = app.client.get(app.api_path("/posts")).send().await.unwrap();

        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            saw_429 = true;
            break;
        }
    }

    assert!(saw_429, "expected global rate limit to produce HTTP 429");
    std::env::set_var("RATE_LIMIT_ENABLED", "false");
}
