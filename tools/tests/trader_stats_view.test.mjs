// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Trader Stats tab says each fact once, in its one place.
 *
 * - A monitor switch shows its saved setting; while the trader is stopped its note says
 *   it starts with the Auto Trader, and nothing calls the trader off.
 * - The toolbar names the run state and why the shown book takes no new entry.
 * - A tripped loss limit is one notice with Resume, the reset beside the meter; a force
 *   stop is one error notice and hides the force stop it already applied.
 * - An empty window shows one empty state and no breakdown.
 * - A live recent close opens its position details; a paper close has none to open.
 * - The paper book adds its cash and the reset of its run, and only it reads the run.
 * - The tab never scrolls sideways, and no heading carries a glyph.
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
const indexes = [await loadIndex("trader"), await loadIndex("shell")];

const STOPPED_STATUS = {
  enabled: false,
  running: false,
  available: true,
  live_strategies: 2,
  paper_strategies: 1,
  run_state: "stopped",
  live_entry_idle: null,
  paper_entry_idle: null,
};
const STOPPED_MONITORS = {
  entry_monitor: { enabled: true, running: false },
  exit_monitor: { enabled: true, running: false },
  master_enabled: false,
  force_stopped: false,
  available: true,
  anything_running: false,
};
const LOSS_LIMIT_TRIPPED = {
  enabled: true,
  limit_native: 0.1,
  current_loss_native: 0.12,
  is_limited: true,
  limited_at: "2026-10-07T09:00:00Z",
  period_start: "2026-10-07T00:00:00Z",
  period_remaining_secs: 50400,
  progress_percent: 120,
};
const FORCE_STOPPED = {
  is_stopped: true,
  stopped_at: "2026-10-07T09:00:00Z",
  reason: "Stopped from the dashboard",
};
const ACCOUNT = {
  epoch: 2,
  started_at: 1_790_812_800,
  starting_balance_native: 1,
  cash_native: 0.9412,
};

/**
 * The trader page on Stats. `overrides` answers a GET path with a fixed body; every API
 * request is recorded as `pathname?search`.
 */
async function openStats({ variant = "populated", overrides = {}, width = 1440 } = {}) {
  const context = await browser.newContext({ viewport: { width, height: 900 } });
  const api = createApiHandler(indexes, variant);
  const requests = [];
  const onApi = (request) => {
    const { pathname, search } = request.url;
    requests.push(`${pathname}${search}`);
    const body = overrides[pathname];
    if (request.method === "GET" && body) return { body: JSON.stringify(body) };
    return api.onApi(request);
  };
  const host = await serveDashboard(context, { locale: "en", onApi, startupPage: "trader" });
  const tab = await context.newPage();
  tab.setDefaultTimeout(15000);
  await tab.goto(`${host.origin}/trader#stats`);
  await tab.waitForSelector(PAGE_READY);
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { tab, requests, close };
}

/** Text of a locator without the bidi isolates Fluent wraps each placeable in. */
async function plainText(locator) {
  return (await locator.textContent()).replace(/[⁦-⁩]/g, "").trim();
}

const chipValue = (tab, id) => tab.locator(`[data-summary-id="${id}"] .table-toolbar-chip__value`);

const bookOption = (tab, value) =>
  tab.locator("#stats-book .segmented-option", { has: tab.locator(`input[value="${value}"]`) });

test("a monitor keeps its saved setting while the trader is stopped", async () => {
  const { tab, close } = await openStats({
    overrides: {
      "/api/trader/status": STOPPED_STATUS,
      "/api/trader/monitors/status": STOPPED_MONITORS,
    },
  });
  try {
    await tab.locator("#entry-monitor-status:not([hidden])").waitFor();
    for (const kind of ["entry", "exit"]) {
      assert.equal(await tab.locator(`#${kind}-monitor-toggle`).isChecked(), true);
      assert.equal(await tab.locator(`#${kind}-monitor-toggle`).isDisabled(), false);
      assert.equal(
        await plainText(tab.locator(`#${kind}-monitor-status`)),
        "Starts with Auto Trader"
      );
    }
    assert.equal(await plainText(chipValue(tab, "run")), "Stopped");
    const text = await tab.locator("#stats-tab").innerText();
    assert.doesNotMatch(text, /Auto Trader off/i);
  } finally {
    await close();
  }
});

test("the toolbar names the run state and the entry block of the shown book", async () => {
  const { tab, close } = await openStats();
  try {
    await chipValue(tab, "entries").waitFor();
    assert.equal(await plainText(chipValue(tab, "run")), "Running");
    assert.equal(
      await tab.locator('[data-summary-id="run"]').getAttribute("data-variant"),
      "success"
    );
    // The live book is at its open-position limit; the paper book takes entries.
    assert.equal(await plainText(chipValue(tab, "entries")), "Open-position limit reached");
    await bookOption(tab, "paper").click();
    await tab
      .locator('[data-summary-id="entries"] .table-toolbar-chip__value', { hasText: /^Open$/ })
      .waitFor();
    assert.equal(
      await tab.locator('[data-summary-id="entries"]').getAttribute("data-variant"),
      null
    );
  } finally {
    await close();
  }
});

test("a tripped loss limit shows one notice with Resume and the Reset beside the meter", async () => {
  const { tab, close } = await openStats({
    overrides: { "/api/trader/loss-limit/status": LOSS_LIMIT_TRIPPED },
  });
  try {
    const notice = tab.locator('#stats-notices .notice[data-variant="warning"]');
    await notice.waitFor();
    assert.equal(await tab.locator("#stats-notices .notice").count(), 1);
    assert.equal(await plainText(notice.locator(".notice-title")), "Loss limit reached");
    assert.equal(await notice.locator('[data-action="loss-limit-resume"]').count(), 1);
    const reset = tab.locator("#loss-limit-panel #loss-limit-reset-btn");
    assert.equal(await reset.isVisible(), true);
    assert.match(
      await tab.locator("#loss-limit-progress").getAttribute("class"),
      /\blimit-exceeded\b/
    );
  } finally {
    await close();
  }
});

test("a force stop shows one error notice and hides the force stop button", async () => {
  const { tab, close } = await openStats({
    overrides: {
      "/api/trader/force-stop/status": FORCE_STOPPED,
      "/api/trader/status": { ...STOPPED_STATUS, run_state: "force_stopped" },
    },
  });
  try {
    const notice = tab.locator('#stats-notices .notice[data-variant="error"]');
    await notice.waitFor();
    assert.equal(await tab.locator("#stats-notices .notice").count(), 1);
    assert.equal(await plainText(notice.locator(".notice-title")), "Trading halted");
    assert.equal(await notice.locator('[data-action="resume"]').count(), 1);
    assert.equal(await tab.locator("#force-stop-btn").isVisible(), false);
    assert.equal(await tab.locator("#stats-trader-action").isVisible(), false);
    assert.equal(await plainText(chipValue(tab, "run")), "Force stopped");
    for (const id of ["#entry-monitor-toggle", "#exit-monitor-toggle"]) {
      assert.equal(await tab.locator(id).isDisabled(), true, `${id} is locked while stopped`);
    }
  } finally {
    await close();
  }
});

test("a force stop with a tripped loss limit shows one paused notice", async () => {
  const { tab, close } = await openStats({
    overrides: {
      "/api/trader/force-stop/status": FORCE_STOPPED,
      "/api/trader/loss-limit/status": LOSS_LIMIT_TRIPPED,
      "/api/trader/status": { ...STOPPED_STATUS, run_state: "force_stopped" },
    },
  });
  try {
    await tab.locator('#stats-notices .notice[data-variant="error"]').waitFor();
    assert.equal(await tab.locator("#stats-notices .notice").count(), 1);
    assert.doesNotMatch(await tab.locator("#stats-notices").innerText(), /Exits continue/);
  } finally {
    await close();
  }
});

test("a group whose rounds have no known P&L shows dashes, never zero", async () => {
  const { tab, close } = await openStats();
  try {
    const row = tab.locator("#exit-breakdown tbody tr", { hasText: "Not recorded" });
    await row.waitFor();
    const cells = (await row.locator("td").allTextContents()).map((text) =>
      text.replace(/[\u2066-\u2069]/g, "").trim()
    );
    assert.deepEqual(cells.slice(-3), ["—", "—", "—"]);
  } finally {
    await close();
  }
});

test("an unknown P&L stays out of the outcome histogram", async () => {
  const { tab, close } = await openStats();
  try {
    const columns = tab.locator("#outcome-distribution .inline-histogram-col");
    await columns.first().waitFor();
    const counts = (
      await tab.locator("#outcome-distribution .inline-histogram-count").allTextContents()
    )
      .map(Number)
      .reduce((sum, count) => sum + count, 0);
    assert.equal(counts, 23, "24 counted rounds, one with an unknown P&L");
    assert.ok(await tab.locator("#outcome-distribution [role=list][aria-label]").count());
    for (const label of await tab
      .locator("#outcome-distribution .inline-histogram-label")
      .allTextContents()) {
      assert.match(label, /%/, `the bucket label "${label}" names its unit`);
    }
  } finally {
    await close();
  }
});

test("an empty window shows one empty state and hides the breakdowns", async () => {
  const { tab, close } = await openStats({ variant: "empty" });
  try {
    await tab.locator("#stats-empty:not([hidden]) .state-view-empty").waitFor();
    assert.equal(await tab.locator("#stats-loading").isVisible(), false);
    assert.equal(await tab.locator("#stats-breakdowns").isVisible(), false);
    const shown = await tab.$$eval(
      "#stats-sheet .state-view",
      (views) => views.filter((view) => view.getClientRects().length > 0).length
    );
    assert.equal(shown, 1, "one empty state speaks for the window");
    assert.equal(await plainText(tab.locator("#total-trades")), "0");
  } finally {
    await close();
  }
});

test("a live recent row opens the position details and a paper row does not", async () => {
  const { tab, close } = await openStats({
    overrides: { "/api/positions/paper/account": ACCOUNT },
  });
  try {
    await tab.evaluate(() => {
      window.__openedPositions = [];
      window.addEventListener("screenerbot:open-position-details", (event) =>
        window.__openedPositions.push(event.detail.id)
      );
    });
    const firstRow = tab.locator("#recent-rounds tbody tr").first();
    await firstRow.waitFor();
    await firstRow.click();
    await tab.waitForFunction(() => window.__openedPositions.length === 1);
    assert.deepEqual(await tab.evaluate(() => window.__openedPositions), [812]);
    await tab.keyboard.press("Escape");

    await bookOption(tab, "paper").click();
    await tab.locator("#stats-paper-reset:not([hidden])").waitFor();
    await tab.locator("#recent-rounds tbody tr").first().waitFor();
    await tab.locator("#recent-rounds tbody tr").first().click();
    await tab.waitForTimeout(300);
    assert.deepEqual(await tab.evaluate(() => window.__openedPositions), [812]);
  } finally {
    await close();
  }
});

test("the paper book shows cash and reset and fetches the account only there", async () => {
  const { tab, requests, close } = await openStats({
    overrides: { "/api/positions/paper/account": ACCOUNT },
  });
  const accountReads = () =>
    requests.filter((url) => url.startsWith("/api/positions/paper/account")).length;
  try {
    await chipValue(tab, "open").waitFor();
    assert.equal(await plainText(chipValue(tab, "open")), "2 / 2");
    assert.equal(await tab.locator('[data-summary-id="cash"]').count(), 0);
    assert.equal(await tab.locator("#stats-paper-reset").isVisible(), false);
    assert.equal(accountReads(), 0, "the live book never reads the paper run");

    await bookOption(tab, "paper").click();
    await chipValue(tab, "cash").waitFor();
    assert.match(await plainText(chipValue(tab, "cash")), /^0\.9412\sSOL$/);
    assert.equal(await tab.locator("#stats-paper-reset").isVisible(), true);
    // The results name their book, and the live-only loss limit steps out.
    assert.equal(await plainText(tab.locator("#stats-book-marker")), "Paper");
    assert.equal(await tab.locator("#loss-limit-panel").isVisible(), false);
    assert.ok(accountReads() >= 1, "the paper book reads its run");
    assert.ok(
      requests.some((url) => /^\/api\/trader\/stats\?window=30d&book=paper$/.test(url)),
      "the paper figures come from the paper book"
    );

    await bookOption(tab, "live").click();
    await tab.locator('[data-summary-id="cash"]').waitFor({ state: "detached" });
    assert.equal(await tab.locator("#stats-paper-reset").isVisible(), false);
    assert.equal(await tab.locator("#stats-book-marker").isVisible(), false);
    assert.equal(await tab.locator("#loss-limit-panel").isVisible(), true);
  } finally {
    await close();
  }
});

for (const width of [420, 900, 1280]) {
  test(`the Stats tab never scrolls sideways at ${width}px`, async () => {
    const { tab, close } = await openStats({ width });
    try {
      await tab.locator("#recent-rounds tbody tr").first().waitFor();
      const overflow = await tab.evaluate(() => {
        const root = document.scrollingElement;
        const sheet = document.querySelector("#stats-tab");
        return {
          page: root.scrollWidth - root.clientWidth,
          tab: sheet.scrollWidth - sheet.clientWidth,
        };
      });
      assert.deepEqual(overflow, { page: 0, tab: 0 });
    } finally {
      await close();
    }
  });
}

test("no heading in the Stats tab carries a glyph", async () => {
  const { tab, close } = await openStats();
  try {
    await tab.locator("#recent-rounds tbody tr").first().waitFor();
    const glyphs = await tab.$$eval(
      "#stats-tab :is(h1, h2, h3, h4, dt, .sheet-section-title, .metric-label, .stats-control-label)",
      (headings) =>
        headings
          .filter((heading) => heading.querySelector("i, svg, [class*='icon-']"))
          .map((heading) => heading.textContent.trim())
    );
    assert.deepEqual(glyphs, []);
  } finally {
    await close();
  }
});
