// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a DataTable column's alignment comes from its declared value `type` and
 * nowhere else. The table stamps the type as `data-type` on the header and every
 * cell, so header and values align together from one declaration.
 *
 * - A column whose renderer formats a number (an owner formatter or a page cell
 *   helper built on one) declares a numeric `type`. A composite cell that only
 *   contains numbers escapes with `// column-type-ok: <reason>` inside the column.
 * - No column carries an alignment class or an `align` key, and no stylesheet
 *   aligns a cell by its column id.
 * - The numeric types the table stamps are exactly the ones the stylesheet aligns.
 * - A dollar column keeps one decimal count: its cells render compact amounts at
 *   fixed fraction digits ("$9.90M" beside "$12.48M"), never the trimmed
 *   free-standing form ("$9.9M").
 * - A SOL column names its unit once, in the header ("P&L (SOL)"), and its cells
 *   carry the digits alone.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";

import {
  SCRIPTS_ROOT,
  STYLES_ROOT,
  REPO_ROOT,
  columnBlocks,
  repoPath,
  rulesIn,
  walk,
} from "../lib/dashboard_ui.mjs";

const NUMERIC_RENDER =
  /\b(?:formatSol|formatPriceSol|formatCurrencyUSD|formatPercent\w*|formatPnL|formatSignedSol|formatNumber|formatDuration|formatUptime|formatTimeSpan|formatFixed|priceCell|solCell|pnlCell|percentCell|usdCell)\b/;
const NUMERIC_TYPE = /\btype:\s*"(?:number|price|percent|sol|currency)"/;
const ESCAPE = /\/\/\s*column-type-ok:\s*\S/;

const scripts = (await walk(SCRIPTS_ROOT)).filter((file) => file.endsWith(".js"));
const consumers = scripts.filter((file) => {
  const source = readFileSync(file, "utf8");
  return /\bnew DataTable\(|\bbuildColumns\b/.test(source) && !file.endsWith("ui/data_table.js");
});

test("numeric DataTable columns declare a value type", () => {
  const missing = [];
  for (const file of consumers) {
    for (const block of columnBlocks(readFileSync(file, "utf8"))) {
      if (
        NUMERIC_RENDER.test(block.text) &&
        !NUMERIC_TYPE.test(block.text) &&
        !ESCAPE.test(block.text)
      )
        missing.push(`${repoPath(file)}:${block.line} ${block.id}`);
    }
  }
  assert.deepEqual(missing, [], "numeric columns without a numeric `type`");
});

test("a dollar column keeps fixed compact fraction digits", () => {
  const trimmed = [];
  for (const file of consumers) {
    for (const block of columnBlocks(readFileSync(file, "utf8"))) {
      const dollars =
        /\bformatCurrencyUSD\(/.test(block.text) ||
        /\bformatCompactNumber\([^)]*usd:\s*true/.test(block.text);
      if (dollars && !/\btrim:\s*false\b/.test(block.text))
        trimmed.push(`${repoPath(file)}:${block.line} ${block.id}`);
    }
  }
  assert.deepEqual(trimmed, [], "dollar columns that trim compact fraction digits");
  const usdCell = /export function usdCell\([^)]*\) \{([\s\S]*?)\n\}/.exec(
    readFileSync(`${SCRIPTS_ROOT}/pages/tokens/formatters.js`, "utf8")
  );
  assert.match(
    usdCell?.[1] ?? "",
    /formatCurrencyUSD\([^)]*trim:\s*false/,
    "usdCell keeps fixed fraction digits"
  );
});

/** Every English catalog message value by id. */
function englishMessages() {
  const dir = `${REPO_ROOT}/locales/en`;
  const messages = new Map();
  for (const name of readdirSync(dir).filter((file) => file.endsWith(".ftl"))) {
    for (const match of readFileSync(`${dir}/${name}`, "utf8").matchAll(/^([a-z][\w-]*) = (.*)$/gm))
      messages.set(match[1], match[2]);
  }
  return messages;
}

// A SOL formatter call that leaves the unit off: `suffix: ""` or `unit: false`.
const SOL_CALL = /\b(formatSol|formatPnL|formatSignedSol|signedSol|sol)\(([^\n]*)/g;
const BARE = /suffix:\s*""|unit:\s*false/;

test("a SOL column names its unit in the header and leaves the cells bare", () => {
  const messages = englishMessages();
  const found = [];
  const cellHelpers = new Map();
  for (const file of consumers) {
    const source = readFileSync(file, "utf8");
    for (const helper of source.matchAll(/const (solCell|pnlCell) = \([^)]*\) => ([^\n]*)/g))
      cellHelpers.set(`${repoPath(file)} ${helper[1]}`, helper[2]);
    for (const block of columnBlocks(source)) {
      if (!/\btype:\s*"sol"/.test(block.text)) continue;
      const where = `${repoPath(file)}:${block.line} ${block.id}`;
      const key = /label:\s*I18n\.t\("([^"]+)"\)/.exec(block.text)?.[1];
      if (!key || !/\(\{ -sol \}\)$/.test(messages.get(key) ?? ""))
        found.push(`${where}: header ${key ?? "(dynamic)"} does not end in "({ -sol })"`);
      if (/\bwithSolUnit\(/.test(block.text)) found.push(`${where}: withSolUnit in a cell`);
      for (const call of block.text.matchAll(SOL_CALL)) {
        if (!BARE.test(call[2])) found.push(`${where}: ${call[1]}() keeps the unit`);
      }
    }
  }
  for (const [helper, body] of cellHelpers) {
    if (!BARE.test(body)) found.push(`${helper} keeps the unit`);
  }
  assert.ok(cellHelpers.size > 0, "page SOL cell helpers not found");
  assert.deepEqual(found, []);
});

test("no DataTable column aligns itself outside its type", () => {
  const found = [];
  for (const file of consumers) {
    for (const block of columnBlocks(readFileSync(file, "utf8"))) {
      if (/\balign:|dt-cell-numeric/.test(block.text))
        found.push(`${repoPath(file)}:${block.line} ${block.id}`);
    }
  }
  assert.deepEqual(found, [], "columns carrying their own alignment");
});

test("no stylesheet aligns a table cell by column id", async () => {
  const found = [];
  for (const file of (await walk(STYLES_ROOT)).filter((path) => path.endsWith(".css"))) {
    const css = readFileSync(file, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    for (const rule of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      if (/(?:td|th)\[data-column-id/.test(rule[1]) && /\btext-align\s*:/.test(rule[2]))
        found.push(`${repoPath(file)}: ${rule[1].trim().split("\n")[0]}`);
    }
  }
  assert.deepEqual(found, []);
});

test("the stylesheet aligns exactly the numeric types the table stamps", () => {
  const table = readFileSync(`${SCRIPTS_ROOT}/ui/data_table.js`, "utf8");
  const declared = /NUMERIC_COLUMN_TYPES = new Set\(\[([^\]]*)\]\)/.exec(table)?.[1];
  assert.ok(declared, "NUMERIC_COLUMN_TYPES not found in ui/data_table.js");
  const types = [...declared.matchAll(/"(\w+)"/g)].map((match) => match[1]).sort();
  const css = readFileSync(`${STYLES_ROOT}/ui/data_table/column_types.css`, "utf8");
  for (const cell of ["th", "td"]) {
    const block = new RegExp(
      `\\.data-table ${cell}:is\\(([^)]*)\\)\\s*\\{[^}]*text-align: end`
    ).exec(css)?.[1];
    assert.ok(block, `no end-aligned ${cell} rule for value types`);
    const styled = [...block.matchAll(/data-type="(\w+)"/g)].map((match) => match[1]).sort();
    assert.deepEqual(styled, types, `${cell} alignment covers every numeric type`);
  }
});

test("the sort indicator sits beside its label on the label's side", () => {
  const rules = rulesIn(readFileSync(`${STYLES_ROOT}/ui/data_table/core.css`, "utf8"));
  const body = (selector) => rules.find((rule) => rule.selector === selector)?.body ?? "";
  // The label takes its own width, so the indicator follows it instead of the cell edge.
  assert.match(body(".dt-header-content"), /justify-content:\s*flex-start/);
  assert.doesNotMatch(body(".dt-header-label"), /flex:\s*1\b/);

  const numeric = rulesIn(readFileSync(`${STYLES_ROOT}/ui/data_table/column_types.css`, "utf8"));
  const endAligned = (suffix) =>
    numeric.find(
      (rule) => rule.selector.startsWith(".data-table th:is(") && rule.selector.endsWith(suffix)
    );
  assert.match(endAligned(".dt-header-content")?.body ?? "", /justify-content:\s*flex-end/);
  assert.match(endAligned(".dt-sort-icon")?.body ?? "", /order:\s*-1/);
});

test("a sorted header is marked by an accent label and a small arrow only", async () => {
  const offenders = [];
  for (const file of (await walk(STYLES_ROOT)).filter((path) => path.endsWith(".css"))) {
    for (const rule of rulesIn(readFileSync(file, "utf8"))) {
      if (!/th\.sorted\b/.test(rule.selector) || /\.dt-sort-icon/.test(rule.selector)) continue;
      if (/\b(?:background|border|box-shadow|text-decoration)[\w-]*\s*:/.test(rule.body))
        offenders.push(`${repoPath(file)}: ${rule.selector}`);
    }
  }
  assert.deepEqual(offenders, [], "sorted headers carry no tint, edge or underline");
  const table = readFileSync(`${SCRIPTS_ROOT}/ui/data_table.js`, "utf8");
  assert.doesNotMatch(table, /[▲▼]/, "the sort arrow is the small icon glyph");
  assert.match(table, /function sortArrow\([^)]*\)[\s\S]*?icon-arrow-up[\s\S]*?icon-arrow-down/);
});
