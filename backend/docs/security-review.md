# Production Security Review Checklist

Complete this against the actual infrastructure before launch. The application
cannot verify these controls from the repository alone.

## Edge And Transport

- TLS terminates at a trusted ingress; HTTP is redirected to HTTPS.
- The app container or process port is not directly exposed to the internet.
- Only ingress/load-balancer CIDRs are listed in `APP_TRUSTED_PROXY_CIDRS`.
- `X-Forwarded-For` is stripped or overwritten by the trusted ingress.
- `/metrics` is not internet-accessible.
- Admin API access is restricted by network, origin, or an equivalent control.

## Browser And API Boundaries

- `APP_CORS_ALLOWED_ORIGINS` contains exact production origins only.
- `APP_PUBLIC_BASE_URL` is HTTPS and points at the public API origin.
- Admin cookies are `Secure`, `HttpOnly` where appropriate, and `SameSite=Strict`.
- State-changing admin API calls include `X-CSRF-Token`.
- WebAuthn `APP_WEBAUTHN_RP_ORIGIN` exactly matches the admin SPA origin.
- WebAuthn `APP_WEBAUTHN_RP_ID` is the intended admin host or parent domain.
- OpenAPI exposure is approved for the deployment environment.

## Secrets

- JWT private key, database password, Redis password, and SMTP credentials are
  stored in the platform secret manager.
- Secret values are not present in images, logs, shell history, or repo files.
- There is an owner and documented process for JWT key rotation.
- Previous JWT public keys are removed from `APP_JWT_PREVIOUS_PUBLIC_KEYS` after
  the access-token rotation window expires.

## Auth Policy

- Access-token TTL is accepted by the risk owner.
- Redis-backed per-user revocation is accepted as the logout/password-reset
  access-token invalidation mechanism, including its fail-open behavior during
  Redis outages.
- Admin users have a separate account review process before access is granted.
- Each admin has at least two registered passkeys, or there is a documented
  exception and recovery path.
- Lost-passkey recovery requires operator identity verification and a documented
  database/admin procedure; it is not self-service.
- Password reset and verification email sender domains have SPF, DKIM, and DMARC.

## Data And Audit

- `auth_events` retention is defined.
- `email_outbox` retention and operational cleanup are defined.
- Database backups are encrypted and restore-tested.
- Logs do not include raw tokens, passwords, or email body contents.

## Abuse And Availability

- Redis outage behavior is accepted. Rate limiting currently fails open.
- Edge/WAF/load-balancer rate limits are configured for auth-sensitive routes,
  especially when Redis rate limiting is unavailable.
- Alerts exist for login failure spikes, account lockouts, refresh token reuse,
  Redis rate-limit/revocation errors, passkey registration/removal, WebAuthn
  failures, and email outbox failures.
- Load tests cover login, refresh, password reset request, admin listing, and
  the admin WebAuthn challenge/finish flow.
