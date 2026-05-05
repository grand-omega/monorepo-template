# lab-rust-server

A production-leaning authentication & user-management API in Rust (Axum 0.8) — registration, login, JWT (Ed25519) access tokens, refresh-token rotation with reuse detection, email verification, password reset, and account management. Backed by PostgreSQL (sqlx) and Redis (rate limiting), with structured tracing and an OpenAPI / Swagger UI.

> Status: foundation. Auth and identity are done; product-specific endpoints are not. Read this as a starter kit, not a finished service.

## Quickstart

```sh
make keys   # generate Ed25519 dev keys → .env.local
make dev    # docker compose up: postgres, redis, mailhog, app
```

The API listens on `:8080`, metrics on `:9090`, MailHog UI on `:8025`, Swagger UI at `http://localhost:8080/docs` (dev only).

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
| GET | `/admin/api/auth-events` | recent auth/security events |
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

### Management access

The management surface is a JSON API mounted under `/admin/api/*`. It uses a
separate `HttpOnly` admin session cookie, a double-submit CSRF cookie/header for
state-changing requests, and only allows users with `role = 'admin'`.

To promote your own account in development:

```sql
UPDATE users SET role = 'admin' WHERE email = 'you@example.test'::citext;
```

The API includes user search/inspection, lock/unlock, manual email verification,
refresh-session revocation, and recent auth-event inspection.

## Configuration

All settings come from `APP_*` environment variables. Copy `.env.example` to `.env.local` and fill in the gaps. Highlights:

- `APP_ENV` — `dev` | `test` | `prod`. In `prod`, weak Argon2 parameters and missing JWT keys fail startup.
- `APP_PUBLIC_BASE_URL` — used to build verification / reset links sent in email.
- `APP_TRUSTED_PROXY_CIDRS` — comma-separated CIDRs allowed to set `X-Forwarded-For`. Leave empty if the app is internet-facing.
- `APP_ALLOW_NOOP_MAILER` — set to `true` only in dev/test to allow startup if SMTP init fails. Otherwise startup is fatal, so verification emails are not silently dropped.
- `APP_ARGON2_*` — defaults match OWASP minimums (`m=19456 t=2 p=1`); lower values are rejected in `prod`.

See `.env.example` for the full list with comments.
Use `.env.production.example` as the production configuration inventory.

## Development

```sh
make fmt              # rustfmt
make clippy           # cargo clippy --all-targets -- -D warnings
make test             # unit tests only (no Docker required)
make test-integration # integration tests (spins up Postgres + Redis via testcontainers)
make test-all         # both
make check            # fmt + clippy + unit tests
```

GitHub Actions runs formatting, clippy, unit tests, and Docker-backed
integration tests on pushes and pull requests.

Migrations live in `migrations/` and run on startup when `APP_MIGRATE_ON_START=true`. To run them out of band:

```sh
cargo run -- migrate
```

For production deployment, see `docs/production.md`. For launch security review
items that depend on the actual infrastructure, see `docs/security-review.md`.

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
- **JWTs** are signed with Ed25519 (`jsonwebtoken` + `ed25519-dalek`).
- **Errors** are a single `AppError` enum mapped to HTTP status codes; validation errors include per-field detail.
- **Health checks** are split: `/healthz` is liveness, `/readyz` actually pings DB and Redis with a 500 ms budget.
- **Email delivery** is persisted to `email_outbox`, attempted immediately, and retried by a background worker.

## License

Unspecified — add one before publishing.
