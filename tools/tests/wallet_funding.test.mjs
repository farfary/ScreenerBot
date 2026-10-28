// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a trading wallet without SOL is told how to start, and a buy it cannot fund
 * says why its confirm is disabled.
 *
 * - Wallets shows the funding notice only while the main wallet is known to hold no
 *   SOL, without repeating the address its holdings toolbar already carries, and sets
 *   it off the toolbar by the same space that insets it from the page edge.
 * - The Buy dialog shows the same notice at 0 SOL and names the refusal inline in the
 *   trade API's own words and figures: what the wallet holds and what the trade needs.
 * - A wallet with SOL but not enough for the typed amount gets the same refusal; an
 *   unread balance is unknown, never 0, and shows no notice.
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

const MINT = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";
const NOTICE_TITLE = "Fund your trading wallet";
const ERROR = ".trade-action-error-msg[data-visible='true'] .trade-action-error-text";
// The network fee reserve the buy-funding route serves; the wallet keeps every account.
const FEE_RESERVE = 0.002;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const strip = (text) => text.replace(/[⁨⁩]/g, "").trim();

const wallets = JSON.parse(
  await readFile(join(FIXTURES_ROOT, "wallets/wallets_list.json"), "utf8")
);
const main = wallets.wallets.find((wallet) => wallet.role === "main");

/** Opens Wallets with the main wallet at `balance` and the balance route answering `current`. */
async function open(balance, current) {
  const list = {
    ...wallets,
    wallets: wallets.wallets.map((wallet) =>
      wallet.role === "main" ? { ...wallet, balance } : wallet
    ),
  };
  const context = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const api = createApiHandler([await loadIndex("wallets"), await loadIndex("shell")], "populated");
  const onApi = async (request) => {
    const path = request.url.pathname;
    if (path === "/api/wallets") return { body: JSON.stringify(list) };
    if (path === "/api/wallet/balance") return current;
    if (path === `/api/wallet/buy-account-rent/${MINT}`) {
      return { body: JSON.stringify({ rent_sol: 0, fee_reserve_sol: FEE_RESERVE }) };
    }
    return api.onApi(request);
  };
  const host = await serveDashboard(context, { locale: "en", onApi });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/wallets`);
  await page.waitForSelector(PAGE_READY);
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { page, close };
}

const balanceBody = (sol) => ({
  body: JSON.stringify({
    sol_balance: sol,
    native_balance_raw: Math.round(sol * 1e9),
    total_tokens_count: 0,
    token_balances: [],
    snapshot_time: "2026-10-26T12:00:00Z",
  }),
});

async function openBuy(page) {
  await page.evaluate(async (mint) => {
    const { manualTrade } = await import("/scripts/ui/manual_trade.js");
    void manualTrade({ action: "buy", mint, symbol: "JUP" });
  }, MINT);
  await page.waitForSelector(".trade-action-dialog-overlay.is-visible .trade-action-context-grid");
}

test("Wallets shows funding guidance only while the main wallet holds no SOL", async () => {
  const empty = await open(0, balanceBody(0));
  try {
    const notice = empty.page.locator("#wallets-funding .notice");
    await notice.waitFor();
    assert.match(strip(await notice.textContent()), new RegExp(NOTICE_TITLE));
    assert.ok(!strip(await notice.textContent()).includes(main.address));
    assert.equal(await notice.locator("[data-copy]").count(), 0);
    await empty.page.locator(`#tokens-datatable-root [data-copy="${main.address}"]`).waitFor();
    const spacing = await empty.page.evaluate(() => {
      const panel = document.querySelector("#panel-main").getBoundingClientRect();
      const box = document.querySelector("#wallets-funding").getBoundingClientRect();
      const table = document.querySelector("#tokens-datatable-root").getBoundingClientRect();
      return { inset: Math.round(box.left - panel.left), below: Math.round(table.top - box.bottom) };
    });
    assert.ok(spacing.inset > 0, "the notice is inset from the page edge");
    assert.equal(spacing.below, spacing.inset, "the notice is set off the holdings toolbar");
  } finally {
    await empty.close();
  }

  const funded = await open(4.2, balanceBody(4.2));
  try {
    await funded.page.waitForSelector(
      "#tokens-datatable-root .data-table, #tokens-datatable-root table"
    );
    assert.equal(await funded.page.locator("#wallets-funding").isVisible(), false);
  } finally {
    await funded.close();
  }
});

test("the Buy dialog at 0 SOL shows the notice and says why it cannot buy", async () => {
  const { page, close } = await open(0, balanceBody(0));
  try {
    await openBuy(page);
    const notice = page.locator(".trade-action-funding .notice");
    await notice.waitFor();
    await page.waitForFunction(
      (address) =>
        document.querySelector(".trade-action-funding .notice")?.textContent.includes(address),
      main.address
    );
    const error = strip(await page.locator(ERROR).textContent());
    assert.match(error, /^The wallet holds 0 SOL; this trade needs [\d.,]+ SOL$/);
    assert.equal(
      await page.locator(".trade-action-footer .trade-action-btn-confirm").isDisabled(),
      true
    );
  } finally {
    await close();
  }
});

test("a buy larger than the balance is refused inline with both figures", async () => {
  const { page, close } = await open(1, balanceBody(1));
  try {
    await openBuy(page);
    assert.equal(await page.locator(".trade-action-funding").count(), 0);
    await page.locator(".trade-action-input").fill("5");
    const refusal = "The wallet holds 1 SOL; this trade needs 5.002 SOL";
    // The dialog re-validates the typed amount once the served reserve arrives.
    await page
      .waitForFunction(
        ([selector, text]) =>
          document
            .querySelector(selector)
            ?.textContent.replace(/[\u2068\u2069]/g, "")
            .trim() === text,
        [ERROR, refusal]
      )
      .catch(() => {});
    assert.equal(strip(await page.locator(ERROR).textContent()), refusal);
    assert.equal(
      await page.locator(".trade-action-footer .trade-action-btn-confirm").isDisabled(),
      true
    );
  } finally {
    await close();
  }
});

test("an unread balance shows no funding notice", async () => {
  const { page, close } = await open(null, {
    status: 503,
    body: JSON.stringify({
      error: {
        code: "SERVICE_UNAVAILABLE",
        message: "unread",
        text: { id: "errors-wallet-read-failed" },
      },
    }),
  });
  try {
    assert.equal(await page.locator("#wallets-funding").isVisible(), false);
    await openBuy(page);
    await page.waitForSelector(".trade-action-context-grid");
    assert.equal(await page.locator(".trade-action-funding").count(), 0);
  } finally {
    await close();
  }
});
