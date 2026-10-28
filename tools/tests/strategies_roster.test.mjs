// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the strategies roster of the Auto Trader Strategies sub-tab.
 *
 * - All, the first view, shows every strategy with its kind, entries before exits, and the
 *   columns every kind shares; the Entry and Exit views each add their own columns. The
 *   view switch counts the strategies on of each view.
 * - Every figure is the server's: a figure the server could not attribute reads as one
 *   dash, readiness shows the paper rounds on one line with the rules change they count
 *   from and any check not passing in its tooltip, and positions still open in the
 *   other book show under Open.
 * - An exit strategy names the entry strategies it manages, and a view holding exit
 *   strategies names the rule exits that are checked before any of them.
 * - A strategy that is off offers no mode action; one that is on offers Arm live on
 *   paper and Return to paper on live, ahead of Edit and Delete. The arm dialog grows
 *   sideways before it scrolls, asks only for ticks, and opens at its top, so the readiness
 *   checks are read before the acknowledgements.
 * - A switch the server refuses returns to its place, and the toast carries the reason
 *   the server gave.
 * - A refresh that fails keeps the rows on screen and marks them stale, and a switch the
 *   server stored before it is released; only a first read that fails shows the error,
 *   with Retry.
 * - A trader-level block is stated once, in the toolbar, with the action that lifts it.
 * - A row opens its strategy in the editor; its switch does not.
 * - An empty view offers New Strategy with that view's kind chosen; All chooses none and
 *   focuses the first kind, and Escape closes the dialog back to its opener.
 * - At a laptop width the view switch keeps one line, every row has the same height,
 *   and every row action sits inside its cell with Edit and Delete lined up down the
 *   column, whatever number of actions a row offers.
 * - The columns follow the window: narrower, the table fits without scrolling sideways;
 *   wider again, it fills the panel. A row action that appears after the fit keeps the
 *   table inside the panel.
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
async function openRoster(
  t,
  { variant = "populated", route = () => null, viewport = { width: 1600, height: 900 } } = {}
) {
  const context = await browser.newContext({ viewport });
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

test("All shows every strategy with its kind, and each kind view adds its own columns", async (t) => {
  const page = await openRoster(t);
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`);
  assert.deepEqual(
    (await headers(page)).filter(([id]) => id !== "actions"),
    [
      ["on", "On"],
      ["name", "Strategy"],
      ["kind", "Kind"],
      ["mode", "Mode"],
      ["status", "Status"],
      ["rounds", "Rounds"],
      ["win_rate", "Win rate"],
      ["realized", "Realized (SOL)"],
      ["last_signal", "Last signal"],
    ]
  );
  assert.equal(await page.locator("#strategies-roster tbody tr[data-row-id]").count(), 5);
  assert.equal(await cell(page, MOMENTUM, "kind"), "Entry");
  assert.equal(await cell(page, STALE_EXIT, "kind"), "Exit");
  const counts = await page.$$eval("#strategies-roster .segmented-option", (options) =>
    options.map((option) => [
      option.querySelector("input").dataset.segValue,
      option.querySelector(".segmented-count")?.textContent.trim(),
    ])
  );
  assert.deepEqual(counts, [
    ["all", "3/5"],
    ["entry", "2/3"],
    ["exit", "1/2"],
  ]);

  await view(page, "entry");
  await page.waitForFunction(
    (id) => !document.querySelector(`#strategies-roster tr[data-row-id="${id}"]`),
    STALE_EXIT
  );
  assert.deepEqual(
    (await headers(page)).filter(([id]) => id !== "actions"),
    [
      ["on", "On"],
      ["name", "Strategy"],
      ["order", "Rank"],
      ["mode", "Mode"],
      ["status", "Status"],
      ["open", "Open"],
      ["rounds", "Rounds"],
      ["win_rate", "Win rate"],
      ["realized", "Realized (SOL)"],
      ["last_signal", "Last signal"],
    ]
  );

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
      ["realized", "Realized (SOL)"],
      ["last_signal", "Last signal"],
    ]
  );
  assert.equal(await page.locator(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`).count(), 0);
});

test("All lists the entry strategies before the exit strategies, each in the server's order", async (t) => {
  const exitsFirst = JSON.parse(JSON.stringify(OVERVIEW));
  exitsFirst.strategies.reverse();
  const page = await openRoster(t, {
    route: (request) =>
      request.url.pathname === "/api/strategies/overview" ? { body: JSON.stringify(exitsFirst) } : null,
  });
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`);
  assert.deepEqual(
    await page.$$eval("#strategies-roster tbody tr[data-row-id]", (trs) => trs.map((tr) => tr.dataset.rowId)),
    [LOW_LIQUIDITY, DIP_RECOVERY, MOMENTUM, MA_EXIT, STALE_EXIT]
  );
});

test("every figure is the server's, and one it could not attribute reads as a dash", async (t) => {
  const page = await openRoster(t);
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`);
  await view(page, "entry");
  await page.waitForSelector(`#strategies-roster td[data-column-id="order"]`);

  assert.equal(await cell(page, MOMENTUM, "mode"), "Paper 3/10 rounds");
  assert.match(
    plain(
      await page.getAttribute(
        `#strategies-roster tr[data-row-id="${MOMENTUM}"] .strategy-roster-readiness`,
        "title"
      )
    ),
    /^3 of 10 closed paper rounds since the rules changed .+2026\b/
  );
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

test("a view holding exit strategies names the rule exits checked before them", async (t) => {
  const page = await openRoster(t);
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`);
  const chip = page.locator('#strategies-roster .table-toolbar-chip[data-summary-id="rule-exits"]');
  await chip.waitFor();
  assert.match(plain(await chip.textContent()), /Checked first\s*Stop Loss, Take Profit,? and Time Exit/);
  assert.equal(
    await chip.getAttribute("title"),
    "These exit rules are checked on every position before any exit strategy"
  );
  await view(page, "entry");
  await chip.waitFor({ state: "hidden" });
  await view(page, "exit");
  await chip.waitFor();
});

test("only a strategy that is on offers the action that changes its mode", async (t) => {
  const page = await openRoster(t);
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`);
  assert.deepEqual(await actions(page, MOMENTUM), ["arm", "edit", "delete"]);
  assert.deepEqual(await actions(page, DIP_RECOVERY), ["return-paper", "edit", "delete"]);
  assert.deepEqual(await actions(page, LOW_LIQUIDITY), ["edit", "delete"]);
  assert.deepEqual(await actions(page, STALE_EXIT), ["return-paper", "edit", "delete"]);
  assert.deepEqual(await actions(page, MA_EXIT), ["edit", "delete"]);
});

/** The roster at `viewport` with Momentum's readiness unready on its rounds and profit. */
async function openArmDialog(t, viewport) {
  const readinessCheck = (id, passed) => ({
    id,
    passed,
    text: { id: "paper-readiness-exits", args: {} },
    detail: { id: "paper-readiness-exits-ok", args: {} },
  });
  const readiness = {
    ready: false,
    checks: ["history", "profit", "priced", "exits", "runtime"].map((id, index) =>
      readinessCheck(id, index > 1)
    ),
    live_confirmation: "ARM LIVE STRATEGY",
  };
  const page = await openRoster(t, {
    viewport,
    route: (request) =>
      request.url.pathname === `/api/strategies/${MOMENTUM}/readiness`
        ? { body: JSON.stringify(readiness) }
        : null,
  });
  await page.click(`#strategies-roster tr[data-row-id="${MOMENTUM}"] [data-action-id="arm"]`);
  await page.waitForFunction(() => document.activeElement?.matches?.("[data-ack]"));
  return page;
}

/** How the open dialog's body sits: whether it scrolls, and where. */
const dialogBody = (page) =>
  page.$eval(".confirmation-dialog__body", (element) => ({
    overflows: element.scrollHeight > element.clientHeight,
    top: element.scrollTop,
  }));

test("at a laptop height the arm dialog shows every check and acknowledgement unscrolled", async (t) => {
  const page = await openArmDialog(t, { width: 1280, height: 660 });
  assert.deepEqual(await dialogBody(page), { overflows: false, top: 0 });
  const width = await page.$eval(".confirmation-dialog", (dialog) => dialog.getBoundingClientRect().width);
  assert.ok(width > 600, `the dialog grows sideways before it scrolls, got ${width}px`);
  const columns = await page.$$eval(".arm-gate-check", (checks) =>
    new Set(checks.map((check) => Math.round(check.getBoundingClientRect().left))).size
  );
  assert.equal(columns, 2, "the checks fill two columns");
  assert.equal(await page.locator(".confirmation-dialog input[type=text]").count(), 0, "nothing is typed");
  const confirm = page.locator('.confirmation-dialog [data-action="confirm"]');
  assert.equal(await confirm.isDisabled(), true);
  for (const ack of await page.locator(".confirmation-dialog [data-ack]").all()) await ack.check();
  assert.equal(await confirm.isDisabled(), false, "ticking every acknowledgement allows Arm live");
  await page.click('.confirmation-dialog [data-action="cancel"]');
});

test("in a short window the arm dialog still opens at its top", async (t) => {
  const page = await openArmDialog(t, { width: 1280, height: 420 });
  assert.deepEqual(
    await dialogBody(page),
    { overflows: true, top: 0 },
    "the first acknowledgement takes the focus in place"
  );
  await page.click('.confirmation-dialog [data-action="cancel"]');
});

test("at a laptop width the view switch keeps one line, rows share a height and every row action fits its cell", async (t) => {
  const page = await openRoster(t, { viewport: { width: 1200, height: 800 } });
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`);
  const optionTops = await page.$$eval("#strategies-roster .segmented-option", (options) =>
    options.map((option) => Math.round(option.getBoundingClientRect().top))
  );
  assert.equal(new Set(optionTops).size, 1, "the view options share one line");

  const spilled = await page.$$eval('#strategies-roster td[data-column-id="actions"]', (cells) =>
    cells.flatMap((cell) => {
      const { left, right } = cell.getBoundingClientRect();
      return Array.from(cell.querySelectorAll("[data-action-id]"))
        .filter((button) => {
          const box = button.getBoundingClientRect();
          return box.left < left - 0.5 || box.right > right + 0.5;
        })
        .map((button) => `${cell.closest("tr").dataset.rowId} ${button.dataset.actionId}`);
    })
  );
  assert.deepEqual(spilled, []);

  const rows = await page.$$eval("#strategies-roster tbody tr[data-row-id]", (trs) =>
    trs.map((tr) => Math.round(tr.getBoundingClientRect().height))
  );
  assert.equal(new Set(rows).size, 1, `every row has one height, got ${rows.join(", ")}`);

  const edges = await page.$$eval("#strategies-roster tbody tr[data-row-id]", (trs) =>
    ["edit", "delete"].map(
      (id) =>
        new Set(
          trs.map((tr) =>
            Math.round(tr.querySelector(`[data-action-id="${id}"]`).getBoundingClientRect().left)
          )
        ).size
    )
  );
  assert.deepEqual(edges, [1, 1], "Edit and Delete line up down the column");
});

/**
 * True once the roster's body table is as wide as its scroll container, which must be
 * wider than `minWidth`: no column cut off behind a sideways scroll, no blank band.
 */
const fillsContainer = (minWidth) => {
  const scroller = document.querySelector("#strategies-roster .data-table-scroll-container");
  const table = scroller?.querySelector("table.data-table");
  if (!table || scroller.clientWidth <= minWidth) return false;
  return Math.abs(table.getBoundingClientRect().width - scroller.clientWidth) < 1;
};

test("the columns follow the window: narrower they fit, wider they fill the panel", async (t) => {
  const page = await openRoster(t);
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`);
  await page.waitForFunction(fillsContainer, 1500);

  await page.setViewportSize({ width: 1250, height: 900 });
  await page.waitForFunction(fillsContainer, 1150);

  await page.setViewportSize({ width: 1800, height: 900 });
  await page.waitForFunction(fillsContainer, 1700);
});

test("a row action that appears after the fit keeps the table inside its panel", async (t) => {
  const allOff = JSON.parse(JSON.stringify(OVERVIEW));
  for (const entry of allOff.strategies) entry.item.enabled = false;
  let switchedOn = false;
  const page = await openRoster(t, {
    route: (request) => {
      if (request.method === "PATCH") {
        switchedOn = true;
        return { body: JSON.stringify({ success: true }) };
      }
      if (request.url.pathname !== "/api/strategies/overview") return null;
      const overview = JSON.parse(JSON.stringify(allOff));
      overview.strategies.find((entry) => entry.item.id === DIP_RECOVERY).item.enabled = switchedOn;
      return { body: JSON.stringify(overview) };
    },
  });
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${DIP_RECOVERY}"]`);
  await page.waitForFunction(fillsContainer, 1500);
  assert.deepEqual(await actions(page, DIP_RECOVERY), ["edit", "delete"]);

  await page.click(`#strategies-roster label:has(> input[data-enable-strategy="${DIP_RECOVERY}"])`);
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${DIP_RECOVERY}"] [data-action-id="return-paper"]`);
  await page.waitForFunction(fillsContainer, 1500);
  const action = await page.$eval(
    `#strategies-roster tr[data-row-id="${DIP_RECOVERY}"] [data-action-id="return-paper"]`,
    (button) => {
      const box = button.getBoundingClientRect();
      const cell = button.closest("td").getBoundingClientRect();
      return { inside: box.left >= cell.left - 0.5 && box.right <= cell.right + 0.5, right: box.right };
    }
  );
  assert.equal(action.inside, true, "the new action sits inside its cell");
  assert.ok(action.right <= 1600, "and inside the window");
});

test("a paper strategy held by a check other than its rounds names that check", async (t) => {
  const held = JSON.parse(JSON.stringify(OVERVIEW));
  const momentum = held.strategies.find((entry) => entry.item.id === MOMENTUM);
  momentum.readiness = { closed: 10, needed: 10, ready: false, failing: "profit", since_rules_changed: null };
  const page = await openRoster(t, {
    route: (request) =>
      request.url.pathname === "/api/strategies/overview" ? { body: JSON.stringify(held) } : null,
  });
  const readiness = page.locator(
    `#strategies-roster tr[data-row-id="${MOMENTUM}"] .strategy-roster-readiness`
  );
  await readiness.waitFor();
  assert.equal(plain(await readiness.textContent()), "10/10 rounds");
  assert.equal(await readiness.getAttribute("data-readiness"), "blocked");
  assert.equal(
    plain(await readiness.getAttribute("title")),
    "10 of 10 closed paper rounds\nNot passing: Profitable in paper"
  );
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
  assert.equal(await page.locator("#strategies-roster tbody tr[data-row-id]").count(), 5);
  assert.equal(await page.locator("#strategies-roster .state-view-empty").count(), 0);
  const stored = page.locator(`#strategies-roster input[data-enable-strategy="${DIP_RECOVERY}"]`);
  assert.equal(await stored.isDisabled(), false, "the stored switch is released though the reload failed");
});

test("a first read that fails shows the error with Retry", async (t) => {
  let failing = true;
  const page = await openRoster(t, {
    route: (request) =>
      failing && request.url.pathname === "/api/strategies/overview"
        ? { status: 500, body: JSON.stringify({ error: { message: "Store unavailable" } }) }
        : null,
  });
  const empty = page.locator("#strategies-roster .state-view-empty", { hasText: "Load Failed" });
  await empty.waitFor();
  failing = false;
  await empty.locator('[data-action="table-empty-action"]', { hasText: "Retry" }).click();
  await page.waitForSelector(`#strategies-roster tr[data-row-id="${MOMENTUM}"]`);
});

test("a Retry that finds no strategy leaves the error for the empty roster", async (t) => {
  let failing = true;
  const page = await openRoster(t, {
    route: (request) => {
      if (request.url.pathname !== "/api/strategies/overview") return null;
      return failing
        ? { status: 500, body: JSON.stringify({ error: { message: "Store unavailable" } }) }
        : { body: JSON.stringify({ ...OVERVIEW, strategies: [] }) };
    },
  });
  const failed = page.locator("#strategies-roster .state-view-empty", { hasText: "Load Failed" });
  await failed.waitFor();
  failing = false;
  await failed.locator('[data-action="table-empty-action"]', { hasText: "Retry" }).click();
  const empty = page.locator("#strategies-roster .state-view-empty", { hasText: "No strategies yet" });
  await empty.waitFor();
  assert.equal(
    await empty.locator('[data-action="table-empty-action"]', { hasText: "New Strategy" }).count(),
    1
  );
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
  assert.match(plain(await chip.textContent()), /Status\s*Auto Trader is off/);
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
  await action.click();
  await page.locator("#create-strategy-modal:not([hidden])").waitFor();
  assert.deepEqual(
    await page.$$eval("#create-strategy-modal .type-card", (cards) =>
      cards.map((card) => card.getAttribute("aria-pressed"))
    ),
    ["false", "false"],
    "All chooses no kind"
  );
  assert.equal(
    await page.evaluate(() => document.activeElement?.dataset.type),
    "ENTRY",
    "the first kind takes the focus"
  );
  await page.keyboard.press("Escape");
  await page.locator("#create-strategy-modal").waitFor({ state: "hidden" });
  assert.equal(
    await page.evaluate(() => document.activeElement?.dataset.action),
    "table-empty-action",
    "Escape closes the dialog and returns the focus to its opener"
  );
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
