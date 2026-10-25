// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Add Condition catalog lists every condition at once, as buttons that say
 * what they read and whether the strategy can use them.
 *
 * The catalog once opened with every category folded behind a chevron and a bulk
 * Fold All control, its entries were `div`s no keyboard could reach, and position
 * conditions were offered to entry strategies, where they can never be true.
 *
 * - No script, markup or message carries a fold control or a stored fold state.
 * - A condition that does not apply to the strategy type is a disabled button naming
 *   the strategy type it is for; an applicable one is an enabled button.
 * - Every schema has a data tag, derived from its category and parameters.
 * - Search matches the localized name or description, and a category narrows the list.
 *
 * The strategies stylesheets also read only defined tokens: the exit kind once named
 * `--danger-color`, which no theme defines, so the Exit icon rendered in the text
 * colour while Entry was green. A condition glyph is bare (size and colour), never a
 * filled or gradient tile.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";

import {
  PAGES_ROOT,
  REPO_ROOT,
  SCRIPTS_ROOT,
  STYLES_ROOT,
  rulesIn,
  walk,
} from "../lib/dashboard_ui.mjs";
import { schemas } from "../lib/strategy_rules.mjs";
import {
  DATA_TAG_LABELS,
  catalogEntries,
  categoriesOf,
  createConditionCatalog,
  dataTagOf,
  filterEntries,
} from "../../src/webserver/templates/scripts/pages/strategies/condition_catalog.js";

const read = (path) => readFileSync(path, "utf8");
const catalogSource = read(`${SCRIPTS_ROOT}/pages/strategies/condition_catalog.js`);
const pageSource = read(`${SCRIPTS_ROOT}/pages/strategies.js`);
const markup = read(`${PAGES_ROOT}/strategies.html`);
const escapeHtml = (text) =>
  String(text).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;");

const catalog = createConditionCatalog({
  conditionSchemas: schemas,
  $: () => null,
  Utils: { escapeHtml },
  addTrackedListener: () => {},
});

test("the catalog has no fold controls and keeps no fold state", () => {
  for (const source of [catalogSource, pageSource, markup]) {
    assert.doesNotMatch(source, /[▶▼◀▲►◄]/);
    assert.doesNotMatch(source, /condition-category-states|category-toggle|toggle-all-categories/);
    assert.doesNotMatch(source, /collapsed|chevrons-(?:up|down)/);
  }
  assert.doesNotMatch(markup, /catalog-toggle-btn/);
  const locales = readdirSync(`${REPO_ROOT}/locales`, { withFileTypes: true }).filter((entry) =>
    entry.isDirectory()
  );
  assert.ok(locales.length >= 16);
  for (const { name: locale } of locales) {
    const ftl = read(`${REPO_ROOT}/locales/${locale}/strategies.ftl`);
    assert.doesNotMatch(ftl, /strategies-catalog-(?:fold|unfold)-all/, locale);
  }
});

test("a condition the strategy type cannot use is a disabled button naming its type", () => {
  const entry = (type, strategyType) =>
    catalogEntries(schemas, strategyType).find((candidate) => candidate.type === type);

  const exitOnly = catalog.renderEntry(entry("PositionHoldingTime", "ENTRY"));
  assert.match(exitOnly, /^\s*<button type="button" class="catalog-entry"[^>]*\sdisabled>/);
  assert.match(exitOnly, /Exit strategies only/);

  const forExit = catalog.renderEntry(entry("PositionHoldingTime", "EXIT"));
  assert.doesNotMatch(forExit, /\sdisabled/);
  assert.doesNotMatch(forExit, /Exit strategies only/);

  const everywhere = catalog.renderEntry(entry("Rsi", "ENTRY"));
  assert.match(everywhere, /<button type="button" class="catalog-entry" data-condition-type="Rsi">/);
  assert.match(everywhere, /catalog-entry-tag">Candles</);
});

test("every schema has a data tag derived from its category and parameters", () => {
  const tags = Object.fromEntries(
    Object.entries(schemas).map(([type, schema]) => [type, dataTagOf(schema)])
  );
  for (const [type, tag] of Object.entries(tags)) {
    assert.ok(Object.hasOwn(DATA_TAG_LABELS, tag), `${type} has no tag label`);
    assert.ok(I18n.has(DATA_TAG_LABELS[tag]), `${type}: ${DATA_TAG_LABELS[tag]} is not a message`);
  }
  assert.deepEqual(tags, {
    BuyPressure: "market",
    CandleSize: "candles",
    ConsecutiveCandles: "candles",
    DrawdownFromPeak: "position",
    LiquidityLevel: "pool",
    MaCross: "candles",
    PositionHoldingTime: "position",
    PositionProfit: "position",
    PriceBreakout: "candles",
    PriceChangePercent: "candles",
    PriceToMA: "candles",
    Rsi: "candles",
    VolumeSpike: "candles",
  });
});

test("search matches the localized name or description and a category narrows the list", () => {
  const entries = catalogEntries(schemas, "ENTRY");
  const types = (list) => list.map((entry) => entry.type).sort();

  assert.ok(types(filterEntries(entries, { query: "rsi" })).includes("Rsi"));
  assert.ok(types(filterEntries(entries, { query: "  BUY " })).includes("BuyPressure"));
  assert.equal(filterEntries(entries, { query: "no such condition" }).length, 0);
  assert.equal(filterEntries(entries).length, entries.length);

  const categories = categoriesOf(entries);
  assert.equal(categories.reduce((sum, { count }) => sum + count, 0), entries.length);
  const technical = categories.find(({ category }) => category === "Technical Indicators");
  assert.equal(filterEntries(entries, { category: "Technical Indicators" }).length, technical.count);
});

test("strategies stylesheets read only defined tokens", async () => {
  const defined = new Set();
  for (const file of (await walk(STYLES_ROOT)).filter((path) => path.endsWith(".css"))) {
    for (const match of read(file).matchAll(/(--[\w-]+)\s*:/g)) defined.add(match[1]);
  }
  const dir = `${STYLES_ROOT}/pages/strategies`;
  const found = [];
  for (const name of readdirSync(dir).filter((file) => file.endsWith(".css"))) {
    for (const match of read(`${dir}/${name}`).matchAll(/var\(\s*(--[\w-]+)\s*\)/g)) {
      if (!defined.has(match[1])) found.push(`${name}: ${match[1]}`);
    }
  }
  assert.deepEqual(found, []);
});

test("a condition glyph is a bare glyph", () => {
  for (const [file, glyph] of [
    ["condition_cards.css", /\.rule-glyph\b/],
    ["modals.css", /\.catalog-entry-glyph\b/],
  ]) {
    const rules = rulesIn(read(`${STYLES_ROOT}/pages/strategies/${file}`)).filter(({ selector }) =>
      glyph.test(selector)
    );
    assert.ok(rules.length > 0, `${file} styles its glyph`);
    for (const { selector, body } of rules) {
      assert.doesNotMatch(body, /(?:^|;)\s*(?:background|border|box-shadow|filter)\b/, selector);
    }
  }
});
