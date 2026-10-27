// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the token details Positions tab states every exit rule the exit monitor
 * applies, as the resolved policy switches them on, and nothing else.
 *
 * - The stop loss reads as the loss it sells at.
 * - Each take-profit step reads as its profit level and the share it sells; a step
 *   this round already sold reads as Sold.
 * - The trailing stop reads as its activation and distance, and each band as the
 *   profit it starts at and the distance it trails by.
 * - The break-even stop, the losing-time exit and the flat-position exit read as
 *   their own values.
 * - A rule that is off is not listed; with every rule off the plan says so.
 * - A position the monitor evaluates no policy exit for shows no plan at all.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import "./fixtures/document_shim.mjs";
import test from "node:test";
import assert from "node:assert/strict";

import { renderExitPlan } from "../../src/webserver/templates/scripts/ui/token_details/positions_tab.js";

const text = (fragment) =>
  fragment
    .replace(/<[^>]+>/g, "")
    .replace(/[⁨⁩]/g, "")
    .replace(/\s+/g, " ")
    .trim();

/** The label and value of every fact the rendered plan holds, without markup or isolates. */
function facts(html) {
  return [...html.matchAll(/<span>([\s\S]*?)<\/span>\s*<strong>([\s\S]*?)<\/strong>/g)].map(
    ([, label, value]) => [text(label), text(value)]
  );
}

const OFF = {
  stop_loss_pct: null,
  take_profit_steps: [],
  trailing_stop: null,
  break_even: null,
  time_loss: null,
  time_stagnation: null,
};

test("the plan states every rule that is on, a sold step as Sold", () => {
  const html = renderExitPlan({
    stop_loss_pct: 50,
    take_profit_steps: [
      { profit_pct: 20, sell_pct: 40, taken: true },
      { profit_pct: 60, sell_pct: 60, taken: false },
    ],
    trailing_stop: {
      activation_pct: 20,
      distance_pct: 10,
      bands: [{ profit_pct: 50, distance_pct: 6 }],
    },
    break_even: { trigger_pct: 15, offset_pct: 2 },
    time_loss: { duration_seconds: 7200, loss_threshold_pct: -10 },
    time_stagnation: { duration_seconds: 14400, band_pct: 3 },
  });
  assert.match(html, /Exit plan/);
  assert.deepEqual(facts(html), [
    ["Stop Loss", "-50.0%"],
    ["Take Profit 1", "Sold"],
    ["Take Profit 2", "At +60.0%, sells 60%"],
    ["Trailing Stop", "Arms at +20.0%, trails 10% below the peak"],
    ["Trailing Band 1", "From +50.0%, trails 6% below the peak"],
    ["Break-Even Stop", "Arms at +15.0%, sells back at +2.0%"],
    ["Losing-Time Exit", "After 2h, at or below -10.0%"],
    ["Flat Position Exit", "After 4h, while within 3% of the entry"],
  ]);
});

test("a rule that is off is not listed", () => {
  const html = renderExitPlan({ ...OFF, stop_loss_pct: 25 });
  assert.deepEqual(facts(html), [["Stop Loss", "-25.0%"]]);
});

test("with every rule off the plan says no rule is on", () => {
  const html = renderExitPlan(OFF);
  assert.deepEqual(facts(html), []);
  assert.match(text(html), /No exit rule is on/);
});

test("a position with no policy exit shows no plan", () => {
  assert.equal(renderExitPlan(null), "");
  assert.equal(renderExitPlan(undefined), "");
});
