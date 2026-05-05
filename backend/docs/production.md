# Production Runbook

This service is production-oriented but expects the deploy environment to provide
PostgreSQL, Redis, SMTP, TLS termination, metrics scraping, alerting, backups,
and secret storage.

## Release Gate

Run these before promoting an image:

```sh
just check
just test-integration
```

CI runs the same checks in `.github/workflows/ci.yml`. Integration tests require
Docker because they use real Postgres and Redis through testcontainers.

## Configuration

Use `.env.production.example` as the production config inventory. Keep real
values in the platform secret manager.

Required production decisions:

- `APP_MIGRATE_ON_START=false` for controlled production migrations.
- `APP_DATABASE_URL` should require TLS when the database is off-host.
- `APP_REDIS_URL` should use `rediss://` where the provider supports TLS.
- `APP_CORS_ALLOWED_ORIGINS` should list exact browser origins.
- `APP_TRUSTED_PROXY_CIDRS` must match only the load balancer or ingress CIDRs.
- `APP_METRICS_BIND_ADDR` should be reachable only from the metrics network.
- `APP_ALLOW_NOOP_MAILER=false` is enforced in `prod`.
- `APP_PUBLIC_BASE_URL` must be the public HTTPS API base URL used in
  verification and password-reset emails.
- `APP_WEBAUTHN_RP_ID` should be the bare host of the admin SPA, for example
  `admin.example.com`.
- `APP_WEBAUTHN_RP_ORIGIN` must exactly match the admin SPA origin that calls
  `navigator.credentials.{create,get}`, including scheme and port if present.

## Migrations

Run migrations before starting the new application version:

```sh
cargo run -- migrate
```

For managed deploys, run the compiled binary with `migrate` as a one-shot job
using the same image and production database secret. Back up the database before
schema changes that touch auth/session/token tables.

## TLS And Reverse Proxy

Terminate public HTTPS at a trusted edge such as Caddy, a load balancer, or a
platform ingress. It is normal for the public client to speak HTTPS to the edge
while the edge speaks HTTP to the Rust app on a private network:

```text
client -> HTTPS -> Caddy/load balancer -> HTTP -> app:8080
```

Do not expose the app's `:8080` or metrics `:9090` ports directly to the
internet. See `deploy/Caddyfile.example` for a minimal Caddy reverse-proxy
example.

If a proxy sets `X-Forwarded-For`, configure `APP_TRUSTED_PROXY_CIDRS` to only
that proxy or ingress network. Do not trust arbitrary internet clients to set
forwarded headers.

## Email Delivery

Verification and password-reset email is persisted in `email_outbox` before
delivery. The server tries to send immediately and then retries due rows from a
background worker every 30 seconds.

Operational checks:

- Alert on sustained email delivery failures or a growing failed outbox count.
- Inspect `email_outbox WHERE status IN ('pending', 'failed', 'sending')`.
- A `sending` row older than 10 minutes is requeued automatically.
- SMTP outages no longer lose messages, but users still wait for delivery until
  SMTP recovers.

## Observability

Use the deployment platform's logging, tracing, and health-check tooling.
Recommended dashboards or monitors:

- Request rate, 4xx/5xx rate, and p95/p99 latency by route.
- `/readyz` failures.
- DB pool usage and query error rate.
- Redis rate-limit errors and blocked requests.
- Login failures, account lockouts, refresh reuse detection.
- Email delivery failures and pending outbox age. Query `email_outbox` for
  pending/failed rows if the platform does not collect custom app events.
- Admin passkey registration/removal and WebAuthn login failures.

## Admin Passkeys

Admin accounts can use WebAuthn passkeys for the second login step. The
password step still runs first. If the admin has registered passkeys, login
returns `webauthn_required` with a challenge, and session cookies are issued
only after `/admin/api/webauthn/login/finish` verifies the assertion.

Bootstrap behavior is intentional: a fresh admin with zero registered passkeys
can sign in with password alone so the first passkey can be enrolled. After
enrollment, require each admin to register at least two passkeys, such as a
platform authenticator and a hardware key or phone.

Before production launch:

- Configure `APP_WEBAUTHN_RP_ID`, `APP_WEBAUTHN_RP_ORIGIN`, and optionally
  `APP_WEBAUTHN_RP_NAME`.
- Restrict admin API access by network, origin, VPN, or equivalent control.
- Document recovery for an admin who loses every registered passkey. Current
  recovery is an operator-controlled database change after identity
  verification, not an automated self-service flow.
- Alert or review auth events for passkey registration, removal, and failed
  WebAuthn login attempts.

## JWT Keys

The server signs new access tokens with the active Ed25519 key configured by
`APP_JWT_PRIVATE_KEY`, `APP_JWT_PUBLIC_KEY`, and `APP_JWT_KID`. It can also
verify tokens signed by older public keys listed in `APP_JWT_PREVIOUS_PUBLIC_KEYS`
as a JSON array of `{kid, public_key_pem}` objects.

Zero-downtime key rotation:

1. Deploy the old public key in `APP_JWT_PREVIOUS_PUBLIC_KEYS` while keeping
   the old key active.
2. Deploy the new private/public keypair with a new `APP_JWT_KID`, keeping the
   old public key in `APP_JWT_PREVIOUS_PUBLIC_KEYS`.
3. Wait at least `APP_ACCESS_TOKEN_TTL` plus clock skew after all old-token
   issuers are gone.
4. Remove the old key from `APP_JWT_PREVIOUS_PUBLIC_KEYS`.

The active `APP_JWT_KID` must not collide with any previous key, and duplicate
previous kids fail startup.

## Deployment

Use `/healthz` as liveness and `/readyz` as readiness. `/readyz` checks both DB
and Redis with a 500 ms budget, so remove instances from traffic when it fails.

Use a rolling deployment only after migrations are complete. Keep at least one
old instance available until the new version passes readiness and metrics show
normal auth/login behavior.

## Dev Versus Prod

`docker-compose.yml` is a development stack: it uses MailHog, local credentials,
HTTP, automatic migrations, and publicly published local ports. Do not use it as
the production deployment file without hardening.

Production should supply `APP_*` values from the platform configuration/secret
system, set `APP_ENV=prod`, use JSON logs, run controlled migrations, terminate
TLS at the edge, and restrict metrics/admin access by network or equivalent
policy.
