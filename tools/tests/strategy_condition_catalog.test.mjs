// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Add Condition catalog folds its categories with the icon chevrons and
 * one bulk control.
 *
 * The category disclosure was a text triangle that the stylesheet also rotated, so
 * an open section pointed back at its title ("◀"), and folding every category took a
 * Fold All and an Unfold All button side by side.
 *
 * - No strategies script writes a text triangle; a category toggle is a Lucide
 *   chevron (`icon-chevron-right` closed, mirrored in RTL; `icon-chevron-down` open)
 *   and the stylesheet does not rotate it.
 * - The catalog modal has one bulk toggle, labelled through a Fluent id.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { PAGES_ROOT, SCRIPTS_ROOT, STYLES_ROOT, rulesIn } from "../lib/dashboard_ui.mjs";

const catalog = readFileSync(`${SCRIPTS_ROOT}/pages/strategies/condition_catalog.js`, "utf8");
const page = readFileSync(`${SCRIPTS_ROOT}/pages/strategies.js`, "utf8");
const markup = readFileSync(`${PAGES_ROOT}/strategies.html`, "utf8");

test("category disclosure is a chevron icon, not a rotated text triangle", () => {
  for (const source of [catalog, page]) assert.doesNotMatch(source, /[▶▼◀▲►◄]/);
  assert.match(
    catalog,
    /category-toggle \$\{collapsed \? "icon-chevron-right" : "icon-chevron-down"\}/
  );
  const rules = rulesIn(readFileSync(`${STYLES_ROOT}/pages/strategies/modals.css`, "utf8")).filter(
    ({ selector }) => selector.includes(".category-toggle")
  );
  assert.ok(rules.length > 0);
  for (const { selector, body } of rules) assert.doesNotMatch(body, /transform/, selector);
});

test("the catalog has one bulk fold control", () => {
  const toggles = markup.match(/<button[^>]*class="catalog-toggle-btn"[^>]*>/g) ?? [];
  assert.equal(toggles.length, 1);
  assert.match(toggles[0], /id="toggle-all-categories"/);
});
