// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the token details bar above the tabs carries only dialog-wide notices.
 *
 * The bar once gained a "No chart data yet" notice after the chart poll backed off
 * (six empty polls, about 18 s after opening), on every tab, and pushed the tabs
 * down. Chart data is stated by the chart's own overlay and data indicator, so the
 * bar lists market and security notices only.
 *
 * - A token with no candles, its market and security data present: after the poll
 *   backs off the bar stays hidden and the tab row does not move, even when the
 *   detail response carries a chart entry.
 * - A token no market source lists: the bar names exactly the market sources.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";
const DIALOG = ".token-details-dialog";
// Long enough for the grace window and the chart poll's back-off on an empty chart.
const SETTLE_MS = 40_000;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [tokens, shell] = await Promise.all([loadIndex("tokens"), loadIndex("shell")]);

const notice = (source, label, state, id) => ({
  source,
  label,
  state,
  text: state === "ok" ? { id } : { id, args: { label: { type: "text", value: label } } },
});

/** Opens the first token's dialog with no candles and the given source statuses. */
async function openEmptyChartDialog(t, sourceStatus) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([tokens, shell], "populated");
  const onApi = async (request) => {
    const { pathname } = request.url;
    if (/^\/api\/tokens\/[^/]+\/ohlcv$/.test(pathname)) return { body: "[]" };
    const answer = await api.onApi(request);
    if (/^\/api\/tokens\/[^/]+\/ohlcv\/status$/.test(pathname)) {
      const status = JSON.parse(answer.body);
      status.has_data = false;
      status.total_candles = 0;
      status.timeframes = status.timeframes.map((tf) => ({ ...tf, candles: 0 }));
      return { ...answer, body: JSON.stringify(status) };
    }
    if (/^\/api\/tokens\/[^/]+$/.test(pathname) && request.method === "GET") {
      const detail = JSON.parse(answer.body);
      detail.source_status = sourceStatus;
      return { ...answer, body: JSON.stringify(detail) };
    }
    return answer;
  };
  const host = await serveDashboard(context, { locale: "en", onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.clock.install();
  await page.goto(`${host.origin}/tokens`);
  await page.waitForSelector(READY);
  await page.click("#tokens-root tr[data-row-id] .ti-row-cell__symbol");
  await page.waitForSelector(`${DIALOG} [data-dialog-tab="overview"]`);
  return page;
}

const tabRowTop = (page) =>
  page.$eval(`${DIALOG} [data-dialog-tab="overview"]`, (tab) => tab.getBoundingClientRect().top);

const bar = (page) =>
  page.$eval(`${DIALOG} #sourceIssuesRow`, (row) => ({
    hidden: row.hidden,
    notices: [...row.querySelectorAll(".source-issue-text")].map((el) =>
      el.textContent.replace(/[⁦-⁩]/g, "").trim()
    ),
  }));

test("an empty chart adds nothing above the tabs", async (t) => {
  const page = await openEmptyChartDialog(t, [
    notice("dexscreener", "DexScreener", "ok", "tokens-result-source-live"),
    notice("geckoterminal", "GeckoTerminal", "ok", "tokens-result-source-live"),
    notice("rugcheck", "Rugcheck", "ok", "tokens-result-security-available"),
    notice("ohlcv", "Chart", "no_data", "tokens-result-source-not-listed"),
  ]);
  const before = await tabRowTop(page);
  await page.clock.runFor(SETTLE_MS);
  assert.deepEqual(await bar(page), { hidden: true, notices: [] });
  assert.equal(await tabRowTop(page), before, "the tab row moved");
});

test("the bar names only dialog-wide sources", async (t) => {
  const page = await openEmptyChartDialog(t, [
    notice("dexscreener", "DexScreener", "no_data", "tokens-result-source-not-listed"),
    notice("geckoterminal", "GeckoTerminal", "no_data", "tokens-result-source-not-listed"),
    notice("rugcheck", "Rugcheck", "ok", "tokens-result-security-available"),
    notice("ohlcv", "Chart", "no_data", "tokens-result-source-not-listed"),
  ]);
  await page.clock.runFor(SETTLE_MS);
  assert.deepEqual(await bar(page), {
    hidden: false,
    notices: ["Not listed on DexScreener", "Not listed on GeckoTerminal"],
  });
});
