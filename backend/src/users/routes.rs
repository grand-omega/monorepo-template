use crate::AppState;
use crate::auth::dto::{AcceptedResponse, UserSummary};
use crate::error::{AppError, AppResult};
use crate::middleware::auth::AuthUser;
use crate::middleware::rate_limit::{self, Class};
use crate::users::{repo, service};
use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{delete, get, patch};
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/me", get(get_me).layer(rate_limit::layer(Class::Medium)))
        .route(
            "/me",
            patch(patch_me).layer(rate_limit::layer(Class::Low)),
        )
        .route(
            "/me",
            delete(delete_me).layer(rate_limit::layer(Class::Low)),
        )
        .route(
            "/me/password",
            patch(change_password).layer(rate_limit::layer(Class::Low)),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PatchMeRequest {
    #[validate(length(min = 1, max = 100))]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct DeleteMeRequest {
    #[validate(length(min = 1, max = 128))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1, max = 128))]
    pub current_password: String,
    #[validate(length(min = 12, max = 128))]
    pub new_password: String,
}

async fn get_me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<UserSummary>> {
    let row = repo::find_by_id(&state.db, user.claims.sub)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(UserSummary {
        id: row.id,
        email: row.email,
        email_verified: row.email_verified,
        display_name: row.display_name,
    }))
}

async fn patch_me(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<PatchMeRequest>,
) -> AppResult<Json<UserSummary>> {
    body.validate().map_err(AppError::from)?;
    if let Some(name) = &body.display_name {
        service::update_display_name(&state, user.claims.sub, Some(name.as_str())).await?;
    }
    let row = repo::find_by_id(&state.db, user.claims.sub)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(UserSummary {
        id: row.id,
        email: row.email,
        email_verified: row.email_verified,
        display_name: row.display_name,
    }))
}

async fn delete_me(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<DeleteMeRequest>,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    body.validate().map_err(AppError::from)?;
    service::delete_self(&state, user.claims.sub, &body.password).await?;
    Ok((StatusCode::OK, Json(AcceptedResponse::default())))
}

async fn change_password(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<ChangePasswordRequest>,
) -> AppResult<(StatusCode, Json<AcceptedResponse>)> {
    body.validate().map_err(AppError::from)?;
    service::change_password(
        &state,
        user.claims.sub,
        &body.current_password,
        &body.new_password,
    )
    .await?;
    Ok((StatusCode::OK, Json(AcceptedResponse::default())))
}
