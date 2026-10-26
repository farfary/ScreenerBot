// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the token details Overview states a token's age as a span.
 *
 * The Age fact once printed a moment under its label ("Age 468d ago"), and read a
 * second creation field the detail response no longer carries. It reads the one
 * `created_at` and shows the elapsed span ("468d 5h"); a token whose creation is not
 * known reads "—".
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const DIALOG = ".token-details-dialog";
const DAY = 86_400;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [tokens, shell] = await Promise.all([loadIndex("tokens"), loadIndex("shell")]);

/** The Age fact of the first token's Overview, its detail answering `createdAt`. */
async function ageFact(t, createdAt) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([tokens, shell], "populated");
  const onApi = async (request) => {
    const answer = await api.onApi(request);
    if (/^\/api\/tokens\/[^/]+$/.test(request.url.pathname) && request.method === "GET") {
      return { ...answer, body: JSON.stringify({ ...JSON.parse(answer.body), created_at: createdAt }) };
    }
    return answer;
  };
  const host = await serveDashboard(context, { locale: "en", onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/tokens`);
  await page.waitForSelector(PAGE_READY);
  await page.click("#tokens-root tr[data-row-id] .ti-row-cell__symbol");
  const fact = page.locator(`${DIALOG} .overview-fact`, { hasText: "Age" });
  await fact.waitFor();
  return (await fact.locator(".overview-fact-value").textContent()).trim();
}

test("a known creation reads as the span since it", async (t) => {
  const now = Math.floor(Date.now() / 1000);
  assert.match(await ageFact(t, now - 468 * DAY - 5 * 3600 - 60), /^468d 5h$/);
});

test("an unknown creation reads as one dash", async (t) => {
  assert.equal(await ageFact(t, null), "—");
});
