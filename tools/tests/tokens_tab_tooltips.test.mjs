// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Tokens sub-tabs explain themselves in their own tooltip, never with a
 * help glyph squeezed into the tab.
 *
 * Every view tab but Favorites carries plain-text help (no markdown markers) that
 * the shared tooltip shows on hover, in every locale, and the tab strip holds no
 * hint trigger.
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

for (const locale of ["en", "de", "fa"]) {
  test(`Tokens sub-tabs carry their help as a tooltip (${locale})`, async () => {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    const api = createApiHandler(indexes, "populated");
    const host = await serveDashboard(context, { locale, onApi: api.onApi });
    try {
      const page = await context.newPage();
      page.setDefaultTimeout(15000);
      await page.goto(`${host.origin}/tokens`);
      await page.waitForSelector(PAGE_READY);
      await page.waitForSelector('#subTabsContainer [data-tab-id="pool"]');

      const tabs = await page.evaluate(() =>
        [...document.querySelectorAll("#subTabsContainer .sub-tab")].map((tab) => ({
          id: tab.dataset.tabId,
          tooltip: tab.dataset.tooltip ?? null,
        }))
      );
      assert.equal(tabs.length, 10);
      for (const tab of tabs) {
        if (tab.id === "favorites") {
          assert.equal(tab.tooltip, null);
          continue;
        }
        assert.ok(tab.tooltip && tab.tooltip.length > 20, `${tab.id} has help text`);
        assert.doesNotMatch(tab.tooltip, /\*\*/, `${tab.id} help is plain text`);
      }
      assert.equal(await page.locator("#subTabsContainer .hint-trigger").count(), 0);

      const passed = tabs.find((tab) => tab.id === "passed");
      await page.hover('#subTabsContainer [data-tab-id="passed"]');
      const tooltip = page.locator(".app-tooltip:not([hidden])");
      await tooltip.waitFor();
      assert.equal((await tooltip.textContent()).trim(), passed.tooltip.trim());
    } finally {
      await context.close();
      await host.close();
    }
  });
}
