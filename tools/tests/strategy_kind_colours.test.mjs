// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a strategy's kind has one colour pair on every surface. Entry is the
 * primary blue and Exit the warning amber, declared once as the
 * `--strategy-kind-*` properties; no kind selector picks a success or error
 * colour of its own, so an Exit strategy never reads as a failure.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { STYLES_ROOT, repoPath, rulesIn, walk } from "../lib/dashboard_ui.mjs";

const KIND_SELECTOR = /\.(?:entry|exit)\b|data-type="(?:ENTRY|EXIT)"/;
const SEMANTIC_COLOUR = /--(?:success|error|danger)-|rgb\(/;

test("strategy kinds take one blue and amber pair", async () => {
  const pairs = new Map();
  const offenders = [];
  for (const file of (await walk(`${STYLES_ROOT}/pages`)).filter((path) => path.endsWith(".css"))) {
    if (!/strateg/.test(file)) continue;
    for (const rule of rulesIn(readFileSync(file, "utf8"))) {
      if (!KIND_SELECTOR.test(rule.selector)) continue;
      if (SEMANTIC_COLOUR.test(rule.body)) offenders.push(`${repoPath(file)}: ${rule.selector}`);
      const color = /--strategy-kind-color:\s*([^;]+);/.exec(rule.body)?.[1];
      // One declaration per kind: a selector list yields one rule per selector.
      if (color)
        pairs.set(`${repoPath(file)}:${rule.line}`, [
          /entry|ENTRY/.test(rule.selector) ? "entry" : "exit",
          color.trim(),
        ]);
    }
  }
  assert.deepEqual(offenders, [], "kind selectors carry no colour of their own");
  assert.deepEqual([...pairs.values()].sort(), [
    ["entry", "var(--primary-color)"],
    ["exit", "var(--warning-color)"],
  ]);
});
