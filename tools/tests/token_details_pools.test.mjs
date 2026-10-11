// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the token details Pools tab states each fact once. The summary grid fills whole
 * rows (no lone cell beside an empty one), the pool count lives only in the All pools
 * heading, and a pool card never lists one address under two labels.
 *
 * Market cap, liquidity and 24h volume are shown in the dialog header and in one
 * section of a tab, never again: Overview names each at most once, and Pools states
 * liquidity and volume once for the summary and once per pool card (no separate
 * canonical-pool block repeating its card). A total no pool reports is unknown, "—",
 * never "$0.00". The transaction activity counts read as one unit, "42 / 33", set at the
 * same edge as the percent line under them.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [tokens, shell] = await Promise.all([loadIndex("tokens"), loadIndex("shell")]);

test("the Pools tab fills its summary grid and repeats no address", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([tokens, shell], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}/tokens`);
  await page.waitForSelector(PAGE_READY);
  await page.click("#tokens-root tr[data-row-id] .ti-row-cell__symbol");
  await page.click('.token-details-dialog [data-dialog-tab="pools"]');
  await page.waitForSelector(".token-details-dialog .pools-summary-grid .pools-summary-fact");

  const layout = await page.evaluate(() => {
    const grid = document.querySelector(".token-details-dialog .pools-summary-grid");
    const columns = getComputedStyle(grid).gridTemplateColumns.split(" ").length;
    const cards = [...document.querySelectorAll(".token-details-dialog .pool-detail")];
    return {
      columns,
      facts: grid.querySelectorAll(".pools-summary-fact").length,
      cards: cards.length,
      repeated: cards.flatMap((card) => {
        const addresses = [
          ...card.querySelectorAll(".pool-detail-addresses .ti-address-copy[data-copy]"),
        ].map((button) => button.dataset.copy);
        return addresses.filter((address, index) => addresses.indexOf(address) !== index);
      }),
    };
  });

  assert.ok(layout.cards > 0, "the fixture token lists at least one pool");
  assert.equal(
    layout.facts % layout.columns,
    0,
    `${layout.facts} facts leave a lone cell in ${layout.columns} columns`
  );
  assert.deepEqual(layout.repeated, [], "a pool card lists one address under two labels");
});

test("Overview and Pools state market cap, liquidity and volume once", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([tokens, shell], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}/tokens`);
  await page.waitForSelector(PAGE_READY);
  await page.click("#tokens-root tr[data-row-id] .ti-row-cell__symbol");
  const labels = (panel) =>
    page.$$eval(`.token-details-dialog [data-tab-content="${panel}"] *`, (nodes) =>
      nodes
        .filter((node) => node.children.length === 0 && node.getClientRects().length > 0)
        .map((node) => node.textContent.trim())
    );
  const count = (texts, label) => texts.filter((text) => text === label).length;

  await page.waitForSelector(
    '.token-details-dialog [data-tab-content="overview"] .overview-section'
  );
  const overview = await labels("overview");
  for (const label of ["Market Cap", "Liquidity"]) {
    assert.ok(
      count(overview, label) <= 1,
      `Overview names ${label} ${count(overview, label)} times`
    );
  }

  await page.click('.token-details-dialog [data-dialog-tab="pools"]');
  await page.waitForSelector(".token-details-dialog .pool-detail");
  const cards = await page.locator(".token-details-dialog .pool-detail").count();
  const pools = await labels("pools");
  assert.equal(count(pools, "Liquidity"), cards + 1, "summary plus one per pool card");
  assert.equal(count(pools, "24h Volume"), cards + 1, "summary plus one per pool card");
});

async function openDetails(t, answers = {}) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([tokens, shell], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      const body = answers[request.url.pathname];
      return body ? { body } : api.onApi(request);
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}/tokens`);
  await page.waitForSelector(PAGE_READY);
  await page.click("#tokens-root tr[data-row-id] .ti-row-cell__symbol");
  return page;
}

test("a pool total no pool reports reads as unknown, not zero", async (t) => {
  const mint = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";
  const body = JSON.parse(readFileSync(`${FIXTURES_ROOT}/tokens/token_detail_bonk.json`, "utf8"));
  body.pools = body.pools.map((pool) => ({ ...pool, liquidity_usd: null }));
  const page = await openDetails(t, { [`/api/tokens/${mint}`]: JSON.stringify(body) });
  await page.click('.token-details-dialog [data-dialog-tab="pools"]');
  await page.waitForSelector(".token-details-dialog .pools-summary-fact");
  const summary = await page.$$eval(".token-details-dialog .pools-summary-fact", (facts) =>
    Object.fromEntries(
      facts.map((fact) => [
        fact.querySelector("span").textContent.trim(),
        fact.querySelector("strong").textContent.trim(),
      ])
    )
  );
  assert.equal(summary.Liquidity, "—");
  assert.notEqual(summary["24h Volume"], "—", "a reported total still shows");
});

test("transaction activity counts read as one unit over their percent line", async (t) => {
  const page = await openDetails(t);
  await page.waitForSelector(".token-details-dialog .overview-flow-counts-main");
  const rows = await page.$$eval(".token-details-dialog .overview-flow-counts", (cells) =>
    cells.map((cell) => {
      const box = (selector) => cell.querySelector(selector).getBoundingClientRect();
      return {
        split: box(".overview-flow-sell-count").left - box(".overview-flow-buy-count").right,
        edges: Math.abs(
          box(".overview-flow-counts-main").right - box(".overview-flow-counts-sub").right
        ),
      };
    })
  );
  assert.ok(rows.length > 0, "the fixture token shows transaction activity");
  for (const { split, edges } of rows) {
    assert.ok(split < 16, `buy and sell counts sit ${split}px apart`);
    assert.ok(edges < 2, `the counts and the percent line end ${edges}px apart`);
  }
});
