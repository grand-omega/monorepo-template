# lab-rust-server-admin — implementation plan

Audience: an autonomous coding agent. Read this top to bottom before writing any code. Do not deviate without surfacing the deviation in the PR description.

## 0. Context

`lab-rust-server-admin` is the React SPA management interface for the Rust backend at `../lab-rust-server`. The backend already ships:

- A server-rendered HTML admin at `/admin/*` (to be replaced by this SPA — keep the URL space).
- An `admin_sessions` table with opaque session tokens (`id.secret` format, SHA-256 hashed at rest), HttpOnly cookie auth, role checks at session validation.
- An OpenAPI spec served at `/openapi.json` (utoipa-generated). Currently dev-only — the backend agent will expose it in prod too.
- A token-bucket Redis rate limiter, structured tracing, Prometheus metrics, and an `auth_events` audit table.

This SPA lives in its own git repo, deployed as part of the backend's Docker image (multi-stage build copies `dist/` into the Rust container, which serves it from `/admin/*` via `tower-http::services::ServeDir`). One binary, one deploy, no CORS.

## 1. Coordination with the backend repo

Before writing any frontend code, **post a checklist** to the backend repo (open an issue or PR description listing these prerequisites). Do not start frontend work until the backend confirms these:

1. **CSRF middleware** on all `/admin/api/*` state-changing endpoints. Pattern: double-submit cookie. On admin login, backend sets a non-HttpOnly `csrf_token` cookie alongside the HttpOnly session cookie; frontend echoes it in `X-CSRF-Token` header on every POST/PATCH/DELETE; backend compares header against `admin_sessions.csrf_hash` (new column). Reject mismatches with HTTP 403, code `csrf_invalid`.
2. **Admin audit-log writes**. Every state-changing admin handler must call `events::record(...)` with `EventCtx { user_id: Some(target_user), detail: Some(json!({"by_admin": admin.user_id})), ip, user_agent }`. New `EventKind` variants: `AdminUserLocked`, `AdminUserUnlocked`, `AdminEmailVerified`, `AdminSessionsRevoked`, `AdminLogin`.
3. **Per-admin login lockout**. Mirror `record_failed_login` for admin failures. Currently admin login bypasses the lockout flow entirely.
4. **Timing leak fix on unknown admin email**. Run a dummy Argon2 verify when `find_admin_by_email` returns `None`, like `src/auth/service.rs:179` does for the regular login.
5. **Convert `src/admin/routes.rs` from HTML to JSON.** Routes move under `/admin/api/*` (see §3 for the full surface). HTML page templates in `src/admin/routes.rs` get deleted. The `AdminUser` extractor stays.
6. **Expose `/openapi.json` in prod.** Currently gated to `cfg.env.is_dev()` in `src/router.rs`. Remove the gate. The schema is not sensitive.
7. **Rate-limit `/admin/api/login`** with `Class::Strict` (already done in HTML version — preserve it).

When the above land, the frontend can codegen against the spec and proceed.

## 2. Stack

Locked. Do not substitute without justification.

```
Vite 5+
React 19
TypeScript (strict: true, noUncheckedIndexedAccess: true)
TanStack Query v5     server-state cache
TanStack Router       file-based routing (preferred over React Router for type-safety)
shadcn/ui             components (copy-paste source, owned in repo)
Tailwind CSS v4
Zod                   runtime validation at API boundary
React Hook Form       forms, with @hookform/resolvers/zod
openapi-fetch         typed HTTP client
openapi-typescript    codegen (devDependency)
date-fns              dates (no Moment, no Day.js — date-fns is tree-shakable)
Vitest                unit tests
Playwright            E2E tests
ESLint flat config + typescript-eslint strict
Prettier
```

Skip: Next.js (this is an SPA, not an SSR app), Redux/Zustand for server state (TanStack Query owns it), `react-admin` / Refine (you'll fight them within a month), `axios` (`openapi-fetch` is enough), CSS-in-JS (Tailwind only).

Use Zustand sparingly only if you have genuinely cross-route ephemeral UI state. For now: don't.

## 3. Backend API surface (target — what the SPA calls)

The backend agent will rewrite `src/admin/routes.rs` to expose the following. Use these as the source of truth for the codegen client.

```
POST   /admin/api/login                 — body: { email, password }; sets session + csrf cookies
POST   /admin/api/logout                — revokes current session, clears cookies
GET    /admin/api/me                    — { user_id, email, role: "admin", csrf_token }

GET    /admin/api/users?q=&limit=&cursor=
                                        — { items: ManagedUser[], next_cursor: string | null }
GET    /admin/api/users/{id}            — ManagedUser
POST   /admin/api/users/{id}/lock       — body: { until?: ISO8601, reason?: string }
POST   /admin/api/users/{id}/unlock
POST   /admin/api/users/{id}/verify-email
POST   /admin/api/users/{id}/revoke-sessions

GET    /admin/api/auth-events?event_type=&user_id=&ip=&from=&to=&limit=&cursor=
                                        — { items: AuthEvent[], next_cursor }

GET    /admin/api/users/{id}/sessions   — list of refresh-token families (last_used, ip, ua)

ManagedUser  = { id, email, role, email_verified, display_name, created_at,
                 last_login_at, failed_login_count, locked_until }
AuthEvent    = { id, user_id, user_email, event_type, ip, user_agent,
                 detail (json), created_at }
```

All responses are JSON. All errors follow the existing `ErrorBody` shape: `{ code, message, fields? }`. Status codes match the existing API conventions (401 for missing/invalid session → frontend redirects to `/login`; 403 for CSRF failure → toast and refetch CSRF token; 422 for validation; 423 for locked admin account; 429 for rate-limited; 5xx for backend faults).

## 4. Repository layout

```
lab-rust-server-admin/
  .github/workflows/ci.yml           lint + typecheck + unit + e2e + bundle-budget
  docs/
    plan.md                          this document
    architecture.md                  decisions log; update when you make non-obvious calls
  src/
    api/
      schema.ts                      generated by openapi-typescript; gitignored
      client.ts                      openapi-fetch instance + middleware (auth, CSRF, errors)
      queries.ts                     TanStack Query keys + factory hooks
    auth/
      use-session.ts                 hook: reads /admin/api/me, throws to redirect on 401
      login-page.tsx
      require-admin.tsx              route guard component
    components/
      ui/                            shadcn copy-paste
      data-table.tsx                 generic table w/ sorting + pagination
      confirm-dialog.tsx             two-step confirmation for destructive actions
      copy-button.tsx                for IDs
      time.tsx                       relative + absolute time tooltip
    features/
      users/
        list-page.tsx                table with search + pagination
        detail-page.tsx              user detail + actions sidebar
        actions/
          lock-action.tsx            confirm dialog; useMutation; invalidates ['users', id]
          unlock-action.tsx
          verify-email-action.tsx
          revoke-sessions-action.tsx
        sessions-list.tsx            refresh-token families
      auth-events/
        list-page.tsx                virtualized table w/ filters
        filters.tsx                  event_type, user, ip, time range
        detail-drawer.tsx            full row + JSON detail viewer
    routes/                          TanStack Router file-based; one file per route
      __root.tsx                     layout with header + nav + auth guard
      _authenticated.tsx             requires admin session
      _authenticated/users.tsx
      _authenticated/users.$id.tsx
      _authenticated/auth-events.tsx
      login.tsx
      index.tsx                      redirects to /users
    lib/
      cn.ts                          tailwind-merge helper
      format.ts                      time, ip, role formatting
      errors.ts                      ApiError class + handler
    main.tsx                         entry: QueryClient + Router + Theme
    index.css                        tailwind directives
  tests/
    e2e/
      login.spec.ts
      lock-user.spec.ts
      revoke-sessions.spec.ts
      auth-events.spec.ts
    setup/
      docker-compose.yml             postgres + redis + backend image for E2E
  .env.example
  Dockerfile                         optional: standalone build, if backend pulls a tagged image
  vite.config.ts
  tsconfig.json
  tailwind.config.ts
  package.json
  README.md                          quickstart, codegen workflow, deploy
```

## 5. API client — the single most important file

`src/api/client.ts` is the spine. Get it right once:

```ts
import createClient, { Middleware } from "openapi-fetch";
import type { paths } from "./schema";

const csrfMiddleware: Middleware = {
  async onRequest({ request }) {
    const method = request.method.toUpperCase();
    if (["POST", "PATCH", "PUT", "DELETE"].includes(method)) {
      const token = readCookie("csrf_token");
      if (token) request.headers.set("X-CSRF-Token", token);
    }
    return request;
  },
};

const authMiddleware: Middleware = {
  async onResponse({ response }) {
    if (response.status === 401) {
      // session expired — redirect to login, clear all queries
      window.location.assign("/admin/login");
    }
    return response;
  },
};

export const api = createClient<paths>({ baseUrl: "/admin/api" });
api.use(csrfMiddleware);
api.use(authMiddleware);

function readCookie(name: string): string | null {
  return (
    document.cookie
      .split("; ")
      .find((row) => row.startsWith(`${name}=`))
      ?.split("=")[1] ?? null
  );
}
```

Use this everywhere. No raw `fetch` calls in the codebase except inside `client.ts`. Lint rule (`no-restricted-globals: ["fetch"]`) enforces it.

## 6. Codegen workflow

```bash
# package.json scripts
"openapi:fetch": "curl -fsSL ${API_URL:-http://localhost:8080}/openapi.json -o openapi.json"
"openapi:gen":   "openapi-typescript openapi.json -o src/api/schema.ts"
"openapi":       "bun run openapi:fetch && bun run openapi:gen"
```

`openapi.json` and `src/api/schema.ts` are **gitignored**. CI runs `openapi:gen` against a checked-in `openapi.snapshot.json` (committed when the contract changes intentionally). A drift-check job in CI fetches the live spec from a deployed backend and diffs against the snapshot — fail if they diverge without a corresponding PR.

This is how the polyrepo stays sane. Don't skip the snapshot.

## 7. Auth flow

1. User hits `/admin/*` (any route).
2. `__root.tsx` route guard calls `useQuery({ queryKey: ['me'], queryFn: () => api.GET('/me') })`.
3. On 401, `authMiddleware` redirects to `/admin/login`.
4. Login page submits `{ email, password }` to `/admin/api/login`.
5. Backend sets `admin_session` (HttpOnly) and `csrf_token` (readable) cookies, returns `{ user_id, email, csrf_token }`.
6. Frontend invalidates `['me']`, navigates to `/admin/users`.
7. Logout: `POST /admin/api/logout`, then `queryClient.clear()`, then navigate to login.

Session expires server-side after 12h (current backend setting). Frontend doesn't track expiry — the 401 interceptor handles it.

## 8. State management rules

- **Server state** → TanStack Query. Always. No exceptions.
- **URL state** (filters, pagination, search query) → TanStack Router search params. Bookmarkable, shareable, refresh-safe.
- **Form state** → React Hook Form. Always.
- **Ephemeral UI state** (modal open, hover, focus) → `useState`.
- **Cross-route UI state** → only if you genuinely need it; Zustand if so. Default to passing through routes.

If you find yourself wanting Redux, you've made a mistake somewhere. Re-read this section.

## 9. Critical UX requirements

These are not optional:

1. **Confirm-with-target-displayed** on every destructive action. The dialog must show the user's email or ID. One missed confirm and an admin locks the wrong account.
2. **Optimistic updates with rollback on error** for fast actions (lock, verify, etc.). TanStack Query's `onMutate` / `onError` pattern.
3. **Loading skeletons**, not spinners, for table loads. Skeletons preserve layout and feel faster.
4. **Empty states with calls-to-action**. "No users matching `foo` — clear search" beats a blank table.
5. **Error toasts that show the request ID** from the `x-request-id` response header. Backend already sets it — surface it so support can grep logs.
6. **Disable destructive buttons during mutation** and show a spinner inside the button, not over the page.
7. **Keyboard shortcuts**: `/` to focus search, `g u` to go to users, `g e` to go to auth-events. Use a small library or hand-roll.
8. **Time display**: relative ("3 min ago") with absolute on hover (RFC3339 in user's locale). Use `date-fns` `formatDistanceToNow` and `<Tooltip>`.
9. **No flash of unauthenticated content.** The route guard renders a skeleton, not the protected page, while `['me']` is loading.

## 10. Testing

| Layer | Tool | Coverage target |
|---|---|---|
| Unit | Vitest | Pure functions: formatters, CSRF cookie read, error parsers. ~100% on `lib/`. |
| Component | Vitest + Testing Library | One test per feature page: renders, handles loading, error, empty. |
| E2E | Playwright | Login → list → detail → action → assert audit event. Run against real backend in docker-compose. |

E2E is **mandatory** and runs in CI against a docker-compose stack (postgres + redis + backend image + frontend dev server). The `tests/setup/docker-compose.yml` reuses the backend's compose file with `extends`. Don't fake the backend with MSW for E2E — the whole point is to catch contract drift.

Component-level tests *can* mock the API with MSW for speed, but use the same generated `schema.ts` types so mocks stay in sync.

Playwright must cover, at minimum:

- Login happy path.
- Login wrong password — generic error message visible.
- Demoted admin (revoked role mid-session) — gets 401 on next request, redirected.
- Lock user → confirm dialog shows target email → submit → audit event appears in `/auth-events`.
- Revoke sessions → confirm → user's refresh tokens dead in DB.
- Search users with special chars (no XSS).

## 11. Deployment

Backend's `Dockerfile` becomes multi-stage:

```dockerfile
FROM oven/bun:1.3.13-alpine AS frontend
WORKDIR /app
COPY lab-rust-server-admin/package.json lab-rust-server-admin/bun.lock ./
RUN bun install --frozen-lockfile
COPY lab-rust-server-admin/ ./
RUN bun run build  # outputs dist/

FROM rust:1-bookworm AS backend
# ... existing Rust build ...

FROM debian:bookworm-slim
COPY --from=backend /app/target/release/lab-rust-server /usr/local/bin/
COPY --from=frontend /app/dist /var/lib/lab-rust-server/admin-ui
ENV APP_ADMIN_UI_DIR=/var/lib/lab-rust-server/admin-ui
```

Two ways to wire this up — pick one and document the choice in `architecture.md`:

A. **Submodule-free**: backend repo's CI clones this repo at a pinned tag during the Docker build. Pin via `ARG ADMIN_UI_TAG=v0.3.1` in the Dockerfile. Cross-repo PR coordination required for breaking changes.

B. **Frontend publishes a Docker image** (`ghcr.io/yanwenxu/lab-rust-server-admin:v0.3.1`) containing only `dist/`. Backend `COPY --from=ghcr.io/yanwenxu/lab-rust-server-admin:v0.3.1 /dist /var/lib/...`. Cleanest separation; requires registry plumbing.

Default to (A) — fewer moving parts.

The Rust backend serves `/admin/*` static assets via `tower_http::services::ServeDir::new(env!("APP_ADMIN_UI_DIR"))` with a `not_found_service` that returns `index.html` (SPA fallback). The static service is mounted **after** the JSON `/admin/api/*` routes so API requests don't get swallowed.

## 12. Observability

- **Sentry** (or self-hosted GlitchTip) for frontend errors. Tag releases with the git SHA (`VITE_GIT_SHA` injected at build time).
- **Web Vitals** logged to a `/admin/api/telemetry/web-vitals` endpoint (optional follow-up; backend doesn't have it yet — open an issue).
- **No localStorage**. Ever. Cookies only.
- **Console logs forbidden in prod builds.** ESLint rule `no-console: ["error", { allow: ["warn", "error"] }]`.

## 13. Security checklist (must pass before first deploy)

- [ ] CSP header from backend allows `'self'` for everything; no `unsafe-inline`. Frontend uses Tailwind, not styled-components, so this works without nonces.
- [ ] No third-party scripts (no Google Analytics, no fonts from CDN — bundle Inter or use system font stack).
- [ ] No tokens in localStorage or sessionStorage.
- [ ] Every state-changing request sends `X-CSRF-Token`.
- [ ] Every input that goes into the DOM is rendered through React (auto-escaped) — never `dangerouslySetInnerHTML`.
- [ ] Search inputs trimmed and length-capped client-side (backend will also cap, but defense in depth).
- [ ] `target="_blank"` links include `rel="noopener noreferrer"`.
- [ ] ESLint plugins `eslint-plugin-react`, `eslint-plugin-jsx-a11y`, `eslint-plugin-security` enabled.
- [ ] No `eval`, no `new Function`, no `innerHTML`. Lint-enforced.
- [ ] Dependency audit (`bun audit --audit-level=high`) clean in CI.
- [ ] Subresource integrity if any CDN — but you shouldn't have any.

## 14. Roadmap (deliverable order)

Each numbered step is a shippable PR. Do not combine.

1. **Repo scaffolding**: Vite + TS + Tailwind + shadcn-init + ESLint + Prettier + Vitest + Playwright + CI workflow + README + this plan committed. No app code yet.
2. **Codegen pipeline**: `openapi:fetch`/`openapi:gen` scripts + snapshot + drift-check CI job. Snapshot a stub `openapi.json` if backend isn't ready yet.
3. **API client**: `src/api/client.ts` with both middlewares + tests. Mock backend with MSW for the unit tests.
4. **Routing skeleton**: `__root.tsx`, `_authenticated.tsx` guard, `login.tsx`, `index.tsx`. Login is non-functional but route guard works.
5. **Auth wired**: `useSession`, login form, logout. E2E: login.spec.ts.
6. **Users list**: search, pagination, table, time formatting. E2E: ensure list renders post-login.
7. **User detail + actions**: lock/unlock/verify-email/revoke-sessions, all with confirm dialogs and optimistic updates. E2E: lock-user.spec.ts, revoke-sessions.spec.ts.
8. **Auth events**: virtualized table, filters by event_type/user/ip/time, detail drawer. E2E: auth-events.spec.ts.
9. **User sessions sub-view**: list refresh-token families on user detail. Per-session revoke. (Backend may need an endpoint addition — coordinate.)
10. **Polish pass**: keyboard shortcuts, empty states, error toasts with request IDs, accessibility audit (axe-core in CI).
11. **Deploy**: multi-stage Dockerfile in backend repo, integration test that the served SPA + API both work behind a single port.

## 15. What NOT to build (explicitly out of scope for v1)

- TOTP / WebAuthn for admin login. Backend doesn't support it yet; SPA waits.
- Email-change confirmation flow. Same.
- Internationalization (i18n). One-language only — English. Don't reach for `react-intl`.
- Dark mode. Not now. Single light theme. (You can add it later with `next-themes` + Tailwind v4's `@variant dark`.)
- Real-time updates (websocket / SSE for live audit events). Polling is fine for v1; revisit when traffic justifies it.
- Bulk actions (select multiple users, lock all). Single-target only for v1 — fewer footguns.
- File uploads, image rendering, rich text. None of this exists in the backend; SPA stays simple.
- Service worker / offline support. Pointless for an admin panel.
- Server-side rendering. Pointless for an authed-only SPA.

## 16. Coding conventions

- TypeScript strict + `noUncheckedIndexedAccess`. Yes, it's annoying. It catches real bugs.
- `function` for components, not `const () => {}`. Easier stack traces, hoisting works.
- One component per file. Co-locate small subcomponents only when they're truly never reused.
- Props interfaces named `<ComponentName>Props`. Not `IProps`, not inline.
- `import type { ... }` for type-only imports. Lint-enforced.
- `kebab-case.tsx` for file names.
- No barrel files (`index.ts` re-exports). They wreck tree-shaking and IDE go-to-definition.
- `// TODO(@yourname, YYYY-MM-DD):` for TODOs with owner and date.

## 17. Definition of done (for v1)

A new admin can:

- [ ] Log in.
- [ ] See the list of users with search and pagination.
- [ ] Open a user, see their full state.
- [ ] Lock a user (with confirmation showing the email).
- [ ] Unlock a user.
- [ ] Force-verify a user's email.
- [ ] Revoke a user's sessions.
- [ ] See the auth-events log filtered by event type, user, IP, and time range.
- [ ] Log out.
- [ ] Have every action they took show up in `/auth-events` with their admin identity stamped on it.
- [ ] Get a clear error message with a request ID if anything fails.
- [ ] Use the panel from a keyboard, with screen-reader-friendly labels.

CI must be green. Bundle must be ≤ 250 KB gzipped main chunk. E2E must pass against a real backend. Lighthouse accessibility ≥ 95.

When all of the above are true, tag `v1.0.0` and let the backend repo pin to it.

---

End of plan. Open the first PR with §14 step 1 (scaffolding) and reference this document.
