/**
 * Tests for the dashboard's shared boost module (`core/boosts.js`).
 *
 * This module is the ONE client-side answer to "is this token boosted, and how
 * strongly" — the featured row, the featured dialog and every token table read it.
 * A disagreement here shows up as the same token reading gold on one surface and
 * plain on another, which is the first thing a paying owner would notice, so the
 * tier rules, the feed parsing and the change signal are all covered.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

const MODULE = "../../src/webserver/templates/scripts/core/boosts.js";

/**
 * A fresh module instance per test: the boost map is module-level state, so tests
 * that load a feed would otherwise leak into the ones that assert an empty map.
 */
async function freshModule() {
  return import(`${MODULE}?t=${Math.random()}`);
}

/** Minimal `window` + `fetch` stand-ins; the module touches nothing else. */
function installBrowserGlobals() {
  const events = [];
  globalThis.window = {
    dispatchEvent: (event) => events.push(event.type),
  };
  globalThis.CustomEvent = class CustomEvent {
    constructor(type) {
      this.type = type;
    }
  };
  return events;
}

function stubFeed(tokens, { ok = true, success = true } = {}) {
  globalThis.fetch = async () => ({
    ok,
    json: async () => ({ success, tokens }),
  });
}

test("boostTier reads a card's own standing", async () => {
  const { boostTier } = await freshModule();
  assert.equal(boostTier({ boosts: 500, golden: true }), "golden");
  assert.equal(boostTier({ boosts: 10, golden: false }), "boosted");
  assert.equal(boostTier({ boosts: 0, golden: false }), null);
  assert.equal(boostTier(null), null);
});

test("a zero-boost token is never gold, even if the feed flags it golden", async () => {
  const { boostTier } = await freshModule();
  // `golden` is a threshold on a count. Without an active boost there is no
  // paid standing to show, and honouring the flag alone would gold an organic row.
  assert.equal(boostTier({ boosts: 0, golden: true }), null);
});

test("formatBoostCount prints the pack label, and nothing for no boost", async () => {
  const { formatBoostCount } = await freshModule();
  assert.equal(formatBoostCount(500), "500x");
  assert.equal(formatBoostCount(1), "1x");
  assert.equal(formatBoostCount(0), "");
  assert.equal(formatBoostCount(undefined), "");
});

test("an unloaded feed marks nothing", async () => {
  const { boostTierForMint, boostRowClass, boostCountForMint } = await freshModule();
  assert.equal(boostTierForMint("mint1"), null);
  assert.equal(boostRowClass("mint1"), "");
  assert.equal(boostCountForMint("mint1"), 0);
});

test("loading the feed marks the right rows at the right tier", async () => {
  installBrowserGlobals();
  stubFeed([
    { mint: "golden1", boosts: 500, golden: true },
    { mint: "plain1", boosts: 30, golden: false },
  ]);

  const { loadBoosts, boostRowClass, boostCountForMint } = await freshModule();
  await loadBoosts();

  assert.equal(boostRowClass("golden1"), "boosted-row golden-row");
  assert.equal(boostRowClass("plain1"), "boosted-row");
  assert.equal(boostRowClass("organic"), "");
  assert.equal(boostCountForMint("golden1"), 500);
  assert.equal(boostCountForMint("organic"), 0);
});

test("feed rows without a mint or an active boost are dropped", async () => {
  installBrowserGlobals();
  stubFeed([
    { mint: "", boosts: 40, golden: false },
    { mint: "expired", boosts: 0, golden: false },
    { mint: "live", boosts: 40, golden: false },
  ]);

  const { loadBoosts, boostRowClass } = await freshModule();
  await loadBoosts();

  assert.equal(boostRowClass("expired"), "");
  assert.equal(boostRowClass("live"), "boosted-row");
});

test("the change event fires only when the standing actually changed", async () => {
  const events = installBrowserGlobals();
  stubFeed([{ mint: "a", boosts: 10, golden: false }]);

  const { loadBoosts, BOOSTS_CHANGED_EVENT, boostCountForMint } = await freshModule();
  await loadBoosts();
  assert.deepEqual(events, [BOOSTS_CHANGED_EVENT]);

  // Same feed, forced re-read: surfaces must not repaint every row for nothing.
  await loadBoosts({ force: true });
  assert.deepEqual(events, [BOOSTS_CHANGED_EVENT]);

  // A bigger pack on the same mint IS a change.
  stubFeed([{ mint: "a", boosts: 500, golden: true }]);
  await loadBoosts({ force: true });
  assert.deepEqual(events, [BOOSTS_CHANGED_EVENT, BOOSTS_CHANGED_EVENT]);
  assert.equal(boostCountForMint("a"), 500);
});

test("the TTL collapses a burst of surfaces into one request", async () => {
  installBrowserGlobals();
  let calls = 0;
  globalThis.fetch = async () => {
    calls += 1;
    return { ok: true, json: async () => ({ success: true, tokens: [] }) };
  };

  const { loadBoosts, ensureBoosts } = await freshModule();
  await loadBoosts();
  await loadBoosts();
  ensureBoosts();
  await loadBoosts();
  assert.equal(calls, 1);
});

test("a failing feed keeps the last known standing instead of clearing it", async () => {
  installBrowserGlobals();
  stubFeed([{ mint: "a", boosts: 10, golden: false }]);

  const { loadBoosts, boostRowClass } = await freshModule();
  await loadBoosts();
  assert.equal(boostRowClass("a"), "boosted-row");

  // The app must work offline; a boost is decoration on top of it.
  globalThis.fetch = async () => {
    throw new Error("offline");
  };
  await loadBoosts({ force: true });
  assert.equal(boostRowClass("a"), "boosted-row");

  stubFeed([], { ok: false });
  await loadBoosts({ force: true });
  assert.equal(boostRowClass("a"), "boosted-row");

  stubFeed([], { success: false });
  await loadBoosts({ force: true });
  assert.equal(boostRowClass("a"), "boosted-row");
});

test("an empty successful feed clears every mark", async () => {
  installBrowserGlobals();
  stubFeed([{ mint: "a", boosts: 10, golden: false }]);

  const { loadBoosts, boostRowClass } = await freshModule();
  await loadBoosts();

  // A boost EXPIRING is a real state change, not a failure — the gold must go.
  stubFeed([]);
  await loadBoosts({ force: true });
  assert.equal(boostRowClass("a"), "");
});
