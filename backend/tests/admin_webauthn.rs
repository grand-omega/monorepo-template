//! Integration tests for the admin WebAuthn endpoints.
//!
//! End-to-end ceremony testing (real attestation, real assertion) needs a
//! virtual authenticator, which lives on the frontend Playwright side. These
//! backend tests cover the routing, authentication gating, and the password →
//! WebAuthn step branching that triggers when an admin has registered keys.

mod common;

use common::{promote_to_admin, spawn_app};
use reqwest::StatusCode;
use serde_json::{Value, json};

const PASSWORD: &str = "correct-horse-battery-staple";

async fn register_and_promote(client: &reqwest::Client, app: &common::TestApp, email: &str) {
    let r = client
        .post(format!("{}/v1/auth/register", app.base_url))
        .json(&json!({ "email": email, "password": PASSWORD }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::ACCEPTED);
    promote_to_admin(&app.db, email).await;
}

/// Log in as admin (password-only path) and return the cookie jar's Cookie
/// header value plus the CSRF token for subsequent state-changing calls.
async fn admin_login(client: &reqwest::Client, app: &common::TestApp, email: &str) -> AdminAuth {
    let resp = client
        .post(format!("{}/admin/api/login", app.base_url))
        .json(&json!({ "email": email, "password": PASSWORD }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let cookies = resp
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or("").to_string())
        .collect::<Vec<_>>();
    let cookie_header = cookies.join("; ");
    let csrf = cookies
        .iter()
        .find_map(|c| c.strip_prefix("admin_csrf="))
        .expect("admin_csrf cookie")
        .to_string();

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["kind"], "authenticated");
    AdminAuth {
        cookie: cookie_header,
        csrf,
    }
}

struct AdminAuth {
    cookie: String,
    csrf: String,
}

#[tokio::test]
async fn login_returns_authenticated_kind_when_admin_has_no_passkeys() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    register_and_promote(&client, &app, "admin0@example.test").await;

    // The admin_login helper already asserts kind == "authenticated".
    let _ = admin_login(&client, &app, "admin0@example.test").await;
}

#[tokio::test]
async fn credentials_list_is_empty_for_new_admin() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    register_and_promote(&client, &app, "admin1@example.test").await;
    let auth = admin_login(&client, &app, "admin1@example.test").await;

    let resp = client
        .get(format!("{}/admin/api/webauthn/credentials", app.base_url))
        .header(reqwest::header::COOKIE, &auth.cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let items = body["items"].as_array().expect("items array");
    assert!(items.is_empty(), "expected no credentials, got {items:?}");
}

#[tokio::test]
async fn register_begin_returns_challenge() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    register_and_promote(&client, &app, "admin2@example.test").await;
    let auth = admin_login(&client, &app, "admin2@example.test").await;

    let resp = client
        .post(format!(
            "{}/admin/api/webauthn/register/begin",
            app.base_url
        ))
        .header(reqwest::header::COOKIE, &auth.cookie)
        .header("x-csrf-token", &auth.csrf)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    // We don't validate the full PublicKeyCredentialCreationOptions schema here —
    // webauthn-rs is responsible for that — but the challenge field should be
    // present and non-empty.
    let pk = &body["challenge"]["publicKey"];
    assert!(!pk["challenge"].as_str().unwrap_or("").is_empty());
    assert_eq!(pk["rp"]["id"], "localhost");
}

#[tokio::test]
async fn login_branches_to_webauthn_required_when_credentials_exist() {
    // We can't easily perform a real registration without a virtual authenticator,
    // so we sneak a row into webauthn_credentials with a placeholder Passkey
    // JSON. `admin_has_credentials` only counts rows, so the branch fires.
    // `start_passkey_authentication` then deserializes the row, which will
    // fail for our placeholder — we accept either a "webauthn_required"
    // response (if webauthn-rs is lenient) or a 500. Both prove the branch
    // ran; what we want to assert is that the password-only path did NOT
    // succeed (no `kind = "authenticated"`, no session cookie).
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    register_and_promote(&client, &app, "admin3@example.test").await;

    let admin_id: (uuid::Uuid,) = sqlx::query_as("SELECT id FROM users WHERE email = $1::citext")
        .bind("admin3@example.test")
        .fetch_one(&app.db)
        .await
        .unwrap();

    sqlx::query(
        r#"INSERT INTO webauthn_credentials
                (id, admin_user_id, credential_id, passkey, label)
           VALUES ($1, $2, $3, $4::jsonb, $5)"#,
    )
    .bind(uuid::Uuid::now_v7())
    .bind(admin_id.0)
    .bind(b"\x01\x02\x03\x04".as_slice())
    .bind(serde_json::json!({ "placeholder": true }))
    .bind("placeholder")
    .execute(&app.db)
    .await
    .unwrap();

    let resp = client
        .post(format!("{}/admin/api/login", app.base_url))
        .json(&json!({
            "email": "admin3@example.test",
            "password": PASSWORD,
        }))
        .send()
        .await
        .unwrap();

    // Either: WebauthnRequired payload (200) with no auth cookies, or 500
    // because the placeholder couldn't be deserialized into a Passkey. In
    // both cases, the password-only "authenticated" branch must NOT have run.
    let cookies: Vec<String> = resp
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .collect();
    assert!(
        !cookies
            .iter()
            .any(|c| c.starts_with("admin_session=") && !c.starts_with("admin_session=;")),
        "password-only login set a session cookie when WebAuthn was required: {cookies:?}"
    );

    if resp.status() == StatusCode::OK {
        let body: Value = resp.json().await.unwrap();
        assert_eq!(body["kind"], "webauthn_required");
        assert!(!body["pending_token"].as_str().unwrap_or("").is_empty());
    } else {
        // 500 from the placeholder deserialization is acceptable for this test —
        // the flow correctly chose the WebAuthn branch.
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}

#[tokio::test]
async fn cannot_delete_final_admin_passkey() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "admin-final-key@example.test";
    register_and_promote(&client, &app, email).await;
    let auth = admin_login(&client, &app, email).await;

    let admin_id: (uuid::Uuid,) = sqlx::query_as("SELECT id FROM users WHERE email = $1::citext")
        .bind(email)
        .fetch_one(&app.db)
        .await
        .unwrap();
    let credential_id = uuid::Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO webauthn_credentials
                (id, admin_user_id, credential_id, passkey, label)
           VALUES ($1, $2, $3, $4::jsonb, $5)"#,
    )
    .bind(credential_id)
    .bind(admin_id.0)
    .bind(b"\x05\x06\x07\x08".as_slice())
    .bind(serde_json::json!({ "placeholder": true }))
    .bind("only key")
    .execute(&app.db)
    .await
    .unwrap();

    let resp = client
        .delete(format!(
            "{}/admin/api/webauthn/credentials/{}",
            app.base_url, credential_id
        ))
        .header(reqwest::header::COOKIE, &auth.cookie)
        .header("x-csrf-token", &auth.csrf)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    let remaining: (i64,) =
        sqlx::query_as("SELECT count(*) FROM webauthn_credentials WHERE admin_user_id = $1")
            .bind(admin_id.0)
            .fetch_one(&app.db)
            .await
            .unwrap();
    assert_eq!(remaining.0, 1);
}
