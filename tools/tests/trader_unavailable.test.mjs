// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Trader Stats tab states only what is true. While the trader cannot run
 * (Explore Mode, setup incomplete) no switch reads as on, no run chip or start action
 * shows, and one warning notice carries the setup action; the force stop and the loss-limit
 * reset are hidden, since there is nothing to halt or clear. Once a configured loss limit
 * has tripped, Resume sits in the notice and the reset beside the meter. Max drawdown is an
 * unsigned magnitude and stays neutral whatever its size.
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
const shell = await loadIndex("shell");
const traderPage = await loadIndex("trader");

const UNAVAILABLE_STATUS = {
  enabled: false,
  running: false,
  available: false,
  unavailable_reason: { id: "paper-live-block-setup-incomplete" },
  live_strategies: 0,
  paper_strategies: 0,
  run_state: "explore",
  live_entry_idle: null,
  paper_entry_idle: null,
};
const UNAVAILABLE_MONITORS = {
  entry_monitor: { enabled: true, running: false },
  exit_monitor: { enabled: true, running: false },
  master_enabled: false,
  force_stopped: false,
  available: false,
  anything_running: false,
};
const LOSS_LIMIT_OFF = {
  enabled: false,
  limit_native: 0.1,
  current_loss_native: 0,
  is_limited: false,
  limited_at: null,
  period_start: "2026-10-07T00:00:00Z",
  period_remaining_secs: 86399,
  progress_percent: 0,
};
const LOSS_LIMIT_TRIPPED = {
  ...LOSS_LIMIT_OFF,
  enabled: true,
  current_loss_native: 0.12,
  is_limited: true,
  limited_at: "2026-10-07T09:00:00Z",
  progress_percent: 120,
};

/** The trader page on its Stats tab, with the given live responses and fixture variant. */
async function openTrader({ variant, overrides, width = 1440 }) {
  const context = await browser.newContext({ viewport: { width, height: 900 } });
  const api = createApiHandler([traderPage, shell], variant);
  const onApi = (request) => {
    const body = overrides[request.url.pathname];
    if (request.method === "GET" && body) return { body: JSON.stringify(body) };
    return api.onApi(request);
  };
  const host = await serveDashboard(context, { locale: "en", onApi, startupPage: "trader" });
  const tab = await context.newPage();
  await tab.goto(`${host.origin}/trader`);
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { tab, close };
}

/** Text of a locator without the bidi isolates Fluent wraps each placeable in. */
async function plainText(locator) {
  return (await locator.textContent()).replace(/[\u2066-\u2069]/g, "").trim();
}

test("an unavailable trader shows no control as on and offers setup", async () => {
  const { tab, close } = await openTrader({
    variant: "empty",
    overrides: {
      "/api/trader/status": UNAVAILABLE_STATUS,
      "/api/trader/monitors/status": UNAVAILABLE_MONITORS,
      "/api/trader/loss-limit/status": LOSS_LIMIT_OFF,
    },
    width: 1200,
  });
  try {
    const notice = tab.locator('#stats-notices .notice[data-variant="warning"]');
    await notice.waitFor();
    assert.equal(await tab.locator("#stats-notices .notice").count(), 1, "one notice speaks");
    assert.equal(await plainText(notice.locator(".notice-title")), "Setup required");
    assert.equal(
      await plainText(notice.locator(".notice-message")),
      "Finish wallet and RPC setup first"
    );
    assert.equal(await notice.evaluate((el) => getComputedStyle(el).textTransform), "none");

    for (const id of ["#entry-monitor-toggle", "#exit-monitor-toggle"]) {
      const toggle = tab.locator(id);
      assert.equal(await toggle.isChecked(), false, `${id} must read off`);
      assert.equal(await toggle.isDisabled(), true, `${id} must not be interactive`);
    }
    // The setup notice is the one statement of the state: no run chip, no note under a
    // switch, no start action.
    assert.equal(await tab.locator('#stats-state [data-summary-id="run"]').count(), 0);
    for (const id of ["#entry-monitor-status", "#exit-monitor-status", "#stats-trader-action"]) {
      assert.equal(await tab.locator(id).isVisible(), false, `${id} stays hidden`);
    }
    assert.equal(await tab.locator("#loss-limit-reset-btn").isVisible(), false);
    assert.equal(await tab.locator("#force-stop-btn").isVisible(), false);
    assert.equal(await plainText(tab.locator("#loss-limit-value")), "Off");

    // An empty window counts zero; every other figure reads "—" with no detail, and one
    // empty state says why.
    await tab.locator("#total-trades", { hasText: "0" }).waitFor();
    for (const id of [
      "#net-pnl",
      "#win-rate",
      "#profit-factor",
      "#max-drawdown",
      "#avg-win-loss",
      "#median-hold",
      "#fees-paid",
    ]) {
      assert.equal(await plainText(tab.locator(id)), "—", `${id} reads —`);
      assert.equal(await plainText(tab.locator(`${id}-detail`)), "", `${id} carries no detail`);
    }
    await tab
      .locator("#stats-empty:not([hidden])", {
        hasText: /No closed trades in the last \W*30\W* days/,
      })
      .waitFor();
  } finally {
    await close();
  }
});

test("the setup notice opens the setup dialog", async () => {
  const { tab, close } = await openTrader({
    variant: "empty",
    overrides: {
      "/api/trader/status": UNAVAILABLE_STATUS,
      "/api/trader/monitors/status": UNAVAILABLE_MONITORS,
      "/api/trader/loss-limit/status": LOSS_LIMIT_OFF,
    },
  });
  try {
    const setup = tab.locator('#stats-notices [data-action="setup"]');
    await setup.waitFor();
    assert.equal(await plainText(setup), "Complete setup");
    await setup.click();
    await tab.locator(".setup-dialog").waitFor();
  } finally {
    await close();
  }
});

test("a tripped loss limit offers recovery and the force stop says what it halts", async () => {
  const { tab, close } = await openTrader({
    variant: "populated",
    overrides: { "/api/trader/loss-limit/status": LOSS_LIMIT_TRIPPED },
  });
  try {
    await tab.locator('#stats-notices [data-action="loss-limit-resume"]').waitFor();
    assert.equal(await tab.locator("#loss-limit-reset-btn").isVisible(), true);
    assert.equal(await tab.locator('#stats-notices [data-action="setup"]').count(), 0);
    assert.equal(await tab.locator("#entry-monitor-toggle").isChecked(), true);
    await tab
      .locator("#force-stop-hint", { hasText: "Halts every monitor, copy task and trade at once." })
      .waitFor();
    assert.equal(await tab.locator("#force-stop-btn").isDisabled(), false);
  } finally {
    await close();
  }
});

test("max drawdown stays neutral", async () => {
  const { tab, close } = await openTrader({ variant: "populated", overrides: {} });
  try {
    const drawdown = tab.locator("#max-drawdown");
    await tab.waitForFunction(() => document.querySelector("#max-drawdown")?.textContent !== "—");
    assert.doesNotMatch(await drawdown.getAttribute("class"), /\b(positive|negative)\b/);
    const card = tab.locator(".metric-card", { has: drawdown });
    const borders = await tab.$$eval("#stats-figures .metric-card", (cards) =>
      cards.map((el) => getComputedStyle(el).borderTopColor)
    );
    assert.equal(
      await card.evaluate((el) => getComputedStyle(el).borderTopColor),
      borders[0],
      "the drawdown card carries no accent"
    );
  } finally {
    await close();
  }
});
