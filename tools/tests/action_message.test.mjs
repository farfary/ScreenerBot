// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Tests for the wording of a live trade (`core/action_message.js`).
 *
 * What a user reads while a swap runs: which router is submitting it, and —
 * when the cost guard refused a route that would have locked SOL in a venue's
 * own account — which venue was avoided and how much it would have cost. A
 * re-routing trade that says nothing looks like a hung trade.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import "./fixtures/i18n_en.mjs";

const MODULE = new URL(
  "../../src/webserver/templates/scripts/core/action_message.js",
  import.meta.url
);

const messages = await import(MODULE);
const { costGuardOf, routerOf, symbolOf } = messages;

// Fluent isolates every placeable (U+2068/U+2069); the wording is asserted without them.
const plain = (text) => (typeof text === "string" ? text.replace(/[\u2068\u2069]/g, "") : text);
const costGuardNote = (action) => plain(messages.costGuardNote(action));
const outcomeMessage = (action) => plain(messages.outcomeMessage(action));
const stepMessage = (action) => plain(messages.stepMessage(action));

function action({ steps = [], state = {}, metadata = {} } = {}) {
  return {
    id: "a1",
    steps,
    metadata,
    state: { status: "in_progress", current_step: "swap", total_steps: 4, current_step_index: 2, ...state },
  };
}

const routerStep = { metadata: { router: "Jupiter" } };
const guardStep = {
  metadata: {
    router: "Jupiter",
    cost_guard: { venue: "HumidiFi", extra_lamports: 13045440 },
  },
};

test("a running trade names the step and the router that is submitting it", () => {
  assert.equal(stepMessage(action({ steps: [routerStep] })), "Executing Swap via Jupiter · 3/4");
});

test("a trade with no router yet still reports its step", () => {
  assert.equal(stepMessage(action()), "Executing Swap · 3/4");
  assert.equal(routerOf(action()), "");
});

test("a refused route says which venue is being avoided and what it would have cost", () => {
  const live = action({ steps: [guardStep] });
  assert.equal(costGuardNote(live), "avoiding HumidiFi · 0.0130 SOL");
  assert.equal(
    stepMessage(live),
    "Executing Swap via Jupiter · avoiding HumidiFi · 0.0130 SOL · 3/4"
  );
});

test("the most recent refusal is the one shown", () => {
  const first = { metadata: { cost_guard: { venue: "VenueA", extra_lamports: 1_000_000 } } };
  const second = { metadata: { cost_guard: { venue: "VenueB", extra_lamports: 2_000_000 } } };
  assert.deepEqual(costGuardOf(action({ steps: [first, second] })), {
    venue: "VenueB",
    sol: "0.0020 SOL",
  });
});

test("a venue with no name still reports the cost, and a cost of zero is not reported", () => {
  const unnamed = { metadata: { cost_guard: { extra_lamports: 13045440 } } };
  assert.equal(costGuardNote(action({ steps: [unnamed] })), "avoiding a venue · 0.0130 SOL");

  const zero = { metadata: { cost_guard: { venue: "HumidiFi", extra_lamports: 0 } } };
  assert.deepEqual(costGuardOf(action({ steps: [zero] })), { venue: "HumidiFi", sol: "" });
  assert.equal(costGuardNote(action({ steps: [zero] })), "avoiding HumidiFi");
});

test("a malformed or absent guard record never produces a note", () => {
  assert.equal(costGuardOf(action()), null);
  assert.equal(costGuardOf(action({ steps: [{ metadata: { cost_guard: "nope" } }] })), null);
  assert.equal(costGuardOf(action({ steps: [{ metadata: { cost_guard: {} } }] })), null);
  assert.equal(
    costGuardOf(action({ steps: [{ metadata: { cost_guard: { extra_lamports: "x" } } }] })),
    null
  );
  assert.equal(costGuardNote(action()), "");
  assert.equal(costGuardOf(null), null);
});

test("a finished trade reports its size, its router and anything it avoided", () => {
  const done = {
    steps: [guardStep],
    metadata: { size_sol: 0.005 },
    state: { status: "completed" },
  };
  assert.equal(
    outcomeMessage(done),
    "0.005 SOL via Jupiter · avoided 0.0130 SOL in HumidiFi rent"
  );

  const exit = { steps: [routerStep], metadata: { percentage: 100 } };
  assert.equal(outcomeMessage(exit), "Full exit via Jupiter");

  const partial = { steps: [routerStep], metadata: { percentage: 25 } };
  assert.equal(outcomeMessage(partial), "25% exit via Jupiter");

  assert.equal(outcomeMessage({ metadata: { reason: "Stop loss" } }), "Stop loss");
  assert.equal(outcomeMessage({}), null);
});

test("only an in-progress action produces a step message", () => {
  assert.equal(stepMessage(action({ state: { status: "completed" } })), null);
  assert.equal(stepMessage(action({ state: { current_step: null } })), null);
  assert.equal(stepMessage(null), null);
});

test("an unresolved symbol is treated as no symbol", () => {
  assert.equal(symbolOf({ metadata: { symbol: "Unknown" } }), "");
  assert.equal(symbolOf({ metadata: { symbol: "JUP" } }), "JUP");
  assert.equal(symbolOf({}), "");
});

test("a router display name renders through its venue label, and an unknown one as received", () => {
  const direct = { metadata: { router: "Direct Pool" } };
  assert.equal(stepMessage(action({ steps: [direct] })), "Executing Swap via Direct Pool · 3/4");

  const lower = { metadata: { router: "jupiter" } };
  assert.equal(stepMessage(action({ steps: [lower] })), "Executing Swap via Jupiter · 3/4");

  const other = { metadata: { router: "SomeRouter" } };
  assert.equal(stepMessage(action({ steps: [other] })), "Executing Swap via SomeRouter · 3/4");
});
