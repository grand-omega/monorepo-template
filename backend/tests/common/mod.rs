// Test harness shared by integration tests. Spawns Postgres + Redis via
// testcontainers, builds the real Axum router, and binds it on a random port.
//
// One container set per `spawn_app()` call. Containers are dropped when
// `TestApp` drops, so each test has full isolation at the cost of
// ~1-2s startup time for PG.

use async_trait::async_trait;
use ed25519_dalek::SigningKey;
use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey};
use lab_rust_server::auth::JwtKeys;
use lab_rust_server::auth::refresh::fill_random;
use lab_rust_server::config::{AppEnv, Config, LogFormat};
use lab_rust_server::email::mailer::{DynMailer, Mailer};
use lab_rust_server::{AppState, db, redis_pool, router};
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::Redis;
use tokio::net::TcpListener;
use url::Url;

#[allow(dead_code)] // some tests don't touch every field
pub struct TestApp {
    pub base_url: String,
    pub db: PgPool,
    pub jwt_keys: Arc<JwtKeys>,
    pub mailer: Arc<CapturingMailer>,
    // Containers must outlive the test. Field order matters: server stops
    // first (drop order = decl order), then we tear down infra.
    _server: tokio::task::JoinHandle<()>,
    _pg: ContainerAsync<Postgres>,
    _redis: ContainerAsync<Redis>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CapturedEmail {
    pub to: String,
    pub subject: String,
    pub text: String,
}

#[derive(Default)]
pub struct CapturingMailer {
    pub sent: Mutex<Vec<CapturedEmail>>,
}

#[async_trait]
impl Mailer for CapturingMailer {
    async fn send(
        &self,
        to: &str,
        subject: &str,
        _html: String,
        text: String,
    ) -> anyhow::Result<()> {
        self.sent.lock().unwrap().push(CapturedEmail {
            to: to.to_string(),
            subject: subject.to_string(),
            text,
        });
        Ok(())
    }
}

impl CapturingMailer {
    /// Extract the first URL whose path contains `path_fragment` from the most
    /// recent email sent to `to`. Body templates render the link as a bare URL
    /// on its own line, so a whitespace-bounded scan is enough.
    #[allow(dead_code)]
    pub fn last_url_to(&self, to: &str, path_fragment: &str) -> Option<String> {
        let sent = self.sent.lock().unwrap();
        for captured in sent.iter().rev() {
            if captured.to != to {
                continue;
            }
            for tok in captured.text.split_whitespace() {
                if (tok.starts_with("http://") || tok.starts_with("https://"))
                    && tok.contains(path_fragment)
                {
                    return Some(tok.to_string());
                }
            }
        }
        None
    }
}

pub async fn spawn_app() -> TestApp {
    let pg = Postgres::default()
        .start()
        .await
        .expect("start postgres container");
    let pg_port = pg
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres host port");
    let database_url = format!("postgres://postgres:postgres@127.0.0.1:{pg_port}/postgres");

    let redis = Redis::default()
        .start()
        .await
        .expect("start redis container");
    let redis_port = redis
        .get_host_port_ipv4(6379)
        .await
        .expect("redis host port");
    let redis_url = format!("redis://127.0.0.1:{redis_port}");

    let (priv_pem, pub_pem) = ed25519_pem();
    let jwt_keys = Arc::new(
        JwtKeys::from_pem("test-kid", "test-iss", "test-aud", &priv_pem, &pub_pem)
            .expect("build JwtKeys from generated PEMs"),
    );

    let pool = db::connect(&database_url, 5).await.expect("connect pg");
    db::run_migrations(&pool).await.expect("migrate");
    let redis_pool = redis_pool::connect(&redis_url).expect("redis pool");

    let config = Config {
        env: AppEnv::Test,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        metrics_bind_addr: "127.0.0.1:0".parse().unwrap(),
        public_base_url: Url::parse("http://test.invalid/").unwrap(),
        admin_ui_dir: None,
        database_url,
        database_max_connections: 5,
        redis_url,
        jwt_private_key: priv_pem,
        jwt_public_key: pub_pem,
        jwt_kid: "test-kid".into(),
        jwt_issuer: "test-iss".into(),
        jwt_audience: "test-aud".into(),
        jwt_previous_public_keys: None,
        access_token_ttl: Duration::from_secs(900),
        refresh_token_ttl: Duration::from_secs(60 * 60 * 24 * 7),
        smtp_url: "smtp://localhost:1".into(),
        smtp_from: "test@test.invalid".into(),
        smtp_from_name: "Test".into(),
        allow_noop_mailer: true,
        // Tiny argon2 params keep the suite fast.
        argon2_m_cost: 8,
        argon2_t_cost: 1,
        argon2_p_cost: 1,
        login_max_failures: 100,
        login_lock_duration: Duration::from_secs(900),
        cors_allowed_origins: vec![],
        // Empty: tests that connect from 127.0.0.1 are "untrusted peers", so
        // X-Forwarded-For is intentionally ignored. Tests that need to
        // exercise the trusted-proxy path can add 127.0.0.1/32 here per-test.
        trusted_proxy_cidrs: vec![],
        migrate_on_start: false,
        log_format: LogFormat::Pretty,
        webauthn_rp_id: "localhost".to_string(),
        webauthn_rp_origin: Url::parse("http://localhost:5173").unwrap(),
        webauthn_rp_name: "Admin Console (test)".to_string(),
    };

    let capturing = Arc::new(CapturingMailer::default());
    let mailer: DynMailer = capturing.clone();
    let webauthn =
        webauthn_rs::WebauthnBuilder::new(&config.webauthn_rp_id, &config.webauthn_rp_origin)
            .expect("webauthn builder")
            .rp_name(&config.webauthn_rp_name)
            .build()
            .expect("webauthn build");
    let state = AppState {
        config: Arc::new(config),
        db: pool.clone(),
        redis: redis_pool,
        jwt_keys: jwt_keys.clone(),
        mailer,
        webauthn: Arc::new(webauthn),
    };

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let app = router::build_router(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("axum::serve");
    });

    TestApp {
        base_url: format!("http://{addr}"),
        db: pool,
        jwt_keys,
        mailer: capturing,
        _server: server,
        _pg: pg,
        _redis: redis,
    }
}

fn ed25519_pem() -> (String, String) {
    let mut seed = [0u8; 32];
    fill_random(&mut seed);
    let signing = SigningKey::from_bytes(&seed);
    let priv_pem = signing
        .to_pkcs8_pem(LineEnding::LF)
        .expect("encode pkcs8 priv")
        .to_string();
    let pub_pem = signing
        .verifying_key()
        .to_public_key_pem(LineEnding::LF)
        .expect("encode public pem");
    (priv_pem, pub_pem)
}

/// Mark a registered email as verified directly in the DB. Useful when a test
/// needs an authenticated session without exercising the email-link flow.
#[allow(dead_code)]
pub async fn force_verify_email(db: &PgPool, email: &str) {
    sqlx::query("UPDATE users SET email_verified = TRUE WHERE email = $1::citext")
        .bind(email)
        .execute(db)
        .await
        .expect("force-verify");
}

/// Promote a registered user to `role = 'admin'`.
#[allow(dead_code)]
pub async fn promote_to_admin(db: &PgPool, email: &str) {
    sqlx::query("UPDATE users SET role = 'admin' WHERE email = $1::citext")
        .bind(email)
        .execute(db)
        .await
        .expect("promote-to-admin");
}

/// Demote an admin back to `role = 'user'`. Mid-session calls to /admin/api/me
/// after this should return 401 because find_session re-checks the role.
#[allow(dead_code)]
pub async fn demote_admin(db: &PgPool, email: &str) {
    sqlx::query("UPDATE users SET role = 'user' WHERE email = $1::citext")
        .bind(email)
        .execute(db)
        .await
        .expect("demote-admin");
}
