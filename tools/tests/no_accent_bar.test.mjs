// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: no surface marks its state with an inline-start accent stripe. A row,
 * card or toast states its condition through a tint, a caption or a badge; a
 * coloured stripe down one edge is decoration the design system does not use.
 *
 * - A pseudo-element painted as a full-height bar 1-4px wide is a stripe.
 * - A 2-6px solid inline-start (or left) border is a stripe. The allowlist holds
 *   the rules not yet moved off it and only shrinks; the DataTable header's
 *   drag-over edge is a drop-position indicator, not a state mark.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { STYLES_ROOT, repoPath, walk } from "../lib/dashboard_ui.mjs";

const BORDER_STRIPE_ALLOWLIST = new Set([
  "src/webserver/templates/styles/ui/data_table/core.css .dt-header-column.is-drag-over",
  "src/webserver/templates/styles/pages/tools/token_tools.css .ta-authority-item.success",
  "src/webserver/templates/styles/pages/tools/token_tools.css .ta-authority-item.warning",
  "src/webserver/templates/styles/pages/tools/token_tools.css .ta-risk-item.danger",
  "src/webserver/templates/styles/pages/tools/token_tools.css .ta-risk-item.warn, .ta-risk-item.warning",
  "src/webserver/templates/styles/pages/tools/token_tools.css .ta-risk-item.info",
]);

const rules = async () => {
  const found = [];
  for (const file of (await walk(STYLES_ROOT)).filter((path) => path.endsWith(".css"))) {
    const css = readFileSync(file, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    for (const match of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      found.push({
        file: repoPath(file),
        selector: match[1]
          .trim()
          .split(/\s*\n\s*/)
          .join(" "),
        body: match[2],
      });
    }
  }
  return found;
};

test("no pseudo-element paints a full-height edge stripe", async () => {
  const found = [];
  for (const { file, selector, body } of await rules()) {
    if (!/::?(?:before|after)\b/.test(selector)) continue;
    const thin = /(?:^|;)\s*width\s*:\s*[1-4]px\b/.test(body);
    const fullHeight =
      /\binset(?:-block)?\s*:\s*0\b/.test(body) ||
      (/\btop\s*:\s*0\b/.test(body) && /\bbottom\s*:\s*0\b/.test(body)) ||
      /(?:^|;)\s*height\s*:\s*100%/.test(body);
    if (thin && fullHeight) found.push(`${file} ${selector}`);
  }
  assert.deepEqual(found, []);
});

test("no inline-start border stripe outside the shrinking allowlist", async () => {
  const found = [];
  for (const { file, selector, body } of await rules()) {
    if (!/border-(?:inline-start|left)\s*:\s*[2-6]px solid/.test(body)) continue;
    const key = `${file} ${selector}`;
    if (!BORDER_STRIPE_ALLOWLIST.has(key)) found.push(key);
  }
  assert.deepEqual(found, []);
});

test("the stripe allowlist names only rules that still need it", async () => {
  const present = new Set(
    (await rules())
      .filter(({ body }) => /border-(?:inline-start|left)\s*:\s*[2-6]px solid/.test(body))
      .map(({ file, selector }) => `${file} ${selector}`)
  );
  for (const key of BORDER_STRIPE_ALLOWLIST)
    assert.ok(present.has(key), `${key} no longer needs the allowlist`);
});
