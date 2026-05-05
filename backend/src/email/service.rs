use crate::AppState;
use crate::email::templates;
use tracing::error;

const APP_NAME: &str = "Lab Rust Server";

pub async fn send_verification(state: &AppState, to: &str, verify_url: &str) {
    let rendered = match templates::render_verify_email(verify_url, APP_NAME) {
        Ok(r) => r,
        Err(e) => {
            error!(error = ?e, "failed to render verify email template");
            return;
        }
    };
    if let Err(e) = state
        .mailer
        .send(to, "Verify your email", rendered.html, rendered.text)
        .await
    {
        error!(error = ?e, "failed to send verification email");
    }
}

pub async fn send_password_reset(state: &AppState, to: &str, reset_url: &str) {
    let rendered = match templates::render_password_reset(reset_url, APP_NAME) {
        Ok(r) => r,
        Err(e) => {
            error!(error = ?e, "failed to render password reset template");
            return;
        }
    };
    if let Err(e) = state
        .mailer
        .send(to, "Reset your password", rendered.html, rendered.text)
        .await
    {
        error!(error = ?e, "failed to send password reset email");
    }
}
