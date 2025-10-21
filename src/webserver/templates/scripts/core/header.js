// Header controls for global dashboard interactions (trader toggle)
import { Poller, getInterval as getGlobalPollingInterval } from "./poller.js";
import * as Utils from "./utils.js";

const MIN_STATUS_POLL_INTERVAL = 5000;

const state = {
  enabled: false,
  running: false,
  available: false,
  loading: false,
  fetching: false,
};

let statusPoller = null;
let currentStatusPromise = null;
let currentController = null;

function getElements() {
  return {
    toggle: document.getElementById("traderToggle"),
    icon: document.getElementById("traderIcon"),
    text: document.getElementById("traderText"),
    badge: document.getElementById("botBadge"),
  };
}

function updateBadge({ badge }) {
  if (!badge) {
    return;
  }

  badge.classList.remove("loading", "success", "warning", "error");

  if (state.loading) {
    badge.classList.add("loading");
    badge.textContent = "🤖 LOADING";
    return;
  }

  if (!state.available) {
    badge.classList.add("warning");
    badge.textContent = "🤖 UNKNOWN";
    return;
  }

  if (state.running) {
    badge.classList.add("success");
    badge.textContent = "🤖 RUNNING";
  } else {
    badge.classList.add("warning");
    badge.textContent = "🤖 STOPPED";
  }
}

function updateToggle({ toggle, icon, text }) {
  if (!toggle || !icon || !text) {
    return;
  }

  toggle.disabled = state.loading || !state.available;
  toggle.setAttribute("aria-busy", state.loading ? "true" : "false");
  toggle.dataset.traderState = state.running ? "running" : "stopped";

  if (state.loading) {
    icon.textContent = "⏳";
    text.textContent = "Updating...";
    return;
  }

  if (!state.available) {
    icon.textContent = "⚠️";
    text.textContent = "Status unavailable";
    return;
  }

  if (state.running) {
    icon.textContent = "⏸️";
    text.textContent = "Stop Trader";
  } else {
    icon.textContent = "▶️";
    text.textContent = "Start Trader";
  }
}

function applyStatus(newStatus) {
  if (!newStatus || typeof newStatus !== "object") {
    return;
  }
  if (typeof newStatus.enabled === "boolean") {
    state.enabled = newStatus.enabled;
  }
  if (typeof newStatus.running === "boolean") {
    state.running = newStatus.running;
  }
  state.available = true;
  const elements = getElements();
  updateToggle(elements);
  updateBadge(elements);
}

function setAvailability(isAvailable) {
  state.available = isAvailable;
  const elements = getElements();
  updateToggle(elements);
  updateBadge(elements);
}

function setLoading(isLoading) {
  state.loading = Boolean(isLoading);
  const elements = getElements();
  updateToggle(elements);
  updateBadge(elements);
}

async function fetchTraderStatus({ silent = false, showLoading = false } = {}) {
  if (state.fetching && currentStatusPromise) {
    return currentStatusPromise;
  }

  const elements = getElements();

  if (!elements.toggle) {
    return null;
  }

  if (showLoading) {
    setLoading(true);
  }

  state.fetching = true;
  const controller = new AbortController();
  if (currentController) {
    currentController.abort();
  }
  currentController = controller;

  const request = fetch("/api/trader/status", {
    method: "GET",
    headers: { "X-Requested-With": "fetch" },
    cache: "no-store",
    signal: controller.signal,
  })
    .then(async (res) => {
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      const data = await res.json();
      applyStatus(data);
      return data;
    })
    .catch((err) => {
      if (err?.name === "AbortError") {
        return null;
      }
      console.error("[TraderHeader] Failed to fetch status", err);
      setAvailability(false);
      if (!silent) {
        Utils.showToast("⚠️ Failed to refresh trader status", "warning");
      }
      return null;
    })
    .finally(() => {
      if (currentController === controller) {
        currentController = null;
      }
      state.fetching = false;
      if (showLoading) {
        setLoading(false);
      }
    });

  currentStatusPromise = request;
  return request;
}

async function controlTrader(action) {
  const elements = getElements();
  if (!elements.toggle) {
    return;
  }

  if (state.loading) {
    return;
  }

  setLoading(true);

  const endpoint = action === "start" ? "/api/trader/start" : "/api/trader/stop";

  try {
    const res = await fetch(endpoint, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Requested-With": "fetch",
      },
      cache: "no-store",
    });

    const payload = await res.json().catch(() => null);

    if (!res.ok) {
      const message = payload?.message || `Trader request failed (${res.status})`;
      throw new Error(message);
    }

    if (payload?.status) {
      applyStatus(payload.status);
    }

    Utils.showToast(action === "start" ? "✅ Trader started" : "✅ Trader stopped", "success");
  } catch (err) {
    console.error("[TraderHeader] Control action failed", err);
    Utils.showToast(`❌ ${err.message || "Trader control failed"}`, "error");
    setAvailability(false);
  } finally {
    setLoading(false);
    fetchTraderStatus({ silent: true });
  }
}

function attachToggleHandler(toggle) {
  if (!toggle) {
    return;
  }

  toggle.addEventListener("click", (event) => {
    event.preventDefault();
    if (!state.available || state.loading) {
      return;
    }

    const action = state.running ? "stop" : "start";
    controlTrader(action);
  });
}

function startStatusPolling() {
  if (statusPoller) {
    statusPoller.cleanup();
  }

  const pollIntervalProvider = () => {
    try {
      const interval = Number(getGlobalPollingInterval());
      if (Number.isFinite(interval) && interval > 0) {
        return Math.max(interval, MIN_STATUS_POLL_INTERVAL);
      }
    } catch (err) {
      console.warn("[TraderHeader] Failed to read polling interval", err);
    }
    return MIN_STATUS_POLL_INTERVAL;
  };

  statusPoller = new Poller(() => fetchTraderStatus({ silent: true }), {
    label: "TraderStatus",
    getInterval: pollIntervalProvider,
  });

  statusPoller.start({ silent: true });
}

function initTraderControls() {
  const elements = getElements();
  if (!elements.toggle) {
    return;
  }

  attachToggleHandler(elements.toggle);
  fetchTraderStatus({ silent: true, showLoading: true }).finally(() => {
    startStatusPolling();
  });
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initTraderControls);
} else {
  initTraderControls();
}
