use crate::error::AppError;
use anyhow::{Context, Result};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;
use zeroize::Zeroizing;

const CLOCK_SKEW_LEEWAY_SECS: u64 = 30;

#[derive(Clone)]
pub struct JwtKeys {
    pub kid: String,
    pub issuer: String,
    pub audience: String,
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl std::fmt::Debug for JwtKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtKeys")
            .field("kid", &self.kid)
            .field("issuer", &self.issuer)
            .field("audience", &self.audience)
            .finish_non_exhaustive()
    }
}

impl JwtKeys {
    pub fn from_pem(
        kid: impl Into<String>,
        issuer: impl Into<String>,
        audience: impl Into<String>,
        private_key_pem: &str,
        public_key_pem: &str,
    ) -> Result<Self> {
        let private = Zeroizing::new(private_key_pem.as_bytes().to_vec());
        let encoding = EncodingKey::from_ed_pem(&private).context("invalid Ed25519 private key PEM")?;
        let decoding = DecodingKey::from_ed_pem(public_key_pem.as_bytes())
            .context("invalid Ed25519 public key PEM")?;
        Ok(Self {
            kid: kid.into(),
            issuer: issuer.into(),
            audience: audience.into(),
            encoding,
            decoding,
        })
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
    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.set_audience(&[&keys.audience]);
    validation.set_issuer(&[&keys.issuer]);
    validation.leeway = CLOCK_SKEW_LEEWAY_SECS;
    let data = decode::<AccessClaims>(token, &keys.decoding, &validation)
        .map_err(|_| AppError::InvalidToken)?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey, spki::der::pem::LineEnding};
    use rand::rngs::OsRng;

    fn test_keys() -> JwtKeys {
        let mut csprng = OsRng;
        let signing = SigningKey::generate(&mut csprng);
        let priv_pem = signing.to_pkcs8_pem(LineEnding::LF).unwrap().to_string();
        let pub_pem = signing
            .verifying_key()
            .to_public_key_pem(LineEnding::LF)
            .unwrap();
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
        assert_eq!(decoded.email_verified, true);
    }

    #[test]
    fn rejects_wrong_audience() {
        let keys = test_keys();
        let user = Uuid::now_v7();
        let (tok, _) = issue_access_token(&keys, user, false, Duration::from_secs(60)).unwrap();
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_audience(&["wrong"]);
        validation.set_issuer(&[&keys.issuer]);
        let res = decode::<AccessClaims>(&tok, &keys.decoding, &validation);
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
        let res = decode::<AccessClaims>(&tok, &keys.decoding, &validation);
        assert!(res.is_err());
    }
}
