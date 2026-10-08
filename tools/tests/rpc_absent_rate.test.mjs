// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the RPC readouts show "—" until there is a call to measure.
 *
 * Right after a restart the header ticker read "RPC: 0.0/min · 0%" and the status
 * bar "100.0%": a success rate over no calls is not a measurement, and either
 * number reads as one.
 *
 * - Header ticker: with no RPC manager the rate and the success rate are "—";
 *   with a manager and no calls the rate is 0 and the success rate is "—".
 * - Status bar: with no calls the success rate is "—" and the health dot is
 *   "unknown".
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

const fixture = (name) => JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/${name}`, "utf8"));
const header = fixture("header_metrics.json");
const status = fixture("status.json");

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const shellIndex = await loadIndex("shell");
const homeIndex = await loadIndex("home");

/** Opens the dashboard with the header and status answers replaced. */
async function open(t, { headerRpc, statusRpc }) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([homeIndex, shellIndex], "populated");
  const answers = {
    "/api/header/metrics": JSON.stringify({ ...header, rpc: { ...header.rpc, ...headerRpc } }),
    "/api/status": JSON.stringify({
      ...status,
      rpc_stats: { ...status.rpc_stats, ...statusRpc },
    }),
  };
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      const body = answers[request.url.pathname];
      return body ? { body } : api.onApi(request);
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector("body:not(.initialization-mode) main.content:not([data-loading])");
  return page;
}

const text = (page, selector) => page.locator(selector).evaluate((node) => node.textContent.trim());

/** Waits until `selector` has been painted from data: its markup placeholder is "—". */
const painted = (page, selector) =>
  page.waitForFunction((css) => {
    const value = document.querySelector(css)?.textContent.trim();
    return value && value !== "—";
  }, selector);

test("no RPC manager: the ticker shows no rate", async (t) => {
  const page = await open(t, {
    headerRpc: { success_rate_percent: null, calls_per_minute: null },
    statusRpc: {},
  });
  await painted(page, "#tickerMonitoringCount");
  assert.equal(await text(page, "#tickerRPCCalls"), "—");
  assert.equal(await text(page, "#tickerRPCSuccess"), "—");
});

test("no calls yet: no success rate anywhere", async (t) => {
  const page = await open(t, {
    headerRpc: { success_rate_percent: null, calls_per_minute: 0 },
    statusRpc: { total_calls: 0, total_errors: 0, success_rate: 100 },
  });
  await painted(page, "#tickerRPCCalls");
  assert.equal(await text(page, "#tickerRPCSuccess"), "—");
  await painted(page, "#statusBarRpcRate");
  assert.equal(await text(page, "#statusBarRpcSuccess"), "—");
  assert.equal(await page.locator("#statusBarRpcHealth").getAttribute("data-health"), "unknown");
});
