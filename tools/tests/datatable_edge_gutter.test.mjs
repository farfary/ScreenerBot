// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the last DataTable column keeps an end gutter wider than the gap between
 * columns, in both the regular and the compact density. Without it an end-aligned
 * value at the end of a horizontally scrolled table sits flush against the table's
 * edge and reads as clipped.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { STYLES_ROOT } from "../lib/dashboard_ui.mjs";

const css = readFileSync(`${STYLES_ROOT}/ui/data_table/core.css`, "utf8").replace(
  /\/\*[\s\S]*?\*\//g,
  ""
);
const rules = [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((match) => ({
  selector: match[1].trim(),
  body: match[2],
  index: match.index,
}));

/** Inline padding of a `padding: <block> <inline>` shorthand, in px. */
const inlinePadding = (body) => Number(/\bpadding:\s*\d+px\s+(\d+)px/.exec(body)?.[1]);

test("the last column's end gutter is wider than every cell's inline padding", () => {
  const gutter = rules.find(({ selector }) => selector === ".data-table :is(th, td):last-child");
  assert.ok(gutter, "no last-column gutter rule in core.css");
  const end = Number(/padding-inline-end:\s*(\d+)px/.exec(gutter.body)?.[1]);
  for (const selector of [
    ".data-table td",
    ".data-table th",
    ".data-table.compact td",
    ".data-table.compact th",
  ]) {
    const rule = rules.find((candidate) => candidate.selector === selector);
    assert.ok(rule, `${selector} not found in core.css`);
    assert.ok(end > inlinePadding(rule.body), `${selector} padding reaches the gutter`);
    assert.ok(
      rule.index < gutter.index,
      `${selector} is declared after the gutter and overrides it`
    );
  }
});
