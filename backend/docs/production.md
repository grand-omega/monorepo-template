# Production Runbook

This service is production-oriented but expects the deploy environment to provide
PostgreSQL, Redis, SMTP, TLS termination, metrics scraping, and secret storage.

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

## Migrations

Run migrations before starting the new application version:

```sh
cargo run -- migrate
```

For managed deploys, run the compiled binary with `migrate` as a one-shot job
using the same image and production database secret. Back up the database before
schema changes that touch auth/session/token tables.

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

## JWT Keys

The current implementation supports one Ed25519 keypair at a time. Rotate with
a planned deploy:

1. Shorten access-token TTL if needed.
2. Deploy the new private/public keypair with a new `APP_JWT_KID`.
3. Keep refresh tokens valid; new access tokens will use the new key.
4. Old access tokens expire naturally within `APP_ACCESS_TOKEN_TTL`.

Multi-key verification is still a future hardening item if zero-overlap access
token rotation is required.

## Deployment

Use `/healthz` as liveness and `/readyz` as readiness. `/readyz` checks both DB
and Redis with a 500 ms budget, so remove instances from traffic when it fails.

Use a rolling deployment only after migrations are complete. Keep at least one
old instance available until the new version passes readiness and metrics show
normal auth/login behavior.
