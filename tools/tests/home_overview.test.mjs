// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Home portfolio overview states every figure legibly and once.
 *
 * - The balance trend is drawn at least at its CSS minimum width, or not at all; it is
 *   never squeezed into a sliver beside a wide balance.
 * - Every calendar day that shows a figure is hoverable, and its popover names the
 *   figure; a past day without trades is classed as such, never as empty.
 * - The day-change percent has one precision in the header Worth card and the hero.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";
// Inside the fixture month, so past, today and future days all render.
const FIXED_NOW = new Date("2026-10-15T12:00:00Z");

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [home, shell] = await Promise.all([loadIndex("home"), loadIndex("shell")]);

async function openHome(t, width) {
  const context = await browser.newContext({ viewport: { width, height: 900 } });
  const api = createApiHandler([home, shell], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.clock.setFixedTime(FIXED_NOW);
  await page.goto(`${host.origin}/`);
  await page.waitForSelector(READY);
  await page.waitForSelector(".home-dashboard.loaded");
  return page;
}

test("the balance trend is drawn at its minimum width or not at all", async (t) => {
  for (const width of [820, 1000, 1200, 1440]) {
    const page = await openHome(t, width);
    // A wide balance takes the row; the trend must give way whole.
    await page.evaluate(() => {
      document.getElementById("walletBalance").textContent = "12,345,678.9012 SOL";
    });
    await page.waitForFunction(() => {
      const svg = document.getElementById("heroSpark");
      const shown = getComputedStyle(svg).display !== "none";
      return (
        !shown || svg.getBoundingClientRect().width >= parseFloat(getComputedStyle(svg).minWidth)
      );
    });
    const spark = await page.$eval("#heroSpark", (svg) => ({
      shown: getComputedStyle(svg).display !== "none",
      width: svg.getBoundingClientRect().width,
      min: parseFloat(getComputedStyle(svg).minWidth),
    }));
    assert.ok(spark.min > 0, `${width}px: the trend declares a minimum width`);
    if (spark.shown) {
      assert.ok(
        spark.width >= spark.min,
        `${width}px: the trend is ${spark.width}px, below ${spark.min}px`
      );
    }
  }
});

test("every calendar figure is hoverable and named", async (t) => {
  const page = await openHome(t, 1440);
  await page.waitForSelector(".calendar-cell.no-trades .cell-value");
  const cells = await page.$$eval(".calendar-grid > .calendar-cell", (all) =>
    all.map((cell) => ({
      cls: cell.className,
      figure: !!cell.querySelector(".cell-value, .cell-pnl"),
      date: cell.dataset.date ?? null,
    }))
  );
  assert.ok(cells.length > 0, "calendar cells render");
  for (const cell of cells) {
    assert.doesNotMatch(cell.cls, /\bempty\b/, "no day is classed empty");
    if (cell.figure) assert.ok(cell.date, `a ${cell.cls} cell shows a figure without a popover`);
  }

  await page.hover(".calendar-cell.no-trades[data-date]");
  await page.waitForSelector(".cal-day-popover--visible");
  const labels = await page.$$eval(".cal-day-popover .cal-pop-label", (els) =>
    els.map((el) => el.textContent.trim())
  );
  assert.ok(labels.includes("End balance"), `a no-trade day names its figure: ${labels}`);
});

test("the day-change percent has one precision in the header and the hero", async (t) => {
  const page = await openHome(t, 1440);
  await page.waitForSelector("#homeWalletChange .change-percent");
  const decimals = (text) => /[.,](\d+)\s*%/.exec(text)?.[1].length ?? 0;
  const hero = await page.$eval("#homeWalletChange .change-percent", (el) => el.textContent);
  const header = await page.$eval("#walletChange", (el) => el.textContent);
  assert.ok(decimals(hero) > 0, `hero percent "${hero}" carries decimals`);
  assert.equal(decimals(hero), decimals(header), `hero "${hero}" and header "${header}"`);
});
