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
 * - Every pipeline label fits inside its own cell at every desktop width and locale.
 * - Today's P&L in the header bot card and ticker takes the shared SOL precision (4
 *   decimals, like the Home hero), not a precision of its own.
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

async function openHome(t, width, locale = "en", answers = {}) {
  const context = await browser.newContext({ viewport: { width, height: 900 } });
  const api = createApiHandler([home, shell], "populated");
  const host = await serveDashboard(context, {
    locale,
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
  await page.clock.setFixedTime(FIXED_NOW);
  await page.goto(`${host.origin}/`);
  await page.waitForSelector(PAGE_READY);
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

for (const locale of ["en", "de"]) {
  test(`every pipeline label fits inside its cell (${locale})`, async (t) => {
    const page = await openHome(t, 1024, locale);
    for (const width of [1024, 1328, 1440, 1920]) {
      await page.setViewportSize({ width, height: 900 });
      const overflowing = await page.$$eval(".pipeline-metric", (cells) =>
        cells
          .map((cell) => {
            const label = cell.querySelector("span");
            const box = cell.getBoundingClientRect();
            const text = label.getBoundingClientRect();
            const fits =
              label.scrollWidth <= label.clientWidth &&
              text.left >= box.left - 0.5 &&
              text.right <= box.right + 0.5;
            return fits ? null : `${label.textContent} (${label.scrollWidth}/${label.clientWidth})`;
          })
          .filter(Boolean)
      );
      assert.deepEqual(overflowing, [], `${width}px: a pipeline label overflows its cell`);
    }
  });
}

test("today's P&L in the header takes the shared SOL precision", async (t) => {
  const header = JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/header_metrics.json`, "utf8"));
  header.trader.today_pnl_native = 0.01234;
  const page = await openHome(t, 1440, "en", {
    "/api/header/metrics": JSON.stringify(header),
  });
  const text = (selector) =>
    page
      .waitForFunction((css) => {
        const shown = globalThis.document.querySelector(css)?.textContent ?? "";
        return /\d/.test(shown) && shown.replace(/[\u2066-\u2069]/g, "").trim();
      }, selector)
      .then((handle) => handle.jsonValue());
  assert.equal(await text("#botPnL .pnl-num"), "+0.0123");
  assert.match(await text("#tickerTodayPnL"), /^\+0\.0123 SOL /);
});
