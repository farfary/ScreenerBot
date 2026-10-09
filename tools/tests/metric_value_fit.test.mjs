// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a metric value that reads as one figure keeps one line. The Auto Trader
 * Stats "Avg Win / Loss" pair once wrapped after its slash in a 1328px window, so
 * "+3.5% / -24.4%" read as two numbers, and the German "Median hold" duration took
 * two lines beside one-line neighbours. `.metric-value-fit` (components.css) fits
 * the font to the card instead; this test holds it at every grid column count the
 * stats cards take, in a left-to-right, a long-label and a right-to-left locale, for
 * the longest values the formatters produce.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";
const WIDTHS = [1100, 1280, 1328, 1440, 1920];
const LOCALES = ["en", "de", "fa"];
// Each metric with the stats that render its longest values: the widest win/loss
// pair, and hold times naming days, hours and minutes.
const METRICS = [
  {
    name: "Avg Win / Loss",
    selector: "#avg-win-loss",
    stats: [
      { avg_win_pct: 3.5, avg_loss_pct: -24.4 },
      { avg_win_pct: 999.9, avg_loss_pct: -100 },
    ],
  },
  {
    name: "Median hold",
    selector: "#median-hold",
    stats: [{ median_hold_time_hours: 47.99 }, { median_hold_time_hours: 2399.99 }],
  },
];

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("trader"), await loadIndex("shell")];

for (const metric of METRICS) {
  for (const locale of LOCALES) {
    test(`the ${metric.name} value stays on one line inside its card (${locale})`, async () => {
      for (const stats of metric.stats) {
        const context = await browser.newContext({
          viewport: { width: WIDTHS[0], height: 900 },
          locale,
        });
        const api = createApiHandler(indexes, "populated");
        const onApi = async (request) => {
          const answer = await api.onApi(request);
          if (request.url.pathname !== "/api/trader/stats") return answer;
          return { ...answer, body: JSON.stringify({ ...JSON.parse(answer.body), ...stats }) };
        };
        const host = await serveDashboard(context, { locale, onApi });
        try {
          const page = await context.newPage();
          page.setDefaultTimeout(15000);
          await page.goto(`${host.origin}/trader`);
          await page.waitForSelector(READY);
          await page.waitForFunction(
            (selector) => /\d/.test(document.querySelector(selector)?.textContent ?? ""),
            metric.selector
          );
          for (const width of WIDTHS) {
            await page.setViewportSize({ width, height: 900 });
            const box = await page.locator(metric.selector).evaluate((el) => {
              const lineHeight = parseFloat(getComputedStyle(el).lineHeight);
              return {
                text: el.textContent,
                lines: Math.round(el.getBoundingClientRect().height / lineHeight),
                overflow: el.scrollWidth - el.clientWidth,
              };
            });
            const where = `${locale} ${width}px "${box.text}"`;
            assert.equal(box.lines, 1, `${where} wraps`);
            assert.ok(box.overflow <= 0, `${where} overflows its card by ${box.overflow}px`);
          }
        } finally {
          await context.close();
          await host.close();
        }
      }
    });
  }
}
