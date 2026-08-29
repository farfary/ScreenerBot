import { $ } from "../../core/dom.js";
import { closeMenu, openMenu, trackAnchoredMenu } from "../../core/menu_manager.js";
import * as Utils from "../../core/utils.js";
import { ConfirmationDialog } from "../../ui/confirmation_dialog.js";

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
    el("auto-stat-total", stats.total_tasks || 0);
    el("auto-stat-active", stats.active_tasks || 0);
    el("auto-stat-runs", stats.total_runs || 0);
    el(
      "auto-stat-success-rate",
      stats.total_runs > 0
        ? Math.round((stats.successful_runs / stats.total_runs) * 100) + "%"
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
        <p class="empty-text">No scheduled tasks yet</p>
        <p class="empty-state-subtitle">Create your first automated AI task to get started</p>
        <button class="btn btn-secondary" onclick="window.assistantPage.createAutomationTask()">Create Your First Task</button>
      </div>
    `;
      return;
    }

    container.innerHTML = tasks
      .map((task) => {
        const statusClass = task.enabled ? "active" : "paused";
        const statusLabel = task.enabled ? "Active" : "Paused";
        const scheduleLabel = formatSchedule(task.schedule_type, task.schedule_value);
        const lastRun = task.last_run_at
          ? Utils.formatTimeAgo(new Date(task.last_run_at))
          : "Never";
        const nextRun =
          task.next_run_at && task.enabled
            ? Utils.formatTimeUntil(new Date(task.next_run_at))
            : "—";
        const permLabel = task.tool_permissions === "full" ? "Full Access" : "Read Only";
        const permClass = task.tool_permissions === "full" ? "full" : "readonly";

        return `
      <div class="automation-task-item" data-id="${task.id}">
        <div class="automation-task-info">
          <div class="automation-task-name">${Utils.escapeHtml(task.name)}</div>
          <div class="automation-task-meta">
            <span class="schedule-badge"><i class="icon-clock"></i> ${scheduleLabel}</span>
            <span class="perm-badge ${permClass}">${permLabel}</span>
            <span class="meta-sep">·</span>
            <span class="meta-text">Last: ${lastRun}</span>
            <span class="meta-sep">·</span>
            <span class="meta-text">Next: ${nextRun}</span>
          </div>
        </div>
        <div class="automation-task-actions">
          <span class="status-indicator ${statusClass}">${statusLabel}</span>
          <label class="toggle toggle-sm">
            <input type="checkbox" ${task.enabled ? "checked" : ""}
                   onchange="window.assistantPage.toggleAutomationTask(${task.id}, this.checked)">
            <span class="toggle-track"></span>
          </label>
          <button class="btn btn-sm btn-secondary" onclick="window.assistantPage.runAutomationTask(${task.id})" title="Run Now">
            <i class="icon-play"></i>
          </button>
          <button class="automation-menu-btn" type="button" aria-label="Automation actions" aria-haspopup="menu" aria-expanded="false" onclick="window.assistantPage.showAutomationMenu(event, ${task.id})">⋮</button>
        </div>
      </div>
    `;
      })
      .join("");
  }

  function renderAutomationRuns(runs) {
    const key = JSON.stringify(runs);
    if (key === _lastRunsKey) return;
    _lastRunsKey = key;

    const container = $("#automation-runs-list");
    const countEl = $("#auto-runs-count");
    if (!container) return;
    if (countEl) countEl.textContent = runs.length > 0 ? `${runs.length} runs` : "";

    if (!runs || runs.length === 0) {
      container.innerHTML = '<div class="automation-runs-empty">No runs yet</div>';
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
        const taskName =
          state.automationTasks.find((t) => t.id === run.task_id)?.name || `Task #${run.task_id}`;
        const time = run.started_at ? Utils.formatTimeAgo(new Date(run.started_at)) : "";
        const duration = run.duration_ms ? (run.duration_ms / 1000).toFixed(1) + "s" : "";

        return `
      <div class="automation-run-item ${statusClass}" onclick="window.assistantPage.viewAutomationRun(${run.id})">
        <i class="${statusIcon} run-status-icon"></i>
        <div class="run-info">
          <span class="run-task-name">${Utils.escapeHtml(taskName)}</span>
          <span class="run-time">${time}</span>
        </div>
        <span class="run-duration">${duration}</span>
      </div>
    `;
      })
      .join("");
  }

  function formatSchedule(type, value) {
    if (type === "interval") {
      const secs = parseInt(value);
      if (secs >= 3600) return `Every ${Math.round(secs / 3600)}h`;
      if (secs >= 60) return `Every ${Math.round(secs / 60)}m`;
      return `Every ${secs}s`;
    }
    if (type === "daily") return `Daily at ${value} UTC`;
    if (type === "weekly") {
      const parts = value.split(":");
      const days = parts[0];
      const time = parts.slice(1).join(":");
      return `${days} at ${time} UTC`;
    }
    return value;
  }

  async function createAutomationTask() {
    // Remove any existing automation modal
    document.querySelectorAll(".modal-overlay.automation-modal-overlay").forEach((m) => m.remove());
    const modal = document.createElement("div");
    modal.className = "modal-overlay automation-modal-overlay";
    modal.innerHTML = `
    <div class="modal automation-modal">
      <div class="modal-header">
        <h3><i class="icon-plus"></i> Create Automation Task</h3>
        <button class="modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label>Task Name</label>
          <input type="text" id="auto-name" placeholder="e.g., Portfolio Monitor">
        </div>
        <div class="form-group">
          <label>Instruction</label>
          <textarea id="auto-instruction" rows="6" class="instruction-editor" placeholder="What should the AI do? e.g., Check open positions for reversal signs and report findings."></textarea>
        </div>
        <div class="form-row">
          <div class="form-group form-group-half">
            <label>Schedule Type</label>
            <select id="auto-schedule-type" data-custom-select onchange="window.assistantPage.updateScheduleHint()">
              <option value="interval">Interval</option>
              <option value="daily">Daily</option>
              <option value="weekly">Weekly</option>
            </select>
          </div>
          <div class="form-group form-group-half">
            <label>Schedule Value</label>
            <input type="text" id="auto-schedule-value" placeholder="300">
            <small class="form-hint" id="schedule-hint">Interval in seconds (e.g., 300 = every 5 minutes)</small>
          </div>
        </div>
        <div class="form-row">
          <div class="form-group form-group-half">
            <label>Tool Permissions</label>
            <select id="auto-tool-permissions" data-custom-select>
              <option value="read_only">Read Only (safe)</option>
              <option value="full">Full Access (can trade)</option>
            </select>
          </div>
          <div class="form-group form-group-half">
            <label>Timeout (seconds)</label>
            <input type="number" id="auto-timeout" value="120" min="30" max="600">
          </div>
        </div>
        <div class="form-group">
          <div class="checkbox-group">
            <label class="checkbox-label checkbox-label--centered">
              <input type="checkbox" id="auto-notify-telegram" checked>
              <span>Notify via Telegram</span>
            </label>
            <label class="checkbox-label checkbox-label--centered">
              <input type="checkbox" id="auto-notify-success" checked>
              <span>Notify on success</span>
            </label>
            <label class="checkbox-label checkbox-label--centered">
              <input type="checkbox" id="auto-notify-failure" checked>
              <span>Notify on failure</span>
            </label>
          </div>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Cancel</button>
        <button class="btn btn-primary" onclick="window.assistantPage.saveNewAutomationTask()">
          <i class="icon-plus"></i> Create Task
        </button>
      </div>
    </div>
  `;
    document.body.appendChild(modal);
  }

  function updateScheduleHint() {
    const type = $("#auto-schedule-type")?.value;
    const hint = $("#schedule-hint");
    const input = $("#auto-schedule-value");
    if (!hint || !input) return;

    if (type === "interval") {
      hint.textContent = "Interval in seconds (e.g., 300 = every 5 minutes)";
      input.placeholder = "300";
    } else if (type === "daily") {
      hint.textContent = "Time in HH:MM UTC (e.g., 14:00)";
      input.placeholder = "14:00";
    } else if (type === "weekly") {
      hint.textContent = "Days and time: mon,wed,fri:09:00";
      input.placeholder = "mon,wed,fri:09:00";
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
        title: "Validation",
        message: "Please fill in all required fields",
      });
      return;
    }

    // Validate schedule value format
    if (scheduleType === "interval") {
      const secs = parseInt(scheduleValue);
      if (isNaN(secs) || secs < 60) {
        Utils.showToast({
          type: "error",
          title: "Validation",
          message: "Interval must be at least 60 seconds",
        });
        return;
      }
    } else if (scheduleType === "daily") {
      if (!/^([01]?\d|2[0-3]):[0-5]\d$/.test(scheduleValue)) {
        Utils.showToast({
          type: "error",
          title: "Validation",
          message: "Daily schedule must be in HH:MM format",
        });
        return;
      }
    } else if (scheduleType === "weekly") {
      if (!/^[a-z,]+(:\d{1,2}:\d{2})?$/i.test(scheduleValue)) {
        Utils.showToast({
          type: "error",
          title: "Validation",
          message: "Weekly schedule must be in format: mon,wed,fri:09:00",
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
        throw new Error(err.error || "Failed to create task");
      }

      document.querySelector(".modal-overlay")?.remove();
      Utils.showToast({ type: "success", title: "Task created" });
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
        throw new Error(err.error || "Failed to toggle task");
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
        throw new Error(err.error || "Failed to trigger task");
      }
      Utils.showToast({ type: "success", title: "Task triggered" });
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
      title: "Delete Task",
      message:
        "Are you sure you want to delete this automation task? This action cannot be undone.",
      confirmText: "Delete",
      type: "danger",
    });
    if (!confirmed) return;

    try {
      const response = await fetch(`/api/assistant/automation/${id}`, { method: "DELETE" });
      if (!response.ok) throw new Error("Failed to delete task");
      Utils.showToast({ type: "success", title: "Task deleted" });
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
    <div class="modal automation-modal">
      <div class="modal-header">
        <h3><i class="icon-square-pen"></i> Edit Task</h3>
        <button class="modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label>Task Name</label>
          <input type="text" id="edit-auto-name" value="${Utils.escapeHtml(task.name)}">
        </div>
        <div class="form-group">
          <label>Instruction</label>
          <textarea id="edit-auto-instruction" rows="6" class="instruction-editor">${Utils.escapeHtml(task.instruction)}</textarea>
        </div>
        <div class="form-row">
          <div class="form-group form-group-half">
            <label>Schedule Type</label>
            <select id="edit-auto-schedule-type" data-custom-select>
              <option value="interval" ${task.schedule_type === "interval" ? "selected" : ""}>Interval</option>
              <option value="daily" ${task.schedule_type === "daily" ? "selected" : ""}>Daily</option>
              <option value="weekly" ${task.schedule_type === "weekly" ? "selected" : ""}>Weekly</option>
            </select>
          </div>
          <div class="form-group form-group-half">
            <label>Schedule Value</label>
            <input type="text" id="edit-auto-schedule-value" value="${Utils.escapeHtml(task.schedule_value)}">
          </div>
        </div>
        <div class="form-row">
          <div class="form-group form-group-half">
            <label>Tool Permissions</label>
            <select id="edit-auto-tool-permissions" data-custom-select>
              <option value="read_only" ${task.tool_permissions !== "full" ? "selected" : ""}>Read Only</option>
              <option value="full" ${task.tool_permissions === "full" ? "selected" : ""}>Full Access</option>
            </select>
          </div>
          <div class="form-group form-group-half">
            <label>Timeout (seconds)</label>
            <input type="number" id="edit-auto-timeout" value="${task.timeout_seconds || 120}" min="30" max="600">
          </div>
        </div>
        <div class="form-group">
          <div class="checkbox-group">
            <label class="checkbox-label checkbox-label--centered">
              <input type="checkbox" id="edit-auto-notify-telegram" ${task.notify_telegram !== false ? "checked" : ""}>
              <span>Notify via Telegram</span>
            </label>
            <label class="checkbox-label checkbox-label--centered">
              <input type="checkbox" id="edit-auto-notify-success" ${task.notify_on_success !== false ? "checked" : ""}>
              <span>Notify on success</span>
            </label>
            <label class="checkbox-label checkbox-label--centered">
              <input type="checkbox" id="edit-auto-notify-failure" ${task.notify_on_failure !== false ? "checked" : ""}>
              <span>Notify on failure</span>
            </label>
          </div>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Cancel</button>
        <button class="btn btn-primary" onclick="window.assistantPage.saveEditedAutomationTask(${id})">
          <i class="icon-check"></i> Save Changes
        </button>
      </div>
    </div>
  `;
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
        title: "Validation",
        message: "Please fill in all required fields",
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
      if (!response.ok) throw new Error("Failed to update task");
      document.querySelector(".modal-overlay")?.remove();
      Utils.showToast({ type: "success", title: "Task updated" });
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
      if (!response.ok) throw new Error("Failed to load run details");
      const data = await response.json();
      const run = data.run;
      const taskName =
        state.automationTasks.find((t) => t.id === run.task_id)?.name || `Task #${run.task_id}`;
      let toolCalls = [];
      try {
        toolCalls = run.tool_calls ? JSON.parse(run.tool_calls) : [];
      } catch {
        /* malformed JSON */
      }

      const modal = document.createElement("div");
      modal.className = "modal-overlay automation-modal-overlay";
      modal.innerHTML = `
      <div class="modal automation-modal">
        <div class="modal-header">
          <h3><i class="icon-file-text"></i> Run Details</h3>
          <button class="modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
        </div>
        <div class="modal-body">
          <div class="run-detail-grid">
            <div class="run-detail-item"><span class="run-detail-label">Task</span><span class="run-detail-value">${Utils.escapeHtml(taskName)}</span></div>
            <div class="run-detail-item"><span class="run-detail-label">Status</span><span class="run-detail-value status-${run.status}">${Utils.escapeHtml(run.status)}</span></div>
            <div class="run-detail-item"><span class="run-detail-label">Started</span><span class="run-detail-value">${run.started_at ? new Date(run.started_at).toLocaleString() : "—"}</span></div>
            <div class="run-detail-item"><span class="run-detail-label">Duration</span><span class="run-detail-value">${run.duration_ms ? (run.duration_ms / 1000).toFixed(1) + "s" : "—"}</span></div>
            ${run.provider ? `<div class="run-detail-item"><span class="run-detail-label">Provider</span><span class="run-detail-value">${Utils.escapeHtml(String(run.provider))}</span></div>` : ""}
            ${run.tokens_used ? `<div class="run-detail-item"><span class="run-detail-label">Tokens</span><span class="run-detail-value">${Utils.escapeHtml(String(run.tokens_used))}</span></div>` : ""}
          </div>
          ${run.error_message ? `<div class="run-error-box"><i class="icon-triangle-alert"></i> ${Utils.escapeHtml(run.error_message)}</div>` : ""}
          ${
            toolCalls.length > 0
              ? `
            <div class="run-tools-section">
              <h4>Tool Calls (${toolCalls.length})</h4>
              <div class="run-tools-list">
                ${toolCalls
                  .map(
                    (tc) => `
                  <div class="run-tool-item">
                    <span class="tool-name">${Utils.escapeHtml(tc.tool_name || tc.name || "unknown")}</span>
                    <span class="tool-status ${tc.status === "Executed" ? "success" : "failed"}">${tc.status || "—"}</span>
                  </div>
                `
                  )
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
              <h4>AI Response</h4>
              <div class="run-response-content">${Utils.escapeHtml(run.ai_response)}</div>
            </div>
          `
              : ""
          }
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Close</button>
        </div>
      </div>
    `;
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
    <button type="button" role="menuitem" data-action="edit">
      <i class="icon-square-pen"></i> Edit
    </button>
    <button type="button" role="menuitem" data-action="runs">
      <i class="icon-clock"></i> View Runs
    </button>
    <hr>
    <button class="danger" type="button" role="menuitem" data-action="delete">
      <i class="icon-trash"></i> Delete
    </button>
  `;
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
      if (!response.ok) throw new Error("Failed to load runs");
      const data = await response.json();
      const task = state.automationTasks.find((t) => t.id === id);
      const runs = data.runs || [];

      const modal = document.createElement("div");
      modal.className = "modal-overlay automation-modal-overlay";
      modal.innerHTML = `
      <div class="modal automation-modal">
        <div class="modal-header">
          <h3><i class="icon-clock"></i> Run History — ${Utils.escapeHtml(task?.name || "Task")}</h3>
          <button class="modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
        </div>
        <div class="modal-body">
          ${
            runs.length === 0
              ? '<div class="automation-runs-empty">No runs yet for this task</div>'
              : `<div class="automation-runs-list modal-runs-list">
              ${runs
                .map((run) => {
                  const statusIcon =
                    run.status === "success" ? "icon-circle-check" : "icon-circle-x";
                  const statusClass = run.status === "success" ? "success" : "failed";
                  const time = run.started_at ? new Date(run.started_at).toLocaleString() : "";
                  const duration = run.duration_ms ? (run.duration_ms / 1000).toFixed(1) + "s" : "";
                  return `
                  <div class="automation-run-item ${statusClass}" onclick="window.assistantPage.viewAutomationRun(${run.id}); this.closest('.modal-overlay').remove();">
                    <i class="${statusIcon} run-status-icon"></i>
                    <div class="run-info">
                      <span class="run-task-name">${Utils.escapeHtml(task?.name || `Task #${run.task_id}`)}</span>
                      <span class="run-time">${time}</span>
                    </div>
                    <span class="run-duration">${duration}</span>
                  </div>
                `;
                })
                .join("")}
            </div>`
          }
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Close</button>
        </div>
      </div>
    `;
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
