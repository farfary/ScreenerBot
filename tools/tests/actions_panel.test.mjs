// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Actions panel lists each action as a compact row that says what happened,
 * opens into its detail, and hands Escape to an open dropdown before the drawer.
 *
 * - An unread row carries a border that a read row does not.
 * - Rows sit under a day heading; a settled swap states what it spent and received,
 *   and the swap that closed a position states the round's P&L.
 * - A row opens in place into its step timeline, its transaction and the ways to the
 *   token and the position.
 * - The clear-filters control exists only while the type filter narrows the list, and
 *   the header offers Clear all only on the Active tab it changes, with a tooltip.
 * - Escape with the type dropdown open closes the dropdown and leaves the drawer open;
 *   the next Escape closes the drawer.
 * - Every tab whose count is above zero lists rows, also when every action has been
 *   dismissed (completed and failed actions are auto-dismissed).
 * - An empty tab shows the shared state view, its glyph on the message's row.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

// UTC, so the fixture's actions fall on one calendar day wherever the suite runs.
const context = await browser.newContext({
  viewport: { width: 1440, height: 900 },
  timezoneId: "UTC",
});
const api = createApiHandler([await loadIndex("home"), await loadIndex("shell")], "populated");
const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
after(async () => {
  await context.close();
  await host.close();
});
const page = await context.newPage();
page.setDefaultTimeout(15000);
await page.goto(`${host.origin}/home`);
await page.waitForSelector(PAGE_READY);
await page.click("#notificationBtn");
await page.waitForSelector('#notificationDrawer[data-state="open"] .action-row');

// Fluent isolates every placeable (U+2068/U+2069); the wording is asserted without them.
const plain = (text) => text.replace(/[⁨⁩]/g, "").replace(/\s+/g, " ").trim();
/** A token amount as the rows show it, from the page's own formatter. */
const compact = (value) =>
  page.evaluate(async (amount) => {
    const { formatCompactNumber } = await import("/scripts/core/format.js");
    return formatCompactNumber(amount);
  }, value);
const CLOSE_ID = "3f1c2a9e-7b4d-4e21-9a0c-5d8e1f2a3b01";
const OPEN_ID = "3f1c2a9e-7b4d-4e21-9a0c-5d8e1f2a3b02";
const BUY_ID = "3f1c2a9e-7b4d-4e21-9a0c-5d8e1f2a3b03";

// Runs first: opening the panel marks every action read, and the next render shows that.
test("an unread row is marked where a read row is not", async () => {
  const rows = await page.$$eval("#notificationList .action-row", (items) =>
    items.map((item) => ({
      unread: item.classList.contains("is-unread"),
      skin: getComputedStyle(item).borderTopColor,
    }))
  );
  const unread = new Set(rows.filter((row) => row.unread).map((row) => row.skin));
  const read = new Set(rows.filter((row) => !row.unread).map((row) => row.skin));
  assert.ok(unread.size > 0 && read.size > 0, "the fixture carries unread and read rows");
  for (const skin of unread) {
    assert.ok(!read.has(skin), "an unread row carries its own border");
  }
});

test("rows sit under their day and state what each swap spent and received", async () => {
  const days = await page.$$eval("#notificationList .action-day", (groups) =>
    groups.map((group) => ({
      heading: group.querySelector(".action-day__heading")?.textContent.trim(),
      rows: group.querySelectorAll(".action-row").length,
    }))
  );
  assert.equal(days.length, 1, "the fixture's three actions share one day");
  assert.ok(days[0].heading, "the day has a heading");
  assert.equal(days[0].rows, 3);

  const close = page.locator(`.action-row[data-id="${CLOSE_ID}"]`);
  assert.equal(plain(await close.locator(".action-row__verb").textContent()), "Close");
  assert.equal(plain(await close.locator(".action-row__source").textContent()), "Auto");
  const flowOf = (row) =>
    row
      .locator(".action-row__flow > span")
      .evaluateAll((spans) =>
        spans.map((span) => span.textContent.replace(/[\u2068\u2069]/g, "").trim())
      );
  assert.deepEqual(
    await flowOf(close),
    [`${await compact(12345.678)} WIF`, "0.0061 SOL"],
    "sold, received"
  );
  assert.equal(await close.locator(".action-row__flow .icon-arrow-right").count(), 1);
  assert.equal(await close.locator(".action-row__result .action-row__pnl").count(), 0);
  const pnl = close.locator(".action-row__aside .action-row__pnl");
  assert.equal(plain(await pnl.textContent()), "+0.0011 SOL +22.0%", "the P&L under the time");
  assert.equal(await pnl.locator("*").count(), 0, "amount and percent share one tone");
  assert.match(await pnl.getAttribute("class"), /\bpnl-positive\b/);
  assert.equal(
    await close
      .locator(".action-row__token .ti-logo")
      .evaluate((logo) => logo.closest("[aria-hidden='true']") !== null),
    true,
    "the logo beside the symbol is not announced again"
  );

  const open = page.locator(`.action-row[data-id="${OPEN_ID}"]`);
  assert.deepEqual(
    await flowOf(open),
    ["0.005 SOL", `${await compact(250000)} BONK`],
    "spent, received"
  );
  assert.match(
    (await open.locator(".action-row__time").textContent()).trim(),
    /^\d{1,2}:\d{2}( [AP]M)?$/,
    "an older action shows its time of day"
  );
});

test("a row opens in place into its steps, transaction and links", async () => {
  const row = page.locator(`.action-row[data-id="${CLOSE_ID}"]`);
  const summary = row.locator(".action-row__summary");
  const detail = row.locator(".action-detail");
  assert.equal(await summary.getAttribute("aria-expanded"), "false");
  assert.equal(await detail.isVisible(), false);

  const chevron = summary.locator(".action-row__chevron");
  assert.equal(await chevron.evaluate((glyph) => getComputedStyle(glyph).transform), "none");

  await summary.click();
  assert.equal(await summary.getAttribute("aria-expanded"), "true");
  assert.equal(await detail.isVisible(), true);
  assert.match(await row.getAttribute("class"), /\bis-open\b/, "row and detail share a surface");
  await page.waitForFunction(
    (id) =>
      getComputedStyle(document.querySelector(`.action-row[data-id="${id}"] .action-row__chevron`))
        .transform !== "none",
    CLOSE_ID
  );
  assert.equal(await detail.locator(".action-step").count(), 4, "one entry per step");
  assert.equal(await detail.locator(".action-step--completed").count(), 4);
  assert.ok(
    (await detail.locator(".ti-address-copy").getAttribute("data-copy")).startsWith("5ynthet1c"),
    "the transaction can be copied"
  );
  const facts = plain(await detail.locator(".action-detail__facts").textContent());
  for (const label of ["Fill price", "Network fee", "Position P&L", "Duration", "Reason"]) {
    assert.ok(facts.includes(label), `the detail names its ${label}`);
  }
  assert.equal(await detail.locator("[data-open-token]").count(), 1);
  assert.equal(await detail.locator('[data-open-position="42"]').count(), 1);
  assert.equal(await detail.locator("[data-open-transaction]").count(), 1);

  assert.match(
    await detail.locator(".action-detail__pnl").getAttribute("class"),
    /\bpnl-positive\b/,
    "the detail's P&L is one tone"
  );

  await summary.click();
  assert.equal(await detail.isVisible(), false, "a second click closes it");
  assert.doesNotMatch(await row.getAttribute("class"), /\bis-open\b/);
});

test("a completed swap no position booked says so instead of a success mark", async () => {
  const row = page.locator(`.action-row[data-id="${BUY_ID}"]`);
  assert.equal(await row.locator(".action-row__status.icon-triangle-alert").count(), 1);
  assert.equal(await row.locator(".action-row__status.icon-circle-check").count(), 0);
  assert.equal(
    plain(await row.locator(".action-row__warning").textContent()),
    "Not recorded on position"
  );
  await row.locator(".action-row__summary").click();
  const detail = row.locator(".action-detail");
  assert.match(
    plain(await detail.locator(".action-detail__warning").textContent()),
    /not booked on the position and may not have landed/
  );
  assert.equal(await detail.locator("[data-open-transaction]").count(), 1, "still checkable");

  // The detail's trailing values end where the row's time does.
  const [stepTime, rowTime] = await Promise.all([
    detail.locator(".action-step__time").first().boundingBox(),
    row.locator(".action-row__time").boundingBox(),
  ]);
  assert.ok(
    Math.abs(stepTime.x + stepTime.width - (rowTime.x + rowTime.width)) <= 1,
    `step time ends at ${stepTime.x + stepTime.width}, row time at ${rowTime.x + rowTime.width}`
  );
  await row.locator(".action-row__summary").click();

  const close = page.locator(`.action-row[data-id="${CLOSE_ID}"]`);
  assert.equal(await close.locator(".action-row__status.icon-circle-check").count(), 1);
  assert.equal(await close.locator(".action-row__warning").count(), 0);
});

test("filter and header controls appear only where they act", async () => {
  const clear = page.locator("#clearFiltersBtn");
  assert.equal(await clear.getAttribute("aria-label"), "Clear filters");
  assert.equal(await clear.isVisible(), false, "no clear control while the filter is All");

  const filtered = page.waitForResponse(
    (response) =>
      response.url().includes("/api/actions/history?") &&
      response.url().includes("action_type=sell")
  );
  await page.$eval("#filterActionType", (select) => {
    select.value = "sell";
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await filtered;
  assert.equal(await clear.isVisible(), true, "shown once the filter narrows the list");
  await clear.click();
  assert.equal(await clear.isVisible(), false, "gone again after clearing");

  assert.equal(await page.locator("#markAllReadBtn").count(), 0);
  const clearAll = page.locator("#clearAllBtn");
  assert.equal(await clearAll.isVisible(), false, "Clear all is not offered on All");
  await page.click('.notification-tab[data-tab="active"]');
  assert.equal(await clearAll.isVisible(), true, "Clear all is offered on Active");
  assert.equal(await clearAll.getAttribute("data-tooltip"), "Clear all");
  assert.equal(await clearAll.getAttribute("aria-label"), "Clear all");
  await page.click('.notification-tab[data-tab="all"]');
});

test("the type filter is no taller than the tabs beside it", async () => {
  const [tab, select] = await Promise.all([
    page.locator('.notification-tab[data-tab="all"]').boundingBox(),
    page.locator("#notificationFilters .cs-trigger").boundingBox(),
  ]);
  assert.ok(select.height <= tab.height + 1, `select ${select.height}px, tab ${tab.height}px`);
});

test("Escape closes the open type dropdown first and the drawer on the next press", async () => {
  const dropdown = page.locator("#notificationFilters .custom-select");
  await dropdown.click();
  await page.waitForSelector('#notificationFilters .custom-select[aria-expanded="true"]');

  await page.keyboard.press("Escape");
  assert.equal(await dropdown.getAttribute("aria-expanded"), "false", "the dropdown closed");
  assert.equal(
    await page.getAttribute("#notificationDrawer", "data-state"),
    "open",
    "the drawer stayed open"
  );

  await page.keyboard.press("Escape");
  assert.equal(await page.getAttribute("#notificationDrawer", "data-state"), "closed");
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
  await panel.waitForSelector(PAGE_READY);
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
    const countEl = panel.locator(`#${tab}Count`);
    const count = (await countEl.isVisible()) ? Number(await countEl.textContent()) : 0;
    if (count === 0) continue;
    counted.push(tab);
    const rows = await panel.$$eval("#notificationList .action-row", (items) => items.length);
    assert.ok(rows > 0, `the ${tab} tab counts ${count} actions and lists ${rows}`);
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
  await panel.waitForSelector(PAGE_READY);
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
