use crate::AppState;
use crate::auth::password::{hash_password, verify_password};
use crate::auth::repo as auth_repo;
use crate::error::{AppError, AppResult};
use crate::users::repo;
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
) -> AppResult<()> {
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
    Ok(())
}
