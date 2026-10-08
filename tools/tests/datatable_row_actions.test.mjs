// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: DataTable row buttons come from the table's `type: "actions"` column,
 * which draws icon buttons named by their tooltips at one fixed size, so an
 * actions column fits its width and every table's row actions look alike.
 *
 * A column whose renderer writes its own `<button>` markup draws labelled
 * buttons that wrap, overflow the table and miss the shared skin. The
 * allowlist holds the columns not yet moved to the actions column; it only
 * shrinks.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { SCRIPTS_ROOT, columnBlocks, repoPath, walk } from "../lib/dashboard_ui.mjs";

const RAW_BUTTON_ALLOWLIST = new Set([
  "src/webserver/templates/scripts/pages/tokens/ohlcv.js mint",
]);

const columns = [];
for (const file of (await walk(SCRIPTS_ROOT)).filter((path) => path.endsWith(".js"))) {
  const source = readFileSync(file, "utf8");
  if (!/\bnew DataTable\(|\bbuildColumns\b/.test(source) || file.endsWith("ui/data_table.js"))
    continue;
  for (const block of columnBlocks(source)) columns.push({ file: repoPath(file), ...block });
}

const rawButtonColumns = () =>
  columns.filter((column) => /<button\b/.test(column.text)).map(({ file, id }) => `${file} ${id}`);

test("DataTable columns draw row buttons only through the actions column", () => {
  assert.deepEqual(
    rawButtonColumns().filter((key) => !RAW_BUTTON_ALLOWLIST.has(key)),
    []
  );
});

test("the raw-button allowlist names only columns that still need it", () => {
  const present = new Set(rawButtonColumns());
  for (const key of RAW_BUTTON_ALLOWLIST) assert.ok(present.has(key), `${key} no longer needs it`);
});
