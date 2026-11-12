// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a paper round opens the same position details dialog as a live one.
 *
 * - Clicking a paper row reads the details of the paper book by the round's id, never the
 *   live book, where the same id names another round.
 * - The header carries the Paper marker and none of the live wallet controls: no add, sell
 *   or management. An open round offers its close at a paper fill, a closed one the token.
 * - The close is confirmed, sent to the paper close route once and followed by a fresh read
 *   of the round; no live trade request is made.
 * - The Activity pane lists the round's fills from the details response, never reads the
 *   activity endpoint, shows each fill's strategy and links no transaction.
 * - The paper close request is spelled once, in `ui/manual_trade.js`, for the row action and
 *   the dialog alike.
 * - A paper row's right-click menu offers no live sell, add or management: those act on the
 *   live book by mint.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { loadMarkupSources } from "../lib/dashboard_ui.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const ROW_SYMBOL = "#positions-root tr[data-row-id] .ti-row-cell__symbol";
const DIALOG = ".position-details-dialog";
const OWNER = "src/webserver/templates/scripts/ui/manual_trade.js";
const plain = (text) => text.replace(/[⁨⁩]/g, "").trim();

const closedPatch = {
  status: "closed",
  exit_time: 1791320000,
  transaction_exit_verified: true,
};

/**
 * The positions page on the paper book with the dialog of its first round open.
 * `closed` shows the Closed list; `onApi` answers a request first when it returns a
 * response. Every API request is recorded as `METHOD pathname`.
 */
async function openPaperDialog(t, { closed = false, onApi = () => null } = {}) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(
    [await loadIndex("positions"), await loadIndex("shell")],
    "populated"
  );
  const requests = [];
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      requests.push(`${request.method} ${request.url.pathname}`);
      return onApi(request) ?? api.onApi(request);
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/positions`);
  await page.waitForSelector(PAGE_READY);
  await page
    .locator('[data-seg="book"] .segmented-option', { has: page.locator('input[value="paper"]') })
    .click();
  if (closed) await page.locator('#subTabsContainer [data-tab-id="closed"]').click();
  await page.locator(ROW_SYMBOL).first().click();
  const dialog = page.locator(DIALOG);
  await dialog.locator(".pdd-act-card").first().waitFor();
  return { page, dialog, requests };
}

test("a paper row opens the details of the paper book by its id", async (t) => {
  const { dialog, requests } = await openPaperDialog(t);

  assert.ok(requests.includes("GET /api/positions/paper/7/details"));
  assert.deepEqual(
    requests.filter((request) => /^GET \/api\/positions\/id:/.test(request)),
    [],
    "the live book is not read for a paper round"
  );
  assert.equal((await dialog.locator("#pdd-dialog-title").textContent()).trim(), "Bonk");
  assert.ok(await dialog.locator(".header-metric").count(), "the shared header figures show");
});

test("the header carries the Paper marker and no live wallet control", async (t) => {
  const { dialog } = await openPaperDialog(t);

  const marker = dialog.locator('.header-badges .mode-marker[data-mode="paper"]');
  assert.equal(plain(await marker.textContent()), "Paper");
  assert.equal(await marker.locator("i").count(), 1, "Paper is its glyph inline with its label");
  assert.equal(await dialog.locator('[data-trade-action="add"]').count(), 0);
  assert.equal(await dialog.locator('[data-trade-action="sell"]').count(), 0);
  assert.equal(await dialog.locator('[data-trade-action="close"]').count(), 0);
  assert.equal(await dialog.locator("#pddManagementSelect").count(), 0);
  assert.equal(await dialog.locator(".pdd-management").count(), 0);

  const close = dialog.locator('[data-trade-action="paper-close"]');
  assert.equal(await close.count(), 1, "an open round offers its close at a paper fill");
  assert.equal(await close.getAttribute("title"), "Close paper position");
  assert.equal(await dialog.locator("#pddFavoriteBtn").count(), 1, "favorites stay");
});

test("a closed paper round offers the token and no close", async (t) => {
  const { dialog } = await openPaperDialog(t, { closed: true });

  assert.equal(await dialog.locator('.header-badges .mode-marker[data-mode="paper"]').count(), 1);
  assert.equal(await dialog.locator('[data-trade-action="paper-close"]').count(), 0);
  assert.equal(await dialog.locator('[data-trade-action="token"]').count(), 1);
  assert.equal(await dialog.locator(".pdd-status.is-closed").count(), 1);
});

test("the Activity pane lists the fills without reading the activity endpoint", async (t) => {
  const { dialog, requests } = await openPaperDialog(t);

  const cards = dialog.locator(".pdd-act-card");
  assert.deepEqual(await cards.evaluateAll((nodes) => nodes.map((node) => node.dataset.kind)), [
    "dca",
    "entry",
  ]);
  assert.deepEqual(
    requests.filter((request) => request.includes("/activity")),
    [],
    "the activity endpoint reads the live book"
  );

  const entry = dialog.locator('.pdd-act-card[data-kind="entry"]');
  assert.match(plain(await entry.locator(".pdd-act-description").textContent()), /^Bought .+ for /);
  const strategy = entry.locator(".pdd-act-detail-metric", { hasText: "Strategy" });
  assert.equal(plain(await strategy.locator("strong").textContent()), "paper-momentum");
  const dca = dialog.locator('.pdd-act-card[data-kind="dca"] .pdd-act-detail-metric', {
    hasText: "Strategy",
  });
  assert.equal(plain(await dca.locator("strong").textContent()), "paper-dca");
});

test("a closed round lists its exit and no fill links a transaction", async (t) => {
  const { dialog } = await openPaperDialog(t, { closed: true });

  assert.deepEqual(
    await dialog
      .locator(".pdd-act-card")
      .evaluateAll((nodes) => nodes.map((node) => node.dataset.kind)),
    ["exit", "entry"]
  );
  const exit = dialog.locator('.pdd-act-card[data-kind="exit"] .pdd-act-detail-metric', {
    hasText: "Strategy",
  });
  assert.equal(plain(await exit.locator("strong").textContent()), "paper-take-profit");

  assert.equal(await dialog.locator(".pdd-act-sig-link, .pdd-act-signature").count(), 0);
  const html = await dialog.evaluate((element) => element.innerHTML);
  assert.doesNotMatch(html, /paper-\d+-(?:entry|dca|exit)/, "no paper leg reference is shown");
  assert.equal(await dialog.locator('a[href*="/tx/"]').count(), 0);
  assert.equal(await dialog.locator(".pdd-act-sig-na").count(), 2);
});

test("Close at paper price asks, closes through the paper route once and rereads the round", async (t) => {
  let closed = false;
  const closes = [];
  const { page, dialog, requests } = await openPaperDialog(t, {
    onApi: (request) => {
      if (request.method === "POST" && request.url.pathname === "/api/positions/paper/7/close") {
        closes.push(request.url.pathname);
        closed = true;
        return { body: JSON.stringify({ closed: true }) };
      }
      if (closed && request.url.pathname === "/api/positions/paper/7/details") {
        const details = JSON.parse(
          readFileSync(`${FIXTURES_ROOT}/positions/position_details_paper_open.json`, "utf8")
        );
        Object.assign(details.position, closedPatch);
        return { body: JSON.stringify(details) };
      }
      return null;
    },
  });

  await dialog.locator('[data-trade-action="paper-close"]').click();
  const confirmation = page.locator(".confirmation-dialog--visible");
  await confirmation.waitFor();
  assert.match(
    plain(await confirmation.locator(".confirmation-dialog__message").innerText()),
    /^BONK closes in full at a paper fill\. No swap is sent\.$/
  );
  await confirmation.locator('[data-action="cancel"]').click();
  await confirmation.waitFor({ state: "hidden" });
  assert.deepEqual(closes, [], "a cancelled close sends nothing");

  await dialog.locator('[data-trade-action="paper-close"]').click();
  await confirmation.waitFor();
  await confirmation.locator('[data-action="confirm"]').click();

  await dialog.locator(".pdd-status.is-closed").waitFor();
  assert.deepEqual(closes, ["/api/positions/paper/7/close"]);
  assert.equal(await dialog.locator('[data-trade-action="paper-close"]').count(), 0);
  assert.deepEqual(
    requests.filter((request) => request.startsWith("POST /api/trader/")),
    [],
    "no live trade request is made"
  );
});

/** The root item labels of the right-click menu of the first row on the `book` list. */
async function positionMenuLabels(t, book) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(
    [await loadIndex("positions"), await loadIndex("shell")],
    "populated"
  );
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/positions`);
  await page.waitForSelector(PAGE_READY);
  await page.waitForSelector("#splashScreen", { state: "hidden" });
  await page
    .locator('[data-seg="book"] .segmented-option', { has: page.locator(`input[value="${book}"]`) })
    .click();
  const row = page.locator("#positions-root tr[data-row-id]").first();
  await row.waitFor();
  assert.equal(await row.getAttribute("data-book"), book);
  await row.click({ button: "right" });
  const menu = page.locator(".context-menu.visible");
  await menu.waitFor();
  return menu
    .locator(":scope > .context-menu-item > .context-menu-label")
    .evaluateAll((labels) => labels.map((label) => label.textContent.trim()));
}

test("a paper row's menu offers no live sell, add or management", async (t) => {
  const paper = await positionMenuLabels(t, "paper");
  assert.ok(paper.includes("View Details"), `menu: ${paper.join(", ")}`);
  for (const live of ["Add to Position", "Management"]) {
    assert.ok(!paper.includes(live), `a paper row offers ${live}`);
  }
  assert.deepEqual(
    paper.filter((label) => label.startsWith("Sell ")),
    [],
    "a paper row offers no sale of the live holding"
  );

  const live = await positionMenuLabels(t, "live");
  assert.ok(live.includes("Add to Position") && live.includes("Management"));
  assert.ok(live.some((label) => label.startsWith("Sell ")));
});

test("the paper close request is spelled once, in the manual trade module", async () => {
  const spelled = [];
  for (const { path, source } of await loadMarkupSources()) {
    if (/positions\/paper\/\$\{[^}]*\}\/close/.test(source)) spelled.push(path);
  }
  assert.deepEqual(spelled, [OWNER]);
});
