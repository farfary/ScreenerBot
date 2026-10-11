// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every control that turns the Auto Trader on goes through one owner,
 * `scripts/ui/auto_trader_control.js`, which asks before live trading starts and names
 * the trading wallet and the per-trade size. Declining sends nothing and leaves the
 * control off without it ever showing on; turning the trader off sends at once with no
 * dialog.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { SCRIPTS_ROOT, repoPath, walk } from "../lib/dashboard_ui.mjs";

const OWNER = "src/webserver/templates/scripts/ui/auto_trader_control.js";
const CONTROL_ENDPOINT = /\/api\/trader\/(?:start|stop)\b/;
const WALLET = "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin";
const WALLET_SHORT = "9xQe…VFin";

const METRICS = JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/header_metrics.json`, "utf8"));

test("only the owner calls the trader start and stop endpoints", async () => {
  const callers = [];
  for (const file of (await walk(SCRIPTS_ROOT)).filter((path) => path.endsWith(".js"))) {
    if (repoPath(file) !== OWNER && CONTROL_ENDPOINT.test(readFileSync(file, "utf8"))) {
      callers.push(repoPath(file));
    }
  }
  assert.deepEqual(callers, []);
});

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const shell = await loadIndex("shell");
const traderPage = await loadIndex("trader");

/**
 * A dashboard whose trader is on (`enabled`) or off, recording every control request.
 */
async function openDashboard({ enabled, page = "home" }) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(page === "trader" ? [traderPage, shell] : [shell], "populated");
  const controls = [];
  const onApi = (request) => {
    const { pathname } = request.url;
    if (request.method === "POST" && CONTROL_ENDPOINT.test(pathname)) {
      controls.push(pathname);
      const running = pathname.endsWith("/start");
      return {
        body: JSON.stringify({ success: true, status: { enabled: running, running } }),
      };
    }
    if (pathname === "/api/header/metrics") {
      return {
        body: JSON.stringify({
          ...METRICS,
          trader: { ...METRICS.trader, enabled, state: enabled ? "running" : "stopped" },
        }),
      };
    }
    if (pathname === "/api/trader/status") {
      return { body: JSON.stringify({ enabled, running: enabled, available: true }) };
    }
    if (pathname === "/api/wallets/main") {
      return { body: JSON.stringify({ id: 1, name: "Main", address: WALLET }) };
    }
    if (pathname === "/api/config/trader") {
      return { body: JSON.stringify({ data: { trade_size_sol: 0.05 } }) };
    }
    return api.onApi(request);
  };
  const host = await serveDashboard(context, { locale: "en", onApi, startupPage: page });
  const tab = await context.newPage();
  await tab.goto(`${host.origin}/${page}`);
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { tab, controls, close };
}

test("the header starts the trader only after the confirmation", async () => {
  const { tab, controls, close } = await openDashboard({ enabled: false });
  try {
    const card = tab.locator('#botCard[data-status="stopped"]');
    await card.waitFor();

    await card.click();
    const dialog = tab.locator(".confirmation-dialog");
    await dialog.waitFor();
    // Fluent wraps each placeable in bidi isolates (U+2066-U+2069).
    const message = (await dialog.locator(".confirmation-dialog__message").textContent())
      .replace(/[\u2066-\u2069]/g, "")
      .trim();
    assert.equal(
      message,
      `The Auto Trader will buy and sell on its own with real funds from the trading wallet ${WALLET_SHORT}, 0.05 SOL per trade.`
    );
    assert.equal(
      (await dialog.locator('[data-action="confirm"]').textContent()).trim(),
      "Start trading"
    );
    assert.equal(await card.getAttribute("aria-pressed"), "false");

    await tab.keyboard.press("Escape");
    await dialog.waitFor({ state: "detached" });
    assert.deepEqual(controls, []);
    assert.equal(await card.getAttribute("aria-pressed"), "false");

    await card.click();
    await dialog.locator('[data-action="confirm"]').click();
    await assertEventually(() => assert.deepEqual(controls, ["/api/trader/start"]));
  } finally {
    await close();
  }
});

test("the header stops the trader at once, without a dialog", async () => {
  const { tab, controls, close } = await openDashboard({ enabled: true });
  try {
    const card = tab.locator('#botCard[data-status="running"]');
    await card.waitFor();
    await card.click();
    await assertEventually(() => assert.deepEqual(controls, ["/api/trader/stop"]));
    assert.equal(await tab.locator(".confirmation-dialog").count(), 0);
  } finally {
    await close();
  }
});

test("the Trader page switch stays off while the start is being confirmed", async () => {
  const { tab, controls, close } = await openDashboard({ enabled: false, page: "trader" });
  try {
    const toggle = tab.locator("#stats-trader-toggle:not([disabled])");
    await toggle.waitFor({ state: "attached" });
    await tab.locator("#trader-status-bar .toggle").click();

    const dialog = tab.locator(".confirmation-dialog");
    await dialog.waitFor();
    assert.equal(await toggle.isChecked(), false);

    await dialog.locator('[data-action="cancel"]').click();
    await dialog.waitFor({ state: "detached" });
    assert.equal(await toggle.isChecked(), false);
    assert.deepEqual(controls, []);

    await tab.locator("#trader-status-bar .toggle").click();
    await dialog.locator('[data-action="confirm"]').click();
    await assertEventually(() => assert.deepEqual(controls, ["/api/trader/start"]));
    await tab.waitForFunction(
      () => document.querySelector("#stats-trader-toggle")?.checked === true
    );

    await tab.locator("#trader-status-bar .toggle").click();
    await assertEventually(() =>
      assert.deepEqual(controls, ["/api/trader/start", "/api/trader/stop"])
    );
    assert.equal(await tab.locator(".confirmation-dialog").count(), 0);
  } finally {
    await close();
  }
});

/** Retries `check` until it passes or two seconds elapse. */
async function assertEventually(check) {
  const deadline = Date.now() + 2000;
  for (;;) {
    try {
      check();
      return;
    } catch (error) {
      if (Date.now() > deadline) throw error;
      await new Promise((resolve) => setTimeout(resolve, 25));
    }
  }
}
