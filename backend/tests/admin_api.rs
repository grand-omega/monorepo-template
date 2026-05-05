// Integration tests for the JSON admin API mounted at /admin/api/*.
// Exercises auth, CSRF, audit-log writes, per-admin lockout, and timing parity.

mod common;

use common::{demote_admin, promote_to_admin, spawn_app};
use reqwest::StatusCode;
use reqwest::cookie::{CookieStore, Jar};
use serde_json::{Value, json};
use std::sync::Arc;
use url::Url;

const PASSWORD: &str = "correct-horse-battery-staple";

struct AdminClient {
    http: reqwest::Client,
    jar: Arc<Jar>,
    base_url: String,
}

impl AdminClient {
    fn new(base_url: &str) -> Self {
        let jar = Arc::new(Jar::default());
        let http = reqwest::Client::builder()
            .cookie_provider(jar.clone())
            .build()
            .expect("build reqwest client");
        Self {
            http,
            jar,
            base_url: base_url.to_string(),
        }
    }

    fn csrf_token(&self) -> Option<String> {
        let url: Url = format!("{}/admin/api/foo", self.base_url).parse().ok()?;
        let header = self.jar.cookies(&url)?;
        let cookie_str = header.to_str().ok()?.to_string();
        for part in cookie_str.split(';') {
            let part = part.trim();
            if let Some(v) = part.strip_prefix("admin_csrf=") {
                return Some(v.to_string());
            }
        }
        None
    }

    async fn login(&self, email: &str, password: &str) -> reqwest::Response {
        self.http
            .post(format!("{}/admin/api/login", self.base_url))
            .json(&json!({ "email": email, "password": password }))
            .send()
            .await
            .expect("admin login request")
    }

    async fn me(&self) -> reqwest::Response {
        self.http
            .get(format!("{}/admin/api/me", self.base_url))
            .send()
            .await
            .expect("admin me request")
    }

    async fn post_with_csrf(&self, path: &str, body: Value) -> reqwest::Response {
        let csrf = self.csrf_token().expect("csrf token cookie");
        self.http
            .post(format!("{}{}", self.base_url, path))
            .header("x-csrf-token", csrf)
            .json(&body)
            .send()
            .await
            .expect("post request")
    }

    async fn post_no_csrf(&self, path: &str, body: Value) -> reqwest::Response {
        self.http
            .post(format!("{}{}", self.base_url, path))
            .json(&body)
            .send()
            .await
            .expect("post request")
    }

    async fn delete_with_csrf(&self, path: &str) -> reqwest::Response {
        let csrf = self.csrf_token().expect("csrf token cookie");
        self.http
            .delete(format!("{}{}", self.base_url, path))
            .header("x-csrf-token", csrf)
            .send()
            .await
            .expect("delete request")
    }
}

async fn register(client: &reqwest::Client, base: &str, email: &str) {
    let r = client
        .post(format!("{base}/v1/auth/register"))
        .json(&json!({ "email": email, "password": PASSWORD }))
        .send()
        .await
        .expect("register");
    assert_eq!(r.status(), StatusCode::ACCEPTED);
}

// ---------------------------------------------------------------- login

#[tokio::test]
async fn admin_login_happy_path_sets_cookies_and_me_returns_admin() {
    let app = spawn_app().await;
    let base = reqwest::Client::new();
    let admin_email = "admin@example.test";
    register(&base, &app.base_url, admin_email).await;
    promote_to_admin(&app.db, admin_email).await;

    let client = AdminClient::new(&app.base_url);
    let r = client.login(admin_email, PASSWORD).await;
    assert_eq!(r.status(), StatusCode::OK);

    // Both cookies must be set.
    let url: Url = format!("{}/admin/api/x", app.base_url).parse().unwrap();
    let cookies = client
        .jar
        .cookies(&url)
        .expect("cookies set after login")
        .to_str()
        .unwrap()
        .to_string();
    assert!(cookies.contains("admin_session="), "session cookie missing");
    assert!(cookies.contains("admin_csrf="), "csrf cookie missing");

    let me = client.me().await;
    assert_eq!(me.status(), StatusCode::OK);
    let body: Value = me.json().await.unwrap();
    assert_eq!(body["email"], admin_email);
    assert_eq!(body["role"], "admin");
}

#[tokio::test]
async fn admin_login_non_admin_rejected() {
    let app = spawn_app().await;
    let base = reqwest::Client::new();
    let email = "nope@example.test";
    register(&base, &app.base_url, email).await;
    // Did NOT promote — role stays 'user'.

    let client = AdminClient::new(&app.base_url);
    let r = client.login(email, PASSWORD).await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_login_wrong_password_writes_audit_event() {
    let app = spawn_app().await;
    let base = reqwest::Client::new();
    let email = "wrongpw-admin@example.test";
    register(&base, &app.base_url, email).await;
    promote_to_admin(&app.db, email).await;

    let client = AdminClient::new(&app.base_url);
    let r = client.login(email, "WRONG-but-12-chars").await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);

    let count: (i64,) = sqlx::query_as(
        r#"SELECT count(*) FROM auth_events
           WHERE event_type = 'admin_login_failure_wrong_password'
             AND user_id = (SELECT id FROM users WHERE email = $1::citext)"#,
    )
    .bind(email)
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn admin_login_unknown_email_writes_audit_event_with_null_user() {
    let app = spawn_app().await;
    let client = AdminClient::new(&app.base_url);
    let r = client
        .login("nobody@example.test", "ANYTHING-12-chars")
        .await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);

    let count: (i64,) = sqlx::query_as(
        r#"SELECT count(*) FROM auth_events
           WHERE event_type = 'admin_login_failure_unknown_email'
             AND user_id IS NULL"#,
    )
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn admin_login_per_account_lockout_after_five_failures() {
    let app = spawn_app().await;
    let base = reqwest::Client::new();
    let email = "lockout-admin@example.test";
    register(&base, &app.base_url, email).await;
    promote_to_admin(&app.db, email).await;

    let client = AdminClient::new(&app.base_url);
    for _ in 0..5 {
        let r = client.login(email, "WRONG-but-12-chars").await;
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    }

    // 6th attempt — account is now locked. Even with the right password, login
    // fails with the same generic 401 (locked-vs-wrong-password isn't leaked).
    let r = client.login(email, PASSWORD).await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);

    // DB confirms locked_until is set.
    let locked: (Option<chrono::DateTime<chrono::Utc>>,) =
        sqlx::query_as("SELECT locked_until FROM users WHERE email = $1::citext")
            .bind(email)
            .fetch_one(&app.db)
            .await
            .unwrap();
    assert!(
        locked.0.is_some(),
        "locked_until should be set after 5 fails"
    );
}

// ---------------------------------------------------------------- CSRF

#[tokio::test]
async fn csrf_required_on_state_changing_request() {
    let app = spawn_app().await;
    let base = reqwest::Client::new();
    let admin_email = "csrf-admin@example.test";
    let target_email = "victim@example.test";
    register(&base, &app.base_url, admin_email).await;
    register(&base, &app.base_url, target_email).await;
    promote_to_admin(&app.db, admin_email).await;

    let client = AdminClient::new(&app.base_url);
    let r = client.login(admin_email, PASSWORD).await;
    assert_eq!(r.status(), StatusCode::OK);

    let target_id: (uuid::Uuid,) = sqlx::query_as("SELECT id FROM users WHERE email = $1::citext")
        .bind(target_email)
        .fetch_one(&app.db)
        .await
        .unwrap();

    // No X-CSRF-Token header → 403.
    let path = format!("/admin/api/users/{}/lock", target_id.0);
    let r = client.post_no_csrf(&path, json!({})).await;
    assert_eq!(r.status(), StatusCode::FORBIDDEN);

    // Wrong X-CSRF-Token → 403.
    let r = client
        .http
        .post(format!("{}{}", app.base_url, &path))
        .header("x-csrf-token", "not-the-real-token")
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);

    // Correct token → 200.
    let r = client.post_with_csrf(&path, json!({})).await;
    assert_eq!(r.status(), StatusCode::OK);
}

// ---------------------------------------------------------------- /me + role guard

#[tokio::test]
async fn me_without_session_is_401() {
    let app = spawn_app().await;
    let client = AdminClient::new(&app.base_url);
    let r = client.me().await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn demoted_admin_loses_access_immediately() {
    let app = spawn_app().await;
    let base = reqwest::Client::new();
    let email = "demote-me@example.test";
    register(&base, &app.base_url, email).await;
    promote_to_admin(&app.db, email).await;

    let client = AdminClient::new(&app.base_url);
    client.login(email, PASSWORD).await;
    assert_eq!(client.me().await.status(), StatusCode::OK);

    // Demote — the next request must fail because find_session re-checks the
    // role join.
    demote_admin(&app.db, email).await;
    assert_eq!(client.me().await.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------- audit log on actions

#[tokio::test]
async fn lock_user_writes_admin_user_locked_event_with_by_admin() {
    let app = spawn_app().await;
    let base = reqwest::Client::new();
    let admin_email = "actor-admin@example.test";
    let target_email = "target1@example.test";
    register(&base, &app.base_url, admin_email).await;
    register(&base, &app.base_url, target_email).await;
    promote_to_admin(&app.db, admin_email).await;

    let client = AdminClient::new(&app.base_url);
    client.login(admin_email, PASSWORD).await;

    let admin_id: (uuid::Uuid,) = sqlx::query_as("SELECT id FROM users WHERE email = $1::citext")
        .bind(admin_email)
        .fetch_one(&app.db)
        .await
        .unwrap();
    let target_id: (uuid::Uuid,) = sqlx::query_as("SELECT id FROM users WHERE email = $1::citext")
        .bind(target_email)
        .fetch_one(&app.db)
        .await
        .unwrap();

    let r = client
        .post_with_csrf(
            &format!("/admin/api/users/{}/lock", target_id.0),
            json!({ "reason": "test lockdown" }),
        )
        .await;
    assert_eq!(r.status(), StatusCode::OK);

    let row: (serde_json::Value,) = sqlx::query_as(
        r#"SELECT detail FROM auth_events
           WHERE event_type = 'admin_user_locked'
             AND user_id = $1
           ORDER BY id DESC LIMIT 1"#,
    )
    .bind(target_id.0)
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!(row.0["by_admin"], json!(admin_id.0));
    assert_eq!(row.0["reason"], "test lockdown");
}

#[tokio::test]
async fn revoke_sessions_kills_refresh_tokens_and_logs_admin_actor() {
    let app = spawn_app().await;
    let base = reqwest::Client::new();
    let admin_email = "actor2-admin@example.test";
    let user_email = "session-victim@example.test";
    register(&base, &app.base_url, admin_email).await;
    register(&base, &app.base_url, user_email).await;
    promote_to_admin(&app.db, admin_email).await;

    // Give the user a refresh token.
    let pair = base
        .post(format!("{}/v1/auth/login", app.base_url))
        .json(&json!({ "email": user_email, "password": PASSWORD }))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    let refresh_token = pair["refresh_token"].as_str().unwrap().to_string();

    let admin_client = AdminClient::new(&app.base_url);
    admin_client.login(admin_email, PASSWORD).await;

    let target_id: (uuid::Uuid,) = sqlx::query_as("SELECT id FROM users WHERE email = $1::citext")
        .bind(user_email)
        .fetch_one(&app.db)
        .await
        .unwrap();

    let r = admin_client
        .delete_with_csrf(&format!("/admin/api/users/{}/sessions", target_id.0))
        .await;
    assert_eq!(r.status(), StatusCode::OK);

    // The user's refresh token is now dead.
    let r = base
        .post(format!("{}/v1/auth/refresh", app.base_url))
        .json(&json!({ "refresh_token": refresh_token }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);

    // Audit row exists with by_admin populated.
    let count: (i64,) = sqlx::query_as(
        r#"SELECT count(*) FROM auth_events
           WHERE event_type = 'admin_sessions_revoked'
             AND user_id = $1
             AND detail ? 'by_admin'"#,
    )
    .bind(target_id.0)
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn unlock_and_verify_email_write_their_own_events() {
    let app = spawn_app().await;
    let base = reqwest::Client::new();
    let admin_email = "actor3-admin@example.test";
    let target_email = "target2@example.test";
    register(&base, &app.base_url, admin_email).await;
    register(&base, &app.base_url, target_email).await;
    promote_to_admin(&app.db, admin_email).await;

    let client = AdminClient::new(&app.base_url);
    client.login(admin_email, PASSWORD).await;

    let target_id: (uuid::Uuid,) = sqlx::query_as("SELECT id FROM users WHERE email = $1::citext")
        .bind(target_email)
        .fetch_one(&app.db)
        .await
        .unwrap();

    assert_eq!(
        client
            .post_with_csrf(
                &format!("/admin/api/users/{}/unlock", target_id.0),
                json!({})
            )
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        client
            .post_with_csrf(
                &format!("/admin/api/users/{}/verify-email", target_id.0),
                json!({})
            )
            .await
            .status(),
        StatusCode::OK
    );

    let unlock: (i64,) = sqlx::query_as(
        r#"SELECT count(*) FROM auth_events
           WHERE event_type = 'admin_user_unlocked' AND user_id = $1"#,
    )
    .bind(target_id.0)
    .fetch_one(&app.db)
    .await
    .unwrap();
    let verify: (i64,) = sqlx::query_as(
        r#"SELECT count(*) FROM auth_events
           WHERE event_type = 'admin_email_verified' AND user_id = $1"#,
    )
    .bind(target_id.0)
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!(unlock.0, 1);
    assert_eq!(verify.0, 1);
}

// ---------------------------------------------------------------- listing endpoints

#[tokio::test]
async fn users_list_search_and_get_one() {
    let app = spawn_app().await;
    let base = reqwest::Client::new();
    let admin_email = "lister-admin@example.test";
    register(&base, &app.base_url, admin_email).await;
    register(&base, &app.base_url, "needle@findme.example.test").await;
    register(&base, &app.base_url, "haystack@example.test").await;
    promote_to_admin(&app.db, admin_email).await;

    let client = AdminClient::new(&app.base_url);
    client.login(admin_email, PASSWORD).await;

    let r = client
        .http
        .get(format!("{}/admin/api/users?q=findme", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body: Value = r.json().await.unwrap();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["email"], "needle@findme.example.test");
}

#[tokio::test]
async fn auth_events_list_filter_by_event_type() {
    let app = spawn_app().await;
    let base = reqwest::Client::new();
    let admin_email = "events-admin@example.test";
    register(&base, &app.base_url, admin_email).await;
    promote_to_admin(&app.db, admin_email).await;

    let client = AdminClient::new(&app.base_url);
    let r = client.login(admin_email, PASSWORD).await;
    assert_eq!(r.status(), StatusCode::OK);

    let r = client
        .http
        .get(format!(
            "{}/admin/api/auth-events?event_type=admin_login_success",
            app.base_url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body: Value = r.json().await.unwrap();
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for item in items {
        assert_eq!(item["event_type"], "admin_login_success");
    }
}

// ---------------------------------------------------------------- openapi

#[tokio::test]
async fn openapi_json_is_publicly_served() {
    let app = spawn_app().await;
    let r = reqwest::get(format!("{}/openapi.json", app.base_url))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body: Value = r.json().await.unwrap();
    let paths = body["paths"].as_object().unwrap();
    assert!(paths.contains_key("/admin/api/login"));
    assert!(paths.contains_key("/v1/auth/login"));
}
