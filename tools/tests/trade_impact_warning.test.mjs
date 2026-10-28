// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the trade dialog's high-impact warning answers its own keys.
 *
 * A quote whose price impact exceeds the slippage tolerance opens a confirmation over the
 * trade dialog. While it is open, the trade dialog ignores Enter and refuses a second
 * confirm, so Enter confirms the warning instead of replacing it with a new one, and the
 * trade resolves once with the amount on screen.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const MINT = "7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr";
const AMOUNT = 0.1;
/** A quote whose price impact is far above any slippage tolerance. */
const HIGH_IMPACT_QUOTE = {
  success: true,
  input_amount: AMOUNT,
  output_amount: 1000,
  output_formatted: "1000",
  minimum_output_amount: 900,
  network_fee_sol: 0.000005,
  platform_fee_native: 0.0005,
  platform_fee_pct: 0.5,
  price_impact_pct: 50,
  price_per_token_native: 0.0001,
  route: "Test",
  router: "jupiter",
  slippage_bps: 100,
};

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [shell, home] = await Promise.all([loadIndex("shell"), loadIndex("home")]);

test("Enter on the high-impact warning confirms it and the trade resolves once", async () => {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const fixtures = createApiHandler([home, shell], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      const path = request.url.pathname;
      if (path === "/api/wallet/balance") {
        return { body: JSON.stringify({ sol_balance: 10 }) };
      }
      if (path === `/api/wallet/buy-account-rent/${MINT}`) {
        return { body: JSON.stringify({ rent_sol: 0, fee_reserve_sol: 0.002 }) };
      }
      if (path === "/api/trader/quote") {
        return { body: JSON.stringify(HIGH_IMPACT_QUOTE) };
      }
      return fixtures.onApi(request);
    },
  });
  try {
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/home`);
    await page.waitForSelector(PAGE_READY);
    await page.evaluate(async (mint) => {
      const { TradeActionDialog } = await import("/scripts/ui/trade_action_dialog.js");
      window.__warnings = 0;
      new window.MutationObserver((records) => {
        for (const record of records) {
          for (const node of record.addedNodes) {
            if (node.classList?.contains("confirmation-dialog")) window.__warnings += 1;
          }
        }
      }).observe(document.body, { childList: true });
      window.__results = [];
      window.__dialog = new TradeActionDialog();
      window.__dialog
        .open({ action: "buy", symbol: "TEST", context: { mint, balance: 10, entrySize: 0.1 } })
        .then((result) => window.__results.push(result));
    }, MINT);
    await page.waitForFunction(() => window.__dialog.currentContext.feeReserve != null);
    await page.fill(".trade-action-input", String(AMOUNT));
    await page.waitForFunction((amount) => window.__dialog._quotedAmount === amount, AMOUNT);

    await page.focus(".trade-action-input");
    await page.keyboard.press("Enter");
    const warning = page.locator(".confirmation-dialog");
    await warning.waitFor();

    await page.keyboard.press("Enter");
    await page.waitForFunction(() => window.__results.length === 1);
    const [result] = await page.evaluate(() => window.__results);
    assert.equal(Number(result.amount), AMOUNT, "the trade resolves with the amount on screen");
    assert.equal(
      await page.evaluate(() => window.__warnings),
      1,
      "Enter confirmed the warning instead of opening another"
    );
  } finally {
    await context.close();
    await host.close();
  }
});
