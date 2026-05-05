# monorepo-template

Monorepo template for a Rust backend, Kotlin Multiplatform Android client, and backend-shipped React admin console.

## Layout

```text
backend/                 Rust API server, migrations, Dockerfile, production docs
apps/admin/              React/Vite admin console for /admin/*
apps/mobile/             Kotlin Multiplatform mobile client
contracts/               Shared API contracts and behavior notes
docs/                    Product-level architecture and workflow docs
ops/                     Deployment and runtime glue
.github/workflows/       Path-filtered CI/CD workflows
.claude/agents/          Project-level agent instructions
```

## Architecture

![Monorepo architecture diagram](docs/images/architecture-diagram.svg)

```mermaid
flowchart TD
  Mobile[apps/mobile<br/>Kotlin Multiplatform client] -->|/v1/*| Backend[backend<br/>Rust API server]
  AdminDev[apps/admin<br/>React admin SPA] -->|/admin/api/*| Backend
  AdminBuild[admin dist<br/>built into backend image] -->|served at /admin/*| Backend

  Backend -->|publishes| OpenAPI[/openapi.json/]
  OpenAPI -->|snapshot/codegen| AdminDev
  Contracts[contracts/<br/>shared API notes] -.-> OpenAPI

  Backend -->|SQL| Postgres[(Postgres)]
  Backend -->|rate limits, sessions, jobs| Redis[(Redis)]
  Backend -->|dev SMTP| MailHog[MailHog]

  subgraph Compose[docker-compose local stack]
    Backend
    Postgres
    Redis
    MailHog
  end
```

## Local Development

The root `justfile` is the main entrypoint:

```sh
just --list
just dev
just backend-test
just admin-dev
just mobile-test
```

The imported subprojects keep their own README files for stack-specific details.

## Artifact Ownership

- The backend image is built with `backend/Dockerfile` from the repository root context so it can include the admin build.
- The admin UI lives in `apps/admin/` and is built into the backend image for production.
- The mobile app is built from `apps/mobile/` and released separately.
