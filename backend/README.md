# Backend

A production-leaning Rust authentication server template (Axum 0.8): registration,
login, JWT (Ed25519) access tokens, refresh-token rotation with reuse detection,
email verification, password reset, account management, admin JSON API, and
admin WebAuthn/passkeys. Backed by PostgreSQL (sqlx), Redis, structured
tracing, and OpenAPI / Swagger UI.

> Status: foundation. Auth and identity are done; product-specific endpoints are not. Read this as a starter kit, not a finished service.

## Quickstart

```sh
just keys   # generate Ed25519 dev keys -> .env.local
just dev    # docker compose up: postgres, redis, mailhog, app
```

The API listens on `:8080`, metrics on `:9090`, MailHog UI on `:8025`, Swagger UI at `http://localhost:8080/docs`.

For a real phone on the same Wi-Fi, use your workstation LAN IP instead of
`localhost`, for example `http://192.168.1.15:8080`. Android emulators usually
use `http://10.0.2.2:8080`.

## What This Provides

This repo provides the backend auth/account API and admin API. It does not
include the normal end-user web/mobile screens. A client app still needs to
build the register, login, verify-email, forgot-password, reset-password,
profile, and account-settings screens against the routes below.

## Routes

| Method | Path | Notes |
| --- | --- | --- |
| POST | `/v1/auth/register` | always returns 202, even on duplicate email |
| POST | `/v1/auth/login` | returns access + refresh tokens |
| POST | `/v1/auth/refresh` | rotates the refresh token; reuse revokes the family |
| POST | `/v1/auth/logout` | revoke a single refresh token |
| POST | `/v1/auth/logout-all` | revoke every refresh token for the caller (bearer required) |
| POST | `/v1/auth/verify-email` | consume an email-verification token |
| POST | `/v1/auth/resend-verification` | rate-limited 3/hour per IP |
| POST | `/v1/auth/password-reset/request` | rate-limited 3/hour per IP |
| POST | `/v1/auth/password-reset/confirm` | also revokes all refresh tokens |
| GET | `/v1/me` | bearer required |
| PATCH | `/v1/me` | update display name |
| DELETE | `/v1/me` | soft-delete; password required |
| PATCH | `/v1/me/password` | changes password, revokes existing refresh sessions, returns a new token pair |
| POST | `/admin/api/login` | admin login for users with `role = 'admin'`; sets admin cookies |
| GET | `/admin/api/me` | current admin session |
| GET | `/admin/api/users` | management user list |
| GET | `/admin/api/users/{id}/sessions` | list a user's refresh-session families |
| DELETE | `/admin/api/users/{id}/sessions` | revoke all refresh sessions for a user |
| GET | `/admin/api/auth-events` | recent auth/security events |
| POST | `/admin/api/webauthn/register/begin` | begin admin passkey registration |
| POST | `/admin/api/webauthn/register/finish` | finish admin passkey registration |
| GET | `/admin/api/webauthn/credentials` | list current admin's passkeys |
| DELETE | `/admin/api/webauthn/credentials/{id}` | remove one admin passkey |
| POST | `/admin/api/webauthn/login/finish` | finish passkey challenge after password login |
| GET | `/healthz` | liveness |
| GET | `/readyz` | readiness — pings DB and Redis with a 500 ms budget |
| GET | `/metrics` | served on the metrics port (default `:9090`) |

### curl smoke test

```sh
curl -s -X POST http://localhost:8080/v1/auth/register \
  -H 'content-type: application/json' \
  -d '{"email":"a@example.test","password":"correct horse battery staple"}'

curl -s -X POST http://localhost:8080/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"a@example.test","password":"correct horse battery staple"}' | jq
```

Verification emails land in MailHog at `http://localhost:8025`.

In dev, verification/reset links are generated from `APP_PUBLIC_BASE_URL`, but
there is no user-facing `/verify` or `/reset-password` page in this repository.
Client apps should extract the `token` fragment parameter from the email link and
call `/v1/auth/verify-email` or `/v1/auth/password-reset/confirm`.

### Management access

The management surface is a JSON API mounted under `/admin/api/*`. It uses a
separate `HttpOnly` admin session cookie, a double-submit CSRF cookie/header for
state-changing requests, and only allows users with `role = 'admin'`.

Admin login supports WebAuthn passkeys. A fresh deployment can bootstrap the
first admin with password-only login while that admin has no registered
passkeys. After an admin registers a passkey, future admin logins for accounts
with passkeys return a `webauthn_required` challenge after the password step and
only issue admin cookies after `/admin/api/webauthn/login/finish` succeeds.
Each admin should register at least two passkeys to reduce lockout risk.

To promote your own account in development:

```sql
UPDATE users SET role = 'admin' WHERE email = 'you@example.test'::citext;
```

The API includes user search/inspection, lock/unlock, manual email verification,
refresh-session revocation, and recent auth-event inspection.

### First-time admin setup

1. Register a normal user account through `/v1/auth/register` and verify the
   email address.
2. Promote that account directly in the database:

   ```sql
   UPDATE users SET role = 'admin' WHERE email = 'you@example.test'::citext;
   ```

3. Sign in to the admin UI with that email and password. Because the admin has
   zero registered passkeys, the first login succeeds with password only.
4. Open the Passkeys page and register a passkey. Use Proton Pass, the platform
   keychain, a phone, or a hardware security key.
5. Register a second passkey before relying on the account for production
   operations.

After at least one passkey is registered, admin login changes to a two-step
flow: `/admin/api/login` validates the password and returns
`kind: "webauthn_required"` with a challenge, then the browser signs that
challenge with the passkey and posts it to
`/admin/api/webauthn/login/finish`. Admin session cookies are issued only after
the passkey step succeeds.

If an admin loses every registered passkey, recovery is an operator-controlled
database/admin procedure after identity verification; there is no self-service
lost-passkey recovery flow yet.

## Configuration

All settings come from `APP_*` environment variables. Copy `.env.example` to `.env.local` and fill in the gaps. Highlights:

- `APP_ENV` — `dev` | `test` | `prod`. In `prod`, weak Argon2 parameters and missing JWT keys fail startup.
- `APP_PUBLIC_BASE_URL` — used to build verification / reset links sent in email. Auth tokens are placed in URL fragments.
- `APP_TRUSTED_PROXY_CIDRS` — comma-separated CIDRs allowed to set `X-Forwarded-For`. Leave empty if the app is internet-facing.
- `APP_ALLOW_NOOP_MAILER` — set to `true` only in dev/test to allow startup if SMTP init fails. Otherwise startup is fatal, so verification emails are not silently dropped.
- `APP_ARGON2_*` — defaults match OWASP minimums (`m=19456 t=2 p=1`); lower values are rejected in `prod`.
- `APP_WEBAUTHN_RP_ID` / `APP_WEBAUTHN_RP_ORIGIN` — required for admin passkey registration and login. The origin must exactly match the admin SPA origin.

See `.env.example` for the full list with comments.
Use `.env.production.example` as the production configuration inventory.

## Environments

The app distinguishes environments with `APP_ENV=dev | test | prod`.

Development uses `docker-compose.yml`, MailHog, local Postgres/Redis, HTTP, and
`APP_MIGRATE_ON_START=true`.

Production should use `.env.production.example` as an inventory, store real
values in a secret manager, run migrations as a controlled one-shot job, and put
the app behind TLS termination such as Caddy, a load balancer, or a platform
ingress. In `prod`, startup validation rejects unsafe settings such as weak
Argon2 parameters and `APP_ALLOW_NOOP_MAILER=true`.

## Development

```sh
just fmt              # rustfmt
just clippy           # cargo clippy --all-targets -- -D warnings
just test             # unit tests only (no Docker required)
just test-integration # integration tests (spins up Postgres + Redis via testcontainers)
just test-all         # both
just check            # fmt + clippy + unit tests
```

GitHub Actions runs formatting, clippy, unit tests, and Docker-backed
integration tests on pushes and pull requests.

Migrations live in `migrations/` and run on startup when `APP_MIGRATE_ON_START=true`. To run them out of band:

```sh
cargo run -- migrate
```

For production deployment, see `docs/production.md`. For launch security review
items that depend on the actual infrastructure, see `docs/security-review.md`.
For a Caddy reverse-proxy example, see `deploy/Caddyfile.example`.

## Architecture

```
src/
  main.rs            entry; runtime, signal handling, cleanup task
  router.rs          middleware stack: catch-panic, request-id, trace, CORS, compression, timeout
  state.rs           AppState: config, DB pool, Redis, JWT keys, mailer
  config/            env-driven config + validation
  auth/              register, login, refresh rotation, verify, password reset
  users/             /me endpoints
  email/             SMTP mailer + Tera templates + DB-backed outbox/retry
  middleware/        bearer auth, Redis Lua rate limiter, request-id, security headers, trusted-proxy IP
  health/            /healthz and /readyz
  telemetry/         tracing-subscriber setup
  db/                sqlx pool + migration runner
  error.rs           AppError → HTTP status mapping with field-level validation errors
  openapi.rs         utoipa schema for Swagger UI
```

### Notable design choices

- **Refresh tokens** use family tracking with `SELECT ... FOR UPDATE`. Reuse detection revokes the entire family. Hashes are stored, raw tokens never persisted.
- **Argon2id** with configurable cost; OWASP minimums enforced in `prod`. Unknown emails on login are still verified against a dummy hash to equalize timing.
- **Rate limiting** is a Redis Lua token-bucket with four classes (`VeryStrict` 3/hr, `Strict` 10/min, `Medium` 60/min, `Low` 30/min). Trusted-proxy IP extraction validates against a CIDR allowlist.
- **JWTs** are signed with Ed25519 (`jsonwebtoken` + `ed25519-dalek`) and support verify-only previous public keys for zero-downtime key rotation.
- **Access-token revocation** stores a per-user revocation epoch in Redis for logout-all, password reset/change, account deletion, and admin session revocation.
- **Admin passkeys** use WebAuthn as a second login step after a password for admins with registered credentials.
- **Errors** are a single `AppError` enum mapped to HTTP status codes; validation errors include per-field detail.
- **Health checks** are split: `/healthz` is liveness, `/readyz` actually pings DB and Redis with a 500 ms budget.
- **Email delivery** is persisted to `email_outbox`, attempted immediately, and retried by a background worker.

## License

Unspecified — add one before publishing.
