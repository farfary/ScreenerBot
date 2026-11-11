// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Tokens tables stay still under their polls and show each token's live state.
 *
 * - A poll that changed nothing writes no cell. Cells were compared with the browser's own
 *   serialization of their markup, which never equals the render, so every poll rebuilt
 *   every cell: the row buttons were replaced under the pointer and each logo was requested
 *   again, which a public IPFS gateway answers with 429.
 * - A logo that failed is not requested again when its row is rendered again.
 * - A row reads the live state the list sends: a held token offers add and sell, a
 *   blacklisted token's trade actions are disabled, Status shows the badges, and the risk
 *   score is toned by its band.
 * - The Rejected and Blacklisted views show their reason right after the token.
 * - A sideways scroll settles on a column edge, so no column is left cut by the pinned edge.
 * - A page opened on Favorites or OHLCV polls once the viewer switches to a list view; it
 *   had no poller, so every list view stayed frozen until the page was left.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("tokens"), await loadIndex("shell")];

const ROWS = "#tokens-root tr[data-row-id]";
const BONK = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";
const LOGO = "https://logos.example.invalid/bonk.png";

/** The Tokens page on recorded data; `patchRow` may change each listed row. */
async function tokensPage(t, { width = 1440, patchRow = null, view = null } = {}) {
  const context = await browser.newContext({ viewport: { width, height: 900 } });
  const api = createApiHandler(indexes, "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: async (request) => {
      const answer = await api.onApi(request);
      if (!patchRow || request.url.pathname !== "/api/tokens/list") return answer;
      const body = JSON.parse(answer.body);
      body.items = body.items.map(patchRow);
      return { ...answer, body: JSON.stringify(body) };
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/tokens${view ? `#${view}` : ""}`);
  await page.waitForSelector(PAGE_READY);
  if (!view) await page.waitForSelector(`${ROWS} >> visible=true`);
  return { page, host };
}

/** Wait until the table has taken `count` more polls of the list. */
async function polls(page, count) {
  for (let taken = 0; taken < count; taken += 1) {
    await page.waitForResponse((response) => response.url().includes("/api/tokens/list"));
  }
  await page.waitForTimeout(400);
}

/** Switch to the `view` sub-tab and wait until its rows are rendered. */
async function showView(page, view) {
  const loaded = page.waitForResponse(
    (response) =>
      response.url().includes("/api/tokens/list") && response.url().includes(`view=${view}`)
  );
  await page.locator(`#subTabsContainer [data-tab-id="${view}"]`).click();
  await loaded;
  await page.waitForTimeout(400);
}

const columnIds = (page) =>
  page.$$eval("#tokens-root th[data-column-id]", (headers) =>
    headers.map((header) => header.dataset.columnId)
  );

test("a poll that changed nothing writes no cell and keeps the row buttons", async (t) => {
  const { page } = await tokensPage(t);
  await page.evaluate(() => {
    const body = document.querySelector("#tokens-root tbody");
    window.__buttons = [...body.querySelectorAll(".row-action")];
    window.__cellWrites = 0;
    new window.MutationObserver((records) => {
      window.__cellWrites += records.filter((record) => record.target.closest?.("td")).length;
    }).observe(body, { childList: true, subtree: true });
  });

  await polls(page, 2);

  assert.equal(await page.evaluate(() => window.__cellWrites), 0, "no cell was written");
  assert.equal(
    await page.evaluate(() => window.__buttons.every((button) => button.isConnected)),
    true,
    "every row button kept its node"
  );
});

test("a logo that failed is not requested again when its row renders again", async (t) => {
  const { page, host } = await tokensPage(t, {
    patchRow: (row) => (row.mint === BONK ? { ...row, image_url: LOGO } : row),
  });
  const requests = () => host.unserved.filter((url) => url === LOGO).length;
  await page.waitForFunction(() => !document.querySelector("#tokens-root img.token-logo-artwork"));
  assert.equal(requests(), 1, "the logo was requested and failed");

  await showView(page, "pool");
  await showView(page, "all");
  await polls(page, 1);

  assert.equal(requests(), 1, "the failed logo was not requested again");
  assert.equal(
    await page.locator(`${ROWS}[data-row-id="${BONK}"] .ti-logo-fallback`).count(),
    1,
    "the row shows the letter avatar"
  );
});

test("a row reads the live state its actions, Status and Risk show", async (t) => {
  const { page } = await tokensPage(t);
  const actions = (mint) =>
    page.$$eval(`${ROWS}[data-row-id="${mint}"] .row-action[data-action]`, (buttons) =>
      buttons.map((button) => button.dataset.action)
    );
  assert.deepEqual(await actions(BONK), ["add", "sell"], "a held token offers add and sell");
  assert.deepEqual(await actions("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"), ["buy"]);

  const status = await page.$$eval(
    `${ROWS}[data-row-id="${BONK}"] td[data-column-id="status"] .badge`,
    (badges) => badges.map((badge) => badge.textContent.trim())
  );
  assert.deepEqual(status, ["Price", "OHLCV", "Position"]);
  assert.equal(
    await page
      .locator(`${ROWS}[data-row-id="${BONK}"] td[data-column-id="risk_score"] .value-positive`)
      .count(),
    1,
    "a low-band score is toned"
  );

  await showView(page, "blacklisted");
  const disabled = await page.$$eval(`${ROWS} .row-action[data-action="buy"]`, (buttons) =>
    buttons.map((button) => button.disabled)
  );
  assert.deepEqual(disabled, [true, true], "a blacklisted token cannot be bought");
});

test("the Rejected and Blacklisted views show the reason right after the token", async (t) => {
  const { page } = await tokensPage(t);
  for (const [view, reason] of [
    ["rejected", "reject_reason"],
    ["blacklisted", "blacklist_reason"],
  ]) {
    await showView(page, view);
    const ids = await columnIds(page);
    assert.equal(ids.indexOf(reason), ids.indexOf("token") + 1, `${view}: ${ids.join(", ")}`);
  }
});

test("a sideways scroll settles on a column edge", async (t) => {
  const { page } = await tokensPage(t, { width: 1000 });
  const settled = await page.evaluate(async () => {
    const container = document.querySelector("#tokens-root .data-table-scroll-container");
    // The header is its own table above the scroll container, kept in step with it.
    const scrolled = [
      ...document.querySelectorAll("#tokens-root th[data-column-id]:not(.dt-col-sticky)"),
    ];
    const [first, second] = scrolled.map((th) => th.getBoundingClientRect().width);
    // Most of the way through the second scrolled column: the nearer edge is its end.
    container.scrollLeft = Math.round(first + second * 0.7);
    await new Promise((resolve) => setTimeout(resolve, 1500));
    return {
      left: container.scrollLeft,
      edge: first + second,
      cut: document.querySelectorAll("#tokens-root th.dt-col-under").length,
    };
  });
  assert.ok(Math.abs(settled.left - settled.edge) <= 2, JSON.stringify(settled));
  assert.equal(settled.cut, 0, "no column is left cut by the pinned edge");
});

test("a page opened on Favorites polls the list view it switches to", async (t) => {
  const { page } = await tokensPage(t, { view: "favorites" });
  await showView(page, "all");
  const polled = await page
    .waitForResponse(
      (response) =>
        response.url().includes("/api/tokens/list") && response.url().includes("view=all"),
      { timeout: 12000 }
    )
    .then(() => true)
    .catch(() => false);
  assert.equal(polled, true, "the All view polled after leaving Favorites");
});
