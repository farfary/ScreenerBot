// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Tests for the shared trade and close reason text (`ui/trade_reason.js`).
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import "./fixtures/i18n_en.mjs";

const UI = "../../src/webserver/templates/scripts/ui/";
const { closeReasonText } = await import(new URL(`${UI}trade_reason.js`, import.meta.url));
const { stepLabel, stepShortLabel } = await import(new URL(`${UI}action_step.js`, import.meta.url));

test("known reasons use the single Title Case label", () => {
  assert.equal(closeReasonText("StopLoss"), "Stop Loss");
  assert.equal(closeReasonText("LlmAnalysisExit"), "LLM Analysis Exit");
  assert.equal(closeReasonText("closed_externally"), "Closed Externally");
});

test("the pending verification suffix wraps the base label", () => {
  assert.match(closeReasonText("StopLoss_pending_verification"), /Stop Loss.* \(pending verification\)/);
});

test("force closed carries the operator note", () => {
  assert.match(closeReasonText("force_closed: stuck in the pool"), /^Force closed: .*stuck in the pool/);
});

test("unknown reasons render as stored", () => {
  assert.equal(closeReasonText("some_new_reason"), "some_new_reason");
});

test("step codes resolve to labels", () => {
  assert.equal(stepLabel("quote"), "Getting Quote");
  assert.equal(stepShortLabel("validate"), "Checking");
  assert.equal(stepLabel("unknown"), "Processing");
});
