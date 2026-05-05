#!/usr/bin/env node
// Compare openapi.json (just-pulled) against openapi.snapshot.json.
// Exits 1 if they differ. Run after openapi-pull.mjs.
//
// Why deep-equal on parsed JSON instead of raw diff: utoipa may reorder keys
// or change formatting between runs without changing semantics. We compare
// canonical sorted-key serializations.

import { readFileSync } from "node:fs";
import process from "node:process";

const LIVE = "openapi.json";
const SNAPSHOT = "openapi.snapshot.json";

function canonicalize(value) {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value && typeof value === "object") {
    const sorted = {};
    for (const k of Object.keys(value).sort()) sorted[k] = canonicalize(value[k]);
    return sorted;
  }
  return value;
}

function loadJson(path) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (err) {
    throw new Error(`could not read ${path}: ${err.message}`);
  }
}

function summarizeDiff(live, snapshot) {
  const livePaths = new Set(Object.keys(live.paths ?? {}));
  const snapshotPaths = new Set(Object.keys(snapshot.paths ?? {}));
  const added = [...livePaths].filter((p) => !snapshotPaths.has(p));
  const removed = [...snapshotPaths].filter((p) => !livePaths.has(p));
  const common = [...livePaths].filter((p) => snapshotPaths.has(p));
  const changed = common.filter(
    (p) =>
      JSON.stringify(canonicalize(live.paths[p])) !==
      JSON.stringify(canonicalize(snapshot.paths[p]))
  );
  return { added, removed, changed };
}

function main() {
  const live = loadJson(LIVE);
  const snapshot = loadJson(SNAPSHOT);

  const liveCanon = JSON.stringify(canonicalize(live));
  const snapshotCanon = JSON.stringify(canonicalize(snapshot));
  if (liveCanon === snapshotCanon) {
    process.stderr.write("openapi: live spec matches snapshot.\n");
    return;
  }

  const { added, removed, changed } = summarizeDiff(live, snapshot);
  process.stderr.write("openapi: drift detected between live spec and snapshot.\n");
  if (added.length) process.stderr.write(`  + added paths: ${added.join(", ")}\n`);
  if (removed.length) process.stderr.write(`  - removed paths: ${removed.join(", ")}\n`);
  if (changed.length) process.stderr.write(`  ~ changed paths: ${changed.join(", ")}\n`);
  if (!added.length && !removed.length && !changed.length) {
    process.stderr.write(
      "  (schema, info, or components changed — see openapi.json vs openapi.snapshot.json)\n"
    );
  }
  process.stderr.write(
    "If this drift is intentional, run `npm run openapi:snapshot` and commit the updated openapi.snapshot.json.\n"
  );
  process.exit(1);
}

main();
