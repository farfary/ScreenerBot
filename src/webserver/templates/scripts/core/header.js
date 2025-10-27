// Header controls for global dashboard interactions (trader toggle)
import {
  Poller,
  getInterval as getGlobalPollingInterval,
  setInterval as setGlobalPollingInterval,
} from "./poller.js";
import * as Utils from "./utils.js";
import { Dropdown } from "../ui/dropdown.js";

const MIN_STATUS_POLL_INTERVAL = 5000;

const state = {
  enabled: false,
  running: false,
  available: false,
  loading: false,
  fetching: false,
  connected: false,
};

let statusPoller = null;
let currentStatusPromise = null;
let currentController = null;
let powerDropdown = null;
let refreshDropdown = null;
let traderDropdown = null;
let notificationCount = 0;
let connectionCheckInterval = null;

function getElements() {
  return {
    toggle: document.getElementById("traderToggle"),
    icon: document.getElementById("traderIcon"),
    text: document.getElementById("traderText"),
    badge: document.getElementById("botBadge"),
    badgeIcon: document.getElementById("botStatusIcon"),
    badgeText: document.getElementById("botStatusText"),
    connectionStatus: document.getElementById("connectionStatus"),
    connectionIcon: document.getElementById("connectionIcon"),
  };
}

function updateBadge({ badge, badgeIcon, badgeText }) {
  if (!badge || !badgeIcon || !badgeText) {
    return;
  }

  badge.classList.remove("loading", "success", "warning", "error");

  if (state.loading) {
    badge.classList.add("loading");
    badgeIcon.textContent = "🤖";
    badgeText.textContent = "LOADING";
    return;
  }

  if (!state.available) {
    badge.classList.add("warning");
    badgeIcon.textContent = "⚠️";
    badgeText.textContent = "UNKNOWN";
    return;
  }

  if (state.running) {
    badge.classList.add("success");
    badgeIcon.textContent = "✅";
    badgeText.textContent = "RUNNING";
  } else {
    badge.classList.add("warning");
    badgeIcon.textContent = "🛑";
    badgeText.textContent = "STOPPED";
  }
}

function updateToggle({ toggle, icon, text }) {
  if (!toggle || !icon || !text) {
    return;
  }

  toggle.disabled = state.loading || !state.available;
  toggle.setAttribute("aria-busy", state.loading ? "true" : "false");
  toggle.dataset.traderState = state.running ? "running" : "stopped";

  // Update button classes for styling
  toggle.classList.remove("running", "stopped");
  if (state.running) {
    toggle.classList.add("running");
  } else {
    toggle.classList.add("stopped");
  }

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
  updateConnectionStatus(true);
}

function setAvailability(isAvailable) {
  state.available = isAvailable;
  const elements = getElements();
  updateToggle(elements);
  updateBadge(elements);
  updateConnectionStatus(isAvailable);
}

function updateConnectionStatus(isConnected) {
  const elements = getElements();
  if (!elements.connectionStatus || !elements.connectionIcon) {
    return;
  }

  state.connected = isConnected;

  elements.connectionStatus.classList.remove("connected", "disconnected", "connecting");

  if (isConnected) {
    elements.connectionStatus.classList.add("connected");
    elements.connectionIcon.textContent = "🟢";
    elements.connectionStatus.title = "Backend Connected";
  } else {
    elements.connectionStatus.classList.add("disconnected");
    elements.connectionIcon.textContent = "🔴";
    elements.connectionStatus.title = "Backend Disconnected";
  }
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

  // Initialize connection status as connecting
  if (elements.connectionStatus && elements.connectionIcon) {
    elements.connectionStatus.classList.add("connecting");
    elements.connectionIcon.textContent = "🟡";
    elements.connectionStatus.title = "Connecting to Backend...";
  }

  attachToggleHandler(elements.toggle);

  // Initialize power menu dropdown
  initPowerMenu();

  // Initialize refresh interval dropdown
  initRefreshInterval();

  // Initialize trader dropdown (future)
  initTraderDropdown();

  // Initialize notifications
  initNotifications();

  fetchTraderStatus({ silent: true, showLoading: true }).finally(() => {
    startStatusPolling();
  });
}

function initPowerMenu() {
  const powerBtn = document.getElementById("powerMenuBtn");
  if (!powerBtn) return;

  powerDropdown = new Dropdown({
    trigger: powerBtn,
    align: "right",
    items: [
      {
        id: "restart",
        icon: "🔄",
        label: "Restart Bot",
      },
      {
        id: "pause-services",
        icon: "⏸️",
        label: "Pause Services",
        badge: "Soon",
        disabled: true,
      },
      { divider: true },
      {
        id: "shutdown",
        icon: "🛑",
        label: "Shutdown",
        danger: true,
        disabled: true,
        badge: "Soon",
      },
      { divider: true },
      {
        id: "system-info",
        icon: "ℹ️",
        label: "System Info",
        disabled: true,
      },
    ],
    onSelect: handlePowerMenuAction,
  });
}

function initRefreshInterval() {
  const refreshBtn = document.getElementById("refreshIntervalBtn");
  const refreshText = document.getElementById("refreshIntervalText");

  if (!refreshBtn || !refreshText) return;

  // Get current interval from poller
  const currentInterval = getGlobalPollingInterval();

  // Format display text
  const formatInterval = (ms) => {
    if (ms >= 60000) return `${ms / 60000}m`;
    return `${ms / 1000}s`;
  };

  // Update initial display
  refreshText.textContent = formatInterval(currentInterval);

  // Define interval options
  const intervals = [
    { id: "1000", label: "1 second", display: "1s", ms: 1000 },
    { id: "2000", label: "2 seconds", display: "2s", ms: 2000 },
    { id: "3000", label: "3 seconds", display: "3s", ms: 3000 },
    { id: "5000", label: "5 seconds", display: "5s", ms: 5000 },
    { id: "10000", label: "10 seconds", display: "10s", ms: 10000 },
    { id: "15000", label: "15 seconds", display: "15s", ms: 15000 },
    { id: "30000", label: "30 seconds", display: "30s", ms: 30000 },
    { id: "60000", label: "1 minute", display: "1m", ms: 60000 },
  ];

  // Create dropdown items with checkmark for current selection
  const items = intervals.map((interval) => ({
    id: interval.id,
    icon: currentInterval === interval.ms ? "✓" : " ",
    label: interval.label,
  }));

  refreshDropdown = new Dropdown({
    trigger: refreshBtn,
    align: "right",
    items: items,
    onSelect: (action) => {
      const selected = intervals.find((i) => i.id === action);
      if (selected) {
        setGlobalPollingInterval(selected.ms);
        refreshText.textContent = selected.display;
        Utils.showToast(`⏱️ Refresh interval: ${selected.display}`, "success");

        // Recreate dropdown with updated checkmarks
        if (refreshDropdown) {
          refreshDropdown.destroy();
        }

        const updatedItems = intervals.map((interval) => ({
          id: interval.id,
          icon: interval.ms === selected.ms ? "✓" : " ",
          label: interval.label,
        }));

        refreshDropdown = new Dropdown({
          trigger: refreshBtn,
          align: "right",
          items: updatedItems,
          onSelect: arguments.callee,
        });
      }
    },
  });
}

function initTraderDropdown() {
  // Placeholder for future trader dropdown menu
  // Will add: Start, Stop, Restart, View Logs options
}

function initNotifications() {
  const notifBtn = document.getElementById("notificationBtn");

  if (!notifBtn) return;

  notifBtn.addEventListener("click", () => {
    // Placeholder - will show notifications panel in future
    Utils.showToast("📬 Notifications panel coming soon", "info");
  });

  // Update badge count (placeholder)
  updateNotificationBadge(0);
}

function updateNotificationBadge(count) {
  const badge = document.getElementById("notificationBadge");
  if (!badge) return;

  notificationCount = count;
  badge.textContent = count > 99 ? "99+" : count.toString();
  badge.style.display = count > 0 ? "flex" : "none";
}

async function handlePowerMenuAction(action) {
  switch (action) {
    case "restart":
      await handleRestart();
      break;
    case "pause-services":
      Utils.showToast("⏸️ Pause Services feature coming soon", "info");
      break;
    case "shutdown":
      Utils.showToast("🛑 Shutdown feature coming soon", "info");
      break;
    case "system-info":
      Utils.showToast("ℹ️ System Info panel coming soon", "info");
      break;
  }
}

async function handleRestart() {
  const confirmed = window.confirm(
    "Are you sure you want to restart the bot?\n\nThis will:\n• Stop all services\n• Restart the process\n• Take ~10-15 seconds"
  );

  if (!confirmed) return;

  try {
    Utils.showToast("🔄 Restarting bot...", "info");

    const res = await fetch("/api/system/reboot", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });

    if (!res.ok) {
      throw new Error(`Restart failed: ${res.status}`);
    }

    Utils.showToast("✅ Bot restarting... Please wait.", "success");

    // Poll for reconnection
    setTimeout(() => {
      let attempts = 0;
      const checkConnection = setInterval(async () => {
        attempts++;
        try {
          const ping = await fetch("/api/trader/status", { cache: "no-store" });
          if (ping.ok) {
            clearInterval(checkConnection);
            Utils.showToast("✅ Bot restarted successfully!", "success");
            window.location.reload();
          }
        } catch {
          if (attempts > 30) {
            clearInterval(checkConnection);
            Utils.showToast("⚠️ Restart taking longer than expected", "warning");
          }
        }
      }, 1000);
    }, 2000);
  } catch (err) {
    console.error("[Header] Restart failed:", err);
    Utils.showToast(`❌ ${err.message}`, "error");
  }
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initTraderControls);
} else {
  initTraderControls();
}
