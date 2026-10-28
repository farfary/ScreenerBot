// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The selected task's sheet: the header (name, wallet, state, wallet watch, actions),
// the Overview / Holdings / Activity / Rules / Execution tabs, and the task notices under
// the tab row (why the task is paused and how it comes back, exit rules that no longer fit).
import { renderAddress } from "../../ui/token_identity.js";
import { rangeQuery, stateLabel, stateText, taskName, timeAgo } from "./format.js";
import { tradingModeMarker } from "../../ui/trading_mode.js";
import { forgetPaint } from "../../core/dom.js";
import { attachTabScrollStrip } from "../../ui/tab_bar.js";
import { renderOverview } from "./overview.js";
import { renderNotice } from "../../ui/notice.js";
import { keepSegmentFocus } from "../../ui/segmented.js";
import { renderStateView } from "../../ui/state_view.js";
import { renderExecution } from "./execution.js";
import { renderRules } from "./rules.js";
import { createHoldings } from "./holdings.js";
import { createActivity } from "./activity.js";
import { postWatch, watchTarget } from "./watch_restore.js";

/** Analytics are heavier than the workspace read; a poll refreshes them at most this often. */
const INSIGHTS_TTL_MS = 15_000;
const WATCH_STATUS_TTL_MS = 15_000;

const TABS = [
  {
    id: "overview",
    get label() {
      return I18n.t("copy-tab-overview");
    },
  },
  {
    id: "holdings",
    get label() {
      return I18n.t("copy-tab-holdings");
    },
  },
  {
    id: "activity",
    get label() {
      return I18n.t("copy-tab-activity");
    },
  },
  {
    id: "rules",
    get label() {
      return I18n.t("copy-tab-rules");
    },
  },
  {
    id: "execution",
    get label() {
      return I18n.t("copy-tab-execution");
    },
  },
];

// Task states with a sentence of their own; `effective_state` in
// src/trader/copy/control.rs, without the paused state.
const STATE_DETAIL_LABELS = Object.freeze({
  paper: "copy-state-detail-paper",
  live: "copy-state-detail-live",
  system_paused: "copy-state-detail-system-paused",
  entries_blocked: "copy-state-detail-entries-blocked",
  force_stopped: "copy-state-detail-force-stopped",
});

/** What a paused task's notice says about its cause, by pause kind. */
const PAUSE_HINTS = Object.freeze({
  latency_kill_switch: "copy-paused-resume-latency",
  watch_detached: "copy-paused-resume-detached",
  watch_budget_exceeded: "copy-watch-recovery-budget-intro",
  helius_unavailable: "copy-watch-recovery-provider-failed",
  watch_processing_failed: "copy-watch-recovery-processing-failed",
});

/** The notice action of each way back the server names (`recovery`); a plain resume has none. */
const RECOVERY_ACTIONS = Object.freeze({
  restore_watch: { id: "restore-watch", label: "copy-watch-recovery-title" },
  retry_watch: { id: "retry-watch", label: "copy-action-retry-watch" },
});

/** The editor step an exit rule problem opens. */
const EXITS_STEP = "exits";

/** What still closes a paused task's holdings, by who decides its exits. */
function pausedHoldings(exitMode, count) {
  if (exitMode === "buy_only") return I18n.t("copy-paused-holdings-rules", { count });
  if (exitMode === "mirror") return I18n.t("copy-paused-holdings-mirror", { count });
  return I18n.t("copy-paused-holdings-hybrid", { count });
}

export function createWorkspace(page) {
  const { $, Utils, api, requestManager, state, on, paint, notify, confirm } = page;
  const esc = Utils.escapeHtml;
  const insights = new Map();
  const insightErrors = new Map();
  const insightsFetchedAt = new Map();
  let ws = null;
  let wsError = null;
  let shellFor = null;
  const watchStatuses = new Map();
  const holdings = createHoldings(page, { rerender: renderPanel, showActivityFor });
  const activity = createActivity(page, { rerender: renderPanel });

  const selectedSummary = () =>
    state.overview?.tasks?.find((task) => task.id === state.selectedId) || null;
  const current = () => (ws && ws.id === state.selectedId ? ws : null);
  const tabRange = () => (state.tab === "holdings" ? "all" : state.range);

  function setup() {
    const root = $("#copy-workspace");
    on(root, "click", (event) => {
      const tab = event.target.closest("[data-tab]");
      if (tab) return switchTab(tab.dataset.tab);
      const notice = event.target.closest("#copy-ws-notices [data-action]");
      if (notice) return void runNoticeAction(notice.dataset.action, notice);
      const action = event.target.closest("[data-ws-action]");
      if (action) void runAction(action.dataset.wsAction, action);
    });
    on(root, "change", (event) => {
      const range = event.target.closest('[data-seg="range"] input[data-seg-value]');
      if (range) setRange(range.value);
    });
    holdings.setup(root);
    activity.setup(root);
  }

  async function refresh() {
    const id = state.selectedId;
    if (state.view !== "task" || id === null) return;
    let changed = false;
    try {
      const next = await api.workspace(id);
      if (id !== state.selectedId) return;
      changed = ws?.id === id && ws.stats?.decisions !== next.stats?.decisions;
      ws = next;
      holdings.sync(next);
      wsError = null;
      await refreshWatchStatus(id, next.target_address);
    } catch (error) {
      if (id !== state.selectedId) return;
      wsError = error.detail;
    }
    render();
    await refreshTab(id, { force: changed });
  }

  async function refreshTab(id, { force = false } = {}) {
    if (id === null) return;
    const tab = state.tab;
    if (tab === "activity") {
      await activity.refresh(id);
    } else if (tab !== "rules") {
      const rangeId = tabRange();
      const key = `${id}:${rangeId}`;
      const fresh = Date.now() - (insightsFetchedAt.get(key) || 0) < INSIGHTS_TTL_MS;
      if (fresh && !force && insights.has(key)) return;
      try {
        insights.set(key, await api.insights(id, rangeQuery(rangeId)));
        insightsFetchedAt.set(key, Date.now());
        insightErrors.delete(key);
      } catch (error) {
        insightErrors.set(key, error.detail);
      }
    }
    if (id === state.selectedId && tab === state.tab) renderPanel();
  }

  function switchTab(id) {
    if (!TABS.some((tab) => tab.id === id) || id === state.tab) return;
    state.tab = id;
    const task = current() || selectedSummary();
    if (task) renderTabs(task);
    renderPanel();
    void refreshTab(state.selectedId, { force: true });
  }

  function setRange(id) {
    if (id === state.range) return;
    state.range = id;
    renderPanel();
    void refreshTab(state.selectedId, { force: true });
  }

  function showActivityFor(mint) {
    activity.setMint(mint);
    state.tab = "activity";
    render();
    void refreshTab(state.selectedId);
  }

  function render() {
    const root = $("#copy-workspace");
    if (!root || state.view !== "task") return;
    root.hidden = false;
    const compare = $("#copy-compare");
    if (compare) compare.hidden = true;
    const task = current() || selectedSummary();
    if (!task) {
      shellFor = null;
      paint(root, renderStateView({ message: I18n.t("copy-workspace-select"), compact: true }));
      return;
    }
    if (shellFor !== task.id) buildShell(root, task.id);
    renderHeader(task);
    renderTabs(task);
    renderPanel();
  }

  function buildShell(root, id) {
    shellFor = id;
    forgetPaint(root);
    root.innerHTML = `<header class="copy-ws-head">
      <div class="copy-ws-identity">
        <div class="copy-ws-title"><h2 id="copy-ws-name"></h2><div id="copy-ws-mode"></div></div>
        <div class="copy-ws-address" id="copy-ws-address"></div>
        <div class="copy-ws-state" id="copy-ws-state" aria-live="polite"></div>
        <div class="copy-ws-watch-status" id="copy-ws-watch-status"></div>
      </div>
      <div class="copy-ws-actions" id="copy-ws-actions"></div>
    </header>
    <div class="tab-scroll-wrapper copy-ws-tab-strip">
      <div class="tab-row tab-scroll-area" role="tablist" aria-label="${esc(I18n.t("copy-tabs-label"))}" id="copy-ws-tabs"></div>
      <button type="button" class="tab-scroll-btn tab-scroll-btn--start" aria-label="${esc(I18n.attr("shell-tabs-scroll-start", "aria-label"))}" title="${esc(I18n.attr("shell-tabs-scroll-start", "title"))}"><i class="icon-chevron-left" aria-hidden="true"></i></button>
      <button type="button" class="tab-scroll-btn tab-scroll-btn--end" aria-label="${esc(I18n.attr("shell-tabs-scroll-end", "aria-label"))}" title="${esc(I18n.attr("shell-tabs-scroll-end", "title"))}"><i class="icon-chevron-right" aria-hidden="true"></i></button>
    </div>
    <div class="copy-ws-notices" id="copy-ws-notices" aria-live="polite" hidden></div>
    <section class="copy-ws-panel" id="copy-ws-panel" role="tabpanel"></section>`;
    attachTabScrollStrip($("#copy-ws-tabs"));
    activity.reset(id);
    holdings.reset();
  }

  /** The state line: what an enabled task is doing, or the short reason it is paused. */
  function stateLine(task) {
    if (!task.enabled) return stateText(task);
    return Object.hasOwn(STATE_DETAIL_LABELS, task.effective_state)
      ? I18n.label(STATE_DETAIL_LABELS, task.effective_state)
      : stateLabel(task.effective_state);
  }

  async function refreshWatchStatus(taskId, address) {
    if (!address) return;
    const cached = watchStatuses.get(address);
    if (cached && Date.now() - cached.fetchedAt < WATCH_STATUS_TTL_MS) return;
    try {
      const { targets = [] } = await requestManager.fetch("/api/wallets/watch");
      const target = targets.find((item) => item.address === address);
      const status = target
        ? await requestManager.fetch(`/api/wallets/watch/${target.id}/status`)
        : null;
      watchStatuses.set(address, { target, status, fetchedAt: Date.now() });
      if (state.selectedId === taskId && current()?.target_address === address)
        renderHeader(current());
    } catch {
      // Retain the last status while a transient watch-status request fails.
    }
  }

  function watchStatusHtml(task) {
    const watch = watchStatuses.get(task.target_address);
    const status = watch?.status;
    if (!status?.target?.enabled || status.mode !== "helius_high_activity") return "";
    const watching = status.catching_up
      ? I18n.t("copy-watch-state-catching-up")
      : I18n.t("copy-watch-state-watching");
    const lastCheck = status.last_checked_at
      ? ` ${I18n.t("copy-watch-last-check", { ago: timeAgo(status.last_checked_at) })}`
      : "";
    return `<small>${esc(`${watching}${lastCheck}`)}</small>`;
  }

  /**
   * True once a paused task's wallet watch runs again: the server names a watch repair as
   * the task's way back, and the watch reports itself enabled.
   */
  function watchRestored(task) {
    return (
      !task.enabled &&
      Object.hasOwn(RECOVERY_ACTIONS, task.recovery) &&
      Boolean(watchStatuses.get(task.target_address)?.status?.target?.enabled)
    );
  }

  function pausedNotice(task) {
    if (task.enabled) return "";
    if (watchRestored(task)) {
      const watch = watchStatuses.get(task.target_address).status;
      return renderNotice({
        variant: "info",
        title: watch.catching_up
          ? I18n.t("copy-watch-recovery-catching-up")
          : I18n.t("copy-watch-recovery-active"),
        message: I18n.t("copy-watch-recovery-still-paused"),
      });
    }
    const reason = I18n.text(task.pause_text);
    const kind = task.pause_reason?.kind;
    const hint = Object.hasOwn(PAUSE_HINTS, kind) ? I18n.label(PAUSE_HINTS, kind) : "";
    // Pausing stops new copies only; say what still closes the holdings.
    const open = Number(task.stats?.open_positions) || 0;
    const holdings = open ? pausedHoldings(task.exit_mode, open) : "";
    const action = Object.hasOwn(RECOVERY_ACTIONS, task.recovery)
      ? {
          id: RECOVERY_ACTIONS[task.recovery].id,
          label: I18n.t(RECOVERY_ACTIONS[task.recovery].label),
        }
      : null;
    return renderNotice({
      variant: task.recovery === "resume" && kind === "user" ? "info" : "warning",
      title: task.paused_at
        ? I18n.t("copy-paused-since", { reason, since: timeAgo(task.paused_at) })
        : reason,
      message: [hint, holdings].filter(Boolean).join(" "),
      action,
    });
  }

  function exitIssueNotice(task) {
    if (!task.exit_policy_issue) return "";
    return renderNotice({
      variant: "warning",
      message: I18n.text(task.exit_policy_issue),
      action: { id: "edit-rules", label: I18n.t("copy-action-edit-rules") },
    });
  }

  /** The header's resume: a plain resume, or resuming copying once the watch runs again. */
  function resumeButton(task, button) {
    if (task.enabled)
      return button("pause", "btn-outline", "icon-pause", I18n.t("copy-action-pause"));
    if (task.recovery === "resume")
      return button("resume", "btn-primary", "icon-play", I18n.t("copy-action-resume"));
    if (watchRestored(task))
      return button("resume", "btn-primary", "icon-play", I18n.t("copy-action-resume-copy"));
    return "";
  }

  function actionsHtml(task) {
    const button = (action, cls, icon, label) =>
      `<button class="btn ${cls} btn-sm" type="button" data-ws-action="${action}"><i class="${icon}" aria-hidden="true"></i> ${esc(label)}</button>`;
    return [
      resumeButton(task, button),
      task.mode === "live"
        ? button("paper", "btn-outline", "icon-rotate-ccw", I18n.t("copy-action-return-paper"))
        : "",
      button("edit", "btn-outline", "icon-pencil", I18n.t("copy-action-edit-rules")),
      button("clone", "btn-outline", "icon-copy-plus", I18n.t("copy-action-clone")),
      button("profile", "btn-outline", "icon-user-search", I18n.t("copy-action-profile")),
      button(
        "delete",
        "btn-outline copy-danger-action",
        "icon-trash-2",
        I18n.t("common-action-delete")
      ),
    ].join("");
  }

  function renderHeader(task) {
    const name = $("#copy-ws-name");
    if (name) name.textContent = taskName(task);
    paint($("#copy-ws-mode"), tradingModeMarker(task.mode, esc));
    paint($("#copy-ws-address"), renderAddress(task.target_address, { explorer: "account" }));
    const stateNode = $("#copy-ws-state");
    if (stateNode) {
      stateNode.dataset.state = !task.enabled
        ? "paused"
        : ["paper", "live"].includes(task.effective_state)
          ? "running"
          : "blocked";
      paint(stateNode, esc(stateLine(task)));
    }
    paint($("#copy-ws-watch-status"), watchStatusHtml(task));
    paint($("#copy-ws-actions"), actionsHtml(task));
    const notices = $("#copy-ws-notices");
    if (notices) {
      const html = pausedNotice(task) + exitIssueNotice(task);
      paint(notices, html);
      notices.hidden = !html;
    }
  }

  function renderTabs(task) {
    const counts = { holdings: task.stats?.open_positions, activity: task.stats?.decisions };
    paint(
      $("#copy-ws-tabs"),
      TABS.map((tab) => {
        const active = tab.id === state.tab;
        const count = counts[tab.id]
          ? `<span class="sub-tab-count">${esc(String(counts[tab.id]))}</span>`
          : "";
        return `<button type="button" role="tab" id="copy-tab-${tab.id}" class="sub-tab${active ? " active" : ""}" data-tab="${tab.id}" aria-selected="${active}">${esc(tab.label)}${count}</button>`;
      }).join("")
    );
  }

  function renderPanel() {
    const panel = $("#copy-ws-panel");
    if (!panel) return;
    panel.setAttribute("aria-labelledby", `copy-tab-${state.tab}`);
    const task = current();
    if (!task) {
      paint(
        panel,
        wsError
          ? renderStateView({
              kind: "error",
              message: I18n.t("copy-workspace-load-failed", { error: wsError }),
              compact: true,
            })
          : renderStateView({
              kind: "loading",
              message: I18n.t("copy-workspace-loading"),
              compact: true,
            })
      );
      return;
    }
    const key = `${task.id}:${tabRange()}`;
    const context = {
      ws: task,
      defaults: state.defaults,
      range: state.range,
      insights: insights.get(key) || null,
      error: insightErrors.get(key) || null,
    };
    const html =
      state.tab === "overview"
        ? renderOverview(context, esc)
        : state.tab === "execution"
          ? renderExecution(context, esc)
          : state.tab === "rules"
            ? renderRules(context, esc)
            : state.tab === "holdings"
              ? holdings.html(context)
              : activity.html(task);
    keepSegmentFocus(panel, () => paint(panel, html));
  }

  async function run(button, work, success, failure) {
    button.disabled = true;
    try {
      await work();
      notify("success", success);
      await page.reload();
    } catch (error) {
      notify("error", failure, error.detail);
    } finally {
      if (button.isConnected) button.disabled = false;
    }
  }

  async function retryWatch(task, button) {
    await run(
      button,
      async () => {
        const target = await watchTarget(requestManager, task.target_address);
        await postWatch(requestManager, target.id, "enabled", { enabled: true });
        watchStatuses.delete(task.target_address);
      },
      I18n.t("copy-watch-retry-started"),
      I18n.t("copy-watch-retry-failed")
    );
  }

  /** The one action of a task notice: repair the watch, or fix the exit rules. */
  function runNoticeAction(action, button) {
    const task = current() || selectedSummary();
    if (!task) return;
    if (action === "restore-watch") {
      page.watchRestore.open(task, watchStatuses.get(task.target_address)?.status);
    } else if (action === "retry-watch") {
      void retryWatch(task, button);
    } else if (action === "edit-rules") {
      page.editor.openEdit(task, EXITS_STEP);
    }
  }

  async function runAction(action, button) {
    const task = current() || selectedSummary();
    if (!task) return;
    if (action === "pause" || action === "resume") {
      const enabled = action === "resume";
      if (enabled && task.mode === "live") {
        const result = await confirm({
          title: I18n.t("copy-resume-live-title"),
          message: I18n.t("copy-resume-live-message", { name: taskName(task) }),
          confirmLabel: I18n.t("copy-resume-live-confirm"),
          cancelLabel: I18n.t("copy-keep-paused"),
          variant: "danger",
        });
        if (!result.confirmed) return;
      }
      await run(
        button,
        () => api.update(task.id, { enabled }),
        enabled ? I18n.t("copy-task-resumed") : I18n.t("copy-task-paused"),
        I18n.t("copy-task-state-failed")
      );
    } else if (action === "edit") {
      page.editor.openEdit(task, button.dataset.step);
    } else if (action === "clone") {
      page.editor.openClone(task);
    } else if (action === "profile") {
      page.profile.open(task.target_address, task.label);
    } else if (action === "arm") {
      if (current()) page.arm.open(current());
    } else if (action === "paper") {
      const result = await confirm({
        title: I18n.t("copy-action-return-paper"),
        message: I18n.t("copy-return-paper-message", { name: taskName(task) }),
        confirmLabel: I18n.t("copy-action-return-paper"),
        cancelLabel: I18n.t("copy-return-paper-cancel"),
        variant: "warning",
      });
      if (!result.confirmed) return;
      await run(
        button,
        () => api.setMode(task.id, "paper"),
        I18n.t("copy-task-returned-paper"),
        I18n.t("copy-mode-change-failed")
      );
    } else if (action === "delete") {
      const result = await confirm({
        title: I18n.t("copy-delete-title"),
        message: I18n.t("copy-delete-settle-message", { name: taskName(task) }),
        confirmLabel: I18n.t("copy-delete-confirm"),
        cancelLabel: I18n.t("copy-delete-cancel"),
        variant: "danger",
      });
      if (!result.confirmed) return;
      await run(
        button,
        async () => {
          await api.remove(task.id);
          state.selectedId = null;
        },
        I18n.t("copy-task-deleted"),
        I18n.t("copy-task-delete-failed")
      );
    }
  }

  return {
    setup,
    render,
    refresh,
    /** Drops the cached watch status of `address`, so the next refresh reads it again. */
    forgetWatch: (address) => watchStatuses.delete(address),
  };
}
