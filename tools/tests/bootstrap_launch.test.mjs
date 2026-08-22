/**
 * Tests for the dashboard's launch loader (`core/bootstrap.js`).
 *
 * This module decides when the window may stop showing its loading state, and
 * it is the last thing standing between a broken backend and a dashboard that
 * sits in its skeleton forever. It used to poll `/api/system/bootstrap` in an
 * unbounded `while (!ready)` loop: when the backend died mid-boot (a failed
 * one-time token-database migration), the loop simply kept retrying a dead
 * endpoint and `waitForReady()` never fulfilled, so the router never ran and
 * nothing on screen ever said why.
 *
 * So what is covered here is termination: every branch must reach a terminal
 * outcome, and each outcome must be the right one.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

const MODULE = "../../src/webserver/templates/scripts/core/bootstrap.js";

/**
 * The module starts polling at import time and keeps module-level launch state,
 * so every test needs its own instance with its own stubbed backend.
 */
async function freshModule() {
  return import(`${MODULE}?t=${Math.random()}`);
}

/** Minimal browser globals; the loader touches nothing else. */
function installBrowserGlobals() {
  const events = [];
  globalThis.window = {
    dispatchEvent: (event) => {
      events.push(event);
      return true;
    },
  };
  globalThis.CustomEvent = class CustomEvent {
    constructor(type, init) {
      this.type = type;
      this.detail = init?.detail;
    }
  };
  globalThis.AbortController = class AbortController {
    constructor() {
      this.signal = {};
    }
    abort() {}
  };
  return events;
}

/** Answer every poll with one boot-status payload. */
function stubBootStatus(payload) {
  globalThis.fetch = async () => ({
    ok: true,
    status: 200,
    statusText: "OK",
    json: async () => payload,
  });
}

const RUNNING = {
  ready: false,
  elapsedMs: 5_000,
  failingForMs: 0,
  hasStatus: true,
};

test("a launch inside both budgets keeps polling", async () => {
  installBrowserGlobals();
  stubBootStatus({ ui_ready: true });
  const { evaluateLaunch } = await freshModule();

  assert.equal(evaluateLaunch(RUNNING), null);
  assert.equal(evaluateLaunch({ ...RUNNING, elapsedMs: 119_000 }), null);
  assert.equal(evaluateLaunch({ ...RUNNING, failingForMs: 19_000 }), null);
});

test("a backend that stops answering settles as unreachable", async () => {
  installBrowserGlobals();
  stubBootStatus({ ui_ready: true });
  const { evaluateLaunch, BOOTSTRAP_OUTCOME } = await freshModule();

  // Well inside the ready deadline — a dead backend must not be waited out.
  assert.equal(
    evaluateLaunch({ ready: false, elapsedMs: 21_000, failingForMs: 20_000, hasStatus: true }),
    BOOTSTRAP_OUTCOME.UNREACHABLE
  );
});

test("a backend that answers but never reports ready settles as degraded", async () => {
  installBrowserGlobals();
  stubBootStatus({ ui_ready: true });
  const { evaluateLaunch, BOOTSTRAP_OUTCOME } = await freshModule();

  assert.equal(
    evaluateLaunch({ ready: false, elapsedMs: 120_000, failingForMs: 0, hasStatus: true }),
    BOOTSTRAP_OUTCOME.DEGRADED
  );
});

test("a launch that never got one answer settles as unreachable, not degraded", async () => {
  installBrowserGlobals();
  stubBootStatus({ ui_ready: true });
  const { evaluateLaunch, BOOTSTRAP_OUTCOME } = await freshModule();

  // Degraded means "load against partial data"; with no payload there is none,
  // so this has to be reported as a failed launch instead.
  assert.equal(
    evaluateLaunch({ ready: false, elapsedMs: 120_000, failingForMs: 0, hasStatus: false }),
    BOOTSTRAP_OUTCOME.UNREACHABLE
  );
});

test("ui_ready releases the loader and resolves waitForReady", async () => {
  const events = installBrowserGlobals();
  stubBootStatus({ ui_ready: true, phase: "ready", initialization_required: false });
  const { waitForReady, getBootstrapState, BOOTSTRAP_OUTCOME } = await freshModule();

  const status = await waitForReady();

  assert.equal(status.ui_ready, true);
  assert.equal(getBootstrapState().ready, true);
  assert.equal(getBootstrapState().settled, true);
  assert.equal(getBootstrapState().outcome, BOOTSTRAP_OUTCOME.READY);
  assert.ok(
    events.some((event) => event.type === "screenerbot:ready"),
    "a ready launch must announce itself"
  );
  assert.equal(globalThis.window.__screenerbot_ready, true);
});

test("a first run needing setup releases the loader too", async () => {
  installBrowserGlobals();
  // Setup has never been completed: nothing is ready and nothing will be, but
  // the loader must hand the router the setup screen instead of waiting.
  stubBootStatus({ initialization_required: true, ui_ready: false, onboarding_complete: true });
  const { waitForReady } = await freshModule();

  const status = await waitForReady();

  assert.equal(status.initialization_required, true);
});

test("the loader announces its terminal outcome for the splash to render", async () => {
  const events = installBrowserGlobals();
  stubBootStatus({ ui_ready: true });
  const { waitForReady } = await freshModule();

  await waitForReady();

  const settled = events.find((event) => event.type === "screenerbot:bootstrap-settled");
  assert.ok(settled, "every launch must announce a terminal outcome exactly once");
  assert.equal(settled.detail.outcome, "ready");
  assert.equal(
    events.filter((event) => event.type === "screenerbot:bootstrap-settled").length,
    1
  );
});
