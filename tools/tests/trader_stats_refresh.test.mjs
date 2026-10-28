// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Auto Trader Stats tab reads its figures as soon as it is shown.
 *
 * The stats poller waits a full interval before its first run, so a switch back to Stats
 * left every card on its previous figures (or on "—") for five seconds. The tab loads
 * first and then starts its poller; activating the page on Stats reads once, not twice.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("trader"), await loadIndex("shell")];

test("Stats loads on activation once and again on every switch back, before any interval", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(indexes, "populated");
  let statsReads = 0;
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      if (request.url.pathname === "/api/trader/stats") statsReads += 1;
      return api.onApi(request);
    },
  });
  try {
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/trader#stats`);
    await page.waitForSelector(PAGE_READY);
    await page.waitForFunction(
      () => document.querySelector("#total-trades")?.textContent.trim() !== "—"
    );
    await page.waitForTimeout(500);
    assert.equal(statsReads, 1, "activation reads the stats once");

    await page.click('#subTabsContainer [data-tab-id="take-profit"]');
    const reread = page.waitForRequest((request) => request.url().includes("/api/trader/stats"), {
      timeout: 2000,
    });
    await page.click('#subTabsContainer [data-tab-id="stats"]');
    await reread;
    assert.equal(statsReads, 2);
  } finally {
    await context.close();
    await host.close();
  }
});
