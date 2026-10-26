// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every help hint is the one shared trigger from `ui/hint_popover.js`, a bare
 * "?" glyph. A page header that draws its own help button with an info glyph and a
 * separate click path makes the same help read as "i" on one page and "?" on every
 * other, so no dashboard source spells a help glyph outside the owner.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { loadMarkupSources } from "../lib/dashboard_ui.mjs";

const OWNER = "scripts/ui/hint_popover.js";
const HELP_GLYPH =
  /icon-(?:circle-question-mark|circle-help|help-circle|badge-question-mark)\b|help-btn\b/;

test("only the hint owner draws a help glyph", async () => {
  const found = [];
  for (const { path, source } of await loadMarkupSources()) {
    if (path.endsWith(OWNER)) continue;
    source.split("\n").forEach((line, index) => {
      if (HELP_GLYPH.test(line)) found.push(`${path}:${index + 1}`);
    });
  }
  assert.deepEqual(found, []);
});
