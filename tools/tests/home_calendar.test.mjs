// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Home performance calendar paints every day cell opaque in both themes, and a
 * day not yet reached shares the out-of-month fill. A translucent cell shows the grid's
 * border-coloured gaps through its face, which turned the rest of the month into a grey slab.
 * A traded day is tinted by its P&L: the gain or loss tone, deeper for a larger move, the
 * month's largest move at full depth; a day without trades stays neutral.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

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

test("traded days are tinted by tone and magnitude, untraded days stay neutral", async (t) => {
  const calendar = JSON.parse(
    readFileSync(`${FIXTURES_ROOT}/home/portfolio_calendar.json`, "utf8")
  );
  const traded = calendar.days.filter((day) => day.has_data);
  const largest = traded.reduce((top, day) =>
    Math.abs(day.net_pnl_native) > Math.abs(top.net_pnl_native) ? day : top
  );
  assert.ok(
    traded.some((day) => day.net_pnl_native > 0) && traded.some((day) => day.net_pnl_native < 0),
    "the fixture month holds a gain and a loss"
  );

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
  await page.waitForSelector(".calendar-cell.profit[data-date]");

  const cells = await page.$$eval(".calendar-grid > .calendar-cell[data-date]", (all) =>
    Object.fromEntries(
      all.map((cell) => [
        cell.dataset.date,
        {
          profit: cell.classList.contains("profit"),
          loss: cell.classList.contains("loss"),
          tint: cell.style.getPropertyValue("--cell-tint").trim(),
        },
      ])
    )
  );
  const tradedDates = new Set(traded.map((day) => day.date));
  for (const day of traded) {
    const cell = cells[day.date];
    assert.ok(cell, `${day.date} renders`);
    assert.equal(cell.profit, day.net_pnl_native > 0, `${day.date} gain tone`);
    assert.equal(cell.loss, day.net_pnl_native < 0, `${day.date} loss tone`);
    assert.match(cell.tint, /^\d+(\.\d)?%$/, `${day.date} carries a tint`);
  }
  assert.equal(cells[largest.date].tint, "26.0%", "the largest move takes the full tint");
  // A larger move is never paler than a smaller one.
  const byMove = [...traded].sort(
    (a, b) => Math.abs(a.net_pnl_native) - Math.abs(b.net_pnl_native)
  );
  const depths = byMove.map((day) => Number.parseFloat(cells[day.date].tint));
  assert.deepEqual(depths, [...depths].sort((a, b) => a - b), "the tint grows with the move");
  assert.ok(depths[0] > 0, "the smallest traded move is still tinted");

  for (const [date, cell] of Object.entries(cells)) {
    if (tradedDates.has(date)) continue;
    assert.deepEqual(cell, { profit: false, loss: false, tint: "" }, `${date} is not neutral`);
  }
});
