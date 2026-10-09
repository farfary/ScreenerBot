// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the transaction details dialog states amounts and counts the way it lists them.
 * Every amount drops trailing fraction zeros, numeric column headers align with their values,
 * each counted tab shows the number of entries its panel lists, glyphs stand bare, and every
 * panel heading uses one label size.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";
const DIALOG = ".transaction-details-dialog";
const TRANSPARENT = new Set(["rgba(0, 0, 0, 0)", "transparent"]);

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [transactions, shell] = await Promise.all([loadIndex("transactions"), loadIndex("shell")]);

async function openDialog(t) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([transactions, shell], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}/transactions`);
  await page.waitForSelector(READY);
  await page.click("tr[data-row-id] td");
  await page.waitForSelector(`${DIALOG} .tx-flow-arrow`);
  return page;
}

async function showTab(page, tab) {
  await page.click(`${DIALOG} [data-dialog-tab="${tab}"]`);
  await page.waitForSelector(`${DIALOG} [data-tab-content="${tab}"].active :not(.loading-spinner)`);
}

/**
 * Visible English text in `scope` ending a decimal fraction on a zero ("0.500", "1.0 SOL").
 * A subscript zero run ("0.0₆12") is price notation, not padding.
 */
function trailingZeros(page, scope) {
  return page.evaluate(
    ([dialog, selector]) =>
      [...document.querySelectorAll(`${dialog} ${selector}`)]
        .filter((element) => element.getClientRects().length && !element.children.length)
        .map((element) => element.textContent.trim())
        .filter((text) => /\d\.\d*0(?![\d₀-₉])/.test(text)),
    [DIALOG, scope]
  );
}

test("tab counts match the entries each panel lists", async (t) => {
  const page = await openDialog(t);
  const badge = (id) => page.$eval(`${DIALOG} #${id}`, (element) => Number(element.textContent));

  const counts = {};
  for (const [tab, badgeId, entry] of [
    ["instructions", "instructionsBadge", ".instructions-list > *"],
    ["logs", "logsBadge", "[data-tab-content=logs] .log-entry"],
    ["ata", "ataBadge", ".ata-table tbody tr"],
  ]) {
    await showTab(page, tab);
    counts[tab] = {
      badge: await badge(badgeId),
      listed: await page.$$eval(`${DIALOG} ${entry}`, (elements) => elements.length),
    };
  }

  for (const [tab, { badge: shown, listed }] of Object.entries(counts)) {
    assert.ok(listed > 0, `the fixture lists ${tab} entries`);
    assert.equal(shown, listed, `the ${tab} tab counts ${shown} but lists ${listed}`);
  }
});

test("amounts drop trailing zeros and numeric headers align with their values", async (t) => {
  const page = await openDialog(t);
  const found = { overview: await trailingZeros(page, "*") };
  for (const tab of ["balances", "ata"]) {
    await showTab(page, tab);
    found[tab] = await trailingZeros(page, `[data-tab-content=${tab}] *`);
  }
  assert.deepEqual(found, { overview: [], balances: [], ata: [] });

  const misaligned = await page.evaluate((dialog) => {
    const tables = document.querySelectorAll(`${dialog} table`);
    return [...tables].flatMap((table) => {
      const headers = [...table.querySelectorAll("thead th")];
      const firstRow = table.querySelector("tbody tr");
      if (!firstRow) return [];
      return [...firstRow.children].flatMap((cell, index) => {
        const header = headers[index];
        const cellAlign = getComputedStyle(cell).textAlign;
        const headerAlign = getComputedStyle(header).textAlign;
        return cellAlign === headerAlign
          ? []
          : [`${header.textContent.trim()}: ${headerAlign}/${cellAlign}`];
      });
    });
  }, DIALOG);
  assert.deepEqual(misaligned, [], "a column header aligns apart from its values");
});

test("glyphs stand bare and panel headings share one label size", async (t) => {
  const page = await openDialog(t);
  const boxed = await page.evaluate(
    ([dialog, transparent]) =>
      [`${dialog} .header-icon`, `${dialog} .tx-flow-arrow`].flatMap((selector) =>
        [...document.querySelectorAll(selector)]
          .filter((element) => {
            const style = getComputedStyle(element);
            return (
              !transparent.includes(style.backgroundColor) || parseFloat(style.borderTopWidth) > 0
            );
          })
          .map(() => selector)
      ),
    [DIALOG, [...TRANSPARENT]]
  );
  assert.deepEqual(boxed, [], "a glyph sits on a tile");

  const overviewLabel = await page.$eval(
    `${DIALOG} .tx-section-label`,
    (element) => getComputedStyle(element).fontSize
  );
  const sizes = {};
  for (const tab of ["balances", "ata"]) {
    await showTab(page, tab);
    sizes[tab] = await page.$$eval(
      [".tx-panel-header", ".tx-panel-title", ".tx-panel-count"]
        .map((part) => `${DIALOG} [data-tab-content=${tab}] ${part}`)
        .join(", "),
      (elements, transparent) =>
        elements.map((element) => {
          const style = getComputedStyle(element);
          return transparent.includes(style.backgroundColor) || element.matches(".tx-panel-header")
            ? style.fontSize
            : "filled";
        }),
      [...TRANSPARENT]
    );
  }
  for (const [tab, list] of Object.entries(sizes)) {
    assert.ok(list.length > 0, `the ${tab} tab renders panel headings`);
    assert.deepEqual(
      [...new Set(list)],
      [overviewLabel],
      `${tab} panel headings differ from the ${overviewLabel} section label`
    );
  }
});
