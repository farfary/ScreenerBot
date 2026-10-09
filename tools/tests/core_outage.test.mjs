// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: while the core is unreachable the dashboard has one coherent outage surface.
 * The live figures (the header ticker's counts, P&L, RPC and Services, and the status
 * bar) read "—" instead of their last values, the open page keeps its shell, a page
 * opened during the outage stays in its loading state rather than adding a second
 * outage card, and everything refills once the core answers again.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";
const OVERLAY = ".conn-overlay.is-visible";
const ISOLATES = /[⁦-⁩]/g;

function liveFigures(page) {
  return page.evaluate((isolates) => {
    const text = (selector) =>
      document.querySelector(selector)?.textContent.replace(new RegExp(isolates, "g"), "").trim();
    return {
      ticker: [
        "#tickerMonitoringCount",
        "#tickerPassedCount",
        "#tickerRejectedCount",
        "#tickerTodayPnL",
        "#tickerRPCCalls",
        "#tickerRPCSuccess",
      ].map(text),
      services: text("#tickerServicesLine"),
      dot: document.querySelector("#tickerServicesText .status-dot").className,
      bar: [
        "#statusBarUptime",
        "#statusBarRpcRate",
        "#statusBarRpcSuccess",
        "#statusBarRpcLatency",
        "#statusBarTrading",
        "#statusBarPositions",
      ].map(text),
      health: document.getElementById("statusBarRpcHealth")?.dataset.health,
    };
  }, ISOLATES.source);
}

test("an unreachable core blanks the live figures and keeps one outage card", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const indexes = await Promise.all(["trader", "positions", "shell"].map(loadIndex));
  const api = createApiHandler(indexes, "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(() => host.close());
  const page = await context.newPage();
  page.setDefaultTimeout(20000);
  await page.goto(`${host.origin}/trader`);
  await page.waitForSelector(READY);
  await page.waitForFunction(
    () =>
      document.querySelector("#tickerServicesLine")?.textContent.includes("Healthy") &&
      /\d/.test(document.querySelector("#statusBarRpcSuccess")?.textContent ?? "")
  );
  const subTabs = await page.locator(".sub-tab").count();
  assert.ok(subTabs > 0, "Auto Trader renders its sub-tabs");

  const outage = (route) => route.abort("connectionrefused");
  await context.route(`${host.origin}/api/**`, outage);
  await page.waitForSelector(OVERLAY);

  const down = await liveFigures(page);
  assert.deepEqual(down.ticker, Array(6).fill("—"), "the ticker shows no last values");
  assert.equal(down.services, "Services: —");
  assert.match(down.dot, /\bunknown\b/);
  assert.deepEqual(down.bar, Array(6).fill("—"), "the status bar shows no last values");
  assert.equal(down.health, "unknown");
  assert.equal(await page.locator(".sub-tab").count(), subTabs, "the open page keeps its shell");

  await page.$eval('#navTabs a.tab[href="/positions"]', (link) => link.click());
  await page.waitForSelector("main.content > .page-loading .loading-spinner");
  const notices = await page.evaluate(
    () =>
      [...document.querySelectorAll("body *")].filter(
        (element) =>
          element.getClientRects().length &&
          !element.children.length &&
          element.textContent.includes("Waiting for core")
      ).length
  );
  assert.equal(notices, 1, "the overlay is the only outage notice");

  await context.unroute(`${host.origin}/api/**`, outage);
  await page.waitForFunction(() => !document.querySelector(".conn-overlay.is-visible"));
  await page.waitForSelector(`${READY} #page-positions, ${READY} [data-page="positions"]`);
  await page.waitForFunction(
    () =>
      document.querySelector("#tickerServicesLine")?.textContent.includes("Healthy") &&
      /\d/.test(document.querySelector("#statusBarRpcSuccess")?.textContent ?? "")
  );
});
