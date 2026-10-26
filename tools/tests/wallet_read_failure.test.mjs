// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a wallet that cannot be read shows unknown figures with the reason, never an
 * empty or zero wallet.
 *
 * While the last wallet read failed and no snapshot exists, the server refuses the
 * balance and holdings with the cause. Home keeps the dash and says the balance is
 * unavailable, with the cause on hover; Wallets names the holdings unknown, gives the
 * cause in place of "No token holdings", and counts no tokens.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex, FIXTURES_ROOT } from "../lib/dashboard_fixtures.mjs";

const CAUSE = "getBalance: HTTP status: 403 Forbidden (Request blocked)";
const READ_FAILED = {
  status: 503,
  body: JSON.stringify({
    error: {
      code: "SERVICE_UNAVAILABLE",
      message: "The wallet cannot be read",
      text: { id: "errors-wallet-read-failed", args: { reason: { type: "text", value: CAUSE } } },
    },
  }),
};

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const strip = (text) => text.replace(/[⁨⁩]/g, "").trim();

async function open(page, indexes, overrides) {
  const context = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const api = createApiHandler(indexes, "populated");
  const onApi = async (request) => overrides(request.url.pathname) ?? api.onApi(request);
  const host = await serveDashboard(context, { locale: "en", onApi });
  const tab = await context.newPage();
  tab.setDefaultTimeout(15000);
  await tab.goto(`${host.origin}/${page}`);
  await tab.waitForSelector(PAGE_READY);
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { tab, close };
}

test("Home says the balance is unavailable and why", async () => {
  const home = JSON.parse(await readFile(join(FIXTURES_ROOT, "home/dashboard_home.json"), "utf8"));
  const unknown = {
    ...home,
    wallet: {
      ...home.wallet,
      current_balance_native: null,
      tokens_worth_native: null,
      total_equity_native: null,
      token_count: null,
      unpriced_token_count: null,
    },
  };
  const { tab, close } = await open(
    "home",
    [await loadIndex("home"), await loadIndex("shell")],
    (path) => {
      if (path === "/api/dashboard/home") return { body: JSON.stringify(unknown) };
      if (path === "/api/wallet/balance") return READ_FAILED;
      return null;
    }
  );
  try {
    await tab.waitForSelector("#walletUsd[data-tooltip]");
    assert.equal(strip(await tab.textContent("#walletBalance")), "—");
    assert.equal(strip(await tab.textContent("#walletUsd")), "Balance unavailable");
    assert.equal(
      strip(await tab.getAttribute("#walletUsd", "data-tooltip")),
      `The wallet cannot be read: ${CAUSE}`
    );
  } finally {
    await close();
  }
});

test("Wallets names unknown holdings with the cause", async () => {
  const { tab, close } = await open(
    "wallets",
    [await loadIndex("wallets"), await loadIndex("shell")],
    (path) => (path === "/api/wallet/tokens" ? READ_FAILED : null)
  );
  try {
    await tab.waitForFunction(() =>
      document.querySelector("#tokens-datatable-root")?.textContent.includes("Holdings unknown")
    );
    const text = strip(await tab.textContent("#tokens-datatable-root"));
    assert.ok(text.includes(`The wallet cannot be read: ${CAUSE}`), text);
    assert.ok(!text.includes("No token holdings"), text);
    const count = await tab.$eval('[data-summary-id="wt-tokens-count"], #wt-tokens-count', (el) =>
      el.textContent.trim()
    );
    assert.match(count, /—/);
  } finally {
    await close();
  }
});
