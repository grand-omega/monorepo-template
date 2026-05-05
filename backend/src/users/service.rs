use crate::AppState;
use crate::auth::dto::{TokenPair, UserSummary};
use crate::auth::events::{self, EventCtx, EventKind};
use crate::auth::password::{hash_password, verify_password};
use crate::auth::refresh;
use crate::auth::repo as auth_repo;
use crate::auth::tokens::issue_access_token;
use crate::error::{AppError, AppResult};
use crate::users::repo;
use chrono::Utc;
use uuid::Uuid;

pub async fn delete_self(state: &AppState, user_id: Uuid, password_confirm: &str) -> AppResult<()> {
    let user = repo::find_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let ok = verify_password(&user.password_hash, password_confirm)?;
    if !ok {
        return Err(AppError::InvalidCredentials);
    }
    let mut tx = state.db.begin().await?;
    repo::soft_delete(&mut *tx, user_id).await?;
    auth_repo::revoke_all_for_user(&mut *tx, user_id, "account_deleted").await?;
    tx.commit().await?;
    Ok(())
}

pub async fn update_display_name(
    state: &AppState,
    user_id: Uuid,
    new_name: Option<&str>,
) -> AppResult<()> {
    repo::update_display_name(&state.db, user_id, new_name).await
}

pub async fn change_password(
    state: &AppState,
    user_id: Uuid,
    current_password: &str,
    new_password: &str,
) -> AppResult<TokenPair> {
    let user = repo::find_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if !verify_password(&user.password_hash, current_password)? {
        return Err(AppError::InvalidCredentials);
    }
    let new_hash = hash_password(state.argon2(), new_password)?;
    let mut tx = state.db.begin().await?;
    repo::update_password(&mut *tx, user_id, &new_hash).await?;
    auth_repo::revoke_all_for_user(&mut *tx, user_id, "password_change").await?;
    tx.commit().await?;

    let (access_token, _claims) = issue_access_token(
        &state.jwt_keys,
        user.id,
        user.email_verified,
        state.config.access_token_ttl,
    )
    .map_err(AppError::Internal)?;
    let refresh = refresh::generate();
    let expires_at = Utc::now()
        + chrono::Duration::from_std(state.config.refresh_token_ttl)
            .unwrap_or(chrono::Duration::days(30));
    auth_repo::insert_refresh(
        &state.db,
        refresh.id,
        user.id,
        Uuid::now_v7(),
        &refresh.hash,
        None,
        expires_at,
        None,
        None,
    )
    .await?;

    events::record(
        &state.db,
        EventKind::PasswordChanged,
        EventCtx {
            user_id: Some(user_id),
            ..Default::default()
        },
    )
    .await;
    Ok(TokenPair {
        access_token,
        refresh_token: refresh.wire,
        token_type: "Bearer",
        access_expires_in: state.config.access_token_ttl.as_secs() as i64,
        user: UserSummary {
            id: user.id,
            email: user.email,
            email_verified: user.email_verified,
            display_name: user.display_name,
        },
    })
}
