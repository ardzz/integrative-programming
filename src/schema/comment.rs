use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateComment {
    #[validate(length(min = 1, max = 250))]
    pub comment: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateComment {
    #[validate(length(min = 1, max = 250))]
    pub comment: String,
}
