use crate::AppState;
use crate::email::outbox::{self, EmailKind, EmailOutboxRow};
use crate::email::templates;
use chrono::Utc;
use tracing::{error, warn};
use uuid::Uuid;

const APP_NAME: &str = "Lab Rust Server";

pub async fn send_verification(state: &AppState, to: &str, verify_url: &str) {
    let rendered = match templates::render_verify_email(verify_url, APP_NAME) {
        Ok(r) => r,
        Err(e) => {
            error!(error = ?e, "failed to render verify email template");
            return;
        }
    };
    enqueue_and_try_send(
        state,
        EmailKind::EmailVerification,
        to,
        "Verify your email",
        rendered.html,
        rendered.text,
    )
    .await;
}

pub async fn send_password_reset(state: &AppState, to: &str, reset_url: &str) {
    let rendered = match templates::render_password_reset(reset_url, APP_NAME) {
        Ok(r) => r,
        Err(e) => {
            error!(error = ?e, "failed to render password reset template");
            return;
        }
    };
    enqueue_and_try_send(
        state,
        EmailKind::PasswordReset,
        to,
        "Reset your password",
        rendered.html,
        rendered.text,
    )
    .await;
}

async fn enqueue_and_try_send(
    state: &AppState,
    kind: EmailKind,
    to: &str,
    subject: &str,
    html_body: String,
    text_body: String,
) {
    let id = match outbox::enqueue(&state.db, kind, to, subject, &html_body, &text_body).await {
        Ok(id) => id,
        Err(e) => {
            error!(error = ?e, recipient = %to, subject, "failed to enqueue email");
            return;
        }
    };
    if let Err(e) = try_send_one(state, id).await {
        warn!(error = ?e, email_id = %id, "email delivery deferred for retry");
    }
}

pub async fn dispatch_due(state: &AppState, limit: i64) {
    let stale_before = Utc::now() - chrono::Duration::minutes(10);
    if let Err(e) = outbox::requeue_stale_sending(&state.db, stale_before).await {
        warn!(error = ?e, "failed to requeue stale email outbox rows");
    }

    let rows = match outbox::claim_due(&state.db, limit, Utc::now()).await {
        Ok(rows) => rows,
        Err(e) => {
            error!(error = ?e, "failed to claim due email outbox rows");
            return;
        }
    };

    for row in rows {
        if let Err(e) = deliver_claimed(state, row).await {
            warn!(error = ?e, "email delivery deferred for retry");
        }
    }
}

async fn try_send_one(state: &AppState, id: Uuid) -> anyhow::Result<()> {
    outbox::mark_sending(&state.db, id).await?;
    let Some(row) = outbox::find(&state.db, id).await? else {
        return Ok(());
    };
    deliver_claimed(state, row).await
}

async fn deliver_claimed(state: &AppState, row: EmailOutboxRow) -> anyhow::Result<()> {
    match state
        .mailer
        .send(
            &row.recipient,
            &row.subject,
            row.html_body.clone(),
            row.text_body.clone(),
        )
        .await
    {
        Ok(()) => {
            outbox::mark_sent(&state.db, row.id).await?;
            Ok(())
        }
        Err(e) => {
            let attempts = row.attempts + 1;
            if row.is_auth_token_email() {
                outbox::mark_failed_scrubbed(&state.db, row.id, attempts, &format!("{e:?}"))
                    .await?;
                return Err(e);
            }
            let delay = retry_delay(attempts);
            outbox::mark_failed(
                &state.db,
                row.id,
                attempts,
                &format!("{e:?}"),
                Utc::now() + delay,
            )
            .await?;
            Err(e)
        }
    }
}

fn retry_delay(attempts: i32) -> chrono::Duration {
    let minutes = match attempts {
        0 | 1 => 1,
        2 => 5,
        3 => 15,
        4 => 60,
        _ => 6 * 60,
    };
    chrono::Duration::minutes(minutes)
}

pub async fn cleanup_retained_auth_token_emails(state: &AppState) -> crate::AppResult<u64> {
    let now = Utc::now();
    outbox::delete_retained_auth_token_emails(
        &state.db,
        now - chrono::Duration::days(7),
        now - chrono::Duration::hours(24),
    )
    .await
}
