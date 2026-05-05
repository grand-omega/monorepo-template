# Agent Roster

This repo uses project-level agent instructions under `.claude/agents/`.

## Product Engineering

- `backend-rust-engineer`: Rust API, migrations, backend tests, auth/session/email/rate-limit behavior.
- `client-app-engineer`: mobile KMP app and React admin UI behavior, API integration, navigation, secure client storage, WebAuthn browser flow.

The product engineer role is intentionally split between backend and client work. Rust/backend and KMP/React client work have different failure modes, tools, and review needs. Mobile and admin are kept together for now because both are client applications that consume the same backend contracts; split them later only if either surface becomes large enough to need a dedicated owner.

## Review And Release

- `security-reviewer`: application security and production readiness review.
- `dependency-reviewer`: dependency, toolchain, and vulnerability freshness review.
- `devops-release-engineer`: CI/CD, Docker, deployment, runbooks, release gates, observability.
- `qa-test-strategist`: regression strategy, smoke tests, edge cases, flaky test diagnosis.

## Product And Repository Operations

- `frontend-designer`: UX polish, visual hierarchy, accessibility, copy, loading/error/empty states.
- `github-maintainer`: README/docs, issue/PR templates, labels, changelogs, releases, repo hygiene.
- `product-manager`: roadmap, prioritization, scope control, acceptance criteria, release planning.

## Default Routing

- Backend code change: start with `backend-rust-engineer`; use `security-reviewer` for auth/session/admin/token changes.
- Client behavior change: start with `client-app-engineer`; use `frontend-designer` for look-and-feel work.
- CI, Docker, deploy, release: use `devops-release-engineer`.
- Test gap or release confidence question: use `qa-test-strategist`.
- Repo communication or GitHub process: use `github-maintainer`.
- Ambiguous product idea: use `product-manager` first.
