// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the token details Pools tab states each fact once. The summary grid fills whole
 * rows (no lone cell beside an empty one), the pool count lives only in the All pools
 * heading, and a pool card never lists one address under two labels.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [tokens, shell] = await Promise.all([loadIndex("tokens"), loadIndex("shell")]);

test("the Pools tab fills its summary grid and repeats no address", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([tokens, shell], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}/tokens`);
  await page.waitForSelector(READY);
  await page.click("#tokens-root tr[data-row-id] .ti-row-cell__symbol");
  await page.click('.token-details-dialog [data-dialog-tab="pools"]');
  await page.waitForSelector(".token-details-dialog .pools-summary-grid .pools-summary-fact");

  const layout = await page.evaluate(() => {
    const grid = document.querySelector(".token-details-dialog .pools-summary-grid");
    const columns = getComputedStyle(grid).gridTemplateColumns.split(" ").length;
    const cards = [...document.querySelectorAll(".token-details-dialog .pool-detail")];
    return {
      columns,
      facts: grid.querySelectorAll(".pools-summary-fact").length,
      cards: cards.length,
      repeated: cards.flatMap((card) => {
        const addresses = [
          ...card.querySelectorAll(".pool-detail-addresses .ti-address-copy[data-copy]"),
        ].map((button) => button.dataset.copy);
        return addresses.filter((address, index) => addresses.indexOf(address) !== index);
      }),
    };
  });

  assert.ok(layout.cards > 0, "the fixture token lists at least one pool");
  assert.equal(
    layout.facts % layout.columns,
    0,
    `${layout.facts} facts leave a lone cell in ${layout.columns} columns`
  );
  assert.deepEqual(layout.repeated, [], "a pool card lists one address under two labels");
});
