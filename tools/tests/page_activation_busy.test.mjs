// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the content viewport stays `aria-busy` until the displayed page is active.
 *
 * The router displays a page before its module is imported and its lifecycle runs,
 * so for that interval the page's controls have no handlers and its sub-tab bar does
 * not exist. A click there is dropped, and a check that only waited for the page to
 * be displayed measured a page that was not built yet.
 *
 * - The server-rendered first page shows its markup but keeps the viewport busy
 *   until its module has activated; once the busy state clears, its controls work.
 * - A navigation clears `data-loading` when it displays the page and keeps the
 *   viewport busy until activation has built the page's sub-tab bar, which is then
 *   on screen. The router coordinates the same TabBarManager instance the pages
 *   register with; a second module instance hid every navigated page's sub-tabs.
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

/** Holds the page module's request until the returned release is called. */
async function holdPageModule(context, pageName) {
  let release;
  const released = new Promise((resolve) => {
    release = resolve;
  });
  const requested = new Promise((resolve) => {
    void context.route(`**/scripts/pages/${pageName}.js*`, async (route) => {
      resolve();
      await released;
      await route.continue();
    });
  });
  return { requested, release };
}

function viewportState(page) {
  return page.evaluate(() => {
    const main = document.querySelector("main.content");
    return {
      busy: main.getAttribute("aria-busy"),
      loading: main.hasAttribute("data-loading"),
    };
  });
}

test("the first page stays busy until its module has activated", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("copy"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const held = await holdPageModule(context, "copy");
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/copy`);
  await held.requested;
  await page.waitForSelector("body:not(.initialization-mode) #copy-settings-open");

  assert.deepEqual(await viewportState(page), { busy: "true", loading: false });
  assert.equal(await page.locator(PAGE_READY).count(), 0, "a page without handlers is not ready");

  held.release();
  await page.waitForSelector(PAGE_READY);
  await page.locator("#copy-settings-open").click();
  await page.waitForSelector("#copy-settings:visible");
});

test("a navigation stays busy until activation has built the sub-tab bar", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const indexes = await Promise.all(["home", "trader", "shell"].map(loadIndex));
  const api = createApiHandler(indexes, "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector(PAGE_READY);

  const held = await holdPageModule(context, "trader");
  await page.$eval('#navTabs a.tab[href="/trader"]', (link) => link.click());
  await held.requested;
  assert.deepEqual(await viewportState(page), { busy: "true", loading: true });

  held.release();
  await page.waitForSelector(`${PAGE_READY} #page-trader`);
  assert.ok(
    (await page.locator("#subTabsContainer .sub-tab:visible").count()) > 0,
    "the ready page carries its sub-tab bar"
  );
});
