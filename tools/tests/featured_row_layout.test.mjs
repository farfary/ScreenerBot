// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the featured row's scroll arrows sit beside the card scroller, never over it,
 * so no arrow covers or cuts a card.
 *
 * The row is rendered with more cards than fit and scrolled one step, so both arrows
 * show, at the narrowest desktop width and a common laptop width, in a left-to-right
 * and a right-to-left locale.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const WIDTHS = [1200, 1440];
const LOCALES = ["en", "fa"];

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const indexes = [await loadIndex("tokens"), await loadIndex("shell")];
const FEATURED = JSON.parse(readFileSync(`${FIXTURES_ROOT}/tokens/featured_all.json`, "utf8"));
// Thirty more trending cards than any row width holds, each under its own mint.
const TRENDING = Array.from({ length: 30 }, (_, index) => ({
  ...FEATURED.boosted[0],
  mint: `Trend${String(index).padStart(2, "0")}${"1".repeat(37)}`,
  symbol: `TRND${index}`,
  name: `Trending ${index}`,
  boosts: 0,
  golden: false,
}));
const FEATURED_FULL = JSON.stringify({
  ...FEATURED,
  dexscreener_trending: [...(FEATURED.dexscreener_trending || []), ...TRENDING],
});

for (const locale of LOCALES) {
  for (const width of WIDTHS) {
    test(`featured row arrows never cover a card at ${width}px (${locale})`, async () => {
      const context = await browser.newContext({ viewport: { width, height: 900 } });
      const api = createApiHandler(indexes, "populated");
      const onApi = async (request) =>
        request.url.pathname === "/api/featured/all" ? { body: FEATURED_FULL } : api.onApi(request);
      const host = await serveDashboard(context, { locale, onApi });
      try {
        const page = await context.newPage();
        await page.goto(`${host.origin}/tokens`);
        await page.waitForSelector(PAGE_READY);
        await page.waitForSelector(".featured-row-tokens .featured-row-card");
        // One step along the row shows both arrows around a full scroller.
        await page.click(".featured-row-arrow-end");
        await page.waitForSelector(".featured-row-arrow-start:not(.hidden)");
        const report = await page.evaluate(() => {
          const scroller = document.querySelector(".featured-row-tokens");
          const box = scroller.getBoundingClientRect();
          const arrows = [...document.querySelectorAll(".featured-row-arrow")]
            .filter((arrow) => getComputedStyle(arrow).visibility === "visible")
            .map((arrow) => {
              const rect = arrow.getBoundingClientRect();
              return { name: arrow.className, start: rect.left, end: rect.right };
            });
          return {
            overflowing: scroller.scrollWidth > scroller.clientWidth,
            scroller: { start: box.left, end: box.right },
            arrows,
          };
        });
        assert.ok(report.overflowing, "the fixture holds more cards than the row shows");
        assert.equal(report.arrows.length, 2, "both arrows are shown");
        for (const arrow of report.arrows) {
          const overlaps =
            arrow.start < report.scroller.end - 0.5 && arrow.end > report.scroller.start + 0.5;
          assert.ok(!overlaps, `${arrow.name} lies over the card scroller`);
        }
      } finally {
        await context.close();
        await host.close();
      }
    });
  }
}
