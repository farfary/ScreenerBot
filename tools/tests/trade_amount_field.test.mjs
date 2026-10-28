// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the trade dialog's amount field keeps its text, unit and Max apart in every
 * shipped locale.
 *
 * The unit and Max button once sat at fixed offsets over the input, sized for the
 * English labels; a longer Max label (Persian "حداکثر") covered the unit, and the
 * placeholder ran under both in right-to-left text. The field is now a row, so the
 * check is geometric: the typing area, the unit and Max never overlap and all three
 * stay inside the field, for a buy (unit SOL, Max) and a sell (unit %, Max 100%).
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const MINT = "7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr";
const LOCALES = JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/i18n_locales.json`, "utf8"))
  .locales.map(({ code }) => code);

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [shell, home] = await Promise.all([loadIndex("shell"), loadIndex("home")]);

/** Rectangles of the field and its three parts, and the parts that overlap. */
function measure() {
  const rect = (selector) => {
    const box = document.querySelector(selector).getBoundingClientRect();
    return { left: box.left, right: box.right, top: box.top, bottom: box.bottom };
  };
  const field = rect(".trade-action-input-wrapper");
  const parts = {
    input: rect(".trade-action-input"),
    unit: rect(".trade-action-input-suffix"),
    max: rect(".trade-action-input-max"),
  };
  const names = Object.keys(parts);
  const overlaps = [];
  names.forEach((a, i) =>
    names.slice(i + 1).forEach((b) => {
      const x = Math.min(parts[a].right, parts[b].right) - Math.max(parts[a].left, parts[b].left);
      const y = Math.min(parts[a].bottom, parts[b].bottom) - Math.max(parts[a].top, parts[b].top);
      if (x > 0.5 && y > 0.5) overlaps.push(`${a}/${b}`);
    })
  );
  const outside = names.filter(
    (name) => parts[name].left < field.left - 0.5 || parts[name].right > field.right + 0.5
  );
  return { overlaps, outside };
}

for (const locale of LOCALES) {
  test(`${locale}: the amount, its unit and Max never overlap`, async (t) => {
    const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
    const fixtures = createApiHandler([home, shell], "populated");
    const host = await serveDashboard(context, {
      locale,
      onApi: (request) => {
        const path = request.url.pathname;
        if (path === "/api/wallet/balance") return { body: JSON.stringify({ sol_balance: 10 }) };
        if (path === `/api/wallet/buy-account-rent/${MINT}`) {
          return { body: JSON.stringify({ rent_sol: 0, fee_reserve_sol: 0.002 }) };
        }
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
    await page.waitForSelector(PAGE_READY);

    for (const action of ["buy", "sell"]) {
      await page.evaluate(
        async ({ mint, tradeAction }) => {
          const { TradeActionDialog } = await import("/scripts/ui/trade_action_dialog.js");
          window.__dialog?.close({ restoreFocus: false });
          window.__dialog = new TradeActionDialog();
          void window.__dialog.open({
            action: tradeAction,
            symbol: "TEST",
            context: { mint, balance: 10, entrySize: 0.1, tokenBalance: 1000 },
          });
        },
        { mint: MINT, tradeAction: action }
      );
      await page.locator(".trade-action-dialog-overlay.is-visible .trade-action-input").waitFor();
      assert.deepEqual(await page.evaluate(measure), { overlaps: [], outside: [] }, action);
    }
  });
}
