#!/usr/bin/env bun
// Fetch the live /openapi.json from a backend, strip the /admin/api prefix
// from path keys (so it matches the SPA client baseUrl), and write to
// openapi.json. The snapshot is updated separately via openapi-snapshot.mjs.

import { writeFileSync } from "node:fs";
import process from "node:process";

const ADMIN_API_PREFIX = "/admin/api";
const DEFAULT_URL = "http://localhost:8080";

function parseArgs(argv) {
  const out = { url: process.env.API_URL ?? DEFAULT_URL, output: "openapi.json" };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--url" || a === "-u") out.url = argv[++i];
    else if (a === "--output" || a === "-o") out.output = argv[++i];
    else if (!a.startsWith("-")) out.url = a;
  }
  return out;
}

function stripPrefix(spec) {
  if (!spec || typeof spec !== "object" || !spec.paths) return spec;
  const next = {};
  for (const [key, value] of Object.entries(spec.paths)) {
    const stripped = key.startsWith(ADMIN_API_PREFIX) ? key.slice(ADMIN_API_PREFIX.length) : key;
    // Drop non-/admin/api paths so the SPA client only sees the admin surface.
    if (!key.startsWith(ADMIN_API_PREFIX)) continue;
    next[stripped || "/"] = value;
  }
  return { ...spec, paths: next, servers: [{ url: ADMIN_API_PREFIX }] };
}

async function main() {
  const { url, output } = parseArgs(process.argv.slice(2));
  const specUrl = url.replace(/\/+$/, "") + "/openapi.json";
  process.stderr.write(`fetching ${specUrl}\n`);

  const res = await fetch(specUrl);
  if (!res.ok) {
    throw new Error(`GET ${specUrl} failed: ${res.status} ${res.statusText}`);
  }
  const spec = await res.json();
  const stripped = stripPrefix(spec);
  const pathCount = Object.keys(stripped.paths ?? {}).length;
  if (pathCount === 0) {
    throw new Error(
      `no paths under ${ADMIN_API_PREFIX} found in spec — backend likely hasn't moved /admin/* to JSON yet (see plan §1)`
    );
  }
  writeFileSync(output, JSON.stringify(stripped, null, 2) + "\n");
  process.stderr.write(`wrote ${output} (${pathCount} admin paths)\n`);
}

main().catch((err) => {
  process.stderr.write(`openapi:pull failed — ${err.message}\n`);
  process.exit(1);
});
