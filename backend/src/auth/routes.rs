use crate::AppState;
use crate::auth::dto::{
    AcceptedResponse, LoginRequest, LogoutRequest, PasswordResetConfirmRequest,
    PasswordResetRequest, RefreshRequest, RegisterRequest, ResendVerificationRequest, TokenPair,
    VerifyEmailRequest,
};
use crate::auth::service;
use crate::error::{AppError, AppResult};
use crate::middleware::auth::AuthUser;
use crate::middleware::rate_limit::{self, Class};
use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::routing::post;
use std::net::IpAddr;
use validator::Validate;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/register",
            post(register).layer(rate_limit::layer(Class::Strict)),
        )
        .route(
            "/login",
            post(login).layer(rate_limit::layer(Class::Strict)),
        )
        .route(
            "/refresh",
            post(refresh).layer(rate_limit::layer(Class::Medium)),
        )
        .route("/logout", post(logout).layer(rate_limit::layer(Class::Low)))
        .route(
            "/logout-all",
            post(logout_all).layer(rate_limit::layer(Class::Low)),
        )
        .route(
            "/verify-email",
            post(verify_email).layer(rate_limit::layer(Class::Strict)),
        )
        .route(
            "/resend-verification",
            post(resend_verification).layer(rate_limit::layer(Class::VeryStrict)),
        )
        .route(
            "/password-reset/request",
            post(password_reset_request).layer(rate_limit::layer(Class::VeryStrict)),
        )
        .route(
            "/password-reset/confirm",
            post(password_reset_confirm).layer(rate_limit::layer(Class::Strict)),
        )
        .with_state(state)
}

fn ctx_from(headers: &HeaderMap) -> service::ClientContext<'_> {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.trim().parse::<IpAddr>().ok())
        });
    let user_agent = headers
        .get(http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok());
    service::ClientContext { ip, user_agent }
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    body.validate().map_err(AppError::from)?;
    service::register(&state, &body.email, &body.password, body.display_name.as_deref()).await?;
    Ok((StatusCode::ACCEPTED, Json(AcceptedResponse::default())))
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LoginRequest>,
) -> AppResult<Json<TokenPair>> {
    body.validate().map_err(AppError::from)?;
    let ctx = ctx_from(&headers);
    let pair = service::login(&state, &body.email, &body.password, ctx).await?;
    Ok(Json(pair))
}

async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RefreshRequest>,
) -> AppResult<Json<TokenPair>> {
    body.validate().map_err(AppError::from)?;
    let ctx = ctx_from(&headers);
    let pair = service::refresh_token(&state, &body.refresh_token, ctx).await?;
    Ok(Json(pair))
}

async fn logout(
    State(state): State<AppState>,
    Json(body): Json<LogoutRequest>,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    body.validate().map_err(AppError::from)?;
    service::logout(&state, &body.refresh_token).await?;
    Ok((StatusCode::OK, Json(AcceptedResponse::default())))
}

async fn logout_all(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    service::logout_all(&state, user.claims.sub).await?;
    Ok((StatusCode::OK, Json(AcceptedResponse::default())))
}

async fn verify_email(
    State(state): State<AppState>,
    Json(body): Json<VerifyEmailRequest>,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    body.validate().map_err(AppError::from)?;
    service::verify_email(&state, &body.token).await?;
    Ok((StatusCode::OK, Json(AcceptedResponse::default())))
}

async fn resend_verification(
    State(state): State<AppState>,
    Json(body): Json<ResendVerificationRequest>,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    body.validate().map_err(AppError::from)?;
    service::resend_verification(&state, &body.email).await?;
    Ok((StatusCode::ACCEPTED, Json(AcceptedResponse::default())))
}

async fn password_reset_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PasswordResetRequest>,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    body.validate().map_err(AppError::from)?;
    let ip = ctx_from(&headers).ip;
    service::request_password_reset(&state, &body.email, ip).await?;
    Ok((StatusCode::ACCEPTED, Json(AcceptedResponse::default())))
}

async fn password_reset_confirm(
    State(state): State<AppState>,
    Json(body): Json<PasswordResetConfirmRequest>,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    body.validate().map_err(AppError::from)?;
    service::confirm_password_reset(&state, &body.token, &body.new_password).await?;
    Ok((StatusCode::OK, Json(AcceptedResponse::default())))
}
