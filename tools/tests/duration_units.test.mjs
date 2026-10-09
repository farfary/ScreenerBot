// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: `formatDuration` (core/format.js) is given nanoseconds, and nothing else.
 *
 * It reads its argument as nanoseconds. A hold time in seconds passed to it printed
 * "130.2µs" for a position held for hours, and a Telegram session age in seconds
 * times 1000 printed microseconds as well. An elapsed span in seconds is
 * `formatUptime`'s job.
 *
 * - Every `formatDuration` call in the dashboard scripts passes a value whose name
 *   says nanoseconds (`_ns`).
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { loadMarkupSources } from "../lib/dashboard_ui.mjs";

test("formatDuration is only given nanoseconds", async () => {
  const calls = [];
  for (const { path, source } of await loadMarkupSources()) {
    const shared =
      /\bimport\s*\{[^}]*\bformatDuration\b[^}]*\}\s*from\s*["'][./]*core\/(format|utils)\.js["']/.test(
        source
      );
    // The rest of the call's line: an argument may itself contain a call.
    const pattern = shared ? /\bformatDuration\(([^\n]*)/g : /\bUtils\.formatDuration\(([^\n]*)/g;
    for (const match of source.matchAll(pattern)) calls.push({ path, argument: match[1] });
  }
  assert.ok(calls.length >= 5, "the formatDuration call sites were found");
  const wrong = calls.filter(({ argument }) => !/_ns\b/.test(argument));
  assert.deepEqual(
    wrong.map(({ path, argument }) => `${path}: formatDuration(${argument})`),
    []
  );
});
