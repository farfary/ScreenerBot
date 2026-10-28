// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the strategies roster of the Auto Trader Strategies sub-tab.
 *
 * - The Entry and Exit views each show their own columns, and the view switch counts the
 *   strategies on of each kind.
 * - Every figure is the server's: a figure the server could not attribute reads as one
 *   dash, readiness shows the paper rounds and the rules change they count from, and
 *   positions still open in the other book show under Open.
 * - An exit strategy names the entry strategies it manages, and the Exit view names the
 *   rule exits that are checked before any exit strategy.
 * - A strategy that is off offers no mode action; one that is on offers Arm live on
 *   paper and Return to paper on live.
 * - A switch the server refuses returns to its place, and the toast carries the reason
 *   the server gave.
 * - A refresh that fails keeps the rows on screen and marks them stale; only a first read
 *   that fails shows the error, with Retry.
 * - A trader-level block is stated once, in the toolbar, with the action that lifts it.
 * - A row opens its strategy in the editor; its switch does not.
 * - An empty view offers New Strategy with that view's kind chosen.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { leaf } from "../lib/strategy_rules.mjs";

const OVERVIEW = JSON.parse(
  readFileSync(`${FIXTURES_ROOT}/trader/strategies_overview.json`, "utf8")
);
const MOMENTUM = "8f3c2a1e-4b6d-4c7a-9e21-3a5f7b9d1c01";
const DIP_RECOVERY = "8f3c2a1e-4b6d-4c7a-9e21-3a5f7b9d1c02";
const LOW_LIQUIDITY = "8f3c2a1e-4b6d-4c7a-9e21-3a5f7b9d1c03";
const STALE_EXIT = "8f3c2a1e-4b6d-4c7a-9e21-3a5f7b9d1c04";
const MA_EXIT = "8f3c2a1e-4b6d-4c7a-9e21-3a5f7b9d1c05";
const MOMENTUM_STORED = {
  id: MOMENTUM,
  name: "Momentum breakout",
  description: "Buys a 1h breakout.",
  strategy_type: "ENTRY",
  enabled: true,
  priority: 10,
  timeframe: "15m",
  rules: leaf("Rsi", { period: 21, timeframe: "1h" }),
  parameters: {},
  created_at: "2026-09-10T08:30:00+00:00",
  updated_at: "2026-10-01T12:15:00+00:00",
  author: null,
  version: 1,
  mode: "paper",
  entry_strategy_ids: [],
};

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("trader"), await loadIndex("shell")];

/** Text without the Unicode isolates Fluent places around arguments. */
const plain = (text) => String(text).replace(/[⁨⁩]/g, "");

/**
 * The Strategies sub-tab over the `variant` fixtures. `route(request)` answers a request
 * first when it returns a response.
 */
async function openRoster(t, { variant = "populated", route = () => null } = {}) {
  const context = await browser.newContext({ viewport: { width: 1600, height: 900 } });
  const api = createApiHandler(indexes, variant);
  const onApi = async (request) => (await route(request)) ?? api.onApi(request);
  const host = await serveDashboard(context, { locale: "en", onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/trader`);
  await page.waitForSelector(PAGE_READY);
  await page.click('#subTabsContainer [data-tab-id="strategies"]');
  return page;
}

const view = (page, kind) =>
  page.click(`#strategies-roster label:has(> input[data-seg-value="${kind}"])`);

/** The header texts of the roster table, without the switch and actions columns' blanks. */
const headers = (page) =>
  page.$$eval("#strategies-roster thead th[data-column-id]", (cells) =>
    cells.map((cell) => [cell.dataset.columnId, cell.textContent.replace(/[⁨⁩]/g, "").trim()])
  );

/** The rendered text of one cell, its lines joined by single spaces. */
const cell = async (page, id, column) =>
  plain(
    await page.innerText(`#strategies-roster tr[data-row-id="${id}"] td[data-column-id="${column}"]`)
  )
    .replace(/\s+/g, " ")
    .trim();

/** The ids of the mode and row actions a row offers. */
const actions = (page, id) =>
  page.$$eval(`#strategies-roster tr[data-row-id="${id}"] [data-action-id]`, (buttons) =>
    buttons.map((button) => button.dataset.actionId)
  );

test("the Entry and Exit views show their own columns and count the strategies on", async (t) => {
  const page = await openRoster(t);
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`);
  assert.deepEqual(
    (await headers(page)).filter(([id]) => id !== "actions"),
    [
      ["on", "On"],
      ["name", "Strategy"],
      ["order", "Order"],
      ["mode", "Mode"],
      ["status", "Status"],
      ["open", "Open"],
      ["rounds", "Rounds"],
      ["win_rate", "Win rate"],
      ["realized", "Realized SOL"],
      ["last_signal", "Last signal"],
    ]
  );
  assert.equal(await page.locator(`#strategies-roster tr[data-row-id="${STALE_EXIT}"]`).count(), 0);
  const counts = await page.$$eval("#strategies-roster .segmented-option", (options) =>
    options.map((option) => [
      option.querySelector("input").dataset.segValue,
      option.querySelector(".segmented-count")?.textContent.trim(),
    ])
  );
  assert.deepEqual(counts, [
    ["entry", "2/3"],
    ["exit", "1/2"],
  ]);

  await view(page, "exit");
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${STALE_EXIT}"]`);
  assert.deepEqual(
    (await headers(page)).filter(([id]) => id !== "actions"),
    [
      ["on", "On"],
      ["name", "Strategy"],
      ["manages", "Manages"],
      ["mode", "Mode"],
      ["status", "Status"],
      ["watching", "Watching"],
      ["closed", "Closed"],
      ["win_rate", "Win rate"],
      ["realized", "Realized SOL"],
      ["last_signal", "Last signal"],
    ]
  );
  assert.equal(await page.locator(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`).count(), 0);
});

test("every figure is the server's, and one it could not attribute reads as a dash", async (t) => {
  const page = await openRoster(t);
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`);

  assert.match(await cell(page, MOMENTUM, "mode"), /^Paper Paper rounds 3\/10 Rules changed .+2026/);
  assert.equal(await cell(page, MOMENTUM, "order"), "1");
  assert.equal(await cell(page, MOMENTUM, "status"), "Running");
  assert.equal(await cell(page, MOMENTUM, "rounds"), "3");
  assert.equal(await cell(page, DIP_RECOVERY, "open"), "2 Paper: 1 position");
  assert.equal(await cell(page, DIP_RECOVERY, "mode"), "Live", "a live strategy shows no readiness");
  assert.equal(await cell(page, LOW_LIQUIDITY, "order"), "—", "an off strategy has no rank");
  assert.equal(await cell(page, LOW_LIQUIDITY, "status"), "Off");
  assert.equal(await cell(page, LOW_LIQUIDITY, "last_signal"), "—");
  assert.equal(
    await page.locator(`#strategies-roster tr[data-row-id="${LOW_LIQUIDITY}"] .strategy-roster-description`).count(),
    0,
    "no placeholder for a missing description"
  );

  await view(page, "exit");
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${STALE_EXIT}"]`);
  assert.equal(await cell(page, STALE_EXIT, "manages"), "All entry strategies");
  assert.equal(await cell(page, MA_EXIT, "manages"), "Momentum breakout and 1 more");
  assert.equal(await cell(page, STALE_EXIT, "watching"), "3");
  assert.equal(await cell(page, STALE_EXIT, "closed"), "6");
  assert.equal(await cell(page, MA_EXIT, "watching"), "—", "an off exit strategy watches nothing");
});

test("the Exit view names the rule exits checked before any exit strategy", async (t) => {
  const page = await openRoster(t);
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`);
  const chip = page.locator('#strategies-roster .table-toolbar-chip[data-summary-id="rule-exits"]');
  assert.equal(await chip.isHidden(), true, "the Entry view leaves the rule exits out");
  await view(page, "exit");
  await chip.waitFor();
  assert.match(plain(await chip.textContent()), /Checked first\s*Stop Loss, Take Profit,? and Time Exit/);
  assert.equal(
    await chip.getAttribute("title"),
    "These exit rules are checked on every position before any exit strategy"
  );
});

test("only a strategy that is on offers the action that changes its mode", async (t) => {
  const page = await openRoster(t);
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`);
  assert.deepEqual(await actions(page, MOMENTUM), ["edit", "arm", "delete"]);
  assert.deepEqual(await actions(page, DIP_RECOVERY), ["edit", "return-paper", "delete"]);
  assert.deepEqual(await actions(page, LOW_LIQUIDITY), ["edit", "delete"]);

  await view(page, "exit");
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${STALE_EXIT}"]`);
  assert.deepEqual(await actions(page, STALE_EXIT), ["edit", "return-paper", "delete"]);
  assert.deepEqual(await actions(page, MA_EXIT), ["edit", "delete"]);
});

test("a switch the server refuses returns, and the toast carries its reason", async (t) => {
  const page = await openRoster(t, {
    route: (request) =>
      request.method === "PATCH" && request.url.pathname === `/api/strategies/${LOW_LIQUIDITY}/enabled`
        ? {
            status: 400,
            body: JSON.stringify({ error: { message: "A group needs at least one condition switched on" } }),
          }
        : null,
  });
  const input = page.locator(`#strategies-roster input[data-enable-strategy="${LOW_LIQUIDITY}"]`);
  await input.waitFor({ state: "attached" });
  assert.equal(await input.isChecked(), false);
  await page.click(`#strategies-roster label:has(> input[data-enable-strategy="${LOW_LIQUIDITY}"])`);
  const toast = page.locator(".toast", { hasText: "Failed to toggle strategy" });
  await toast.waitFor();
  assert.match(plain(await toast.innerText()), /A group needs at least one condition switched on/);
  assert.equal(await input.isChecked(), false, "the switch returns to off");
  assert.equal(await input.isDisabled(), false);
  assert.equal(await page.locator("#strategy-editor").isHidden(), true, "the switch opens nothing");
});

test("a failed refresh keeps the rows and marks them stale", async (t) => {
  let failing = false;
  const page = await openRoster(t, {
    route: (request) => {
      if (request.method === "PATCH") return { body: JSON.stringify({ success: true }) };
      if (failing && request.url.pathname === "/api/strategies/overview") {
        return { status: 500, body: JSON.stringify({ error: { message: "Store unavailable" } }) };
      }
      return null;
    },
  });
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`);
  const stale = page.locator('#strategies-roster .table-toolbar-chip[data-summary-id="stale"]');
  assert.equal(await stale.isHidden(), true);

  failing = true;
  // A stored switch reads the overview again, which now fails.
  await page.click(`#strategies-roster label:has(> input[data-enable-strategy="${DIP_RECOVERY}"])`);
  await stale.waitFor();
  assert.equal(await stale.getAttribute("data-variant"), "warning");
  assert.equal(await page.locator("#strategies-roster tbody tr[data-row-id]").count(), 3);
  assert.equal(await page.locator("#strategies-roster .state-view-empty").count(), 0);
});

test("a first read that fails shows the error with Retry", async (t) => {
  let failing = true;
  const page = await openRoster(t, {
    route: (request) =>
      failing && request.url.pathname === "/api/strategies/overview"
        ? { status: 500, body: JSON.stringify({ error: { message: "Store unavailable" } }) }
        : null,
  });
  const empty = page.locator("#strategies-roster .state-view-empty");
  await empty.waitFor();
  assert.match(plain(await empty.textContent()), /Load Failed/);
  failing = false;
  await empty.locator('[data-action="table-empty-action"]', { hasText: "Retry" }).click();
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`);
});

/** The roster over the recorded overview with `notice` as the trader-level block. */
const withNotice = (notice) => (request) =>
  request.url.pathname === "/api/strategies/overview"
    ? { body: JSON.stringify({ ...OVERVIEW, run_state: "stopped", notice }) }
    : null;

test("a trader-level block is stated once in the toolbar, with the action that lifts it", async (t) => {
  const page = await openRoster(t, { route: withNotice("trader_off") });
  const chip = page.locator('#strategies-roster .table-toolbar-chip[data-summary-id="run-notice"]');
  await chip.waitFor();
  assert.match(plain(await chip.textContent()), /Not running\s*Auto Trader is off/);
  assert.equal(await chip.getAttribute("data-variant"), "warning");
  const action = page.locator('#strategies-roster [data-btn-id="run-notice-action"]');
  assert.equal(plain((await action.textContent()).trim()), "Start Auto Trader");
  assert.equal(await action.isVisible(), true);
  assert.equal(
    await page.locator("#strategies-tab .state-view-warning, #strategies-tab .notice").count(),
    0,
    "no notice of the page's own"
  );
});

test("an install without setup offers to complete it from the toolbar", async (t) => {
  const page = await openRoster(t, { route: withNotice("setup_required") });
  const chip = page.locator('#strategies-roster .table-toolbar-chip[data-summary-id="run-notice"]');
  await chip.waitFor();
  assert.match(plain(await chip.textContent()), /Paper trading starts after setup/);
  assert.equal(
    plain((await page.textContent('#strategies-roster [data-btn-id="run-notice-action"]')).trim()),
    "Complete setup"
  );
});

test("a row opens its strategy in the editor and the back control returns", async (t) => {
  const page = await openRoster(t, {
    route: (request) =>
      request.url.pathname === `/api/strategies/${MOMENTUM}`
        ? { body: JSON.stringify(MOMENTUM_STORED) }
        : null,
  });
  await page.click(`#strategies-roster tr[data-row-id="${MOMENTUM}"] td[data-column-id="status"]`);
  await page.waitForFunction(
    (name) => document.querySelector("#strategy-name")?.value === name,
    MOMENTUM_STORED.name
  );
  assert.equal(await page.locator("#strategies-roster").isHidden(), true);
  assert.equal(await page.isChecked("#strategy-enabled-toggle"), true);
  assert.equal(plain(await page.textContent("label.editor-active")).trim(), "On");

  await page.click("#strategy-back");
  await page.locator("#strategies-roster").waitFor({ state: "visible" });
  assert.equal(await page.locator("#strategy-editor").isHidden(), true);
});

test("an empty view offers New Strategy with its kind chosen", async (t) => {
  const page = await openRoster(t, { variant: "empty" });
  const action = page.locator('#strategies-roster .state-view-empty [data-action="table-empty-action"]');
  await action.waitFor();
  assert.equal((await action.textContent()).trim(), "New Strategy");
  await view(page, "exit");
  await action.click();
  await page.locator("#create-strategy-modal:not([hidden])").waitFor();
  assert.deepEqual(
    await page.$$eval("#create-strategy-modal .type-card", (cards) =>
      cards.map((card) => [card.dataset.type, card.getAttribute("aria-pressed")])
    ),
    [
      ["ENTRY", "false"],
      ["EXIT", "true"],
    ]
  );
});
