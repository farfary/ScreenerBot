// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: in Explore Mode the wallet-backed pages state that setup is needed and
 * never call the routes a full setup opens.
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
 * - Tools shows the setup notice in place of a wallet-backed tool and keeps its
 *   actions visible, disabled with the reason.
 * - Home shows no wallet figure: the worth the API cannot know arrives as null and
 *   every portfolio, P&L and exposure value reads "—", never a measured zero.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

const WAIT_MS = 15000;
const REASON = "Explore Mode runs without a wallet or RPC. Complete setup to connect them.";

const fixture = (path) => JSON.parse(readFileSync(`${FIXTURES_ROOT}/${path}`, "utf8"));
const EXPLORE_BOOTSTRAP = JSON.stringify({
  ...fixture("shell/system_bootstrap.json"),
  initialization_complete: false,
  explore_mode: true,
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
async function openExplore(id, overrides = {}) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex(id), shellIndex], "populated");
  const requests = [];
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      requests.push(`${request.method} ${request.url.pathname}`);
      if (request.url.pathname === "/api/system/bootstrap") return { body: EXPLORE_BOOTSTRAP };
      const body = overrides[request.url.pathname];
      return body ? { body } : api.onApi(request);
    },
  });
  const page = await context.newPage();
  page.setDefaultTimeout(WAIT_MS);
  await page.goto(`${host.origin}/${id}`);
  await page.waitForSelector("body:not(.initialization-mode) main.content:not([data-loading])");
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { page, requests, close };
}

test("Wallets shows the setup notice and reads no wallet route", async (t) => {
  const { page, requests, close } = await openExplore("wallets");
  t.after(close);

  const gate = page.locator("#wallets-setup-gate .setup-gate");
  await gate.waitFor();
  assert.equal(await gate.locator(".empty-state-title").textContent(), "Wallets need setup");
  assert.equal(await gate.locator(".empty-state-description").textContent(), REASON);
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

  const gate = page.locator("#copy-setup-gate .setup-gate");
  await gate.waitFor();
  assert.equal(
    await gate.locator(".empty-state-title").textContent(),
    "Copy trading needs a wallet"
  );
  assert.equal(await page.locator("#copy-onboarding").isVisible(), false);
  assert.equal(await page.locator("#copy-add").isVisible(), false, "one call to action");
  assert.equal(
    await page.locator("#copy-system-state").textContent(),
    "Setup required · copy trading needs a wallet and RPC"
  );
  const global = page.locator("#copy-global-action");
  assert.equal(await global.isDisabled(), true);
  assert.equal(await global.getAttribute("title"), REASON);

  const populated = await openExplore("copy");
  t.after(populated.close);
  const add = populated.page.locator("#copy-add");
  await populated.page.waitForSelector("#copy-main:not([hidden])");
  assert.equal(await populated.page.locator("#copy-setup-gate").isVisible(), false);
  assert.equal(await add.isVisible(), true);
  assert.equal(await add.isDisabled(), true);
  assert.equal(await add.getAttribute("title"), REASON);
});

test("Transactions shows the setup notice and reads no transaction route", async (t) => {
  const { page, requests, close } = await openExplore("transactions");
  t.after(close);

  const gate = page.locator("#transactions-setup-gate .setup-gate");
  await gate.waitFor();
  assert.equal(
    await gate.locator(".empty-state-title").textContent(),
    "Transactions need a wallet"
  );
  assert.equal(await gate.locator(".empty-state-description").textContent(), REASON);
  assert.equal(await page.locator("#transactions-root").isVisible(), false);
  await page.waitForTimeout(1000);
  assert.deepEqual(
    requests.filter((request) => / \/api\/(?:transactions|wallets)\b/.test(request)),
    []
  );
  assert.equal(await page.locator(".toast").count(), 0);
});

test("Tools gates a wallet-backed tool with the setup notice", async (t) => {
  const { page, close } = await openExplore("tools");
  t.after(close);

  const gate = page.locator("#tools-content .setup-gate");
  await gate.waitFor();
  assert.equal(await gate.locator(".empty-state-title").textContent(), "This tool needs a wallet");
  assert.equal(await gate.locator(".empty-state-description").textContent(), REASON);
  const scan = page.locator("#tool-actions #scan-atas-btn");
  assert.equal(await scan.isDisabled(), true);
  assert.equal(await scan.getAttribute("title"), REASON);
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
