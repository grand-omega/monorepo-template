# Architecture

This monorepo contains one product with three surfaces:

- `backend/`: Rust API server, migrations, OpenAPI, admin API, and the production Dockerfile.
- `apps/admin/`: React admin console for `/admin/*`.
- `apps/mobile/`: Kotlin Multiplatform mobile client.

The backend owns the public `/v1/*` API, `/admin/api/*`, and `/openapi.json`.
The admin UI builds from the backend OpenAPI snapshot and is copied into the
backend image for production. The mobile app is released separately.
