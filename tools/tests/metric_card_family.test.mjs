// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: one stat card family, stacked from the top, coloured by meaning.
 *
 * Stat cards were vertically centred, so a card with a detail line pushed its label and
 * value away from those of its neighbours; Filtering Analytics drew its own card; and
 * some cards carried a primary border tint with no state behind it. A count that was
 * zero or absent still showed green or red.
 *
 * - Within one row of stat cards, every label and every value starts on the same line
 *   (Home strip, Auto Trader Stats, Filtering Status and Analytics).
 * - Filtering Analytics uses the shared `.metric-card`.
 * - Only a state class tints a stat card border; peers otherwise share one border.
 * - A zero or absent pipeline count carries no tone.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { SCRIPTS_ROOT, walk } from "../lib/dashboard_ui.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";
const tab = (id) => `#subTabsContainer [data-tab-id="${id}"]`;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const shell = await loadIndex("shell");

async function openPage(t, name, variant = "populated") {
  const context = await browser.newContext({ viewport: { width: 1200, height: 900 } });
  const api = createApiHandler([await loadIndex(name), shell], variant);
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/${name}`);
  await page.waitForSelector(READY);
  return page;
}

/** Rows of cards (grouped by their top edge) with the top of each named part. */
async function rowTops(page, cardSelector, parts) {
  return page.$$eval(
    cardSelector,
    (cards, parts) => {
      const rows = new Map();
      for (const card of cards) {
        const top = Math.round(card.getBoundingClientRect().top);
        const tops = {};
        for (const [name, selector] of Object.entries(parts)) {
          const el = card.querySelector(selector);
          tops[name] = el ? Math.round(el.getBoundingClientRect().top) : null;
        }
        if (!rows.has(top)) rows.set(top, []);
        rows.get(top).push(tops);
      }
      return [...rows.values()];
    },
    parts
  );
}

function assertAligned(rows, where) {
  assert.ok(rows.length > 0, `${where} renders stat cards`);
  for (const row of rows) {
    for (const part of Object.keys(row[0])) {
      const tops = new Set(row.map((card) => card[part]));
      assert.equal(tops.size, 1, `${where}: every ${part} in a row starts on one line`);
    }
  }
}

test("Home strip labels and values share one line", async (t) => {
  const page = await openPage(t, "home");
  await page.waitForSelector(".home-dashboard:not(.loading)");
  const rows = await rowTops(page, ".portfolio-stat", {
    label: ".hero-stat-label",
    value: ".hero-stat-value",
  });
  assertAligned(rows, "Home strip");
});

test("Auto Trader Stats cards align and only a state tints a border", async (t) => {
  const page = await openPage(t, "trader");
  await page.click(tab("stats"));
  await page.waitForFunction(() => document.querySelector("#net-pnl")?.textContent !== "—");
  const rows = await rowTops(page, ".metrics-grid .metric-card", {
    label: ".metric-label",
    value: ".metric-value",
  });
  assertAligned(rows, "Auto Trader Stats");

  const borders = await page.$$eval(".metrics-grid .metric-card", (cards) =>
    cards
      .filter((card) => card.offsetParent !== null)
      .map((card) => ({
        state: card.classList.contains("metric-highlight-danger"),
        border: getComputedStyle(card).borderTopColor,
      }))
  );
  const plain = new Set(borders.filter((card) => !card.state).map((card) => card.border));
  assert.equal(plain.size, 1, "stat cards without a state share one border colour");
});

test("Filtering Status and Analytics use the shared card, aligned", async (t) => {
  const page = await openPage(t, "filtering");
  await page.click(tab("status"));
  await page.waitForSelector(".status-view .metric-card");
  assertAligned(
    await rowTops(page, ".status-view .metric-card", {
      label: ".metric-label",
      value: ".metric-value",
    }),
    "Filtering Status"
  );

  await page.click(tab("analytics"));
  await page.waitForSelector(".analytics-view .metric-card");
  assertAligned(
    await rowTops(page, ".analytics-view .metric-card", {
      label: ".metric-label",
      value: ".metric-value",
    }),
    "Filtering Analytics"
  );
});

test("a zero or absent pipeline count is neutral", async (t) => {
  const page = await openPage(t, "home", "empty");
  await page.waitForSelector(".home-dashboard:not(.loading)");
  const counts = await page.$$eval("#tokensPassed, #tokensRejected", (els) =>
    els.map((el) => ({ id: el.id, text: el.textContent.trim(), classes: el.className }))
  );
  assert.equal(counts.length, 2);
  for (const count of counts) {
    assert.match(count.text, /^(—|0)$/, `${count.id} shows an absent or zero count`);
    assert.equal(count.classes, "", `${count.id} carries no tone`);
  }
});

test("no source draws a decorative stat card or a second card owner", async () => {
  const found = [];
  const files = (await walk(SCRIPTS_ROOT)).filter((path) => path.endsWith(".js"));
  const pages = new URL("../../src/webserver/templates/pages/", import.meta.url);
  for (const name of ["trader.html", "assistant.html", "home.html"]) {
    files.push(new URL(name, pages).pathname);
  }
  for (const file of files) {
    const source = readFileSync(file, "utf8");
    for (const match of source.matchAll(/metric-card-primary|kpi-card|data-accent="primary"/g)) {
      const line = source.slice(0, match.index).split("\n").length;
      found.push(`${file}:${line} ${match[0]}`);
    }
  }
  assert.deepEqual(found, []);
});
