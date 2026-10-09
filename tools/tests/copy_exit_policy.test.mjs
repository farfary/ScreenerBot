// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a Copy Trading rule card and the exit warnings name one stop-loss value.
 *
 * The backend merges a task's exit overrides per field, so a stored threshold under a
 * switch that follows the Trader still applies. The card note and the warnings both
 * read the resolved policy; neither may fall back to the Trader default alone.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  RULES,
  effectivePolicy,
  exitWarnings,
  inheritNote,
} from "../../src/webserver/templates/scripts/pages/copy/policy.js";

const plain = (text) => text.replace(/[⁨⁩]/g, "");
const workspace = JSON.parse(
  readFileSync(new URL("./fixtures/dashboard/copy/copy_workspace.json", import.meta.url), "utf8")
);
const STOP = RULES.find((rule) => rule.group === "stop_loss");

/** Every percentage a text names, unsigned. */
const percents = (text) => [...plain(text).matchAll(/(\d+(?:\.\d+)?)%/g)].map((m) => m[1]);

test("a stored threshold under an inherited switch shows in the card and the warning alike", () => {
  const { trader_defaults: traderDefaults, exit_policy_overrides: overrides } = workspace;
  assert.equal(
    workspace.effective_policy.stop_loss.threshold_pct,
    overrides.stop_loss.threshold_pct,
    "the backend resolves the stored threshold"
  );
  assert.equal(overrides.stop_loss.enabled, null, "the switch follows the Trader");
  assert.notEqual(overrides.stop_loss.threshold_pct, traderDefaults.stop_loss.threshold_pct);

  const note = plain(inheritNote(STOP, traderDefaults, overrides));
  const resolved = Number(overrides.stop_loss.threshold_pct).toFixed(1);
  assert.ok(percents(note).includes(resolved), `the card names ${resolved}%: ${note}`);
  assert.ok(
    !percents(note).includes(Number(traderDefaults.stop_loss.threshold_pct).toFixed(1)),
    `the card does not name the Trader default: ${note}`
  );
  assert.match(note, /this task's values/);

  const warnings = exitWarnings(effectivePolicy(traderDefaults, overrides), workspace.exit_mode, {})
    .map(plain)
    .filter((text) => /stop/i.test(text));
  for (const text of warnings) {
    assert.ok(
      !percents(text).includes(Number(traderDefaults.stop_loss.threshold_pct).toFixed(1)),
      `the warning does not name the Trader default: ${text}`
    );
  }
});

test("a rule with no stored values follows the Trader's own summary", () => {
  const traderDefaults = { stop_loss: { enabled: true, threshold_pct: 40, min_hold_seconds: 120 } };
  const note = plain(inheritNote(STOP, traderDefaults, { stop_loss: { enabled: null } }));
  assert.match(note, /^Follows the Trader: /);
  assert.ok(percents(note).includes("40.0"), note);
});
