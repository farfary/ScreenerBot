// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: in every locale every page's sub-tab row fits at 1200 px, with room to spare.
 *
 * A sub-row that overflows hides its last tabs under the edge fade (the Russian Auto
 * Trader row once hid Settings). Each page is visited through the router, and every page
 * that shows a sub-tab row is measured, so a new page with sub-tabs is covered without
 * being listed here. The tabs keep a margin of the row's content box (the row minus its
 * inline padding) for platform fonts that render a little wider than the test
 * browser's; a locale that loses it needs shorter tab labels, never a smaller face.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, statSync } from "node:fs";
import { join } from "node:path";

import { chromium } from "playwright";

import { PAGE_IDS, PAGE_READY, serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const LOCALES_ROOT = new URL("../../locales/", import.meta.url).pathname;
const LOCALES = readdirSync(LOCALES_ROOT).filter((name) =>
  statSync(join(LOCALES_ROOT, name)).isDirectory()
);
const WIDTH = 1200;
// The share of the row's content box the tabs may take; the rest absorbs font differences.
const MAX_FILL = 0.96;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = await Promise.all([...PAGE_IDS, "shell"].map((name) => loadIndex(name)));

/** The shown sub-tab row's tabs extent, content box and overflow, or null without one. */
function measure(page) {
  return page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() =>
          requestAnimationFrame(() => {
            const row = document.getElementById("subTabsContainer");
            if (row?.dataset.ui !== "tab-bar" || row.getClientRects().length === 0) {
              resolve(null);
              return;
            }
            const style = getComputedStyle(row);
            const boxes = [...row.querySelectorAll(".sub-tab")].map((tab) =>
              tab.getBoundingClientRect()
            );
            const start = Math.min(...boxes.map((box) => box.left));
            const end = Math.max(...boxes.map((box) => box.right));
            resolve({
              extent: Math.round(end - start),
              room:
                row.clientWidth -
                parseFloat(style.paddingInlineStart) -
                parseFloat(style.paddingInlineEnd),
              overflow: row.scrollWidth - row.clientWidth,
            });
          })
        )
      )
  );
}

for (const locale of LOCALES) {
  test(`every sub-tab row fits at ${WIDTH}px with a margin (${locale})`, async () => {
    const context = await browser.newContext({ viewport: { width: WIDTH, height: 800 } });
    const fixtures = createApiHandler(indexes, "populated");
    const host = await serveDashboard(context, { locale, onApi: fixtures.onApi });
    try {
      const page = await context.newPage();
      page.setDefaultTimeout(20000);
      await page.goto(`${host.origin}/home`);
      await page.waitForSelector(PAGE_READY);
      const measured = [];
      for (const id of PAGE_IDS) {
        await page.click(`#navTabs > a.tab[data-page="${id}"]`);
        await page.waitForSelector(`#navTabs > a.tab.active[data-page="${id}"]`);
        await page.waitForSelector(`${PAGE_READY} #page-${id}`, { state: "attached" });
        const row = await measure(page);
        if (!row) continue;
        measured.push(id);
        assert.ok(row.overflow <= 0, `${locale} ${id}: the sub-tabs scroll by ${row.overflow}px`);
        assert.ok(
          row.extent <= row.room * MAX_FILL,
          `${locale} ${id}: sub-tabs take ${row.extent}px of ${row.room}px (limit ${Math.floor(row.room * MAX_FILL)}px)`
        );
      }
      assert.ok(measured.includes("tokens"), `${locale}: no sub-tab row measured on Tokens`);
    } finally {
      await context.close();
      await host.close();
    }
  });
}
