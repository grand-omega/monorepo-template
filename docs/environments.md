# Environments

Recommended environment tiers:

- `local`: Docker Compose services, MailHog, local database, local admin UI.
- `staging`: deployed non-production backend/admin with seeded safe test accounts.
- `prod`: production secrets, HTTPS, real SMTP, controlled migrations.
