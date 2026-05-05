use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tracing::error;
use utoipa::ToSchema;

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation failed")]
    Validation(Vec<FieldError>),

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("account locked")]
    AccountLocked,

    #[error("email not verified")]
    EmailNotVerified,

    #[error("token invalid or expired")]
    InvalidToken,

    #[error("token reuse detected")]
    TokenReuseDetected,

    #[error("not found")]
    NotFound,

    #[error("conflict")]
    Conflict(&'static str),

    #[error("forbidden")]
    Forbidden,

    #[error("unauthorized")]
    Unauthorized,

    #[error("rate limited")]
    RateLimited,

    #[error("payload too large")]
    PayloadTooLarge,

    #[error("bad request: {0}")]
    BadRequest(&'static str),

    #[error("service unavailable")]
    ServiceUnavailable,

    #[error("internal error")]
    Internal(#[from] anyhow::Error),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct FieldError {
    pub field: String,
    pub code: String,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<FieldError>>,
}

impl AppError {
    fn parts(&self) -> (StatusCode, &'static str) {
        match self {
            AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "validation_failed"),
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "invalid_credentials"),
            AppError::AccountLocked => (StatusCode::LOCKED, "account_locked"),
            AppError::EmailNotVerified => (StatusCode::FORBIDDEN, "email_not_verified"),
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, "invalid_token"),
            AppError::TokenReuseDetected => (StatusCode::UNAUTHORIZED, "token_reuse_detected"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            AppError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            AppError::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            AppError::ServiceUnavailable => (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable"),
            AppError::Internal(_)
            | AppError::Sqlx(_)
            | AppError::Jwt(_)
            | AppError::Json(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.parts();
        let public_message = match &self {
            AppError::Conflict(m) | AppError::BadRequest(m) => (*m).to_string(),
            _ => self.to_string(),
        };
        let fields = match &self {
            AppError::Validation(f) => Some(f.clone()),
            _ => None,
        };

        if status.is_server_error() {
            error!(error = %self, "request failed with server error");
        }

        let body = ErrorBody {
            code,
            message: public_message,
            fields,
        };
        (status, Json(body)).into_response()
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(errors: validator::ValidationErrors) -> Self {
        let mut out = Vec::new();
        for (field, errs) in errors.field_errors() {
            for e in errs {
                out.push(FieldError {
                    field: field.to_string(),
                    code: e.code.to_string(),
                });
            }
        }
        AppError::Validation(out)
    }
}
