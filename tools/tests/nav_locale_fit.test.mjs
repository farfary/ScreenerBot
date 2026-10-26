// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: in every locale the default navigation set fits the header row at the
 * common desktop widths in its full face, with room to spare.
 *
 * The row can tighten its labels (`is-condensed`), but at 1200 px and wider every
 * catalog must fit without it, so the row reads the same in every language. Long labels
 * can still push the last tab past the edge once the platform font renders a little
 * wider than the test browser's, so each locale's tabs keep a margin of the row's
 * content box (the row minus its inline padding); a locale that loses it needs shorter
 * nav labels (`nav-*`), which leave the page titles (`nav-page-title-*`) untouched.
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

/** The tabs' extent, the row's content box, the strip's overflow and the row's face. */
function measure(page) {
  return page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() =>
          requestAnimationFrame(() => {
            const nav = document.getElementById("navTabs");
            const row = document.querySelector(".header-row-2");
            const style = getComputedStyle(nav);
            const boxes = [...nav.querySelectorAll(":scope > a.tab")]
              .filter((tab) => !tab.hidden && tab.getClientRects().length > 0)
              .map((tab) => tab.getBoundingClientRect());
            const start = Math.min(...boxes.map((box) => box.left));
            const end = Math.max(...boxes.map((box) => box.right));
            resolve({
              extent: Math.round(end - start),
              room:
                row.clientWidth -
                parseFloat(style.paddingInlineStart) -
                parseFloat(style.paddingInlineEnd),
              overflow: row.scrollWidth - row.clientWidth,
              condensed: nav.classList.contains("is-condensed"),
            });
          })
        )
      )
  );
}

for (const locale of LOCALES) {
  test(`the default nav set fits in its full face with a margin (${locale})`, async () => {
    const context = await browser.newContext({ viewport: { width: WIDTHS[0], height: 800 } });
    const fixtures = createApiHandler([home, shell], "populated");
    const host = await serveDashboard(context, { locale, onApi: fixtures.onApi });
    try {
      const page = await context.newPage();
      await page.goto(`${host.origin}/home`);
      await page.waitForSelector("#navTabs > a.tab.active");
      for (const width of WIDTHS) {
        await page.setViewportSize({ width, height: 800 });
        const { extent, room, overflow, condensed } = await measure(page);
        assert.equal(condensed, false, `${locale} at ${width}px condenses the nav`);
        assert.equal(overflow, 0, `${locale} at ${width}px scrolls the nav`);
        assert.ok(
          extent <= room * MAX_FILL,
          `${locale} at ${width}px: tabs take ${extent}px of ${room}px (limit ${Math.floor(room * MAX_FILL)}px)`
        );
      }
    } finally {
      await context.close();
      await host.close();
    }
  });
}
