use anyhow::{Context, Result};
use std::sync::OnceLock;
use tera::{Context as TeraContext, Tera};

static TERA: OnceLock<Tera> = OnceLock::new();

fn tera() -> &'static Tera {
    TERA.get_or_init(|| {
        let mut t = Tera::default();
        t.add_raw_templates(vec![
            ("verify_email.html", VERIFY_HTML),
            ("verify_email.txt", VERIFY_TEXT),
            ("password_reset.html", RESET_HTML),
            ("password_reset.txt", RESET_TEXT),
        ])
        .expect("templates compile");
        t
    })
}

pub struct Rendered {
    pub html: String,
    pub text: String,
}

pub fn render_verify_email(verify_url: &str, app_name: &str) -> Result<Rendered> {
    let mut ctx = TeraContext::new();
    ctx.insert("verify_url", verify_url);
    ctx.insert("app_name", app_name);
    let html = tera()
        .render("verify_email.html", &ctx)
        .context("render verify_email.html")?;
    let text = tera()
        .render("verify_email.txt", &ctx)
        .context("render verify_email.txt")?;
    Ok(Rendered { html, text })
}

pub fn render_password_reset(reset_url: &str, app_name: &str) -> Result<Rendered> {
    let mut ctx = TeraContext::new();
    ctx.insert("reset_url", reset_url);
    ctx.insert("app_name", app_name);
    let html = tera()
        .render("password_reset.html", &ctx)
        .context("render password_reset.html")?;
    let text = tera()
        .render("password_reset.txt", &ctx)
        .context("render password_reset.txt")?;
    Ok(Rendered { html, text })
}

const VERIFY_HTML: &str = r#"<!doctype html>
<html><body style="font-family:system-ui,sans-serif;max-width:560px;margin:auto;padding:24px">
<h2>Verify your email</h2>
<p>Welcome to {{ app_name }}. Please confirm this email address belongs to you.</p>
<p><a href="{{ verify_url | safe }}" style="background:#111;color:#fff;padding:10px 18px;text-decoration:none;border-radius:4px">Verify email</a></p>
<p>If the button doesn't work, paste this URL into your browser:<br><code>{{ verify_url | safe }}</code></p>
<p>If you didn't sign up, you can safely ignore this email.</p>
</body></html>"#;

const VERIFY_TEXT: &str = r#"Verify your email for {{ app_name }}

Open this link to confirm your email address:
{{ verify_url | safe }}

If you didn't sign up, you can ignore this message.
"#;

const RESET_HTML: &str = r#"<!doctype html>
<html><body style="font-family:system-ui,sans-serif;max-width:560px;margin:auto;padding:24px">
<h2>Reset your password</h2>
<p>We received a request to reset your {{ app_name }} password.</p>
<p><a href="{{ reset_url | safe }}" style="background:#111;color:#fff;padding:10px 18px;text-decoration:none;border-radius:4px">Reset password</a></p>
<p>If the button doesn't work, paste this URL into your browser:<br><code>{{ reset_url | safe }}</code></p>
<p>If you didn't request this, you can safely ignore this email — your password won't change.</p>
</body></html>"#;

const RESET_TEXT: &str = r#"Reset your {{ app_name }} password

Open this link to choose a new password:
{{ reset_url | safe }}

If you didn't request this, ignore this message — nothing will change.
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_verify_email() {
        let r = render_verify_email("https://example.test/v?t=abc", "TestApp").unwrap();
        assert!(r.html.contains("https://example.test/v?t=abc"));
        assert!(r.text.contains("https://example.test/v?t=abc"));
        assert!(r.html.contains("TestApp"));
    }

    #[test]
    fn renders_password_reset() {
        let r = render_password_reset("https://example.test/r?t=abc", "TestApp").unwrap();
        assert!(r.html.contains("https://example.test/r?t=abc"));
        assert!(r.text.contains("https://example.test/r?t=abc"));
    }
}
