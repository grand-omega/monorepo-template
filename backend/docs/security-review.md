# Production Security Review Checklist

Complete this against the actual infrastructure before launch. The application
cannot verify these controls from the repository alone.

## Edge And Transport

- TLS terminates at a trusted ingress; HTTP is redirected to HTTPS.
- Only ingress/load-balancer CIDRs are listed in `APP_TRUSTED_PROXY_CIDRS`.
- `X-Forwarded-For` is stripped or overwritten by the trusted ingress.
- `/metrics` is not internet-accessible.
- Admin API access is restricted by network, origin, or an equivalent control.

## Browser And API Boundaries

- `APP_CORS_ALLOWED_ORIGINS` contains exact production origins only.
- Admin cookies are `Secure`, `HttpOnly` where appropriate, and `SameSite=Strict`.
- State-changing admin API calls include `X-CSRF-Token`.
- OpenAPI exposure is approved for the deployment environment.

## Secrets

- JWT private key, database password, Redis password, and SMTP credentials are
  stored in the platform secret manager.
- Secret values are not present in images, logs, shell history, or repo files.
- There is an owner and documented process for JWT key rotation.

## Auth Policy

- Access-token TTL is accepted by the risk owner.
- The residual risk that old access tokens remain valid until expiry after
  logout/password reset is documented.
- Admin users have a separate account review process. Add MFA before exposing
  admin API to broad networks.
- Password reset and verification email sender domains have SPF, DKIM, and DMARC.

## Data And Audit

- `auth_events` retention is defined.
- `email_outbox` retention and operational cleanup are defined.
- Database backups are encrypted and restore-tested.
- Logs do not include raw tokens, passwords, or email body contents.

## Abuse And Availability

- Redis outage behavior is accepted. Rate limiting currently fails open.
- Alerts exist for login failure spikes, account lockouts, refresh token reuse,
  Redis rate-limit errors, and email outbox failures.
- Load tests cover login, refresh, password reset request, and admin listing.
