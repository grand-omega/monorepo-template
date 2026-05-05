use governor::middleware::NoOpMiddleware;
use std::sync::Arc;
use tower_governor::GovernorLayer;
use tower_governor::governor::{GovernorConfig, GovernorConfigBuilder};
use tower_governor::key_extractor::SmartIpKeyExtractor;

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

pub type RateLimitLayer =
    GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware, axum::body::Body>;

pub fn layer(class: Class) -> RateLimitLayer {
    let (per, burst) = match class {
        Class::VeryStrict => (1200u64, 3u32), // 1 every 1200s, burst 3 → 3/hour
        Class::Strict => (6, 10),             // ~10/min
        Class::Medium => (1, 60),             // ~60/min
        Class::Low => (2, 30),                // ~30/min
    };
    let cfg: GovernorConfig<SmartIpKeyExtractor, NoOpMiddleware> = GovernorConfigBuilder::default()
        .per_second(per)
        .burst_size(burst)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .expect("governor config valid");
    GovernorLayer::new(Arc::new(cfg))
}
