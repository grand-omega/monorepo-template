// Integration tests for the auth + refresh-rotation state machine.
// Requires Docker; uses testcontainers for real PG + Redis.

mod common;

use base64::Engine;
use common::spawn_app;
use reqwest::StatusCode;
use serde_json::{Value, json};

const PASSWORD: &str = "correct-horse-battery-staple";

async fn register(client: &reqwest::Client, base: &str, email: &str) -> StatusCode {
    client
        .post(format!("{base}/v1/auth/register"))
        .json(&json!({ "email": email, "password": PASSWORD }))
        .send()
        .await
        .expect("register")
        .status()
}

async fn login(client: &reqwest::Client, base: &str, email: &str) -> Value {
    let r = client
        .post(format!("{base}/v1/auth/login"))
        .json(&json!({ "email": email, "password": PASSWORD }))
        .send()
        .await
        .expect("login");
    assert_eq!(r.status(), StatusCode::OK, "login failed");
    r.json().await.expect("parse login body")
}

async fn refresh(client: &reqwest::Client, base: &str, refresh_token: &str) -> reqwest::Response {
    client
        .post(format!("{base}/v1/auth/refresh"))
        .json(&json!({ "refresh_token": refresh_token }))
        .send()
        .await
        .expect("refresh")
}

fn forge_refresh_with_wrong_secret(token: &str) -> String {
    let stripped = token.strip_prefix("v1.").expect("refresh token prefix");
    let (id_part, _) = stripped.split_once('.').expect("refresh token shape");
    let wrong_secret = [7u8; 32];
    format!(
        "v1.{}.{}",
        id_part,
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(wrong_secret)
    )
}

#[tokio::test]
async fn happy_path_register_login_me() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    assert_eq!(
        register(&client, &app.base_url, "alice@example.test").await,
        StatusCode::ACCEPTED
    );
    let pair = login(&client, &app.base_url, "alice@example.test").await;
    assert!(pair["user"]["avatar_url"].is_null());
    let access = pair["access_token"].as_str().unwrap();

    let me = client
        .get(format!("{}/v1/me", app.base_url))
        .bearer_auth(access)
        .send()
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::OK);
    let body: Value = me.json().await.unwrap();
    assert_eq!(body["email"], "alice@example.test");
    assert_eq!(body["email_verified"], false);
    assert!(body["avatar_url"].is_null());
}

#[tokio::test]
async fn duplicate_register_stays_202() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let e = "dup@example.test";
    assert_eq!(
        register(&client, &app.base_url, e).await,
        StatusCode::ACCEPTED
    );
    // Same email a second time — must NOT 5xx (was the bug); must NOT leak existence.
    assert_eq!(
        register(&client, &app.base_url, e).await,
        StatusCode::ACCEPTED
    );

    // Only one row should exist.
    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM users WHERE email = $1::citext")
        .bind(e)
        .fetch_one(&app.db)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn refresh_rotates_old_token_rejected() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    register(&client, &app.base_url, "rot@example.test").await;
    let pair = login(&client, &app.base_url, "rot@example.test").await;
    let r0 = pair["refresh_token"].as_str().unwrap().to_string();

    let resp = refresh(&client, &app.base_url, &r0).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let new_pair: Value = resp.json().await.unwrap();
    let r1 = new_pair["refresh_token"].as_str().unwrap().to_string();
    assert_ne!(r0, r1);

    // R1 works.
    let resp = refresh(&client, &app.base_url, &r1).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // R0 (already rotated) is rejected.
    let resp = refresh(&client, &app.base_url, &r0).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_reuse_revokes_family() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    register(&client, &app.base_url, "reuse@example.test").await;
    let pair = login(&client, &app.base_url, "reuse@example.test").await;
    let r0 = pair["refresh_token"].as_str().unwrap().to_string();

    // Rotate once: R0 -> R1.
    let resp = refresh(&client, &app.base_url, &r0).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let r1 = resp.json::<Value>().await.unwrap()["refresh_token"]
        .as_str()
        .unwrap()
        .to_string();

    // Replay R0 — server must detect reuse and revoke the family.
    let resp = refresh(&client, &app.base_url, &r0).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "token_reuse_detected");

    // R1 must now also be revoked (family kill).
    let resp = refresh(&client, &app.base_url, &r1).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // DB sanity: every refresh row for this user has revoked_at set.
    let unrev: (i64,) = sqlx::query_as(
        r#"SELECT count(*) FROM refresh_tokens
           WHERE user_id = (SELECT id FROM users WHERE email = 'reuse@example.test'::citext)
             AND revoked_at IS NULL"#,
    )
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!(unrev.0, 0, "all refresh rows must be revoked after reuse");
}

#[tokio::test]
async fn concurrent_refresh_only_one_succeeds() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    register(&client, &app.base_url, "race@example.test").await;
    let pair = login(&client, &app.base_url, "race@example.test").await;
    let r0 = pair["refresh_token"].as_str().unwrap().to_string();

    // Fire two refreshes concurrently with the same wire token.
    let a = refresh(&client, &app.base_url, &r0);
    let b = refresh(&client, &app.base_url, &r0);
    let (ra, rb) = tokio::join!(a, b);
    let mut codes = [ra.status(), rb.status()];
    codes.sort_by_key(|s| s.as_u16());

    // Exactly one 200, one 401. The FOR UPDATE lock means the loser sees the
    // already-rotated state and gets reuse-detected.
    assert_eq!(codes[0], StatusCode::OK);
    assert_eq!(codes[1], StatusCode::UNAUTHORIZED);

    // Family must remain consistent: at most one unrevoked refresh row.
    let unrev: (i64,) = sqlx::query_as(
        r#"SELECT count(*) FROM refresh_tokens
           WHERE user_id = (SELECT id FROM users WHERE email = 'race@example.test'::citext)
             AND revoked_at IS NULL"#,
    )
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert!(
        unrev.0 <= 1,
        "expected ≤1 unrevoked refresh, got {}",
        unrev.0
    );
}

#[tokio::test]
async fn logout_revokes_only_presented_token_logout_all_revokes_family() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    register(&client, &app.base_url, "out@example.test").await;
    let p1 = login(&client, &app.base_url, "out@example.test").await;
    let p2 = login(&client, &app.base_url, "out@example.test").await;
    let r1 = p1["refresh_token"].as_str().unwrap().to_string();
    let r2 = p2["refresh_token"].as_str().unwrap().to_string();
    let access2 = p2["access_token"].as_str().unwrap().to_string();

    // /logout revokes only r1.
    let resp = client
        .post(format!("{}/v1/auth/logout", app.base_url))
        .json(&json!({ "refresh_token": r1 }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        refresh(&client, &app.base_url, &r1).await.status(),
        StatusCode::UNAUTHORIZED
    );
    // r2 still works.
    let resp = refresh(&client, &app.base_url, &r2).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let r2_rotated = resp.json::<Value>().await.unwrap()["refresh_token"]
        .as_str()
        .unwrap()
        .to_string();

    // /logout-all kills everything for the user.
    let resp = client
        .post(format!("{}/v1/auth/logout-all", app.base_url))
        .bearer_auth(&access2)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        refresh(&client, &app.base_url, &r2_rotated).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn logout_all_invalidates_outstanding_access_tokens() {
    // Refresh-token revocation is already covered above. This test asserts the
    // newly-added per-user revocation epoch: after /logout-all, the *access*
    // token presented to a bearer-protected endpoint must also fail.
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    register(&client, &app.base_url, "rev@example.test").await;
    let pair = login(&client, &app.base_url, "rev@example.test").await;
    let access = pair["access_token"].as_str().unwrap().to_string();

    // Sanity: the access token works before revocation.
    let me = client
        .get(format!("{}/v1/me", app.base_url))
        .bearer_auth(&access)
        .send()
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::OK);

    // JWT iat has 1-second resolution; sleep so the access token's iat is
    // strictly earlier than the revocation epoch we're about to write.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    // Revoke everything for this user.
    let resp = client
        .post(format!("{}/v1/auth/logout-all", app.base_url))
        .bearer_auth(&access)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // The same access token must now be rejected.
    let me = client
        .get(format!("{}/v1/me", app.base_url))
        .bearer_auth(&access)
        .send()
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::UNAUTHORIZED);
    let body: Value = me.json().await.unwrap();
    assert_eq!(body["code"], "invalid_token");
}

#[tokio::test]
async fn logout_requires_refresh_secret_not_only_token_id() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    register(&client, &app.base_url, "logout-forged@example.test").await;
    let pair = login(&client, &app.base_url, "logout-forged@example.test").await;
    let refresh_token = pair["refresh_token"].as_str().unwrap().to_string();
    let forged = forge_refresh_with_wrong_secret(&refresh_token);

    let resp = client
        .post(format!("{}/v1/auth/logout", app.base_url))
        .json(&json!({ "refresh_token": forged }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let resp = refresh(&client, &app.base_url, &refresh_token).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
