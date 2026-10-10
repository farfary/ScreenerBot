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
 * - Every tab whose count is above zero lists cards, also when every action has been
 *   dismissed (completed and failed actions are auto-dismissed).
 * - An empty tab shows the shared state view, its glyph on the message's row.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

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

test("every tab with a count above zero lists its actions, dismissed ones included", async (t) => {
  const history = JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/actions_history.json`, "utf8"));
  for (const action of history.actions) action.dismissed = true;
  const dismissedContext = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const dismissedApi = createApiHandler(
    [await loadIndex("home"), await loadIndex("shell")],
    "populated"
  );
  const dismissedHost = await serveDashboard(dismissedContext, {
    locale: "en",
    onApi: (request) =>
      request.url.pathname === "/api/actions/history"
        ? { body: JSON.stringify(history) }
        : dismissedApi.onApi(request),
  });
  t.after(async () => {
    await dismissedContext.close();
    await dismissedHost.close();
  });
  const panel = await dismissedContext.newPage();
  await panel.goto(`${dismissedHost.origin}/home`);
  await panel.waitForSelector(READY);
  await panel.click("#notificationBtn");
  await panel.waitForSelector('#notificationDrawer[data-state="open"]');

  // Active renders from memory; the history tabs render after their own history read.
  const HISTORY_TABS = new Set(["all", "completed", "failed"]);
  const tabs = ["all", "active", "completed", "failed", "all"];
  const counted = [];
  for (const tab of tabs) {
    const rendered = HISTORY_TABS.has(tab)
      ? panel.waitForResponse((response) => response.url().includes("/api/actions/history"))
      : Promise.resolve();
    await panel.click(`.notification-tab[data-tab="${tab}"]`);
    await rendered;
    await panel.waitForFunction(
      () => getComputedStyle(document.getElementById("notificationLoading")).display === "none"
    );
    const count = Number(await panel.textContent(`#${tab}Count`));
    if (count === 0) continue;
    counted.push(tab);
    const cards = await panel.$$eval(
      "#notificationList .notification-item",
      (items) => items.length
    );
    assert.ok(cards > 0, `the ${tab} tab counts ${count} actions and lists ${cards}`);
  }
  assert.deepEqual(counted, ["all", "completed", "all"], "the fixture counts All and Done");
});

test("an empty tab shows the shared state view with its glyph on the message's row", async (t) => {
  const emptyContext = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const emptyApi = createApiHandler([await loadIndex("home"), await loadIndex("shell")], "empty");
  const emptyHost = await serveDashboard(emptyContext, { locale: "en", onApi: emptyApi.onApi });
  t.after(async () => {
    await emptyContext.close();
    await emptyHost.close();
  });
  const panel = await emptyContext.newPage();
  await panel.goto(`${emptyHost.origin}/home`);
  await panel.waitForSelector(READY);
  // The list holds a static empty view until the open tab's history read re-renders it;
  // measuring before that render reads a node the render detaches.
  const rendered = panel.waitForResponse((response) =>
    response.url().includes("/api/actions/history")
  );
  await panel.click("#notificationBtn");
  await rendered;
  await panel.waitForFunction(
    () => getComputedStyle(document.getElementById("notificationLoading")).display === "none"
  );
  const state = panel.locator("#notificationList > .state-view-empty");
  await state.waitFor();
  assert.equal((await state.locator(".state-view-message").textContent()).trim(), "No actions");
  const [icon, message] = await Promise.all([
    state.locator(".state-view-icon").boundingBox(),
    state.locator(".state-view-message").boundingBox(),
  ]);
  const centre = icon.y + icon.height / 2;
  assert.ok(centre >= message.y && centre <= message.y + message.height);
});
