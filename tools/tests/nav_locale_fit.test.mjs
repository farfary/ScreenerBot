// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: in every locale the default navigation set fits the header row at the
 * common desktop widths, with room to spare.
 *
 * The row compacts itself (icons first, then tighter labels), but long labels can
 * still push the last tab past the edge once the platform font renders a little
 * wider than the test browser's. Each locale's row therefore keeps a margin at
 * 1200 px and 1400 px; a locale that loses it needs shorter nav labels
 * (`nav-*`), which leave the page titles (`nav-page-title-*`) untouched.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, statSync } from "node:fs";
import { join } from "node:path";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const LOCALES_ROOT = new URL("../../locales/", import.meta.url).pathname;
const LOCALES = readdirSync(LOCALES_ROOT).filter((name) =>
  statSync(join(LOCALES_ROOT, name)).isDirectory()
);
const WIDTHS = [1200, 1400];
// The share of the row the tabs may take; the rest absorbs font differences.
const MAX_FILL = 0.96;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [shell, home] = await Promise.all([loadIndex("shell"), loadIndex("home")]);

/** The tabs' extent along the row, the row's width and the nav's own overflow. */
function measure(page) {
  return page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() =>
          requestAnimationFrame(() => {
            const nav = document.getElementById("navTabs");
            const row = document.querySelector(".header-row-2");
            const boxes = [...nav.querySelectorAll(":scope > a.tab")]
              .filter((tab) => !tab.hidden && tab.getClientRects().length > 0)
              .map((tab) => tab.getBoundingClientRect());
            const start = Math.min(...boxes.map((box) => box.left));
            const end = Math.max(...boxes.map((box) => box.right));
            resolve({
              extent: Math.round(end - start),
              row: row.clientWidth,
              overflow: nav.scrollWidth - nav.clientWidth,
            });
          })
        )
      )
  );
}

for (const locale of LOCALES) {
  test(`the default nav set fits with a margin (${locale})`, async () => {
    const context = await browser.newContext({ viewport: { width: WIDTHS[0], height: 800 } });
    const fixtures = createApiHandler([home, shell], "populated");
    const host = await serveDashboard(context, { locale, onApi: fixtures.onApi });
    try {
      const page = await context.newPage();
      await page.goto(`${host.origin}/home`);
      await page.waitForSelector("#navTabs > a.tab.active");
      for (const width of WIDTHS) {
        await page.setViewportSize({ width, height: 800 });
        const { extent, row, overflow } = await measure(page);
        assert.equal(overflow, 0, `${locale} at ${width}px scrolls the nav`);
        assert.ok(
          extent <= row * MAX_FILL,
          `${locale} at ${width}px: tabs take ${extent}px of ${row}px (limit ${Math.floor(row * MAX_FILL)}px)`
        );
      }
    } finally {
      await context.close();
      await host.close();
    }
  });
}
