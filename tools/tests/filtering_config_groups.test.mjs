/**
 * Tests for the filtering page's parameter grouping (`pages/filtering/config_metadata.js`).
 *
 * This module decides what the DexScreener / GeckoTerminal / RugCheck / On-Chain
 * sub-tabs actually show: which fields become one range row, which group they
 * land in, in what order, and which switch masters them. `/api/config/metadata`
 * is a BTreeMap, so the payload it works from is always in alphabetical key
 * order — the presentation order asserted here is entirely this module's doing.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import {
  buildConfigGroups,
  getSourceMasterField,
  SETTINGS_TABS,
} from "../../src/webserver/templates/scripts/pages/filtering/config_metadata.js";

const number = (label, extras = {}) => ({ type: "number", label, ...extras });
const boolean = (label, extras = {}) => ({ type: "boolean", label, ...extras });

/** A source object as `/api/config/metadata` delivers it: alphabetical children. */
function source(children) {
  return { type: "object", children };
}

/** The shape of the real DexScreener payload, trimmed to the interesting parts. */
function dexscreenerMetadata() {
  return {
    dexscreener: source({
      enabled: boolean("Enable DexScreener Filters", { category: "Source Control" }),
      fdv_enabled: boolean("Enable FDV Checks", { category: "FDV" }),
      max_fdv_usd: number("Max FDV", { category: "FDV", unit: "USD", impact: "medium" }),
      max_price_change_1h: number("Max Price Change 1h", { category: "Price Change", unit: "%" }),
      max_price_change_24h: number("Max Price Change 24h", { category: "Price Change", unit: "%" }),
      max_price_change_5m: number("Max Price Change 5m", { category: "Price Change", unit: "%" }),
      max_price_change_6h: number("Max Price Change 6h", { category: "Price Change", unit: "%" }),
      min_fdv_usd: number("Min FDV", { category: "FDV", unit: "USD", impact: "critical" }),
      min_price_change_1h: number("Min Price Change 1h", { category: "Price Change", unit: "%" }),
      min_price_change_24h: number("Min Price Change 24h", { category: "Price Change", unit: "%" }),
      min_price_change_5m: number("Min Price Change 5m", { category: "Price Change", unit: "%" }),
      min_price_change_6h: number("Min Price Change 6h", { category: "Price Change", unit: "%" }),
      min_transactions_1h: number("Min TX (1h)", { category: "Activity", unit: "txs" }),
      min_transactions_5min: number("Min TX (5min)", { category: "Activity", unit: "txs" }),
    }),
  };
}

function groupOf(groups, source_, title) {
  const group = groups.find((entry) => entry.source === source_ && entry.title === title);
  assert.ok(group, `expected a ${source_} group titled ${title}`);
  return group;
}

test("a min/max pair collapses into one range row", () => {
  const fdv = groupOf(buildConfigGroups(dexscreenerMetadata()), "dexscreener", "FDV");

  assert.equal(fdv.rows.length, 1, "two bounds are one parameter, not two rows");
  const [row] = fdv.rows;
  assert.equal(row.kind, "range");
  assert.equal(row.label, "FDV", "the shared subject, with Min/Max stripped");
  assert.deepEqual(
    row.fields.map((field) => field.key),
    ["min_fdv_usd", "max_fdv_usd"],
    "min first, whichever bound the payload listed first"
  );
  assert.equal(row.unit, "USD");
  assert.equal(row.impact, "critical", "a range takes the stronger impact of its two bounds");
});

test("range rows are ordered by timeframe, not by the payload's alphabetical keys", () => {
  const groups = buildConfigGroups(dexscreenerMetadata());
  const priceChange = groupOf(groups, "dexscreener", "Price Change");

  assert.deepEqual(
    priceChange.rows.map((row) => row.label),
    ["Price Change 5m", "Price Change 1h", "Price Change 6h", "Price Change 24h"]
  );
  assert.ok(
    priceChange.rows.every((row) => row.kind === "range"),
    "each timeframe is one row with both bounds"
  );

  // Unpaired fields keep their own labels and still sort by timeframe.
  assert.deepEqual(
    groupOf(groups, "dexscreener", "Activity").rows.map((row) => row.label),
    ["Min TX (5min)", "Min TX (1h)"]
  );
});

test("a group's own _enabled boolean becomes its master, not one of its rows", () => {
  const fdv = groupOf(buildConfigGroups(dexscreenerMetadata()), "dexscreener", "FDV");

  assert.equal(fdv.enableKey, "fdv_enabled");
  assert.equal(fdv.enableHint, undefined, "no hint declared in this fixture");
  assert.ok(
    fdv.rows.every((row) => row.fields.every((field) => field.key !== "fdv_enabled")),
    "the master switch is not repeated as a parameter row"
  );
});

test("a source's `enabled` field is its master switch and never a row", () => {
  const metadata = dexscreenerMetadata();
  const groups = buildConfigGroups(metadata);

  assert.equal(
    groups.some((group) => group.title === "Source Control"),
    false,
    "the source master is the sub-tab's control, so its category has no rows left"
  );
  assert.equal(getSourceMasterField(metadata, "dexscreener").label, "Enable DexScreener Filters");
  assert.equal(getSourceMasterField(metadata, "meta"), null, "Core has no source switch");
  assert.equal(getSourceMasterField(metadata, "onchain"), null, "absent source, absent switch");
});

test("the same category name under two sources stays two groups", () => {
  const groups = buildConfigGroups({
    dexscreener: source({
      liquidity_enabled: boolean("Enable Liquidity Checks", { category: "Liquidity" }),
      max_liquidity_usd: number("Max Liquidity", { category: "Liquidity", unit: "USD" }),
      min_liquidity_usd: number("Min Liquidity", { category: "Liquidity", unit: "USD" }),
    }),
    geckoterminal: source({
      max_liquidity_usd: number("Max Liquidity", { category: "Liquidity", unit: "USD" }),
      min_liquidity_usd: number("Min Liquidity", { category: "Liquidity", unit: "USD" }),
    }),
  });

  const liquidity = groups.filter((group) => group.title === "Liquidity");
  assert.equal(liquidity.length, 2, "keyed by source + category, so neither can overwrite the other");
  assert.deepEqual(
    liquidity.map((group) => group.id),
    ["dexscreener:Liquidity", "geckoterminal:Liquidity"]
  );
});

test("root-level fields belong to the Core sub-tab", () => {
  const groups = buildConfigGroups({
    age_enabled: boolean("Enable Age Check", { category: "Age" }),
    min_token_age_minutes: number("Min Token Age", { category: "Age", unit: "minutes" }),
    dexscreener: source({ enabled: boolean("Enable DexScreener Filters") }),
  });

  const age = groupOf(groups, "meta", "Age");
  assert.equal(age.enableKey, "age_enabled");
  assert.deepEqual(
    age.rows.map((row) => row.fields[0].key),
    ["min_token_age_minutes"],
    "an unpaired min stays a single row"
  );
  assert.ok(SETTINGS_TABS.includes("meta") && SETTINGS_TABS.includes("onchain"));
});

test("bounds of different subjects are never paired", () => {
  const groups = buildConfigGroups({
    rugcheck: source({
      max_risk_score: number("Max Risk Score", { category: "Risk" }),
      min_unique_holders: number("Min Unique Holders", { category: "Risk" }),
    }),
  });

  const risk = groupOf(groups, "rugcheck", "Risk");
  assert.equal(risk.rows.length, 2);
  assert.deepEqual(
    risk.rows.map((row) => [row.kind, row.label]),
    [
      ["field", "Max Risk Score"],
      ["field", "Min Unique Holders"],
    ],
    "different subjects keep their own labels and rows"
  );
});
