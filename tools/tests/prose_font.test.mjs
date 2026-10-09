// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: words are set in the UI face (`--font-sans`); only numbers and machine
 * values use the tabular data faces (`--font-data`, `--font-mono`).
 *
 * The shared form controls once set every text input, textarea and select in the
 * data face, so a strategy name, a search placeholder and every select option read
 * as code. The fix lives in the owners every surface inherits from, so this guard
 * pins those owners rather than each page.
 *
 * - `components/form_controls.css`: the base text input, textarea and select rules
 *   use the UI face, and a data face only reaches number, date and time inputs and
 *   inputs marked as left-to-right islands (`dir="ltr"`: addresses, mints, keys).
 *   Their placeholders are words and stay in the UI face.
 * - A number field's unit sits inside the field's box, in the value's data face.
 * - The shared prose surfaces (select trigger, menu and search, table search and
 *   menus, row action items, every table cell, the strategy condition summary, the
 *   position header's sub-lines, the Copy Trading segmented choices) declare the UI face. A table cell reaches a data
 *   face only through its numeric column type or a machine-value class.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { STYLES_ROOT, rulesIn } from "../lib/dashboard_ui.mjs";

const fontOf = (body) => /(?:^|;)\s*font-family\s*:\s*([^;]+)/.exec(body)?.[1]?.trim();
const rulesOf = (path) => rulesIn(readFileSync(`${STYLES_ROOT}/${path}`, "utf8"));

const DATA_FACE = /var\(--font-(?:data|mono)\)/;
const DATA_INPUT =
  /^(?:input\[type="(?:number|date|time|datetime-local)"\]|input\[dir="ltr"\]|textarea\[dir="ltr"\])$/;

test("base form controls set typed words in the UI face", () => {
  const rules = rulesOf("components/form_controls.css");
  for (const subject of ['input[type="text"]', 'input[type="search"]', "textarea", "select"]) {
    const base = rules.find(({ selector, body }) => selector === subject && fontOf(body));
    assert.ok(base, `no font rule for ${subject}`);
    assert.equal(fontOf(base.body), "var(--font-sans)", `${subject} is not in the UI face`);
  }
});

test("a data face reaches only number, date, time and left-to-right inputs", () => {
  const found = rulesOf("components/form_controls.css")
    .filter(({ selector, body }) => /^(?:input|textarea|select)\b/.test(selector))
    .filter(({ body }) => DATA_FACE.test(fontOf(body) ?? ""))
    .map(({ selector }) => selector)
    .filter((selector) => !DATA_INPUT.test(selector));
  assert.deepEqual(found, []);
});

test("a left-to-right field's placeholder is words, in the UI face", () => {
  const rules = rulesOf("components/form_controls.css");
  const faces = ['input[dir="ltr"]::placeholder', 'textarea[dir="ltr"]::placeholder'].map(
    (subject) => [
      subject,
      rules
        .filter(({ selector }) => selector === subject)
        .map(({ body }) => fontOf(body))
        .find(Boolean) ?? "none",
    ]
  );
  assert.deepEqual(
    faces.filter(([, font]) => font !== "var(--font-sans)"),
    []
  );
});

test("a number-field unit is in the data face, inside the field", () => {
  const rules = rulesOf("components/form_controls.css");
  const faces = rules
    .filter(({ selector, body }) => /\.input-unit\b/.test(selector) && fontOf(body))
    .map(({ selector, body }) => [selector, fontOf(body)]);
  assert.deepEqual(faces, [[".input-unit", "var(--font-data)"]]);
});

test("shared prose surfaces declare the UI face", () => {
  const owners = {
    "ui/custom_select.css": [".custom-select", ".cs-dropdown", ".cs-search-input"],
    "ui/table_toolbar.css": [".dt-search-input", ".table-toolbar-menu"],
    "ui/data_table/column_types.css": [".dt-actions-dropdown-item"],
    "ui/data_table/core.css": [".data-table td"],
    "pages/strategies/condition_cards.css": [".summary-content"],
    "ui/position_details/header.css": [".position-details-dialog .header-metric-sub"],
    "pages/copy.css": [".copy-seg-btn"],
  };
  const found = [];
  for (const [path, selectors] of Object.entries(owners)) {
    const rules = rulesOf(path);
    for (const subject of selectors) {
      const font = rules
        .filter(({ selector }) => selector === subject)
        .map(({ body }) => fontOf(body))
        .find(Boolean);
      if (font !== "var(--font-sans)") found.push(`${path}: ${subject} -> ${font ?? "none"}`);
    }
  }
  assert.deepEqual(found, []);
});
