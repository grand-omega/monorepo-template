use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub email_verified: bool,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub failed_login_count: i32,
    pub locked_until: Option<DateTime<Utc>>,
}

impl User {
    pub fn is_locked(&self, now: DateTime<Utc>) -> bool {
        self.locked_until.map(|t| t > now).unwrap_or(false)
    }
}
