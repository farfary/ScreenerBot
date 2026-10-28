// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guards that the recorded dashboard responses describe one consistent state.
 *
 * The fixtures stand in for the server in every view test and browser review. When two of
 * them disagree (an open count above its limit, a readiness verdict that contradicts its
 * own checks, Home counting positions the Positions list does not hold), the views state
 * something false and the disagreement reads as a product defect.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const fixture = (path) =>
  JSON.parse(readFileSync(new URL(`./fixtures/dashboard/${path}`, import.meta.url), "utf8"));

test("the trader's open count stays within its limit, and a limit block means it is reached", () => {
  const stats = fixture("trader/trader_stats.json");
  const status = fixture("trader/trader_status.json");
  assert.ok(stats.open_positions_count <= stats.max_open_positions);
  const blockedByLimit = status.live_entry_idle?.block?.kind === "position_limit";
  assert.equal(
    blockedByLimit,
    stats.open_positions_count === stats.max_open_positions,
    "the position-limit block matches the stats open count"
  );
});

test("Home counts the open positions the Positions list holds", () => {
  const home = fixture("home/dashboard_home.json");
  const open = fixture("positions/positions_open.json");
  assert.equal(home.positions.open_count, open.length);
  assert.ok(home.auto_trader.open_positions <= home.positions.open_count);
  assert.ok(home.auto_trader.open_positions <= home.auto_trader.max_open_positions);
});

test("a Copy readiness verdict agrees with its checks", () => {
  for (const path of ["copy/copy_workspace.json", "copy/copy_workspace.empty.json"]) {
    const { readiness } = fixture(path);
    if (!readiness) continue;
    assert.equal(
      readiness.ready,
      readiness.checks.every((check) => check.passed),
      `${path}: ready is true exactly when every check passed`
    );
  }
});
