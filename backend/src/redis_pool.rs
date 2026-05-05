use anyhow::{Context, Result};
use deadpool_redis::{Config as RedisCfg, Pool, Runtime};

pub type RedisPool = Pool;

pub fn connect(redis_url: &str) -> Result<RedisPool> {
    let cfg = RedisCfg::from_url(redis_url);
    cfg.create_pool(Some(Runtime::Tokio1))
        .context("failed to build Redis pool")
}
