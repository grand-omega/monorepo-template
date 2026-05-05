# monorepo-template

Monorepo template for a Rust backend, Kotlin Multiplatform Android client, and backend-shipped React admin console.

## Layout

```text
backend/                 Rust API server, migrations, Dockerfile, production docs
apps/android/            Kotlin Multiplatform Android client
tools/admin-webui/       React/Vite admin console for /admin/*
contracts/               Shared API contracts and behavior notes
docs/                    Product-level architecture and workflow docs
ops/                     Deployment and runtime glue
.github/workflows/       Path-filtered CI/CD workflows
.claude/agents/          Project-level agent instructions
```

## Local Development

The root `justfile` is the main entrypoint:

```sh
just --list
just dev
just backend-test
just admin-dev
just android-test
```

The imported subprojects keep their own README files for stack-specific details.

## Artifact Ownership

- The backend image is built from `backend/`.
- The admin UI lives in `tools/admin-webui/` and is intended to be built into the backend image in a follow-up.
- The Android app is built from `apps/android/` and released separately.
