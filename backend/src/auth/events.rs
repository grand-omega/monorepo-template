use serde_json::Value;
use sqlx::PgPool;
use std::net::IpAddr;
use tracing::warn;
use uuid::Uuid;

/// Kinds of security-relevant events written to the `auth_events` table.
///
/// These are best-effort: a failed insert is logged but never propagated, so the
/// audit log can never break a user-facing flow.
#[derive(Debug, Clone, Copy)]
pub enum EventKind {
    LoginSuccess,
    LoginFailureWrongPassword,
    LoginFailureUnknownEmail,
    LoginBlockedLocked,
    AccountLockedTriggered,
    RefreshRotated,
    RefreshReuseDetected,
    LogoutAll,
    PasswordChanged,
    PasswordResetRequested,
    PasswordResetCompleted,
    EmailVerified,
    AdminLoginSuccess,
    AdminLoginFailureWrongPassword,
    AdminLoginFailureUnknownEmail,
    AdminLoginBlockedLocked,
    AdminUserLocked,
    AdminUserUnlocked,
    AdminEmailVerified,
    AdminSessionsRevoked,
    AdminWebauthnRegistered,
    AdminWebauthnRemoved,
}

impl EventKind {
    fn as_str(self) -> &'static str {
        match self {
            EventKind::LoginSuccess => "login_success",
            EventKind::LoginFailureWrongPassword => "login_failure_wrong_password",
            EventKind::LoginFailureUnknownEmail => "login_failure_unknown_email",
            EventKind::LoginBlockedLocked => "login_blocked_locked",
            EventKind::AccountLockedTriggered => "account_locked_triggered",
            EventKind::RefreshRotated => "refresh_rotated",
            EventKind::RefreshReuseDetected => "refresh_reuse_detected",
            EventKind::LogoutAll => "logout_all",
            EventKind::PasswordChanged => "password_changed",
            EventKind::PasswordResetRequested => "password_reset_requested",
            EventKind::PasswordResetCompleted => "password_reset_completed",
            EventKind::EmailVerified => "email_verified",
            EventKind::AdminLoginSuccess => "admin_login_success",
            EventKind::AdminLoginFailureWrongPassword => "admin_login_failure_wrong_password",
            EventKind::AdminLoginFailureUnknownEmail => "admin_login_failure_unknown_email",
            EventKind::AdminLoginBlockedLocked => "admin_login_blocked_locked",
            EventKind::AdminUserLocked => "admin_user_locked",
            EventKind::AdminUserUnlocked => "admin_user_unlocked",
            EventKind::AdminEmailVerified => "admin_email_verified",
            EventKind::AdminSessionsRevoked => "admin_sessions_revoked",
            EventKind::AdminWebauthnRegistered => "admin_webauthn_registered",
            EventKind::AdminWebauthnRemoved => "admin_webauthn_removed",
        }
    }
}

#[derive(Debug, Default)]
pub struct EventCtx<'a> {
    pub user_id: Option<Uuid>,
    pub ip: Option<IpAddr>,
    pub user_agent: Option<&'a str>,
    pub detail: Option<Value>,
}

/// Fire-and-forget insert. Logs a warning on failure but never returns an error.
pub async fn record(pool: &PgPool, kind: EventKind, ctx: EventCtx<'_>) {
    let id = Uuid::now_v7();
    let ip = ctx.ip.map(ipnetwork::IpNetwork::from);
    let res = sqlx::query(
        r#"INSERT INTO auth_events (id, user_id, event_type, ip, user_agent, detail)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(id)
    .bind(ctx.user_id)
    .bind(kind.as_str())
    .bind(ip)
    .bind(ctx.user_agent)
    .bind(ctx.detail)
    .execute(pool)
    .await;

    if let Err(e) = res {
        warn!(error = ?e, event = kind.as_str(), "failed to write auth_events row");
    }
}
