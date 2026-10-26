// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the header states nothing it does not know. An unknown P&L or SOL price
 * (`null` from the metrics route) reads "—" on the cards and the ticker, never "0".
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const METRICS = JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/header_metrics.json`, "utf8"));

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("home"), await loadIndex("shell")];

/** Home with the header metrics replaced by `metrics`. */
async function openHome(metrics) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(indexes, "populated");
  const onApi = (request) =>
    request.url.pathname === "/api/header/metrics"
      ? { body: JSON.stringify(metrics) }
      : api.onApi(request);
  const host = await serveDashboard(context, { locale: "en", onApi });
  const page = await context.newPage();
  await page.goto(`${host.origin}/home`);
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { page, close };
}

test("an unknown P&L and SOL price read as a dash", async () => {
  const { page, close } = await openHome({
    ...METRICS,
    trader: { ...METRICS.trader, today_pnl_native: null, today_pnl_percent: null },
    sol: { price_usd: null, change_24h_percent: null },
  });
  try {
    // The services line reads "Healthy" once the replaced metrics have rendered.
    await page.locator("#tickerServicesLine", { hasText: "Healthy" }).waitFor();
    assert.equal((await page.locator("#botPnL").textContent()).trim(), "—");
    assert.equal((await page.locator("#tickerTodayPnL").textContent()).trim(), "—");
    assert.equal((await page.locator("#solPriceValue").textContent()).trim(), "—");
  } finally {
    await close();
  }
});
