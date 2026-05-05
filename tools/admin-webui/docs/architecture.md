# Architecture decisions log

Append entries when you make a non-obvious call. Newest on top. Keep entries short — link to PRs/commits for the long version.

## 2026-05-05 — Bun, passkeys, and local e2e

- **Bun is the only project package manager.** `package.json` pins `bun@1.3.13`, the root lockfile is `bun.lock`, and first-party scripts/docs/CI avoid npm/npx/yarn/pnpm commands.
- **Playwright uses the installed Chrome channel.** Playwright 1.59.1 cannot install its bundled Chromium on this Ubuntu 26.04 host, so local e2e runs against system Chrome. CI should either provide Chrome or switch this decision when the runner image changes.
- **Backend pairing is `../rust-server-template` in this workspace.** The admin UI proxies `/admin/api/*` to `VITE_API_URL`, defaulting to `http://localhost:8080`; run the backend with `just keys` then `just dev` before using protected routes locally.
- **Admin passkeys are now in scope.** The backend exposes WebAuthn admin endpoints, and the SPA has a Passkeys route plus login-step handling for `webauthn_required`.
- **`/me` query rejects empty proxy responses explicitly.** When the backend is down, Vite may return a non-JSON proxy failure; `meQueryOptions` now throws a request/session error instead of letting React Query fail with `["me"] data is undefined`.

## 2026-05-04 — Step 3: API client

- **Cookie names track the backend, not plan §1 wording.** Backend sets `admin_session` (HttpOnly) and `admin_csrf` (readable) per its actual implementation. The double-submit pattern reads `admin_csrf` and echoes it in `X-CSRF-Token` on POST/PATCH/PUT/DELETE.
- **`createApi(baseUrl?)` factory in addition to the default `api` singleton.** openapi-fetch captures `globalThis.fetch` at create time, so middleware tests need to install the spy first and then construct a fresh client. The factory also lets tests use an absolute baseUrl since jsdom + undici don't resolve relative URLs the way browsers do.
- **No redirect when already on `/admin/login`.** A 401 on the login attempt itself shouldn't trigger a full reload back to the same page. Cheap guard inside `authMiddleware`.
- **`vi.spyOn(globalThis, "fetch")` for middleware tests, MSW reserved for component tests.** Plan §10 already separates these. Direct fetch spying gives precise control over request/response shapes without an extra moving part. MSW stays installed for the component tests added in later steps.

## 2026-05-04 — Step 2: codegen pipeline

- **Backend `/admin/api/*` JSON routes already exist.** Plan §1 prerequisites are largely landed: `/login`, `/logout`, `/me`, `/users[?q,limit,cursor]`, `/users/{id}`, `/users/{id}/{lock,unlock,verify-email}`, `/auth-events`, and `DELETE /users/{id}/sessions` all return JSON. Snapshot is now the live spec, not a hand-drafted stub. Status of the rest of §1 (CSRF middleware behavior, audit-log writes, admin login lockout, timing-leak fix, prod `/openapi.json` gate) needs verification before step 3 ships.
- **Endpoint shape follows the live spec, not the original draft.** Backend uses `DELETE /users/{id}/sessions` for revoke-all and now includes the refresh-session list endpoint plus WebAuthn admin routes. Frontend codegen follows the snapshot pulled from the live spec.
- **Path-prefix stripping in pull, not in codegen.** Backend emits absolute paths (`/admin/api/me`); SPA client sets `baseUrl: "/admin/api"` and writes `api.GET("/me")`. `scripts/openapi-pull.mjs` strips the prefix in-place and drops non-`/admin/api` paths, so the snapshot is exactly what `paths` should contain. Cheap, reversible, and keeps the backend free to emit a single global spec.
- **Drift check is opt-in via a repo variable.** CI's `openapi-drift` job runs `bun run openapi:drift` only when `DEPLOYED_API_URL` is set; otherwise it warns and passes. Avoids breaking PR builds before any staging backend exists.
- **Canonical-key JSON deep-equal for drift, not raw text diff.** utoipa may reorder keys without changing semantics; we compare canonicalized (sorted-key) serializations and report path-level adds/removes/changes for human-readable failure output.

## 2026-05-04 — Step 1: scaffolding

- **Vite `base: "/admin/"`**. The backend serves the SPA from `/admin/*`, so all asset URLs need that prefix at build time. Dev server runs at `http://localhost:5173/admin/`.
- **Project references in TS** (`tsconfig.app.json` + `tsconfig.node.json`). Keeps DOM-typed app code separate from Node-typed config files; `verbatimModuleSyntax` is strict in both.
- **Manual chunk split** for `react`, `router`, `query` so the main chunk stays under the 250 KB gzipped budget as features land. Revisit after step 8.
- **Snapshot-driven codegen**. CI runs `openapi:gen` against `openapi.snapshot.json` (committed). Live `/openapi.json` and the generated `src/api/schema.ts` are gitignored. See README "Codegen workflow."
- **Deploy mode A** (per plan §11): backend's Docker build clones this repo at a pinned tag. Default until proven painful.
