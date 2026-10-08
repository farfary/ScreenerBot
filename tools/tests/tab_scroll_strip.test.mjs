// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every tab row that scrolls sideways is a tab scroll strip, drawn and driven by
 * one owner (`.tab-scroll-*` in `styles/ui/tab_bar.css`, `attachTabScrollStrip` in
 * `scripts/ui/tab_bar.js`).
 *
 * A row that overflows without the strip cuts its last tab at the edge with nothing to
 * say more tabs exist, and a second copy of the overflow tracking drifts from the first
 * (the sub-tab row once toggled its fade classes on a parent that drew no fade).
 *
 * - Each `.tab-scroll-area` in the shell sits in a `.tab-scroll-wrapper` with a start
 *   and an end page button, each labelled through a Fluent id.
 * - Only `tab_bar.js` toggles the `can-scroll-*` state and only `tab_bar.css` styles it.
 * - The page button is a bare glyph: no background, border or shadow.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  BASE_HTML,
  SCRIPTS_ROOT,
  STYLES_ROOT,
  classTokens,
  findOpenTags,
  repoPath,
  rulesIn,
  walk,
} from "../lib/dashboard_ui.mjs";

const OWNER_SCRIPT = `${SCRIPTS_ROOT}/ui/tab_bar.js`;
const OWNER_STYLES = `${STYLES_ROOT}/ui/tab_bar.css`;

/** The markup between a wrapper's opening tag and its matching close. */
function elementSource(source, start) {
  const tag = /<\/?div\b[^>]*>/g;
  tag.lastIndex = start;
  let depth = 0;
  let match;
  while ((match = tag.exec(source))) {
    depth += match[0].startsWith("</") ? -1 : 1;
    if (depth === 0) return source.slice(start, tag.lastIndex);
  }
  return source.slice(start);
}

test("every scrolling tab row in the shell is a complete tab scroll strip", () => {
  const source = readFileSync(BASE_HTML, "utf8");
  const wrappers = findOpenTags(source, "div").filter((tag) =>
    classTokens(tag.tag).statics.includes("tab-scroll-wrapper")
  );
  const areas = findOpenTags(source).filter((tag) =>
    classTokens(tag.tag).statics.includes("tab-scroll-area")
  );
  assert.ok(areas.length >= 2, "the main navigation and sub-tab rows are both strips");
  assert.equal(wrappers.length, areas.length, "one wrapper per scrolling tab row");

  for (const wrapper of wrappers) {
    const inner = elementSource(source, wrapper.index);
    for (const side of ["start", "end"]) {
      const button = findOpenTags(inner, "button").find((tag) =>
        classTokens(tag.tag).statics.includes(`tab-scroll-btn--${side}`)
      );
      assert.ok(button, `${wrapper.tag} has no ${side} page button`);
      assert.match(button.tag, /data-l10n-id="shell-tabs-scroll-/);
      assert.match(button.tag, /type="button"/);
    }
    assert.match(inner, /class="[^"]*\btab-scroll-area\b/);
  }
});

test("only the tab bar owner tracks and styles the strip overflow state", async () => {
  const found = [];
  for (const file of (await walk(SCRIPTS_ROOT)).filter((path) => path.endsWith(".js"))) {
    if (file === OWNER_SCRIPT) continue;
    if (/can-scroll-(?:start|end)/.test(readFileSync(file, "utf8"))) found.push(repoPath(file));
  }
  for (const file of (await walk(STYLES_ROOT)).filter((path) => path.endsWith(".css"))) {
    if (file === OWNER_STYLES) continue;
    if (
      /\.can-scroll-(?:start|end)\b|\.tab-scroll-(?:wrapper|btn)\b/.test(readFileSync(file, "utf8"))
    )
      found.push(repoPath(file));
  }
  assert.deepEqual(found, []);
});

test("the strip page button is a bare glyph", () => {
  const rules = rulesIn(readFileSync(OWNER_STYLES, "utf8")).filter(({ selector }) =>
    /\.tab-scroll-btn\b/.test(selector)
  );
  assert.ok(rules.length > 0, "no .tab-scroll-btn rule in tab_bar.css");
  for (const { selector, body } of rules) {
    for (const declaration of body.matchAll(
      /(?:^|;)\s*(background(?:-color|-image)?|border(?:-[a-z-]+)?|box-shadow|filter)\s*:\s*([^;]+)/g
    )) {
      assert.match(
        declaration[2].trim(),
        /^(?:none|transparent|0)$/,
        `${selector} paints ${declaration[1]}: ${declaration[2].trim()}`
      );
    }
  }
});
