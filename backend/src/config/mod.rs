use anyhow::{Context, Result};
use figment::Figment;
use figment::providers::Env;
use ipnetwork::IpNetwork;
use serde::Deserialize;
use std::net::SocketAddr;
use std::time::Duration;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub env: AppEnv,
    pub bind_addr: SocketAddr,
    pub metrics_bind_addr: SocketAddr,
    pub public_base_url: Url,

    pub database_url: String,
    pub database_max_connections: u32,
    pub redis_url: String,

    pub jwt_private_key: String,
    pub jwt_public_key: String,
    pub jwt_kid: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    /// Optional JSON array of `{kid, public_key_pem}` for verify-only keys
    /// kept around during a zero-downtime kid rotation. Empty in normal ops.
    #[serde(default)]
    pub jwt_previous_public_keys: Option<String>,
    #[serde(with = "humantime_serde")]
    pub access_token_ttl: Duration,
    #[serde(with = "humantime_serde")]
    pub refresh_token_ttl: Duration,

    pub smtp_url: String,
    pub smtp_from: String,
    pub smtp_from_name: String,
    /// Permit fallback to the no-op mailer when SMTP init fails. Refused in prod.
    #[serde(default)]
    pub allow_noop_mailer: bool,

    pub argon2_m_cost: u32,
    pub argon2_t_cost: u32,
    pub argon2_p_cost: u32,

    pub login_max_failures: i32,
    #[serde(with = "humantime_serde")]
    pub login_lock_duration: Duration,

    #[serde(default, deserialize_with = "deserialize_csv")]
    pub cors_allowed_origins: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_cidr_csv")]
    pub trusted_proxy_cidrs: Vec<IpNetwork>,

    pub migrate_on_start: bool,
    pub log_format: LogFormat,

    /// WebAuthn relying-party id — usually the bare host (e.g. "admin.example.com").
    /// Cookies and credentials are scoped to this domain. Required when admin
    /// 2FA is enabled; if any admin has registered credentials, login will
    /// require the WebAuthn step.
    pub webauthn_rp_id: String,
    /// WebAuthn relying-party origin — full URL with scheme (e.g. "https://admin.example.com").
    /// Must match the page that performs `navigator.credentials.{create,get}`.
    pub webauthn_rp_origin: Url,
    /// Display name shown in the operating-system passkey UI. Defaults to "Admin Console".
    #[serde(default = "default_webauthn_rp_name")]
    pub webauthn_rp_name: String,
}

fn default_webauthn_rp_name() -> String {
    "Admin Console".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppEnv {
    Dev,
    Prod,
    Test,
}

impl AppEnv {
    pub fn is_dev(self) -> bool {
        matches!(self, AppEnv::Dev | AppEnv::Test)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Pretty,
    Json,
}

fn deserialize_csv<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    let s = String::deserialize(d)?;
    Ok(s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect())
}

fn deserialize_cidr_csv<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<IpNetwork>, D::Error> {
    let s = String::deserialize(d)?;
    s.split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| {
            p.parse::<IpNetwork>()
                .map_err(|e| serde::de::Error::custom(format!("invalid CIDR {p:?}: {e}")))
        })
        .collect()
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let cfg: Config = Figment::new()
            .merge(Env::prefixed("APP_").split("__"))
            .extract()
            .context("failed to load config from environment (APP_* vars)")?;
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        if self.jwt_private_key.trim().is_empty() {
            anyhow::bail!("APP_JWT_PRIVATE_KEY is required");
        }
        if self.jwt_public_key.trim().is_empty() {
            anyhow::bail!("APP_JWT_PUBLIC_KEY is required");
        }
        if self.jwt_kid.trim().is_empty() {
            anyhow::bail!("APP_JWT_KID is required");
        }
        if self.access_token_ttl > Duration::from_secs(60 * 60) {
            anyhow::bail!("access_token_ttl must be <= 1 hour");
        }
        if self.refresh_token_ttl < Duration::from_secs(60 * 60) {
            anyhow::bail!("refresh_token_ttl must be >= 1 hour");
        }

        match self.public_base_url.scheme() {
            "http" | "https" => {}
            other => anyhow::bail!("public_base_url scheme must be http or https, got {other}"),
        }
        if !self.public_base_url.has_host() {
            anyhow::bail!("public_base_url must be absolute (include a host)");
        }

        if self.webauthn_rp_id.trim().is_empty() {
            anyhow::bail!("APP_WEBAUTHN_RP_ID is required (e.g. \"admin.example.com\")");
        }
        match self.webauthn_rp_origin.scheme() {
            "http" | "https" => {}
            other => anyhow::bail!("webauthn_rp_origin scheme must be http or https, got {other}"),
        }
        if !self.webauthn_rp_origin.has_host() {
            anyhow::bail!("webauthn_rp_origin must be absolute (include a host)");
        }

        // Argon2 parameter sanity bounds (RFC 9106).
        if !(8..=1_048_576).contains(&self.argon2_m_cost) {
            anyhow::bail!(
                "argon2_m_cost must be between 8 and 1048576 KiB, got {}",
                self.argon2_m_cost
            );
        }
        if !(1..=64).contains(&self.argon2_t_cost) {
            anyhow::bail!(
                "argon2_t_cost must be between 1 and 64, got {}",
                self.argon2_t_cost
            );
        }
        if !(1..=16).contains(&self.argon2_p_cost) {
            anyhow::bail!(
                "argon2_p_cost must be between 1 and 16, got {}",
                self.argon2_p_cost
            );
        }

        // OWASP-aligned minimums when running in production.
        if matches!(self.env, AppEnv::Prod) {
            if self.allow_noop_mailer {
                anyhow::bail!(
                    "APP_ALLOW_NOOP_MAILER must not be true in prod — verification emails would be silently dropped"
                );
            }
            if self.argon2_m_cost < 19_456 {
                anyhow::bail!(
                    "argon2_m_cost must be >= 19456 KiB in prod (OWASP minimum), got {}",
                    self.argon2_m_cost
                );
            }
            if self.argon2_t_cost < 2 {
                anyhow::bail!(
                    "argon2_t_cost must be >= 2 in prod (OWASP minimum), got {}",
                    self.argon2_t_cost
                );
            }
        }

        Ok(())
    }
}
