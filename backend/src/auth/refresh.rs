use crate::error::AppError;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

pub const REFRESH_VERSION_PREFIX: &str = "v1.";
const SECRET_BYTES: usize = 32;

#[derive(Debug)]
pub struct NewRefreshToken {
    pub id: Uuid,
    pub wire: String,
    pub hash: Vec<u8>,
}

#[derive(Debug)]
pub struct ParsedRefreshToken {
    pub id: Uuid,
    pub secret: [u8; SECRET_BYTES],
}

pub fn generate() -> NewRefreshToken {
    let id = Uuid::now_v7();
    let mut secret = [0u8; SECRET_BYTES];
    OsRng.fill_bytes(&mut secret);
    let wire = format!(
        "{prefix}{id}.{secret}",
        prefix = REFRESH_VERSION_PREFIX,
        id = URL_SAFE_NO_PAD.encode(id.as_bytes()),
        secret = URL_SAFE_NO_PAD.encode(secret),
    );
    let hash = sha256(&secret);
    NewRefreshToken { id, wire, hash }
}

pub fn parse(wire: &str) -> Result<ParsedRefreshToken, AppError> {
    let stripped = wire
        .strip_prefix(REFRESH_VERSION_PREFIX)
        .ok_or(AppError::InvalidToken)?;
    let (id_b64, secret_b64) = stripped.split_once('.').ok_or(AppError::InvalidToken)?;
    let id_bytes = URL_SAFE_NO_PAD
        .decode(id_b64)
        .map_err(|_| AppError::InvalidToken)?;
    if id_bytes.len() != 16 {
        return Err(AppError::InvalidToken);
    }
    let mut id_arr = [0u8; 16];
    id_arr.copy_from_slice(&id_bytes);
    let id = Uuid::from_bytes(id_arr);
    let secret_bytes = URL_SAFE_NO_PAD
        .decode(secret_b64)
        .map_err(|_| AppError::InvalidToken)?;
    if secret_bytes.len() != SECRET_BYTES {
        return Err(AppError::InvalidToken);
    }
    let mut secret = [0u8; SECRET_BYTES];
    secret.copy_from_slice(&secret_bytes);
    Ok(ParsedRefreshToken { id, secret })
}

pub fn sha256(input: &[u8]) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(input);
    h.finalize().to_vec()
}

pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_parse_roundtrip() {
        let t = generate();
        let parsed = parse(&t.wire).unwrap();
        assert_eq!(parsed.id, t.id);
        assert!(ct_eq(&sha256(&parsed.secret), &t.hash));
    }

    #[test]
    fn rejects_missing_prefix() {
        let t = generate();
        let no_prefix = t
            .wire
            .strip_prefix(REFRESH_VERSION_PREFIX)
            .unwrap()
            .to_string();
        assert!(parse(&no_prefix).is_err());
    }

    #[test]
    fn rejects_bad_format() {
        assert!(parse("v1.").is_err());
        assert!(parse("v1.abc").is_err());
        assert!(parse("v1.AAAA.BBBB").is_err());
    }
}
