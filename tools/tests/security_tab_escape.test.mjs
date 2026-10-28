// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every modal the Security settings tab raises takes its Escape from the shared
 * stack (`core/escape_stack.js`) and releases it on every close path.
 *
 * Each function that shows the tab's modal (`modal.hidden = false`) registers
 * `pushEscapeHandler(closeModal)`, and its `closeModal` releases the entry before it hides
 * the modal. No modal carries an inline `style` attribute.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { SCRIPTS_ROOT } from "../lib/dashboard_ui.mjs";

const source = readFileSync(`${SCRIPTS_ROOT}/ui/settings/security_tab.js`, "utf8");
const functions = source
  .split(/\n(?=(?:export )?(?:async )?function )/)
  .filter((body) => body.includes("modal.hidden = false"));

test("every Security tab modal registers and releases its Escape entry", () => {
  assert.ok(functions.length >= 4, `expected the modal functions, found ${functions.length}`);
  for (const body of functions) {
    const name = /function (\w+)/.exec(body)?.[1];
    assert.match(body, /pushEscapeHandler\(closeModal\)/, `${name} registers Escape`);
    assert.match(
      body,
      /const closeModal = \(\) => \{\s*releaseEscape(?:\?\.)?\(\);/,
      `${name} releases Escape before it hides the modal`
    );
  }
});

test("the password and two-factor modals are styled by classes, not inline", () => {
  for (const body of functions) {
    assert.doesNotMatch(body, /style="/);
  }
});
