# Architecture decisions log

Append entries when you make a non-obvious call. Newest on top. Keep entries short — link to PRs/commits for the long version.

## 2026-05-04 — Step 3: API client

- **Cookie names track the backend, not plan §1 wording.** Backend sets `admin_session` (HttpOnly) and `admin_csrf` (readable) per its actual implementation. The double-submit pattern reads `admin_csrf` and echoes it in `X-CSRF-Token` on POST/PATCH/PUT/DELETE.
- **`createApi(baseUrl?)` factory in addition to the default `api` singleton.** openapi-fetch captures `globalThis.fetch` at create time, so middleware tests need to install the spy first and then construct a fresh client. The factory also lets tests use an absolute baseUrl since jsdom + undici don't resolve relative URLs the way browsers do.
- **No redirect when already on `/admin/login`.** A 401 on the login attempt itself shouldn't trigger a full reload back to the same page. Cheap guard inside `authMiddleware`.
- **`vi.spyOn(globalThis, "fetch")` for middleware tests, MSW reserved for component tests.** Plan §10 already separates these. Direct fetch spying gives precise control over request/response shapes without an extra moving part. MSW stays installed for the component tests added in later steps.

## 2026-05-04 — Step 2: codegen pipeline

- **Backend `/admin/api/*` JSON routes already exist.** Plan §1 prerequisites are largely landed: `/login`, `/logout`, `/me`, `/users[?q,limit,cursor]`, `/users/{id}`, `/users/{id}/{lock,unlock,verify-email}`, `/auth-events`, and `DELETE /users/{id}/sessions` all return JSON. Snapshot is now the live spec, not a hand-drafted stub. Status of the rest of §1 (CSRF middleware behavior, audit-log writes, admin login lockout, timing-leak fix, prod `/openapi.json` gate) needs verification before step 3 ships.
- **Endpoint shape differs from plan §3** in two places: backend uses `DELETE /users/{id}/sessions` instead of `POST /users/{id}/revoke-sessions`, and there is no `GET /users/{id}/sessions` (list) yet. Frontend codegen follows the live spec; plan step 9 (sessions sub-view) still needs the GET endpoint added on the backend.
- **Path-prefix stripping in pull, not in codegen.** Backend emits absolute paths (`/admin/api/me`); SPA client sets `baseUrl: "/admin/api"` and writes `api.GET("/me")`. `scripts/openapi-pull.mjs` strips the prefix in-place and drops non-`/admin/api` paths, so the snapshot is exactly what `paths` should contain. Cheap, reversible, and keeps the backend free to emit a single global spec.
- **Drift check is opt-in via a repo variable.** CI's `openapi-drift` job runs `bun run openapi:drift` only when `DEPLOYED_API_URL` is set; otherwise it warns and passes. Avoids breaking PR builds before any staging backend exists.
- **Canonical-key JSON deep-equal for drift, not raw text diff.** utoipa may reorder keys without changing semantics; we compare canonicalized (sorted-key) serializations and report path-level adds/removes/changes for human-readable failure output.

## 2026-05-04 — Step 1: scaffolding

- **Vite `base: "/admin/"`**. The backend serves the SPA from `/admin/*`, so all asset URLs need that prefix at build time. Dev server runs at `http://localhost:5173/admin/`.
- **Project references in TS** (`tsconfig.app.json` + `tsconfig.node.json`). Keeps DOM-typed app code separate from Node-typed config files; `verbatimModuleSyntax` is strict in both.
- **Manual chunk split** for `react`, `router`, `query` so the main chunk stays under the 250 KB gzipped budget as features land. Revisit after step 8.
- **Snapshot-driven codegen**. CI runs `openapi:gen` against `openapi.snapshot.json` (committed). Live `/openapi.json` and the generated `src/api/schema.ts` are gitignored. See README "Codegen workflow."
- **Deploy mode A** (per plan §11): backend's Docker build clones this repo at a pinned tag. Default until proven painful.
