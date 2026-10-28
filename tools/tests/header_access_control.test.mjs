// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the header access control names a paused ScreenerBot service connection
 * and its one way out.
 *
 * - Ready on the service, or on your own RPC, the control is hidden.
 * - Signed out and sign-in-again states read "Trading paused · …" with a Sign in
 *   action that opens Settings > Account.
 * - The daily limit offers Add RPC, which opens Settings > Connection.
 * - Busy and unreachable states name themselves with no action. Every paused state
 *   keeps its title when the header compacts to its minimal step; one with an action
 *   drops the action there, and the control still opens the tab that resumes it.
 * - A connection still being checked stays hidden for its first seconds.
 * - The title is sentence case: no uppercase transform.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const BOOTSTRAP = JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/system_bootstrap.json`, "utf8"));
const CAPABILITIES = {
  live_pool_prices: false,
  direct_pool_swaps: false,
  watch_other_wallets: false,
  other_wallet_balances: false,
  full_history: false,
  rpc_statistics: false,
  own_wallet_activity: true,
  aggregator_swaps: true,
};
const TEXT_IDS = {
  checking: "connectivity-service-checking",
  ready: "connectivity-service-ready",
  sign_in_required: "connectivity-service-signed-out",
  reauthorization_required: "connectivity-service-reauthorize",
  busy: "connectivity-service-busy",
  daily_limit: "connectivity-service-daily-limit",
  unreachable: "connectivity-service-unreachable",
};

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [home, shell] = await Promise.all([loadIndex("home"), loadIndex("shell")]);

function accessStatus(mode, state) {
  return {
    chain: "solana",
    mode,
    state,
    ready: state === "ready",
    actionable: state !== "ready" && state !== "checking",
    text: { id: TEXT_IDS[state], args: {} },
    capabilities: mode === "service" ? CAPABILITIES : {},
    account: { signed_in: state !== "sign_in_required", chain_scope: true, submit_scope: true },
  };
}

/** Opens Home with the connection in `state` and returns the header control. */
async function openHeader(t, mode, state, width = 1440) {
  const context = await browser.newContext({ viewport: { width, height: 800 } });
  const fixtures = createApiHandler([home, shell], "populated");
  const status = accessStatus(mode, state);
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      const path = request.url.pathname;
      if (path === "/api/system/bootstrap") {
        return { body: JSON.stringify({ ...BOOTSTRAP, chain_access: status }) };
      }
      if (path === "/api/chain/access") return { body: JSON.stringify(status) };
      return fixtures.onApi(request);
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector("#navTabs > a.tab.active");
  return { page, control: page.locator("#accessControl") };
}

const read = (control) =>
  control.evaluate((node) => ({
    title: node.querySelector(".access-control-title").textContent,
    action: node.querySelector(".access-control-action").hidden
      ? null
      : node.querySelector(".access-control-action").textContent,
    transform: getComputedStyle(node.querySelector(".access-control-title")).textTransform,
  }));

for (const [mode, state] of [
  ["service", "ready"],
  ["own_rpc", "ready"],
]) {
  test(`the control is hidden on ${mode} when ${state}`, async (t) => {
    const { page, control } = await openHeader(t, mode, state);
    await page.waitForTimeout(500);
    assert.equal(await control.isVisible(), false);
  });
}

test("signed out reads Trading paused with a Sign in action to the account", async (t) => {
  const { page, control } = await openHeader(t, "service", "sign_in_required");
  await control.waitFor();
  assert.deepEqual(await read(control), {
    title: "Trading paused · Signed out",
    action: "Sign in",
    transform: "none",
  });
  await control.click();
  await page.locator('.settings-nav-item[data-tab="account"].active').waitFor();
});

test("sign in again offers Sign in", async (t) => {
  const { control } = await openHeader(t, "service", "reauthorization_required");
  await control.waitFor();
  assert.equal((await read(control)).action, "Sign in");
});

test("the daily limit offers Add RPC, which opens Connection", async (t) => {
  const { page, control } = await openHeader(t, "service", "daily_limit");
  await control.waitFor();
  assert.equal((await read(control)).action, "Add RPC");
  await control.click();
  await page.locator('.settings-nav-item[data-tab="connection"].active').waitFor();
});

for (const state of ["busy", "unreachable"]) {
  test(`${state} names itself with no action`, async (t) => {
    const { control } = await openHeader(t, "service", state);
    await control.waitFor();
    const view = await read(control);
    assert.match(view.title, /^Trading paused · /);
    assert.equal(view.action, null);
  });
}

test("a compacted header keeps the title of a state without an action", async (t) => {
  const { page, control } = await openHeader(t, "service", "busy", 1280);
  await control.waitFor();
  await page.locator(".header-row-1.is-access-minimal").waitFor();
  assert.equal(await control.locator(".access-control-title").isVisible(), true);
});

test("a compacted header keeps the state of a pause with an action", async (t) => {
  const { page, control } = await openHeader(t, "service", "daily_limit", 1280);
  await control.waitFor();
  await page.locator(".header-row-1.is-access-minimal").waitFor();
  assert.equal(await control.locator(".access-control-title").isVisible(), true);
  assert.equal(await control.locator(".access-control-action").isVisible(), false);
  await control.click();
  await page.locator('.settings-nav-item[data-tab="connection"].active').waitFor();
});

test("a connection being checked stays hidden at first", async (t) => {
  const { page, control } = await openHeader(t, "service", "checking");
  await page.waitForTimeout(1000);
  assert.equal(await control.isVisible(), false);
});
