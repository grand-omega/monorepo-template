use crate::config::{Config, LogFormat};
use anyhow::{Context, Result};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init_tracing(config: &Config) -> Result<()> {
    let filter = EnvFilter::try_from_env("APP_LOG")
        .or_else(|_| EnvFilter::try_new("info,sqlx=warn,tower_http=info,hyper=warn"))
        .context("failed to build log filter")?;

    let registry = tracing_subscriber::registry().with(filter);

    match config.log_format {
        LogFormat::Pretty => registry
            .with(tracing_subscriber::fmt::layer().with_target(true))
            .try_init()
            .context("failed to init tracing")?,
        LogFormat::Json => registry
            .with(tracing_subscriber::fmt::layer().json().flatten_event(true))
            .try_init()
            .context("failed to init tracing")?,
    }
    Ok(())
}

pub fn init_metrics() -> Result<PrometheusHandle> {
    let recorder = PrometheusBuilder::new()
        .install_recorder()
        .context("failed to install prometheus recorder")?;
    Ok(recorder)
}
