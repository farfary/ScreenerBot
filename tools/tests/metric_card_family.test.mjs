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
 *   (Home strip, Auto Trader Stats, Filtering Status and Analytics, the Copy Trading
 *   figures and task rows).
 * - Filtering Analytics uses the shared `.metric-card`.
 * - Only a state class tints a stat card border; peers otherwise share one border, and
 *   no Auto Trader Stats card carries a state.
 * - A zero or absent pipeline count carries no tone.
 * - No stat card carries a glyph, and only `ui/metric_card.js` builds stat card markup
 *   in a script.
 * - Only a signed amount is toned, by its sign; a ratio (win rate, profit factor) or a
 *   magnitude (max drawdown, fees) is not.
 * - A flat sheet grid's column count divides its cell count, so no empty track shows
 *   the hairline colour (the Copy task sheets and the Auto Trader Stats figures).
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { PAGES_ROOT, SCRIPTS_ROOT, walk } from "../lib/dashboard_ui.mjs";
import { metricCard } from "../../src/webserver/templates/scripts/ui/metric_card.js";

const tab = (id) => `#subTabsContainer [data-tab-id="${id}"]`;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const shell = await loadIndex("shell");

async function openPage(t, name, variant = "populated", width = 1200) {
  const context = await browser.newContext({ viewport: { width, height: 900 } });
  const api = createApiHandler([await loadIndex(name), shell], variant);
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/${name}`);
  await page.waitForSelector(PAGE_READY);
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

test("Auto Trader Stats cards align and share one border", async (t) => {
  const page = await openPage(t, "trader");
  await page.click(tab("stats"));
  await page.waitForFunction(() => document.querySelector("#net-pnl")?.textContent !== "—");
  const rows = await rowTops(page, "#stats-tab .metric-grid .metric-card", {
    label: ".metric-label",
    value: ".metric-value",
  });
  assertAligned(rows, "Auto Trader Stats");

  // No Stats card carries a state: a drawdown or a fee is a figure, not an alarm.
  const borders = await page.$$eval("#stats-tab .metric-grid .metric-card", (cards) =>
    cards
      .filter((card) => card.offsetParent !== null)
      .map((card) => getComputedStyle(card).borderTopColor)
  );
  assert.equal(borders.length, 8, "the Stats sheet shows its eight cards");
  assert.equal(new Set(borders).size, 1, "every Stats card shares one border colour");
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

/** The Copy Trading page with the first task's sheet open on `sheetTab`. */
async function openCopyTask(t, sheetTab = "overview", width = 1200) {
  const page = await openPage(t, "copy", "populated", width);
  await page.click("#copy-list-rows .copy-row");
  await page.click(`#copy-tab-${sheetTab}`);
  await page.waitForSelector(`#copy-ws-panel[aria-labelledby="copy-tab-${sheetTab}"] .metric-card`);
  await page.waitForSelector("#copy-figures .metric-card");
  return page;
}

/** Label text and value tone of every stat card under `root`. */
async function cardTones(page, root) {
  return page.$$eval(`${root} .metric-card`, (cards) =>
    cards.map((card) => {
      const value = card.querySelector(".metric-value");
      return {
        label: card.querySelector(".metric-label").textContent.trim(),
        positive: value.classList.contains("positive"),
        negative: value.classList.contains("negative"),
      };
    })
  );
}

const toned = (card) => card.positive || card.negative;

test("Copy page and task rows align", async (t) => {
  const page = await openCopyTask(t);
  const parts = { label: ".metric-label", value: ".metric-value" };
  assertAligned(await rowTops(page, "#copy-figures .metric-card", parts), "Copy figures");
  assertAligned(await rowTops(page, "#copy-ws-panel .metric-card", parts), "Copy task results");
});

for (const width of [1440, 1200, 820]) {
  test(`a sheet grid shows no empty track at ${width}px`, async (t) => {
    const page = await openCopyTask(t, "overview", width);
    const grids = [];
    for (const sheetTab of ["overview", "execution"]) {
      await page.click(`#copy-tab-${sheetTab}`);
      await page.waitForSelector(
        `#copy-ws-panel[aria-labelledby="copy-tab-${sheetTab}"] .metric-card`
      );
      grids.push(
        ...(await page.$$eval('.metric-grid[data-surface="sheet"]', (nodes) =>
          nodes
            .filter((node) => node.offsetParent !== null)
            .map((node) => ({
              columns: getComputedStyle(node).gridTemplateColumns.split(" ").filter(Boolean).length,
              cells: node.children.length,
            }))
        ))
      );
    }
    assert.ok(grids.length >= 2, "the Overview and Execution tabs render sheet grids");

    const trader = await openPage(t, "trader", "populated", width);
    await trader.click(tab("stats"));
    await trader.waitForFunction(() => document.querySelector("#net-pnl")?.textContent !== "—");
    grids.push(
      await trader.$eval("#stats-figures", (node) => ({
        columns: getComputedStyle(node).gridTemplateColumns.split(" ").filter(Boolean).length,
        cells: node.children.length,
      }))
    );
    for (const grid of grids) {
      assert.equal(
        grid.cells % grid.columns,
        0,
        `${grid.cells} cells in ${grid.columns} columns leave an empty track`
      );
    }
  });
}

test("a ratio never carries a sign tone and a signed SOL figure does", async (t) => {
  const trader = await openPage(t, "trader");
  await trader.click(tab("stats"));
  await trader.waitForFunction(() => document.querySelector("#net-pnl")?.textContent !== "—");
  const tones = await trader.$$eval(
    "#net-pnl, #win-rate, #profit-factor, #max-drawdown, #fees-paid",
    (els) => Object.fromEntries(els.map((el) => [el.id, el.className]))
  );
  assert.match(tones["net-pnl"], /\bpositive\b/, "a gain in Net P&L is toned");
  // A drawdown and the fees are unsigned magnitudes, never toned.
  for (const id of ["win-rate", "profit-factor", "max-drawdown", "fees-paid"]) {
    assert.doesNotMatch(tones[id], /\b(positive|negative)\b/, `${id} stays neutral`);
  }

  const copy = await openCopyTask(t);
  const cards = [
    ...(await cardTones(copy, "#copy-figures")),
    ...(await cardTones(copy, "#copy-ws-panel")),
  ];
  const ratios = cards.filter((card) => /^(Win rate|Profit factor)$/.test(card.label));
  assert.ok(ratios.length >= 3, "the Copy figures and results show their ratios");
  assert.deepEqual(ratios.filter(toned), [], "a ratio stays neutral");
  const realized = cards.filter((card) => card.label === "Realized P&L");
  assert.ok(realized.length >= 2, "the Copy figures and results show Realized P&L");
  for (const card of realized) assert.ok(card.positive, "a realized gain is toned");
});

test("no stat card carries a glyph", async (t) => {
  const found = [];
  const scripts = (await walk(SCRIPTS_ROOT)).filter((path) => path.endsWith(".js"));
  const pages = (await walk(PAGES_ROOT)).filter((path) => path.endsWith(".html"));
  for (const file of [...scripts, ...pages]) {
    const source = readFileSync(file, "utf8");
    const at = (index) => `${file}:${source.slice(0, index).split("\n").length}`;
    for (const match of source.matchAll(
      /metric-icon|metric-card--with-icon|metric-content|data-accent/g
    )) {
      found.push(`${at(match.index)} ${match[0]}`);
    }
    if (file.endsWith(".js") && !file.endsWith("/ui/metric_card.js")) {
      for (const match of source.matchAll(/class="metric-card\b/g)) {
        found.push(`${at(match.index)} stat card markup outside ui/metric_card.js`);
      }
    }
    if (file.endsWith(".html")) {
      for (const match of source.matchAll(/<div class="metric-card\b[^>]*>/g)) {
        // The card's own block: up to the </div> that closes it.
        let depth = 1;
        const tags = /<(\/?)div\b[^>]*>|<i\b/g;
        tags.lastIndex = match.index + match[0].length;
        let tag;
        while (depth > 0 && (tag = tags.exec(source))) {
          if (tag[0] === "<i") found.push(`${at(tag.index)} glyph inside a stat card`);
          else depth += tag[1] ? -1 : 1;
        }
      }
    }
  }
  assert.deepEqual(found, []);

  const rendered = [];
  const trader = await openPage(t, "trader");
  await trader.click(tab("stats"));
  rendered.push(...(await trader.$$eval(".metric-card i", (els) => els.map(() => "trader"))));
  const filtering = await openPage(t, "filtering");
  for (const view of ["status", "analytics"]) {
    await filtering.click(tab(view));
    await filtering.waitForSelector(`.${view}-view .metric-card`);
    rendered.push(...(await filtering.$$eval(".metric-card i", (els) => els.map(() => view))));
  }
  const copy = await openCopyTask(t);
  rendered.push(...(await copy.$$eval(".metric-card i", (els) => els.map(() => "copy"))));
  assert.deepEqual(rendered, [], "no rendered stat card holds a glyph");
});

test("the stat card markup takes a colour only from a sign tone", () => {
  const esc = (text) => String(text).replace(/&/g, "&amp;").replace(/</g, "&lt;");
  const card = (fields) => metricCard({ label: "Figure", value: "1", ...fields }, esc);
  assert.equal(
    card({ value: "+0.0412 SOL", detail: "12 closed rounds", tone: "positive" }),
    '<div class="metric-card"><div class="metric-label">Figure</div><div class="metric-value positive">+0.0412 SOL</div><div class="metric-detail">12 closed rounds</div></div>'
  );
  assert.match(card({ tone: "negative" }), /class="metric-value negative"/);
  for (const tone of ["neutral", null, "success", "is-positive"]) {
    assert.match(card({ tone }), /class="metric-value">/, `${tone} adds no class`);
  }
  assert.match(
    card({ value: "2.54 / 8 SOL", fit: true }),
    /class="metric-value metric-value-fit" style="--metric-value-chars:12">/
  );
  assert.match(card({ value: null }), /class="metric-value">—</, "an absent figure prints a dash");
  assert.match(card({ label: "<b>" }), /&lt;b>/, "the label is escaped");
});
