// Bootstrap Manager - coordinates backend readiness before heavy dashboard work

// Terminal outcomes. The loader must always reach one of these: it used to poll
// `/api/system/bootstrap` in an unbounded `while (!ready)` loop, so a backend
// that died mid-boot — or one grinding through a one-time database migration
// that never reports ready — left the dashboard sitting in its skeleton forever
// with nothing on screen to act on.
export const BOOTSTRAP_OUTCOME = {
  READY: "ready",
  DEGRADED: "degraded",
  UNREACHABLE: "unreachable",
};

// Longest a launch may stay un-settled while the backend still answers. Past
// this the dashboard loads against whatever the API can serve (every page owns
// its own empty/error state) instead of waiting on a service that may never
// report ready.
const READY_DEADLINE_MS = 120000;
// How long the backend may be continuously unreachable before the launch is
// declared failed. A dead backend answers nothing, so waiting out the full
// ready deadline would only delay the error the user needs to see.
const UNREACHABLE_GRACE_MS = 20000;

const state = {
  ready: false,
  settled: false,
  outcome: null,
  status: null,
  lastError: null,
  // Last payload the backend actually returned, kept across failed polls so a
  // degraded settle can still hand consumers real boot state.
  lastGoodStatus: null,
  firstFailureAt: null,
};

const subscribers = new Set();
let resolveReady;

const readyPromise = new Promise((resolve) => {
  resolveReady = resolve;
});

function notify(status) {
  state.status = status;
  if (status) {
    state.lastGoodStatus = status;
  }
  subscribers.forEach((callback) => {
    try {
      callback(status);
    } catch (error) {
      console.error("[Bootstrap] Subscriber error", error);
    }
  });

  window.dispatchEvent(
    new CustomEvent("screenerbot:bootstrap-status", {
      detail: status,
    })
  );
}

/**
 * Settle the launch exactly once. Every outcome resolves `waitForReady()` so no
 * consumer can be left awaiting a promise that never fulfils.
 */
function settle(outcome, status) {
  if (state.settled) {
    return;
  }
  state.settled = true;
  state.outcome = outcome;
  state.ready = outcome === BOOTSTRAP_OUTCOME.READY;

  // Electron polls this flag to detect a fully loaded frontend; only a genuine
  // ready launch may claim it.
  window.__screenerbot_ready = state.ready;

  if (typeof resolveReady === "function") {
    resolveReady(status || null);
  }

  if (state.ready) {
    window.dispatchEvent(
      new CustomEvent("screenerbot:ready", {
        detail: status,
      })
    );
  } else {
    console.warn(`[Bootstrap] Launch settled as ${outcome}`);
  }

  window.dispatchEvent(
    new CustomEvent("screenerbot:bootstrap-settled", {
      detail: {
        outcome,
        status: status || null,
        error: state.lastError ? state.lastError.message || String(state.lastError) : null,
      },
    })
  );
}

function markReady(status) {
  settle(BOOTSTRAP_OUTCOME.READY, status);
}

async function pollStatus() {
  const controller = new AbortController();
  try {
    const response = await fetch("/api/system/bootstrap", {
      method: "GET",
      cache: "no-store",
      headers: {
        "X-Requested-With": "fetch",
      },
      signal: controller.signal,
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const data = await response.json();
    notify(data);

    const initializationRequired = Boolean(data?.initialization_required);
    const uiReady = Boolean(data?.ui_ready);
    const readyFlag = initializationRequired || uiReady;

    if (readyFlag) {
      markReady(data);
    }

    state.lastError = null;
    return {
      ready: readyFlag,
      failed: false,
      retryAfter: Number(data?.retry_after_ms) || 750,
    };
  } catch (error) {
    state.lastError = error;
    console.warn("[Bootstrap] Status check failed", error);
    notify(null);
    window.dispatchEvent(
      new CustomEvent("screenerbot:bootstrap-error", {
        detail: error?.message || String(error),
      })
    );
    return {
      ready: false,
      failed: true,
      retryAfter: 1500,
    };
  } finally {
    controller.abort();
  }
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function clamp(value, min, max) {
  return Math.min(Math.max(value, min), max);
}

/**
 * The launch state machine, kept pure so every branch is provable without
 * timers: given how long the launch has run, how long the backend has been
 * unreachable, and whether any status has ever arrived, decide whether the
 * launch is over. Returns null only while polling may legitimately continue.
 *
 * A backend that still answers but never reports ready is DEGRADED — the
 * dashboard loads against what the API can serve, every page owning its own
 * empty state. A backend that stops answering is UNREACHABLE, which is an
 * actionable failure, not something to keep waiting on.
 */
export function evaluateLaunch({ ready, elapsedMs, failingForMs, hasStatus }) {
  if (ready) {
    return BOOTSTRAP_OUTCOME.READY;
  }
  if (failingForMs >= UNREACHABLE_GRACE_MS) {
    return BOOTSTRAP_OUTCOME.UNREACHABLE;
  }
  if (elapsedMs >= READY_DEADLINE_MS) {
    return hasStatus ? BOOTSTRAP_OUTCOME.DEGRADED : BOOTSTRAP_OUTCOME.UNREACHABLE;
  }
  return null;
}

async function startPolling() {
  const startedAt = Date.now();
  let retryMs = 750;

  while (!state.settled) {
    const { ready, failed, retryAfter } = await pollStatus();
    if (state.settled || ready) {
      break;
    }

    if (failed) {
      if (state.firstFailureAt === null) {
        state.firstFailureAt = Date.now();
      }
    } else {
      state.firstFailureAt = null;
    }

    const now = Date.now();
    const outcome = evaluateLaunch({
      ready: false,
      elapsedMs: now - startedAt,
      failingForMs: state.firstFailureAt === null ? 0 : now - state.firstFailureAt,
      hasStatus: Boolean(state.lastGoodStatus),
    });
    if (outcome) {
      settle(outcome, outcome === BOOTSTRAP_OUTCOME.DEGRADED ? state.lastGoodStatus : null);
      break;
    }

    retryMs = clamp(retryAfter || retryMs, 500, 4000);
    await delay(retryMs);
  }
}

startPolling().catch((error) => {
  console.error("[Bootstrap] Unexpected failure", error);
  // An unexpected throw must not strand the launch either.
  settle(BOOTSTRAP_OUTCOME.UNREACHABLE, null);
});

export function waitForReady() {
  return readyPromise;
}

export function subscribeToBootstrap(callback) {
  if (typeof callback !== "function") {
    return () => {};
  }
  subscribers.add(callback);
  if (state.status !== null) {
    try {
      callback(state.status);
    } catch (error) {
      console.error("[Bootstrap] Subscriber callback failed", error);
    }
  }
  return () => subscribers.delete(callback);
}

export function getBootstrapState() {
  return {
    ready: state.ready,
    settled: state.settled,
    outcome: state.outcome,
    status: state.status,
    lastError: state.lastError,
  };
}
