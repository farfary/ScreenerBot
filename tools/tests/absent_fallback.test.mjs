// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: no dashboard script passes a bare hyphen as a formatter's fallback. An
 * absent value reads as an em dash or as the surface's own "not available" text; a
 * hyphen reads as a minus sign or a typo. The events dialog's copied report once
 * wrote an unknown age as "-" between lines whose other absent fields read "N/A".
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { SCRIPTS_ROOT, repoPath, walk } from "../lib/dashboard_ui.mjs";

const HYPHEN_FALLBACK = /\bfallback\s*:\s*(["'`])-\1/;

test("no formatter call falls back to a bare hyphen", async () => {
  const found = [];
  for (const file of (await walk(SCRIPTS_ROOT)).filter((path) => path.endsWith(".js"))) {
    readFileSync(file, "utf8")
      .split("\n")
      .forEach((line, index) => {
        if (HYPHEN_FALLBACK.test(line)) found.push(`${repoPath(file)}:${index + 1}`);
      });
  }
  assert.deepEqual(found, []);
});
