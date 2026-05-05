use crate::auth::JwtKeys;
use crate::auth::password::Argon2Config;
use crate::config::Config;
use crate::email::mailer::DynMailer;
use crate::redis_pool::RedisPool;
use sqlx::PgPool;
use std::sync::Arc;
use webauthn_rs::Webauthn;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: PgPool,
    pub redis: RedisPool,
    pub jwt_keys: Arc<JwtKeys>,
    pub mailer: DynMailer,
    pub webauthn: Arc<Webauthn>,
}

impl AppState {
    pub fn argon2(&self) -> Argon2Config {
        Argon2Config {
            m_cost: self.config.argon2_m_cost,
            t_cost: self.config.argon2_t_cost,
            p_cost: self.config.argon2_p_cost,
        }
    }
}
