// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the closed notification drawer holds no keyboard stop.
 *
 * The closed drawer is only translated off-screen, so its buttons stayed focusable and
 * keyboard Tab landed on invisible controls between the header and the page. The drawer
 * is `inert` from the first paint and again the moment it closes, not after its slide
 * transition, and closing it returns focus to the control that opened it.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [shell, home] = await Promise.all([loadIndex("shell"), loadIndex("home")]);

/** Every element keyboard Tab visits, in order, until focus leaves the document. */
async function tabStops(page, limit = 200) {
  await page.evaluate(() => document.activeElement?.blur());
  const stops = [];
  for (let index = 0; index < limit; index += 1) {
    await page.keyboard.press("Tab");
    const stop = await page.evaluate(() => {
      const el = document.activeElement;
      if (!el || el === document.body) return null;
      return {
        id: el.id,
        inDrawer: Boolean(el.closest("#notificationDrawer")),
      };
    });
    if (!stop) break;
    stops.push(stop);
  }
  return stops;
}

test("the closed drawer is inert and closing it returns focus to its trigger", async () => {
  const context = await browser.newContext({ viewport: { width: 1600, height: 900 } });
  const fixtures = createApiHandler([home, shell], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: fixtures.onApi });
  try {
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/home`);
    await page.waitForSelector("#navTabs > a.tab.active");

    const drawer = page.locator("#notificationDrawer");
    assert.equal(await drawer.evaluate((el) => el.inert), true, "closed at first paint");
    const before = await tabStops(page);
    assert.ok(before.length > 0, "the page has keyboard stops");
    assert.deepEqual(
      before.filter((stop) => stop.inDrawer),
      [],
      "no Tab stop inside the closed drawer"
    );

    const trigger = page.locator("#notificationBtn");
    await trigger.focus();
    await page.keyboard.press("Enter");
    await page.waitForSelector('#notificationDrawer[data-state="open"]');
    assert.equal(await drawer.evaluate((el) => el.inert), false, "the open drawer is live");

    await page.locator("#notificationDrawerClose").focus();
    await page.keyboard.press("Enter");
    // Inert in the same task that marks it closed, before any transition ends.
    const closed = await drawer.evaluate((el) => ({
      state: el.dataset.state,
      inert: el.inert,
      focus: document.activeElement?.id,
    }));
    assert.deepEqual(closed, { state: "closed", inert: true, focus: "notificationBtn" });
  } finally {
    await context.close();
    await host.close();
  }
});
