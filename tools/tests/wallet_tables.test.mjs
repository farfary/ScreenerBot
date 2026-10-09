// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Wallets page names a wallet one way and states each figure once. Every wallet
 * name in the Secondaries, Archive and Watched tables uses one font and weight, and no
 * toolbar chip repeats its label inside its value ("SOL 4.2187 SOL"). A holding worth less
 * than the Value column's resolution keeps its digits in subscript notation rather than
 * reading "0.0000"; only a holding without a price reads "—".
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";
const LIST_TABS = ["secondaries", "archive", "watched"];

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [wallets, shell] = await Promise.all([loadIndex("wallets"), loadIndex("shell")]);

async function openWallets(t, answers = {}) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([wallets, shell], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      const body = answers[request.url.pathname];
      return body ? { body } : api.onApi(request);
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}/wallets`);
  await page.waitForSelector(READY);
  return page;
}

test("toolbar chips state their label once", async (t) => {
  const page = await openWallets(t);
  await page.waitForSelector('.table-toolbar-chip[data-summary-id="wt-sol-balance"]');
  const repeated = await page.$$eval(".table-toolbar-chip", (chips) =>
    chips
      .filter((chip) => chip.getClientRects().length)
      .map((chip) => ({
        label: chip.querySelector(".table-toolbar-chip__label")?.textContent.trim() || "",
        value: chip.querySelector(".table-toolbar-chip__value")?.textContent.trim() || "",
      }))
      .filter(({ label, value }) => label && value.includes(label))
      .map(({ label, value }) => `${label} ${value}`)
  );
  assert.deepEqual(repeated, [], "a toolbar chip repeats its label in its value");
});

test("every wallet table names a wallet in one font and weight", async (t) => {
  const page = await openWallets(t);
  const styles = {};
  for (const tab of LIST_TABS) {
    await page.click(`[data-tab-id="${tab}"]`);
    await page.waitForSelector(`.ti-named-address-name >> visible=true`);
    styles[tab] = await page.$$eval(".ti-named-address-name", (names) => [
      ...new Set(
        names
          .filter((name) => name.getClientRects().length)
          .map((name) => {
            const style = getComputedStyle(name);
            return `${style.fontFamily} ${style.fontWeight}`;
          })
      ),
    ]);
  }
  const all = [...new Set(Object.values(styles).flat())];
  for (const tab of LIST_TABS) {
    assert.ok(styles[tab].length > 0, `the ${tab} tab lists named wallets`);
  }
  assert.equal(all.length, 1, `wallet names render in ${JSON.stringify(styles)}`);
});

test("the holdings Value column keeps a tiny value's digits and dashes only an unpriced one", async (t) => {
  const holdings = JSON.parse(readFileSync(`${FIXTURES_ROOT}/wallets/wallet_tokens.json`, "utf8"));
  const [tiny, zero, unpriced] = holdings.tokens;
  tiny.value_sol = 5.29e-9;
  zero.value_sol = 0;
  unpriced.price_sol = null;
  unpriced.value_sol = null;
  const page = await openWallets(t, { "/api/wallet/tokens": JSON.stringify(holdings) });
  const cell = (mint) =>
    page
      .locator(`#tokens-datatable-root tr[data-row-id="${mint}"] td[data-column-id="value_sol"]`)
      .evaluate((node) => node.textContent.replace(/[\u2066-\u2069]/g, "").trim());
  await page.waitForSelector(`#tokens-datatable-root tr[data-row-id="${tiny.mint}"]`);
  assert.equal(await cell(tiny.mint), "0.0₈5290");
  assert.equal(await cell(zero.mint), "0.0000");
  assert.equal(await cell(unpriced.mint), "—");
});
