// Status Bar - Fetches and displays system metrics
// Non-module script for immediate execution

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

  function formatUptime(seconds) {
    if (!seconds || seconds < 0) return "—";
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    if (h > 0) return `${h}h ${m}m`;
    if (m > 0) return `${m}m`;
    return "<1m";
  }

  function formatMemory(mb) {
    if (!mb || mb < 0) return "—";
    if (mb >= 1024) return `${(mb / 1024).toFixed(1)}GB`;
    return `${Math.round(mb)}MB`;
  }

  function formatLatency(ms) {
    if (!ms || ms < 0) return "—";
    if (ms >= 1000) return `${(ms / 1000).toFixed(2)}s`;
    return `${Math.round(ms)}ms`;
  }

  function updateDisplay(data) {
    // Version
    if (elements.version && data.version) {
      elements.version.textContent = data.version;
    }

    // Uptime
    if (elements.uptime && typeof data.uptime_seconds === "number") {
      elements.uptime.textContent = formatUptime(data.uptime_seconds);
    }

    // Memory
    if (elements.memory && data.metrics) {
      const memMB = data.metrics.process_memory_mb || data.metrics.memory_usage_mb;
      elements.memory.textContent = formatMemory(memMB);
    }

    // RPC Stats
    if (data.rpc_stats) {
      const rpc = data.rpc_stats;

      // RPC Rate (calls per minute)
      if (elements.rpcRate) {
        const rate = rpc.recent_calls_per_minute || 0;
        elements.rpcRate.textContent = `${Math.round(rate)}/min`;
      }

      // RPC Success Rate
      if (elements.rpcSuccess && elements.rpcHealth) {
        // success_rate is already 0-1 range, multiply by 100 for percentage
        const successRate = (rpc.success_rate || 0) * 100;
        // Cap at 100% to avoid display issues
        const displayRate = Math.min(successRate, 100);
        elements.rpcSuccess.textContent = `${displayRate.toFixed(1)}%`;

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
        elements.rpcLatency.textContent = formatLatency(latency);
      }
    }

    // Trading Status
    if (elements.trading) {
      const isRunning = data.trader_running || false;
      const isEnabled = data.trading_enabled || false;
      const active = isRunning && isEnabled;

      elements.trading.textContent = active ? "Active" : "Inactive";
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

  // Initialize when DOM is ready
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
