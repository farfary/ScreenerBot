// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every dashboard menu wears the one menu look from `base/floating.css`.
 *
 * Select lists, action menus, row menus, the toolbar overflow, the column menu and the
 * context menu each drifted into their own surface (frosted blur, hardcoded shadows,
 * notched corners, solid-primary hover) and their own row (pills, 32/34/36px heights,
 * a hover that recoloured the text). The `--menu-*` tokens are the single owner, so:
 *
 * - each menu surface's base rule paints from the surface tokens;
 * - each menu row's base rule takes its height and padding from the row tokens;
 * - no rule drawing a menu surface or row blurs its backdrop or hardcodes a colour;
 * - a row's hover paints a menu token behind the text and never recolours the text;
 * - every script that places a menu clamps it into `menuBounds` (`core/menu_manager.js`),
 *   the window above the status bar, which stays on top of every menu.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { SCRIPTS_ROOT, loadStylesheets, subjectCompound, classesIn } from "../lib/dashboard_ui.mjs";

/** Each menu surface: the sheet that owns it and its base selector. */
const SURFACES = [
  { sheet: "ui/custom_select.css", selector: ".cs-dropdown", shadow: false },
  { sheet: "ui/dropdown.css", selector: ".dropdown-menu" },
  { sheet: "ui/data_table/column_types.css", selector: ".dt-actions-dropdown-menu" },
  { sheet: "ui/data_table/core.css", selector: ".dt-col-menu" },
  { sheet: "ui/table_toolbar.css", selector: ".table-toolbar-menu" },
  { sheet: "ui/context_menu.css", selector: ".context-menu" },
  { sheet: "ui/context_menu.css", selector: ".context-menu-submenu" },
  { sheet: "pages/assistant/instructions.css", selector: ".instruction-context-menu" },
  { sheet: "pages/assistant/automation.css", selector: ".automation-context-menu" },
];

/** Each menu row: the sheet that owns it and its base selector. */
const ROWS = [
  { sheet: "ui/dropdown.css", selector: ".dropdown-item" },
  { sheet: "ui/data_table/column_types.css", selector: ".dt-actions-dropdown-item" },
  { sheet: "ui/data_table/core.css", selector: ".dt-col-menu-item" },
  { sheet: "ui/table_toolbar.css", selector: ".table-toolbar-menu__item" },
  { sheet: "ui/context_menu.css", selector: ".context-menu-item" },
  {
    sheet: "pages/assistant/instructions.css",
    selector: ".instruction-context-menu .context-menu-item",
  },
  {
    sheet: "pages/assistant/automation.css",
    selector: ".automation-context-menu .context-menu-item",
  },
];

/** The scripts that place a menu in the window. */
const PLACEMENT_SCRIPTS = [
  "core/menu_manager.js",
  "ui/custom_select.js",
  "ui/context_menu.js",
  "ui/data_table.js",
];

/** A select option fills its field's height, so its row is owned by the select. */
const ROW_CLASSES = new Set(
  ROWS.map(({ selector }) => classesIn(subjectCompound(selector))[0]).concat("cs-option")
);
const SURFACE_CLASSES = new Set(SURFACES.map(({ selector }) => selector.slice(1)));

const RAW_COLOUR =
  /(?:^|;)\s*(?:background(?:-color)?|border(?:-[\w-]+)?|box-shadow|color)\s*:[^;]*(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i;
const BACKDROP = /(?:^|;)\s*backdrop-filter\s*:/;
const COLOUR = /(?:^|;)\s*color\s*:/;
const HOVER_BG = /(?:^|;)\s*background\s*:\s*var\(--(?:menu-item-[\w-]+|success-alpha-10)\)/;

const sheets = await loadStylesheets();

function baseRule(sheet, selector) {
  const found = sheets.find(({ path }) => path === sheet);
  assert.ok(found, `${sheet} exists`);
  const rule = found.rules.find((candidate) => candidate.selector === selector);
  assert.ok(rule, `${sheet} has a base rule for ${selector}`);
  return rule;
}

/** Every rule, in every sheet, whose subject is a menu surface or a menu row. */
function menuRules(classes) {
  const found = [];
  for (const { path, rules } of sheets) {
    for (const rule of rules) {
      const subject = classesIn(subjectCompound(rule.selector));
      if (subject.some((name) => classes.has(name))) found.push({ path, ...rule });
    }
  }
  return found;
}

test("every menu surface paints from the menu tokens", () => {
  for (const { sheet, selector, shadow = true } of SURFACES) {
    const { body, line } = baseRule(sheet, selector);
    const where = `${sheet}:${line} ${selector}`;
    assert.match(body, /background:\s*var\(--menu-bg\)/, `${where} background`);
    assert.match(body, /border[\w-]*:[^;]*var\(--menu-border\)/, `${where} border`);
    assert.match(body, /border-radius:[^;]*var\(--menu-radius\)/, `${where} radius`);
    if (shadow) assert.match(body, /box-shadow:\s*var\(--menu-shadow\)/, `${where} shadow`);
  }
});

test("every menu row takes its height and padding from the row tokens", () => {
  for (const { sheet, selector } of ROWS) {
    const { body, line } = baseRule(sheet, selector);
    const where = `${sheet}:${line} ${selector}`;
    assert.match(body, /min-height:\s*var\(--menu-item-min-height\)/, `${where} height`);
    assert.match(body, /padding:\s*var\(--menu-item-padding\)/, `${where} padding`);
    assert.doesNotMatch(body, /(?:^|;)\s*margin\s*:\s*(?!0\b)/, `${where} is edge to edge`);
  }
});

test("no menu surface or row blurs its backdrop or hardcodes a colour", () => {
  const offenders = menuRules(new Set([...SURFACE_CLASSES, ...ROW_CLASSES]))
    .filter(({ body }) => BACKDROP.test(body) || RAW_COLOUR.test(body))
    .map(({ path, line, selector }) => `${path}:${line} ${selector}`);
  assert.deepEqual(offenders, [], "menus draw from the --menu-* tokens");
});

test("a menu row's hover paints a menu token and keeps the text colour", () => {
  const hovers = menuRules(ROW_CLASSES).filter(({ selector }) =>
    /:hover\b/.test(subjectCompound(selector))
  );
  assert.ok(hovers.length >= ROWS.length, "every row has a hover rule");
  for (const { path, line, selector, body } of hovers) {
    const where = `${path}:${line} ${selector}`;
    assert.doesNotMatch(body, COLOUR, `${where} recolours the text`);
    if (/(?:^|;)\s*background\s*:/.test(body)) assert.match(body, HOVER_BG, where);
  }
});

test("every menu placement clamps into the menu bounds", async () => {
  for (const script of PLACEMENT_SCRIPTS) {
    const source = await readFile(resolve(SCRIPTS_ROOT, script), "utf8");
    assert.match(source, /\bmenuBounds\(/, `${script} places a menu outside menuBounds`);
  }
});
