use crate::AppState;
use crate::auth::dto::{
    AcceptedResponse, LoginRequest, LogoutRequest, PasswordResetConfirmRequest,
    PasswordResetRequest, RefreshRequest, RegisterRequest, ResendVerificationRequest, TokenPair,
    VerifyEmailRequest,
};
use crate::auth::service;
use crate::error::{AppError, AppResult, ErrorBody};
use crate::middleware::auth::AuthUser;
use crate::middleware::peer::client_ip;
use crate::middleware::rate_limit::{self, Class};
use axum::Json;
use axum::Router;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::routing::post;
use ipnetwork::IpNetwork;
use std::net::SocketAddr;
use validator::Validate;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/register",
            post(register).layer(rate_limit::layer(&state, Class::Strict)),
        )
        .route(
            "/login",
            post(login).layer(rate_limit::layer(&state, Class::Strict)),
        )
        .route(
            "/refresh",
            post(refresh).layer(rate_limit::layer(&state, Class::Medium)),
        )
        .route("/logout", post(logout).layer(rate_limit::layer(&state, Class::Low)))
        .route(
            "/logout-all",
            post(logout_all).layer(rate_limit::layer(&state, Class::Low)),
        )
        .route(
            "/verify-email",
            post(verify_email).layer(rate_limit::layer(&state, Class::Strict)),
        )
        .route(
            "/resend-verification",
            post(resend_verification).layer(rate_limit::layer(&state, Class::VeryStrict)),
        )
        .route(
            "/password-reset/request",
            post(password_reset_request).layer(rate_limit::layer(&state, Class::VeryStrict)),
        )
        .route(
            "/password-reset/confirm",
            post(password_reset_confirm).layer(rate_limit::layer(&state, Class::Strict)),
        )
        .with_state(state)
}

fn ctx_from<'a>(
    headers: &'a HeaderMap,
    peer: SocketAddr,
    trusted: &[IpNetwork],
) -> service::ClientContext<'a> {
    let ip = Some(client_ip(headers, peer.ip(), trusted));
    let user_agent = headers
        .get(http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok());
    service::ClientContext { ip, user_agent }
}

#[utoipa::path(
    post,
    path = "/v1/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 202, description = "Registration accepted (always 202 even on duplicate)", body = AcceptedResponse),
        (status = 422, description = "Validation failed", body = ErrorBody),
        (status = 429, description = "Rate limited", body = ErrorBody),
    ),
)]
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    body.validate().map_err(AppError::from)?;
    service::register(&state, &body.email, &body.password, body.display_name.as_deref()).await?;
    Ok((StatusCode::ACCEPTED, Json(AcceptedResponse::default())))
}

#[utoipa::path(
    post,
    path = "/v1/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Logged in", body = TokenPair),
        (status = 401, description = "Invalid credentials", body = ErrorBody),
        (status = 423, description = "Account locked after too many failed attempts", body = ErrorBody),
        (status = 422, description = "Validation failed", body = ErrorBody),
        (status = 429, description = "Rate limited", body = ErrorBody),
    ),
)]
pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<LoginRequest>,
) -> AppResult<Json<TokenPair>> {
    body.validate().map_err(AppError::from)?;
    let ctx = ctx_from(&headers, peer, &state.config.trusted_proxy_cidrs);
    let pair = service::login(&state, &body.email, &body.password, ctx).await?;
    Ok(Json(pair))
}

#[utoipa::path(
    post,
    path = "/v1/auth/refresh",
    tag = "auth",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Rotated", body = TokenPair),
        (status = 401, description = "Invalid, expired, or replayed refresh token", body = ErrorBody),
        (status = 429, description = "Rate limited", body = ErrorBody),
    ),
)]
pub async fn refresh(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<RefreshRequest>,
) -> AppResult<Json<TokenPair>> {
    body.validate().map_err(AppError::from)?;
    let ctx = ctx_from(&headers, peer, &state.config.trusted_proxy_cidrs);
    let pair = service::refresh_token(&state, &body.refresh_token, ctx).await?;
    Ok(Json(pair))
}

#[utoipa::path(
    post,
    path = "/v1/auth/logout",
    tag = "auth",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "Refresh token revoked", body = AcceptedResponse),
        (status = 422, description = "Validation failed", body = ErrorBody),
    ),
)]
pub async fn logout(
    State(state): State<AppState>,
    Json(body): Json<LogoutRequest>,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    body.validate().map_err(AppError::from)?;
    service::logout(&state, &body.refresh_token).await?;
    Ok((StatusCode::OK, Json(AcceptedResponse::default())))
}

#[utoipa::path(
    post,
    path = "/v1/auth/logout-all",
    tag = "auth",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "All refresh tokens for the user revoked", body = AcceptedResponse),
        (status = 401, description = "Missing or invalid access token", body = ErrorBody),
    ),
)]
pub async fn logout_all(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    service::logout_all(&state, user.claims.sub).await?;
    Ok((StatusCode::OK, Json(AcceptedResponse::default())))
}

#[utoipa::path(
    post,
    path = "/v1/auth/verify-email",
    tag = "auth",
    request_body = VerifyEmailRequest,
    responses(
        (status = 200, description = "Email verified", body = AcceptedResponse),
        (status = 401, description = "Invalid or expired token", body = ErrorBody),
        (status = 422, description = "Validation failed", body = ErrorBody),
    ),
)]
pub async fn verify_email(
    State(state): State<AppState>,
    Json(body): Json<VerifyEmailRequest>,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    body.validate().map_err(AppError::from)?;
    service::verify_email(&state, &body.token).await?;
    Ok((StatusCode::OK, Json(AcceptedResponse::default())))
}

#[utoipa::path(
    post,
    path = "/v1/auth/resend-verification",
    tag = "auth",
    request_body = ResendVerificationRequest,
    responses(
        (status = 202, description = "Always 202 even when the email is unknown", body = AcceptedResponse),
        (status = 422, description = "Validation failed", body = ErrorBody),
        (status = 429, description = "Rate limited", body = ErrorBody),
    ),
)]
pub async fn resend_verification(
    State(state): State<AppState>,
    Json(body): Json<ResendVerificationRequest>,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    body.validate().map_err(AppError::from)?;
    service::resend_verification(&state, &body.email).await?;
    Ok((StatusCode::ACCEPTED, Json(AcceptedResponse::default())))
}

#[utoipa::path(
    post,
    path = "/v1/auth/password-reset/request",
    tag = "auth",
    request_body = PasswordResetRequest,
    responses(
        (status = 202, description = "Always 202 even when the email is unknown", body = AcceptedResponse),
        (status = 422, description = "Validation failed", body = ErrorBody),
        (status = 429, description = "Rate limited", body = ErrorBody),
    ),
)]
pub async fn password_reset_request(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<PasswordResetRequest>,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    body.validate().map_err(AppError::from)?;
    let ip = ctx_from(&headers, peer, &state.config.trusted_proxy_cidrs).ip;
    service::request_password_reset(&state, &body.email, ip).await?;
    Ok((StatusCode::ACCEPTED, Json(AcceptedResponse::default())))
}

#[utoipa::path(
    post,
    path = "/v1/auth/password-reset/confirm",
    tag = "auth",
    request_body = PasswordResetConfirmRequest,
    responses(
        (status = 200, description = "Password updated; all sessions revoked", body = AcceptedResponse),
        (status = 401, description = "Invalid or expired token", body = ErrorBody),
        (status = 422, description = "Validation failed", body = ErrorBody),
    ),
)]
pub async fn password_reset_confirm(
    State(state): State<AppState>,
    Json(body): Json<PasswordResetConfirmRequest>,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    body.validate().map_err(AppError::from)?;
    service::confirm_password_reset(&state, &body.token, &body.new_password).await?;
    Ok((StatusCode::OK, Json(AcceptedResponse::default())))
}
