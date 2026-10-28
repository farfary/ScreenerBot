// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Assistant Instructions list says so when a search or filter hides every
 * instruction, and drops that message as soon as something matches again.
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
const indexes = [await loadIndex("assistant"), await loadIndex("shell")];

test("filtering every instruction out shows the no-match state until the search clears", async () => {
  const context = await browser.newContext({ viewport: { width: 1200, height: 780 } });
  const api = createApiHandler(indexes, "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  try {
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/assistant#instructions`);
    await page.waitForSelector(PAGE_READY);
    await page.waitForSelector("#instructions-list .instruction-item");
    const total = await page.locator("#instructions-list .instruction-item").count();
    assert.equal(await page.locator(".instructions-no-match").count(), 0);

    await page.fill("#instructions-search", "no instruction is named like this");
    await page.waitForSelector(".instructions-no-match");
    assert.match(await page.textContent(".instructions-no-match"), /No results found/);
    assert.equal(await page.locator("#instructions-list .instruction-item:visible").count(), 0);

    await page.fill("#instructions-search", "");
    await page.waitForSelector(".instructions-no-match", { state: "detached" });
    assert.equal(await page.locator("#instructions-list .instruction-item:visible").count(), total);
  } finally {
    await context.close();
    await host.close();
  }
});
