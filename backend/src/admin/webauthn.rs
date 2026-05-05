//! WebAuthn (passkey) endpoints for the admin console.
//!
//! Two responsibilities:
//!
//! 1. **Registration** — an authenticated admin enrolls a passkey under their
//!    own account (`/admin/api/webauthn/register/begin` + `.../finish`).
//!    Multiple passkeys per admin are encouraged: a primary key on the work
//!    laptop, a backup on a phone or hardware token. Without this, losing the
//!    one device locks the admin out.
//!
//! 2. **Login completion** — once an admin has at least one registered passkey,
//!    the password step alone is no longer sufficient. `POST /admin/api/login`
//!    verifies the password and, finding registered credentials, returns a
//!    short-lived `pending_token` plus the WebAuthn challenge instead of
//!    setting session cookies. The browser performs `navigator.credentials.get`
//!    and posts the assertion back to `/admin/api/login/webauthn/finish`,
//!    which is where session cookies finally get issued.
//!
//! Cross-request state (the registration challenge, the authentication
//! challenge) is parked in Redis with a 5-minute TTL. Crash-restart of the
//! server only forces the user to retry the ceremony — there is no DB write
//! until `finish` succeeds.
use crate::AppState;
use crate::admin::repo::AdminUserRow;
use crate::auth::events::{self, EventCtx, EventKind};
use crate::auth::refresh;
use crate::config::AppEnv;
use crate::error::{AppError, AppResult, ErrorBody};
use crate::middleware::peer::client_ip;
use crate::middleware::rate_limit::{self, Class};
use axum::Json;
use axum::Router;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{AppendHeaders, IntoResponse, Response};
use axum::routing::{delete, get, post};
use base64::Engine;
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::net::SocketAddr;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;
use webauthn_rs::prelude::*;

use super::routes::AdminUser;

const PENDING_LOGIN_TTL_SECS: u64 = 300;
const REGISTER_STATE_TTL_SECS: u64 = 300;

// --------------------------------------------------------------- DTOs

#[derive(Debug, Serialize, ToSchema)]
pub struct RegisterBeginResponse {
    /// PublicKeyCredentialCreationOptions, ready to be passed straight to
    /// `navigator.credentials.create({ publicKey })` after the browser-side
    /// helper converts the base64url-encoded fields to ArrayBuffers.
    pub challenge: serde_json::Value,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterFinishRequest {
    /// Friendly label (e.g. "MacBook Touch ID", "YubiKey 5C"). Shown back in
    /// the credential list so the admin knows which device to remove later.
    #[validate(length(min = 1, max = 80))]
    pub label: String,
    /// `RegisterPublicKeyCredential` JSON as produced by `@simplewebauthn/browser`.
    pub credential: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CredentialSummary {
    pub id: Uuid,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CredentialListResponse {
    pub items: Vec<CredentialSummary>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginFinishOk {
    pub user_id: Uuid,
    pub email: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginFinishRequest {
    #[validate(length(min = 1, max = 200))]
    pub pending_token: String,
    /// `PublicKeyCredential` JSON as produced by `@simplewebauthn/browser`.
    pub assertion: serde_json::Value,
}

// --------------------------------------------------------------- Router

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/register/begin", post(register_begin))
        .route("/register/finish", post(register_finish))
        .route("/credentials", get(list_credentials))
        .route("/credentials/{id}", delete(delete_credential))
        .route(
            "/login/finish",
            post(login_finish).layer(rate_limit::layer(&state, Class::Strict)),
        )
        .with_state(state)
}

// --------------------------------------------------------------- Registration

#[utoipa::path(
    post,
    path = "/admin/api/webauthn/register/begin",
    tag = "admin-webauthn",
    responses(
        (status = 200, description = "Creation challenge", body = RegisterBeginResponse),
        (status = 401, description = "No active admin session", body = ErrorBody),
        (status = 403, description = "CSRF token missing or invalid", body = ErrorBody),
    ),
)]
pub async fn register_begin(
    State(state): State<AppState>,
    admin: AdminUser,
) -> AppResult<Json<RegisterBeginResponse>> {
    // Exclude any keys this admin has already registered so the browser refuses
    // to enroll the same authenticator twice.
    let existing = list_credential_ids_for_user(&state.db, admin.user_id).await?;

    let (creation, reg_state) = state
        .webauthn
        .start_passkey_registration(admin.user_id, &admin.email, &admin.email, Some(existing))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("webauthn start_register: {e}")))?;

    let payload = serde_json::to_string(&reg_state)?;
    redis_set_ex(
        &state,
        &register_state_key(admin.user_id),
        &payload,
        REGISTER_STATE_TTL_SECS,
    )
    .await?;

    Ok(Json(RegisterBeginResponse {
        challenge: serde_json::to_value(creation)?,
    }))
}

#[utoipa::path(
    post,
    path = "/admin/api/webauthn/register/finish",
    tag = "admin-webauthn",
    request_body = RegisterFinishRequest,
    responses(
        (status = 200, description = "Credential stored", body = CredentialSummary),
        (status = 401, description = "No active admin session", body = ErrorBody),
        (status = 403, description = "CSRF token missing or invalid", body = ErrorBody),
        (status = 422, description = "Validation failed", body = ErrorBody),
    ),
)]
pub async fn register_finish(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(body): Json<RegisterFinishRequest>,
) -> AppResult<Json<CredentialSummary>> {
    body.validate().map_err(AppError::from)?;

    let reg_state_json = redis_get(&state, &register_state_key(admin.user_id))
        .await?
        .ok_or(AppError::InvalidToken)?;
    let reg_state: PasskeyRegistration = serde_json::from_str(&reg_state_json)?;

    let credential: RegisterPublicKeyCredential = serde_json::from_value(body.credential)
        .map_err(|_| AppError::BadRequest("malformed credential payload"))?;

    let passkey = state
        .webauthn
        .finish_passkey_registration(&credential, &reg_state)
        .map_err(|_| AppError::Unauthorized)?;

    // One-shot: the registration state is consumed.
    let _ = redis_del(&state, &register_state_key(admin.user_id)).await;

    let row_id = Uuid::now_v7();
    let credential_id = passkey.cred_id().as_ref().to_vec();
    let passkey_json = serde_json::to_value(&passkey)?;
    sqlx::query(
        r#"INSERT INTO webauthn_credentials
                (id, admin_user_id, credential_id, passkey, label, created_at)
           VALUES ($1, $2, $3, $4, $5, now())"#,
    )
    .bind(row_id)
    .bind(admin.user_id)
    .bind(&credential_id)
    .bind(&passkey_json)
    .bind(&body.label)
    .execute(&state.db)
    .await?;

    events::record(
        &state.db,
        EventKind::AdminWebauthnRegistered,
        EventCtx {
            user_id: Some(admin.user_id),
            ip: None,
            user_agent: None,
            detail: Some(serde_json::json!({ "credential_row": row_id, "label": body.label })),
        },
    )
    .await;

    Ok(Json(CredentialSummary {
        id: row_id,
        label: body.label,
        created_at: Utc::now(),
        last_used_at: None,
    }))
}

// --------------------------------------------------------------- List / delete

#[utoipa::path(
    get,
    path = "/admin/api/webauthn/credentials",
    tag = "admin-webauthn",
    responses(
        (status = 200, description = "Passkeys for the current admin", body = CredentialListResponse),
        (status = 401, description = "No active admin session", body = ErrorBody),
    ),
)]
pub async fn list_credentials(
    State(state): State<AppState>,
    admin: AdminUser,
) -> AppResult<Json<CredentialListResponse>> {
    let rows = sqlx::query_as::<_, CredentialRow>(
        r#"SELECT id, label, created_at, last_used_at
             FROM webauthn_credentials
            WHERE admin_user_id = $1
            ORDER BY created_at DESC"#,
    )
    .bind(admin.user_id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(CredentialListResponse {
        items: rows
            .into_iter()
            .map(|r| CredentialSummary {
                id: r.id,
                label: r.label,
                created_at: r.created_at,
                last_used_at: r.last_used_at,
            })
            .collect(),
    }))
}

#[utoipa::path(
    delete,
    path = "/admin/api/webauthn/credentials/{id}",
    tag = "admin-webauthn",
    params(("id" = Uuid, Path, description = "Credential row id")),
    responses(
        (status = 200, description = "Credential removed"),
        (status = 401, description = "No active admin session", body = ErrorBody),
        (status = 403, description = "CSRF token missing or invalid", body = ErrorBody),
        (status = 409, description = "Cannot remove final admin passkey", body = ErrorBody),
        (status = 404, description = "Credential not found", body = ErrorBody),
    ),
)]
pub async fn delete_credential(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let mut tx = state.db.begin().await?;

    let rows: Vec<(Uuid,)> = sqlx::query_as(
        r#"SELECT id
           FROM webauthn_credentials
           WHERE admin_user_id = $1
           FOR UPDATE"#,
    )
    .bind(admin.user_id)
    .fetch_all(&mut *tx)
    .await?;
    if !rows.iter().any(|row| row.0 == id) {
        return Err(AppError::NotFound);
    }
    if rows.len() <= 1 {
        return Err(AppError::Conflict("cannot delete final admin passkey"));
    }

    let n = sqlx::query("DELETE FROM webauthn_credentials WHERE id = $1 AND admin_user_id = $2")
        .bind(id)
        .bind(admin.user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(AppError::NotFound);
    }
    tx.commit().await?;

    events::record(
        &state.db,
        EventKind::AdminWebauthnRemoved,
        EventCtx {
            user_id: Some(admin.user_id),
            ip: None,
            user_agent: None,
            detail: Some(serde_json::json!({ "credential_row": id })),
        },
    )
    .await;
    Ok(StatusCode::OK)
}

// --------------------------------------------------------------- Login finish

#[utoipa::path(
    post,
    path = "/admin/api/webauthn/login/finish",
    tag = "admin-webauthn",
    request_body = LoginFinishRequest,
    responses(
        (status = 200, description = "Logged in; sets admin_session and admin_csrf cookies", body = LoginFinishOk),
        (status = 401, description = "Invalid or expired pending token, or assertion failed", body = ErrorBody),
        (status = 422, description = "Validation failed", body = ErrorBody),
        (status = 429, description = "Rate limited", body = ErrorBody),
    ),
)]
pub async fn login_finish(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<LoginFinishRequest>,
) -> AppResult<Response> {
    body.validate().map_err(AppError::from)?;
    let ip = Some(client_ip(
        &headers,
        peer.ip(),
        &state.config.trusted_proxy_cidrs,
    ));
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok());

    // 1. Recover the admin id + authentication state we parked at the password step.
    let pending_json = redis_get(&state, &pending_login_key(&body.pending_token))
        .await?
        .ok_or(AppError::InvalidToken)?;
    let pending: PendingLoginState = serde_json::from_str(&pending_json)?;
    // Single-use: drop the marker before doing the (slow) signature verification
    // so a replay can't race a second finish call.
    let _ = redis_del(&state, &pending_login_key(&body.pending_token)).await;

    // 2. Verify the assertion.
    let public_key_credential: PublicKeyCredential = serde_json::from_value(body.assertion)
        .map_err(|_| AppError::BadRequest("malformed assertion payload"))?;
    let auth_result = state
        .webauthn
        .finish_passkey_authentication(&public_key_credential, &pending.auth_state)
        .map_err(|_| AppError::Unauthorized)?;

    // 3. Re-fetch the admin row so we mint the session against the current DB state
    //    (role/lock could have changed between password step and this call).
    let admin = super::repo::find_admin_by_id(&state.db, pending.admin_user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let now = Utc::now();
    if admin.locked_until.map(|t| t > now).unwrap_or(false) {
        return Err(AppError::InvalidCredentials);
    }

    // 4. Bump the credential's sign counter if webauthn-rs says we should.
    if auth_result.needs_update() {
        update_passkey_after_auth(&state.db, auth_result.cred_id(), &auth_result).await?;
    } else {
        // Touch last_used_at even when the counter didn't move — useful for the
        // admin "last used" UI.
        sqlx::query(
            "UPDATE webauthn_credentials SET last_used_at = now() WHERE credential_id = $1",
        )
        .bind(auth_result.cred_id().as_ref())
        .execute(&state.db)
        .await?;
    }

    // 5. Mint the admin session + CSRF cookies (same shape as the password-only path).
    let (session_token, session_id, session_secret) = new_token();
    let (csrf_token, _, _) = new_token();
    super::repo::insert_session(
        &state.db,
        session_id,
        admin.id,
        &sha256(&session_secret),
        &sha256(csrf_token.as_bytes()),
        now + chrono::Duration::hours(12),
        ip.map(ipnetwork::IpNetwork::from),
        user_agent,
    )
    .await?;

    events::record(
        &state.db,
        EventKind::AdminLoginSuccess,
        EventCtx {
            user_id: Some(admin.id),
            ip,
            user_agent,
            detail: Some(serde_json::json!({ "factor": "password+webauthn" })),
        },
    )
    .await;

    let max_age = 12 * 3600;
    Ok((
        StatusCode::OK,
        AppendHeaders([
            (
                header::SET_COOKIE,
                session_cookie(&state, &session_token, Some(max_age)),
            ),
            (
                header::SET_COOKIE,
                csrf_cookie(&state, &csrf_token, Some(max_age)),
            ),
        ]),
        Json(LoginFinishOk {
            user_id: admin.id,
            email: admin.email,
        }),
    )
        .into_response())
}

// --------------------------------------------------------------- Helpers used by routes::login

/// State parked between the password step and the WebAuthn assertion step.
#[derive(Debug, Serialize, Deserialize)]
pub struct PendingLoginState {
    pub admin_user_id: Uuid,
    pub auth_state: PasskeyAuthentication,
}

/// Whether this admin has at least one registered passkey. If true, the
/// password step alone is not sufficient and the login route must defer
/// session-cookie issuance to `/admin/api/login/webauthn/finish`.
pub async fn admin_has_credentials(db: &PgPool, admin_user_id: Uuid) -> AppResult<bool> {
    let (n,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM webauthn_credentials WHERE admin_user_id = $1")
            .bind(admin_user_id)
            .fetch_one(db)
            .await?;
    Ok(n > 0)
}

/// Begin authentication. Returns the wire-format challenge and the in-memory
/// state. Caller is responsible for parking the state in Redis under a
/// pending-login token before responding to the client.
pub async fn start_admin_authentication(
    state: &AppState,
    admin: &AdminUserRow,
) -> AppResult<(serde_json::Value, PasskeyAuthentication)> {
    let passkeys = load_passkeys(&state.db, admin.id).await?;
    if passkeys.is_empty() {
        // Defensive: the caller should have checked admin_has_credentials first.
        return Err(AppError::Internal(anyhow::anyhow!(
            "start_admin_authentication called with no passkeys"
        )));
    }
    let (request_options, auth_state) = state
        .webauthn
        .start_passkey_authentication(&passkeys)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("webauthn start_auth: {e}")))?;
    Ok((serde_json::to_value(request_options)?, auth_state))
}

/// Park the pending login in Redis under a fresh opaque token. The token is
/// returned to the client and must be presented to `/login/webauthn/finish`.
pub async fn park_pending_login(
    state: &AppState,
    admin_user_id: Uuid,
    auth_state: PasskeyAuthentication,
) -> AppResult<String> {
    let pending = PendingLoginState {
        admin_user_id,
        auth_state,
    };
    let pending_token = random_token();
    let payload = serde_json::to_string(&pending)?;
    redis_set_ex(
        state,
        &pending_login_key(&pending_token),
        &payload,
        PENDING_LOGIN_TTL_SECS,
    )
    .await?;
    Ok(pending_token)
}

// --------------------------------------------------------------- Internals

#[derive(sqlx::FromRow)]
struct CredentialRow {
    id: Uuid,
    label: String,
    created_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
}

async fn list_credential_ids_for_user(
    db: &PgPool,
    admin_user_id: Uuid,
) -> AppResult<Vec<CredentialID>> {
    let rows: Vec<(Vec<u8>,)> =
        sqlx::query_as("SELECT credential_id FROM webauthn_credentials WHERE admin_user_id = $1")
            .bind(admin_user_id)
            .fetch_all(db)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(bytes,)| CredentialID::from(bytes))
        .collect())
}

async fn load_passkeys(db: &PgPool, admin_user_id: Uuid) -> AppResult<Vec<Passkey>> {
    let rows: Vec<(serde_json::Value,)> =
        sqlx::query_as("SELECT passkey FROM webauthn_credentials WHERE admin_user_id = $1")
            .bind(admin_user_id)
            .fetch_all(db)
            .await?;
    let mut out = Vec::with_capacity(rows.len());
    for (raw,) in rows {
        let pk: Passkey = serde_json::from_value(raw)?;
        out.push(pk);
    }
    Ok(out)
}

async fn update_passkey_after_auth(
    db: &PgPool,
    cred_id: &CredentialID,
    auth_result: &AuthenticationResult,
) -> AppResult<()> {
    let raw: Option<(serde_json::Value,)> =
        sqlx::query_as("SELECT passkey FROM webauthn_credentials WHERE credential_id = $1")
            .bind(cred_id.as_ref())
            .fetch_optional(db)
            .await?;
    let Some((value,)) = raw else {
        return Ok(());
    };
    let mut passkey: Passkey = serde_json::from_value(value)?;
    if passkey.update_credential(auth_result).is_some() {
        let new_value = serde_json::to_value(&passkey)?;
        sqlx::query(
            "UPDATE webauthn_credentials SET passkey = $1, last_used_at = now() WHERE credential_id = $2",
        )
        .bind(&new_value)
        .bind(cred_id.as_ref())
        .execute(db)
        .await?;
    }
    Ok(())
}

fn pending_login_key(token: &str) -> String {
    format!("admin_webauthn_login:{token}")
}

fn register_state_key(admin_user_id: Uuid) -> String {
    format!("admin_webauthn_register:{admin_user_id}")
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    refresh::fill_random(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn new_token() -> (String, Uuid, [u8; 32]) {
    let id = Uuid::now_v7();
    let mut secret = [0u8; 32];
    refresh::fill_random(&mut secret);
    let token = format!(
        "{}.{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(id.as_bytes()),
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(secret)
    );
    (token, id, secret)
}

fn sha256(input: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(input);
    h.finalize().to_vec()
}

fn session_cookie(state: &AppState, value: &str, max_age_seconds: Option<i64>) -> String {
    let mut c = format!("admin_session={value}; HttpOnly; SameSite=Strict; Path=/admin");
    if matches!(state.config.env, AppEnv::Prod) {
        c.push_str("; Secure");
    }
    if let Some(max_age) = max_age_seconds {
        c.push_str(&format!("; Max-Age={max_age}"));
    }
    c
}

fn csrf_cookie(state: &AppState, value: &str, max_age_seconds: Option<i64>) -> String {
    let mut c = format!("admin_csrf={value}; SameSite=Strict; Path=/admin");
    if matches!(state.config.env, AppEnv::Prod) {
        c.push_str("; Secure");
    }
    if let Some(max_age) = max_age_seconds {
        c.push_str(&format!("; Max-Age={max_age}"));
    }
    c
}

async fn redis_set_ex(state: &AppState, key: &str, value: &str, ttl_secs: u64) -> AppResult<()> {
    let mut conn = state
        .redis
        .get()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("redis pool: {e}")))?;
    let _: () = conn
        .set_ex(key, value, ttl_secs)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("redis set_ex: {e}")))?;
    Ok(())
}

async fn redis_get(state: &AppState, key: &str) -> AppResult<Option<String>> {
    let mut conn = state
        .redis
        .get()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("redis pool: {e}")))?;
    let v: Option<String> = conn
        .get(key)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("redis get: {e}")))?;
    Ok(v)
}

async fn redis_del(state: &AppState, key: &str) -> AppResult<()> {
    let mut conn = state
        .redis
        .get()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("redis pool: {e}")))?;
    let _: () = conn
        .del(key)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("redis del: {e}")))?;
    Ok(())
}
