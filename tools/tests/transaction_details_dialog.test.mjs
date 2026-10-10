// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the transaction details dialog states amounts and counts the way it lists them.
 * Every amount drops trailing fraction zeros, while the execution price keeps the four
 * significant digits of every trade price ("0.009790 SOL", as in Positions); numeric column
 * headers align with their values,
 * each counted tab shows the number of entries its panel lists, glyphs stand bare, and every
 * panel heading uses one label size. The header opens the explorers from one control
 * whose menu names each explorer. Each tab is as wide as what it shows: no minimum
 * width reserves a trailing gap after a short label.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

const DIALOG = ".transaction-details-dialog";
const TRANSPARENT = new Set(["rgba(0, 0, 0, 0)", "transparent"]);

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [transactions, shell] = await Promise.all([loadIndex("transactions"), loadIndex("shell")]);

async function openDialog(t, answers = {}) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([transactions, shell], "populated");
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
  await page.goto(`${host.origin}/transactions`);
  await page.waitForSelector(PAGE_READY);
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
 * A subscript zero run ("0.0₆12") is price notation, not padding, and the execution price
 * keeps its significant digits by rule.
 */
function trailingZeros(page, scope) {
  return page.evaluate(
    ([dialog, selector]) =>
      [...document.querySelectorAll(`${dialog} ${selector}`)]
        .filter((element) => element.getClientRects().length && !element.children.length)
        .filter(
          (element) =>
            element.closest(".tx-execution-metric")?.querySelector("span")?.textContent !==
            "Execution price"
        )
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

test("the header opens explorers from one control with a named menu", async (t) => {
  const page = await openDialog(t);
  const header = `${DIALOG} .dialog-header-actions`;
  assert.equal(await page.locator(`${header} a[href]`).count(), 0, "no bare explorer links");
  const control = page.locator(`${header} .explorer-menu-btn`);
  assert.equal(await control.count(), 1, "one explorer control");
  assert.equal(await control.getAttribute("aria-haspopup"), "menu");
  assert.ok((await control.getAttribute("aria-label")).length > 0, "the control is named");

  await control.click();
  await page.waitForSelector(".context-menu.visible");
  const items = await page.$$eval(".context-menu.visible .context-menu-item", (nodes) =>
    nodes.map((node) =>
      node.textContent
        .replace(/[\u2066-\u2069]/g, "")
        .trim()
        .replace(/\s+/g, " ")
    )
  );
  assert.deepEqual(items, ["View on Solscan", "View on Solana FM"]);

  const menu = await page.$eval(
    ".context-menu.visible",
    (node) => node.getBoundingClientRect().right
  );
  const box = await control.boundingBox();
  assert.ok(Math.abs(menu - (box.x + box.width)) < 2, "the menu drops from the control's end edge");
});

test("each tab is as wide as its glyph, label and count", async (t) => {
  const page = await openDialog(t);
  const slack = await page.$$eval(`${DIALOG} .details-tab`, (tabs) =>
    tabs.map((tab) => {
      const style = getComputedStyle(tab);
      const children = [...tab.children].filter((child) => child.getClientRects().length);
      const outer = (child) => {
        const box = getComputedStyle(child);
        const margin = parseFloat(box.marginInlineStart) + parseFloat(box.marginInlineEnd);
        return child.getBoundingClientRect().width + margin;
      };
      const content =
        children.reduce((sum, child) => sum + outer(child), 0) +
        parseFloat(style.columnGap) * (children.length - 1);
      const padding = parseFloat(style.paddingInlineStart) + parseFloat(style.paddingInlineEnd);
      return {
        tab: tab.dataset.dialogTab,
        slack: tab.getBoundingClientRect().width - content - padding,
      };
    })
  );
  assert.equal(slack.length, 6);
  for (const { tab, slack: extra } of slack) {
    assert.ok(
      Math.abs(extra) < 1,
      `the ${tab} tab reserves ${extra.toFixed(1)}px beyond its content`
    );
  }
});

test("the execution price keeps four significant digits", async (t) => {
  const detail = JSON.parse(
    readFileSync(`${FIXTURES_ROOT}/transactions/transaction_detail.json`, "utf8")
  );
  detail.swap_pnl_info.calculated_price_sol = 0.00979;
  const page = await openDialog(t, {
    [`/api/transactions/${detail.signature}`]: JSON.stringify(detail),
  });
  const price = await page
    .locator(`${DIALOG} .tx-execution-metric`, { hasText: "Execution price" })
    .locator("strong")
    .evaluate((node) => node.textContent.replace(/[\u2066-\u2069]/g, "").trim());
  assert.equal(price, "0.009790 SOL");
});
