// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the dashboard opens on the configured Default Page.
 *
 * The root `/` is server-rendered as Settings > Startup > Default Page
 * (`DashboardConfig::startup_page`). The router must open that page as rendered,
 * keep the address at `/`, and never let a remembered last tab override it.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_IDS, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const shell = await loadIndex("shell");

for (const startupPage of ["home", "positions", "transactions"]) {
  test(`the root opens the Default Page "${startupPage}" over a remembered tab`, async (t) => {
    assert.ok(PAGE_IDS.includes(startupPage));
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    const api = createApiHandler([await loadIndex(startupPage), shell], "populated");
    const onApi = (request) =>
      request.url.pathname === "/api/ui-state/all"
        ? { body: JSON.stringify({ theme: "dark", lastTab: "tokens" }) }
        : api.onApi(request);
    const host = await serveDashboard(context, { locale: "en", onApi, startupPage });
    t.after(async () => {
      await context.close();
      await host.close();
    });
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/`);
    await page.waitForSelector(PAGE_READY);
    await page.waitForSelector(`main.content .page-container[data-page="${startupPage}"]`);

    const opened = await page.evaluate(() => ({
      path: globalThis.location.pathname,
      active: [...globalThis.document.querySelectorAll("nav .tab.active")].map(
        (tab) => tab.dataset.page
      ),
      pages: [...globalThis.document.querySelectorAll("main.content .page-container")].map(
        (container) => container.dataset.page
      ),
    }));
    assert.deepEqual(opened, { path: "/", active: [startupPage], pages: [startupPage] });
  });
}
