// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Trader controls - auto-trader toggle, force stop banner and loss limit panel actions.

import { formatTimeSpan } from "../../core/format.js";
import { apiErrorMessage } from "../../core/request_manager.js";

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
  requestManager,
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
    // The status carries a reason only when the backend knows why the trader cannot
    // run; without one the generic setup message stands in.
    const statusText = !isAvailable
      ? status?.unavailable_reason
        ? I18n.t("trader-status-unavailable")
        : I18n.t("trader-status-setup-required")
      : isRunning
        ? I18n.t("trader-status-running")
        : I18n.t("trader-status-stopped");
    const statusAttr = isRunning ? "running" : "stopped";
    const toggleLabel = !isAvailable
      ? I18n.t("trader-toggle-unavailable")
      : isRunning
        ? I18n.t("trader-toggle-on")
        : I18n.t("trader-toggle-off");

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
        throw new Error(
          shouldStart ? I18n.t("trader-toggle-start-failed") : I18n.t("trader-toggle-stop-failed")
        );
      }

      // Play sound feedback
      if (shouldStart) {
        playToggleOn();
      } else {
        playToggleOff();
      }

      // No success toast: the toggle, the status bars below and the sound all
      // report the new state already.

      // Update status bars
      updateAutoTraderStatusBars({ running: shouldStart });
    } catch (error) {
      console.error("Toggle auto trader error:", error);
      Utils.showToast({
        key: "auto-trader-control",
        type: "error",
        title: I18n.t("trader-toast-control-failed"),
        message: error.message || null,
      });
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
      updateAutoTraderStatusBars(
        await requestManager.fetch("/api/trader/status", { priority: "normal" })
      );
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
    // These endpoints return the status object directly (success_response =
    // raw Json(data), no { success, data } envelope), so pass it as-is.
    //
    // Three independent reads, so they go out together and through requestManager —
    // awaiting raw fetches one after another cost three round trips every 5s with no
    // dedupe, no priority and no cancellation on tab switch.
    const read = (url) =>
      requestManager.fetch(url, { priority: "normal" }).catch((err) => {
        console.error(`[Trader] Failed to load ${url}:`, err);
        return null;
      });

    const [forceStop, monitors, lossLimit] = await Promise.all([
      read("/api/trader/force-stop/status"),
      read("/api/trader/monitors/status"),
      read("/api/trader/loss-limit/status"),
    ]);

    if (forceStop) updateForceStopBanner(forceStop);
    if (monitors) updateMonitorControls(monitors);
    updateLossLimitPanel(lossLimit);
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
        reasonEl.textContent = data.reason || I18n.t("trader-halt-reason-default");
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

    // A monitor can be enabled and still not be running because the Auto Trader
    // master switch is off. Saying "Stopped" for both conflated a user's own setting
    // with a monitor that failed to start.
    const masterOff = data.master_enabled === false;
    const paint = (el, monitor) => {
      if (!el) return;
      const running = monitor?.running ?? false;
      const text = !available
        ? I18n.t("trader-status-setup-required")
        : running
          ? I18n.t("trader-status-running")
          : masterOff
            ? I18n.t("trader-monitor-master-off")
            : I18n.t("trader-status-stopped");
      el.textContent = text;
      el.className = "control-status " + (running ? "status-running" : "status-stopped");
    };

    paint(entryStatus, data.entry_monitor);
    paint(exitStatus, data.exit_monitor);
  }

  /**
   * Update loss limit panel display
   */
  function updateLossLimitPanel(data) {
    const panel = $("#loss-limit-panel");
    if (!panel) return;

    const value = $("#loss-limit-value");
    const progress = $("#loss-limit-progress");
    const period = $("#loss-limit-period");
    const status = $("#loss-limit-status");
    const actions = $("#loss-limit-actions");

    // The panel stays on screen when the limit is off. Hiding it made the safety net
    // invisible, so a user could not tell whether it existed or was simply disabled.
    const enabled = Boolean(data?.enabled);
    panel.classList.toggle("loss-limit-panel--off", !enabled);

    if (!enabled) {
      if (value) value.textContent = I18n.t("trader-loss-limit-off");
      if (progress) {
        progress.style.width = "0%";
        progress.classList.remove("limit-exceeded", "limit-warning");
      }
      if (period) period.textContent = I18n.t("trader-loss-limit-none");
      if (status) {
        status.textContent = "";
        status.className = "loss-limit-status";
      }
      if (actions) actions.hidden = true;
      return;
    }

    if (value) {
      const currentLoss = Utils.formatSol(data.current_loss_sol, { suffix: "", fallback: "—" });
      const limitSol = Utils.formatSol(data.limit_sol, { fallback: "—" });
      value.textContent = `${currentLoss} / ${limitSol}`;
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
      period.textContent = I18n.t("trader-loss-limit-resets-in", {
        hours: formatTimeSpan(hours, { unit: "hour" }),
        minutes: formatTimeSpan(mins, { unit: "minute" }),
      });
    }

    if (status) {
      status.textContent = data.is_limited ? I18n.t("trader-loss-limit-reached") : "";
      status.className = data.is_limited ? "loss-limit-status status-limited" : "loss-limit-status";
    }

    // /loss-limit/resume and /loss-limit/reset existed with no caller, so a tripped
    // limit was a dead end: the panel said "LIMIT REACHED" and offered no way out.
    if (actions) actions.hidden = !data.is_limited;
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
          title: I18n.t("trader-force-stop-confirm"),
          message: I18n.attr("trader-force-stop-confirm", "message"),
          confirmLabel: I18n.attr("trader-force-stop-confirm", "confirm"),
          variant: "danger",
        });
        if (!result.confirmed) return;

        try {
          const res = await fetch("/api/trader/force-stop", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({}),
          });
          if (res.ok) {
            Utils.showToast({
              key: "trader-force-stop",
              type: "warning",
              title: I18n.t("trader-toast-force-stop-on"),
            });
            playToggleOff();
            await loadControlsStatus();
          } else {
            const data = await res.json().catch(() => null);
            Utils.showToast({
              type: "error",
              title: I18n.t("trader-toast-force-stop-failed"),
              message: apiErrorMessage(data, null),
            });
            playError();
          }
        } catch {
          Utils.showToast({ type: "error", title: I18n.t("trader-toast-force-stop-failed") });
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
            Utils.showToast({
              key: "trader-force-stop",
              type: "success",
              title: I18n.t("trader-toast-force-stop-cleared"),
            });
            playToggleOn();
            await loadControlsStatus();
          } else {
            const data = await res.json().catch(() => null);
            Utils.showToast({
              type: "error",
              title: I18n.t("trader-toast-resume-failed"),
              message: apiErrorMessage(data, null),
            });
            playError();
          }
        } catch {
          Utils.showToast({ type: "error", title: I18n.t("trader-toast-resume-failed") });
          playError();
        }
      });
    }

    // Loss limit recovery. Both endpoints already existed; nothing called them.
    const lossLimitAction = async (endpoint, failureTitle) => {
      try {
        const res = await fetch(endpoint, { method: "POST" });
        if (res.ok) {
          playToggleOn();
          await loadControlsStatus();
        } else {
          const data = await res.json().catch(() => null);
          Utils.showToast({
            key: "loss-limit-action",
            type: "error",
            title: failureTitle,
            message: apiErrorMessage(data, null),
          });
          playError();
        }
      } catch {
        Utils.showToast({ key: "loss-limit-action", type: "error", title: failureTitle });
        playError();
      }
    };

    const lossLimitResumeBtn = $("#loss-limit-resume-btn");
    if (lossLimitResumeBtn) {
      addTrackedListener(lossLimitResumeBtn, "click", async () => {
        const result = await ConfirmationDialog.show({
          title: I18n.t("trader-loss-limit-resume-confirm"),
          message: I18n.attr("trader-loss-limit-resume-confirm", "message"),
          confirmLabel: I18n.t("trader-loss-limit-resume"),
          variant: "warning",
        });
        if (!result.confirmed) return;
        await lossLimitAction(
          "/api/trader/loss-limit/resume",
          I18n.t("trader-toast-resume-failed")
        );
      });
    }

    const lossLimitResetBtn = $("#loss-limit-reset-btn");
    if (lossLimitResetBtn) {
      addTrackedListener(lossLimitResetBtn, "click", async () => {
        const result = await ConfirmationDialog.show({
          title: I18n.t("trader-loss-limit-reset-confirm"),
          message: I18n.attr("trader-loss-limit-reset-confirm", "message"),
          confirmLabel: I18n.t("trader-loss-limit-reset"),
          variant: "warning",
        });
        if (!result.confirmed) return;
        await lossLimitAction(
          "/api/trader/loss-limit/reset",
          I18n.t("trader-toast-loss-limit-reset-failed")
        );
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
            Utils.showToast({
              key: "monitor-toggle",
              type: "error",
              title: I18n.t("trader-toast-entry-monitor-failed"),
              message: apiErrorMessage(data, null),
            });
            playError();
          } else {
            e.target.checked ? playToggleOn() : playToggleOff();
          }
        } catch {
          e.target.checked = !e.target.checked;
          Utils.showToast({
            key: "monitor-toggle",
            type: "error",
            title: I18n.t("trader-toast-entry-monitor-failed"),
          });
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
            Utils.showToast({
              key: "monitor-toggle",
              type: "error",
              title: I18n.t("trader-toast-exit-monitor-failed"),
              message: apiErrorMessage(data, null),
            });
            playError();
          } else {
            e.target.checked ? playToggleOn() : playToggleOff();
          }
        } catch {
          e.target.checked = !e.target.checked;
          Utils.showToast({
            key: "monitor-toggle",
            type: "error",
            title: I18n.t("trader-toast-exit-monitor-failed"),
          });
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
