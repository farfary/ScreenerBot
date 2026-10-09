// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the header's main row never paints one control over another. The metric cards
 * keep their content width; when they outgrow their slot they overflow towards the end,
 * where the strip scrolls and the quick actions fold, never back over the trader card.
 *
 * Every trader state the card can show (Explore, Running with a Today P&L, Paused,
 * Stopped) is rendered at the narrowest desktop window and at a common laptop width, in
 * a left-to-right and a right-to-left locale.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const WIDTHS = [1200, 1328];
const LOCALES = ["en", "fa"];
const METRICS = JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/header_metrics.json`, "utf8"));
const INITIALIZATION = JSON.parse(
  readFileSync(`${FIXTURES_ROOT}/shell/initialization_status.json`, "utf8")
);

const STATES = {
  explore: { trader: { enabled: false, state: "explore" }, explore: true },
  running: { trader: { ...METRICS.trader, state: "running" }, explore: false },
  paused: { trader: { ...METRICS.trader, state: "entry_paused" }, explore: false },
  stopped: { trader: { ...METRICS.trader, enabled: false, state: "stopped" }, explore: false },
};

const browser = await chromium.launch({ headless: true });
const hosts = [];
after(async () => {
  await browser.close();
  await Promise.all(hosts.map((host) => host.close()));
});

const [shell, home] = await Promise.all([loadIndex("shell"), loadIndex("home")]);

/** The main header row's boxes for one trader state, width and locale. */
async function measure(state, width, locale) {
  const context = await browser.newContext({ viewport: { width, height: 800 }, locale });
  const fixtures = createApiHandler([home, shell], "populated");
  const { trader, explore } = STATES[state];
  const onApi = async (request) => {
    if (request.url.pathname === "/api/header/metrics") {
      return { body: JSON.stringify({ ...METRICS, trader }) };
    }
    if (request.url.pathname === "/api/initialization/status") {
      return {
        body: JSON.stringify({
          ...INITIALIZATION,
          explore_mode: explore,
          explore_mode_enabled: explore,
        }),
      };
    }
    return fixtures.onApi(request);
  };
  const host = await serveDashboard(context, { locale, onApi });
  hosts.push(host);
  const page = await context.newPage();
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector(`#botCard[data-status="${trader.state}"]`);
  // The quick actions fold by measurement one frame after the cards settle.
  await page.waitForFunction(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve(true)))
      )
  );
  const boxes = await page.evaluate(() => {
    const rect = (element) => {
      const box = element.getBoundingClientRect();
      return { start: box.left, end: box.right };
    };
    const visible = (element) => element && !element.hidden && element.getClientRects().length > 0;
    const controls = [
      "#headerBrand",
      "#botCard",
      "#exploreSetupControl",
      "#headerCards",
      "#headerActions",
    ]
      .map((selector) => [selector, document.querySelector(selector)])
      .filter(([, element]) => visible(element))
      .map(([selector, element]) => ({ selector, ...rect(element) }));
    const strip = document.getElementById("headerCards");
    const cards = [...strip.querySelectorAll(".header-card")]
      .filter(visible)
      .map((card) => ({ selector: `#${card.id}`, ...rect(card) }));
    return { controls, strip: rect(strip), cards };
  });
  await context.close();
  return boxes;
}

for (const locale of LOCALES) {
  for (const width of WIDTHS) {
    test(`header controls never overlap at ${width}px (${locale})`, async () => {
      for (const state of Object.keys(STATES)) {
        const { controls, strip, cards } = await measure(state, width, locale);
        const where = `${state} at ${width}px (${locale})`;
        const ordered = [...controls].sort((a, b) => a.start - b.start);
        for (let index = 1; index < ordered.length; index += 1) {
          const [before, next] = [ordered[index - 1], ordered[index]];
          assert.ok(
            next.start >= before.end - 0.5,
            `${where}: ${next.selector} starts at ${next.start} inside ${before.selector} (ends ${before.end})`
          );
        }
        // A card may run past the strip on its scrolling end, never on the trader's side.
        for (const card of cards) {
          const outside =
            locale === "fa" ? card.end > strip.end + 0.5 : card.start < strip.start - 0.5;
          assert.ok(
            !outside,
            `${where}: ${card.selector} overflows the card strip towards the trader card`
          );
        }
      }
    });
  }
}
