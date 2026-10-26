// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the featured overlay offers Buy only on tokens a buy can acquire, and its
 * cards share one height.
 *
 * Wrapped SOL is the native coin every buy spends, so its card has no Buy; a
 * stablecoin is an ordinary token and keeps it. A card missing stats or socials is
 * as tall as its neighbours, so each grid reads as even rows with aligned footers.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const WSOL = "So11111111111111111111111111111111111111112";
const USDC = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const FEATURED = JSON.parse(readFileSync(`${FIXTURES_ROOT}/tokens/featured_all.json`, "utf8"));
const [template] = FEATURED.jupiter_organic;
const sparse = (token) => ({
  ...token,
  holders: null,
  txns_24h_buys: null,
  txns_24h_sells: null,
  volume_24h: null,
  website: null,
  twitter: null,
  telegram: null,
});
const BODY = JSON.stringify({
  ...FEATURED,
  jupiter_organic: [
    { ...template, mint: WSOL, symbol: "WSOL", name: "Wrapped SOL" },
    sparse({ ...template, mint: USDC, symbol: "USDC", name: "USD Coin" }),
    ...FEATURED.jupiter_organic,
  ],
});

test("the featured overlay has no Buy on wrapped SOL and cards of one height", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("tokens"), await loadIndex("shell")], "populated");
  const onApi = async (request) =>
    request.url.pathname === "/api/featured/all" ? { body: BODY } : api.onApi(request);
  const host = await serveDashboard(context, { locale: "en", onApi });
  try {
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/tokens`);
    await page.waitForSelector(PAGE_READY);
    await page.evaluate(async () => {
      const { openFeaturedDialog } = await import("/scripts/ui/featured_dialog.js");
      openFeaturedDialog();
    });
    await page.waitForSelector(`.featured-dialog .feat-card[data-mint="${WSOL}"]`);

    const buy = (mint) => page.locator(`.feat-card[data-mint="${mint}"] [data-action="buy"]`);
    assert.equal(await buy(WSOL).count(), 0, "wrapped SOL offers no Buy");
    assert.equal(await buy(USDC).count(), 1, "a stablecoin keeps Buy");

    const grids = await page.evaluate(() =>
      [...document.querySelectorAll(".featured-cat-grid")].map((grid) =>
        [...grid.querySelectorAll(".feat-card")].map((card) => ({
          mint: card.dataset.mint,
          height: Math.round(card.getBoundingClientRect().height),
          footer: Math.round(
            card.querySelector(".feat-card-actions").getBoundingClientRect().bottom -
              card.getBoundingClientRect().top
          ),
        }))
      )
    );
    for (const cards of grids.filter((cards) => cards.length > 1)) {
      const heights = new Set(cards.map((card) => card.height));
      assert.equal(heights.size, 1, `cards differ in height: ${JSON.stringify(cards)}`);
      const footers = new Set(cards.map((card) => card.footer));
      assert.equal(footers.size, 1, `footers are not aligned: ${JSON.stringify(cards)}`);
    }
  } finally {
    await context.close();
    await host.close();
  }
});
