// Copy Trading page: the status strip, totals, the wallet list and the selected
// task's workspace, plus the dialogs that create, arm, configure and vet tasks.
import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { requestManager } from "../core/request_manager.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { COPY_HANDOFF_EVENT, takeCopyHandoff } from "../ui/copy_handoff.js";
import { createApi } from "./copy/api.js";
import { createDialogs } from "./copy/dialogs.js";
import { paint } from "./copy/tokens.js";
import { plural } from "./copy/format.js";
import { renderFigures, renderStrip } from "./copy/summary.js";
import { createTaskList } from "./copy/list.js";
import { createWorkspace } from "./copy/workspace.js";
import { createCompare } from "./copy/compare.js";
import { createEditor } from "./copy/editor.js";
import { createArmGate } from "./copy/arm_gate.js";
import { createSettings } from "./copy/settings.js";
import { createProfile } from "./copy/profile.js";

const POLL_MS = 5000;

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

  function toast(type, title, message) {
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
        state.loadError = error.detail || "Request failed";
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

  async function loadFeatureLabel() {
    try {
      const features = await requestManager.fetch("/api/features");
      const label = $("#copy-feature-label");
      if (label) label.hidden = features?.trading?.copy_wallet !== "beta";
    } catch {
      // Without the feature map the page simply carries no Beta label.
    }
  }

  function render() {
    renderStrip(page);
    const error = $("#copy-load-error");
    if (error) {
      error.hidden = !state.loadError;
      error.textContent = state.loadError
        ? `Copy trading could not be loaded: ${state.loadError}`
        : "";
    }
    const tasks = state.overview?.tasks || [];
    const onboarding = $("#copy-onboarding");
    const main = $("#copy-main");
    if (onboarding) onboarding.hidden = !state.overview || tasks.length > 0;
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

  async function toggleGlobal(event) {
    const status = state.overview?.status;
    if (!status) return;
    const enabled = !status.enabled;
    const live = (state.overview.tasks || []).filter(
      (task) => task.enabled && task.mode === "live"
    ).length;
    if (enabled && live > 0) {
      const result = await ConfirmationDialog.show({
        title: "Resume copy processing",
        message: `${plural(live, "live task")} will submit real swaps when their wallets trade again.`,
        confirmLabel: "Resume processing",
        cancelLabel: "Keep paused",
        variant: "danger",
      });
      if (!result.confirmed) return;
    }
    const button = event.currentTarget;
    button.disabled = true;
    try {
      await page.api.patchConfig({ enabled });
      toast("success", enabled ? "Copy processing resumed" : "All copy processing paused");
      await load();
    } catch (error) {
      toast("error", "Copy processing could not be changed", error.detail);
    } finally {
      button.disabled = false;
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
        state,
        on,
        toast,
        paint,
        confirm: (config) => ConfirmationDialog.show(config),
        reload: load,
        reloadDefaults: loadDefaults,
        select,
        openCompare,
      };
      page.dialogs = createDialogs($, on);
      page.list = createTaskList(page);
      page.workspace = createWorkspace(page);
      page.compare = createCompare(page);
      page.editor = createEditor(page);
      page.arm = createArmGate(page);
      page.settings = createSettings(page);
      page.profile = createProfile(page);

      page.dialogs.setup($("#copy-page"));
      [
        page.list,
        page.workspace,
        page.compare,
        page.editor,
        page.arm,
        page.settings,
        page.profile,
      ].forEach((part) => part.setup());
      on($("#copy-add"), "click", () => page.editor.openCreate());
      on($("#copy-onboarding-add"), "click", () => page.editor.openCreate());
      on($("#copy-settings-open"), "click", () => page.settings.open());
      on($("#copy-global-action"), "click", toggleGlobal);
      on($("#copy-compare-open"), "click", openCompare);
      on(window, COPY_HANDOFF_EVENT, () => {
        if (ctxRef?.isActive()) applyHandoff();
      });

      render();
      void load();
      void loadDefaults();
      void loadFeatureLabel();
    },

    activate(ctx) {
      ctxRef = ctx;
      if (!poller) {
        poller = ctx.managePoller(
          new Poller(() => load(), { label: "CopyTrading", intervalMs: POLL_MS })
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
