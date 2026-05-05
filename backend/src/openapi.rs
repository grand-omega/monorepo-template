use crate::admin::routes as admin_routes;
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
        admin_routes::login,
        admin_routes::logout,
        admin_routes::me,
        admin_routes::users_list,
        admin_routes::user_show,
        admin_routes::user_lock,
        admin_routes::user_unlock,
        admin_routes::user_verify_email,
        admin_routes::user_revoke_sessions,
        admin_routes::auth_events_list,
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
        admin_routes::AdminLoginRequest,
        admin_routes::AdminLoginResponse,
        admin_routes::AdminMe,
        admin_routes::AdminAck,
        admin_routes::ManagedUser,
        admin_routes::AuthEvent,
        admin_routes::LockRequest,
        admin_routes::PageManagedUser,
        admin_routes::PageAuthEvent,
        ErrorBody,
        FieldError,
    )),
    modifiers(&BearerSecurity),
    tags(
        (name = "auth", description = "Registration, login, token rotation, email verification, password reset"),
        (name = "users", description = "Authenticated /me endpoints"),
        (name = "admin", description = "Cookie-auth admin management API; CSRF required on state-changing requests"),
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
