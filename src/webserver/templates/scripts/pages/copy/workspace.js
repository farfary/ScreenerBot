// The selected task's workspace: the header (state, why it is paused, actions)
// and the Overview / Holdings / Activity / Rules / Execution tabs.
import { renderAddress } from "../../ui/token_identity.js";
import {
  MODE_LABELS,
  STATE_LABELS,
  pauseReasonText,
  plural,
  rangeQuery,
  taskName,
  timeAgo,
} from "./format.js";
import { forget } from "./tokens.js";
import { panelMessage, renderOverview } from "./overview.js";
import { renderExecution } from "./execution.js";
import { renderRules } from "./rules.js";
import { createHoldings } from "./holdings.js";
import { createActivity } from "./activity.js";

/** Analytics are heavier than the workspace read; a poll refreshes them at most this often. */
const INSIGHTS_TTL_MS = 15_000;

const TABS = [
  { id: "overview", label: "Overview" },
  { id: "holdings", label: "Holdings" },
  { id: "activity", label: "Activity" },
  { id: "rules", label: "Rules" },
  { id: "execution", label: "Execution" },
];

const RUNNING_DETAIL = {
  paper: "Running in Paper · trades are simulated, nothing is spent",
  live: "Running live · wallet trades are copied with real swaps",
  system_paused: "Waiting · copy processing is paused globally, exits still run",
  entries_blocked: "Entries blocked by the loss limit · exits still run",
  force_stopped: "Force stopped · nothing is copied",
};

export function createWorkspace(page) {
  const { $, Utils, api, requestManager, state, on, paint, toast, confirm } = page;
  const esc = Utils.escapeHtml;
  const insights = new Map();
  const insightErrors = new Map();
  const insightsFetchedAt = new Map();
  let ws = null;
  let wsError = null;
  let shellFor = null;
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
      const range = event.target.closest('[data-seg="range"] [data-seg-value]');
      if (range) return setRange(range.dataset.segValue);
      const action = event.target.closest("[data-ws-action]");
      if (action) void runAction(action.dataset.wsAction, action);
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
      wsError = null;
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
      paint(root, panelMessage("Select a wallet to open its workspace.", esc));
      return;
    }
    if (shellFor !== task.id) buildShell(root, task.id);
    renderHeader(task);
    renderTabs(task);
    renderPanel();
  }

  function buildShell(root, id) {
    shellFor = id;
    forget(root);
    root.innerHTML = `<header class="copy-ws-head">
      <div class="copy-ws-identity">
        <div class="copy-ws-title"><h2 id="copy-ws-name"></h2><span id="copy-ws-mode"></span></div>
        <div class="copy-ws-address" id="copy-ws-address"></div>
      <div class="copy-ws-state" id="copy-ws-state" aria-live="polite"></div>
      </div>
      <div class="copy-ws-actions" id="copy-ws-actions"></div>
    </header>
    <div class="copy-tabs" role="tablist" aria-label="Task views" id="copy-ws-tabs"></div>
    <section class="copy-ws-panel" id="copy-ws-panel" role="tabpanel"></section>`;
    activity.reset(id);
    holdings.reset();
  }

  function stateHtml(task) {
    if (task.enabled) {
      return esc(
        RUNNING_DETAIL[task.effective_state] || STATE_LABELS[task.effective_state] || "Unknown"
      );
    }
    const since = task.paused_at ? ` · ${timeAgo(task.paused_at)}` : "";
    const kind = task.pause_reason?.kind;
    const resume =
      kind === "latency_kill_switch"
        ? "Resuming keeps the same limit, so it pauses again while trades still arrive late. Check the RPC stream or raise the arrival limit in Settings."
        : kind === "watch_detached"
          ? "Resuming watches the wallet again."
          : "";
    // Pausing stops new copies only; say what still closes the holdings.
    const open = Number(task.stats?.open_positions) || 0;
    const closer =
      task.exit_mode === "buy_only"
        ? "Its exit rules"
        : task.exit_mode === "mirror"
          ? "The wallet's sells"
          : "The wallet's sells and its exit rules";
    const holdings = open ? `${closer} still close its ${plural(open, "open holding")}.` : "";
    const hint = [resume, holdings].filter(Boolean).join(" ");
    const budget = kind === "watch_budget_exceeded"
      ? `<div class="copy-watch-resume"><small>A higher limit makes more signature-page and transaction-detail RPC calls. It may use more provider credits and can still fall behind. Resume starts from the current wallet head, so signatures since the last completed check will not be copied.</small><label class="copy-watch-budget-field">Signatures checked per poll<input data-watch-page-budget type="number" min="500" max="5000" step="100" value="${Math.min(5000, Math.max(500, (Number(task.pause_reason.page_budget) + 5) * 100))}" /></label><label class="checkbox-label copy-watch-ack"><input type="checkbox" data-watch-resume-ack /><span>I understand signatures since the last completed check will be skipped.</span></label></div>`
      : "";
    return `${esc(pauseReasonText(task.pause_reason) + since)}${hint ? `<small>${esc(hint)}</small>` : ""}${budget}`;
  }

  function actionsHtml(task) {
    const button = (action, cls, icon, label) =>
      `<button class="btn ${cls} btn-sm" type="button" data-ws-action="${action}"><i class="${icon}" aria-hidden="true"></i> ${esc(label)}</button>`;
    return [
      task.enabled
        ? button("pause", "btn-outline", "icon-pause", "Pause")
        : task.pause_reason?.kind === "watch_budget_exceeded"
          ? button("resume-budget", "btn-primary", "icon-play", "Resume from now")
          : button("resume", "btn-primary", "icon-play", "Resume"),
      task.mode === "live"
        ? button("paper", "btn-outline", "icon-rotate-ccw", "Return to Paper")
        : "",
      button("edit", "btn-secondary", "icon-pencil", "Edit rules"),
      button("clone", "btn-ghost", "icon-copy-plus", "Clone"),
      button("profile", "btn-ghost", "icon-user-search", "Wallet profile"),
      button("delete", "btn-ghost copy-danger-action", "icon-trash-2", "Delete"),
    ].join("");
  }

  function renderHeader(task) {
    const name = $("#copy-ws-name");
    if (name) name.textContent = taskName(task);
    const mode = $("#copy-ws-mode");
    if (mode) {
      mode.textContent = MODE_LABELS[task.mode] || task.mode;
      mode.className = `copy-row-mode copy-mode-${task.mode}`;
    }
    paint($("#copy-ws-address"), renderAddress(task.target_address, { explorer: "account" }));
    const stateNode = $("#copy-ws-state");
    if (stateNode) {
      stateNode.dataset.state = !task.enabled
        ? "paused"
        : ["paper", "live"].includes(task.effective_state)
          ? "running"
          : "blocked";
      paint(stateNode, stateHtml(task));
    }
    paint($("#copy-ws-actions"), actionsHtml(task));
  }

  function renderTabs(task) {
    const counts = { holdings: task.stats?.open_positions, activity: task.stats?.decisions };
    paint(
      $("#copy-ws-tabs"),
      TABS.map((tab) => {
        const active = tab.id === state.tab;
        const count = counts[tab.id]
          ? `<span class="copy-tab-count">${esc(String(counts[tab.id]))}</span>`
          : "";
        return `<button type="button" role="tab" id="copy-tab-${tab.id}" class="copy-tab${active ? " is-active" : ""}" data-tab="${tab.id}" aria-selected="${active}">${esc(tab.label)}${count}</button>`;
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
          ? panelMessage(`This task could not be loaded: ${wsError}`, esc, "is-error")
          : panelMessage("Loading task…", esc)
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
    paint(panel, html);
  }

  async function run(button, work, success, failure) {
    button.disabled = true;
    try {
      await work();
      toast("success", success);
      await page.reload();
    } catch (error) {
      toast("error", failure, error.detail);
    } finally {
      if (button.isConnected) button.disabled = false;
    }
  }

  async function resumeBudget(task, button) {
    const signatureLimit = Number($("[data-watch-page-budget]")?.value);
    const pageBudget = signatureLimit / 100;
    if (!Number.isInteger(pageBudget) || pageBudget < 5 || pageBudget > 50) {
      toast("error", "Choose between 500 and 5,000 signatures per poll in 100-signature steps");
      return;
    }
    if (!$("[data-watch-resume-ack]")?.checked) {
      toast("error", "Acknowledge that signatures since the last completed check will be skipped");
      return;
    }
    if (task.mode === "live") {
      const result = await confirm({
        title: "Resume live copying",
        message: `“${taskName(task)}” may submit real swaps after the watch resumes. Signatures since the last completed check will not be copied.`,
        confirmLabel: "Resume live",
        cancelLabel: "Keep paused",
        variant: "danger",
      });
      if (!result.confirmed) return;
    }
    await run(button, async () => {
      const { targets = [] } = await requestManager.fetch("/api/wallets/watch");
      const target = targets.find((item) => item.address === task.target_address);
      if (!target) throw new Error("The wallet watch target could not be found");
      if (target.enabled) {
        if (target.page_budget !== pageBudget) {
          await requestManager.fetch(`/api/wallets/watch/${target.id}/budget`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ page_budget: pageBudget }),
            priority: "high",
            skipDedup: true,
          });
        }
      } else {
        await requestManager.fetch(`/api/wallets/watch/${target.id}/resume`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ page_budget: pageBudget, acknowledge_missed_activity: true }),
          priority: "high",
          skipDedup: true,
        });
      }
      await api.update(task.id, { enabled: true });
    }, "Wallet watch and copy task resumed from now", "Wallet watch or copy task could not be resumed");
  }

  async function runAction(action, button) {
    const task = current() || selectedSummary();
    if (!task) return;
    if (action === "resume-budget") {
      await resumeBudget(task, button);
    } else if (action === "pause" || action === "resume") {
      const enabled = action === "resume";
      if (enabled && task.mode === "live") {
        const result = await confirm({
          title: "Resume live copying",
          message: `“${taskName(task)}” will submit real swaps from your wallet when this wallet trades again.`,
          confirmLabel: "Resume live",
          cancelLabel: "Keep paused",
          variant: "danger",
        });
        if (!result.confirmed) return;
      }
      await run(
        button,
        () => api.update(task.id, { enabled }),
        enabled ? "Task resumed" : "Task paused",
        "Task state could not be changed"
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
        title: "Return to Paper",
        message: `New copies by “${taskName(task)}” will be simulated again, without spending SOL.`,
        confirmLabel: "Return to Paper",
        cancelLabel: "Keep live",
        variant: "warning",
      });
      if (!result.confirmed) return;
      await run(
        button,
        () => api.setMode(task.id, "paper"),
        "Task returned to Paper",
        "Execution mode could not be changed"
      );
    } else if (action === "delete") {
      const result = await confirm({
        title: "Delete copy task",
        message: `Delete “${taskName(task)}”? Its decisions and paper results are removed and the wallet is no longer watched for this task.`,
        confirmLabel: "Delete task",
        cancelLabel: "Keep task",
        variant: "danger",
      });
      if (!result.confirmed) return;
      await run(
        button,
        async () => {
          await api.remove(task.id);
          state.selectedId = null;
        },
        "Copy task deleted",
        "Copy task could not be deleted"
      );
    }
  }

  return { setup, render, refresh };
}
