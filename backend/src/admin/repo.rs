use crate::error::AppResult;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AdminUserRow {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub locked_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AdminSessionRow {
    pub admin_user_id: Uuid,
    pub email: String,
    pub csrf_hash: Vec<u8>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ManagedUserRow {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub failed_login_count: i32,
    pub locked_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthEventRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub user_email: Option<String>,
    pub event_type: String,
    pub ip: Option<ipnetwork::IpNetwork>,
    pub user_agent: Option<String>,
    pub detail: Option<Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserSessionRow {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub ip: Option<ipnetwork::IpNetwork>,
    pub user_agent: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
}

pub async fn find_admin_by_email(
    executor: impl PgExecutor<'_>,
    email: &str,
) -> AppResult<Option<AdminUserRow>> {
    let row = sqlx::query_as::<_, AdminUserRow>(
        r#"SELECT id, email::text AS email, password_hash, locked_until
           FROM users
           WHERE email = $1::citext
             AND role = 'admin'
             AND deleted_at IS NULL"#,
    )
    .bind(email)
    .fetch_optional(executor)
    .await?;
    Ok(row)
}

pub async fn find_admin_by_id(
    executor: impl PgExecutor<'_>,
    id: Uuid,
) -> AppResult<Option<AdminUserRow>> {
    let row = sqlx::query_as::<_, AdminUserRow>(
        r#"SELECT id, email::text AS email, password_hash, locked_until
           FROM users
           WHERE id = $1
             AND role = 'admin'
             AND deleted_at IS NULL"#,
    )
    .bind(id)
    .fetch_optional(executor)
    .await?;
    Ok(row)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_session(
    executor: impl PgExecutor<'_>,
    id: Uuid,
    admin_user_id: Uuid,
    token_hash: &[u8],
    csrf_hash: &[u8],
    expires_at: DateTime<Utc>,
    ip: Option<ipnetwork::IpNetwork>,
    user_agent: Option<&str>,
) -> AppResult<()> {
    sqlx::query(
        r#"INSERT INTO admin_sessions
            (id, admin_user_id, token_hash, csrf_hash, expires_at, ip, user_agent)
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(id)
    .bind(admin_user_id)
    .bind(token_hash)
    .bind(csrf_hash)
    .bind(expires_at)
    .bind(ip)
    .bind(user_agent)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn find_session(
    pool: &PgPool,
    id: Uuid,
    token_hash: &[u8],
    now: DateTime<Utc>,
) -> AppResult<Option<AdminSessionRow>> {
    let row = sqlx::query_as::<_, AdminSessionRow>(
        r#"UPDATE admin_sessions s
           SET last_seen_at = now()
           FROM users u
           WHERE s.id = $1
             AND s.token_hash = $2
             AND s.revoked_at IS NULL
             AND s.expires_at > $3
             AND s.admin_user_id = u.id
             AND u.role = 'admin'
             AND u.deleted_at IS NULL
           RETURNING s.admin_user_id, u.email::text AS email, s.csrf_hash"#,
    )
    .bind(id)
    .bind(token_hash)
    .bind(now)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn revoke_session(pool: &PgPool, id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"UPDATE admin_sessions
           SET revoked_at = COALESCE(revoked_at, now())
           WHERE id = $1"#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_users(
    pool: &PgPool,
    email_query: Option<&str>,
    limit: i64,
) -> AppResult<Vec<ManagedUserRow>> {
    list_users_paged(pool, email_query, None, limit).await
}

pub async fn list_users_paged(
    pool: &PgPool,
    email_query: Option<&str>,
    cursor: Option<Uuid>,
    limit: i64,
) -> AppResult<Vec<ManagedUserRow>> {
    let like = email_query.map(|q| format!("%{}%", q.trim()));
    let rows = sqlx::query_as::<_, ManagedUserRow>(
        r#"SELECT id, email::text AS email, role, email_verified, display_name,
                  created_at, last_login_at, failed_login_count, locked_until
           FROM users
           WHERE deleted_at IS NULL
             AND ($1::text IS NULL OR email::text ILIKE $1)
             AND ($2::uuid IS NULL OR id < $2)
           ORDER BY id DESC
           LIMIT $3"#,
    )
    .bind(like)
    .bind(cursor)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn find_managed_user(pool: &PgPool, id: Uuid) -> AppResult<Option<ManagedUserRow>> {
    let row = sqlx::query_as::<_, ManagedUserRow>(
        r#"SELECT id, email::text AS email, role, email_verified, display_name,
                  created_at, last_login_at, failed_login_count, locked_until
           FROM users
           WHERE id = $1 AND deleted_at IS NULL"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn list_user_sessions(pool: &PgPool, user_id: Uuid) -> AppResult<Vec<UserSessionRow>> {
    let rows = sqlx::query_as::<_, UserSessionRow>(
        r#"WITH family_summary AS (
               SELECT family_id,
                      min(issued_at) AS created_at,
                      max(used_at) AS last_used_at,
                      CASE
                          WHEN bool_or(revoked_at IS NULL AND expires_at > now()) THEN NULL
                          ELSE max(revoked_at)
                      END AS revoked_at
               FROM refresh_tokens
               WHERE user_id = $1
               GROUP BY family_id
           ),
           latest_token AS (
               SELECT DISTINCT ON (family_id)
                      family_id, ip, user_agent
               FROM refresh_tokens
               WHERE user_id = $1
               ORDER BY family_id, issued_at DESC, id DESC
           )
           SELECT s.family_id AS id,
                  s.created_at,
                  s.last_used_at,
                  l.ip,
                  l.user_agent,
                  s.revoked_at
           FROM family_summary s
           JOIN latest_token l ON l.family_id = s.family_id
           ORDER BY s.created_at DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_auth_events(pool: &PgPool, limit: i64) -> AppResult<Vec<AuthEventRow>> {
    list_auth_events_filtered(pool, None, None, None, limit).await
}

/// UUID v7 cursor (rows ordered by id desc since v7 is time-sortable). Pass
/// `cursor = Some(last_id_seen)` to fetch the next page.
pub async fn list_auth_events_filtered(
    pool: &PgPool,
    event_type: Option<&str>,
    user_id: Option<Uuid>,
    cursor: Option<Uuid>,
    limit: i64,
) -> AppResult<Vec<AuthEventRow>> {
    let rows = sqlx::query_as::<_, AuthEventRow>(
        r#"SELECT e.id, e.user_id, u.email::text AS user_email, e.event_type, e.ip,
                  e.user_agent, e.detail, e.created_at
           FROM auth_events e
           LEFT JOIN users u ON u.id = e.user_id
           WHERE ($1::text IS NULL OR e.event_type = $1)
             AND ($2::uuid IS NULL OR e.user_id = $2)
             AND ($3::uuid IS NULL OR e.id < $3)
           ORDER BY e.id DESC
           LIMIT $4"#,
    )
    .bind(event_type)
    .bind(user_id)
    .bind(cursor)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn delete_expired_sessions(pool: &PgPool, older_than: DateTime<Utc>) -> AppResult<u64> {
    let res = sqlx::query(
        r#"DELETE FROM admin_sessions
           WHERE expires_at < $1 OR revoked_at < $1"#,
    )
    .bind(older_than)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}
