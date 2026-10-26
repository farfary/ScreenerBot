// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the header states nothing it does not know. An unknown P&L or SOL price
 * (`null` from the metrics route) reads "—" on the cards and the ticker, never "0".
 * Services that have not reported yet read a neutral "Starting"; only a confirmed
 * fault is an issue.
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

/** The ticker's services line and its dot, once the metrics have rendered. */
async function servicesStatus(page, expected) {
  await page.locator("#tickerServicesLine", { hasText: expected }).waitFor();
  return page.evaluate(() => ({
    text: document.querySelector("#tickerServicesLine").textContent.trim(),
    dot: document.querySelector("#tickerServicesText .status-dot").className,
  }));
}

test("an unknown P&L and SOL price read as a dash, and startup is neutral", async () => {
  const { page, close } = await openHome({
    ...METRICS,
    trader: { ...METRICS.trader, today_pnl_native: null, today_pnl_percent: null },
    sol: { price_usd: null, change_24h_percent: null },
    system: {
      all_services_healthy: false,
      unhealthy_services: [],
      critical_degraded: false,
      starting: true,
    },
  });
  try {
    const status = await servicesStatus(page, "Starting");
    assert.equal(status.text, "Services: Starting");
    assert.equal(status.dot, "status-dot unknown");
    assert.equal((await page.locator("#botPnL").textContent()).trim(), "—");
    assert.equal((await page.locator("#tickerTodayPnL").textContent()).trim(), "—");
    assert.equal((await page.locator("#solPriceValue").textContent()).trim(), "—");
  } finally {
    await close();
  }
});

test("a confirmed fault is an issue even while other services start", async () => {
  const { page, close } = await openHome({
    ...METRICS,
    system: {
      all_services_healthy: false,
      unhealthy_services: ["pools"],
      critical_degraded: false,
      starting: true,
    },
  });
  try {
    const status = await servicesStatus(page, "Issue");
    // Fluent isolates the interpolated count with bidi marks.
    assert.equal(status.text.replace(/[\u2068\u2069]/g, ""), "Services: 1 Issue");
    assert.equal(status.dot, "status-dot warning");
  } finally {
    await close();
  }
});
