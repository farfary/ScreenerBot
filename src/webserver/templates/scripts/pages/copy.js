// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Copy Trading page: the toolbar and page notices, the all-tasks figures, the wallet
// list and the selected task's sheet, plus the dialogs that create, arm, configure and
// vet tasks and restore a wallet watch.
import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { paintHtml } from "../core/dom.js";
import { requestManager } from "../core/request_manager.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { COPY_HANDOFF_EVENT, takeCopyHandoff } from "../ui/copy_handoff.js";
import { createApi } from "./copy/api.js";
import { createDialogs } from "./copy/dialogs.js";
import { renderFigures, renderToolbar } from "./copy/summary.js";
import { createTaskList } from "./copy/list.js";
import { createWorkspace } from "./copy/workspace.js";
import { createCompare } from "./copy/compare.js";
import { createEditor } from "./copy/editor.js";
import { createArmGate } from "./copy/arm_gate.js";
import { createSettings } from "./copy/settings.js";
import { createProfile } from "./copy/profile.js";
import { createWatchRestore } from "./copy/watch_restore.js";
import { gateControl, renderSetupGate, setupRequired } from "../ui/setup_gate.js";

const POLL_MS = 5000;

/** Message key of each reason a bulk resume keeps a task paused. */
const KEPT_REASON_LABELS = Object.freeze({
  task_limit: "copy-resume-kept-limit",
  needs_watch_recovery: "copy-resume-kept-watch",
  live_not_included: "copy-resume-kept-live",
  failed: "copy-resume-kept-failed",
});

function initialState() {
  return {
    overview: null,
    defaults: null,
    selectedId: null,
    view: "task",
    sort: "pnl",
    tab: "overview",
    range: "all",
    loadError: null,
    toolbarBusy: false,
  };
}

function createLifecycle() {
  let ctxRef = null;
  let page = null;
  let poller = null;
  let listeners = [];
  let loading = null;
  const state = initialState();

  function on(target, type, handler, options) {
    if (!target) return;
    target.addEventListener(type, handler, options);
    listeners.push(() => target.removeEventListener(type, handler, options));
  }

  function notify(type, title, message) {
    Utils.showToast({ type, title, ...(message ? { message } : {}) });
  }

  function load() {
    if (loading) return loading;
    loading = (async () => {
      try {
        const overview = await page.api.overview();
        state.overview = overview;
        state.loadError = null;
        const tasks = overview.tasks || [];
        if (state.selectedId !== null && !tasks.some((task) => task.id === state.selectedId)) {
          state.selectedId = null;
        }
        if (state.selectedId === null) state.selectedId = tasks[0]?.id ?? null;
      } catch (error) {
        state.loadError = error.detail || I18n.t("copy-request-failed");
      }
      if (!page) return;
      render();
      if (state.view === "compare") await page.compare.refresh();
      else await page.workspace.refresh();
    })().finally(() => {
      loading = null;
    });
    return loading;
  }

  async function loadDefaults() {
    try {
      state.defaults = await page.api.defaults();
      page?.workspace.render();
    } catch (error) {
      console.warn("[Copy] Defaults unavailable:", error.detail);
    }
  }

  function render() {
    renderToolbar(page);
    const tasks = state.overview?.tasks || [];
    const onboarding = $("#copy-onboarding");
    const main = $("#copy-main");
    const gate = $("#copy-setup-gate");
    const gated = setupRequired();
    if (gate) gate.hidden = !gated || tasks.length > 0;
    if (onboarding) onboarding.hidden = gated || !state.overview || tasks.length > 0;
    if (main) main.hidden = tasks.length === 0;
    if (!tasks.length) return;
    renderFigures(page);
    page.list.render();
    if (state.view === "compare") page.compare.render();
    else page.workspace.render();
  }

  function select(id) {
    state.view = "task";
    state.selectedId = id;
    page.list.render();
    page.workspace.render();
    void page.workspace.refresh();
  }

  function openCompare() {
    state.view = state.view === "compare" ? "task" : "compare";
    page.list.render();
    if (state.view === "compare") {
      page.compare.render();
      void page.compare.refresh();
    } else {
      page.workspace.render();
      void page.workspace.refresh();
    }
  }

  function toggleGlobal(event) {
    const status = state.overview?.status;
    if (!status) return;
    void setProcessing(!status.enabled, event.currentTarget);
  }

  /** Disables the toolbar's bulk actions while one of them is saving. */
  function setToolbarBusy(busy) {
    state.toolbarBusy = busy;
    if (page) renderToolbar(page);
  }

  /**
   * Resumes every paused task a single Resume would restart, through the one server
   * path. With a live task among them it asks first, naming the server's live count, and
   * only then includes live tasks.
   */
  async function resumeTasks() {
    const status = state.overview?.status;
    if (!status) return;
    const live = Number(status.resumable_live_tasks) || 0;
    if (live > 0) {
      const result = await ConfirmationDialog.show({
        title: I18n.t("copy-resume-tasks-title"),
        message: I18n.t("copy-resume-tasks-live-message", { count: live }),
        confirmLabel: I18n.t("copy-resume-tasks-confirm"),
        cancelLabel: I18n.t("copy-keep-paused"),
        variant: "danger",
      });
      if (!result.confirmed) return;
    }
    setToolbarBusy(true);
    try {
      notifyResumed(await page.api.resumeTasks(live > 0));
    } catch (error) {
      notify("error", I18n.t("copy-toast-tasks-resume-failed"), error.detail);
    } finally {
      setToolbarBusy(false);
    }
    await load();
  }

  /** One toast for a bulk resume: how many resumed, and why the rest stayed paused. */
  function notifyResumed({ resumed = [], kept_paused: kept = [] } = {}) {
    const counts = new Map();
    const failures = new Set();
    let maximum = null;
    for (const { reason } of kept) {
      counts.set(reason.kind, (counts.get(reason.kind) || 0) + 1);
      if (reason.kind === "task_limit") maximum = reason.maximum;
      if (reason.kind === "failed" && reason.message) failures.add(I18n.text(reason.message));
    }
    const reasons = [...counts]
      .filter(([kind]) => Object.hasOwn(KEPT_REASON_LABELS, kind))
      .map(([kind, count]) => I18n.t(KEPT_REASON_LABELS[kind], { count, maximum }));
    const why = [...reasons, ...failures].join(" ");
    if (!kept.length) {
      notify("success", I18n.t("copy-toast-tasks-resumed", { count: resumed.length }));
    } else if (resumed.length) {
      notify(
        "success",
        I18n.t("copy-toast-tasks-resumed", { count: resumed.length }),
        `${I18n.t("copy-toast-tasks-kept-paused", { count: kept.length })} ${why}`.trim()
      );
    } else {
      notify("warning", I18n.t("copy-toast-tasks-kept-paused", { count: kept.length }), why);
    }
  }

  /**
   * Pauses or resumes copy processing for every task; resuming asks first while a
   * live task would submit real swaps. Resolves true once the change is saved.
   */
  async function setProcessing(enabled, button) {
    if (!state.overview) return false;
    const live = (state.overview.tasks || []).filter(
      (task) => task.enabled && task.mode === "live"
    ).length;
    if (enabled && live > 0) {
      const result = await ConfirmationDialog.show({
        title: I18n.t("copy-resume-all-title"),
        message: I18n.t("copy-resume-all-message", { count: live }),
        confirmLabel: I18n.t("copy-toolbar-resume"),
        cancelLabel: I18n.t("copy-keep-paused"),
        variant: "danger",
      });
      if (!result.confirmed) return false;
    }
    setToolbarBusy(true);
    if (button) button.disabled = true;
    try {
      await page.api.patchConfig({ enabled });
      notify(
        "success",
        enabled ? I18n.t("copy-toast-resumed-all") : I18n.t("copy-toast-paused-all")
      );
      await load();
      return true;
    } catch (error) {
      notify("error", I18n.t("copy-toast-global-failed"), error.detail);
      return false;
    } finally {
      setToolbarBusy(false);
      if (button) button.disabled = false;
    }
  }

  function applyHandoff() {
    const handoff = takeCopyHandoff();
    if (!handoff) return;
    if (Number.isFinite(handoff.taskId)) select(handoff.taskId);
    else if (handoff.address) page.profile.open(handoff.address, handoff.label);
  }

  return {
    init(ctx) {
      ctxRef = ctx;
      page = {
        $,
        Utils,
        api: createApi(requestManager),
        requestManager,
        state,
        on,
        notify,
        paint: paintHtml,
        confirm: (config) => ConfirmationDialog.show(config),
        reload: load,
        reloadDefaults: loadDefaults,
        select,
        openCompare,
        setProcessing,
      };
      page.dialogs = createDialogs($, on);
      page.list = createTaskList(page);
      page.workspace = createWorkspace(page);
      page.compare = createCompare(page);
      page.editor = createEditor(page);
      page.arm = createArmGate(page);
      page.settings = createSettings(page);
      page.profile = createProfile(page);
      page.watchRestore = createWatchRestore(page);

      page.dialogs.setup($("#copy-page"));
      [
        page.list,
        page.workspace,
        page.compare,
        page.editor,
        page.arm,
        page.settings,
        page.profile,
        page.watchRestore,
      ].forEach((part) => part.setup());
      on($("#copy-add"), "click", () => page.editor.openCreate());
      on($("#copy-onboarding-add"), "click", () => page.editor.openCreate());
      on($("#copy-settings-open"), "click", () => page.settings.open());
      on($("#copy-global-action"), "click", toggleGlobal);
      on($("#copy-resume-tasks"), "click", () => void resumeTasks());
      on($("#copy-notices"), "click", (event) => {
        if (event.target.closest('[data-action="retry"]')) void load();
      });
      on($("#copy-compare-open"), "click", openCompare);
      gateControl($("#copy-add"));
      if (setupRequired()) renderSetupGate($("#copy-setup-gate"), I18n.t("copy-setup-gate-title"));
      on(window, COPY_HANDOFF_EVENT, () => {
        if (ctxRef?.isActive()) applyHandoff();
      });

      render();
      void load();
      void loadDefaults();
    },

    activate(ctx) {
      ctxRef = ctx;
      if (!poller) {
        poller = ctx.managePoller(
          new Poller(() => load(), {
            label: "CopyTrading", // l10n-ignore: poller log name
            intervalMs: POLL_MS,
          })
        );
      }
      poller.start({ silent: true });
      applyHandoff();
      void load();
    },

    deactivate() {
      page?.dialogs.closeAll();
    },

    dispose() {
      page?.compare.dispose();
      poller?.stop({ silent: true });
      poller = null;
      listeners.forEach((remove) => remove());
      listeners = [];
      page = null;
      ctxRef = null;
      Object.assign(state, initialState());
    },
  };
}

registerPage("copy", createLifecycle());
