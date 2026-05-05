use anyhow::{Context, Result};
use async_trait::async_trait;
use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::{AsyncTransport, Message, Tokio1Executor};
use std::sync::Arc;
use tracing::{info, warn};

#[async_trait]
pub trait Mailer: Send + Sync + 'static {
    async fn send(
        &self,
        to: &str,
        subject: &str,
        html_body: String,
        text_body: String,
    ) -> Result<()>;
}

pub type DynMailer = Arc<dyn Mailer>;

pub struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpMailer {
    pub fn new(smtp_url: &str, from_address: &str, from_name: &str) -> Result<Self> {
        let transport: AsyncSmtpTransport<Tokio1Executor> =
            AsyncSmtpTransport::<Tokio1Executor>::from_url(smtp_url)
                .context("invalid SMTP URL")?
                .build();
        let from: Mailbox = format!("{from_name} <{from_address}>")
            .parse()
            .context("invalid SMTP from address")?;
        Ok(Self { transport, from })
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, to: &str, subject: &str, html: String, text: String) -> Result<()> {
        let to: Mailbox = to.parse().context("invalid recipient address")?;
        let email = Message::builder()
            .from(self.from.clone())
            .to(to.clone())
            .subject(subject)
            .multipart(MultiPart::alternative_plain_html(text, html))
            .context("failed to build email")?;
        self.transport
            .send(email)
            .await
            .context("failed to send email via SMTP")?;
        info!(recipient = %to, "sent email");
        Ok(())
    }
}

/// Used in tests where an actual SMTP server isn't available.
pub struct NoopMailer;

#[async_trait]
impl Mailer for NoopMailer {
    async fn send(&self, to: &str, subject: &str, _html: String, _text: String) -> Result<()> {
        warn!(to, subject, "NoopMailer: dropping email");
        Ok(())
    }
}
