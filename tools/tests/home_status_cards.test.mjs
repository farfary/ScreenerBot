// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Tests for the Home status cards module (`pages/home/status_cards.js`).
 *
 * Every figure on the Auto Trader and Copy Trading cards is the server's: the module maps
 * each card's state to its label, formats each figure, and renders an absent one as a dash.
 * The guard at the end reads the module source and fails on any arithmetic, aggregation or
 * numeric parsing, so a figure cannot start being derived in the page.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import "./fixtures/i18n_en.mjs";

const MODULE = "../../src/webserver/templates/scripts/pages/home/status_cards.js";
const SOURCE = new URL(MODULE, import.meta.url);

const { describeAutoTrader, describeCopyTrading } = await import(MODULE);

// Formatters that show what they were given, so each figure is traced to its field.
const Utils = {
  formatNumber: (value, { fallback }) => (value == null ? fallback : `n:${value}`),
  formatTimeAgo: (value, { fallback }) => (value == null ? fallback : `ago:${value}`),
};

const plain = (text) => text.replace(/[⁦-⁩]/g, "");
const shown = (view) =>
  Object.fromEntries(
    Object.entries(view).map(([key, value]) => [key, typeof value === "string" ? plain(value) : value])
  );

test("a running Auto Trader shows capacity and today's entries and exits", () => {
  const view = describeAutoTrader(
    { state: "running", open_positions: 5, max_open_positions: 8, entries_today: 4, exits_today: 3 },
    Utils
  );
  assert.deepEqual(shown(view), {
    state: "running",
    stateText: "Running",
    unavailable: false,
    openPositions: "n:5 of n:8",
    entriesToday: "n:4",
    exitsToday: "n:3",
  });
});

test("a stopped Auto Trader renders every absent figure as a dash", () => {
  const view = describeAutoTrader(
    { state: "stopped", open_positions: 2, max_open_positions: null, entries_today: null, exits_today: 0 },
    Utils
  );
  assert.equal(view.stateText, "Stopped");
  assert.equal(view.openPositions, "—", "capacity needs both the count and the limit");
  assert.equal(view.entriesToday, "—");
  assert.equal(view.exitsToday, "n:0", "a zero is a figure, not an absence");
});

test("an unavailable card says setup is required", () => {
  const trader = describeAutoTrader(
    { state: "unavailable", open_positions: null, max_open_positions: null, entries_today: null, exits_today: null },
    Utils
  );
  assert.equal(trader.unavailable, true);
  assert.equal(trader.stateText, "Setup required");
  const copy = describeCopyTrading(
    { state: "unavailable", followed_wallets: null, active_tasks: null, total_tasks: null, last_copy_at: null },
    Utils
  );
  assert.equal(copy.unavailable, true);
  assert.equal(copy.stateText, "Setup required");
  assert.deepEqual([copy.followedWallets, copy.activeTasks, copy.lastCopy], ["—", "—", "—"]);
});

test("Copy Trading shows followed wallets, active of total tasks and the last copy", () => {
  const view = describeCopyTrading(
    {
      state: "running",
      followed_wallets: 2,
      active_tasks: 2,
      total_tasks: 3,
      last_copy_at: "2026-10-07T09:12:00+00:00",
    },
    Utils
  );
  assert.deepEqual(shown(view), {
    state: "running",
    stateText: "Running",
    unavailable: false,
    followedWallets: "n:2",
    activeTasks: "n:2 of n:3",
    lastCopy: "ago:2026-10-07T09:12:00+00:00",
  });
});

test("an unreadable copy store or a missing card shows no state and no figures", () => {
  for (const card of [
    { state: null, followed_wallets: null, active_tasks: null, total_tasks: null, last_copy_at: null },
    undefined,
  ]) {
    const view = describeCopyTrading(card, Utils);
    assert.equal(view.state, null);
    assert.equal(view.stateText, "—");
    assert.equal(view.unavailable, false);
    assert.deepEqual([view.followedWallets, view.activeTasks, view.lastCopy], ["—", "—", "—"]);
  }
  assert.equal(describeAutoTrader(undefined, Utils).stateText, "—");
});

/** The module source with comments, strings and template literals blanked out. */
function codeOnly(source) {
  let out = "";
  for (let i = 0; i < source.length; i += 1) {
    const ch = source[i];
    const next = source[i + 1];
    if (ch === "/" && next === "/") {
      while (i < source.length && source[i] !== "\n") i += 1;
      out += "\n";
    } else if (ch === "/" && next === "*") {
      i = source.indexOf("*/", i + 2) + 1;
      out += " ";
    } else if (ch === '"' || ch === "'" || ch === "`") {
      i += 1;
      while (i < source.length && source[i] !== ch) i += source[i] === "\\" ? 2 : 1;
      out += '""';
    } else {
      out += ch;
    }
  }
  return out;
}

test("guard: the status cards compute no figure in the page", () => {
  const code = codeOnly(readFileSync(SOURCE, "utf8"));
  assert.ok(code.includes("describeAutoTrader"), "the guard reads the module");
  for (const pattern of [
    /[+\-*/%]/,
    /\bMath\./,
    /\.reduce\(/,
    /\.length\b/,
    /\bNumber\(/,
    /\bparse(Int|Float)\(/,
  ]) {
    assert.doesNotMatch(code, pattern, `status_cards.js computes a figure (${pattern})`);
  }
});
