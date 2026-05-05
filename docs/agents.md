# Agent Roster

This repo uses project-level agent instructions under `.claude/agents/`.

The agents are intentionally practical, not ceremonial. Each one should protect
founder time, prefer the smallest useful change, and report the cheapest
meaningful verification command. Use them to isolate context and judgment, not
to simulate a big-company process.

## Product Engineering

- `founder-orchestrator`: coordinates a product-scoped feature through specialist agents until it is ready to ship.
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

- Product-scoped feature that needs multiple surfaces: start with `founder-orchestrator` after `product-manager` has cut scope.
- Backend code change: start with `backend-rust-engineer`; use `security-reviewer` for auth/session/admin/token changes.
- Client behavior/API/state change: start with `client-app-engineer`.
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
implementation, resolve conflicts, then execute phase by phase.
```

## Practical Gates

- Backend local gate: `just backend-check`
- Admin local gate: `just admin-check`
- Mobile unit gate: `just mobile-test`
- Cross-project gate: `just check`
