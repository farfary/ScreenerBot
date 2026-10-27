// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the paper or live mode reads the same everywhere, from one owner
 * (`scripts/ui/trading_mode.js`).
 *
 * - `tradingModeMarker` renders Paper as its bare glyph inline with the label and Live as
 *   the label alone, with no glyph.
 * - No other dashboard source spells the Paper glyph; it reads `TRADING_MODE_ICONS`. A file
 *   still spelling it is listed, and the list only shrinks.
 * - No source names a private mode class (`copy-mode-*`, `*-mode-paper`, `*-mode-live`)
 *   in markup, a `className` or a stylesheet.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { installEnglishI18n } from "../lib/dashboard_i18n.mjs";
import { classesIn, loadMarkupSources, loadStylesheets } from "../lib/dashboard_ui.mjs";

installEnglishI18n();
const { TRADING_MODE_ICONS, tradingModeMarker } = await import(
  new URL("../../src/webserver/templates/scripts/ui/trading_mode.js", import.meta.url)
);

const esc = (text) =>
  String(text).replace(/[&<>"']/g, (char) => `&#${char.charCodeAt(0)};`);

const OWNER = "src/webserver/templates/scripts/ui/trading_mode.js";
const PAPER_GLYPH = "icon-test-tube-diagonal";

/** Sources that still spell the Paper glyph themselves; this list only shrinks. */
const GLYPH_SPELLED = ["src/webserver/templates/pages/trader.html"];

const PRIVATE_MODE_CLASS = /^(?:copy-mode-[\w-]*|[\w-]+-mode-(?:paper|live))$/;

test("Paper is its bare glyph inline with its label", () => {
  assert.equal(TRADING_MODE_ICONS.paper, PAPER_GLYPH);
  assert.equal(
    tradingModeMarker("paper", esc),
    `<span class="mode-marker" data-mode="paper"><i class="${PAPER_GLYPH}" aria-hidden="true"></i><span>Paper</span></span>`
  );
});

test("Live is its label alone, with no glyph", () => {
  assert.equal(TRADING_MODE_ICONS.live, undefined);
  assert.equal(
    tradingModeMarker("live", esc),
    '<span class="mode-marker" data-mode="live"><span>Live</span></span>'
  );
});

test("only the owner spells the Paper glyph", async () => {
  const found = [];
  for (const { path, source } of await loadMarkupSources()) {
    if (path !== OWNER && source.includes(PAPER_GLYPH)) found.push(path);
  }
  assert.deepEqual(found.sort(), GLYPH_SPELLED, "read TRADING_MODE_ICONS.paper instead");
});

test("no source names a private mode class", async () => {
  const found = [];
  const classValues =
    /(?:class|className)\s*=\s*(?:"([^"]*)"|'([^']*)'|`([^`]*)`)|classList\.(?:add|toggle)\(\s*["'`]([^"'`]+)/g;
  for (const { path, source } of await loadMarkupSources()) {
    source.split("\n").forEach((line, index) => {
      for (const match of line.matchAll(classValues)) {
        const value = match.slice(1).find((part) => part !== undefined) || "";
        const tokens = value.replace(/\$\{[^}]*\}?/g, " ").split(/\s+/);
        if (/copy-mode-\$\{/.test(value) || tokens.some((token) => PRIVATE_MODE_CLASS.test(token))) {
          found.push(`${path}:${index + 1}`);
        }
      }
    });
  }
  for (const { path, rules } of await loadStylesheets()) {
    for (const { selector, line } of rules) {
      if (classesIn(selector).some((name) => PRIVATE_MODE_CLASS.test(name))) {
        found.push(`${path}:${line} ${selector}`);
      }
    }
  }
  assert.deepEqual(found, []);
});
