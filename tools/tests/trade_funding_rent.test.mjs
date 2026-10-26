// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the trade dialog's funding figure counts the rent of the accounts a buy opens.
 *
 * The server reports the rent of every account the buy opens (the wrapped-SOL account
 * and a token account the trading wallet does not keep yet), counted as the swap's own
 * funds check counts them; "this trade needs X SOL" adds it to the fee reserve. A wallet
 * that already keeps both pays no rent, and the figure is the amount plus the reserve.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const MINT = "7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr";
const BALANCE = 0.011;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [shell, home] = await Promise.all([loadIndex("shell"), loadIndex("home")]);

/**
 * Opens a buy dialog for MINT on a wallet holding BALANCE, with the server reporting
 * `rentSol` for the token account the buy opens, types `amount` and returns the
 * refusal the dialog shows.
 */
async function buyRefusal(rentSol, amount) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const fixtures = createApiHandler([home, shell], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      const path = request.url.pathname;
      if (path === "/api/wallet/balance") {
        return { body: JSON.stringify({ sol_balance: BALANCE }) };
      }
      if (path === `/api/wallet/buy-account-rent/${MINT}`) {
        return { body: JSON.stringify({ rent_sol: rentSol }) };
      }
      // The quote stays pending, so the amount check alone decides the refusal.
      if (path === "/api/trader/quote") return new Promise(() => {});
      return fixtures.onApi(request);
    },
  });
  try {
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/home`);
    await page.waitForSelector(PAGE_READY);
    await Promise.all([
      page.waitForResponse((response) => response.url().endsWith(`/buy-account-rent/${MINT}`)),
      page.evaluate(
        async ({ mint, balance }) => {
          const { TradeActionDialog } = await import("/scripts/ui/trade_action_dialog.js");
          window.__dialog = new TradeActionDialog();
          window.__dialog.open({
            action: "buy",
            symbol: "TEST",
            context: { mint, balance, entrySize: 0.01 },
          });
        },
        { mint: MINT, balance: BALANCE }
      ),
    ]);
    await page.waitForFunction(() => window.__dialog.currentContext.accountRent != null);
    await page.fill(".trade-action-input", String(amount));
    const error = page.locator('.trade-action-error-msg[data-visible="true"]');
    await error.waitFor();
    return (await error.textContent()).trim();
  } finally {
    await context.close();
    await host.close();
  }
}

test("a buy that opens accounts needs their rent too", async () => {
  assert.equal(
    await buyRefusal(0.002, 0.01),
    `The wallet holds ${BALANCE} SOL; this trade needs 0.014 SOL`
  );
});

test("a buy into kept accounts needs the amount and the reserve only", async () => {
  assert.equal(
    await buyRefusal(0, 0.01),
    `The wallet holds ${BALANCE} SOL; this trade needs 0.012 SOL`
  );
});
