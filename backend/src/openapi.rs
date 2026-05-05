use crate::auth::dto;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(title = "Lab Rust Server API", version = "0.1.0"),
    components(schemas(
        dto::RegisterRequest,
        dto::LoginRequest,
        dto::RefreshRequest,
        dto::LogoutRequest,
        dto::VerifyEmailRequest,
        dto::ResendVerificationRequest,
        dto::PasswordResetRequest,
        dto::PasswordResetConfirmRequest,
        dto::TokenPair,
        dto::UserSummary,
        dto::AcceptedResponse,
    ))
)]
pub struct ApiDoc;
