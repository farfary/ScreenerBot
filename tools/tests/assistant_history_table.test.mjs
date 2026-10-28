// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Assistant History tab is a DataTable that fills the panel.
 *
 * - Every decision is one row, with the decision, confidence and model columns.
 * - The table scrolls inside the panel; the page itself does not scroll.
 * - Every request names a real page (never `page=null`).
 * - A failed first load shows the error with Retry, and Retry loads page 1 again.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex, FIXTURES_ROOT } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("assistant"), await loadIndex("shell")];
const history = JSON.parse(await readFile(join(FIXTURES_ROOT, "assistant/history.json"), "utf8"));
const decisions = history.decisions.slice(0, 2);

test("History lists each decision as a table row and does not scroll the page", async () => {
  const context = await browser.newContext({ viewport: { width: 1200, height: 780 } });
  const api = createApiHandler(indexes, "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) =>
      request.url.pathname === "/api/llm-analysis/history"
        ? { body: JSON.stringify({ ...history, decisions, total: decisions.length }) }
        : api.onApi(request),
  });
  try {
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/assistant#history`);
    await page.waitForSelector(PAGE_READY);
    await page.waitForSelector("#history-root tbody tr[data-row-id]");

    const rows = await page.$$eval("#history-root tbody tr[data-row-id]", (found) =>
      found.map((row) => row.dataset.rowId)
    );
    assert.deepEqual(
      rows,
      decisions.map((decision) => String(decision.id))
    );

    const columns = await page.$$eval("#history-root th[data-column-id]", (headers) =>
      headers.map((header) => header.dataset.columnId)
    );
    assert.deepEqual(columns, [
      "token",
      "decision",
      "confidence",
      "risk_level",
      "reasoning",
      "model",
      "latency_ms",
      "created_at",
    ]);

    const overflow = await page.evaluate(() => {
      const panel = document.querySelector(".assistant-main-panel");
      return {
        panel: panel.scrollHeight - panel.clientHeight,
        document: document.scrollingElement.scrollHeight - document.scrollingElement.clientHeight,
      };
    });
    assert.ok(overflow.panel <= 0, `the panel scrolls by ${overflow.panel}px`);
    assert.ok(overflow.document <= 0, `the page scrolls by ${overflow.document}px`);
  } finally {
    await context.close();
    await host.close();
  }
});

async function openHistory({ respond }) {
  const context = await browser.newContext({ viewport: { width: 1200, height: 780 } });
  const api = createApiHandler(indexes, "populated");
  const requests = [];
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      if (request.url.pathname !== "/api/llm-analysis/history") return api.onApi(request);
      requests.push(request.url.search);
      return respond(requests.length);
    },
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/assistant#history`);
  await page.waitForSelector(PAGE_READY);
  return {
    page,
    requests,
    close: async () => {
      await context.close();
      await host.close();
    },
  };
}

const okay = () => ({ body: JSON.stringify({ ...history, decisions, total: decisions.length }) });

test("History requests only real pages", async () => {
  const { page, requests, close } = await openHistory({ respond: okay });
  try {
    await page.waitForSelector("#history-root tbody tr[data-row-id]");
    assert.deepEqual(requests, ["?page=1&per_page=20"]);
  } finally {
    await close();
  }
});

test("Retry that returns no rows replaces the failure text with the plain empty state", async () => {
  const { page, close } = await openHistory({
    respond: (count) =>
      count === 1
        ? { status: 500, body: "{}" }
        : { body: JSON.stringify({ ...history, decisions: [], total: 0 }) },
  });
  try {
    const empty = page.locator("#history-root .dt-state-cell");
    await empty.waitFor();
    assert.match(await empty.textContent(), /Failed to load history/);
    await empty.getByRole("button", { name: "Retry" }).click();
    await page.waitForFunction(() =>
      /No LLM-analysis requests yet/.test(
        document.querySelector("#history-root .dt-state-cell")?.textContent ?? ""
      )
    );
    assert.doesNotMatch(await empty.textContent(), /Failed to load history/);
  } finally {
    await close();
  }
});

test("a failed first load shows the error with Retry, and Retry loads page 1 again", async () => {
  const { page, requests, close } = await openHistory({
    respond: (count) => (count === 1 ? { status: 500, body: "{}" } : okay()),
  });
  try {
    const empty = page.locator("#history-root .dt-state-cell");
    await empty.waitFor();
    assert.match(await empty.textContent(), /Failed to load history/);
    assert.match(await empty.textContent(), /The decision history could not be read\./);
    await empty.getByRole("button", { name: "Retry" }).click();
    await page.waitForSelector("#history-root tbody tr[data-row-id]");
    assert.deepEqual(requests, ["?page=1&per_page=20", "?page=1&per_page=20"]);
  } finally {
    await close();
  }
});
