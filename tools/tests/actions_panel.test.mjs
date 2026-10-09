// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Actions panel names its controls and shows what its badge counts.
 *
 * The filter clear "×" had no accessible name and stayed enabled with both filters on
 * "All", and the bell badge counted unread actions while the cards showed no unread
 * mark a user could see.
 *
 * - An unread card carries a tint and border that a read card does not.
 * - The clear control is named, disabled while no filter applies and enabled by one.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
const api = createApiHandler([await loadIndex("home"), await loadIndex("shell")], "populated");
const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
after(async () => {
  await context.close();
  await host.close();
});
const page = await context.newPage();
page.setDefaultTimeout(15000);
await page.goto(`${host.origin}/home`);
await page.waitForSelector(READY);
await page.click("#notificationBtn");
await page.waitForSelector('#notificationDrawer[data-state="open"] .notification-item');

// Runs first: opening the panel marks every action read, and the next render shows that.
test("an unread card is marked where a read card is not", async () => {
  const cards = await page.$$eval("#notificationList .notification-item", (items) =>
    items.map((item) => {
      const style = getComputedStyle(item);
      return {
        unread: item.classList.contains("unread"),
        // The background alone was a 5% tint no one could see; the border carries the mark.
        skin: style.borderTopColor,
      };
    })
  );
  const unread = new Set(cards.filter((card) => card.unread).map((card) => card.skin));
  const read = new Set(cards.filter((card) => !card.unread).map((card) => card.skin));
  assert.ok(unread.size > 0 && read.size > 0, "the fixture carries unread and read cards");
  for (const skin of unread) {
    assert.ok(!read.has(skin), "an unread card carries its own border");
  }
});

test("the filter clear control is named and applies only to an active filter", async () => {
  const clear = page.locator("#clearFiltersBtn");
  assert.equal(await clear.getAttribute("aria-label"), "Clear filters");
  assert.equal(await clear.getAttribute("title"), "Clear filters");
  assert.equal(await clear.isDisabled(), true, "disabled while both filters are All");

  await page.$eval("#filterActionType", (select) => {
    select.value = "swap_buy";
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  assert.equal(await clear.isDisabled(), false, "enabled once a filter applies");

  await clear.click();
  assert.equal(await clear.isDisabled(), true, "disabled again after clearing");
});
