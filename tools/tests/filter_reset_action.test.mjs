// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a table's filter reset says what it resets and is offered only while there
 * is something to reset.
 *
 * Transactions and Events once showed an always-enabled "Reset" beside their filters,
 * which did not say it resets the filters and stayed live with every filter at its
 * default. Each reads "Reset filters", is disabled until a filter or the search
 * differs from its default, and is disabled again once it has reset them.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const PAGES = [
  { page: "transactions", filter: "status", value: "Failed" },
  { page: "events", filter: "severity", value: "error" },
];

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const shell = await loadIndex("shell");

for (const { page: id, filter, value } of PAGES) {
  test(`${id}: Reset filters follows the filters`, async (t) => {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    const api = createApiHandler([await loadIndex(id), shell], "populated");
    const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
    t.after(async () => {
      await context.close();
      await host.close();
    });
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/${id}`);
    await page.waitForSelector(PAGE_READY);

    const reset = page.locator('[data-btn-id="reset"]');
    await reset.waitFor();
    assert.equal((await reset.textContent()).trim(), "Reset filters");
    assert.equal(await reset.isDisabled(), true, "nothing to reset at the defaults");

    // The native select sits under the custom dropdown; choosing through it is a change.
    await page.$eval(
      `#tt-filter-${filter}`,
      (select, chosen) => {
        select.value = chosen;
        select.dispatchEvent(new Event("change", { bubbles: true }));
      },
      value
    );
    await page.waitForFunction(() => !document.querySelector('[data-btn-id="reset"]').disabled);

    await reset.click();
    await page.waitForFunction(() => document.querySelector('[data-btn-id="reset"]').disabled);
    assert.notEqual(await page.inputValue(`#tt-filter-${filter}`), value);
  });
}
