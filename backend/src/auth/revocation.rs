//! Per-user access-token revocation epoch, stored in Redis.
//!
//! When a flow needs to invalidate every access token previously issued for a
//! user (logout-all, password change, password reset, admin-revoke-sessions,
//! account deletion), it writes the current Unix epoch (seconds) under
//! `user_rev:<uuid>`. The auth middleware reads this on each authenticated
//! request and rejects tokens whose `iat` predates the recorded epoch.
//!
//! The TTL on the Redis key is `access_token_ttl + 60s`, just enough to keep
//! the marker alive for as long as a previously-issued token could still be
//! presented. After that, the key auto-expires and the read becomes a no-op.
//!
//! Failure mode: this module logs Redis errors and returns "not revoked" / "no
//! marker written" to the caller (fail-open). The reasoning matches the rate
//! limiter: a Redis outage must not disable authentication for everyone.
//! Short access-token TTLs and refresh-token revocation in Postgres bound the
//! window during which an unrevoked old access token can still be used.

use crate::redis_pool::RedisPool;
use redis::AsyncCommands;
use std::time::Duration;
use tracing::{error, warn};
use uuid::Uuid;

fn key(user_id: Uuid) -> String {
    format!("user_rev:{user_id}")
}

/// Mark every access token issued for `user_id` before now as revoked.
/// Logs Redis errors but never returns one — a Redis hiccup must not break the
/// surrounding business operation (which has typically already revoked the
/// refresh tokens in Postgres).
pub async fn revoke_user(pool: &RedisPool, user_id: Uuid, access_ttl: Duration) {
    let mut conn = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            metrics::counter!("auth_revocation_redis_errors_total", "op" => "set_conn")
                .increment(1);
            error!(error = ?e, %user_id, "revocation: redis pool error; access tokens NOT revoked in cache");
            return;
        }
    };
    let now = chrono::Utc::now().timestamp();
    // +60s slack so the marker outlives any token that was issued just before
    // the revocation point.
    let ttl_secs = access_ttl.as_secs() + 60;
    let res: redis::RedisResult<()> = conn.set_ex(key(user_id), now, ttl_secs).await;
    if let Err(e) = res {
        metrics::counter!("auth_revocation_redis_errors_total", "op" => "set").increment(1);
        error!(error = ?e, %user_id, "revocation: redis SET error; access tokens NOT revoked in cache");
    }
}

/// Returns true if `iat` predates a revocation marker for this user.
/// Returns false on Redis errors (fail-open) and emits a warning + metric.
pub async fn is_revoked(pool: &RedisPool, user_id: Uuid, iat: i64) -> bool {
    let mut conn = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            metrics::counter!("auth_revocation_redis_errors_total", "op" => "get_conn")
                .increment(1);
            warn!(error = ?e, "revocation: redis pool error; failing open");
            return false;
        }
    };
    let res: redis::RedisResult<Option<i64>> = conn.get(key(user_id)).await;
    match res {
        Ok(None) => false,
        // Strict `iat < rev`: a token issued at the same epoch second as the
        // revocation is treated as still valid. JWT `iat` only carries second
        // resolution, so we cannot distinguish "issued just before" from
        // "issued just after" the revocation point. Flows that revoke and
        // immediately mint a new token (e.g. password change) rely on this:
        // the new token's `iat` equals (or exceeds) `rev`, so it stays valid.
        Ok(Some(rev)) => iat < rev,
        Err(e) => {
            metrics::counter!("auth_revocation_redis_errors_total", "op" => "get").increment(1);
            warn!(error = ?e, "revocation: redis GET error; failing open");
            false
        }
    }
}
