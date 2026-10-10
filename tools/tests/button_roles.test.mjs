// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a button role looks the same wherever it appears.
 *
 * The page-level primary button filled with `--button-primary-bg` while the DataTable
 * toolbar, the dialogs and the sign-in screens filled theirs with the lighter
 * `--primary-color`, at a different height. Watched wallets drew its row Remove as a
 * filled red square in every row while the sibling tables used neutral outline icons.
 *
 * - Every primary, submit or confirm button fills with the button primary token.
 * - The Copy Trading "Add wallet" and the Wallets toolbar "Add Wallet" share one fill and
 *   one height.
 * - A destructive row action rests on the same neutral surface as its neighbours.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { loadStylesheets } from "../lib/dashboard_ui.mjs";

// Rules in stylesheets that no page loads any more; the list only shrinks.
const UNUSED_RULES = new Set([
  "pages/trader.css .trader-action-bar .btn-primary",
  "pages/trader.css .form-actions .btn.primary",
]);

const tab = (id) => `#subTabsContainer [data-tab-id="${id}"]`;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const shell = await loadIndex("shell");

async function openPage(t, name) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex(name), shell], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/${name}`);
  await page.waitForSelector(PAGE_READY);
  return page;
}

const skin = (locator) =>
  locator.evaluate((el) => {
    const style = getComputedStyle(el);
    return {
      background: style.backgroundColor,
      height: Math.round(el.getBoundingClientRect().height),
    };
  });

test("every primary, submit or confirm button fills with the button primary token", async () => {
  const found = [];
  for (const sheet of await loadStylesheets()) {
    for (const rule of sheet.rules) {
      if (!/(btn|button|submit)/i.test(rule.selector)) continue;
      if (!/(primary|submit|confirm)/i.test(rule.selector)) continue;
      if (!/background(-color)?:\s*var\(--primary(-color|-hover)?[,)]/.test(rule.body)) continue;
      const key = `${sheet.path} ${rule.selector.replace(/:hover.*$/, "")}`;
      if (!UNUSED_RULES.has(key)) found.push(`${sheet.path}:${rule.line} ${rule.selector}`);
    }
  }
  assert.deepEqual(found, []);
});

test("the page and toolbar primary buttons share one fill and one height", async (t) => {
  const copy = await openPage(t, "copy");
  await copy.waitForSelector("#copy-add:not([hidden])");
  const pageButton = await skin(copy.locator("#copy-add"));

  const wallets = await openPage(t, "wallets");
  await wallets.click(tab("secondaries"));
  const toolbarButton = wallets.locator(".table-toolbar-btn--primary:visible").first();
  await toolbarButton.waitFor();
  assert.deepEqual(await skin(toolbarButton), pageButton);
});

test("a destructive row action rests on the neutral row-action surface", async (t) => {
  const page = await openPage(t, "wallets");
  await page.click(tab("watched"));
  const remove = page.locator('#watched-wallets-root [data-action-id="delete"]').first();
  await remove.waitFor();
  const neighbour = page
    .locator('#watched-wallets-root .dt-action-btn:not([data-action-id="delete"])')
    .first();
  const [removeSkin, neighbourSkin] = await Promise.all([
    remove.evaluate((el) => getComputedStyle(el).backgroundColor),
    neighbour.evaluate((el) => getComputedStyle(el).backgroundColor),
  ]);
  assert.equal(removeSkin, neighbourSkin);
});
