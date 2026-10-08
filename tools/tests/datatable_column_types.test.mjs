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
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { SCRIPTS_ROOT, STYLES_ROOT, columnBlocks, repoPath, walk } from "../lib/dashboard_ui.mjs";

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
