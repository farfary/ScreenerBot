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
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { STYLES_ROOT, rulesIn } from "../lib/dashboard_ui.mjs";

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
