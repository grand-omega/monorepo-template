use crate::error::AppResult;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailOutboxRow {
    pub id: Uuid,
    pub recipient: String,
    pub subject: String,
    pub html_body: String,
    pub text_body: String,
    pub attempts: i32,
}

pub async fn enqueue(
    pool: &PgPool,
    recipient: &str,
    subject: &str,
    html_body: &str,
    text_body: &str,
) -> AppResult<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO email_outbox (id, recipient, subject, html_body, text_body)
           VALUES ($1, $2::citext, $3, $4, $5)"#,
    )
    .bind(id)
    .bind(recipient)
    .bind(subject)
    .bind(html_body)
    .bind(text_body)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn find(pool: &PgPool, id: Uuid) -> AppResult<Option<EmailOutboxRow>> {
    let row = sqlx::query_as::<_, EmailOutboxRow>(
        r#"SELECT id, recipient::text AS recipient, subject, html_body, text_body, attempts
           FROM email_outbox
           WHERE id = $1 AND status IN ('pending', 'failed', 'sending')"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn claim_due(
    pool: &PgPool,
    limit: i64,
    now: DateTime<Utc>,
) -> AppResult<Vec<EmailOutboxRow>> {
    let rows = sqlx::query_as::<_, EmailOutboxRow>(
        r#"UPDATE email_outbox
           SET status = 'sending', locked_at = now(), updated_at = now()
           WHERE id IN (
               SELECT id
               FROM email_outbox
               WHERE status IN ('pending', 'failed')
                 AND next_attempt_at <= $1
               ORDER BY next_attempt_at ASC, id ASC
               LIMIT $2
               FOR UPDATE SKIP LOCKED
           )
           RETURNING id, recipient::text AS recipient, subject, html_body, text_body, attempts"#,
    )
    .bind(now)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn mark_sending(pool: &PgPool, id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"UPDATE email_outbox
           SET status = 'sending', locked_at = now(), updated_at = now()
           WHERE id = $1 AND status IN ('pending', 'failed')"#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_sent(pool: &PgPool, id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"UPDATE email_outbox
           SET status = 'sent', sent_at = now(), locked_at = NULL, updated_at = now()
           WHERE id = $1"#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_failed(
    pool: &PgPool,
    id: Uuid,
    attempts: i32,
    error: &str,
    next_attempt_at: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"UPDATE email_outbox
           SET status = 'failed',
               attempts = $2,
               last_error = $3,
               next_attempt_at = $4,
               locked_at = NULL,
               updated_at = now()
           WHERE id = $1"#,
    )
    .bind(id)
    .bind(attempts)
    .bind(error.chars().take(1000).collect::<String>())
    .bind(next_attempt_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn requeue_stale_sending(pool: &PgPool, before: DateTime<Utc>) -> AppResult<u64> {
    let res = sqlx::query(
        r#"UPDATE email_outbox
           SET status = 'failed',
               next_attempt_at = now(),
               locked_at = NULL,
               updated_at = now(),
               last_error = COALESCE(last_error, 'stale sending lock requeued')
           WHERE status = 'sending' AND locked_at < $1"#,
    )
    .bind(before)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}
