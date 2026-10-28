// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Trader controls - the Stats tab's run state and entry chips, start and stop, the page notices, the monitor switches, the loss limit meter and the force stop.

import { paintHtml } from "../../core/dom.js";
import { formatTimeSpan } from "../../core/format.js";
import { apiErrorMessage } from "../../core/request_manager.js";
import { setAutoTraderEnabled } from "../../ui/auto_trader_control.js";
import { entryBlockText } from "../../ui/entry_block.js";
import { renderNotice } from "../../ui/notice.js";
import { SetupDialog } from "../../ui/setup_dialog.js";
import { renderSummaryChip } from "../../ui/table_toolbar.js";

/**
 * Message key of each `TraderRunState` the run chip names. `explore` shows no chip (the
 * setup notice speaks), and `entry_paused` reads as `running`: the monitors run, and the
 * entries chip names the block.
 */
const RUN_STATE_LABELS = Object.freeze({
  force_stopped: "trader-run-state-force-stopped",
  stopped: "trader-status-stopped",
  waiting: "trader-run-state-waiting",
  idle: "trader-run-state-idle",
  running: "trader-status-running",
});

/** Chip variant of each run state the chip names; a stopped trader is untoned. */
const RUN_STATE_VARIANTS = Object.freeze({
  force_stopped: "error",
  stopped: null,
  waiting: "warning",
  idle: "warning",
  running: "success",
});

/** Run states in which new entries are taken, so the entries chip has something to say. */
const ENTRY_STATES = new Set(["running", "entry_paused"]);

/**
 * Create the Stats tab's controls.
 * @param {Object} deps - Dependencies
 * @param {Function} deps.$ - DOM selector function
 * @param {Object} deps.Utils - Utility functions module
 * @param {Object} deps.requestManager - Request manager instance
 * @param {Object} deps.ConfirmationDialog - Confirmation dialog module
 * @param {Function} deps.playToggleOn - Sound effect function
 * @param {Function} deps.playToggleOff - Sound effect function
 * @param {Function} deps.playError - Sound effect function
 * @param {Array} deps.eventCleanups - Event cleanup tracking array
 * @param {Function} deps.getBook - The book the Stats tab shows, `live` or `paper`
 * @param {Function} [deps.onStatus] - Receives each `/api/trader/status` read
 * @returns {Object} Control functions
 */
export function createTraderControls({
  $,
  Utils,
  requestManager,
  ConfirmationDialog,
  playToggleOn,
  playToggleOff,
  playError,
  eventCleanups,
  getBook,
  onStatus,
}) {
  // The last read of each endpoint; a failed read keeps the previous one.
  let status = null;
  let forceStop = null;
  let monitors = null;
  let lossLimit = null;
  let loadFailed = false;
  // Monitor switches with a toggle request in flight.
  const pending = new Set();

  function addTrackedListener(element, event, handler) {
    if (!element) return;
    element.addEventListener(event, handler);
    eventCleanups.push(() => element.removeEventListener(event, handler));
  }

  const available = () => status?.available !== false && monitors?.available !== false;
  const forceStopped = () =>
    Boolean(forceStop?.is_stopped) || status?.run_state === "force_stopped";

  // ============================================================================
  // Reading
  // ============================================================================

  /**
   * Read the trader status, the force stop, the monitors and the loss limit together, and
   * paint the toolbar state, the notices and the controls section.
   */
  async function refresh() {
    // These endpoints return the status object directly, without an envelope.
    const read = (url) =>
      requestManager.fetch(url, { priority: "normal" }).catch((err) => {
        console.error(`[Trader] Failed to load ${url}:`, err);
        return null;
      });

    const [nextStatus, nextForceStop, nextMonitors, nextLossLimit] = await Promise.all([
      read("/api/trader/status"),
      read("/api/trader/force-stop/status"),
      read("/api/trader/monitors/status"),
      read("/api/trader/loss-limit/status"),
    ]);
    status = nextStatus ?? status;
    forceStop = nextForceStop ?? forceStop;
    monitors = nextMonitors ?? monitors;
    lossLimit = nextLossLimit ?? lossLimit;
    paint();
    if (nextStatus) onStatus?.(nextStatus);
  }

  /** Report a failed statistics read (an error), or clear it (`null`). */
  function setLoadError(error) {
    // The notice states the failure in the page language; the transport text goes to
    // the console only.
    if (error) console.warn("Trader statistics load failed:", error);
    loadFailed = Boolean(error);
    paintNotices();
  }

  function paint() {
    paintState();
    paintAction();
    paintNotices();
    paintMonitors();
    paintLossLimit();
    paintForceStop();
  }

  // ============================================================================
  // Toolbar
  // ============================================================================

  function entriesChip() {
    if (!ENTRY_STATES.has(status?.run_state)) return "";
    const live = getBook() === "live";
    const idle = live ? status.live_entry_idle : status.paper_entry_idle;
    let value = I18n.t("trader-entries-open");
    let variant = null;
    if (idle?.kind === "entry_monitor_off") {
      value = I18n.t("trader-entries-monitor-off");
      variant = "warning";
    } else if (idle?.kind === "no_entry_strategy") {
      value = live
        ? I18n.t("trader-entries-no-live-strategy")
        : I18n.t("trader-entries-no-paper-strategy");
      variant = "warning";
    } else if (idle?.kind === "blocked") {
      value = entryBlockText(idle.block?.kind);
      variant = "warning";
    }
    return renderSummaryChip({
      id: "entries",
      label: I18n.t("trader-entries-label"),
      value,
      variant,
      format: "text",
    });
  }

  function paintState() {
    const state = status?.run_state === "entry_paused" ? "running" : status?.run_state;
    const html = Object.hasOwn(RUN_STATE_LABELS, state ?? "")
      ? renderSummaryChip({
          id: "run",
          label: I18n.t("trader-status-title"),
          value: I18n.label(RUN_STATE_LABELS, state),
          variant: RUN_STATE_VARIANTS[state],
          format: "text",
        }) + entriesChip()
      : "";
    paintHtml($("#stats-state"), html);
  }

  function paintAction() {
    const button = $("#stats-trader-action");
    if (!button) return;
    button.hidden = !status || !available() || forceStopped();
    if (button.hidden) return;
    const start = status.enabled === false;
    button.className = `btn btn-sm ${start ? "btn-primary" : "btn-secondary"}`;
    const [icon, label] = start
      ? ["icon-play", I18n.t("trader-action-start")]
      : ["icon-circle-stop", I18n.t("trader-action-stop")];
    paintHtml(
      button,
      `<i class="${icon}" aria-hidden="true"></i> <span>${Utils.escapeHtml(label)}</span>`
    );
  }

  /** Start or stop the Auto Trader through the one start/stop path; starting confirms. */
  async function toggleAutoTrader() {
    const button = $("#stats-trader-action");
    const shouldStart = status?.enabled === false;
    try {
      const control = await setAutoTraderEnabled(shouldStart, {
        onRequest: () => {
          if (button) button.disabled = true;
        },
      });
      if (control.cancelled) return;
      if (!control.response.ok) {
        throw new Error(
          shouldStart ? I18n.t("trader-toggle-start-failed") : I18n.t("trader-toggle-stop-failed")
        );
      }
      // No success toast: the run chip, the button and the sound report the new state.
      if (shouldStart) playToggleOn();
      else playToggleOff();
    } catch (error) {
      console.error("Toggle auto trader error:", error);
      Utils.showToast({
        key: "auto-trader-control",
        type: "error",
        title: I18n.t("trader-toast-control-failed"),
        message: error.message || null,
      });
      playError();
    } finally {
      if (button) button.disabled = false;
      await refresh();
    }
  }

  // ============================================================================
  // Notices
  // ============================================================================

  function paintNotices() {
    const container = $("#stats-notices");
    if (!container) return;
    const notices = [];
    if (status && !available()) {
      notices.push(
        renderNotice({
          variant: "warning",
          title: I18n.t("trader-status-setup-required"),
          message: status.unavailable_reason ? I18n.text(status.unavailable_reason) : "",
          action: { id: "setup", label: I18n.t("trader-setup-action") },
        })
      );
    }
    if (forceStop?.is_stopped) {
      notices.push(
        renderNotice({
          variant: "error",
          title: I18n.t("trader-halt-title"),
          message: forceStop.reason || I18n.t("trader-halt-reason-default"),
          action: { id: "resume", label: I18n.t("trader-halt-resume") },
        })
      );
    }
    // A force stop halts exits too, so it is the one paused state shown; the loss limit
    // still reads in its meter and returns as a notice once trading resumes.
    if (lossLimit?.is_limited && !forceStop?.is_stopped) {
      notices.push(
        renderNotice({
          variant: "warning",
          title: I18n.t("trader-notice-loss-limit"),
          message: I18n.t("trader-notice-loss-limit-message"),
          action: { id: "loss-limit-resume", label: I18n.t("trader-loss-limit-resume") },
        })
      );
    }
    if (loadFailed) {
      notices.push(
        renderNotice({
          variant: "error",
          title: I18n.t("trader-notice-stats-failed"),
          action: { id: "stats-retry", label: I18n.t("common-action-retry") },
        })
      );
    }
    const html = notices.join("");
    paintHtml(container, html);
    container.hidden = !html;
  }

  // ============================================================================
  // Controls section
  // ============================================================================

  /**
   * A switch shows the saved monitor setting. Its note says the monitor starts with the
   * Auto Trader only while the trader is stopped; every other state is said once, by the
   * run chip or a notice.
   */
  function paintMonitors() {
    if (!monitors) return;
    const usable = available();
    const locked = !usable || forceStopped();
    const stopped = status?.run_state === "stopped";
    const paintOne = (kind, monitor) => {
      const toggle = $(`#${kind}-monitor-toggle`);
      const note = $(`#${kind}-monitor-status`);
      const enabled = usable && (monitor?.enabled ?? false);
      if (toggle) {
        toggle.checked = enabled;
        toggle.disabled = locked || pending.has(kind);
      }
      if (note) note.hidden = !(enabled && stopped);
    };
    paintOne("entry", monitors.entry_monitor);
    paintOne("exit", monitors.exit_monitor);
  }

  function paintLossLimit() {
    if (!lossLimit) return;
    const value = $("#loss-limit-value");
    const bar = $("#loss-limit-progress")?.parentElement;
    const progress = $("#loss-limit-progress");
    const period = $("#loss-limit-period");
    const reset = $("#loss-limit-reset-btn");

    if (!lossLimit.enabled) {
      if (value) value.textContent = I18n.t("trader-loss-limit-off");
      if (bar) bar.hidden = true;
      if (period) period.hidden = true;
      if (reset) reset.hidden = true;
      return;
    }

    if (value) {
      const current = Utils.formatSol(lossLimit.current_loss_native, { suffix: "", fallback: "—" });
      const limit = Utils.formatSol(lossLimit.limit_native, { fallback: "—" });
      value.textContent = `${current} / ${limit}`;
    }
    if (bar) bar.hidden = false;
    if (progress) {
      const percent = Math.min(lossLimit.progress_percent ?? 0, 100);
      progress.style.width = `${percent}%`;
      progress.classList.toggle("limit-exceeded", percent >= 100);
      progress.classList.toggle("limit-warning", percent >= 75 && percent < 100);
    }
    if (period) {
      const remainingSecs = lossLimit.period_remaining_secs ?? 0;
      const hours = Math.floor(remainingSecs / 3600);
      const mins = Math.floor((remainingSecs % 3600) / 60);
      period.hidden = false;
      period.textContent = I18n.t("trader-loss-limit-resets-in", {
        hours: formatTimeSpan(hours, { unit: "hour" }),
        minutes: formatTimeSpan(mins, { unit: "minute" }),
      });
    }
    // Resume lives in the notice; the reset sits beside the meter it clears.
    if (reset) reset.hidden = !lossLimit.is_limited;
  }

  /**
   * The force stop is offered only while the trader is set up and not already stopped;
   * with nothing to halt it keeps its place, disabled, and its hint says why.
   */
  function paintForceStop() {
    const button = $("#force-stop-btn");
    const hint = $("#force-stop-hint");
    const group = button?.closest(".stats-force-stop");
    if (!button || !group) return;
    group.hidden = !status || !available() || forceStopped();
    if (group.hidden || !monitors) return;
    const idle = monitors.anything_running === false;
    button.disabled = idle;
    if (hint) {
      hint.textContent = idle ? I18n.t("trader-force-stop-idle") : I18n.t("trader-force-stop-hint");
    }
  }

  // ============================================================================
  // Actions
  // ============================================================================

  /** POST to a control endpoint; a failure toasts `failureTitle` with the server's reason. */
  async function postControl(endpoint, { failureTitle, toastKey, onSuccess, body }) {
    try {
      const res = await fetch(endpoint, {
        method: "POST",
        ...(body
          ? { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }
          : {}),
      });
      if (res.ok) {
        onSuccess?.();
        await refresh();
        return true;
      }
      const data = await res.json().catch(() => null);
      Utils.showToast({
        key: toastKey,
        type: "error",
        title: failureTitle,
        message: apiErrorMessage(data, null),
      });
    } catch {
      Utils.showToast({ key: toastKey, type: "error", title: failureTitle });
    }
    playError();
    return false;
  }

  async function forceStopAll() {
    const result = await ConfirmationDialog.show({
      title: I18n.t("trader-force-stop-confirm"),
      message: I18n.attr("trader-force-stop-confirm", "message"),
      confirmLabel: I18n.attr("trader-force-stop-confirm", "confirm"),
      variant: "danger",
    });
    if (!result.confirmed) return;
    await postControl("/api/trader/force-stop", {
      body: {},
      failureTitle: I18n.t("trader-toast-force-stop-failed"),
      onSuccess: () => {
        Utils.showToast({
          key: "trader-force-stop",
          type: "warning",
          title: I18n.t("trader-toast-force-stop-on"),
        });
        playToggleOff();
      },
    });
  }

  async function resumeAfterForceStop() {
    await postControl("/api/trader/resume", {
      failureTitle: I18n.t("trader-toast-resume-failed"),
      onSuccess: () => {
        Utils.showToast({
          key: "trader-force-stop",
          type: "success",
          title: I18n.t("trader-toast-force-stop-cleared"),
        });
        playToggleOn();
      },
    });
  }

  async function resumeAfterLossLimit() {
    const result = await ConfirmationDialog.show({
      title: I18n.t("trader-loss-limit-resume-confirm"),
      message: I18n.attr("trader-loss-limit-resume-confirm", "message"),
      confirmLabel: I18n.t("trader-loss-limit-resume"),
      variant: "warning",
    });
    if (!result.confirmed) return;
    await postControl("/api/trader/loss-limit/resume", {
      toastKey: "loss-limit-action",
      failureTitle: I18n.t("trader-toast-resume-failed"),
      onSuccess: playToggleOn,
    });
  }

  async function resetLossLimit() {
    const result = await ConfirmationDialog.show({
      title: I18n.t("trader-loss-limit-reset-confirm"),
      message: I18n.attr("trader-loss-limit-reset-confirm", "message"),
      confirmLabel: I18n.t("trader-loss-limit-reset"),
      variant: "warning",
    });
    if (!result.confirmed) return;
    await postControl("/api/trader/loss-limit/reset", {
      toastKey: "loss-limit-action",
      failureTitle: I18n.t("trader-toast-loss-limit-reset-failed"),
      onSuccess: playToggleOn,
    });
  }

  /** Save a monitor switch; a refused change puts the switch back. */
  async function toggleMonitor(kind, toggle, failureTitle) {
    const enabled = toggle.checked;
    pending.add(kind);
    toggle.disabled = true;
    const saved = await postControl(`/api/trader/monitors/${kind}/toggle`, {
      body: { enabled },
      toastKey: "monitor-toggle",
      failureTitle,
      onSuccess: () => {
        pending.delete(kind);
        if (enabled) playToggleOn();
        else playToggleOff();
      },
    });
    if (!saved) {
      pending.delete(kind);
      toggle.checked = !enabled;
      paintMonitors();
    }
  }

  /** Wire the controls section, the start/stop button and the notice actions. */
  function setupControlsEventHandlers() {
    addTrackedListener($("#stats-trader-action"), "click", () => void toggleAutoTrader());
    addTrackedListener($("#force-stop-btn"), "click", () => void forceStopAll());
    addTrackedListener($("#loss-limit-reset-btn"), "click", () => void resetLossLimit());

    const entryToggle = $("#entry-monitor-toggle");
    addTrackedListener(entryToggle, "change", () =>
      toggleMonitor("entry", entryToggle, I18n.t("trader-toast-entry-monitor-failed"))
    );
    const exitToggle = $("#exit-monitor-toggle");
    addTrackedListener(exitToggle, "change", () =>
      toggleMonitor("exit", exitToggle, I18n.t("trader-toast-exit-monitor-failed"))
    );

    addTrackedListener($("#stats-notices"), "click", (event) => {
      const action = event.target.closest("[data-action]")?.dataset.action;
      // The same wallet and RPC setup dialog the header's Explore Mode action opens.
      if (action === "setup") void SetupDialog.show();
      else if (action === "resume") void resumeAfterForceStop();
      else if (action === "loss-limit-resume") void resumeAfterLossLimit();
    });
  }

  return {
    refresh,
    setLoadError,
    setupControlsEventHandlers,
  };
}
