use crate::error::AppError;
use anyhow::{Context, Result};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;
use zeroize::Zeroizing;

const CLOCK_SKEW_LEEWAY_SECS: u64 = 30;

/// One additional verify-only public key carried alongside the active signing
/// key, keyed by `kid`. Used to keep tokens issued under a previous kid valid
/// during a zero-downtime key rotation. Configured via
/// `APP_JWT_PREVIOUS_PUBLIC_KEYS` as a JSON array.
#[derive(Debug, Deserialize, Clone)]
pub struct PreviousPublicKey {
    pub kid: String,
    pub public_key_pem: String,
}

#[derive(Clone)]
pub struct JwtKeys {
    /// The kid used to sign newly minted tokens. Always present in `decoding`.
    pub kid: String,
    pub issuer: String,
    pub audience: String,
    encoding: EncodingKey,
    /// kid → DecodingKey. Holds the active key plus any retired-but-still-valid keys.
    decoding: HashMap<String, DecodingKey>,
}

impl std::fmt::Debug for JwtKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtKeys")
            .field("kid", &self.kid)
            .field("issuer", &self.issuer)
            .field("audience", &self.audience)
            .field("known_kids", &self.decoding.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl JwtKeys {
    /// Build a key set with one active signing key and zero or more retired
    /// verify-only keys. The active kid must not collide with a previous kid.
    pub fn from_pem_set(
        active_kid: impl Into<String>,
        issuer: impl Into<String>,
        audience: impl Into<String>,
        active_private_key_pem: &str,
        active_public_key_pem: &str,
        previous: &[PreviousPublicKey],
    ) -> Result<Self> {
        let private = Zeroizing::new(active_private_key_pem.as_bytes().to_vec());
        let encoding =
            EncodingKey::from_ed_pem(&private).context("invalid Ed25519 private key PEM")?;
        let active_kid = active_kid.into();
        let active_decoding = DecodingKey::from_ed_pem(active_public_key_pem.as_bytes())
            .context("invalid Ed25519 active public key PEM")?;

        let mut decoding = HashMap::with_capacity(1 + previous.len());
        decoding.insert(active_kid.clone(), active_decoding);
        for pk in previous {
            if pk.kid == active_kid {
                anyhow::bail!(
                    "APP_JWT_PREVIOUS_PUBLIC_KEYS contains kid {:?} which collides with the active APP_JWT_KID",
                    pk.kid
                );
            }
            let dk = DecodingKey::from_ed_pem(pk.public_key_pem.as_bytes())
                .with_context(|| format!("invalid Ed25519 public key PEM for kid {:?}", pk.kid))?;
            if decoding.insert(pk.kid.clone(), dk).is_some() {
                anyhow::bail!(
                    "APP_JWT_PREVIOUS_PUBLIC_KEYS contains duplicate kid {:?}",
                    pk.kid
                );
            }
        }

        Ok(Self {
            kid: active_kid,
            issuer: issuer.into(),
            audience: audience.into(),
            encoding,
            decoding,
        })
    }

    /// Convenience: single-key construction (equivalent to `from_pem_set` with
    /// no previous keys). Kept primarily for tests.
    pub fn from_pem(
        kid: impl Into<String>,
        issuer: impl Into<String>,
        audience: impl Into<String>,
        private_key_pem: &str,
        public_key_pem: &str,
    ) -> Result<Self> {
        Self::from_pem_set(kid, issuer, audience, private_key_pem, public_key_pem, &[])
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccessClaims {
    pub sub: Uuid,
    pub iss: String,
    pub aud: String,
    pub iat: i64,
    pub nbf: i64,
    pub exp: i64,
    pub jti: Uuid,
    pub email_verified: bool,
}

pub fn issue_access_token(
    keys: &JwtKeys,
    user_id: Uuid,
    email_verified: bool,
    ttl: Duration,
) -> Result<(String, AccessClaims)> {
    let now = chrono::Utc::now().timestamp();
    let claims = AccessClaims {
        sub: user_id,
        iss: keys.issuer.clone(),
        aud: keys.audience.clone(),
        iat: now,
        nbf: now - CLOCK_SKEW_LEEWAY_SECS as i64,
        exp: now + ttl.as_secs() as i64,
        jti: Uuid::now_v7(),
        email_verified,
    };
    let mut header = Header::new(Algorithm::EdDSA);
    header.kid = Some(keys.kid.clone());
    let token = encode(&header, &claims, &keys.encoding).context("failed to sign access token")?;
    Ok((token, claims))
}

pub fn decode_access_token(keys: &JwtKeys, token: &str) -> Result<AccessClaims, AppError> {
    // Peek at the header so we can pick the right verify key when multiple are
    // configured (rotation window). `decode_header` does not verify the
    // signature, but the subsequent `decode` call does.
    let header = decode_header(token).map_err(|_| AppError::InvalidToken)?;
    let kid = header.kid.as_deref().ok_or(AppError::InvalidToken)?;
    let decoding_key = keys.decoding.get(kid).ok_or(AppError::InvalidToken)?;

    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.set_audience(&[&keys.audience]);
    validation.set_issuer(&[&keys.issuer]);
    validation.leeway = CLOCK_SKEW_LEEWAY_SECS;
    let data = decode::<AccessClaims>(token, decoding_key, &validation)
        .map_err(|_| AppError::InvalidToken)?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey, spki::der::pem::LineEnding};

    fn fresh_pem() -> (String, String) {
        let mut seed = [0u8; 32];
        crate::auth::refresh::fill_random(&mut seed);
        let signing = SigningKey::from_bytes(&seed);
        let priv_pem = signing.to_pkcs8_pem(LineEnding::LF).unwrap().to_string();
        let pub_pem = signing
            .verifying_key()
            .to_public_key_pem(LineEnding::LF)
            .unwrap();
        (priv_pem, pub_pem)
    }

    fn test_keys() -> JwtKeys {
        let (priv_pem, pub_pem) = fresh_pem();
        JwtKeys::from_pem("k1", "test-iss", "mobile", &priv_pem, &pub_pem).unwrap()
    }

    #[test]
    fn roundtrip_encode_decode() {
        let keys = test_keys();
        let user = Uuid::now_v7();
        let (tok, claims) =
            issue_access_token(&keys, user, true, Duration::from_secs(900)).unwrap();
        let decoded = decode_access_token(&keys, &tok).unwrap();
        assert_eq!(decoded.sub, claims.sub);
        assert!(decoded.email_verified);
    }

    #[test]
    fn rejects_wrong_audience() {
        let keys = test_keys();
        let user = Uuid::now_v7();
        let (tok, _) = issue_access_token(&keys, user, false, Duration::from_secs(60)).unwrap();
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_audience(&["wrong"]);
        validation.set_issuer(&[&keys.issuer]);
        let res = decode::<AccessClaims>(&tok, keys.decoding.get(&keys.kid).unwrap(), &validation);
        assert!(res.is_err());
    }

    #[test]
    fn rejects_expired() {
        let keys = test_keys();
        let user = Uuid::now_v7();
        let (tok, _) = issue_access_token(&keys, user, false, Duration::from_secs(0)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_audience(&[&keys.audience]);
        validation.set_issuer(&[&keys.issuer]);
        validation.leeway = 0;
        let res = decode::<AccessClaims>(&tok, keys.decoding.get(&keys.kid).unwrap(), &validation);
        assert!(res.is_err());
    }

    #[test]
    fn previous_kid_still_validates_after_rotation() {
        // Old key signs the token, then we "rotate" by building a new key set
        // where the new key is active and the old one is carried as previous.
        let (old_priv, old_pub) = fresh_pem();
        let old_keys =
            JwtKeys::from_pem("kid-old", "test-iss", "mobile", &old_priv, &old_pub).unwrap();
        let user = Uuid::now_v7();
        let (tok, _) = issue_access_token(&old_keys, user, true, Duration::from_secs(900)).unwrap();

        let (new_priv, new_pub) = fresh_pem();
        let rotated = JwtKeys::from_pem_set(
            "kid-new",
            "test-iss",
            "mobile",
            &new_priv,
            &new_pub,
            &[PreviousPublicKey {
                kid: "kid-old".into(),
                public_key_pem: old_pub.clone(),
            }],
        )
        .unwrap();

        // Token signed under kid-old must still verify under the rotated set.
        let decoded = decode_access_token(&rotated, &tok).unwrap();
        assert_eq!(decoded.sub, user);

        // After the old kid is dropped from the set, the same token must fail.
        let new_only =
            JwtKeys::from_pem("kid-new", "test-iss", "mobile", &new_priv, &new_pub).unwrap();
        assert!(decode_access_token(&new_only, &tok).is_err());
    }

    #[test]
    fn rejects_token_with_unknown_kid() {
        let keys = test_keys();
        // Forge a header with a kid we don't know. Use a token signed with
        // an unrelated key that claims a different kid.
        let (other_priv, _) = fresh_pem();
        let encoding = EncodingKey::from_ed_pem(other_priv.as_bytes()).unwrap();
        let claims = AccessClaims {
            sub: Uuid::now_v7(),
            iss: "test-iss".into(),
            aud: "mobile".into(),
            iat: chrono::Utc::now().timestamp(),
            nbf: chrono::Utc::now().timestamp(),
            exp: chrono::Utc::now().timestamp() + 60,
            jti: Uuid::now_v7(),
            email_verified: false,
        };
        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = Some("not-configured".into());
        let tok = encode(&header, &claims, &encoding).unwrap();
        assert!(decode_access_token(&keys, &tok).is_err());
    }

    #[test]
    fn rejects_duplicate_or_colliding_previous_kid() {
        let (priv1, pub1) = fresh_pem();
        let (_, pub2) = fresh_pem();

        // Active kid collision.
        let res = JwtKeys::from_pem_set(
            "k1",
            "test-iss",
            "mobile",
            &priv1,
            &pub1,
            &[PreviousPublicKey {
                kid: "k1".into(),
                public_key_pem: pub2.clone(),
            }],
        );
        assert!(res.is_err());

        // Duplicate previous kid.
        let res = JwtKeys::from_pem_set(
            "k1",
            "test-iss",
            "mobile",
            &priv1,
            &pub1,
            &[
                PreviousPublicKey {
                    kid: "k0".into(),
                    public_key_pem: pub2.clone(),
                },
                PreviousPublicKey {
                    kid: "k0".into(),
                    public_key_pem: pub2.clone(),
                },
            ],
        );
        assert!(res.is_err());
    }
}
