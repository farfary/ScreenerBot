// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the paper-model contracts of Copy Trading that need no page.
 *
 * - A manual paper close is sent once with the long trade timeout and without the request
 *   manager's connection retry: a lost response is reconciled from the workspace, never
 *   submitted again. Every other writer keeps the shared default.
 * - The take-profit warning states that a small target may not cover the selling costs;
 *   the slippage tolerance is a limit on the sale price, never a predicted cost.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import assert from "node:assert/strict";
import test from "node:test";

import {
  CLOSE_TIMEOUT_MS,
  createApi,
} from "../../src/webserver/templates/scripts/pages/copy/api.js";
import { exitWarnings } from "../../src/webserver/templates/scripts/pages/copy/policy.js";

const plain = (text) => text.replace(/[⁦-⁩]/g, "");

function recordingApi() {
  const calls = [];
  const api = createApi({
    fetch: async (url, options) => {
      calls.push({ url, options });
      return {};
    },
  });
  return { api, calls };
}

test("a manual close is sent once, with the long timeout and no connection retry", async () => {
  const { api, calls } = recordingApi();
  await api.closeHolding(7, "Mint/With Space", "2026-10-07T03:00:00.123456789Z");
  assert.equal(calls.length, 1);
  const [{ url, options }] = calls;
  assert.equal(options.body, '{"opened_at":"2026-10-07T03:00:00.123456789Z"}');
  assert.equal(url, "/api/copy-trading/tasks/7/holdings/Mint%2FWith%20Space/close");
  assert.equal(options.method, "POST");
  assert.equal(options.timeout, CLOSE_TIMEOUT_MS);
  assert.equal(CLOSE_TIMEOUT_MS, 1_800_000);
  assert.equal(options.retryOnConnectionError, false);
  assert.equal(options.skipDedup, true);
  assert.equal(options.priority, "high");
});

test("no copy write is replayed after a lost connection", async () => {
  const { api, calls } = recordingApi();
  await api.create({ target_address: "Target" });
  await api.update(3, { enabled: false });
  await api.remove(3);
  await api.setMode(3, "paper");
  await api.reset(3);
  await api.closeHolding(3, "Mint", "2026-10-07T03:00:00Z");
  await api.patchConfig({ enabled: true });
  assert.equal(calls.length, 7);
  for (const { url, options } of calls) {
    assert.notEqual(options.method, "GET", url);
    assert.equal(options.retryOnConnectionError, false, `${options.method} ${url} must not replay`);
    assert.equal(options.skipDedup, true, `${options.method} ${url}`);
    assert.equal(options.priority, "high", `${options.method} ${url}`);
  }
});

test("reads keep the request manager's retry policy", async () => {
  const { api, calls } = recordingApi();
  await api.overview();
  await api.activity(3, { limit: 5 });
  for (const { url, options } of calls) {
    assert.equal(options.method, "GET", url);
    assert.ok(!Object.hasOwn(options, "retryOnConnectionError"), url);
  }
});

const roi = (target) => ({ roi: { enabled: true, target_profit_pct: target } });
const takeProfit = (target, costs) =>
  exitWarnings(roi(target), "hybrid", costs)
    .map(plain)
    .filter((text) => /^Take profit/.test(text));
const percents = (text) => [...text.matchAll(/(\d+(?:\.\d+)?)%/g)].map((match) => Number(match[1]));

test("a take profit within the selling costs warns without promising a loss", () => {
  const [warning, ...rest] = takeProfit(2, { slippagePct: 1.5, feePct: 0.5 });
  assert.equal(rest.length, 0);
  assert.deepEqual(percents(warning), [2, 1.5, 0.5], warning);
  assert.match(warning, /may not cover/);
  assert.match(warning, /not a predicted cost/);
  assert.doesNotMatch(warning, /closes rounds at a loss|will lose|so it closes/i);
});

test("the warning stops once the target exceeds the slippage tolerance plus the fee", () => {
  assert.equal(takeProfit(2.5, { slippagePct: 2, feePct: 0.5 }).length, 1);
  assert.equal(takeProfit(2.51, { slippagePct: 2, feePct: 0.5 }).length, 0);
  assert.equal(takeProfit(80, { slippagePct: 2, feePct: 0.5 }).length, 0);
});

test("the warning needs both costs; an unknown cost is never assumed", () => {
  assert.equal(takeProfit(1, { slippagePct: 2 }).length, 0);
  assert.equal(takeProfit(1, { feePct: 0.5 }).length, 0);
  assert.equal(takeProfit(1, {}).length, 0);
});
