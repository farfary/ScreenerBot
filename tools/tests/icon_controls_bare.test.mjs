// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: an icon-only control is a bare glyph. The shared icon controls (the icon
 * button, the full-screen dialog header actions and close, the modal close, the help
 * hint, the hint popover's title glyph and the app header's quick actions) state
 * hover, active and semantic variants through the glyph colour alone. No rule for them, in their owner or in a page,
 * paints a background, a border or a shadow behind the glyph.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { STYLES_ROOT, repoPath, walk } from "../lib/dashboard_ui.mjs";

const ICON_CONTROLS = [
  "btn-icon",
  "dialog-header-action",
  "dialog-close",
  "modal-close",
  "hint-trigger",
  "hint-popover__icon",
  "header-action-btn",
  "header-actions-toggle",
];

const names = (selector) =>
  ICON_CONTROLS.filter((name) => new RegExp(`\\.${name}(?![\\w-])`).test(selector));

const PAINT = [
  /(?:^|;)\s*background(?:-color)?\s*:\s*(?!\s|none\b|transparent\b)[^;]+/,
  /(?:^|;)\s*border(?:-(?:inline|block)(?:-(?:start|end))?|-top|-bottom|-left|-right)?\s*:\s*(?!\s|none\b|0\b)[^;]+/,
  /(?:^|;)\s*border-color\s*:\s*(?!\s|transparent\b)[^;]+/,
  /(?:^|;)\s*box-shadow\s*:\s*(?!\s|none\b)[^;]+/,
];

test("icon-only controls paint nothing behind their glyph", async () => {
  const found = [];
  for (const file of (await walk(STYLES_ROOT)).filter((path) => path.endsWith(".css"))) {
    const css = readFileSync(file, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    for (const match of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      const selector = match[1]
        .trim()
        .split(/\s*\n\s*/)
        .join(" ");
      if (names(selector).length === 0) continue;
      // A focus-visible rule belongs to the global input-modality focus ring.
      if (/:focus/.test(selector)) continue;
      for (const pattern of PAINT) {
        const paint = match[2].match(pattern);
        if (paint) found.push(`${repoPath(file)} ${selector} -> ${paint[0].replace(/^;?\s*/, "")}`);
      }
    }
  }
  assert.deepEqual(found, []);
});
