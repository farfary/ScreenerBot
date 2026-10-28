// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the mode action of a strategy, and arming live stays gated.
 *
 * - A paper strategy that is on offers Arm live, a live one Return to paper, and a
 *   strategy that is off offers neither, since it trades on no book.
 * - An exit strategy's arm and return texts speak of the live positions it sells or stops
 *   judging, never of new entries.
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
const { armAllowed, armDialogBody, createStrategyMode, modeAction } = await import(
  new URL("../../src/webserver/templates/scripts/pages/strategies/strategy_mode.js", import.meta.url)
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

test("a strategy that is on offers the action that changes its mode", () => {
  assert.deepEqual(modeAction({ id: "momentum", mode: "paper", enabled: true }), {
    id: "arm",
    label: "Arm live",
  });
  assert.deepEqual(modeAction({ id: "momentum", mode: "live", enabled: true }), {
    id: "return-paper",
    label: "Return to paper",
  });
});

test("a strategy that is off offers no mode action", () => {
  for (const mode of ["paper", "live"]) {
    assert.equal(modeAction({ id: "momentum", mode, enabled: false }), null, mode);
  }
});

/** The mode flows over a dialog that records what it shows and answers `confirmed`. */
function modeFlows(confirmed) {
  const shown = [];
  const flows = createStrategyMode({
    requestManager: { fetch: async () => readiness() },
    ConfirmationDialog: {
      show: async (config) => {
        shown.push({ ...config, message: plain(config.message), body: plain(config.body ?? "") });
        return { confirmed };
      },
    },
    Utils: { escapeHtml: esc, showToast: () => {} },
  });
  return { flows, shown };
}

test("arming an exit strategy live speaks of the positions it sells", async () => {
  const { flows, shown } = modeFlows(false);
  const exit = { id: "trail", name: "Trail", mode: "paper", enabled: true, strategy_type: "EXIT" };
  assert.equal(await flows.armLive(exit), false, "a cancelled dialog changes nothing");
  const [dialog] = shown;
  assert.match(dialog.message, /Trail.* will sell live positions with real swaps/);
  assert.match(dialog.body, /Live positions it manages are sold with real swaps\./);
  assert.doesNotMatch(`${dialog.message} ${dialog.body}`, /New entries/);

  const { flows: entryFlows, shown: entryShown } = modeFlows(false);
  await entryFlows.armLive({ ...exit, strategy_type: "ENTRY" });
  assert.match(entryShown[0].message, /New entries of .*Trail.* will trade with real swaps/);
  assert.match(entryShown[0].body, /New entries spend real SOL from your trading wallet\./);
});

test("returning an exit strategy to paper says it stops judging live positions", async () => {
  const { flows, shown } = modeFlows(false);
  const exit = { id: "trail", name: "Trail", mode: "live", enabled: true, strategy_type: "EXIT" };
  assert.equal(await flows.returnToPaper(exit), false);
  assert.equal(shown[0].message, "It stops judging live positions; they keep their rule exits.");

  const { flows: entryFlows, shown: entryShown } = modeFlows(false);
  await entryFlows.returnToPaper({ ...exit, strategy_type: "ENTRY" });
  assert.match(entryShown[0].message, /Open live positions stay live and finish live\./);
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
