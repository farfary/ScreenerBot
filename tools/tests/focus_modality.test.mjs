// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: one keyboard-only focus ring, owned by `foundation.css`.
 *
 * `foundation.css` removes every native outline and draws the one ring, a thin
 * `--focus-ring-color` outline, only under `html[data-input="keyboard"]` (outset by
 * default, inset inside clipping strips and lists). A component that draws its own ring
 * either fights that owner or shows to pointer users, so:
 *
 * - no stylesheet but `foundation.css` has a `:focus-visible` rule that sets `outline`,
 *   `box-shadow` or a `border`, keyboard-gated or not;
 * - no stylesheet but `foundation.css` sets a visible `outline` on `:focus` (the global
 *   `outline: none !important` makes it dead code that reads like a ring);
 * - no stylesheet but `foundation.css` draws a `box-shadow` or `border` ring on `:focus`
 *   of a control that is not a text-entry field (that ring shows to pointer users too);
 * - the owner's ring is gated on the keyboard modality and uses the ring tokens.
 *
 * Text-entry fields keep their editing state on `:focus` in `components/form_controls.css`;
 * that is a field state shown in every modality, not a focus ring.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { loadStylesheets } from "../lib/dashboard_ui.mjs";

const OWNER = "foundation.css";
const DRAWS_FOCUS = /(?:^|;)\s*(?:outline|box-shadow|border(?:-[\w-]+)?)\s*:/;
const VISIBLE_OUTLINE = /(?:^|;)\s*outline\s*:\s*(?!none\b)[^\s;]/;
const PLAIN_FOCUS = /:focus(?![-\w])/;
const DRAWS_RING = /(?:^|;)\s*(?:box-shadow|border(?:-[\w-]+)?)\s*:/;
/** Text-entry fields and selects, whose `:focus` editing state is shown in every modality. */
const TEXT_ENTRY =
  /(?:\binput\b|\btextarea\b|\bselect\b|[-_]input\b|[-_]select\b|[-_]search\b|\bdt-filter\b|\.form-control\b)/;

async function componentRules(matches) {
  const found = [];
  for (const { path, rules } of await loadStylesheets()) {
    if (path === OWNER) continue;
    for (const rule of rules) {
      if (matches(rule)) found.push(`${path}:${rule.line} ${rule.selector}`);
    }
  }
  return found;
}

test("no component stylesheet draws its own focus-visible ring", async () => {
  const offenders = await componentRules(
    ({ selector, body }) => selector.includes(":focus-visible") && DRAWS_FOCUS.test(body)
  );
  assert.deepEqual(offenders, [], "the ring belongs to foundation.css");
});

test("no component stylesheet sets an outline on plain focus", async () => {
  const offenders = await componentRules(
    ({ selector, body }) => PLAIN_FOCUS.test(selector) && VISIBLE_OUTLINE.test(body)
  );
  assert.deepEqual(offenders, [], "the ring belongs to foundation.css");
});

test("no component stylesheet draws a ring on plain focus of a non-field control", async () => {
  const offenders = await componentRules(
    ({ selector, body }) =>
      PLAIN_FOCUS.test(selector) && !TEXT_ENTRY.test(selector) && DRAWS_RING.test(body)
  );
  assert.deepEqual(offenders, [], "the ring belongs to foundation.css");
});

test("the owner's ring is drawn only in the keyboard modality, with the ring tokens", async () => {
  const sheets = await loadStylesheets();
  const { rules } = sheets.find(({ path }) => path === OWNER);
  const rings = rules.filter(
    ({ selector, body }) => selector.includes(":focus-visible") && VISIBLE_OUTLINE.test(body)
  );
  assert.ok(rings.length > 0, "foundation.css draws the ring");
  for (const { selector, body, line } of rings) {
    assert.match(selector, /^html\[data-input="keyboard"\]/, `${OWNER}:${line} ${selector}`);
    assert.match(body, /var\(--focus-ring-width\) solid var\(--focus-ring-color\)/);
  }
});
