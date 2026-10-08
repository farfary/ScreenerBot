// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the header quick actions fold behind a disclosure toggle only when they do not
 * fit, and the folded group behaves as a disclosure that never covers the navigation.
 *
 * - `core/header.js` opens the group from its toggle alone (no hover or focus opening,
 *   which made a second click on the toggle re-open what it had just closed), closes it
 *   through the shared escape stack, and decides the fold by measurement rather than a
 *   viewport-width drawer range.
 * - On a desktop header the open group fans out inline inside the header row; only the
 *   phone header, which has no room beside the brand and trader, drops it below the row.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { SCRIPTS_ROOT, STYLES_ROOT, rulesIn } from "../lib/dashboard_ui.mjs";

const HEADER_SCRIPT = readFileSync(`${SCRIPTS_ROOT}/core/header.js`, "utf8");
const HEADER_STYLES = readFileSync(`${STYLES_ROOT}/header.css`, "utf8");
const RESPONSIVE_STYLES = readFileSync(`${STYLES_ROOT}/header_responsive.css`, "utf8");

const FOLDED_ITEMS = ".modern-header .header-actions.is-folded .header-actions-items";

/** The body of `initHeaderActionsToggle`, up to the next top-level function. */
function toggleSource() {
  const start = HEADER_SCRIPT.indexOf("function initHeaderActionsToggle()");
  assert.ok(start >= 0, "header.js defines initHeaderActionsToggle");
  const end = HEADER_SCRIPT.indexOf("\nfunction ", start + 1);
  return HEADER_SCRIPT.slice(start, end < 0 ? undefined : end);
}

/** The text of the `@media` block that opens with `query`. */
function mediaBlock(css, query) {
  const start = css.indexOf(`@media ${query}`);
  assert.ok(start >= 0, `header_responsive.css has @media ${query}`);
  let depth = 0;
  for (let index = css.indexOf("{", start); index < css.length; index += 1) {
    if (css[index] === "{") depth += 1;
    if (css[index] === "}" && (depth -= 1) === 0) return css.slice(start, index + 1);
  }
  return css.slice(start);
}

const declaration = (body, property) =>
  body.match(new RegExp(`(?:^|;)\\s*${property}\\s*:\\s*([^;]+)`))?.[1].trim();

test("the header actions toggle is a measured disclosure on the shared escape stack", () => {
  const source = toggleSource();
  assert.match(source, /pushEscapeHandler\(/);
  assert.doesNotMatch(source, /"(?:mouseenter|focusin)"/);
  assert.doesNotMatch(source, /addEventListener\("keydown"/);
  assert.match(source, /new ResizeObserver\(/);
  assert.doesNotMatch(source, /min-width:\s*800px|width\s*>=\s*800px/);
});

test("the folded desktop group opens inside the header row", () => {
  const rules = rulesIn(HEADER_STYLES).filter((rule) => rule.selector === FOLDED_ITEMS);
  assert.equal(rules.length, 1, `header.css declares ${FOLDED_ITEMS} once`);
  assert.equal(declaration(rules[0].body, "top"), "50%");
  assert.match(declaration(rules[0].body, "inset-inline-end") ?? "", /^calc\(100%/);
});

test("only the phone header drops the folded group below the row", () => {
  const phone = mediaBlock(RESPONSIVE_STYLES, "(width <= 500px)");
  const outside = RESPONSIVE_STYLES.replace(phone, "");
  const below = rulesIn(outside).filter(
    (rule) =>
      rule.selector.includes("header-actions-items") &&
      /^calc\(100%/.test(declaration(rule.body, "top") ?? "")
  );
  assert.deepEqual(
    below.map((rule) => `${rule.line} ${rule.selector}`),
    []
  );
});
