// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Automation tab of the assistant page that lists, creates, edits, toggles, runs and deletes scheduled tasks through the /api/assistant/automation routes, renders run history and the run-detail dialog, and manages the per-task context menu.

import { $ } from "../../core/dom.js";
import { closeMenu, openMenu, trackAnchoredMenu } from "../../core/menu_manager.js";
import { formatNumber, formatPercentValue, formatTimeSpan, formatWeekday } from "../../core/format.js";
import { apiErrorMessage } from "../../core/request_manager.js";
import * as Utils from "../../core/utils.js";
import { AGENT_TOOL_LABELS } from "../../ui/agent_tool.js";
import { ConfirmationDialog } from "../../ui/confirmation_dialog.js";
import { LLM_PROVIDER_LABELS } from "../../ui/llm_provider.js";
import { TOOL_CALL_STATUS_LABELS } from "../../ui/tool_call_status.js";

const esc = Utils.escapeHtml;

// Message key of each `ScheduleType` (src/assistant/scheduled/types.rs), by `as_str`.
const SCHEDULE_TYPE_LABELS = Object.freeze({
  interval: "assistant-automation-schedule-type-interval",
  daily: "assistant-automation-schedule-type-daily",
  weekly: "assistant-automation-schedule-type-weekly",
});

// Message key of each `TaskToolPermissions`, by `as_str`: the badge label and
// the longer picker option.
const TOOL_PERMISSION_LABELS = Object.freeze({
  read_only: "assistant-automation-permission-read-only",
  full: "assistant-automation-permission-full",
});
const TOOL_PERMISSION_OPTION_LABELS = Object.freeze({
  read_only: "assistant-automation-permission-option-read-only",
  full: "assistant-automation-permission-option-full",
});

// Message key of each `RunStatus`, by `as_str`.
const RUN_STATUS_LABELS = Object.freeze({
  running: "assistant-automation-run-status-running",
  success: "assistant-automation-run-status-success",
  failed: "assistant-automation-run-status-failed",
  timeout: "assistant-automation-run-status-timeout",
  skipped: "assistant-automation-run-status-skipped",
});

// Weekly schedule values name days with these fixed tokens; the value is the
// day index `formatWeekday` takes (0 is Sunday).
const WEEKDAY_INDEX = Object.freeze({ sun: 0, mon: 1, tue: 2, wed: 3, thu: 4, fri: 5, sat: 6 });

// Example schedule values for the input placeholder of each schedule type.
const SCHEDULE_PLACEHOLDERS = Object.freeze({
  interval: "300",
  daily: "14:00",
  // l10n-ignore: weekly schedule syntax example; the day tokens are fixed machine values
  weekly: "mon,wed,fri:09:00",
});

const SCHEDULE_HINT_LABELS = Object.freeze({
  interval: "assistant-automation-hint-interval",
  daily: "assistant-automation-hint-daily",
  weekly: "assistant-automation-hint-weekly",
});

export function createAutomationTab({ state, _eventCleanups, addTrackedListener }) {
  let activeAutomationMenu = null;
  // Hash guards — skip re-render when polled data is unchanged
  let _lastTasksKey = null;
  let _lastRunsKey = null;

  // Automation Tab
  // ============================================================================

  async function loadAutomationTasks() {
    try {
      const response = await fetch("/api/assistant/automation");
      if (!response.ok) throw new Error("Failed to load tasks");
      const data = await response.json();
      state.automationTasks = data.tasks || [];
      renderAutomationList(state.automationTasks);
    } catch (error) {
      console.error("[Assistant] Error loading automation tasks:", error);
    }
  }

  async function loadAutomationRuns() {
    try {
      const response = await fetch("/api/assistant/automation/runs");
      if (!response.ok) throw new Error("Failed to load runs");
      const data = await response.json();
      state.automationRuns = data.runs || [];
      renderAutomationRuns(state.automationRuns);
    } catch (error) {
      console.error("[Assistant] Error loading automation runs:", error);
    }
  }

  async function loadAutomationStats() {
    try {
      const response = await fetch("/api/assistant/automation/stats");
      if (!response.ok) throw new Error("Failed to load stats");
      const data = await response.json();
      state.automationStats = data.stats;
      renderAutomationStats(data.stats);
    } catch (error) {
      console.error("[Assistant] Error loading automation stats:", error);
    }
  }

  function renderAutomationStats(stats) {
    if (!stats) return;
    const el = (id, val) => {
      const e = $(`#${id}`);
      if (e) e.textContent = val;
    };
    el("auto-stat-total", formatNumber(stats.total_tasks || 0, 0));
    el("auto-stat-active", formatNumber(stats.active_tasks || 0, 0));
    el("auto-stat-runs", formatNumber(stats.total_runs || 0, 0));
    el(
      "auto-stat-success-rate",
      stats.total_runs > 0
        ? formatPercentValue((stats.successful_runs / stats.total_runs) * 100, {
            decimals: 0,
            includeSign: false,
          })
        : "—"
    );
  }

  function renderAutomationList(tasks) {
    activeAutomationMenu?.close("superseded");
    const key = JSON.stringify(tasks);
    if (key === _lastTasksKey) return;
    _lastTasksKey = key;

    const container = $("#automation-list");
    if (!container) return;

    if (!tasks || tasks.length === 0) {
      container.innerHTML = `
      <div class="empty-state" id="no-automation-tasks">
        <i class="empty-icon icon-zap"></i>
        <p class="empty-text" data-l10n-id="assistant-automation-empty"></p>
        <p class="empty-state-subtitle" data-l10n-id="assistant-automation-empty-subtitle"></p>
        <button class="btn btn-secondary" data-l10n-id="assistant-automation-empty-add" onclick="window.assistantPage.createAutomationTask()"></button>
      </div>
    `;
      I18n.localizeTree(container);
      return;
    }

    container.innerHTML = tasks
      .map((task) => {
        const statusClass = task.enabled ? "active" : "paused";
        const statusLabel = task.enabled
          ? I18n.t("assistant-automation-task-active")
          : I18n.t("assistant-automation-task-paused");
        const scheduleLabel = formatSchedule(task.schedule_type, task.schedule_value);
        const lastRun = task.last_run_at
          ? Utils.formatTimeAgo(new Date(task.last_run_at))
          : I18n.t("assistant-automation-never");
        const nextRun =
          task.next_run_at && task.enabled
            ? Utils.formatTimeUntil(new Date(task.next_run_at))
            : "—";
        const permKey = task.tool_permissions === "full" ? "full" : "read_only";
        const permLabel = I18n.label(TOOL_PERMISSION_LABELS, permKey);
        const permClass = task.tool_permissions === "full" ? "full" : "readonly";

        return `
      <div class="automation-task-item" data-id="${task.id}">
        <div class="automation-task-info">
          <div class="automation-task-name">${Utils.escapeHtml(task.name)}</div>
          <div class="automation-task-meta">
            <span class="schedule-badge"><i class="icon-clock"></i> ${esc(scheduleLabel)}</span>
            <span class="perm-badge ${permClass}">${esc(permLabel)}</span>
            <span class="meta-sep">·</span>
            <span class="meta-text">${esc(I18n.t("assistant-automation-last-run", { when: lastRun }))}</span>
            <span class="meta-sep">·</span>
            <span class="meta-text">${esc(I18n.t("assistant-automation-next-run", { when: nextRun }))}</span>
          </div>
        </div>
        <div class="automation-task-actions">
          <span class="status-indicator ${statusClass}">${esc(statusLabel)}</span>
          <label class="toggle">
            <input type="checkbox" ${task.enabled ? "checked" : ""}
                   onchange="window.assistantPage.toggleAutomationTask(${task.id}, this.checked)">
            <span class="toggle-track"></span>
          </label>
          <button class="btn btn-sm btn-secondary" data-l10n-id="assistant-automation-run-now" onclick="window.assistantPage.runAutomationTask(${task.id})">
            <i class="icon-play"></i>
          </button>
          <button class="automation-menu-btn" type="button" data-l10n-id="assistant-automation-actions" aria-haspopup="menu" aria-expanded="false" onclick="window.assistantPage.showAutomationMenu(event, ${task.id})"><span aria-hidden="true">⋮</span></button>
        </div>
      </div>
    `;
      })
      .join("");
    I18n.localizeTree(container);
  }

  function renderAutomationRuns(runs) {
    const key = JSON.stringify(runs);
    if (key === _lastRunsKey) return;
    _lastRunsKey = key;

    const container = $("#automation-runs-list");
    const countEl = $("#auto-runs-count");
    if (!container) return;
    if (countEl) countEl.textContent =
        runs.length > 0
          ? I18n.t("assistant-automation-runs-count", {
              count: runs.length,
              amount: formatNumber(runs.length, 0),
            })
          : "";

    if (!runs || runs.length === 0) {
      container.innerHTML =
        '<div class="automation-runs-empty" data-l10n-id="assistant-automation-runs-empty"></div>';
      I18n.localizeTree(container);
      return;
    }

    container.innerHTML = runs
      .slice(0, 20)
      .map((run) => {
        const statusIcon =
          run.status === "success"
            ? "icon-circle-check"
            : run.status === "running"
              ? "icon-loader"
              : "icon-circle-x";
        const statusClass =
          run.status === "success" ? "success" : run.status === "running" ? "running" : "failed";
        const taskName = runTaskName(run.task_id);
        const time = run.started_at ? Utils.formatTimeAgo(new Date(run.started_at)) : "";
        const duration = run.duration_ms
          ? formatTimeSpan(run.duration_ms / 1000, { decimals: 1 })
          : "";

        return `
      <div class="automation-run-item ${statusClass}" onclick="window.assistantPage.viewAutomationRun(${run.id})">
        <i class="${statusIcon} run-status-icon"></i>
        <div class="run-info">
          <span class="run-task-name">${esc(taskName)}</span>
          <span class="run-time">${esc(time)}</span>
        </div>
        <span class="run-duration">${esc(duration)}</span>
      </div>
    `;
      })
      .join("");
  }

  function runTaskName(taskId) {
    return (
      state.automationTasks.find((t) => t.id === taskId)?.name ||
      I18n.t("assistant-automation-task-fallback", { id: String(taskId) })
    );
  }

  function formatSchedule(type, value) {
    if (type === "interval") {
      const secs = parseInt(value);
      const span =
        secs >= 3600
          ? formatTimeSpan(Math.round(secs / 3600), { unit: "hour" })
          : secs >= 60
            ? formatTimeSpan(Math.round(secs / 60), { unit: "minute" })
            : formatTimeSpan(secs);
      return I18n.t("assistant-automation-schedule-every", { span });
    }
    if (type === "daily") return I18n.t("assistant-automation-schedule-daily", { time: value });
    if (type === "weekly") {
      const parts = value.split(":");
      const days = parts[0]
        .split(",")
        .map((day) => {
          const index = WEEKDAY_INDEX[day.trim().toLowerCase()];
          return index === undefined ? day : formatWeekday(index);
        })
        .join(I18n.t("assistant-automation-schedule-day-separator"));
      const time = parts.slice(1).join(":");
      return I18n.t("assistant-automation-schedule-weekly", { days, time });
    }
    return value;
  }

  function scheduleTypeOptions(selected) {
    return Object.keys(SCHEDULE_TYPE_LABELS)
      .map(
        (id) =>
          `<option value="${id}" ${id === selected ? "selected" : ""}>${esc(I18n.label(SCHEDULE_TYPE_LABELS, id))}</option>`
      )
      .join("");
  }

  function permissionOptions(labels, selected) {
    return Object.keys(labels)
      .map(
        (id) =>
          `<option value="${id}" ${id === selected ? "selected" : ""}>${esc(I18n.label(labels, id))}</option>`
      )
      .join("");
  }

  async function createAutomationTask() {
    // Remove any existing automation modal
    document.querySelectorAll(".modal-overlay.automation-modal-overlay").forEach((m) => m.remove());
    const modal = document.createElement("div");
    modal.className = "modal-overlay automation-modal-overlay";
    modal.innerHTML = `
    <div class="modal-dialog automation-modal">
      <div class="modal-header">
        <h3><i class="icon-plus"></i> <span data-l10n-id="assistant-automation-create-title"></span></h3>
        <button class="modal-close" data-l10n-id="assistant-modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label data-l10n-id="assistant-automation-field-name"></label>
          <input type="text" id="auto-name" data-l10n-id="assistant-automation-name-input">
        </div>
        <div class="form-group">
          <label data-l10n-id="assistant-automation-field-instruction"></label>
          <textarea id="auto-instruction" rows="6" class="instruction-editor" data-l10n-id="assistant-automation-instruction-input"></textarea>
        </div>
        <div class="form-row">
          <div class="form-group form-group-half">
            <label data-l10n-id="assistant-automation-field-schedule-type"></label>
            <select id="auto-schedule-type" data-custom-select onchange="window.assistantPage.updateScheduleHint()">${scheduleTypeOptions("interval")}</select>
          </div>
          <div class="form-group form-group-half">
            <label data-l10n-id="assistant-automation-field-schedule-value"></label>
            <input type="text" id="auto-schedule-value" placeholder="${SCHEDULE_PLACEHOLDERS.interval}">
            <small class="form-hint" id="schedule-hint" data-l10n-id="assistant-automation-hint-interval"></small>
          </div>
        </div>
        <div class="form-row">
          <div class="form-group form-group-half">
            <label data-l10n-id="assistant-automation-field-permissions"></label>
            <select id="auto-tool-permissions" data-custom-select>${permissionOptions(TOOL_PERMISSION_OPTION_LABELS, "read_only")}</select>
          </div>
          <div class="form-group form-group-half">
            <label data-l10n-id="assistant-automation-field-timeout"></label>
            <input type="number" id="auto-timeout" value="120" min="30" max="600"><span class="input-unit" data-l10n-id="assistant-automation-unit-seconds"></span>
          </div>
        </div>
        <div class="form-group">
          <div class="checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" id="auto-notify-telegram" checked>
              <span data-l10n-id="assistant-automation-notify-telegram"></span>
            </label>
            <label class="checkbox-label">
              <input type="checkbox" id="auto-notify-success" checked>
              <span data-l10n-id="assistant-automation-notify-success"></span>
            </label>
            <label class="checkbox-label">
              <input type="checkbox" id="auto-notify-failure" checked>
              <span data-l10n-id="assistant-automation-notify-failure"></span>
            </label>
          </div>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" data-l10n-id="common-action-cancel" onclick="this.closest('.modal-overlay').remove()"></button>
        <button class="btn btn-primary" onclick="window.assistantPage.saveNewAutomationTask()">
          <i class="icon-plus"></i> <span data-l10n-id="assistant-automation-create-task"></span>
        </button>
      </div>
    </div>
  `;
    I18n.localizeTree(modal);
    document.body.appendChild(modal);
  }

  function updateScheduleHint() {
    const type = $("#auto-schedule-type")?.value;
    const hint = $("#schedule-hint");
    const input = $("#auto-schedule-value");
    if (!hint || !input) return;

    if (Object.hasOwn(SCHEDULE_PLACEHOLDERS, type)) {
      hint.textContent = I18n.label(SCHEDULE_HINT_LABELS, type);
      input.placeholder = SCHEDULE_PLACEHOLDERS[type];
    }
  }

  async function saveNewAutomationTask() {
    const name = $("#auto-name")?.value?.trim();
    const instruction = $("#auto-instruction")?.value?.trim();
    const scheduleType = $("#auto-schedule-type")?.value;
    const scheduleValue = $("#auto-schedule-value")?.value?.trim();
    const toolPermissions = $("#auto-tool-permissions")?.value;
    const timeout = parseInt($("#auto-timeout")?.value) || 120;
    const notifyTelegram = $("#auto-notify-telegram")?.checked ?? true;
    const notifySuccess = $("#auto-notify-success")?.checked ?? true;
    const notifyFailure = $("#auto-notify-failure")?.checked ?? true;

    if (!name || !instruction || !scheduleValue) {
      Utils.showToast({
        type: "error",
        title: I18n.t("assistant-automation-validation-title"),
        message: I18n.t("assistant-automation-validation-required"),
      });
      return;
    }

    // Validate schedule value format
    if (scheduleType === "interval") {
      const secs = parseInt(scheduleValue);
      if (isNaN(secs) || secs < 60) {
        Utils.showToast({
          type: "error",
          title: I18n.t("assistant-automation-validation-title"),
          message: I18n.t("assistant-automation-validation-interval"),
        });
        return;
      }
    } else if (scheduleType === "daily") {
      if (!/^([01]?\d|2[0-3]):[0-5]\d$/.test(scheduleValue)) {
        Utils.showToast({
          type: "error",
          title: I18n.t("assistant-automation-validation-title"),
          message: I18n.t("assistant-automation-validation-daily"),
        });
        return;
      }
    } else if (scheduleType === "weekly") {
      if (!/^[a-z,]+(:\d{1,2}:\d{2})?$/i.test(scheduleValue)) {
        Utils.showToast({
          type: "error",
          title: I18n.t("assistant-automation-validation-title"),
          message: I18n.t("assistant-automation-validation-weekly"),
        });
        return;
      }
    }

    try {
      const response = await fetch("/api/assistant/automation", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name,
          instruction,
          schedule_type: scheduleType,
          schedule_value: scheduleValue,
          tool_permissions: toolPermissions,
          timeout_seconds: timeout,
          notify_telegram: notifyTelegram,
          notify_on_success: notifySuccess,
          notify_on_failure: notifyFailure,
        }),
      });

      if (!response.ok) {
        const err = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(err, I18n.t("assistant-automation-create-failed")));
      }

      document.querySelector(".modal-overlay")?.remove();
      Utils.showToast({ type: "success", title: I18n.t("assistant-automation-created") });
      await loadAutomationTasks();
      await loadAutomationStats();
    } catch (error) {
      Utils.showToast({ type: "error", title: error.message });
    }
  }

  async function toggleAutomationTask(id, enabled) {
    try {
      const btn = document.querySelector(`.automation-task-item[data-id="${id}"] .toggle input`);
      if (btn) btn.disabled = true;
      const response = await fetch(`/api/assistant/automation/${id}/toggle`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled }),
      });
      if (!response.ok) {
        const err = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(err, I18n.t("assistant-automation-toggle-failed")));
      }
      await loadAutomationTasks();
      await loadAutomationStats();
    } catch (error) {
      Utils.showToast({ type: "error", title: error.message });
      await loadAutomationTasks();
    }
  }

  async function runAutomationTask(id) {
    const triggerBtn = document.querySelector(
      `.automation-task-item[data-id="${id}"] .btn-sm.btn-secondary`
    );
    if (triggerBtn) {
      triggerBtn.disabled = true;
      triggerBtn.style.opacity = "0.5";
    }
    try {
      const response = await fetch(`/api/assistant/automation/${id}/run`, {
        method: "POST",
      });
      if (!response.ok) {
        const err = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(err, I18n.t("assistant-automation-trigger-failed")));
      }
      Utils.showToast({ type: "success", title: I18n.t("assistant-automation-triggered") });
      setTimeout(() => loadAutomationRuns(), 2000);
    } catch (error) {
      Utils.showToast({ type: "error", title: error.message });
    } finally {
      if (triggerBtn) {
        triggerBtn.disabled = false;
        triggerBtn.style.opacity = "";
      }
    }
  }

  async function deleteAutomationTask(id) {
    const confirmed = await ConfirmationDialog.show({
      title: I18n.t("assistant-automation-delete-title"),
      message: I18n.t("assistant-automation-delete-message"),
      confirmText: I18n.t("common-action-delete"),
      type: "danger",
    });
    if (!confirmed) return;

    try {
      const response = await fetch(`/api/assistant/automation/${id}`, { method: "DELETE" });
      if (!response.ok) {
        const err = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(err, I18n.t("assistant-automation-delete-failed")));
      }
      Utils.showToast({ type: "success", title: I18n.t("assistant-automation-deleted") });
      await loadAutomationTasks();
      await loadAutomationStats();
    } catch (error) {
      Utils.showToast({ type: "error", title: error.message });
    }
  }

  async function editAutomationTask(id) {
    const task = state.automationTasks.find((t) => t.id === id);
    if (!task) return;

    document.querySelectorAll(".modal-overlay.automation-modal-overlay").forEach((m) => m.remove());
    const modal = document.createElement("div");
    modal.className = "modal-overlay automation-modal-overlay";
    modal.innerHTML = `
    <div class="modal-dialog automation-modal">
      <div class="modal-header">
        <h3><i class="icon-square-pen"></i> <span data-l10n-id="assistant-automation-edit-title"></span></h3>
        <button class="modal-close" data-l10n-id="assistant-modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label data-l10n-id="assistant-automation-field-name"></label>
          <input type="text" id="edit-auto-name" value="${esc(task.name)}">
        </div>
        <div class="form-group">
          <label data-l10n-id="assistant-automation-field-instruction"></label>
          <textarea id="edit-auto-instruction" rows="6" class="instruction-editor">${esc(task.instruction)}</textarea>
        </div>
        <div class="form-row">
          <div class="form-group form-group-half">
            <label data-l10n-id="assistant-automation-field-schedule-type"></label>
            <select id="edit-auto-schedule-type" data-custom-select>${scheduleTypeOptions(task.schedule_type)}</select>
          </div>
          <div class="form-group form-group-half">
            <label data-l10n-id="assistant-automation-field-schedule-value"></label>
            <input type="text" id="edit-auto-schedule-value" value="${esc(task.schedule_value)}">
          </div>
        </div>
        <div class="form-row">
          <div class="form-group form-group-half">
            <label data-l10n-id="assistant-automation-field-permissions"></label>
            <select id="edit-auto-tool-permissions" data-custom-select>${permissionOptions(TOOL_PERMISSION_LABELS, task.tool_permissions === "full" ? "full" : "read_only")}</select>
          </div>
          <div class="form-group form-group-half">
            <label data-l10n-id="assistant-automation-field-timeout"></label>
            <input type="number" id="edit-auto-timeout" value="${task.timeout_seconds || 120}" min="30" max="600"><span class="input-unit" data-l10n-id="assistant-automation-unit-seconds"></span>
          </div>
        </div>
        <div class="form-group">
          <div class="checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" id="edit-auto-notify-telegram" ${task.notify_telegram !== false ? "checked" : ""}>
              <span data-l10n-id="assistant-automation-notify-telegram"></span>
            </label>
            <label class="checkbox-label">
              <input type="checkbox" id="edit-auto-notify-success" ${task.notify_on_success !== false ? "checked" : ""}>
              <span data-l10n-id="assistant-automation-notify-success"></span>
            </label>
            <label class="checkbox-label">
              <input type="checkbox" id="edit-auto-notify-failure" ${task.notify_on_failure !== false ? "checked" : ""}>
              <span data-l10n-id="assistant-automation-notify-failure"></span>
            </label>
          </div>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" data-l10n-id="common-action-cancel" onclick="this.closest('.modal-overlay').remove()"></button>
        <button class="btn btn-primary" onclick="window.assistantPage.saveEditedAutomationTask(${id})">
          <i class="icon-check"></i> <span data-l10n-id="assistant-automation-save-changes"></span>
        </button>
      </div>
    </div>
  `;
    I18n.localizeTree(modal);
    document.body.appendChild(modal);
  }

  async function saveEditedAutomationTask(id) {
    const name = $("#edit-auto-name")?.value?.trim();
    const instruction = $("#edit-auto-instruction")?.value?.trim();
    const scheduleType = $("#edit-auto-schedule-type")?.value;
    const scheduleValue = $("#edit-auto-schedule-value")?.value?.trim();
    const toolPermissions = $("#edit-auto-tool-permissions")?.value;
    const timeout = parseInt($("#edit-auto-timeout")?.value) || 120;
    const notifyTelegram = $("#edit-auto-notify-telegram")?.checked ?? true;
    const notifySuccess = $("#edit-auto-notify-success")?.checked ?? true;
    const notifyFailure = $("#edit-auto-notify-failure")?.checked ?? true;

    if (!name || !instruction || !scheduleValue) {
      Utils.showToast({
        type: "error",
        title: I18n.t("assistant-automation-validation-title"),
        message: I18n.t("assistant-automation-validation-required"),
      });
      return;
    }

    try {
      const response = await fetch(`/api/assistant/automation/${id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name,
          instruction,
          schedule_type: scheduleType,
          schedule_value: scheduleValue,
          tool_permissions: toolPermissions,
          timeout_seconds: timeout,
          notify_telegram: notifyTelegram,
          notify_on_success: notifySuccess,
          notify_on_failure: notifyFailure,
        }),
      });
      if (!response.ok) {
        const err = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(err, I18n.t("assistant-automation-update-failed")));
      }
      document.querySelector(".modal-overlay")?.remove();
      Utils.showToast({ type: "success", title: I18n.t("assistant-automation-updated") });
      await loadAutomationTasks();
    } catch (error) {
      Utils.showToast({ type: "error", title: error.message });
    }
  }

  async function viewAutomationRun(runId) {
    // Remove any existing modal first
    document.querySelectorAll(".modal-overlay.automation-modal-overlay").forEach((m) => m.remove());
    try {
      const response = await fetch(`/api/assistant/automation/runs/${runId}`);
      if (!response.ok) {
        const err = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(err, I18n.t("assistant-automation-run-load-failed")));
      }
      const data = await response.json();
      const run = data.run;
      const taskName = runTaskName(run.task_id);
      let toolCalls = [];
      try {
        toolCalls = run.tool_calls ? JSON.parse(run.tool_calls) : [];
      } catch {
        /* malformed JSON */
      }

      const modal = document.createElement("div");
      modal.className = "modal-overlay automation-modal-overlay";
      modal.innerHTML = `
      <div class="modal-dialog automation-modal">
        <div class="modal-header">
          <h3><i class="icon-file-text"></i> <span data-l10n-id="assistant-automation-run-details-title"></span></h3>
          <button class="modal-close" data-l10n-id="assistant-modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
        </div>
        <div class="modal-body">
          <div class="run-detail-grid">
            <div class="run-detail-item"><span class="run-detail-label" data-l10n-id="assistant-automation-run-task"></span><span class="run-detail-value">${esc(taskName)}</span></div>
            <div class="run-detail-item"><span class="run-detail-label" data-l10n-id="assistant-automation-run-status"></span><span class="run-detail-value status-${esc(run.status)}">${esc(I18n.label(RUN_STATUS_LABELS, run.status))}</span></div>
            <div class="run-detail-item"><span class="run-detail-label" data-l10n-id="assistant-automation-run-started"></span><span class="run-detail-value">${run.started_at ? esc(Utils.formatTimestamp(run.started_at)) : "—"}</span></div>
            <div class="run-detail-item"><span class="run-detail-label" data-l10n-id="assistant-automation-run-duration"></span><span class="run-detail-value">${run.duration_ms ? esc(formatTimeSpan(run.duration_ms / 1000, { decimals: 1 })) : "—"}</span></div>
            ${run.provider ? `<div class="run-detail-item"><span class="run-detail-label" data-l10n-id="assistant-automation-run-provider"></span><span class="run-detail-value">${esc(I18n.label(LLM_PROVIDER_LABELS, String(run.provider)))}</span></div>` : ""}
            ${run.tokens_used ? `<div class="run-detail-item"><span class="run-detail-label" data-l10n-id="assistant-automation-run-tokens"></span><span class="run-detail-value">${esc(formatNumber(Number(run.tokens_used), 0))}</span></div>` : ""}
          </div>
          ${run.error_message ? `<div class="run-error-box"><i class="icon-triangle-alert"></i> ${esc(run.error_message)}</div>` : ""}
          ${
            toolCalls.length > 0
              ? `
            <div class="run-tools-section">
              <h4>${esc(I18n.t("assistant-automation-run-tools-title", { amount: formatNumber(toolCalls.length, 0) }))}</h4>
              <div class="run-tools-list">
                ${toolCalls
                  .map((tc) => {
                    const toolId = tc.tool_name || tc.name;
                    const toolLabel = toolId
                      ? I18n.label(AGENT_TOOL_LABELS, toolId)
                      : I18n.t("assistant-chat-tool-unknown");
                    const statusKey = String(tc.status || "").toLowerCase();
                    const statusLabel = tc.status
                      ? I18n.label(
                          TOOL_CALL_STATUS_LABELS,
                          Object.hasOwn(TOOL_CALL_STATUS_LABELS, statusKey) ? statusKey : "pending"
                        )
                      : "—";
                    return `
                  <div class="run-tool-item">
                    <span class="tool-name" title="${esc(toolId || "")}">${esc(toolLabel)}</span>
                    <span class="tool-status ${tc.status === "Executed" ? "success" : "failed"}">${esc(statusLabel)}</span>
                  </div>
                `;
                  })
                  .join("")}
              </div>
            </div>
          `
              : ""
          }
          ${
            run.ai_response
              ? `
            <div class="run-response-section">
              <h4 data-l10n-id="assistant-automation-run-response"></h4>
              <div class="run-response-content">${esc(run.ai_response)}</div>
            </div>
          `
              : ""
          }
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" data-l10n-id="common-action-close" onclick="this.closest('.modal-overlay').remove()"></button>
        </div>
      </div>
    `;
      I18n.localizeTree(modal);
      document.body.appendChild(modal);
    } catch (error) {
      Utils.showToast({ type: "error", title: error.message });
    }
  }

  function showAutomationMenu(event, id) {
    event.preventDefault();
    event.stopPropagation();

    const btn = event.currentTarget;
    if (activeAutomationMenu?.trigger === btn) {
      activeAutomationMenu.close();
      return;
    }
    activeAutomationMenu?.close("superseded");

    const menu = document.createElement("div");
    menu.className = "automation-context-menu";
    menu.setAttribute("role", "menu");
    menu.innerHTML = `
    <button class="context-menu-item" type="button" role="menuitem" data-action="edit">
      <i class="icon-square-pen"></i> <span data-l10n-id="common-action-edit"></span>
    </button>
    <button class="context-menu-item" type="button" role="menuitem" data-action="runs">
      <i class="icon-clock"></i> <span data-l10n-id="assistant-automation-view-runs"></span>
    </button>
    <hr>
    <button class="context-menu-item danger" type="button" role="menuitem" data-action="delete">
      <i class="icon-trash"></i> <span data-l10n-id="common-action-delete"></span>
    </button>
  `;
    I18n.localizeTree(menu);
    document.body.appendChild(menu);

    let stopPositionTracking = null;
    let closeTimer = null;
    const handle = {
      trigger: btn,
      owns: (target) => menu.contains(target) || btn.contains(target),
      close: (reason) => {
        if (activeAutomationMenu === handle) activeAutomationMenu = null;
        stopPositionTracking?.();
        stopPositionTracking = null;
        btn.setAttribute("aria-expanded", "false");
        menu.removeEventListener("click", onClick);
        menu.removeEventListener("keydown", onKeyDown);
        closeMenu(handle);
        menu.classList.remove("open");
        if (reason === "escape") btn.focus({ preventScroll: true });

        const finish = () => {
          if (closeTimer !== null) clearTimeout(closeTimer);
          closeTimer = null;
          if (activeAutomationMenu?.trigger !== btn) {
            btn.classList.remove("active", "menu-above");
          }
          menu.remove();
        };
        if (
          [
            "superseded",
            "outside-pointer",
            "focus-left",
            "document-hidden",
            "navigation",
            "dialog-open",
          ].includes(reason)
        ) {
          finish();
        } else {
          closeTimer = setTimeout(finish, 220);
        }
      },
    };
    const onClick = (clickEvent) => {
      const action = clickEvent.target.closest("[data-action]")?.dataset.action;
      if (!action) return;
      handle.close();
      if (action === "edit") editAutomationTask(id);
      else if (action === "runs") viewAutomationTaskRuns(id);
      else if (action === "delete") deleteAutomationTask(id);
    };
    const onKeyDown = (keyEvent) => {
      const items = Array.from(menu.querySelectorAll("[role='menuitem']"));
      const index = items.indexOf(document.activeElement);
      if (keyEvent.key === "ArrowDown" || keyEvent.key === "ArrowUp") {
        keyEvent.preventDefault();
        const direction = keyEvent.key === "ArrowDown" ? 1 : -1;
        const nextIndex =
          index < 0
            ? direction > 0
              ? 0
              : items.length - 1
            : (index + direction + items.length) % items.length;
        items[nextIndex]?.focus();
      } else if (keyEvent.key === "Home" || keyEvent.key === "End") {
        keyEvent.preventDefault();
        items[keyEvent.key === "Home" ? 0 : items.length - 1]?.focus();
      }
    };

    activeAutomationMenu = handle;
    openMenu(handle);
    btn.classList.add("active");
    btn.setAttribute("aria-haspopup", "menu");
    btn.setAttribute("aria-expanded", "true");
    menu.addEventListener("click", onClick);
    menu.addEventListener("keydown", onKeyDown);
    stopPositionTracking = trackAnchoredMenu({
      trigger: btn,
      menu,
      align: "end",
      onDetach: () => handle.close(),
    });
    requestAnimationFrame(() => {
      if (activeAutomationMenu === handle) {
        menu.classList.add("open");
        menu.querySelector("[role='menuitem']")?.focus({ preventScroll: true });
      }
    });
  }

  async function viewAutomationTaskRuns(id) {
    document.querySelectorAll(".modal-overlay.automation-modal-overlay").forEach((m) => m.remove());
    try {
      const response = await fetch(`/api/assistant/automation/${id}/runs`);
      if (!response.ok) {
        const err = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(err, I18n.t("assistant-automation-runs-load-failed")));
      }
      const data = await response.json();
      const task = state.automationTasks.find((t) => t.id === id);
      const runs = data.runs || [];

      const modal = document.createElement("div");
      modal.className = "modal-overlay automation-modal-overlay";
      modal.innerHTML = `
      <div class="modal-dialog automation-modal">
        <div class="modal-header">
          <h3><i class="icon-clock"></i> <span>${esc(I18n.t("assistant-automation-runs-history-title", { task: task?.name || I18n.t("assistant-automation-task-generic") }))}</span></h3>
          <button class="modal-close" data-l10n-id="assistant-modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
        </div>
        <div class="modal-body">
          ${
            runs.length === 0
              ? '<div class="automation-runs-empty" data-l10n-id="assistant-automation-task-runs-empty"></div>'
              : `<div class="automation-runs-list modal-runs-list">
              ${runs
                .map((run) => {
                  const statusIcon =
                    run.status === "success" ? "icon-circle-check" : "icon-circle-x";
                  const statusClass = run.status === "success" ? "success" : "failed";
                  const time = run.started_at ? Utils.formatTimestamp(run.started_at) : "";
                  const duration = run.duration_ms
                    ? formatTimeSpan(run.duration_ms / 1000, { decimals: 1 })
                    : "";
                  return `
                  <div class="automation-run-item ${statusClass}" onclick="window.assistantPage.viewAutomationRun(${run.id}); this.closest('.modal-overlay').remove();">
                    <i class="${statusIcon} run-status-icon"></i>
                    <div class="run-info">
                      <span class="run-task-name">${esc(task?.name || runTaskName(run.task_id))}</span>
                      <span class="run-time">${esc(time)}</span>
                    </div>
                    <span class="run-duration">${esc(duration)}</span>
                  </div>
                `;
                })
                .join("")}
            </div>`
          }
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" data-l10n-id="common-action-close" onclick="this.closest('.modal-overlay').remove()"></button>
        </div>
      </div>
    `;
      I18n.localizeTree(modal);
      document.body.appendChild(modal);
    } catch (error) {
      Utils.showToast({ type: "error", title: error.message });
    }
  }

  function setupAutomationHandlers() {
    const newBtn = $("#new-automation-btn");
    if (newBtn) {
      addTrackedListener(newBtn, "click", createAutomationTask);
    }
    const emptyBtn = $("#empty-add-automation-btn");
    if (emptyBtn) {
      addTrackedListener(emptyBtn, "click", createAutomationTask);
    }
  }

  // ============================================================================

  // Return public API
  return {
    loadAutomationTasks,
    loadAutomationRuns,
    loadAutomationStats,
    renderAutomationStats,
    renderAutomationList,
    renderAutomationRuns,
    createAutomationTask,
    saveNewAutomationTask,
    saveEditedAutomationTask,
    updateScheduleHint,
    toggleAutomationTask,
    editAutomationTask,
    deleteAutomationTask,
    runAutomationTask,
    viewAutomationRun,
    viewAutomationTaskRuns,
    showAutomationMenu,
    setupAutomationHandlers,
  };
}
