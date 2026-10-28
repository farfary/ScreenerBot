// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the No Market Data view lists the outcome of each token's market check, not
 * the market columns that are always empty for it.
 *
 * - The view has exactly the token, reason, last check and first seen columns.
 * - A token whose checks stopped reads "Not listed · checks stopped"; one never
 *   checked reads "Not checked yet".
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
const indexes = [await loadIndex("tokens"), await loadIndex("shell")];

test("the No Market Data view shows the market check columns", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(indexes, "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  try {
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/tokens`);
    await page.waitForSelector(PAGE_READY);
    await page.locator('#subTabsContainer [data-tab-id="no_market"]').click();
    await page.waitForSelector("#tokens-root tr[data-row-id] >> visible=true");

    const columns = await page.$$eval("#tokens-root th[data-column-id]", (headers) =>
      headers.map((header) => header.dataset.columnId)
    );
    assert.deepEqual(columns, ["token", "market_check", "market_checked_at", "first_seen_at"]);

    const reasons = await page.$$eval(
      '#tokens-root tr[data-row-id] td[data-column-id="market_check"]',
      (cells) => cells.map((cell) => cell.textContent.trim())
    );
    assert.deepEqual(reasons, [
      "Not listed · checks stopped",
      "Not listed · rechecking",
      "Not checked yet",
    ]);
  } finally {
    await context.close();
    await host.close();
  }
});
