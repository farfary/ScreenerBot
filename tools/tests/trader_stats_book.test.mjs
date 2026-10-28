// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Trader Stats tab shows one book at a time.
 *
 * - The tab opens on the live book and the 30-day window, and asks
 *   `/api/trader/stats?window=…&book=…` for exactly the book and window its switches show.
 * - Switching the book or the window reads again; the figures of one book never show
 *   under the other, even when a response of the previous book arrives late.
 * - The paper run is read from the paper account API, which carries the run alone:
 *   `{epoch, started_at, starting_balance_native, cash_native}`.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("trader"), await loadIndex("shell")];

const LIVE = JSON.parse(readFileSync(`${FIXTURES_ROOT}/trader/trader_stats.json`, "utf8"));
const PAPER = { ...LIVE, total_trades: 3, winners: 2, losers: 1, recent_rounds: [] };
const ACCOUNT = {
  epoch: 2,
  started_at: 1_790_812_800,
  starting_balance_native: 1,
  cash_native: 0.9412,
};

/**
 * The trader page on Stats. Every stats request is recorded as `{ book, window }`; a live
 * read is held while `holdLive` is set, until `releaseLive()` answers it.
 */
async function openStats() {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(indexes, "populated");
  const reads = [];
  const held = [];
  const state = { holdLive: false };
  const onApi = async (request) => {
    const { pathname, searchParams } = request.url;
    if (pathname === "/api/trader/stats") {
      const book = searchParams.get("book");
      reads.push({ book, window: searchParams.get("window") });
      if (book === "paper") return { body: JSON.stringify(PAPER) };
      if (state.holdLive) await new Promise((resolve) => held.push(resolve));
      return { body: JSON.stringify(LIVE) };
    }
    if (pathname === "/api/positions/paper/account") return { body: JSON.stringify(ACCOUNT) };
    return api.onApi(request);
  };
  const host = await serveDashboard(context, { locale: "en", onApi });
  const tab = await context.newPage();
  tab.setDefaultTimeout(15000);
  await tab.goto(`${host.origin}/trader#stats`);
  await tab.waitForSelector(PAGE_READY);
  const releaseLive = () => held.splice(0).forEach((resolve) => resolve());
  const close = async () => {
    releaseLive();
    await context.close();
    await host.close();
  };
  return { tab, reads, state, releaseLive, close };
}

const option = (tab, group, value) =>
  tab.locator(`#${group} .segmented-option`, { has: tab.locator(`input[value="${value}"]`) });

const trades = (tab) =>
  tab.evaluate(() => document.querySelector("#total-trades")?.textContent.trim());

test("the tab opens on the live book and the 30-day window", async () => {
  const { tab, reads, close } = await openStats();
  try {
    await tab.waitForFunction(() => document.querySelector("#total-trades")?.textContent === "24");
    assert.deepEqual(reads[0], { book: "live", window: "30d" });
    assert.equal(await tab.locator('#stats-book input[value="live"]').isChecked(), true);
    assert.equal(await tab.locator('#stats-period input[value="30d"]').isChecked(), true);
  } finally {
    await close();
  }
});

test("each switch asks for the book and window it shows", async () => {
  const { tab, reads, close } = await openStats();
  try {
    await tab.waitForFunction(() => document.querySelector("#total-trades")?.textContent === "24");
    await option(tab, "stats-period", "7d").click();
    await tab.waitForFunction(() => document.querySelector("#total-trades")?.textContent === "24");
    await option(tab, "stats-book", "paper").click();
    await tab.waitForFunction(() => document.querySelector("#total-trades")?.textContent === "3");
    assert.deepEqual(reads.at(-2), { book: "live", window: "7d" });
    assert.deepEqual(reads.at(-1), { book: "paper", window: "7d" });
  } finally {
    await close();
  }
});

test("a late response of the previous book never paints over the shown one", async () => {
  const { tab, state, releaseLive, close } = await openStats();
  try {
    await tab.waitForFunction(() => document.querySelector("#total-trades")?.textContent === "24");
    state.holdLive = true;
    await option(tab, "stats-period", "7d").click();
    // The live read is held; the switch to paper answers first.
    await option(tab, "stats-book", "paper").click();
    await tab.waitForFunction(() => document.querySelector("#total-trades")?.textContent === "3");
    state.holdLive = false;
    releaseLive();
    await tab.waitForTimeout(300);
    assert.equal(await trades(tab), "3", "the paper book keeps its own figures");
  } finally {
    await close();
  }
});
