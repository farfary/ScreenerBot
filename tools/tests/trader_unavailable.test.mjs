// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Trader Stats panel states only what is true. While the trader cannot run
 * (Explore Mode, setup incomplete) no control reads as on, the notice is a muted
 * sentence with the setup action rather than an error, and no recovery button shows.
 * The loss-limit recovery buttons appear only once a configured limit has tripped,
 * and the drawdown card carries its danger accent only when a drawdown exists.
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
  unavailable_reason: "setup",
};
const UNAVAILABLE_MONITORS = {
  entry_monitor: { enabled: true, running: false },
  exit_monitor: { enabled: true, running: false },
  master_enabled: false,
  force_stopped: false,
  available: false,
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
async function openTrader({ variant, overrides }) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
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

test("an unavailable trader shows no control as on and offers setup", async () => {
  const { tab, close } = await openTrader({
    variant: "empty",
    overrides: {
      "/api/trader/status": UNAVAILABLE_STATUS,
      "/api/trader/monitors/status": UNAVAILABLE_MONITORS,
      "/api/trader/loss-limit/status": LOSS_LIMIT_OFF,
    },
  });
  try {
    await tab.locator('#trader-status-bar[data-status="unavailable"]').waitFor();
    await tab.locator("#entry-monitor-status", { hasText: "Setup required" }).waitFor();

    for (const id of ["#stats-trader-toggle", "#entry-monitor-toggle", "#exit-monitor-toggle"]) {
      const toggle = tab.locator(id);
      assert.equal(await toggle.isChecked(), false, `${id} must read off`);
      assert.equal(await toggle.isDisabled(), true, `${id} must not be interactive`);
    }

    const notice = tab.locator("#trader-status-text");
    assert.equal(
      (await notice.textContent()).trim(),
      "Complete wallet and RPC setup to use Auto Trader"
    );
    assert.equal(await notice.evaluate((el) => getComputedStyle(el).textTransform), "none");
    const colors = await notice.evaluate((el) => {
      const probe = document.createElement("span");
      probe.style.color = "var(--text-muted)";
      el.parentElement.append(probe);
      const muted = getComputedStyle(probe).color;
      probe.remove();
      return { notice: getComputedStyle(el).color, muted };
    });
    assert.equal(colors.notice, colors.muted, "the notice is muted, not an error colour");

    assert.equal(await tab.locator("#loss-limit-actions").isVisible(), false);
    assert.equal(await tab.locator("#loss-limit-resume-btn").isVisible(), false);
    assert.equal(await tab.locator("#loss-limit-reset-btn").isVisible(), false);
    assert.equal(await tab.locator("#resume-trading-btn").isVisible(), false);

    const drawdownCard = tab.locator(".metric-card", { has: tab.locator("#max-drawdown") });
    assert.equal((await tab.locator("#max-drawdown").textContent()).trim(), "—");
    assert.equal(await drawdownCard.evaluate((el) => el.classList.contains("metric-highlight-danger")), false);

    const setup = tab.locator("#trader-setup-action");
    assert.equal((await setup.textContent()).trim(), "Complete setup");
    await setup.click();
    await tab.locator(".setup-dialog").waitFor();
  } finally {
    await close();
  }
});

test("a tripped loss limit offers recovery and a drawdown carries the accent", async () => {
  const { tab, close } = await openTrader({
    variant: "populated",
    overrides: { "/api/trader/loss-limit/status": LOSS_LIMIT_TRIPPED },
  });
  try {
    await tab.locator('#trader-status-bar[data-status="running"]').waitFor();
    await tab.locator("#loss-limit-resume-btn").waitFor({ state: "visible" });
    assert.equal(await tab.locator("#loss-limit-reset-btn").isVisible(), true);
    assert.equal(await tab.locator("#trader-setup-action").isVisible(), false);
    assert.equal(await tab.locator("#entry-monitor-toggle").isChecked(), true);

    const drawdownCard = tab.locator(".metric-card.metric-highlight-danger", {
      has: tab.locator("#max-drawdown"),
    });
    await drawdownCard.waitFor();
  } finally {
    await close();
  }
});
