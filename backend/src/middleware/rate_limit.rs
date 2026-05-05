use crate::AppState;
use crate::middleware::peer::client_ip;
use crate::redis_pool::RedisPool;
use axum::body::Body;
use axum::extract::ConnectInfo;
use http::{HeaderValue, Request, Response, StatusCode};
use ipnetwork::IpNetwork;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::warn;

/// Rate-limit class — same buckets as before, expressed as
/// `(refill_rate_per_second, burst_capacity)`.
#[derive(Clone, Copy, Debug)]
pub enum Class {
    /// 3 requests per hour per IP — for password reset / resend verification.
    VeryStrict,
    /// 10 requests per minute per IP — for register, login, verify, reset confirm.
    Strict,
    /// 60 requests per minute per IP — for refresh.
    Medium,
    /// 30 requests per minute per IP — general low-volume protected ops.
    Low,
}

impl Class {
    fn tag(self) -> &'static str {
        match self {
            Class::VeryStrict => "vs",
            Class::Strict => "s",
            Class::Medium => "m",
            Class::Low => "l",
        }
    }

    /// (tokens-per-second refill rate, max bucket size).
    fn bucket(self) -> (f64, u32) {
        match self {
            // 3 per hour: 1 token / 1200s.
            Class::VeryStrict => (1.0 / 1200.0, 3),
            Class::Strict => (1.0 / 6.0, 10),
            Class::Medium => (1.0, 60),
            Class::Low => (0.5, 30),
        }
    }
}

const LUA_SCRIPT: &str = r#"
-- KEYS[1] bucket key   ARGV[1] rate (tokens/sec, float)
-- ARGV[2] burst (int)  ARGV[3] now_ms (int)   ARGV[4] ttl_ms (int)
-- Returns 0 if allowed, else ms until 1 token is available.
local key = KEYS[1]
local rate = tonumber(ARGV[1])
local burst = tonumber(ARGV[2])
local now = tonumber(ARGV[3])
local ttl = tonumber(ARGV[4])

local data = redis.call('HMGET', key, 'tokens', 'last_refill')
local tokens = tonumber(data[1])
local last = tonumber(data[2])

if tokens == nil then
  tokens = burst
  last = now
else
  local elapsed = now - last
  if elapsed < 0 then elapsed = 0 end
  tokens = math.min(burst, tokens + elapsed * rate / 1000.0)
  last = now
end

local result
if tokens >= 1 then
  tokens = tokens - 1
  result = 0
else
  local needed = 1 - tokens
  result = math.ceil(needed * 1000.0 / rate)
end

redis.call('HMSET', key, 'tokens', tostring(tokens), 'last_refill', tostring(last))
redis.call('PEXPIRE', key, ttl)
return result
"#;

#[derive(Clone)]
pub struct RateLimitLayer {
    class: Class,
    redis: RedisPool,
    trusted: Arc<Vec<IpNetwork>>,
}

pub fn layer(state: &AppState, class: Class) -> RateLimitLayer {
    RateLimitLayer {
        class,
        redis: state.redis.clone(),
        trusted: Arc::new(state.config.trusted_proxy_cidrs.clone()),
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            class: self.class,
            redis: self.redis.clone(),
            trusted: self.trusted.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    class: Class,
    redis: RedisPool,
    trusted: Arc<Vec<IpNetwork>>,
}

impl<S> Service<Request<Body>> for RateLimitService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Send + Clone + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response<Body>, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let class = self.class;
        let redis = self.redis.clone();
        let trusted = self.trusted.clone();
        // Standard tower clone-and-swap so the cloned `inner` is the one already
        // poll_ready'd — see tower docs on Service::call.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let peer_ip = req
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|c| c.0.ip())
                .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
            let ip = client_ip(req.headers(), peer_ip, &trusted);

            match check_quota(&redis, class, ip).await {
                QuotaResult::Allow => inner.call(req).await,
                QuotaResult::Deny { retry_after_secs } => {
                    metrics::counter!("rate_limit_blocked_total", "class" => class.tag())
                        .increment(1);
                    Ok(too_many_requests(retry_after_secs))
                }
                QuotaResult::Error => {
                    // Fail-open: a flaky cache must not take down auth.
                    metrics::counter!("rate_limit_redis_errors_total", "class" => class.tag())
                        .increment(1);
                    inner.call(req).await
                }
            }
        })
    }
}

enum QuotaResult {
    Allow,
    Deny { retry_after_secs: u64 },
    Error,
}

async fn check_quota(pool: &RedisPool, class: Class, ip: IpAddr) -> QuotaResult {
    let (rate, burst) = class.bucket();
    let key = format!("rl:{}:{}", class.tag(), ip);
    let now_ms = chrono::Utc::now().timestamp_millis();
    // TTL = 2× the time it would take to fully refill a bucket from empty.
    let ttl_ms = ((burst as f64 / rate) * 2000.0) as i64;

    let mut conn = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = ?e, "rate-limit: redis pool error; failing open");
            return QuotaResult::Error;
        }
    };

    let script = redis::Script::new(LUA_SCRIPT);
    let res: redis::RedisResult<i64> = script
        .key(&key)
        .arg(rate)
        .arg(burst as i64)
        .arg(now_ms)
        .arg(ttl_ms)
        .invoke_async(&mut *conn)
        .await;

    match res {
        Ok(0) => QuotaResult::Allow,
        Ok(retry_ms) if retry_ms > 0 => QuotaResult::Deny {
            retry_after_secs: ((retry_ms as f64) / 1000.0).ceil().max(1.0) as u64,
        },
        Ok(_) => QuotaResult::Allow,
        Err(e) => {
            warn!(error = ?e, "rate-limit: redis script error; failing open");
            QuotaResult::Error
        }
    }
}

fn too_many_requests(retry_after_secs: u64) -> Response<Body> {
    let body = r#"{"code":"rate_limited","message":"too many requests"}"#;
    let mut resp = Response::new(Body::from(body));
    *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;
    resp.headers_mut().insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    if let Ok(v) = HeaderValue::from_str(&retry_after_secs.to_string()) {
        resp.headers_mut().insert(http::header::RETRY_AFTER, v);
    }
    resp
}
