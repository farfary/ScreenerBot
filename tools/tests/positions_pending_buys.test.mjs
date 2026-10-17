// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a finished buy whose position already loaded is never pending again.
 *
 * The Positions table keeps a finished buy as a pending row for a grace window until its
 * position loads. A position that loaded and then left the open view (closed moments after
 * the buy) must not bring the "buying" row back.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import {
  buyHasLoadedPosition,
  rememberEntries,
} from "../../src/webserver/templates/scripts/pages/positions/pending_buys.js";

const STARTED_AT = Date.parse("2026-10-17T11:06:58Z");

test("a finished buy whose position loaded is not pending again", () => {
  const latest = new Map();
  rememberEntries(latest, [{ mint: "mint", entry_time: STARTED_AT / 1000 + 15 }]);

  assert.equal(buyHasLoadedPosition({ kind: "completed", startedAt: STARTED_AT }, latest.get("mint")), true);
  assert.equal(buyHasLoadedPosition({ kind: "failed", startedAt: STARTED_AT }, latest.get("mint")), true);
});

test("an earlier position of the mint does not stand in for a new buy", () => {
  const latest = new Map();
  rememberEntries(latest, [{ mint: "mint", entry_time: STARTED_AT / 1000 - 3600 }]);

  assert.equal(buyHasLoadedPosition({ kind: "completed", startedAt: STARTED_AT }, latest.get("mint")), false);
});

test("an in-progress buy is always pending", () => {
  const latest = new Map();
  rememberEntries(latest, [{ mint: "mint", entry_time: STARTED_AT / 1000 + 15 }]);

  assert.equal(buyHasLoadedPosition({ kind: "active", startedAt: STARTED_AT }, latest.get("mint")), false);
});

test("the latest entry per mint is kept and unreadable rows are ignored", () => {
  const latest = new Map();
  rememberEntries(latest, [
    { mint: "mint", entry_time: 200 },
    { mint: "mint", entry_time: 100 },
    { mint: "", entry_time: 300 },
    { mint: "other", entry_time: "not a time" },
  ]);

  assert.deepEqual([...latest.entries()], [["mint", 200_000]]);
  assert.equal(buyHasLoadedPosition({ kind: "completed", startedAt: Number.NaN }, 200_000), false);
});
