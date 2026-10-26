// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the first-run introduction on `/initialization`. The window is titled with
 * the setup page's own catalog title rather than the page it loaded over, every card
 * glyph differs from its slide's title glyph, and the trade and data slides carry
 * their current cards.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const shell = await loadIndex("shell");
const BOOTSTRAP = JSON.parse(
  readFileSync(`${FIXTURES_ROOT}/shell/system_bootstrap.json`, "utf8")
);
const FIRST_RUN = {
  ...BOOTSTRAP,
  initialization_required: true,
  initialization_complete: false,
  onboarding_complete: false,
  ui_ready: false,
};

test("the first-run introduction is titled and its slides are distinct", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([shell], "populated");
  const onApi = (request) =>
    request.url.pathname === "/api/system/bootstrap"
      ? { body: JSON.stringify(FIRST_RUN) }
      : api.onApi(request);
  const host = await serveDashboard(context, { locale: "en", onApi });
  try {
    const page = await context.newPage();
    await page.goto(`${host.origin}/`);
    await page.locator("#onboardingScreen").waitFor({ state: "visible" });
    await page.waitForURL(/\/initialization$/);
    assert.equal(await page.title(), "Setup - ScreenerBot");

    const slides = await page.evaluate(() => {
      const glyph = (icon) => [...icon.classList].find((name) => name.startsWith("icon-"));
      return [...document.querySelectorAll(".onboarding-slide")].map((slide) => ({
        title: glyph(slide.querySelector(".slide-icon")),
        cards: [...slide.querySelectorAll(".slide-feature")].map((card) => ({
          glyph: glyph(card.querySelector(".slide-feature-icon")),
          title: card.querySelector("h4").textContent.trim(),
        })),
        description: slide.querySelector(".slide-description").textContent.trim(),
      }));
    });

    slides.forEach((slide, index) => {
      for (const card of slide.cards) {
        assert.notEqual(
          card.glyph,
          slide.title,
          `slide ${index + 1} card "${card.title}" repeats the slide's ${slide.title}`
        );
      }
    });

    const trade = slides[2].cards.map((card) => card.title);
    assert.ok(trade.includes("Copy Trading"), "the trade slide presents copy trading");
    assert.ok(!trade.includes("Best-Price Routing"));

    const data = slides[4];
    assert.match(data.description, /^We run a shared market-data service, so your install/);
    assert.deepEqual(
      data.cards.map((card) => card.title).slice(2),
      ["Account required", "Lookups only"]
    );
  } finally {
    await context.close();
    await host.close();
  }
});
