// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Trade Watcher tool calls only routes `routes/tools/mod.rs` registers, and
 * its add request is the `AddWatchedTokenRequest` the watched-tokens route reads. The
 * active list is `GET /watched-tokens`, a watch stops with `DELETE /watched-tokens/{id}`,
 * and the monitor starts and stops through `/trade-watcher/start` and `/stop`.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

globalThis.window = globalThis.window || { addEventListener() {} };
globalThis.document = { addEventListener() {}, readyState: "complete" };

const SOURCE_URL = new URL(
  "../../src/webserver/templates/scripts/pages/tools/trading_tools.js",
  import.meta.url
);
const ROUTES_URL = new URL("../../src/webserver/routes/tools/mod.rs", import.meta.url);

const { buildWatchRequest } = await import(SOURCE_URL);

/** A fetch target with its URL constants inlined and its path parameters named `{id}`. */
function resolveTarget(argument, constants) {
  return argument
    .replace(/^`|`$/g, "")
    .replace(/\$\{(\w+_URL)\}/g, (_, name) => constants.get(name))
    .replace(/\$\{[^}]+\}/g, "{id}")
    .replace(/^(\w+_URL)$/, (_, name) => constants.get(name));
}

test("every URL the tool fetches is a registered tools route", () => {
  const source = fs.readFileSync(SOURCE_URL, "utf8");
  const routes = new Set(
    [...fs.readFileSync(ROUTES_URL, "utf8").matchAll(/\.route\(\s*"([^"]+)"/g)].map((m) => m[1])
  );
  const constants = new Map(
    [...source.matchAll(/const (\w+_URL) = "(\/api\/tools\/[^"]+)"/g)].map((m) => [m[1], m[2]])
  );
  const targets = [...source.matchAll(/fetch\(\s*(`[^`]+`|\w+_URL)/g)].map((m) => m[1]);
  assert.ok(targets.length >= 6, "the tool fetches through its URL constants");

  for (const argument of targets) {
    const path = resolveTarget(argument, constants);
    assert.ok(path.startsWith("/api/tools/"), `unresolved fetch target ${argument}`);
    assert.ok(routes.has(path.slice("/api/tools".length)), `unregistered route ${path}`);
  }
});

test("the add request is the watched-tokens body", () => {
  const body = buildWatchRequest({
    mint: "So11111111111111111111111111111111111111112",
    symbol: "SOL",
    pool: { address: "Pool1", source: "GeckoTerminal", dex: "raydium" },
    watchType: "buy-on-sell",
    triggerAmount: 0.5,
    actionAmount: 0.1,
    slippagePct: 5,
  });
  assert.deepEqual(body, {
    mint: "So11111111111111111111111111111111111111112",
    symbol: "SOL",
    pool_address: "Pool1",
    pool_source: "geckoterminal",
    pool_dex: "raydium",
    watch_type: "buy-on-sell",
    trigger_amount_sol: 0.5,
    action_amount_sol: 0.1,
    slippage_bps: 500,
  });
});
