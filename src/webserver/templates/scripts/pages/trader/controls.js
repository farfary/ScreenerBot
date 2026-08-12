/**
 * Trader Controls Module
 *
 * Handles auto-trader toggle, monitor controls, force stop, and loss limit functionality.
 */

/**
 * Create trader control functions
 * @param {Object} deps - Dependencies
 * @param {Object} deps.state - Page state object
 * @param {Function} deps.$ - DOM selector function
 * @param {Object} deps.Utils - Utility functions module
 * @param {Object} deps.requestManager - Request manager instance (for future use)
 * @param {Object} deps.ConfirmationDialog - Confirmation dialog module
 * @param {Function} deps.playToggleOn - Sound effect function
 * @param {Function} deps.playToggleOff - Sound effect function
 * @param {Function} deps.playError - Sound effect function
 * @param {Array} deps.eventCleanups - Event cleanup tracking array
 * @returns {Object} Control functions
 */
export function createTraderControls({
  state: _state,
  $,
  Utils,
  requestManager: _requestManager,
  ConfirmationDialog,
  playToggleOn,
  playToggleOff,
  playError,
  eventCleanups,
}) {
  let traderAvailable = false;

  /**
   * Add tracked event listener for cleanup
   */
  function addTrackedListener(element, event, handler) {
    if (!element) return;
    element.addEventListener(event, handler);
    eventCleanups.push(() => element.removeEventListener(event, handler));
  }

  // ============================================================================
  // Auto Trader Toggle Functions
  // ============================================================================

  /**
   * Update the auto trader status bar (stats tab — the single source of the
   * on/off control)
   */
  function updateAutoTraderStatusBars(status) {
    const isRunning = status?.running === true;
    const isAvailable = status?.available !== false && status !== undefined && status !== null;
    traderAvailable = isAvailable;
    const statusText = !isAvailable ? "Setup required" : isRunning ? "Running" : "Stopped";
    const statusAttr = isRunning ? "running" : "stopped";
    const toggleLabel = !isAvailable ? "UNAVAILABLE" : isRunning ? "ON" : "OFF";

    // Update stats tab status bar
    const statsBar = $("#trader-status-bar");
    if (statsBar) {
      statsBar.setAttribute("data-status", statusAttr);
      const statsStatusText = $("#trader-status-text");
      if (statsStatusText) statsStatusText.textContent = statusText;
      const statsToggle = $("#stats-trader-toggle");
      if (statsToggle) {
        statsToggle.checked = isRunning;
        statsToggle.disabled = !isAvailable;
      }
      const statsToggleLabel = $("#stats-toggle-label");
      if (statsToggleLabel) statsToggleLabel.textContent = toggleLabel;
    }
  }

  /**
   * Toggle auto trader on/off
   */
  async function toggleAutoTrader(shouldStart, _triggerElement) {
    // Disable all toggles while processing
    const allToggles = [$("#stats-trader-toggle")];
    allToggles.forEach((toggle) => {
      if (toggle) toggle.disabled = true;
    });

    const endpoint = shouldStart ? "/api/trader/start" : "/api/trader/stop";

    try {
      const response = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
      });

      if (!response.ok) {
        throw new Error(`Failed to ${shouldStart ? "start" : "stop"} trader`);
      }

      // Play sound feedback
      if (shouldStart) {
        playToggleOn();
      } else {
        playToggleOff();
      }

      // Show toast
      Utils.showToast(`Auto Trader ${shouldStart ? "started" : "stopped"}`, "success");

      // Update status bars
      updateAutoTraderStatusBars({ running: shouldStart });
    } catch (error) {
      console.error("Toggle auto trader error:", error);
      Utils.showToast(error.message, "error");
      playError();

      // Revert toggle states
      updateAutoTraderStatusBars({ running: !shouldStart });
    } finally {
      // Re-enable all toggles
      allToggles.forEach((toggle) => {
        if (toggle) toggle.disabled = !traderAvailable;
      });
    }
  }

  /**
   * Setup auto trader toggle event handlers
   */
  function setupAutoTraderToggles() {
    const statsToggle = $("#stats-trader-toggle");

    if (statsToggle) {
      addTrackedListener(statsToggle, "change", (e) => {
        toggleAutoTrader(e.target.checked, statsToggle);
      });
    }

    // Initial fetch of trader status
    fetchTraderStatus();
  }

  /**
   * Fetch and update trader status
   */
  async function fetchTraderStatus() {
    try {
      const response = await fetch("/api/trader/status");
      if (response.ok) {
        const data = await response.json();
        updateAutoTraderStatusBars(data);
      }
    } catch (error) {
      console.warn("[Trader] Failed to fetch trader status:", error);
    }
  }

  // ============================================================================
  // Trading Controls (Force Stop, Monitor Toggles, Loss Limit)
  // ============================================================================

  /**
   * Load trading controls status from API
   */
  async function loadControlsStatus() {
    try {
      // These endpoints return the status object directly (success_response =
      // raw Json(data), no { success, data } envelope), so pass it as-is.
      const forceStopRes = await fetch("/api/trader/force-stop/status");
      if (forceStopRes.ok) {
        updateForceStopBanner(await forceStopRes.json());
      }

      const monitorsRes = await fetch("/api/trader/monitors/status");
      if (monitorsRes.ok) {
        updateMonitorControls(await monitorsRes.json());
      }

      const lossLimitRes = await fetch("/api/trader/loss-limit/status");
      if (lossLimitRes.ok) {
        updateLossLimitPanel(await lossLimitRes.json());
      }
    } catch (err) {
      console.error("[Trader] Failed to load controls status:", err);
    }
  }

  /**
   * Update force stop banner visibility and content
   */
  function updateForceStopBanner(data) {
    const banner = $("#force-stop-banner");
    const btn = $("#force-stop-btn");

    if (!banner || !btn) return;

    if (data && data.is_stopped) {
      banner.style.display = "flex";
      const reasonEl = $("#force-stop-reason");
      if (reasonEl) {
        reasonEl.textContent = data.reason || "Manual force stop";
      }
      btn.style.display = "none";
    } else {
      banner.style.display = "none";
      btn.style.display = "flex";
    }
  }

  /**
   * Update monitor toggle controls
   */
  function updateMonitorControls(data) {
    const entryToggle = $("#entry-monitor-toggle");
    const exitToggle = $("#exit-monitor-toggle");
    const entryStatus = $("#entry-monitor-status");
    const exitStatus = $("#exit-monitor-status");

    if (!data) return;

    const available = data.available !== false;

    if (entryToggle) {
      entryToggle.checked = data.entry_monitor?.enabled ?? false;
      entryToggle.disabled = !available || (data.force_stopped ?? false);
    }
    if (exitToggle) {
      exitToggle.checked = data.exit_monitor?.enabled ?? false;
      exitToggle.disabled = !available || (data.force_stopped ?? false);
    }

    if (entryStatus) {
      const running = data.entry_monitor?.running ?? false;
      entryStatus.textContent = !available ? "Setup required" : running ? "Running" : "Stopped";
      entryStatus.className = "control-status " + (running ? "status-running" : "status-stopped");
    }

    if (exitStatus) {
      const running = data.exit_monitor?.running ?? false;
      exitStatus.textContent = !available ? "Setup required" : running ? "Running" : "Stopped";
      exitStatus.className = "control-status " + (running ? "status-running" : "status-stopped");
    }
  }

  /**
   * Update loss limit panel display
   */
  function updateLossLimitPanel(data) {
    const panel = $("#loss-limit-panel");

    if (!panel) return;

    if (!data || !data.enabled) {
      panel.style.display = "none";
      return;
    }

    panel.style.display = "block";

    const value = $("#loss-limit-value");
    const progress = $("#loss-limit-progress");
    const period = $("#loss-limit-period");
    const status = $("#loss-limit-status");

    if (value) {
      const currentLoss = data.current_loss_sol?.toFixed(4) ?? "0.0000";
      const limitSol = data.limit_sol?.toFixed(4) ?? "0.0000";
      value.textContent = `${currentLoss} / ${limitSol} SOL`;
    }

    if (progress) {
      const percent = Math.min(data.progress_percent ?? 0, 100);
      progress.style.width = `${percent}%`;

      progress.classList.remove("limit-exceeded", "limit-warning");
      if (percent >= 100) {
        progress.classList.add("limit-exceeded");
      } else if (percent >= 75) {
        progress.classList.add("limit-warning");
      }
    }

    if (period) {
      const remainingSecs = data.period_remaining_secs ?? 0;
      const hours = Math.floor(remainingSecs / 3600);
      const mins = Math.floor((remainingSecs % 3600) / 60);
      period.textContent = `Resets in ${hours}h ${mins}m`;
    }

    if (status) {
      if (data.is_limited) {
        status.textContent = "LIMIT REACHED";
        status.className = "loss-limit-status status-limited";
      } else {
        status.textContent = "";
        status.className = "loss-limit-status";
      }
    }
  }

  /**
   * Setup event handlers for trading controls
   */
  function setupControlsEventHandlers() {
    // Force Stop button
    const forceStopBtn = $("#force-stop-btn");
    if (forceStopBtn) {
      addTrackedListener(forceStopBtn, "click", async () => {
        const result = await ConfirmationDialog.show({
          title: "Force Stop Trading",
          message: "This will immediately halt ALL trading operations. Continue?",
          confirmLabel: "Stop Trading",
          variant: "danger",
        });
        if (!result.confirmed) return;

        try {
          const res = await fetch("/api/trader/force-stop", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ reason: "Manual force stop from dashboard" }),
          });
          if (res.ok) {
            Utils.showToast("Force stop activated", "warning");
            playToggleOff();
            await loadControlsStatus();
          } else {
            const data = await res.json().catch(() => null);
            Utils.showToast(data?.error?.message || "Failed to activate force stop", "error");
            playError();
          }
        } catch {
          Utils.showToast("Failed to activate force stop", "error");
          playError();
        }
      });
    }

    // Resume button
    const resumeBtn = $("#resume-trading-btn");
    if (resumeBtn) {
      addTrackedListener(resumeBtn, "click", async () => {
        try {
          const res = await fetch("/api/trader/resume", { method: "POST" });
          if (res.ok) {
            Utils.showToast("Force stop cleared", "success");
            playToggleOn();
            await loadControlsStatus();
          } else {
            const data = await res.json().catch(() => null);
            Utils.showToast(data?.error?.message || "Failed to resume trading", "error");
            playError();
          }
        } catch {
          Utils.showToast("Failed to resume trading", "error");
          playError();
        }
      });
    }

    // Entry monitor toggle
    const entryToggle = $("#entry-monitor-toggle");
    if (entryToggle) {
      addTrackedListener(entryToggle, "change", async (e) => {
        try {
          const res = await fetch("/api/trader/monitors/entry/toggle", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ enabled: e.target.checked }),
          });
          if (!res.ok) {
            e.target.checked = !e.target.checked; // Revert
            const data = await res.json().catch(() => null);
            Utils.showToast(data?.error?.message || "Failed to toggle entry monitor", "error");
            playError();
          } else {
            e.target.checked ? playToggleOn() : playToggleOff();
          }
        } catch {
          e.target.checked = !e.target.checked;
          Utils.showToast("Failed to toggle entry monitor", "error");
          playError();
        }
      });
    }

    // Exit monitor toggle
    const exitToggle = $("#exit-monitor-toggle");
    if (exitToggle) {
      addTrackedListener(exitToggle, "change", async (e) => {
        try {
          const res = await fetch("/api/trader/monitors/exit/toggle", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ enabled: e.target.checked }),
          });
          if (!res.ok) {
            e.target.checked = !e.target.checked; // Revert
            const data = await res.json().catch(() => null);
            Utils.showToast(data?.error?.message || "Failed to toggle exit monitor", "error");
            playError();
          } else {
            e.target.checked ? playToggleOn() : playToggleOff();
          }
        } catch {
          e.target.checked = !e.target.checked;
          Utils.showToast("Failed to toggle exit monitor", "error");
          playError();
        }
      });
    }
  }

  // Return public API
  return {
    setupAutoTraderToggles,
    toggleAutoTrader,
    fetchTraderStatus,
    updateAutoTraderStatusBars,
    loadControlsStatus,
    updateForceStopBanner,
    updateMonitorControls,
    updateLossLimitPanel,
    setupControlsEventHandlers,
  };
}
