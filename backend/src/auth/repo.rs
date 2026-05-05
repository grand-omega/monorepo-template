use crate::error::AppResult;
use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RefreshTokenRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub token_hash: Vec<u8>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_reason: Option<String>,
}

pub async fn insert_refresh(
    executor: impl PgExecutor<'_>,
    id: Uuid,
    user_id: Uuid,
    family_id: Uuid,
    token_hash: &[u8],
    parent_id: Option<Uuid>,
    expires_at: DateTime<Utc>,
    user_agent: Option<&str>,
    ip: Option<ipnetwork::IpNetwork>,
) -> AppResult<()> {
    sqlx::query(
        r#"INSERT INTO refresh_tokens
            (id, user_id, family_id, token_hash, parent_id, expires_at, user_agent, ip)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8)"#,
    )
    .bind(id)
    .bind(user_id)
    .bind(family_id)
    .bind(token_hash)
    .bind(parent_id)
    .bind(expires_at)
    .bind(user_agent)
    .bind(ip)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn find_refresh_by_id(
    executor: impl PgExecutor<'_>,
    id: Uuid,
) -> AppResult<Option<RefreshTokenRow>> {
    let row = sqlx::query_as::<_, RefreshTokenRow>(
        r#"SELECT id, user_id, family_id, token_hash, expires_at, used_at, revoked_at, revoked_reason
           FROM refresh_tokens WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(executor)
    .await?;
    Ok(row)
}

pub async fn mark_rotated(executor: impl PgExecutor<'_>, id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"UPDATE refresh_tokens
           SET used_at = now(), revoked_at = now(), revoked_reason = 'rotated'
           WHERE id = $1"#,
    )
    .bind(id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn revoke_token(pool: &PgPool, id: Uuid, reason: &str) -> AppResult<()> {
    sqlx::query(
        r#"UPDATE refresh_tokens
           SET revoked_at = COALESCE(revoked_at, now()), revoked_reason = COALESCE(revoked_reason, $2)
           WHERE id = $1"#,
    )
    .bind(id)
    .bind(reason)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn revoke_family(executor: impl PgExecutor<'_>, family_id: Uuid, reason: &str) -> AppResult<u64> {
    let res = sqlx::query(
        r#"UPDATE refresh_tokens
           SET revoked_at = COALESCE(revoked_at, now()),
               revoked_reason = COALESCE(revoked_reason, $2)
           WHERE family_id = $1 AND revoked_at IS NULL"#,
    )
    .bind(family_id)
    .bind(reason)
    .execute(executor)
    .await?;
    Ok(res.rows_affected())
}

pub async fn revoke_all_for_user(
    executor: impl PgExecutor<'_>,
    user_id: Uuid,
    reason: &str,
) -> AppResult<u64> {
    let res = sqlx::query(
        r#"UPDATE refresh_tokens
           SET revoked_at = now(), revoked_reason = $2
           WHERE user_id = $1 AND revoked_at IS NULL"#,
    )
    .bind(user_id)
    .bind(reason)
    .execute(executor)
    .await?;
    Ok(res.rows_affected())
}

pub async fn delete_expired(pool: &PgPool, older_than: DateTime<Utc>) -> AppResult<u64> {
    let res = sqlx::query(r#"DELETE FROM refresh_tokens WHERE expires_at < $1"#)
        .bind(older_than)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

// --- email verification tokens --------------------------------------------------

pub async fn insert_email_verification(
    executor: impl PgExecutor<'_>,
    id: Uuid,
    user_id: Uuid,
    token_hash: &[u8],
    email: &str,
    expires_at: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"INSERT INTO email_verification_tokens (id, user_id, token_hash, email, expires_at)
           VALUES ($1,$2,$3,$4::citext,$5)"#,
    )
    .bind(id)
    .bind(user_id)
    .bind(token_hash)
    .bind(email)
    .bind(expires_at)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn consume_email_verification(
    executor: impl PgExecutor<'_>,
    token_hash: &[u8],
    now: DateTime<Utc>,
) -> AppResult<Option<(Uuid, String)>> {
    // Atomic: only consume an unconsumed, unexpired token.
    let row = sqlx::query_as::<_, (Uuid, String)>(
        r#"UPDATE email_verification_tokens
           SET consumed_at = now()
           WHERE token_hash = $1 AND consumed_at IS NULL AND expires_at > $2
           RETURNING user_id, email::text"#,
    )
    .bind(token_hash)
    .bind(now)
    .fetch_optional(executor)
    .await?;
    Ok(row)
}

// --- password reset tokens ------------------------------------------------------

pub async fn insert_password_reset(
    executor: impl PgExecutor<'_>,
    id: Uuid,
    user_id: Uuid,
    token_hash: &[u8],
    expires_at: DateTime<Utc>,
    requested_ip: Option<ipnetwork::IpNetwork>,
) -> AppResult<()> {
    sqlx::query(
        r#"INSERT INTO password_reset_tokens (id, user_id, token_hash, expires_at, requested_ip)
           VALUES ($1,$2,$3,$4,$5)"#,
    )
    .bind(id)
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .bind(requested_ip)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn consume_password_reset(
    executor: impl PgExecutor<'_>,
    token_hash: &[u8],
    now: DateTime<Utc>,
) -> AppResult<Option<Uuid>> {
    let row = sqlx::query_as::<_, (Uuid,)>(
        r#"UPDATE password_reset_tokens
           SET consumed_at = now()
           WHERE token_hash = $1 AND consumed_at IS NULL AND expires_at > $2
           RETURNING user_id"#,
    )
    .bind(token_hash)
    .bind(now)
    .fetch_optional(executor)
    .await?;
    Ok(row.map(|r| r.0))
}
