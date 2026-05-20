use mockall::{automock, predicate::*};

#[automock]
#[async_trait::async_trait]
trait EmailService: Send + Sync {
    async fn send_welcome(&self, recipient: &str, name: &str) -> Result<String, String>;
}

#[async_trait::async_trait]
trait UserOnboarder {
    async fn onboard(&self, email: &str, name: &str) -> Result<String, String>;
}

struct OnboardingService<E: EmailService> {
    email: E,
}

#[async_trait::async_trait]
impl<E: EmailService + Sync> UserOnboarder for OnboardingService<E> {
    async fn onboard(&self, email: &str, name: &str) -> Result<String, String> {
        if !email.contains('@') {
            return Err("invalid email".into());
        }
        self.email.send_welcome(email, name).await
    }
}

#[tokio::test]
async fn test_onboard_calls_email_service_with_expected_args() {
    let mut mock = MockEmailService::new();
    mock.expect_send_welcome()
        .with(eq("alice@example.com"), eq("Alice"))
        .times(1)
        .returning(|_, _| Ok("msg-abc-123".to_string()));

    let service = OnboardingService { email: mock };
    let result = service.onboard("alice@example.com", "Alice").await;

    assert_eq!(result, Ok("msg-abc-123".to_string()));
}

#[tokio::test]
async fn test_onboard_rejects_invalid_email_without_calling_service() {
    let mut mock = MockEmailService::new();
    mock.expect_send_welcome().times(0);

    let service = OnboardingService { email: mock };
    let result = service.onboard("not-an-email", "Bob").await;

    assert_eq!(result, Err("invalid email".to_string()));
}

#[tokio::test]
async fn test_onboard_propagates_email_service_failure() {
    let mut mock = MockEmailService::new();
    mock.expect_send_welcome()
        .returning(|_, _| Err("smtp unavailable".to_string()));

    let service = OnboardingService { email: mock };
    let result = service.onboard("carol@example.com", "Carol").await;

    assert_eq!(result, Err("smtp unavailable".to_string()));
}
