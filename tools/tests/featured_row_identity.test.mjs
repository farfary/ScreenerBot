// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a featured card names a token the way every other surface does. A token the
 * providers sent without a symbol (empty, missing or an internal "unknown" marker)
 * shows its short mint and the shared letter avatar, never a placeholder such as
 * "???"; a token with a symbol still shows that symbol.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const indexes = [await loadIndex("tokens"), await loadIndex("shell")];
const FEATURED = JSON.parse(readFileSync(`${FIXTURES_ROOT}/tokens/featured_all.json`, "utf8"));

const NAMELESS = [
  { mint: "Nm1aX8kq3pSymbolMissing1111111111111111111pump", symbol: null },
  { mint: "Nm2bY9mr4qSymbolEmpty22222222222222222222222pump", symbol: "" },
  { mint: "Nm3cZ7ns5rSymbolUnknown3333333333333333333333pump", symbol: "UNKNOWN" },
].map((token) => ({
  ...FEATURED.dexscreener_trending[0],
  ...token,
  name: null,
  logo_url: null,
  image_url: null,
  boosts: 0,
  golden: false,
}));

test("a featured token without a symbol shows its short mint and the shared avatar", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(indexes, "populated");
  const body = JSON.stringify({ ...FEATURED, dexscreener_trending: NAMELESS });
  const onApi = async (request) =>
    request.url.pathname === "/api/featured/all" ? { body } : api.onApi(request);
  const host = await serveDashboard(context, { locale: "en", onApi });
  try {
    const page = await context.newPage();
    await page.goto(`${host.origin}/tokens`);
    await page.waitForSelector(PAGE_READY);
    await page.waitForSelector(`.featured-row-card[data-mint="${NAMELESS[0].mint}"]`);

    const cards = await page.evaluate((mints) =>
      mints.map((mint) => {
        const card = document.querySelector(`.featured-row-card[data-mint="${mint}"]`);
        return {
          label: card.querySelector(".featured-row-card-name").textContent.trim(),
          avatar: card.querySelector(".ti-logo .ti-logo-fallback")?.textContent.trim() ?? null,
          title: card.title,
        };
      }),
    NAMELESS.map((token) => token.mint));

    cards.forEach((card, index) => {
      const { mint } = NAMELESS[index];
      assert.equal(card.label, `${mint.slice(0, 4)}…${mint.slice(-4)}`);
      assert.equal(card.avatar, mint[0].toUpperCase());
      assert.ok(card.title.includes(mint), "the title carries the full mint");
    });

    const rowText = await page.locator(".featured-row-tokens").textContent();
    assert.ok(!rowText.includes("???"), "no placeholder symbol on the row");

    const named = FEATURED.boosted[0];
    const namedLabel = await page
      .locator(`.featured-row-card[data-mint="${named.mint}"] .featured-row-card-name`)
      .textContent();
    assert.equal(namedLabel.trim(), named.symbol.toUpperCase());
  } finally {
    await context.close();
    await host.close();
  }
});
