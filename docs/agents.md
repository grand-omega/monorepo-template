# Agent Roster

This repo uses project-level agent instructions under `.claude/agents/`.

The agents are intentionally practical, not ceremonial. Each one should protect
founder time, prefer the smallest useful change, and report the cheapest
meaningful verification command. Use them to isolate context and judgment, not
to simulate a big-company process.

## Product Engineering

- `founder-orchestrator`: coordinates a product-scoped feature through specialist agents until it is ready to ship.
- `backend-rust-engineer`: Rust API, migrations, backend tests, auth/session/email/rate-limit behavior.
- `admin-web-engineer`: React/Vite admin UI behavior, admin API integration, route guards, WebAuthn browser flow, Playwright validation.
- `mobile-app-engineer`: KMP mobile behavior, navigation, secure client storage, deep links, ADB/device validation.
- `client-app-engineer`: cross-client coordination when admin web and mobile must stay aligned.

The product engineer role is intentionally split by real failure mode. Rust/backend, admin web, and mobile have different tools and validation needs. When a feature touches both admin web and mobile, use separate admin and mobile implementation phases, with `client-app-engineer` only for cross-client consistency.

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

- Product-scoped feature that needs multiple surfaces: start with `founder-orchestrator` after `product-manager` has cut scope.
- Backend code change: start with `backend-rust-engineer`; use `security-reviewer` for auth/session/admin/token changes.
- Admin web behavior/API/state change: start with `admin-web-engineer`.
- Mobile behavior/API/state change: start with `mobile-app-engineer`.
- Admin and mobile both changed: use separate `admin-web-engineer` and `mobile-app-engineer` phases; use `client-app-engineer` only for shared contract/consistency planning.
- UX, layout, copy, accessibility, or visual-state change: use `frontend-designer`.
- CI, Docker, deploy, release: use `devops-release-engineer`.
- Test gap or release confidence question: use `qa-test-strategist`.
- Repo communication or GitHub process: use `github-maintainer`.
- Ambiguous product idea: use `product-manager` first.

## Codex Usage

These files are Claude Code agent definitions, but Codex can still use them as
repo-local role instructions. In Codex, explicitly name the agent and ask it to
read the matching file before acting:

```text
Use founder-orchestrator. Read .claude/agents/founder-orchestrator.md, then
drive this PM plan end-to-end on the current branch.
```

For complex work, ask for a plan-first handoff:

```text
Use founder-orchestrator. Have each specialist produce a short plan before
implementation, resolve conflicts, ask me for approval, then execute phase by
phase.
```

## Practical Gates

- Backend local gate: `just backend-check`
- Admin local gate: `just admin-check`; use Playwright for browser workflow changes.
- Mobile unit gate: `just mobile-test`
- Mobile device gate: use `adb devices` and ADB validation when a device is available; check Samsung Z Fold7/foldable behavior when relevant and possible.
- Cross-project gate: `just check`
