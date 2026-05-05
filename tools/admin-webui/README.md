# lab-rust-server-admin

React SPA management interface for the Rust backend at [`../lab-rust-server`](../lab-rust-server). Served from `/admin/*` by the backend in production. See [`docs/plan.md`](docs/plan.md) for the full plan; this README is the quickstart.

## Quickstart

```bash
cp .env.example .env.local
bun install
bun run dev
```

Open http://localhost:5173/admin/. The dev server proxies `/admin/api/*` to `VITE_API_URL` (default `http://localhost:8080`).

## Scripts

| Command                           | Purpose                                                                                          |
| --------------------------------- | ------------------------------------------------------------------------------------------------ |
| `bun run dev`                     | Vite dev server with API proxy.                                                                  |
| `bun run build`                   | Type-check, then production build to `dist/`.                                                    |
| `bun run preview`                 | Serve `dist/` locally.                                                                           |
| `bun run lint`                    | ESLint flat config.                                                                              |
| `bun run format` / `format:check` | Prettier.                                                                                        |
| `bun run typecheck`               | `tsc -b --noEmit` across project references.                                                     |
| `bun run test`                    | Vitest (unit + component).                                                                       |
| `bun run test:e2e`                | Playwright. Spins up dev server unless `CI` is set.                                              |
| `bun run openapi:pull`            | Fetch live `/openapi.json` from `API_URL`, strip `/admin/api`, write `openapi.json`.             |
| `bun run openapi:gen`             | Generate `src/api/schema.ts` from the committed `openapi.snapshot.json` (CI default).            |
| `bun run openapi:gen:live`        | Generate `src/api/schema.ts` from a freshly-pulled `openapi.json` instead.                       |
| `bun run openapi`                 | Pull + generate from live, in one step (handy during local dev).                                 |
| `bun run openapi:snapshot`        | Promote the pulled `openapi.json` to `openapi.snapshot.json`. Review the diff before committing. |
| `bun run openapi:drift`           | Pull + diff against the snapshot. Exits 1 on drift. Used in CI.                                  |

## Codegen workflow

The OpenAPI client is generated from the backend's `/openapi.json`. The flow:

1. `openapi.snapshot.json` (committed) is the contract the SPA is built against. CI runs `openapi:gen` against the snapshot so the build is hermetic.
2. To update the contract: `API_URL=http://localhost:8080 bun run openapi:pull`, then either work against `openapi.json` (`bun run openapi:gen:live`) or — if the new shape is the new contract — `bun run openapi:snapshot` to update the committed snapshot. Review and commit the snapshot diff alongside any client changes.
3. The pull script strips the `/admin/api` prefix from path keys so the SPA's `openapi-fetch` client (with `baseUrl: "/admin/api"`) can call e.g. `api.GET("/me")`. Non-admin paths are dropped.
4. CI's `openapi-drift` job runs on PRs when the `DEPLOYED_API_URL` repo variable is set: it fetches the live spec from that deployment and fails if it diverges from `openapi.snapshot.json` without a corresponding update. Set the variable under **Settings → Secrets and variables → Actions → Variables** once a staging backend is reachable; until then the job warns and passes.

`openapi.json` and `src/api/schema.ts` are gitignored.

## E2E credentials

Login/logout E2E tests run against the real backend when `E2E_ADMIN_EMAIL` and `E2E_ADMIN_PASSWORD` are set. Without those variables, credential-backed tests are skipped while route/form tests still run. E2E runs against the installed Chrome channel.

## Stack

Vite 5 · React 19 · TypeScript (strict + `noUncheckedIndexedAccess`) · TanStack Query v5 · TanStack Router · shadcn/ui · Tailwind CSS v4 · Zod · React Hook Form · openapi-fetch · date-fns · Vitest · Playwright · ESLint flat + typescript-eslint strict · Prettier.

See `docs/plan.md` §2 for the locked stack and §16 for coding conventions.

## Deploy

The backend repo's multi-stage Dockerfile copies `dist/` into the Rust container, which serves it from `/admin/*` via `tower_http::services::ServeDir`. See `docs/plan.md` §11.
