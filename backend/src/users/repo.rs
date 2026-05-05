use crate::error::AppResult;
use crate::users::model::User;
use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub async fn find_by_email(executor: impl PgExecutor<'_>, email: &str) -> AppResult<Option<User>> {
    let row = sqlx::query_as::<_, User>(
        r#"SELECT id, email::text as "email", email_verified, password_hash, display_name,
                  created_at, updated_at, deleted_at, last_login_at,
                  failed_login_count, locked_until
           FROM users WHERE email = $1 AND deleted_at IS NULL"#,
    )
    .bind(email)
    .fetch_optional(executor)
    .await?;
    Ok(row)
}

pub async fn find_by_id(executor: impl PgExecutor<'_>, id: Uuid) -> AppResult<Option<User>> {
    let row = sqlx::query_as::<_, User>(
        r#"SELECT id, email::text as "email", email_verified, password_hash, display_name,
                  created_at, updated_at, deleted_at, last_login_at,
                  failed_login_count, locked_until
           FROM users WHERE id = $1 AND deleted_at IS NULL"#,
    )
    .bind(id)
    .fetch_optional(executor)
    .await?;
    Ok(row)
}

pub struct NewUser<'a> {
    pub id: Uuid,
    pub email: &'a str,
    pub password_hash: &'a str,
    pub display_name: Option<&'a str>,
}

pub async fn insert(executor: impl PgExecutor<'_>, new_user: NewUser<'_>) -> AppResult<User> {
    let row = sqlx::query_as::<_, User>(
        r#"INSERT INTO users (id, email, password_hash, display_name)
           VALUES ($1, $2::citext, $3, $4)
           RETURNING id, email::text as "email", email_verified, password_hash, display_name,
                     created_at, updated_at, deleted_at, last_login_at,
                     failed_login_count, locked_until"#,
    )
    .bind(new_user.id)
    .bind(new_user.email)
    .bind(new_user.password_hash)
    .bind(new_user.display_name)
    .fetch_one(executor)
    .await?;
    Ok(row)
}

pub async fn record_successful_login(pool: &PgPool, id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"UPDATE users
           SET last_login_at = now(), failed_login_count = 0, locked_until = NULL,
               updated_at = now()
           WHERE id = $1"#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Returns true if this attempt pushed the account into a locked state.
pub async fn record_failed_login(
    pool: &PgPool,
    id: Uuid,
    max_failures: i32,
    lock_until: DateTime<Utc>,
) -> AppResult<bool> {
    let row: Option<(i32, Option<DateTime<Utc>>)> = sqlx::query_as(
        r#"UPDATE users
           SET failed_login_count = failed_login_count + 1,
               locked_until = CASE
                   WHEN failed_login_count + 1 >= $2 THEN $3
                   ELSE locked_until
               END,
               updated_at = now()
           WHERE id = $1
           RETURNING failed_login_count, locked_until"#,
    )
    .bind(id)
    .bind(max_failures)
    .bind(lock_until)
    .fetch_optional(pool)
    .await?;
    Ok(matches!(row, Some((c, Some(_))) if c >= max_failures))
}

pub async fn mark_email_verified(executor: impl PgExecutor<'_>, id: Uuid) -> AppResult<()> {
    sqlx::query(r#"UPDATE users SET email_verified = TRUE, updated_at = now() WHERE id = $1"#)
        .bind(id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn update_password(
    executor: impl PgExecutor<'_>,
    id: Uuid,
    new_hash: &str,
) -> AppResult<()> {
    sqlx::query(r#"UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1"#)
        .bind(id)
        .bind(new_hash)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn update_display_name(
    pool: &PgPool,
    id: Uuid,
    display_name: Option<&str>,
) -> AppResult<()> {
    sqlx::query(r#"UPDATE users SET display_name = $2, updated_at = now() WHERE id = $1"#)
        .bind(id)
        .bind(display_name)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn soft_delete(executor: impl PgExecutor<'_>, id: Uuid) -> AppResult<()> {
    // Scrub email so the address can be re-registered.
    sqlx::query(
        r#"UPDATE users
           SET deleted_at = now(),
               email = ('deleted+' || id::text || '@invalid')::citext,
               updated_at = now()
           WHERE id = $1"#,
    )
    .bind(id)
    .execute(executor)
    .await?;
    Ok(())
}
