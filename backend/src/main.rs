use anyhow::{Context, Result};
use clap::Parser;
use lab_rust_server::AppState;
use lab_rust_server::auth::JwtKeys;
use lab_rust_server::config::Config;
use lab_rust_server::email::mailer::{DynMailer, NoopMailer, SmtpMailer};
use lab_rust_server::{db, redis_pool, router, telemetry};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "lab-rust-server", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Parser, Debug)]
enum Cmd {
    /// Run the HTTP server (default).
    Serve,
    /// Run database migrations and exit.
    Migrate,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cmd = cli.cmd.unwrap_or(Cmd::Serve);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        match cmd {
            Cmd::Serve => run_serve().await,
            Cmd::Migrate => run_migrate().await,
        }
    })
}

async fn run_migrate() -> Result<()> {
    let cfg = Config::from_env()?;
    telemetry::init_tracing(&cfg)?;
    info!("running migrations");
    let pool = db::connect(&cfg.database_url, 2).await?;
    db::run_migrations(&pool).await?;
    info!("migrations complete");
    Ok(())
}

async fn run_serve() -> Result<()> {
    let cfg = Config::from_env()?;
    telemetry::init_tracing(&cfg)?;
    let metrics_handle = Arc::new(telemetry::init_metrics()?);

    info!(env = ?cfg.env, bind = %cfg.bind_addr, "starting server");

    let pool = db::connect(&cfg.database_url, cfg.database_max_connections).await?;
    if cfg.migrate_on_start {
        info!("MIGRATE_ON_START=true — running migrations");
        db::run_migrations(&pool).await?;
    }
    let redis = redis_pool::connect(&cfg.redis_url)?;

    let jwt_keys = JwtKeys::from_pem(
        cfg.jwt_kid.clone(),
        cfg.jwt_issuer.clone(),
        cfg.jwt_audience.clone(),
        &cfg.jwt_private_key,
        &cfg.jwt_public_key,
    )?;

    let mailer: DynMailer = match SmtpMailer::new(
        &cfg.smtp_url,
        &cfg.smtp_from,
        &cfg.smtp_from_name,
    ) {
        Ok(m) => Arc::new(m),
        Err(e) if cfg.allow_noop_mailer => {
            error!(
                error = ?e,
                "SMTP init failed; APP_ALLOW_NOOP_MAILER=true → using NoopMailer (verification emails will be dropped)"
            );
            Arc::new(NoopMailer)
        }
        Err(e) => {
            return Err(e).context(
                "failed to init SMTP mailer; refusing to start. \
                 Fix APP_SMTP_URL or set APP_ALLOW_NOOP_MAILER=true (dev/test only).",
            );
        }
    };

    let bind_addr = cfg.bind_addr;
    let metrics_addr = cfg.metrics_bind_addr;

    let state = AppState {
        config: Arc::new(cfg),
        db: pool.clone(),
        redis,
        jwt_keys: Arc::new(jwt_keys),
        mailer,
    };

    spawn_cleanup_task(state.clone());

    let app_router = router::build_router(state.clone());

    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("failed to bind {bind_addr}"))?;
    info!("listening on {bind_addr}");

    let metrics_router = axum::Router::new()
        .route(
            "/metrics",
            axum::routing::get(lab_rust_server::metrics_route::render),
        )
        .with_state(lab_rust_server::metrics_route::MetricsState {
            handle: metrics_handle.clone(),
        });
    let metrics_listener = TcpListener::bind(metrics_addr)
        .await
        .with_context(|| format!("failed to bind metrics {metrics_addr}"))?;
    info!("metrics listening on {metrics_addr}");

    let metrics_task = tokio::spawn(async move {
        if let Err(e) = axum::serve(metrics_listener, metrics_router)
            .with_graceful_shutdown(shutdown_signal())
            .await
        {
            error!(error = ?e, "metrics server error");
        }
    });

    let main_task = tokio::spawn(async move {
        if let Err(e) = axum::serve(
            listener,
            app_router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal())
        .await
        {
            error!(error = ?e, "main server error");
        }
    });

    let _ = tokio::join!(main_task, metrics_task);

    info!("draining DB pool");
    pool.close().await;
    info!("shutdown complete");
    Ok(())
}

fn spawn_cleanup_task(state: AppState) {
    use lab_rust_server::auth::repo;

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(60 * 60));
        loop {
            ticker.tick().await;
            let cutoff = chrono::Utc::now() - chrono::Duration::days(30);

            let refresh = repo::delete_expired_refresh(&state.db, cutoff).await;
            let verify = repo::delete_expired_email_verification(&state.db, cutoff).await;
            let reset = repo::delete_expired_password_reset(&state.db, cutoff).await;

            match (refresh, verify, reset) {
                (Ok(r), Ok(v), Ok(p)) => {
                    if r + v + p > 0 {
                        info!(
                            refresh = r,
                            email_verification = v,
                            password_reset = p,
                            "cleaned expired auth tokens"
                        );
                    }
                }
                (refresh, verify, reset) => {
                    if let Err(e) = refresh {
                        error!(error = ?e, "refresh token cleanup failed");
                    }
                    if let Err(e) = verify {
                        error!(error = ?e, "email verification cleanup failed");
                    }
                    if let Err(e) = reset {
                        error!(error = ?e, "password reset cleanup failed");
                    }
                }
            }
        }
    });
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        let mut term = signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler");
        term.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("received Ctrl-C, shutting down"),
        _ = terminate => info!("received SIGTERM, shutting down"),
    }
}
