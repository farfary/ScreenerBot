// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the paper run card on the Trader Stats tab.
 *
 * - It shows the figures the paper account API sends: the run's start, its starting
 *   balance, cash, open cost and unrealized result, with the bare Paper glyph inline.
 * - A figure the account does not carry reads as a dash, never as a zero.
 * - The card offers the reset of the paper run.
 * - The Stats tab asks for the book and window it shows, and switches both from the shared
 *   segmented control.
 *
 * Run with `npm run test:js`.
 */

process.env.TZ = "UTC";

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

import { installEnglishI18n } from "../lib/dashboard_i18n.mjs";

installEnglishI18n();
const { paperAccountCard } = await import(
  new URL("../../src/webserver/templates/scripts/pages/trader/paper_account.js", import.meta.url)
);

const esc = (text) =>
  String(text).replace(/[&<>"']/g, (char) => `&#${char.charCodeAt(0)};`);
const plain = (text) => String(text).replace(/[⁦-⁩]/g, "");
const read = (path) =>
  fs.readFileSync(new URL(`../../src/webserver/templates/${path}`, import.meta.url), "utf8");

/** The `<dt>`/`<dd>` pairs of the card, as `{ label: value }`. */
function figures(card) {
  return Object.fromEntries(
    [...plain(card).matchAll(/<dt>([^<]*)<\/dt><dd>([^<]*)<\/dd>/g)].map((match) => [
      match[1],
      match[2],
    ])
  );
}

const ACCOUNT = {
  epoch: 2,
  started_at: 1_790_812_800,
  starting_balance_native: 1,
  cash_native: 0.9412,
  open_rounds: 2,
  open_cost_native: 0.0603,
  open_unrealized_pnl_native: -0.0021,
};

test("the card shows the run the paper account API sends", () => {
  const card = paperAccountCard(ACCOUNT, esc);
  assert.match(card, /^<i class="icon-test-tube-diagonal" aria-hidden="true"><\/i>/);
  assert.match(plain(card), /Paper run since Oct 1, 2026/);
  const shown = figures(card);
  assert.deepEqual(Object.keys(shown), ["Start", "Cash", "Open", "Unrealized"]);
  assert.match(shown.Start, /^1\sSOL$/);
  assert.match(shown.Cash, /^0\.9412\sSOL$/);
  assert.match(shown.Open, /^0\.0603\sSOL$/);
  assert.match(shown.Unrealized, /^[-\u2212]0\.0021\sSOL$/);
  assert.match(card, /<button[^>]*data-paper-reset>Reset paper run<\/button>/);
});

test("a figure the account does not carry reads as a dash", () => {
  const shown = figures(
    paperAccountCard({ ...ACCOUNT, open_unrealized_pnl_native: null, started_at: null }, esc)
  );
  assert.equal(shown.Unrealized, "—");
  assert.match(plain(paperAccountCard({ ...ACCOUNT, started_at: null }, esc)), /since —/);
  const empty = figures(paperAccountCard(null, esc));
  assert.deepEqual(Object.values(empty), ["—", "—", "—", "—"]);
});

test("the Stats tab asks for the book and window its switches show", () => {
  const page = read("scripts/pages/trader.js");
  assert.match(page, /\/api\/trader\/stats\?window=\$\{statsWindow\}&book=\$\{statsBook\}/);
  assert.match(page, /let statsBook = "live";/);
  assert.match(page, /let statsWindow = "30d";/);
  // Both switches are the shared segmented control, painted by the page into empty
  // placeholders: the book options name each mode with its glyph, the window options
  // are the one range set.
  const markup = read("pages/trader.html");
  assert.match(markup, /<div id="stats-book"><\/div>/);
  assert.match(markup, /<div id="stats-period"><\/div>/);
  const paint = page.slice(page.indexOf("function renderStatsControls()"));
  assert.match(paint, /\$\("#stats-book"\)[\s\S]*?renderSegmented\(/);
  assert.match(paint, /label: tradingModeLabel\(mode\),\s*icon: TRADING_MODE_ICONS\[mode\]/);
  assert.match(paint, /\$\("#stats-period"\)[\s\S]*?options: RANGE_OPTIONS/);
  assert.match(page, /\$\("#stats-book"\), "change"/);
  assert.match(page, /\$\("#stats-period"\), "change"/);
});
