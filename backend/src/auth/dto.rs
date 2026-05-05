use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    #[validate(email, length(max = 254))]
    pub email: String,
    #[validate(length(min = 12, max = 128))]
    pub password: String,
    #[validate(length(min = 1, max = 100))]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email, length(max = 254))]
    pub email: String,
    #[validate(length(min = 12, max = 128))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RefreshRequest {
    #[validate(length(min = 1, max = 512))]
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LogoutRequest {
    #[validate(length(min = 1, max = 512))]
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VerifyEmailRequest {
    #[validate(length(min = 1, max = 512))]
    pub token: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResendVerificationRequest {
    #[validate(email, length(max = 254))]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PasswordResetRequest {
    #[validate(email, length(max = 254))]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PasswordResetConfirmRequest {
    #[validate(length(min = 1, max = 512))]
    pub token: String,
    #[validate(length(min = 12, max = 128))]
    pub new_password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub access_expires_in: i64,
    pub user: UserSummary,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserSummary {
    pub id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AcceptedResponse {
    pub status: &'static str,
}

impl Default for AcceptedResponse {
    fn default() -> Self {
        Self { status: "accepted" }
    }
}
