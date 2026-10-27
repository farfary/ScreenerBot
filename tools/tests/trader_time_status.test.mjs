// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Time Exit column words the watch the trader route computes, and decides nothing.
 *
 * `/api/trader/time-exits` names, for each open position, whether Auto Trader manages its
 * exits, whether a time rule is on, which rule applies and when it is due. The column
 * reads that watch alone: a position Auto Trader does not manage says so, a due rule
 * shows the time left or the next check once due, and a position the route has no watch
 * for shows a dash rather than a guess.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import assert from "node:assert/strict";
import test from "node:test";

import { timeExitStatus } from "../../src/webserver/templates/scripts/pages/trader/time_status.js";

const NOW = 1_800_000_000;
const plain = (text) => text.replace(/[⁦-⁩]/g, "");
const line = (watch, held = 7200) => plain(timeExitStatus(watch, held, NOW));

test("a position Auto Trader does not manage reads as not managed", () => {
  for (const management of ["user_only", "copy_task"]) {
    assert.equal(line({ status: "not_managed", management }), "Not managed by Auto Trader");
  }
});

test("rules off, no rule applying and no watch each have their own line", () => {
  assert.equal(line({ status: "off" }), "Time exits are off");
  assert.equal(line({ status: "clear" }), "No time exit applies now");
  assert.equal(line({ status: "unpriced" }), "—");
  assert.equal(line(undefined), "—");
});

test("the loss rule shows its loss and the time left, then the next check once due", () => {
  const due = (dueAt) => ({ status: "due", rule: "time_loss", due_at: dueAt, pnl_pct: -45.2 });
  const waiting = line(due(NOW + 5 * 3600));
  assert.match(waiting, /^Losing .*45\.2%; closes in 5h$/);
  assert.match(line(due(NOW)), /^Losing .*45\.2%; closes on the next check$/);
  assert.match(line(due(NOW - 60)), /closes on the next check$/);
});

test("the flat rule shows the time held and the time left, then the next check once due", () => {
  const due = (dueAt) => ({ status: "due", rule: "time_stagnation", due_at: dueAt, pnl_pct: 1.5 });
  assert.equal(line(due(NOW + 3600), 3 * 3600), "Flat for 3h; closes in 1h");
  assert.equal(line(due(NOW - 1), 4 * 3600), "Flat for 4h; closes on the next check");
});

test("a due watch naming no known rule shows a dash", () => {
  assert.equal(line({ status: "due", rule: "unknown", due_at: NOW, pnl_pct: 0 }), "—");
});
