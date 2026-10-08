// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a DataTable loading, empty or error state is the whole table body.
 *
 * Rendered as an ordinary row, the state took its content height and drew the row
 * rule under itself, so an empty table ended in a full-width line halfway down the
 * page with blank space beneath it.
 *
 * - The table that holds a state cell fills its scroll area.
 * - Neither the state cell nor its row draws a bottom rule.
 * - Every table names its own empty state (`emptyTitle` and `emptyMessage`): the
 *   generic "No data" / "No data to display" pair said neither what was missing nor why.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { SCRIPTS_ROOT, STYLES_ROOT, repoPath, rulesIn, walk } from "../lib/dashboard_ui.mjs";

const rules = rulesIn(readFileSync(`${STYLES_ROOT}/ui/data_table/core.css`, "utf8"));
const declaration = (subject, property) =>
  rules
    .filter(({ selector }) => selector === subject)
    .map(({ body }) => new RegExp(`(?:^|;)\\s*${property}\\s*:\\s*([^;]+)`).exec(body)?.[1]?.trim())
    .find(Boolean);

test("a table showing a state fills its scroll area", () => {
  assert.equal(declaration(".data-table:has(> tbody > tr > td.dt-state-cell)", "height"), "100%");
});

test("a state draws no row rule beneath it", () => {
  assert.equal(declaration(".data-table td.dt-state-cell", "border-bottom"), "0");
  assert.equal(declaration(".data-table tbody tr:has(> td.dt-state-cell)", "border-bottom"), "0");
});

/** The object literal passed to each `new DataTable({ ... })` outside comments. */
function dataTableOptions(source) {
  const blocks = [];
  for (const match of source.matchAll(/new DataTable\(\{/g)) {
    const lineStart = source.lastIndexOf("\n", match.index) + 1;
    if (/^\s*(?:\*|\/\/)/.test(source.slice(lineStart, match.index))) continue;
    let depth = 0;
    let end = match.index + match[0].length - 1;
    for (; end < source.length; end += 1) {
      if (source[end] === "{") depth += 1;
      else if (source[end] === "}" && --depth === 0) break;
    }
    blocks.push(source.slice(match.index, end + 1));
  }
  return blocks;
}

test("every DataTable names its own empty state", async () => {
  const found = [];
  let tables = 0;
  for (const file of (await walk(SCRIPTS_ROOT)).filter((path) => path.endsWith(".js"))) {
    for (const options of dataTableOptions(readFileSync(file, "utf8"))) {
      tables += 1;
      for (const key of ["emptyTitle", "emptyMessage"]) {
        if (!new RegExp(`\\b${key}:`).test(options)) found.push(`${repoPath(file)}: ${key}`);
      }
    }
  }
  assert.ok(tables >= 10, `expected the dashboard tables, found ${tables}`);
  assert.deepEqual(found, []);
});
