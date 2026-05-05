use crate::error::AppError;
use anyhow::Result;
use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};

#[derive(Clone, Copy, Debug)]
pub struct Argon2Config {
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl Argon2Config {
    pub fn build(self) -> Result<Argon2<'static>> {
        let params = Params::new(self.m_cost, self.t_cost, self.p_cost, Some(32))
            .map_err(|e| anyhow::anyhow!("invalid argon2 params: {e}"))?;
        Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
    }
}

pub fn hash_password(cfg: Argon2Config, password: &str) -> Result<String, AppError> {
    let argon = cfg
        .build()
        .map_err(AppError::Internal)?;
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("argon2 hash failed: {e}")))?;
    Ok(hash.to_string())
}

/// Returns Ok(true) if password matches, Ok(false) if it doesn't, error only on malformed hash.
pub fn verify_password(stored_hash: &str, candidate: &str) -> Result<bool, AppError> {
    let parsed = PasswordHash::new(stored_hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("malformed stored hash: {e}")))?;
    match Argon2::default().verify_password(candidate.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(AppError::Internal(anyhow::anyhow!("argon2 verify failed: {e}"))),
    }
}

/// True if the stored hash uses weaker params than the current target — caller should rehash.
pub fn rehash_needed(cfg: Argon2Config, stored_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored_hash) else {
        return true;
    };
    let Ok(params) = Params::try_from(&parsed) else {
        return true;
    };
    params.m_cost() < cfg.m_cost || params.t_cost() < cfg.t_cost
}

/// A pre-computed argon2 hash used to equalize timing on unknown-email login.
pub static DUMMY_HASH: std::sync::OnceLock<String> = std::sync::OnceLock::new();

pub fn dummy_hash(cfg: Argon2Config) -> &'static str {
    DUMMY_HASH.get_or_init(|| {
        hash_password(cfg, "this-is-a-dummy-password-for-timing-equalization")
            .expect("dummy hash must succeed")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cfg() -> Argon2Config {
        // Keep params tiny in tests so the suite stays fast.
        Argon2Config {
            m_cost: 8,
            t_cost: 1,
            p_cost: 1,
        }
    }

    #[test]
    fn hash_verify_roundtrip() {
        let cfg = test_cfg();
        let h = hash_password(cfg, "correct horse battery staple").unwrap();
        assert!(verify_password(&h, "correct horse battery staple").unwrap());
        assert!(!verify_password(&h, "wrong").unwrap());
    }

    #[test]
    fn rehash_needed_when_stored_weaker() {
        let weak = test_cfg();
        let h = hash_password(weak, "some_password").unwrap();
        let stronger = Argon2Config {
            m_cost: 19456,
            t_cost: 2,
            p_cost: 1,
        };
        assert!(rehash_needed(stronger, &h));
    }

    #[test]
    fn rehash_not_needed_when_stored_same() {
        let cfg = test_cfg();
        let h = hash_password(cfg, "x").unwrap();
        assert!(!rehash_needed(cfg, &h));
    }
}
