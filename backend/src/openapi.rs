use crate::auth::dto;
use crate::auth::routes as auth_routes;
use crate::error::{ErrorBody, FieldError};
use crate::users::routes as users_routes;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

#[derive(OpenApi)]
#[openapi(
    info(title = "Lab Rust Server API", version = "0.1.0"),
    paths(
        auth_routes::register,
        auth_routes::login,
        auth_routes::refresh,
        auth_routes::logout,
        auth_routes::logout_all,
        auth_routes::verify_email,
        auth_routes::resend_verification,
        auth_routes::password_reset_request,
        auth_routes::password_reset_confirm,
        users_routes::get_me,
        users_routes::patch_me,
        users_routes::delete_me,
        users_routes::change_password,
    ),
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
        users_routes::PatchMeRequest,
        users_routes::DeleteMeRequest,
        users_routes::ChangePasswordRequest,
        ErrorBody,
        FieldError,
    )),
    modifiers(&BearerSecurity),
    tags(
        (name = "auth", description = "Registration, login, token rotation, email verification, password reset"),
        (name = "users", description = "Authenticated /me endpoints"),
    ),
)]
pub struct ApiDoc;

struct BearerSecurity;

impl Modify for BearerSecurity {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}
