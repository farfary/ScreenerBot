// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the access gate. In Explore Mode the wallet-backed pages state that setup
 * is needed and never call the routes a full setup opens; on the ScreenerBot service
 * the pages that need a capability the service lacks state that it needs your own
 * RPC, keep their data, and never call the routes the service refuses.
 *
 * Explore Mode runs without the wallet store and the watch store. Wallets read
 * `/api/wallets` on every visit and drew an empty "No main wallet" card over the
 * failure; Copy Trading let the whole Add wallet wizard be filled in before
 * "Create paper task" failed on the missing watch store.
 *
 * - Wallets shows the shared setup notice in place of its tab panels and requests
 *   no `/api/wallets` route.
 * - Copy Trading names the reason in its strip, disables every wallet-backed
 *   control with that reason, and shows the setup notice in place of the
 *   onboarding card while no task exists.
 * - Transactions shows the setup notice in place of its table and requests no
 *   transaction or wallet route, so no refresh failure is raised.
 * - Home shows no wallet figure: the worth the API cannot know arrives as null and
 *   every portfolio, P&L and exposure value reads "—", never a measured zero.
 * - The Buy and Sell dialogs open as the setup notice: no quick amount or MAX can be
 *   chosen, the confirm is disabled with the reason as its tooltip, the setup action
 *   replaces the dialog with the setup wizard, and the dialogs read no wallet balance.
 * - Complete setup opens the full-screen first-run flow over the dashboard, on an
 *   unchosen wallet source: Back leads to the introduction and its shortcut back to
 *   the wizard, and the wizard's Explore action returns to the dashboard underneath
 *   without asking the server to enter Explore Mode again.
 * - On the ScreenerBot service, Copy Trading shows the own-RPC notice above the kept
 *   tasks with the Connection settings action, and every copy control carries the
 *   watch reason; Wallets > Watched shows the gate and reads no watch route; the
 *   Auto Trader shows the service notice once per session; token details and other
 *   wallets' balances name the missing capability.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

const WAIT_MS = 15000;
const REASON = "Explore Mode runs without a wallet or RPC. Complete setup to connect them.";

const fixture = (path) => JSON.parse(readFileSync(`${FIXTURES_ROOT}/${path}`, "utf8"));
const EXPLORE_BOOTSTRAP = JSON.stringify({
  ...fixture("shell/system_bootstrap.json"),
  initialization_complete: false,
  explore_mode: true,
});
// Fluent isolates every placeable (U+2068/U+2069); the wording is asserted without them.
const plain = (text) => (typeof text === "string" ? text.replace(/[\u2068\u2069]/g, "") : text);
const WATCH_REASON =
  "Copy trading and wallet watching follow other wallets live, which the ScreenerBot service does not do. Your copy tasks and watched wallets are kept and come back when you add an RPC.";
const SERVICE_ACCESS = {
  chain: "solana",
  mode: "service",
  state: "ready",
  ready: true,
  actionable: false,
  text: { id: "connectivity-service-ready", args: {} },
  capabilities: {
    live_pool_prices: false,
    direct_pool_swaps: false,
    watch_other_wallets: false,
    other_wallet_balances: false,
    full_history: false,
    rpc_statistics: false,
    own_wallet_activity: true,
    aggregator_swaps: true,
  },
  account: { signed_in: true, chain_scope: true, submit_scope: true },
};
const SERVICE_BOOTSTRAP = JSON.stringify({
  ...fixture("shell/system_bootstrap.json"),
  chain_access: SERVICE_ACCESS,
});
const overview = fixture("copy/copy_overview.json");
const EMPTY_OVERVIEW = JSON.stringify({
  ...overview,
  status: { ...overview.status, total_tasks: 0, active_tasks: 0, paper_tasks: 0, live_tasks: 0 },
  tasks: [],
});

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const shellIndex = await loadIndex("shell");

/** Opens `id` in Explore Mode and records every `/api` request the page makes. */
function openExplore(id, overrides = {}) {
  return openWith(EXPLORE_BOOTSTRAP, id, overrides);
}

/** Opens `id` on the ScreenerBot service and records every `/api` request. */
function openService(id, overrides = {}) {
  return openWith(SERVICE_BOOTSTRAP, id, {
    "/api/chain/access": JSON.stringify(SERVICE_ACCESS),
    ...overrides,
  });
}

async function openWith(bootstrap, id, overrides) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex(id), shellIndex], "populated");
  const requests = [];
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      requests.push(`${request.method} ${request.url.pathname}`);
      if (request.url.pathname === "/api/system/bootstrap") return { body: bootstrap };
      const body = overrides[request.url.pathname];
      return body ? { body } : api.onApi(request);
    },
  });
  const page = await context.newPage();
  page.setDefaultTimeout(WAIT_MS);
  await page.goto(`${host.origin}/${id}`);
  await page.waitForSelector(PAGE_READY);
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { page, requests, close };
}

test("Wallets shows the setup notice and reads no wallet route", async (t) => {
  const { page, requests, close } = await openExplore("wallets");
  t.after(close);

  const gate = page.locator("#wallets-access-gate .access-gate");
  await gate.waitFor();
  assert.equal(await gate.locator(".state-view-title").textContent(), "Wallets need setup");
  assert.equal(await gate.locator(".state-view-message").textContent(), REASON);
  assert.equal(await gate.locator("button").textContent(), "Complete setup");
  assert.equal(await page.locator(".wallets-tab-panels").isVisible(), false);
  await page.waitForTimeout(500);
  assert.deepEqual(
    requests.filter((request) => / \/api\/wallets\b/.test(request)),
    []
  );
});

test("Copy Trading gates its wallet-backed controls with one reason", async (t) => {
  const empty = await openExplore("copy", { "/api/copy-trading/overview": EMPTY_OVERVIEW });
  t.after(empty.close);
  const { page } = empty;

  const gate = page.locator("#copy-access-gate .access-gate");
  await gate.waitFor();
  assert.equal(
    await gate.locator(".state-view-title").textContent(),
    "Copy trading needs a wallet"
  );
  assert.equal(await page.locator("#copy-onboarding").isVisible(), false);
  assert.equal(await page.locator("#copy-add").isVisible(), false, "one call to action");
  assert.equal(
    await page.locator('#copy-summary [data-summary-id="status"]').count(),
    0,
    "the toolbar does not repeat the setup notice"
  );
  assert.equal(
    await page.locator("#copy-global-action").isVisible(),
    false,
    "no bulk action without a task"
  );

  const populated = await openExplore("copy");
  t.after(populated.close);
  const add = populated.page.locator("#copy-add");
  await populated.page.waitForSelector("#copy-main:not([hidden])");
  assert.equal(await populated.page.locator("#copy-access-gate").isVisible(), false);
  assert.equal(await add.isVisible(), true);
  assert.equal(await add.isDisabled(), true);
  assert.equal(await add.getAttribute("title"), REASON);
  assert.equal(
    await populated.page
      .locator('#copy-summary [data-summary-id="status"] .table-toolbar-chip__value')
      .textContent(),
    "Setup required"
  );
  const global = populated.page.locator("#copy-global-action");
  assert.equal(await global.isVisible(), true);
  assert.equal(await global.isDisabled(), true);
  assert.equal(await global.getAttribute("title"), REASON);
});

test("Transactions shows the setup notice and reads no transaction route", async (t) => {
  const { page, requests, close } = await openExplore("transactions");
  t.after(close);

  const gate = page.locator("#transactions-access-gate .access-gate");
  await gate.waitFor();
  assert.equal(await gate.locator(".state-view-title").textContent(), "Transactions need a wallet");
  assert.equal(await gate.locator(".state-view-message").textContent(), REASON);
  assert.equal(await page.locator("#transactions-root").isVisible(), false);
  await page.waitForTimeout(1000);
  assert.deepEqual(
    requests.filter((request) => / \/api\/(?:transactions|wallets)\b/.test(request)),
    []
  );
  assert.equal(await page.locator(".toast").count(), 0);
});

test("Home shows no wallet figure without a wallet", async (t) => {
  const { page, close } = await openExplore("home", {
    "/api/dashboard/home": readFileSync(`${FIXTURES_ROOT}/home/dashboard_home.empty.json`, "utf8"),
  });
  t.after(close);

  await page.waitForSelector(".home-dashboard:not(.loading)");
  const ids = [
    "walletBalance",
    "heroCash",
    "heroHoldings",
    "heroOpenPnl",
    "heroRealizedToday",
    "positionsAvgSize",
    "positionsAvgHold",
  ];
  const values = {};
  for (const id of ids) values[id] = (await page.locator(`#${id}`).textContent()).trim();
  assert.deepEqual(values, Object.fromEntries(ids.map((id) => [id, "—"])));
});

test("the Buy and Sell dialogs state that trading needs setup", async (t) => {
  const { page, requests, close } = await openExplore("home");
  t.after(close);
  const dialog = page.locator(".trade-action-dialog-overlay.is-visible");
  await page.waitForSelector(".home-dashboard:not(.loading)");
  const before = requests.length;

  for (const action of ["buy", "sell"]) {
    await page.evaluate(async (tradeAction) => {
      const { manualTrade } = await import("/scripts/ui/manual_trade.js");
      void manualTrade({
        action: tradeAction,
        mint: "So11111111111111111111111111111111111111112",
        symbol: "SOL",
      });
    }, action);
    // The balance stays above, so the requirement is a notice clear of it, not a card
    // nested in the panel.
    const gate = dialog.locator(".trade-action-setup .notice");
    await gate.waitFor();
    assert.equal(
      await gate.locator(".notice-title").textContent(),
      "Trading needs a wallet and RPC"
    );
    assert.equal(await gate.locator("button").textContent(), "Complete setup");
    assert.equal(await dialog.locator(".trade-action-setup .access-gate").count(), 0);
    const gap = await dialog.evaluate((root) => {
      const grid = root.querySelector(".trade-action-context-grid").getBoundingClientRect();
      const notice = root.querySelector(".trade-action-setup .notice").getBoundingClientRect();
      return notice.top - grid.bottom;
    });
    assert.ok(gap >= 8, `the notice sits clear of the balance (${gap}px)`);

    const presets = dialog.locator(".trade-action-preset-btn");
    assert.ok((await presets.count()) > 0, `${action} offers quick amounts`);
    for (const preset of await presets.all()) {
      assert.equal(await preset.isDisabled(), true, `${action} quick amounts are disabled`);
    }
    assert.equal(await dialog.locator(".trade-action-input-max").isDisabled(), true);
    const confirm = dialog.locator(".trade-action-footer .trade-action-btn-confirm");
    assert.equal(await confirm.isDisabled(), true);
    assert.equal(await confirm.getAttribute("title"), "Trading needs a wallet and RPC");

    if (action === "buy") {
      await dialog.locator(".trade-action-footer .trade-action-btn-cancel").click();
      await page.waitForFunction(
        () => !document.querySelector(".trade-action-dialog-overlay.is-visible")
      );
    }
  }

  await dialog.locator('.trade-action-setup [data-action="open-setup"]').click();
  await page.locator("body.initialization-mode #setupScreen").waitFor({ state: "visible" });
  assert.equal(await dialog.count(), 0, "the setup wizard replaces the trade dialog");
  assert.deepEqual(
    requests.slice(before).filter((request) => / \/api\/wallet\/balance\b/.test(request)),
    []
  );
});

test("Complete setup opens the first-run flow over the dashboard and Explore returns to it", async (t) => {
  const { page, requests, close } = await openExplore("home");
  t.after(close);
  const wizard = page.locator("#setupScreen");
  const introduction = page.locator("#onboardingScreen");
  const covering = () =>
    page.evaluate(() => document.body.classList.contains("initialization-mode"));

  await page.locator("#accessControl").click();
  await wizard.waitFor({ state: "visible" });
  assert.equal(await covering(), true);
  assert.equal(await page.locator(".app-main").isVisible(), false, "the wizard covers the page");
  assert.equal(await page.locator("#setup-wallet-source :checked").count(), 0);
  assert.equal(await page.locator("#setup-next").isDisabled(), true);

  await page.click("#setup-back");
  await introduction.waitFor({ state: "visible" });
  assert.equal(await wizard.isVisible(), false);
  await page.click("#onboardingSetupShortcut");
  await wizard.waitFor({ state: "visible" });
  assert.equal(await page.locator("#setup-wallet-source :checked").count(), 0);

  const before = requests.length;
  await page.click("#setup-explore");
  await page.locator(".app-main").waitFor({ state: "visible" });
  assert.equal(await covering(), false);
  assert.equal(await wizard.isVisible(), false);
  assert.equal(await introduction.isVisible(), false);
  assert.deepEqual(
    requests.slice(before).filter((request) => request.includes("/api/initialization/explore")),
    [],
    "Explore Mode is already on"
  );
});

test("Copy Trading on the service shows the own-RPC notice above the kept tasks", async (t) => {
  const { page, close } = await openService("copy");
  t.after(close);

  const gate = page.locator("#copy-access-gate .notice");
  await gate.waitFor();
  assert.equal(await gate.locator(".notice-title").textContent(), "Needs your own RPC");
  assert.equal(plain(await gate.locator(".notice-message").textContent()), WATCH_REASON);
  assert.equal(await gate.locator("button").textContent(), "Open Connection settings");
  await page.waitForSelector("#copy-main:not([hidden])");
  const add = page.locator("#copy-add");
  assert.equal(await add.isDisabled(), true);
  assert.equal(plain(await add.getAttribute("title")), WATCH_REASON);
  assert.equal(
    await page
      .locator('#copy-summary [data-summary-id="status"] .table-toolbar-chip__value')
      .textContent(),
    "Needs your own RPC"
  );

  await gate.locator("button").click();
  await page.locator('.settings-nav-item[data-tab="connection"].active').waitFor();
});

test("Wallets > Watched on the service is the gate and reads no watch route", async (t) => {
  const { page, requests, close } = await openService("wallets");
  t.after(close);

  await page.locator('[data-tab-id="watched"], [data-tab="watched"]').first().click();
  const gate = page.locator("#watched-access-gate .access-gate");
  await gate.waitFor();
  assert.equal(plain(await gate.locator(".state-view-message").textContent()), WATCH_REASON);
  assert.equal(await page.locator("#watched-wallets-root").isVisible(), false);
  await page.waitForTimeout(500);
  assert.deepEqual(
    requests.filter((request) => / \/api\/wallets\/watch\b/.test(request)),
    []
  );
});

const SERVICE_NOTICE = "#stats-notices .notice:has(.icon-cloud)";

test("the Auto Trader shows the service notice until it is closed for the session", async (t) => {
  const { page, close } = await openService("trader");
  t.after(close);

  // It sits in the Stats notice area, which scrolls with the sheet, never pinned over it.
  const notice = page.locator(SERVICE_NOTICE);
  await notice.waitFor();
  assert.equal(
    await page.evaluate(() =>
      document.querySelector("#trader-content").contains(document.querySelector("#stats-notices"))
    ),
    true
  );
  assert.equal(
    plain(await notice.locator(".notice-title").textContent()),
    "Running on the ScreenerBot service"
  );
  assert.equal(
    await notice.locator('[data-action="open-connection"]').textContent(),
    "Open Connection settings"
  );
  await notice.locator('[data-action="dismiss"]').click();
  assert.equal(await page.locator(SERVICE_NOTICE).count(), 0);
  await page.reload();
  await page.waitForSelector(PAGE_READY);
  await page.locator("#stats-state").waitFor();
  await page.waitForTimeout(300);
  assert.equal(await page.locator(SERVICE_NOTICE).count(), 0);
});

test("own RPC shows no access gate and no service notice", async (t) => {
  const { page, close } = await openWith(fixtureBootstrap(), "trader", {});
  t.after(close);
  await page.locator("#stats-state").waitFor();
  await page.waitForTimeout(300);
  assert.equal(await page.locator(SERVICE_NOTICE).count(), 0);
});

function fixtureBootstrap() {
  return JSON.stringify({
    ...fixture("shell/system_bootstrap.json"),
    chain_access: {
      ...SERVICE_ACCESS,
      mode: "own_rpc",
      capabilities: Object.fromEntries(
        Object.keys(SERVICE_ACCESS.capabilities).map((key) => [key, true])
      ),
    },
  });
}
