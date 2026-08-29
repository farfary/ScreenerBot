/**
 * Tests for the external-agent approval prompt module (`core/agent_approvals.js`).
 *
 * The DOM-coupled bootstrap (Poller + ConfirmationDialog) only runs in a
 * browser, so importing the module under node exercises just the pure decision
 * helpers. Those helpers are what make the module safe:
 *   - a request is remembered only after a decision actually lands, so a
 *     network/5xx failure re-offers it instead of silently dropping it;
 *   - a failed request goes into a short backoff rather than an immediate
 *     re-prompt loop.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

const MODULE = "../../src/webserver/templates/scripts/core/agent_approvals.js";

async function mod() {
  return import(`${MODULE}?t=${Math.random()}`);
}

test("importing the module in node does not start the browser bootstrap", async () => {
  // No window/document here — the module must import cleanly with only helpers.
  const m = await mod();
  assert.equal(typeof m.selectNewPending, "function");
  assert.equal(typeof m.decisionSettled, "function");
  assert.equal(typeof m.expiryText, "function");
});

test("decisionSettled: ok or 409 is terminal, everything else is transient", async () => {
  const { decisionSettled } = await mod();
  assert.equal(decisionSettled({ ok: true, status: 200 }), true);
  assert.equal(decisionSettled({ ok: false, status: 409 }), true);
  assert.equal(decisionSettled({ ok: false, status: 500 }), false);
  assert.equal(decisionSettled({ ok: false, status: 0 }), false);
  assert.equal(decisionSettled(null), false);
});

test("selectNewPending skips handled, queued and backed-off ids", async () => {
  const { selectNewPending } = await mod();
  const rows = [
    { id: "a", tool: "buy_token" },
    { id: "b", tool: "sell_token" },
    { id: "c", tool: "close_position" },
    { id: "d", tool: "buy_token" },
    { bad: true },
  ];
  const handled = new Set(["a"]);
  const queued = new Set(["b"]);
  const deferredUntil = new Map([["c", 10_000]]);

  const pick = selectNewPending(rows, { handled, queued, deferredUntil, nowMs: 5_000 });
  assert.deepEqual(
    pick.map((r) => r.id),
    ["d"],
  );

  // Once the backoff window passes, "c" is offered again.
  const later = selectNewPending(rows, { handled, queued, deferredUntil, nowMs: 20_000 });
  assert.deepEqual(
    later.map((r) => r.id).sort(),
    ["c", "d"],
  );
});

test("selectNewPending tolerates a non-array payload", async () => {
  const { selectNewPending } = await mod();
  const args = { handled: new Set(), queued: new Set(), deferredUntil: new Map() };
  assert.deepEqual(selectNewPending(null, args), []);
  assert.deepEqual(selectNewPending({ error: "x" }, args), []);
});

test("expiryText renders minutes then seconds", async () => {
  const { expiryText } = await mod();
  assert.equal(expiryText(400, 0), "expires in 7m");
  assert.equal(expiryText(30, 0), "expires in 30s");
  assert.equal(expiryText(-100, 0), "expires in 0s");
});
