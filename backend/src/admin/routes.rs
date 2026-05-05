//! JSON admin API mounted at `/admin/api/*`.
//!
//! Auth is a separate trust boundary from the regular `/v1` API: opaque
//! `id.secret` session token (SHA-256 hashed at rest) in an HttpOnly cookie,
//! plus a CSRF token in a non-HttpOnly companion cookie that the SPA echoes in
//! `X-CSRF-Token` on state-changing requests (double-submit cookie pattern).
//!
//! The `AdminUser` extractor enforces both the session and the CSRF check; for
//! safe methods (`GET`, `HEAD`, `OPTIONS`) the CSRF check is skipped.
use crate::AppState;
use crate::admin::repo::{self, AuthEventRow, ManagedUserRow, UserSessionRow};
use crate::auth::events::{self, EventCtx, EventKind};
use crate::auth::password::{dummy_hash, verify_password};
use crate::auth::refresh;
use crate::config::AppEnv;
use crate::error::{AppError, AppResult, ErrorBody};
use crate::middleware::peer::client_ip;
use crate::middleware::rate_limit::{self, Class};
use crate::users;
use axum::Json;
use axum::Router;
use axum::extract::{ConnectInfo, FromRef, FromRequestParts, Path, Query, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::response::{AppendHeaders, IntoResponse, Response};
use axum::routing::{get, post};
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use subtle::ConstantTimeEq;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

const ADMIN_SESSION_COOKIE: &str = "admin_session";
const ADMIN_CSRF_COOKIE: &str = "admin_csrf";
const ADMIN_CSRF_HEADER: &str = "x-csrf-token";
const ADMIN_SESSION_TTL_HOURS: i64 = 12;
const ADMIN_LOGIN_MAX_FAILURES: i32 = 5;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/login",
            post(login).layer(rate_limit::layer(&state, Class::Strict)),
        )
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/users", get(users_list))
        .route("/users/{id}", get(user_show))
        .route("/users/{id}/lock", post(user_lock))
        .route("/users/{id}/unlock", post(user_unlock))
        .route("/users/{id}/verify-email", post(user_verify_email))
        .route(
            "/users/{id}/sessions",
            get(user_sessions).delete(user_revoke_sessions),
        )
        .route("/auth-events", get(auth_events_list))
        .with_state(state)
}

// ---------------------------------------------------------------- DTO

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AdminLoginRequest {
    #[validate(email, length(max = 254))]
    pub email: String,
    #[validate(length(min = 1, max = 128))]
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminMe {
    pub user_id: Uuid,
    pub email: String,
    pub role: &'static str,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminLoginResponse {
    pub user_id: Uuid,
    pub email: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminAck {
    pub status: &'static str,
}

impl Default for AdminAck {
    fn default() -> Self {
        Self { status: "ok" }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ManagedUser {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub failed_login_count: i32,
    pub locked_until: Option<DateTime<Utc>>,
}

impl From<ManagedUserRow> for ManagedUser {
    fn from(r: ManagedUserRow) -> Self {
        Self {
            id: r.id,
            email: r.email,
            role: r.role,
            email_verified: r.email_verified,
            display_name: r.display_name,
            created_at: r.created_at,
            last_login_at: r.last_login_at,
            failed_login_count: r.failed_login_count,
            locked_until: r.locked_until,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthEvent {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub user_email: Option<String>,
    pub event_type: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub detail: Option<Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserSession {
    /// Refresh-token family id. Raw refresh tokens and token hashes are never exposed.
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl From<UserSessionRow> for UserSession {
    fn from(r: UserSessionRow) -> Self {
        Self {
            id: r.id,
            created_at: r.created_at,
            last_used_at: r.last_used_at,
            ip: r.ip.map(|i| i.to_string()),
            user_agent: r.user_agent,
            revoked_at: r.revoked_at,
        }
    }
}

impl From<AuthEventRow> for AuthEvent {
    fn from(r: AuthEventRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            user_email: r.user_email,
            event_type: r.event_type,
            ip: r.ip.map(|i| i.to_string()),
            user_agent: r.user_agent,
            detail: r.detail,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PageManagedUser {
    pub items: Vec<ManagedUser>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PageAuthEvent {
    pub items: Vec<AuthEvent>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserSessionsResponse {
    pub items: Vec<UserSession>,
}

#[derive(Debug, Deserialize)]
pub struct UsersListQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct AuthEventsQuery {
    pub event_type: Option<String>,
    pub user_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub cursor: Option<Uuid>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LockRequest {
    /// When the lock should expire. Defaults to ~10 years if omitted.
    pub until: Option<DateTime<Utc>>,
    #[validate(length(max = 200))]
    pub reason: Option<String>,
}

// ---------------------------------------------------------------- Extractor

#[derive(Debug, Clone)]
pub struct AdminUser {
    pub user_id: Uuid,
    pub email: String,
    pub session_id: Uuid,
}

impl<S> FromRequestParts<S> for AdminUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let session_cookie = parts
            .headers
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| read_cookie(s, ADMIN_SESSION_COOKIE))
            .ok_or(AppError::Unauthorized)?;

        let (session_id, secret) =
            parse_session_token(&session_cookie).map_err(|_| AppError::Unauthorized)?;
        let token_hash = sha256(&secret);
        let session = repo::find_session(&app_state.db, session_id, &token_hash, Utc::now())
            .await?
            .ok_or(AppError::Unauthorized)?;

        // CSRF: require for non-safe methods (POST, PATCH, PUT, DELETE).
        if !is_safe_method(&parts.method) {
            let header_token = parts
                .headers
                .get(ADMIN_CSRF_HEADER)
                .and_then(|v| v.to_str().ok())
                .ok_or(AppError::Forbidden)?;
            let presented = sha256(header_token.as_bytes());
            if presented.ct_eq(&session.csrf_hash).unwrap_u8() != 1 {
                return Err(AppError::Forbidden);
            }
        }

        Ok(AdminUser {
            user_id: session.admin_user_id,
            email: session.email,
            session_id,
        })
    }
}

fn is_safe_method(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

// ---------------------------------------------------------------- Handlers

#[utoipa::path(
    post,
    path = "/admin/api/login",
    tag = "admin",
    request_body = AdminLoginRequest,
    responses(
        (status = 200, description = "Logged in; sets admin_session and admin_csrf cookies", body = AdminLoginResponse),
        (status = 401, description = "Invalid credentials", body = ErrorBody),
        (status = 422, description = "Validation failed", body = ErrorBody),
        (status = 429, description = "Rate limited", body = ErrorBody),
    ),
)]
pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<AdminLoginRequest>,
) -> AppResult<Response> {
    body.validate().map_err(AppError::from)?;
    let email = users::repo::normalize_email(&body.email);
    let argon = state.argon2();
    let ip = Some(client_ip(
        &headers,
        peer.ip(),
        &state.config.trusted_proxy_cidrs,
    ));
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok());

    let admin = repo::find_admin_by_email(&state.db, &email).await?;
    let Some(admin) = admin else {
        // Equalize timing — verify the attempted password against a dummy hash
        // so unknown emails take the same wall-clock time as known ones.
        let _ = verify_password(dummy_hash(argon), &body.password);
        events::record(
            &state.db,
            EventKind::AdminLoginFailureUnknownEmail,
            EventCtx {
                user_id: None,
                ip,
                user_agent,
                detail: None,
            },
        )
        .await;
        return Err(AppError::InvalidCredentials);
    };

    let now = Utc::now();
    if admin.locked_until.map(|t| t > now).unwrap_or(false) {
        events::record(
            &state.db,
            EventKind::AdminLoginBlockedLocked,
            EventCtx {
                user_id: Some(admin.id),
                ip,
                user_agent,
                detail: None,
            },
        )
        .await;
        // Generic message: don't leak the locked-vs-wrong-password distinction.
        return Err(AppError::InvalidCredentials);
    }

    if !verify_password(&admin.password_hash, &body.password)? {
        let lock_until = now + chrono::Duration::hours(1);
        let now_locked = users::repo::record_failed_login(
            &state.db,
            admin.id,
            ADMIN_LOGIN_MAX_FAILURES,
            lock_until,
        )
        .await?;
        events::record(
            &state.db,
            EventKind::AdminLoginFailureWrongPassword,
            EventCtx {
                user_id: Some(admin.id),
                ip,
                user_agent,
                detail: None,
            },
        )
        .await;
        if now_locked {
            events::record(
                &state.db,
                EventKind::AccountLockedTriggered,
                EventCtx {
                    user_id: Some(admin.id),
                    ip,
                    user_agent,
                    detail: Some(serde_json::json!({ "context": "admin_login" })),
                },
            )
            .await;
        }
        return Err(AppError::InvalidCredentials);
    }

    users::repo::record_successful_login(&state.db, admin.id).await?;

    let (session_token, session_id, session_secret) = new_token();
    let (csrf_token, _, _) = new_token();
    repo::insert_session(
        &state.db,
        session_id,
        admin.id,
        &sha256(&session_secret),
        &sha256(csrf_token.as_bytes()),
        now + chrono::Duration::hours(ADMIN_SESSION_TTL_HOURS),
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
            detail: None,
        },
    )
    .await;

    let max_age = ADMIN_SESSION_TTL_HOURS * 3600;
    let response = Json(AdminLoginResponse {
        user_id: admin.id,
        email: admin.email,
    });
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
        response,
    )
        .into_response())
}

#[utoipa::path(
    post,
    path = "/admin/api/logout",
    tag = "admin",
    responses(
        (status = 200, description = "Session revoked; cookies cleared", body = AdminAck),
        (status = 401, description = "No active admin session", body = ErrorBody),
        (status = 403, description = "CSRF token missing or invalid", body = ErrorBody),
    ),
)]
pub async fn logout(State(state): State<AppState>, admin: AdminUser) -> AppResult<Response> {
    repo::revoke_session(&state.db, admin.session_id).await?;
    Ok((
        StatusCode::OK,
        AppendHeaders([
            (header::SET_COOKIE, session_cookie(&state, "", Some(0))),
            (header::SET_COOKIE, csrf_cookie(&state, "", Some(0))),
        ]),
        Json(AdminAck::default()),
    )
        .into_response())
}

#[utoipa::path(
    get,
    path = "/admin/api/me",
    tag = "admin",
    responses(
        (status = 200, description = "Current admin", body = AdminMe),
        (status = 401, description = "No active admin session", body = ErrorBody),
    ),
)]
pub async fn me(_state: State<AppState>, admin: AdminUser) -> Json<AdminMe> {
    Json(AdminMe {
        user_id: admin.user_id,
        email: admin.email,
        role: "admin",
    })
}

#[utoipa::path(
    get,
    path = "/admin/api/users",
    tag = "admin",
    params(
        ("q" = Option<String>, Query, description = "Email substring search"),
        ("limit" = Option<i64>, Query, description = "Page size (1..=200, default 50)"),
        ("cursor" = Option<Uuid>, Query, description = "Last id from previous page"),
    ),
    responses(
        (status = 200, description = "Page of users", body = PageManagedUser),
        (status = 401, description = "No active admin session", body = ErrorBody),
    ),
)]
pub async fn users_list(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<UsersListQuery>,
) -> AppResult<Json<PageManagedUser>> {
    let q = query.q.as_deref().filter(|q| !q.trim().is_empty());
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let rows = repo::list_users_paged(&state.db, q, query.cursor, limit + 1).await?;
    let (items, next_cursor) = paginate(rows, limit);
    Ok(Json(PageManagedUser {
        items: items.into_iter().map(ManagedUser::from).collect(),
        next_cursor,
    }))
}

#[utoipa::path(
    get,
    path = "/admin/api/users/{id}",
    tag = "admin",
    params(("id" = Uuid, Path, description = "User id")),
    responses(
        (status = 200, description = "User", body = ManagedUser),
        (status = 401, description = "No active admin session", body = ErrorBody),
        (status = 404, description = "User not found", body = ErrorBody),
    ),
)]
pub async fn user_show(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ManagedUser>> {
    let user = repo::find_managed_user(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(user.into()))
}

#[utoipa::path(
    post,
    path = "/admin/api/users/{id}/lock",
    tag = "admin",
    params(("id" = Uuid, Path, description = "User id")),
    request_body = LockRequest,
    responses(
        (status = 200, description = "User locked", body = AdminAck),
        (status = 401, description = "No active admin session", body = ErrorBody),
        (status = 403, description = "CSRF token missing or invalid", body = ErrorBody),
        (status = 422, description = "Validation failed", body = ErrorBody),
    ),
)]
pub async fn user_lock(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(body): Json<LockRequest>,
) -> AppResult<Json<AdminAck>> {
    body.validate().map_err(AppError::from)?;
    let until = body
        .until
        .unwrap_or_else(|| Utc::now() + chrono::Duration::days(3650));
    users::repo::lock_until(&state.db, id, until).await?;
    events::record(
        &state.db,
        EventKind::AdminUserLocked,
        EventCtx {
            user_id: Some(id),
            ip: None,
            user_agent: None,
            detail: Some(serde_json::json!({
                "by_admin": admin.user_id,
                "until": until,
                "reason": body.reason,
            })),
        },
    )
    .await;
    Ok(Json(AdminAck::default()))
}

#[utoipa::path(
    post,
    path = "/admin/api/users/{id}/unlock",
    tag = "admin",
    params(("id" = Uuid, Path, description = "User id")),
    responses(
        (status = 200, description = "Unlocked", body = AdminAck),
        (status = 401, description = "No active admin session", body = ErrorBody),
        (status = 403, description = "CSRF token missing or invalid", body = ErrorBody),
    ),
)]
pub async fn user_unlock(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<AdminAck>> {
    users::repo::clear_lock(&state.db, id).await?;
    events::record(
        &state.db,
        EventKind::AdminUserUnlocked,
        EventCtx {
            user_id: Some(id),
            ip: None,
            user_agent: None,
            detail: Some(serde_json::json!({ "by_admin": admin.user_id })),
        },
    )
    .await;
    Ok(Json(AdminAck::default()))
}

#[utoipa::path(
    post,
    path = "/admin/api/users/{id}/verify-email",
    tag = "admin",
    params(("id" = Uuid, Path, description = "User id")),
    responses(
        (status = 200, description = "Email marked verified", body = AdminAck),
        (status = 401, description = "No active admin session", body = ErrorBody),
        (status = 403, description = "CSRF token missing or invalid", body = ErrorBody),
    ),
)]
pub async fn user_verify_email(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<AdminAck>> {
    users::repo::mark_email_verified(&state.db, id).await?;
    events::record(
        &state.db,
        EventKind::AdminEmailVerified,
        EventCtx {
            user_id: Some(id),
            ip: None,
            user_agent: None,
            detail: Some(serde_json::json!({ "by_admin": admin.user_id })),
        },
    )
    .await;
    Ok(Json(AdminAck::default()))
}

#[utoipa::path(
    get,
    path = "/admin/api/users/{id}/sessions",
    tag = "admin",
    params(("id" = Uuid, Path, description = "User id")),
    responses(
        (status = 200, description = "Refresh-token session families for user", body = UserSessionsResponse),
        (status = 401, description = "No active admin session", body = ErrorBody),
        (status = 404, description = "User not found", body = ErrorBody),
    ),
)]
pub async fn user_sessions(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<UserSessionsResponse>> {
    repo::find_managed_user(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let rows = repo::list_user_sessions(&state.db, id).await?;
    Ok(Json(UserSessionsResponse {
        items: rows.into_iter().map(UserSession::from).collect(),
    }))
}

#[utoipa::path(
    delete,
    path = "/admin/api/users/{id}/sessions",
    tag = "admin",
    params(("id" = Uuid, Path, description = "User id")),
    responses(
        (status = 200, description = "All refresh tokens revoked", body = AdminAck),
        (status = 401, description = "No active admin session", body = ErrorBody),
        (status = 403, description = "CSRF token missing or invalid", body = ErrorBody),
    ),
)]
pub async fn user_revoke_sessions(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<AdminAck>> {
    let n = crate::auth::repo::revoke_all_for_user(&state.db, id, "admin_revoked").await?;
    events::record(
        &state.db,
        EventKind::AdminSessionsRevoked,
        EventCtx {
            user_id: Some(id),
            ip: None,
            user_agent: None,
            detail: Some(serde_json::json!({ "by_admin": admin.user_id, "revoked": n })),
        },
    )
    .await;
    Ok(Json(AdminAck::default()))
}

#[utoipa::path(
    get,
    path = "/admin/api/auth-events",
    tag = "admin",
    params(
        ("event_type" = Option<String>, Query, description = "Exact event_type filter"),
        ("user_id" = Option<Uuid>, Query, description = "Filter by user id"),
        ("limit" = Option<i64>, Query, description = "Page size (1..=500, default 100)"),
        ("cursor" = Option<Uuid>, Query, description = "Last id from previous page"),
    ),
    responses(
        (status = 200, description = "Page of audit events", body = PageAuthEvent),
        (status = 401, description = "No active admin session", body = ErrorBody),
    ),
)]
pub async fn auth_events_list(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<AuthEventsQuery>,
) -> AppResult<Json<PageAuthEvent>> {
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let rows = repo::list_auth_events_filtered(
        &state.db,
        query.event_type.as_deref(),
        query.user_id,
        query.cursor,
        limit + 1,
    )
    .await?;
    let (items, next_cursor) = paginate(rows, limit);
    Ok(Json(PageAuthEvent {
        items: items.into_iter().map(AuthEvent::from).collect(),
        next_cursor,
    }))
}

// ---------------------------------------------------------------- Helpers

trait HasId {
    fn row_id(&self) -> Uuid;
}

impl HasId for ManagedUserRow {
    fn row_id(&self) -> Uuid {
        self.id
    }
}

impl HasId for AuthEventRow {
    fn row_id(&self) -> Uuid {
        self.id
    }
}

fn paginate<T: HasId>(mut rows: Vec<T>, limit: i64) -> (Vec<T>, Option<Uuid>) {
    if rows.len() as i64 > limit {
        let extra = rows.pop();
        (rows, extra.map(|r| r.row_id()))
    } else {
        (rows, None)
    }
}

fn read_cookie(cookie_header: &str, name: &str) -> Option<String> {
    cookie_header.split(';').find_map(|part| {
        let (k, v) = part.trim().split_once('=')?;
        (k == name).then(|| v.to_string())
    })
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

fn parse_session_token(token: &str) -> Result<(Uuid, [u8; 32]), AppError> {
    let (id_part, secret_part) = token.split_once('.').ok_or(AppError::InvalidToken)?;
    let id_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(id_part)
        .map_err(|_| AppError::InvalidToken)?;
    if id_bytes.len() != 16 {
        return Err(AppError::InvalidToken);
    }
    let mut id_arr = [0u8; 16];
    id_arr.copy_from_slice(&id_bytes);
    let secret_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(secret_part)
        .map_err(|_| AppError::InvalidToken)?;
    if secret_bytes.len() != 32 {
        return Err(AppError::InvalidToken);
    }
    let mut secret = [0u8; 32];
    secret.copy_from_slice(&secret_bytes);
    Ok((Uuid::from_bytes(id_arr), secret))
}

fn sha256(input: &[u8]) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(input);
    h.finalize().to_vec()
}

fn session_cookie(state: &AppState, value: &str, max_age_seconds: Option<i64>) -> String {
    // Path=/admin so the cookie is sent both to /admin/api/* and (if it ever
    // returns) to a server-rendered fallback under /admin/*.
    let mut c = format!("{ADMIN_SESSION_COOKIE}={value}; HttpOnly; SameSite=Strict; Path=/admin");
    if matches!(state.config.env, AppEnv::Prod) {
        c.push_str("; Secure");
    }
    if let Some(max_age) = max_age_seconds {
        c.push_str(&format!("; Max-Age={max_age}"));
    }
    c
}

fn csrf_cookie(state: &AppState, value: &str, max_age_seconds: Option<i64>) -> String {
    // NOT HttpOnly — the SPA reads it from document.cookie and echoes it in the
    // X-CSRF-Token header on state-changing requests (double-submit pattern).
    let mut c = format!("{ADMIN_CSRF_COOKIE}={value}; SameSite=Strict; Path=/admin");
    if matches!(state.config.env, AppEnv::Prod) {
        c.push_str("; Secure");
    }
    if let Some(max_age) = max_age_seconds {
        c.push_str(&format!("; Max-Age={max_age}"));
    }
    c
}
