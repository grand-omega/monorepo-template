use anyhow::{Context, Result};
use clap::Parser;
use lab_rust_server::AppState;
use lab_rust_server::auth::JwtKeys;
use lab_rust_server::auth::tokens::PreviousPublicKey;
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

    let previous_keys: Vec<PreviousPublicKey> = match cfg.jwt_previous_public_keys.as_deref() {
        Some(s) if !s.trim().is_empty() => serde_json::from_str(s).context(
            "APP_JWT_PREVIOUS_PUBLIC_KEYS must be a JSON array of {kid, public_key_pem}",
        )?,
        _ => Vec::new(),
    };
    let jwt_keys = JwtKeys::from_pem_set(
        cfg.jwt_kid.clone(),
        cfg.jwt_issuer.clone(),
        cfg.jwt_audience.clone(),
        &cfg.jwt_private_key,
        &cfg.jwt_public_key,
        &previous_keys,
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

    let webauthn = webauthn_rs::WebauthnBuilder::new(&cfg.webauthn_rp_id, &cfg.webauthn_rp_origin)
        .context("invalid APP_WEBAUTHN_RP_ID / APP_WEBAUTHN_RP_ORIGIN")?
        .rp_name(&cfg.webauthn_rp_name)
        .build()
        .context("failed to build Webauthn")?;

    let state = AppState {
        config: Arc::new(cfg),
        db: pool.clone(),
        redis,
        jwt_keys: Arc::new(jwt_keys),
        mailer,
        webauthn: Arc::new(webauthn),
    };

    spawn_cleanup_task(state.clone());
    spawn_email_outbox_task(state.clone());

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
            let admin_sessions =
                lab_rust_server::admin::repo::delete_expired_sessions(&state.db, cutoff).await;
            let email_outbox =
                lab_rust_server::email::service::cleanup_retained_auth_token_emails(&state).await;

            match (refresh, verify, reset, admin_sessions, email_outbox) {
                (Ok(r), Ok(v), Ok(p), Ok(a), Ok(e)) => {
                    if r + v + p + a + e > 0 {
                        info!(
                            refresh = r,
                            email_verification = v,
                            password_reset = p,
                            admin_sessions = a,
                            email_outbox = e,
                            "cleaned expired auth tokens"
                        );
                    }
                }
                (refresh, verify, reset, admin_sessions, email_outbox) => {
                    if let Err(e) = refresh {
                        error!(error = ?e, "refresh token cleanup failed");
                    }
                    if let Err(e) = verify {
                        error!(error = ?e, "email verification cleanup failed");
                    }
                    if let Err(e) = reset {
                        error!(error = ?e, "password reset cleanup failed");
                    }
                    if let Err(e) = admin_sessions {
                        error!(error = ?e, "admin session cleanup failed");
                    }
                    if let Err(e) = email_outbox {
                        error!(error = ?e, "email outbox cleanup failed");
                    }
                }
            }
        }
    });
}

fn spawn_email_outbox_task(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            ticker.tick().await;
            lab_rust_server::email::service::dispatch_due(&state, 25).await;
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
