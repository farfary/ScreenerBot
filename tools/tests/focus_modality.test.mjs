// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a focus indicator is drawn only in the keyboard modality.
 *
 * `foundation.css` removes every outline with `!important` and suppresses box shadows
 * while the input is a pointer, so a `:focus-visible` rule that is not gated on
 * `html[data-input="keyboard"]` is either dead (an outline never shows) or fights the
 * global layer. Every stylesheet rule whose selector contains `:focus-visible` and that
 * sets `outline`, `box-shadow` or a `border` must therefore be keyboard-gated.
 *
 * - `foundation.css`'s global layer (the outline kill and the `data-input="mouse"`
 *   suppression) is the gate itself and is exempt.
 * - The files below still carry ungated rules. The allowlist only shrinks, and each entry
 *   must still need it.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { loadStylesheets } from "../lib/dashboard_ui.mjs";

const UNGATED_ALLOWLIST = new Set([
  "base/floating.css",
  "base/scrollbar.css",
  "components.css",
  "components/chat_widget/input.css",
  "components/form_controls.css",
  "pages/assistant/automation.css",
  "pages/assistant/instructions.css",
  "pages/copy.css",
  "pages/lockscreen.css",
  "pages/strategies/condition_cards.css",
  "pages/strategies/modals.css",
  "settings/agent_connections.css",
  "ui/confirmation_dialog.css",
  "ui/context_menu.css",
  "ui/data_table/column_types.css",
  "ui/data_table/core.css",
  "ui/data_table/pagination.css",
  "ui/search_dialog.css",
  "ui/trade_action_dialog.css",
]);

const DRAWS_FOCUS = /(?:^|;)\s*(?:outline|box-shadow|border(?:-[\w-]+)?)\s*:/;
const GATED = /html\[data-input="(?:keyboard|mouse)"\]/;
const GLOBAL_LAYER = new Set(["*:focus-visible"]);

async function ungatedRules() {
  const found = [];
  for (const { path, rules } of await loadStylesheets()) {
    for (const { selector, body, line } of rules) {
      if (!selector.includes(":focus-visible") || GATED.test(selector)) continue;
      if (path === "foundation.css" && GLOBAL_LAYER.has(selector)) continue;
      if (DRAWS_FOCUS.test(body)) found.push({ path, selector, line });
    }
  }
  return found;
}

test("every focus indicator is gated on the keyboard modality", async () => {
  const offenders = (await ungatedRules())
    .filter(({ path }) => !UNGATED_ALLOWLIST.has(path))
    .map(({ path, line, selector }) => `${path}:${line} ${selector}`);
  assert.deepEqual(offenders, []);
});

test("the tab rows' stylesheet is never allowlisted", () => {
  assert.equal(UNGATED_ALLOWLIST.has("ui/tab_bar.css"), false);
});

test("the ungated allowlist names only files that still need it", async () => {
  const present = new Set((await ungatedRules()).map(({ path }) => path));
  for (const path of UNGATED_ALLOWLIST) assert.ok(present.has(path), `${path} no longer needs it`);
});
