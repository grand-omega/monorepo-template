// Integration tests for the email-verification, password-reset, and /me flows.
// Requires Docker; uses testcontainers for real PG + Redis.

mod common;

use common::spawn_app;
use reqwest::StatusCode;
use serde_json::{Value, json};
use url::Url;

const PASSWORD: &str = "correct-horse-battery-staple";
const NEW_PASSWORD: &str = "S0me-Other-Strong-Pass!";

async fn register(client: &reqwest::Client, base: &str, email: &str) {
    let r = client
        .post(format!("{base}/v1/auth/register"))
        .json(&json!({ "email": email, "password": PASSWORD }))
        .send()
        .await
        .expect("register");
    assert_eq!(r.status(), StatusCode::ACCEPTED);
}

async fn login(client: &reqwest::Client, base: &str, email: &str, password: &str) -> Value {
    let r = client
        .post(format!("{base}/v1/auth/login"))
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await
        .expect("login");
    assert_eq!(
        r.status(),
        StatusCode::OK,
        "login failed: {:?}",
        r.text().await
    );
    r.json().await.expect("parse login body")
}

fn token_from_url(url: &str) -> String {
    let parsed = Url::parse(url).expect("parse url");
    parsed
        .query_pairs()
        .find(|(k, _)| k == "token")
        .map(|(_, v)| v.into_owned())
        .expect("token query param present")
}

// ----------------------------------------------------------------- email verify

#[tokio::test]
async fn email_verification_happy_path() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "verify@example.test";

    register(&client, &app.base_url, email).await;
    let url = app
        .mailer
        .last_url_to(email, "verify")
        .expect("verify URL captured");
    let token = token_from_url(&url);

    let r = client
        .post(format!("{}/v1/auth/verify-email", app.base_url))
        .json(&json!({ "token": token }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);

    // DB now has email_verified = true.
    let verified: (bool,) =
        sqlx::query_as("SELECT email_verified FROM users WHERE email = $1::citext")
            .bind(email)
            .fetch_one(&app.db)
            .await
            .unwrap();
    assert!(verified.0);

    // /me reflects the verified bit.
    let pair = login(&client, &app.base_url, email, PASSWORD).await;
    let me = client
        .get(format!("{}/v1/me", app.base_url))
        .bearer_auth(pair["access_token"].as_str().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::OK);
    assert_eq!(me.json::<Value>().await.unwrap()["email_verified"], true);
}

#[tokio::test]
async fn email_verification_token_single_use() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "single-use@example.test";

    register(&client, &app.base_url, email).await;
    let url = app.mailer.last_url_to(email, "verify").unwrap();
    let token = token_from_url(&url);

    // First use: 200.
    let r = client
        .post(format!("{}/v1/auth/verify-email", app.base_url))
        .json(&json!({ "token": &token }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);

    // Replay: 401.
    let r = client
        .post(format!("{}/v1/auth/verify-email", app.base_url))
        .json(&json!({ "token": &token }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn email_verification_expired_token_rejected() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "expired-verify@example.test";

    register(&client, &app.base_url, email).await;
    let url = app.mailer.last_url_to(email, "verify").unwrap();
    let token = token_from_url(&url);

    // Backdate expiry by 1 day to simulate the link rotting.
    sqlx::query(
        r#"UPDATE email_verification_tokens
           SET expires_at = now() - interval '1 day'
           WHERE user_id = (SELECT id FROM users WHERE email = $1::citext)"#,
    )
    .bind(email)
    .execute(&app.db)
    .await
    .unwrap();

    let r = client
        .post(format!("{}/v1/auth/verify-email", app.base_url))
        .json(&json!({ "token": token }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn resend_verification_issues_new_token_when_unverified() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "resend@example.test";

    register(&client, &app.base_url, email).await;
    let first = app.mailer.last_url_to(email, "verify").unwrap();

    let r = client
        .post(format!("{}/v1/auth/resend-verification", app.base_url))
        .json(&json!({ "email": email }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::ACCEPTED);

    let second = app.mailer.last_url_to(email, "verify").unwrap();
    assert_ne!(first, second, "resend must mint a new token");
}

#[tokio::test]
async fn resend_verification_unknown_email_still_accepted_no_email_sent() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "ghost@example.test";

    let r = client
        .post(format!("{}/v1/auth/resend-verification", app.base_url))
        .json(&json!({ "email": email }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::ACCEPTED);

    // Enumeration prevention: no email actually sent.
    assert!(app.mailer.last_url_to(email, "verify").is_none());
}

#[tokio::test]
async fn resend_verification_rate_limited_after_three() {
    // VeryStrict class caps at 3/hour per IP. The 4th call must come back 429.
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "rl-resend@example.test";
    register(&client, &app.base_url, email).await;

    let url = format!("{}/v1/auth/resend-verification", app.base_url);
    let body = json!({ "email": email });
    let mut codes = Vec::new();
    for _ in 0..4 {
        let r = client.post(&url).json(&body).send().await.unwrap();
        codes.push(r.status());
    }
    assert_eq!(codes[0], StatusCode::ACCEPTED);
    assert_eq!(codes[1], StatusCode::ACCEPTED);
    assert_eq!(codes[2], StatusCode::ACCEPTED);
    assert_eq!(codes[3], StatusCode::TOO_MANY_REQUESTS);
}

// --------------------------------------------------------------- password reset

#[tokio::test]
async fn password_reset_happy_path_revokes_old_sessions() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "reset@example.test";

    register(&client, &app.base_url, email).await;
    // Establish a session before resetting — it must be revoked after.
    let pair = login(&client, &app.base_url, email, PASSWORD).await;
    let old_refresh = pair["refresh_token"].as_str().unwrap().to_string();

    let r = client
        .post(format!("{}/v1/auth/password-reset/request", app.base_url))
        .json(&json!({ "email": email }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::ACCEPTED);

    let url = app
        .mailer
        .last_url_to(email, "reset")
        .expect("reset URL captured");
    let token = token_from_url(&url);

    let r = client
        .post(format!("{}/v1/auth/password-reset/confirm", app.base_url))
        .json(&json!({ "token": token, "new_password": NEW_PASSWORD }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);

    // Old refresh token revoked.
    let r = client
        .post(format!("{}/v1/auth/refresh", app.base_url))
        .json(&json!({ "refresh_token": old_refresh }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);

    // Login with new password works.
    let _new_pair = login(&client, &app.base_url, email, NEW_PASSWORD).await;

    // Login with old password fails.
    let r = client
        .post(format!("{}/v1/auth/login", app.base_url))
        .json(&json!({ "email": email, "password": PASSWORD }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn password_reset_token_single_use() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "reset-replay@example.test";

    register(&client, &app.base_url, email).await;
    client
        .post(format!("{}/v1/auth/password-reset/request", app.base_url))
        .json(&json!({ "email": email }))
        .send()
        .await
        .unwrap();
    let token = token_from_url(&app.mailer.last_url_to(email, "reset").unwrap());

    let body = json!({ "token": &token, "new_password": NEW_PASSWORD });
    let r = client
        .post(format!("{}/v1/auth/password-reset/confirm", app.base_url))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);

    // Replay rejected.
    let r = client
        .post(format!("{}/v1/auth/password-reset/confirm", app.base_url))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn password_reset_expired_token_rejected() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "reset-expired@example.test";

    register(&client, &app.base_url, email).await;
    client
        .post(format!("{}/v1/auth/password-reset/request", app.base_url))
        .json(&json!({ "email": email }))
        .send()
        .await
        .unwrap();
    let token = token_from_url(&app.mailer.last_url_to(email, "reset").unwrap());

    sqlx::query(
        r#"UPDATE password_reset_tokens
           SET expires_at = now() - interval '1 hour'
           WHERE user_id = (SELECT id FROM users WHERE email = $1::citext)"#,
    )
    .bind(email)
    .execute(&app.db)
    .await
    .unwrap();

    let r = client
        .post(format!("{}/v1/auth/password-reset/confirm", app.base_url))
        .json(&json!({ "token": token, "new_password": NEW_PASSWORD }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------- /me

#[tokio::test]
async fn patch_me_updates_display_name() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "patch@example.test";

    register(&client, &app.base_url, email).await;
    let pair = login(&client, &app.base_url, email, PASSWORD).await;
    let access = pair["access_token"].as_str().unwrap().to_string();

    let r = client
        .patch(format!("{}/v1/me", app.base_url))
        .bearer_auth(&access)
        .json(&json!({ "display_name": "Alice Liddell" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body: Value = r.json().await.unwrap();
    assert_eq!(body["display_name"], "Alice Liddell");
}

#[tokio::test]
async fn change_password_revokes_other_sessions_and_keeps_caller() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "chgpw@example.test";

    register(&client, &app.base_url, email).await;
    // Two parallel sessions: one will change the password, the other must die.
    let session_a = login(&client, &app.base_url, email, PASSWORD).await;
    let session_b = login(&client, &app.base_url, email, PASSWORD).await;
    let access_a = session_a["access_token"].as_str().unwrap().to_string();
    let refresh_b = session_b["refresh_token"].as_str().unwrap().to_string();

    let r = client
        .patch(format!("{}/v1/me/password", app.base_url))
        .bearer_auth(&access_a)
        .json(&json!({ "current_password": PASSWORD, "new_password": NEW_PASSWORD }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body: Value = r.json().await.unwrap();
    let new_refresh = body["refresh_token"].as_str().unwrap().to_string();

    // Session B's refresh token is now revoked.
    let r = client
        .post(format!("{}/v1/auth/refresh", app.base_url))
        .json(&json!({ "refresh_token": refresh_b }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);

    // The caller receives a fresh refresh token that works after the change.
    let r = client
        .post(format!("{}/v1/auth/refresh", app.base_url))
        .json(&json!({ "refresh_token": new_refresh }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);

    // Login with new password works.
    let _ = login(&client, &app.base_url, email, NEW_PASSWORD).await;
}

#[tokio::test]
async fn change_password_wrong_current_rejected() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "chgpw-bad@example.test";

    register(&client, &app.base_url, email).await;
    let pair = login(&client, &app.base_url, email, PASSWORD).await;
    let access = pair["access_token"].as_str().unwrap().to_string();

    let r = client
        .patch(format!("{}/v1/me/password", app.base_url))
        .bearer_auth(&access)
        .json(&json!({
            "current_password": "wrong-but-long-enough-12",
            "new_password": NEW_PASSWORD
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn delete_me_requires_password_and_revokes_sessions() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "delete@example.test";

    register(&client, &app.base_url, email).await;
    let pair = login(&client, &app.base_url, email, PASSWORD).await;
    let access = pair["access_token"].as_str().unwrap().to_string();
    let refresh = pair["refresh_token"].as_str().unwrap().to_string();

    // Wrong password rejected.
    let r = client
        .delete(format!("{}/v1/me", app.base_url))
        .bearer_auth(&access)
        .json(&json!({ "password": "this-is-not-it-1234" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);

    // Right password soft-deletes the user.
    let r = client
        .delete(format!("{}/v1/me", app.base_url))
        .bearer_auth(&access)
        .json(&json!({ "password": PASSWORD }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);

    // Refresh token now revoked.
    let r = client
        .post(format!("{}/v1/auth/refresh", app.base_url))
        .json(&json!({ "refresh_token": refresh }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------- audit log

#[tokio::test]
async fn login_failure_writes_audit_event() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "audit-fail@example.test";

    register(&client, &app.base_url, email).await;

    let r = client
        .post(format!("{}/v1/auth/login", app.base_url))
        .json(&json!({ "email": email, "password": "wrong-but-long-enough-1" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);

    let count: (i64,) = sqlx::query_as(
        r#"SELECT count(*) FROM auth_events
           WHERE event_type = 'login_failure_wrong_password'
             AND user_id = (SELECT id FROM users WHERE email = $1::citext)"#,
    )
    .bind(email)
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn login_unknown_email_writes_audit_event_with_null_user() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let r = client
        .post(format!("{}/v1/auth/login", app.base_url))
        .json(&json!({ "email": "ghost@nowhere.test", "password": "anything-12-chars" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);

    let count: (i64,) = sqlx::query_as(
        r#"SELECT count(*) FROM auth_events
           WHERE event_type = 'login_failure_unknown_email' AND user_id IS NULL"#,
    )
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn login_success_writes_audit_event() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let email = "audit-ok@example.test";

    register(&client, &app.base_url, email).await;
    let _ = login(&client, &app.base_url, email, PASSWORD).await;

    let count: (i64,) = sqlx::query_as(
        r#"SELECT count(*) FROM auth_events
           WHERE event_type = 'login_success'
             AND user_id = (SELECT id FROM users WHERE email = $1::citext)"#,
    )
    .bind(email)
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!(count.0, 1);
}
