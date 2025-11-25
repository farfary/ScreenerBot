// Bootstrap Manager - coordinates backend readiness before heavy dashboard work
const state = {
  ready: false,
  status: null,
  lastError: null,
};

const subscribers = new Set();
let resolveReady;

const readyPromise = new Promise((resolve) => {
  resolveReady = resolve;
});

function notify(status) {
  state.status = status;
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

function markReady(status) {
  if (state.ready) {
    return;
  }
  state.ready = true;

  // Set global flag for Tauri to detect frontend ready state
  window.__screenerbot_ready = true;

  if (typeof resolveReady === "function") {
    resolveReady(status);
  }
  window.dispatchEvent(
    new CustomEvent("screenerbot:ready", {
      detail: status,
    })
  );
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
    const readyForRequests = Boolean(data?.ready_for_requests);
    const readyFlag = initializationRequired || uiReady;

    if (readyFlag) {
      markReady(data);
    }

    state.lastError = null;
    return {
      ready: readyFlag,
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

async function startPolling() {
  let retryMs = 750;
  while (!state.ready) {
    const { ready, retryAfter } = await pollStatus();
    if (state.ready || ready) {
      break;
    }
    retryMs = clamp(retryAfter || retryMs, 500, 4000);
    await delay(retryMs);
  }
}

startPolling().catch((error) => {
  console.error("[Bootstrap] Unexpected failure", error);
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
    status: state.status,
    lastError: state.lastError,
  };
}
