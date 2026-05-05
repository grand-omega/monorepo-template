#!/usr/bin/env node
// Promote a freshly-pulled openapi.json to openapi.snapshot.json.
// Run this when the live contract changes intentionally and you want the
// new shape to become the SPA's reference. Always commit the resulting
// snapshot diff in the same PR as the code change that consumes it.

import { copyFileSync, existsSync } from "node:fs";
import process from "node:process";

const LIVE = "openapi.json";
const SNAPSHOT = "openapi.snapshot.json";

if (!existsSync(LIVE)) {
  process.stderr.write(
    `${LIVE} not found — run \`npm run openapi:pull\` first to fetch it from a backend.\n`
  );
  process.exit(1);
}

copyFileSync(LIVE, SNAPSHOT);
process.stderr.write(
  `copied ${LIVE} → ${SNAPSHOT}. Review the diff and commit alongside any client changes.\n`
);
