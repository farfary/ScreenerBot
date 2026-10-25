// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: `core/download.js` is the one place a dashboard script saves a file.
 *
 * Hand-built downloads drifted apart: one revoked the object URL right after the
 * click, which can cancel the save before the browser reads the blob. Every export
 * goes through `downloadBlob` or `downloadJson`.
 *
 * - No script outside the owner creates an object URL or sets an anchor's `download`.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { SCRIPTS_ROOT, lineOf, repoPath, walk } from "../lib/dashboard_ui.mjs";

const OWNER = resolve(SCRIPTS_ROOT, "core/download.js");
const HAND_BUILT = /URL\.createObjectURL\(|\.download\s*=|setAttribute\(\s*["']download["']/g;

test("only core/download.js saves a file", async () => {
  const found = [];
  for (const file of (await walk(SCRIPTS_ROOT)).filter((path) => path.endsWith(".js"))) {
    if (file === OWNER) continue;
    const source = readFileSync(file, "utf8");
    for (const match of source.matchAll(HAND_BUILT)) {
      found.push(`${repoPath(file)}:${lineOf(source, match.index)}`);
    }
  }
  assert.deepEqual(found, []);
});
