use crate::AppState;
use crate::auth::dto::{TokenPair, UserSummary};
use crate::auth::password::{Argon2Config, dummy_hash, hash_password, rehash_needed, verify_password};
use crate::auth::refresh;
use crate::auth::repo;
use crate::auth::tokens::issue_access_token;
use crate::email::service as email_service;
use crate::error::{AppError, AppResult};
use crate::users::repo as users_repo;
use chrono::Utc;
use rand::RngCore;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use tracing::{info, warn};
use uuid::Uuid;

pub struct ClientContext<'a> {
    pub ip: Option<IpAddr>,
    pub user_agent: Option<&'a str>,
}

fn ip_to_inet(ip: Option<IpAddr>) -> Option<ipnetwork::IpNetwork> {
    ip.map(ipnetwork::IpNetwork::from)
}

pub async fn register(
    state: &AppState,
    email: &str,
    password: &str,
    display_name: Option<&str>,
) -> AppResult<()> {
    let email = users_repo::normalize_email(email);
    let argon = state.argon2();

    if let Some(_existing) = users_repo::find_by_email(&state.db, &email).await? {
        // Always 202 even on duplicate to avoid enumeration. Log internally.
        warn!(email = %email, "register attempt for existing email");
        return Ok(());
    }

    let password_hash = hash_password(argon, password)?;
    let id = Uuid::now_v7();

    let mut tx = state.db.begin().await?;
    let user = users_repo::insert(
        &mut *tx,
        users_repo::NewUser {
            id,
            email: &email,
            password_hash: &password_hash,
            display_name,
        },
    )
    .await?;

    let (verify_url, token_id, hash) = build_verification_link(state, &user.id, &user.email);
    repo::insert_email_verification(
        &mut *tx,
        token_id,
        user.id,
        &hash,
        &user.email,
        Utc::now() + chrono::Duration::hours(24),
    )
    .await?;
    tx.commit().await?;

    email_service::send_verification(state, &user.email, &verify_url).await;
    info!(user_id = %user.id, "user registered");
    Ok(())
}

fn build_verification_link(state: &AppState, _user_id: &Uuid, _email: &str) -> (String, Uuid, Vec<u8>) {
    let token_id = Uuid::now_v7();
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let hash = sha256(&secret);
    let token_str = format!(
        "{}.{}",
        base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, token_id.as_bytes()),
        base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, secret),
    );
    let url = format!(
        "{base}verify?token={token}",
        base = ensure_trailing_slash(state.config.public_base_url.as_str()),
        token = token_str
    );
    (url, token_id, hash)
}

fn build_reset_link(state: &AppState) -> (String, Uuid, Vec<u8>) {
    let token_id = Uuid::now_v7();
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let hash = sha256(&secret);
    let token_str = format!(
        "{}.{}",
        base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, token_id.as_bytes()),
        base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, secret),
    );
    let url = format!(
        "{base}reset?token={token}",
        base = ensure_trailing_slash(state.config.public_base_url.as_str()),
        token = token_str
    );
    (url, token_id, hash)
}

fn parse_opaque_token(s: &str) -> Result<(Uuid, [u8; 32]), AppError> {
    let (a, b) = s.split_once('.').ok_or(AppError::InvalidToken)?;
    let id_bytes = base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, a)
        .map_err(|_| AppError::InvalidToken)?;
    if id_bytes.len() != 16 {
        return Err(AppError::InvalidToken);
    }
    let mut id_arr = [0u8; 16];
    id_arr.copy_from_slice(&id_bytes);
    let id = Uuid::from_bytes(id_arr);
    let secret_bytes = base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, b)
        .map_err(|_| AppError::InvalidToken)?;
    if secret_bytes.len() != 32 {
        return Err(AppError::InvalidToken);
    }
    let mut s = [0u8; 32];
    s.copy_from_slice(&secret_bytes);
    Ok((id, s))
}

fn sha256(input: &[u8]) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(input);
    h.finalize().to_vec()
}

fn ensure_trailing_slash(s: &str) -> String {
    if s.ends_with('/') {
        s.to_string()
    } else {
        format!("{s}/")
    }
}

pub async fn login(
    state: &AppState,
    email: &str,
    password: &str,
    ctx: ClientContext<'_>,
) -> AppResult<TokenPair> {
    let email = users_repo::normalize_email(email);
    let argon = state.argon2();

    let user_opt = users_repo::find_by_email(&state.db, &email).await?;

    let user = match user_opt {
        None => {
            // Equalize timing — always run an argon2 verify against a dummy hash.
            let _ = verify_password(dummy_hash(argon), password);
            metrics::counter!("auth_login_total", "result" => "unknown_email").increment(1);
            return Err(AppError::InvalidCredentials);
        }
        Some(u) => u,
    };

    let now = Utc::now();
    if user.is_locked(now) {
        metrics::counter!("auth_login_total", "result" => "locked").increment(1);
        return Err(AppError::AccountLocked);
    }

    let ok = verify_password(&user.password_hash, password)?;
    if !ok {
        let lock_until = now
            + chrono::Duration::from_std(state.config.login_lock_duration)
                .unwrap_or(chrono::Duration::minutes(15));
        users_repo::record_failed_login(
            &state.db,
            user.id,
            state.config.login_max_failures,
            lock_until,
        )
        .await?;
        metrics::counter!("auth_login_total", "result" => "wrong_password").increment(1);
        return Err(AppError::InvalidCredentials);
    }

    if rehash_needed(argon, &user.password_hash) {
        if let Ok(new_hash) = hash_password(argon, password) {
            let _ = users_repo::update_password(&state.db, user.id, &new_hash).await;
        }
    }

    users_repo::record_successful_login(&state.db, user.id).await?;

    let pair = mint_token_pair(state, &user.id, user.email_verified, None, &ctx).await?;
    metrics::counter!("auth_login_total", "result" => "ok").increment(1);
    Ok(TokenPair {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        token_type: "Bearer",
        access_expires_in: state.config.access_token_ttl.as_secs() as i64,
        user: UserSummary {
            id: user.id,
            email: user.email,
            email_verified: user.email_verified,
            display_name: user.display_name,
        },
    })
}

struct InternalPair {
    access_token: String,
    refresh_token: String,
}

async fn mint_token_pair(
    state: &AppState,
    user_id: &Uuid,
    email_verified: bool,
    family_id_for_rotation: Option<Uuid>,
    ctx: &ClientContext<'_>,
) -> AppResult<InternalPair> {
    let (access_token, _claims) = issue_access_token(
        &state.jwt_keys,
        *user_id,
        email_verified,
        state.config.access_token_ttl,
    )
    .map_err(AppError::Internal)?;

    let refresh = refresh::generate();
    let family_id = family_id_for_rotation.unwrap_or_else(Uuid::now_v7);
    let expires_at = Utc::now()
        + chrono::Duration::from_std(state.config.refresh_token_ttl)
            .unwrap_or(chrono::Duration::days(30));

    repo::insert_refresh(
        &state.db,
        refresh.id,
        *user_id,
        family_id,
        &refresh.hash,
        None,
        expires_at,
        ctx.user_agent,
        ip_to_inet(ctx.ip),
    )
    .await?;

    Ok(InternalPair {
        access_token,
        refresh_token: refresh.wire,
    })
}

pub async fn refresh_token(
    state: &AppState,
    presented_wire: &str,
    ctx: ClientContext<'_>,
) -> AppResult<TokenPair> {
    let parsed = refresh::parse(presented_wire)?;
    let now = Utc::now();

    let mut tx = state.db.begin().await?;

    let row = repo::find_refresh_by_id(&mut *tx, parsed.id).await?;
    let row = match row {
        None => return Err(AppError::InvalidToken),
        Some(r) => r,
    };

    if !refresh::ct_eq(&refresh::sha256(&parsed.secret), &row.token_hash) {
        return Err(AppError::InvalidToken);
    }
    if row.expires_at <= now {
        return Err(AppError::InvalidToken);
    }
    if row.revoked_at.is_some() {
        // If this is a stale "rotated" token being replayed, that's reuse — kill the family.
        if row.revoked_reason.as_deref() == Some("rotated") {
            let _ = repo::revoke_family(&mut *tx, row.family_id, "reuse_detected").await;
            tx.commit().await?;
            metrics::counter!("auth_refresh_reuse_detected_total").increment(1);
            return Err(AppError::TokenReuseDetected);
        }
        return Err(AppError::InvalidToken);
    }
    if row.used_at.is_some() {
        let _ = repo::revoke_family(&mut *tx, row.family_id, "reuse_detected").await;
        tx.commit().await?;
        metrics::counter!("auth_refresh_reuse_detected_total").increment(1);
        return Err(AppError::TokenReuseDetected);
    }

    // Rotate: mark old as rotated, insert new with same family.
    repo::mark_rotated(&mut *tx, row.id).await?;

    let new_refresh = refresh::generate();
    let user = users_repo::find_by_id(&mut *tx, row.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let expires_at = now
        + chrono::Duration::from_std(state.config.refresh_token_ttl)
            .unwrap_or(chrono::Duration::days(30));
    repo::insert_refresh(
        &mut *tx,
        new_refresh.id,
        user.id,
        row.family_id,
        &new_refresh.hash,
        Some(row.id),
        expires_at,
        ctx.user_agent,
        ip_to_inet(ctx.ip),
    )
    .await?;

    let (access_token, _claims) = issue_access_token(
        &state.jwt_keys,
        user.id,
        user.email_verified,
        state.config.access_token_ttl,
    )
    .map_err(AppError::Internal)?;

    tx.commit().await?;

    metrics::counter!("auth_refresh_total", "result" => "ok").increment(1);
    Ok(TokenPair {
        access_token,
        refresh_token: new_refresh.wire,
        token_type: "Bearer",
        access_expires_in: state.config.access_token_ttl.as_secs() as i64,
        user: UserSummary {
            id: user.id,
            email: user.email,
            email_verified: user.email_verified,
            display_name: user.display_name,
        },
    })
}

pub async fn logout(state: &AppState, presented_wire: &str) -> AppResult<()> {
    let parsed = refresh::parse(presented_wire)?;
    repo::revoke_token(&state.db, parsed.id, "logout").await?;
    Ok(())
}

pub async fn logout_all(state: &AppState, user_id: Uuid) -> AppResult<()> {
    repo::revoke_all_for_user(&state.db, user_id, "logout_all").await?;
    Ok(())
}

pub async fn verify_email(state: &AppState, token: &str) -> AppResult<()> {
    let (_, secret) = parse_opaque_token(token)?;
    let hash = sha256(&secret);
    let mut tx = state.db.begin().await?;
    let consumed = repo::consume_email_verification(&mut *tx, &hash, Utc::now()).await?;
    let (user_id, _email) = match consumed {
        Some(t) => t,
        None => return Err(AppError::InvalidToken),
    };
    users_repo::mark_email_verified(&mut *tx, user_id).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn resend_verification(state: &AppState, email: &str) -> AppResult<()> {
    let email = users_repo::normalize_email(email);
    let user = users_repo::find_by_email(&state.db, &email).await?;
    let Some(user) = user else {
        return Ok(()); // 202 either way to prevent enumeration
    };
    if user.email_verified {
        return Ok(());
    }
    let (verify_url, token_id, hash) = build_verification_link(state, &user.id, &user.email);
    repo::insert_email_verification(
        &state.db,
        token_id,
        user.id,
        &hash,
        &user.email,
        Utc::now() + chrono::Duration::hours(24),
    )
    .await?;
    email_service::send_verification(state, &user.email, &verify_url).await;
    Ok(())
}

pub async fn request_password_reset(
    state: &AppState,
    email: &str,
    ip: Option<IpAddr>,
) -> AppResult<()> {
    let email = users_repo::normalize_email(email);
    let user = users_repo::find_by_email(&state.db, &email).await?;
    let Some(user) = user else {
        return Ok(()); // 202 either way
    };
    let (reset_url, token_id, hash) = build_reset_link(state);
    repo::insert_password_reset(
        &state.db,
        token_id,
        user.id,
        &hash,
        Utc::now() + chrono::Duration::hours(1),
        ip.map(ipnetwork::IpNetwork::from),
    )
    .await?;
    email_service::send_password_reset(state, &user.email, &reset_url).await;
    Ok(())
}

pub async fn confirm_password_reset(
    state: &AppState,
    token: &str,
    new_password: &str,
) -> AppResult<()> {
    let (_, secret) = parse_opaque_token(token)?;
    let hash = sha256(&secret);
    let argon = state.argon2();
    let new_hash = hash_password(argon, new_password)?;

    let mut tx = state.db.begin().await?;
    let consumed = repo::consume_password_reset(&mut *tx, &hash, Utc::now()).await?;
    let user_id = consumed.ok_or(AppError::InvalidToken)?;
    users_repo::update_password(&mut *tx, user_id, &new_hash).await?;
    repo::revoke_all_for_user(&mut *tx, user_id, "password_reset").await?;
    tx.commit().await?;
    Ok(())
}

#[allow(dead_code)]
fn _ensure_argon_used(_a: Argon2Config) {} // keep import alive in some build configs
