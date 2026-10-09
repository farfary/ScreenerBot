// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the token details dialog shows the holder count compact in its header
 * strip, with the exact count as that figure's tooltip, and exact in Overview >
 * Token Info, so the two never read as different numbers.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";

test("holders are compact in the header with the exact count in its tooltip", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("tokens"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(() => host.close());
  const page = await context.newPage();
  await page.goto(`${host.origin}/tokens`);
  await page.waitForSelector(READY);
  await page.click("#tokens-root tr[data-row-id] .ti-row-cell__symbol");
  const strip = page.locator('.token-details-dialog [data-live-value="holders"]');
  await page.waitForFunction(
    () =>
      globalThis.document.querySelector('.token-details-dialog [data-live-value="holders"]')
        ?.title &&
      globalThis.document.querySelector(
        '.token-details-dialog [data-tab-content="overview"] .overview-fact'
      )
  );
  const exact = await strip.getAttribute("title");
  const shown = (await strip.textContent()).trim();
  assert.match(exact, /^\d{1,3}(,\d{3})*$/, "the tooltip is the exact grouped count");
  const count = Number(exact.replaceAll(",", ""));
  if (count >= 1000) assert.match(shown, /[KMB]$/, "the strip is compact");

  const info = await page.$$eval(
    '.token-details-dialog [data-tab-content="overview"] .overview-fact',
    (facts) =>
      facts
        .filter(
          (fact) => fact.querySelector(".overview-fact-label")?.textContent.trim() === "Holders"
        )
        .map((fact) => fact.querySelector(".overview-fact-value").textContent.trim())
  );
  assert.deepEqual(info, [exact], "Token Info prints the exact count");
});
