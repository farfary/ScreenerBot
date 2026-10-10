// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Home performance calendar paints every day cell opaque in both themes, and a
 * day not yet reached shares the out-of-month fill. A translucent cell shows the grid's
 * border-coloured gaps through its face, which turned the rest of the month into a grey slab.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

// Inside the fixture month, so past, today and future days all render.
const FIXED_NOW = new Date("2026-10-15T12:00:00Z");

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [home, shell] = await Promise.all([loadIndex("home"), loadIndex("shell")]);

test("calendar day cells are opaque and future days take the out-of-month fill", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([home, shell], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.clock.setFixedTime(FIXED_NOW);
  await page.goto(`${host.origin}/`);
  await page.waitForSelector(PAGE_READY);
  await page.waitForSelector(".calendar-cell.future");

  for (const theme of ["light", "dark"]) {
    const cells = await page.evaluate((name) => {
      document.documentElement.setAttribute("data-theme", name);
      const all = [...document.querySelectorAll(".calendar-grid > .calendar-cell")];
      const style = (cell) => getComputedStyle(cell);
      return {
        translucent: all
          .filter((cell) => Number(style(cell).opacity) < 1)
          .map((cell) => cell.className),
        blank: style(document.querySelector(".calendar-cell.blank")).backgroundColor,
        future: [
          ...new Set(
            all.filter((cell) => cell.matches(".future")).map((cell) => style(cell).backgroundColor)
          ),
        ],
      };
    }, theme);
    assert.deepEqual(cells.translucent, [], `${theme}: a calendar cell is translucent`);
    assert.deepEqual(cells.future, [cells.blank], `${theme}: future days differ from blanks`);
  }
});
