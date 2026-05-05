use crate::AppState;
use crate::auth::tokens::{AccessClaims, decode_access_token};
use crate::error::AppError;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub claims: AccessClaims,
}

impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .ok_or(AppError::Unauthorized)?
            .to_str()
            .map_err(|_| AppError::Unauthorized)?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;
        let claims = decode_access_token(&app_state.jwt_keys, token)?;
        Ok(AuthUser { claims })
    }
}

#[derive(Debug, Clone)]
pub struct VerifiedUser(pub AccessClaims);

impl<S> FromRequestParts<S> for VerifiedUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if !user.claims.email_verified {
            return Err(AppError::EmailNotVerified);
        }
        Ok(VerifiedUser(user.claims))
    }
}
