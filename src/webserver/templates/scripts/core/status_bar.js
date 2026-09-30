// Status Bar - Fetches and displays system metrics

import { whenInitialized } from "./bootstrap.js";
import { formatLatencyMs, formatMemoryMb, formatNumber, formatPercentValue, formatUptime } from "./format.js";

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

    // RPC Stats
    if (data.rpc_stats) {
      const rpc = data.rpc_stats;

      // RPC Rate (calls per minute)
      if (elements.rpcRate) {
        const rate = rpc.recent_calls_per_minute || 0;
        elements.rpcRate.textContent = I18n.t("shell-status-rpc-per-minute", { rate: formatNumber(rate, 0) });
      }

      // RPC Success Rate
      if (elements.rpcSuccess && elements.rpcHealth) {
        // success_rate is a 0-100 percentage; scaling it again pinned the badge
        // at 100% and "good" however many calls were failing.
        const successRate = Number(rpc.success_rate);
        const displayRate = Number.isFinite(successRate)
          ? Math.min(Math.max(successRate, 0), 100)
          : 0;
        elements.rpcSuccess.textContent = formatPercentValue(displayRate, { decimals: 1, plus: "" });

        // Set health indicator
        let health = "unknown";
        if (displayRate >= 95) health = "good";
        else if (displayRate >= 80) health = "warning";
        else health = "error";
        elements.rpcHealth.setAttribute("data-health", health);
      }

      // RPC Latency
      if (elements.rpcLatency) {
        const latency = rpc.average_response_time_ms || 0;
        elements.rpcLatency.textContent = latency > 0 ? formatLatencyMs(latency) : "—";
      }
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
    if (elements.positions && typeof data.open_positions === "number") {
      elements.positions.textContent = data.open_positions;
    }

    // Tokens Count (from wallet if available)
    if (elements.tokens && data.wallet) {
      const tokenCount = data.wallet.total_tokens_count || 0;
      elements.tokens.textContent = tokenCount;
    }
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
