// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: Strategy Control states each strategy's trading mode once, as text, and arming
 * live stays gated.
 *
 * - A paper strategy shows the bare Paper glyph, the Paper label and Arm live; a live
 *   strategy shows the Live label with no glyph and Return to paper.
 * - The arm dialog lists the readiness checks and what happens to open paper rounds; while
 *   live trading cannot run it shows why and asks for nothing.
 * - Arm live stays disabled until every acknowledgement is ticked and the exact phrase the
 *   API named is typed; an unready strategy adds its own acknowledgement.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { installEnglishI18n } from "../lib/dashboard_i18n.mjs";

installEnglishI18n();
const { armAllowed, armDialogBody, strategyModeRow } = await import(
  new URL("../../src/webserver/templates/scripts/pages/trader/strategy_mode.js", import.meta.url)
);

const esc = (text) =>
  String(text).replace(/[&<>"']/g, (char) => `&#${char.charCodeAt(0)};`);
const plain = (text) => String(text).replace(/[⁦-⁩]/g, "");
const PHRASE = "ARM LIVE STRATEGY";

const check = (id, passed) => ({
  id,
  passed,
  text: { id: "paper-readiness-exits", args: {} },
  detail: { id: "paper-readiness-exits-ok", args: {} },
});
const readiness = ({ ready = true, runtime = true } = {}) => ({
  ready,
  checks: [check("history", ready), check("runtime", runtime)],
  live_confirmation: PHRASE,
});

/** A dialog root over `acks` checkbox states and the typed `phrase`. */
function dialogRoot(acks, phrase) {
  const boxes = acks.map((checked) => ({ checked }));
  return {
    querySelectorAll: (selector) => (selector === "[data-ack]" ? boxes : []),
    querySelector: (selector) => (selector === "[data-arm-phrase]" ? { value: phrase } : null),
  };
}

test("a paper strategy shows its bare glyph, its label and Arm live", () => {
  const row = strategyModeRow({ id: "momentum", mode: "paper" }, esc);
  assert.match(row, /<i class="icon-test-tube-diagonal" aria-hidden="true"><\/i><span>Paper<\/span>/);
  assert.match(row, /data-arm-strategy="momentum">Arm live</);
  assert.doesNotMatch(row, /data-paper-strategy/);
});

test("a live strategy shows its label with no glyph and Return to paper", () => {
  const row = strategyModeRow({ id: "momentum", mode: "live" }, esc);
  assert.doesNotMatch(row, /<i /);
  assert.match(row, /<span>Live<\/span>/);
  assert.match(row, /data-paper-strategy="momentum">Return to paper</);
  assert.doesNotMatch(row, /data-arm-strategy/);
});

test("the arm dialog lists the checks, the open paper note, the acknowledgements and the phrase", () => {
  const body = plain(armDialogBody(readiness(), esc));
  assert.equal((body.match(/class="arm-gate-check /g) || []).length, 2);
  assert.match(body, /Open paper positions stay paper and finish on paper\./);
  assert.match(body, /New entries spend real SOL from your trading wallet\./);
  assert.match(body, /Live swaps pay real network fees and slippage/);
  assert.doesNotMatch(body, /Arm this strategy anyway/);
  assert.match(body, /Type ARM LIVE STRATEGY to confirm/);
  assert.equal((body.match(/data-ack /g) || []).length, 2);

  const unready = plain(armDialogBody(readiness({ ready: false }), esc));
  assert.equal((unready.match(/data-ack /g) || []).length, 3);
  assert.match(unready, /Some readiness checks have not passed\. Arm this strategy anyway\./);
});

test("while live trading cannot run, the dialog says why and asks for nothing", () => {
  const body = plain(armDialogBody(readiness({ runtime: false }), esc));
  assert.match(body, /Live trading is unavailable right now; see the last check\./);
  assert.doesNotMatch(body, /data-ack|data-arm-phrase/);
  assert.equal(armAllowed(dialogRoot([], PHRASE), readiness({ runtime: false })), false);
  assert.equal(armAllowed(dialogRoot([], PHRASE), null), false, "no readiness read is not ready");
});

test("Arm live needs every acknowledgement and the exact phrase", () => {
  const ready = readiness();
  assert.equal(armAllowed(dialogRoot([true, true], PHRASE), ready), true);
  assert.equal(armAllowed(dialogRoot([true, false], PHRASE), ready), false);
  assert.equal(armAllowed(dialogRoot([true, true], ""), ready), false);
  assert.equal(armAllowed(dialogRoot([true, true], PHRASE.toLowerCase()), ready), false);
  assert.equal(armAllowed(dialogRoot([true, true], `${PHRASE} `), ready), false);
  assert.equal(
    armAllowed(dialogRoot([true, true], PHRASE), { ...ready, live_confirmation: "" }),
    false,
    "an empty phrase never matches"
  );
});
