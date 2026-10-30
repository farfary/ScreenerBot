// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Status Bar - Fetches and displays system metrics

import { whenInitialized } from "./bootstrap.js";
import { capabilityAvailable } from "../ui/access_gate.js";
import { RPC_METRICS_EVENT } from "./header_metrics.js";
import {
  formatCallRate,
  formatLatencyMs,
  formatMemoryMb,
  formatSuccessRate,
  formatUptime,
} from "./format.js";

(function () {
  "use strict";

  // Cache and poll interval
  let pollInterval = null;
  const POLL_INTERVAL_MS = 5000; // 5 seconds

  // DOM element references
  const elements = {
    version: null,
    uptime: null,
    memory: null,
    rpcRate: null,
    rpcSuccess: null,
    rpcLatency: null,
    rpcHealth: null,
    trading: null,
    positions: null,
    tokens: null,
  };

  function cacheElements() {
    elements.version = document.getElementById("statusBarVersion");
    elements.uptime = document.getElementById("statusBarUptime");
    elements.memory = document.getElementById("statusBarMemory");
    elements.rpcRate = document.getElementById("statusBarRpcRate");
    elements.rpcSuccess = document.getElementById("statusBarRpcSuccess");
    elements.rpcLatency = document.getElementById("statusBarRpcLatency");
    elements.rpcHealth = document.getElementById("statusBarRpcHealth");
    elements.trading = document.getElementById("statusBarTrading");
    elements.positions = document.getElementById("statusBarPositions");
    elements.tokens = document.getElementById("statusBarTokens");
  }

  function updateDisplay(data) {
    // Version
    if (elements.version && data.version) {
      elements.version.textContent = data.version;
    }

    // Uptime
    if (elements.uptime && typeof data.uptime_seconds === "number") {
      elements.uptime.textContent =
        data.uptime_seconds > 0 ? formatUptime(data.uptime_seconds, { style: "hm" }) : "—";
    }

    // Memory
    if (elements.memory && data.metrics) {
      const memMB = data.metrics.process_memory_mb || data.metrics.memory_usage_mb;
      elements.memory.textContent = memMB > 0 ? formatMemoryMb(memMB) : "—";
    }

    // Trading Status
    if (elements.trading) {
      const isRunning = data.trader_running || false;
      const isEnabled = data.trading_enabled || false;
      const active = isRunning && isEnabled;

      elements.trading.textContent = active
        ? I18n.t("shell-status-bar-trading-active")
        : I18n.t("shell-status-bar-trading-inactive");
      elements.trading.setAttribute("data-active", active ? "true" : "false");
    }

    // Open Positions
    // Null when the positions store could not be read: unknown, never zero.
    if (elements.positions) {
      elements.positions.textContent =
        typeof data.open_positions === "number" ? data.open_positions : "—";
    }

    // Tokens Count (from wallet if available)
    if (elements.tokens && data.wallet) {
      const tokenCount = data.wallet.total_tokens_count || 0;
      elements.tokens.textContent = tokenCount;
    }
  }

  // RPC figures arrive with the header's answer (`RPC_METRICS_EVENT`), never from
  // `/api/status`: two polls sample the trailing-minute rate at different moments.
  // The header can answer before this bar initializes, so the latest sample is kept.
  let latestRpc;
  window.addEventListener(RPC_METRICS_EVENT, (event) => {
    latestRpc = event.detail;
    updateRpc(latestRpc);
  });

  function updateRpc(rpc) {
    // The ScreenerBot service keeps no RPC statistics, so the group hides there.
    const rpcGroup = elements.rpcRate?.closest(".status-bar-center");
    if (rpcGroup) rpcGroup.hidden = !capabilityAvailable("rpc_statistics");

    if (elements.rpcRate) {
      elements.rpcRate.textContent = formatCallRate(rpc?.calls_per_minute ?? null);
    }

    if (elements.rpcSuccess && elements.rpcHealth) {
      // A 0-100 percentage, absent before the first call: then "—", never 100%.
      const successRate = rpc?.success_rate_percent;
      elements.rpcSuccess.textContent = formatSuccessRate(successRate);

      // Health indicator; it stays "unknown" until there is a rate to judge.
      let health = "unknown";
      if (Number.isFinite(successRate)) {
        if (successRate >= 95) health = "good";
        else if (successRate >= 80) health = "warning";
        else health = "error";
      }
      elements.rpcHealth.setAttribute("data-health", health);
    }

    if (elements.rpcLatency) {
      const latency = rpc?.avg_latency_ms || 0;
      elements.rpcLatency.textContent = latency > 0 ? formatLatencyMs(latency) : "—";
    }
  }

  // While the core is unreachable every live figure reads "—"; the version is not live.
  function markUnknown() {
    const live = [
      "uptime",
      "memory",
      "rpcRate",
      "rpcSuccess",
      "rpcLatency",
      "trading",
      "positions",
      "tokens",
    ];
    for (const key of live) {
      if (elements[key]) elements[key].textContent = "—";
    }
    if (elements.rpcHealth) elements.rpcHealth.setAttribute("data-health", "unknown");
    if (elements.trading) elements.trading.removeAttribute("data-active");
  }

  async function fetchStatusData() {
    try {
      const response = await fetch("/api/status");
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const result = await response.json();
      const data = result.data || result;
      updateDisplay(data);
    } catch (err) {
      console.warn("Status bar update failed:", err);
    }
  }

  function startPolling() {
    // Initial fetch
    fetchStatusData();

    // Poll every 5 seconds
    if (pollInterval) clearInterval(pollInterval);
    pollInterval = setInterval(fetchStatusData, POLL_INTERVAL_MS);
  }

  function stopPolling() {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  }

  function init() {
    cacheElements();
    if (latestRpc !== undefined) updateRpc(latestRpc);
    startPolling();

    // Stop polling when page is hidden to save resources
    document.addEventListener("visibilitychange", () => {
      if (document.hidden) {
        stopPolling();
      } else {
        startPolling();
      }
    });

    // Cleanup on page unload
    window.addEventListener("beforeunload", stopPolling);

    // Events of core/connectivity_watcher.js.
    window.addEventListener("screenerbot:offline", markUnknown);
    window.addEventListener("screenerbot:reconnected", fetchStatusData);
  }

  // The status bar is the one always-on refresh that is not a `Poller`, so it
  // cannot be quieted through the poller registry. This is its control surface:
  // used by Promo Studio to hold the bar still for a screenshot, and by anything
  // else that needs the dashboard to stop changing under it.
  window.StatusBar = {
    pausePolling: stopPolling,
    resumePolling: startPolling,
  };

  // Poll only an initialized backend; a first-run launch answers 503 until setup completes.
  whenInitialized(() => {
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", init);
    } else {
      init();
    }
  });
})();
