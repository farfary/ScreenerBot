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
 * With the RugCheck stage off no token waits for a report: the notice, the awaiting
 * card and the awaiting figure of the info bar are all absent, signed out included.
 *
 * The Analytics headline shows the awaiting count beside passed and rejected, so the
 * three add up to the scanned total, and hides it with the stage off.
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
const ANALYTICS = JSON.parse(
  readFileSync(resolve(FIXTURES_ROOT, "filtering", "filtering_analytics.json"), "utf8")
);

// Fluent wraps each placeable, the `-rugcheck` term included, in Unicode isolation marks.
const AWAITING = "Awaiting \u2068RugCheck\u2069 Report";
const withoutIsolation = (text) => text.replace(/[\u2068\u2069]/g, "");

/**
 * Opens the Filtering status view with the stats fixture, signed in or out, with the
 * RugCheck stage on unless `reportRequired` is false.
 */
async function openStatus(signedIn, reportRequired = true) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const fixtures = createApiHandler([filtering, shell], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    startupPage: "filtering",
    onApi: (request) => {
      if (request.url.pathname === "/api/filtering/stats") {
        return {
          body: JSON.stringify({
            ...STATS,
            signed_in: signedIn,
            rugcheck_report_required: reportRequired,
          }),
        };
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
    const notice = session.page.locator('.status-metrics-section > .notice[data-variant="info"]');
    assert.equal(
      withoutIsolation(await notice.locator(".notice-message").textContent()).trim(),
      "RugCheck reports arrive at about 3 a minute. Signed-in installs get them faster."
    );
    await notice.locator('[data-action="filtering-sign-in"]').click();
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
    assert.equal(await session.page.locator(".status-metrics-section .notice").count(), 0);
  } finally {
    await session.close();
  }
});

test("tokens awaiting their report have their own card, apart from the passed count", async () => {
  const session = await openStatus(true);
  try {
    const card = session.page.locator(".status-view .metric-card", {
      hasText: AWAITING,
    });
    assert.equal((await card.locator(".metric-value").textContent()).trim(), "2,140");
  } finally {
    await session.close();
  }
});

test("with the RugCheck stage off nothing awaits a report and no notice shows", async () => {
  const session = await openStatus(false, false);
  try {
    const { page } = session;
    assert.equal(await page.locator(".status-metrics-section .notice").count(), 0);
    const awaitingCard = page.locator(".status-view .metric-card", {
      hasText: AWAITING,
    });
    assert.equal(await awaitingCard.count(), 0);
    const chips = await page.$$eval(".table-toolbar-chip__label", (labels) =>
      labels.map((label) => label.textContent.trim())
    );
    assert.ok(chips.length > 0, "the info bar renders");
    assert.ok(
      chips.every((label) => !/awaiting/i.test(label)),
      `no awaiting figure in the info bar: ${chips.join(" | ")}`
    );
  } finally {
    await session.close();
  }
});

/** Opens the Filtering Analytics view with the RugCheck stage on or off. */
async function openAnalytics(reportRequired) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const fixtures = createApiHandler([filtering, shell], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    startupPage: "filtering",
    onApi: (request) => {
      if (request.url.pathname === "/api/filtering/analytics") {
        return {
          body: JSON.stringify({ ...ANALYTICS, rugcheck_report_required: reportRequired }),
        };
      }
      return fixtures.onApi(request);
    },
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/filtering`);
  await page.waitForSelector(PAGE_READY);
  await page.locator('#subTabsContainer [data-tab-id="analytics"]').click();
  await page.locator(".analytics-view .metric-card").first().waitFor();
  return {
    page,
    async close() {
      await context.close();
      await host.close();
    },
  };
}

/** Headline card values by label, as grouped numbers parsed back to integers. */
async function headlineCounts(page) {
  const cards = await page.$$eval(".analytics-view .metric-grid .metric-card", (els) =>
    els.map((el) => [
      el.querySelector(".metric-label").textContent.trim(),
      el.querySelector(".metric-value").textContent.trim(),
    ])
  );
  return new Map(cards.map(([label, value]) => [label, Number(value.replace(/,/g, ""))]));
}

test("the Analytics headline shows the awaiting count, and the counts add up", async () => {
  const session = await openAnalytics(true);
  try {
    const counts = await headlineCounts(session.page);
    assert.equal(counts.get(AWAITING), 2140);
    assert.equal(
      counts.get("Passed Tokens") + counts.get("Rejected Tokens") + counts.get(AWAITING),
      counts.get("Total Scanned")
    );
    const share = session.page.locator(".analytics-view .metric-card", {
      hasText: AWAITING,
    });
    assert.equal(
      withoutIsolation(await share.locator(".metric-detail").textContent()).trim(),
      "11.6% awaiting their report"
    );
  } finally {
    await session.close();
  }
});

test("with the RugCheck stage off the Analytics headline has no awaiting card", async () => {
  const session = await openAnalytics(false);
  try {
    const counts = await headlineCounts(session.page);
    assert.equal(counts.size, 3);
    assert.ok(!counts.has(AWAITING));
  } finally {
    await session.close();
  }
});
