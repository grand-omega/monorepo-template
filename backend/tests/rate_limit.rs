// Integration tests for the Redis-backed rate limiter and trusted-proxy
// IP extraction.

mod common;

use common::spawn_app;
use reqwest::StatusCode;
use serde_json::json;

/// Class::VeryStrict caps at 3 requests per hour per IP for password reset.
/// The 4th attempt from the same untrusted peer must come back 429.
#[tokio::test]
async fn very_strict_class_returns_429_after_burst() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let url = format!("{}/v1/auth/password-reset/request", app.base_url);
    let body = json!({ "email": "anyone@example.test" });

    let mut codes = Vec::new();
    for _ in 0..4 {
        let resp = client.post(&url).json(&body).send().await.unwrap();
        codes.push(resp.status());
    }

    assert_eq!(
        codes,
        vec![
            StatusCode::ACCEPTED,
            StatusCode::ACCEPTED,
            StatusCode::ACCEPTED,
            StatusCode::TOO_MANY_REQUESTS,
        ]
    );
}

/// With the trust list empty (the default), a request from 127.0.0.1 is an
/// untrusted peer and X-Forwarded-For MUST be ignored. The bucket is keyed on
/// the real peer IP, so spoofing different XFFs each request does not
/// circumvent the limit.
#[tokio::test]
async fn untrusted_peer_xff_does_not_bypass_limit() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let url = format!("{}/v1/auth/password-reset/request", app.base_url);
    let body = json!({ "email": "spoof@example.test" });

    let xffs = ["1.1.1.1", "2.2.2.2", "3.3.3.3", "4.4.4.4"];
    let mut codes = Vec::new();
    for xff in xffs {
        let resp = client
            .post(&url)
            .header("x-forwarded-for", xff)
            .json(&body)
            .send()
            .await
            .unwrap();
        codes.push(resp.status());
    }

    // Bucket is by 127.0.0.1, so the 4th hits the same cap as the previous test.
    assert_eq!(codes[3], StatusCode::TOO_MANY_REQUESTS);
}
