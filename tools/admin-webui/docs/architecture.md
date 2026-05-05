# Architecture decisions log

Append entries when you make a non-obvious call. Newest on top. Keep entries short — link to PRs/commits for the long version.

## 2026-05-04 — Step 1: scaffolding

- **Vite `base: "/admin/"`**. The backend serves the SPA from `/admin/*`, so all asset URLs need that prefix at build time. Dev server runs at `http://localhost:5173/admin/`.
- **Project references in TS** (`tsconfig.app.json` + `tsconfig.node.json`). Keeps DOM-typed app code separate from Node-typed config files; `verbatimModuleSyntax` is strict in both.
- **Manual chunk split** for `react`, `router`, `query` so the main chunk stays under the 250 KB gzipped budget as features land. Revisit after step 8.
- **Snapshot-driven codegen**. CI runs `openapi:gen` against `openapi.snapshot.json` (committed). Live `/openapi.json` and the generated `src/api/schema.ts` are gitignored. See README "Codegen workflow."
- **Deploy mode A** (per plan §11): backend's Docker build clones this repo at a pinned tag. Default until proven painful.
