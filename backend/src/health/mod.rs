use crate::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use std::time::Duration;

pub async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

#[derive(Serialize)]
pub struct ReadyzResponse {
    pub db: &'static str,
    pub redis: &'static str,
}

pub async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = tokio::time::timeout(Duration::from_millis(500), async {
        sqlx::query("SELECT 1").fetch_one(&state.db).await.is_ok()
    })
    .await
    .unwrap_or(false);

    let redis_ok = tokio::time::timeout(Duration::from_millis(500), async {
        match state.redis.get().await {
            Ok(mut conn) => {
                let r: redis::RedisResult<String> = redis::cmd("PING").query_async(&mut conn).await;
                r.is_ok()
            }
            Err(_) => false,
        }
    })
    .await
    .unwrap_or(false);

    let body = ReadyzResponse {
        db: if db_ok { "ok" } else { "fail" },
        redis: if redis_ok { "ok" } else { "fail" },
    };
    let status = if db_ok && redis_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(body))
}
