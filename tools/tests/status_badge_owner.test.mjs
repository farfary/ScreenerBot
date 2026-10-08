// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the state badges (`.badge.success`, `.badge.warning`, `.badge.error`,
 * `.badge.info`, `.badge.secondary`, ...) carry one weight, declared once by their
 * owner, `styles/components.css`. Every state is a faint tint of its colour behind
 * text in that colour, so a healthy row never outweighs a disabled one. A page or
 * dialog may size a badge but never recolours a state, and no hyphenated variant
 * family (`.badge-success`) stands beside the shared one.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { STYLES_ROOT, repoPath, rulesIn, walk } from "../lib/dashboard_ui.mjs";

const OWNER = "src/webserver/templates/styles/components.css";
const STATES = "online|success|loading|warning|error|danger|info|secondary";
const STATE_BADGE = new RegExp(`\\.badge(?:\\.|-)(?:${STATES})(?![\\w-])`);

async function stateBadgeRules() {
  const found = [];
  for (const file of (await walk(STYLES_ROOT)).filter((path) => path.endsWith(".css"))) {
    for (const rule of rulesIn(readFileSync(file, "utf8"))) {
      if (STATE_BADGE.test(rule.selector)) found.push({ file: repoPath(file), ...rule });
    }
  }
  return found;
}

test("only the badge owner styles a badge state", async () => {
  const outside = (await stateBadgeRules())
    .filter((rule) => rule.file !== OWNER)
    .map((rule) => `${rule.file}:${rule.line} ${rule.selector}`);
  assert.deepEqual(outside, []);
});

test("every badge state is a tint behind state-coloured text", async () => {
  const owned = (await stateBadgeRules()).filter((rule) => rule.file === OWNER);
  assert.ok(owned.length > 0, "components.css declares the badge states");
  const solid = owned
    .filter((rule) => /(?:^|;)\s*color\s*:\s*(?:#fff\b|#ffffff\b|white\b)/i.test(rule.body))
    .map((rule) => rule.selector);
  assert.deepEqual(solid, []);
});
