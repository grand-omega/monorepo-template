# lab-rust-server-admin

React SPA management interface for the Rust backend at [`../lab-rust-server`](../lab-rust-server). Served from `/admin/*` by the backend in production. See [`docs/plan.md`](docs/plan.md) for the full plan; this README is the quickstart.

## Quickstart

```bash
cp .env.example .env.local
npm install
npm run dev
```

Open http://localhost:5173/admin/. The dev server proxies `/admin/api/*` to `VITE_API_URL` (default `http://localhost:8080`).

## Scripts

| Command                           | Purpose                                                                       |
| --------------------------------- | ----------------------------------------------------------------------------- |
| `npm run dev`                     | Vite dev server with API proxy.                                               |
| `npm run build`                   | Type-check, then production build to `dist/`.                                 |
| `npm run preview`                 | Serve `dist/` locally.                                                        |
| `npm run lint`                    | ESLint flat config.                                                           |
| `npm run format` / `format:check` | Prettier.                                                                     |
| `npm run typecheck`               | `tsc -b --noEmit` across project references.                                  |
| `npm test`                        | Vitest (unit + component).                                                    |
| `npm run test:e2e`                | Playwright. Spins up dev server unless `CI` is set.                           |
| `npm run openapi`                 | Fetch live `/openapi.json` from `API_URL` and regenerate `src/api/schema.ts`. |
| `npm run openapi:gen`             | Regenerate from the committed `openapi.snapshot.json` only.                   |

## Codegen workflow

The OpenAPI client is generated from the backend's `/openapi.json`. The flow:

1. `openapi.snapshot.json` (committed) is the contract the SPA is built against. CI runs `openapi:gen` against the snapshot so the build is hermetic.
2. To update the contract: `API_URL=http://localhost:8080 npm run openapi`. Review the diff in `src/api/schema.ts`; if the change is intentional, also update `openapi.snapshot.json` in the same PR.
3. CI's `openapi-drift` job (enabled in step 2) fetches the live spec from a deployed backend and fails if it diverges from `openapi.snapshot.json` without a corresponding PR.

`openapi.json` and `src/api/schema.ts` are gitignored.

## Stack

Vite 5 · React 19 · TypeScript (strict + `noUncheckedIndexedAccess`) · TanStack Query v5 · TanStack Router · shadcn/ui · Tailwind CSS v4 · Zod · React Hook Form · openapi-fetch · date-fns · Vitest · Playwright · ESLint flat + typescript-eslint strict · Prettier.

See `docs/plan.md` §2 for the locked stack and §16 for coding conventions.

## Deploy

The backend repo's multi-stage Dockerfile copies `dist/` into the Rust container, which serves it from `/admin/*` via `tower_http::services::ServeDir`. See `docs/plan.md` §11.
