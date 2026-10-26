// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Filtering status panel states the signed-out RugCheck report rate.
 *
 * Signed out, reports come only from the bounded direct fetches, so the panel says how
 * many arrive a minute, that signed-in installs get them faster, and offers the sign-in
 * that opens the account tab of Settings. Signed in, the notice is absent. The tokens
 * waiting for their report have their own card, apart from the passed count.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex, FIXTURES_ROOT } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [shell, filtering] = await Promise.all([loadIndex("shell"), loadIndex("filtering")]);
const STATS = JSON.parse(
  readFileSync(resolve(FIXTURES_ROOT, "filtering", "filtering_stats.json"), "utf8")
);

/** Opens the Filtering status view with the stats fixture, signed in or out. */
async function openStatus(signedIn) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const fixtures = createApiHandler([filtering, shell], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    startupPage: "filtering",
    onApi: (request) => {
      if (request.url.pathname === "/api/filtering/stats") {
        return { body: JSON.stringify({ ...STATS, signed_in: signedIn }) };
      }
      return fixtures.onApi(request);
    },
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/filtering`);
  await page.waitForSelector(PAGE_READY);
  await page.locator('#subTabsContainer [data-tab-id="status"]').click();
  await page.locator(".status-view .metric-card").first().waitFor();
  return {
    page,
    async close() {
      await context.close();
      await host.close();
    },
  };
}

test("signed out, the status panel states the direct report rate and offers sign-in", async () => {
  const session = await openStatus(false);
  try {
    const notice = session.page.locator(".status-report-notice");
    assert.equal(
      (await notice.locator(".status-report-notice__text").textContent()).trim(),
      "RugCheck reports arrive at about 3 a minute. Signed-in installs get them faster."
    );
    await notice.locator("[data-filtering-sign-in]").click();
    await session.page
      .locator('.settings-nav-item.active[data-tab="account"]')
      .waitFor({ state: "attached" });
  } finally {
    await session.close();
  }
});

test("signed in, the status panel shows no report notice", async () => {
  const session = await openStatus(true);
  try {
    assert.equal(await session.page.locator(".status-report-notice").count(), 0);
  } finally {
    await session.close();
  }
});

test("tokens awaiting their report have their own card, apart from the passed count", async () => {
  const session = await openStatus(true);
  try {
    const card = session.page.locator(".status-view .metric-card", {
      hasText: "Awaiting RugCheck Report",
    });
    assert.equal(
      (await card.locator(".metric-value").textContent()).trim(),
      "2,140"
    );
  } finally {
    await session.close();
  }
});
