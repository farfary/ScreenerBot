// The sortable wallet list: one row per task with its state (and why it is
// paused), its execution mode, P&L with a trend, and its budget use.
import { sparkline } from "./charts.js";
import {
  MODE_LABELS,
  STATE_LABELS,
  fixed,
  pauseReasonShort,
  segmented,
  signedSol,
  taskName,
  toneClass,
} from "./format.js";

const SORTS = [
  { id: "pnl", label: "P&L" },
  { id: "state", label: "State" },
  { id: "name", label: "Name" },
];

const pnlOf = (task) =>
  (Number(task.stats?.realized_pnl_sol) || 0) + (Number(task.stats?.unrealized_pnl_sol) || 0);

function sorted(tasks, sort) {
  const list = [...tasks];
  if (sort === "name") return list.sort((a, b) => taskName(a).localeCompare(taskName(b)));
  if (sort === "state") {
    return list.sort(
      (a, b) =>
        Number(b.enabled) - Number(a.enabled) ||
        Number(b.mode === "live") - Number(a.mode === "live") ||
        taskName(a).localeCompare(taskName(b))
    );
  }
  return list.sort((a, b) => pnlOf(b) - pnlOf(a));
}

export function stateText(task) {
  if (!task.enabled) {
    const short = pauseReasonShort(task.pause_reason);
    return short ? `Paused · ${short}` : "Paused";
  }
  return STATE_LABELS[task.effective_state] || "Unknown";
}

export function createTaskList(page) {
  const { $, Utils, state, on } = page;
  const esc = Utils.escapeHtml;
  let lastHash = "";

  function setup() {
    on($("#copy-list-rows"), "click", (event) => {
      const row = event.target.closest("[data-task-id]");
      if (row) page.select(Number(row.dataset.taskId));
    });
    on($("#copy-list-sort"), "click", (event) => {
      const button = event.target.closest("[data-seg-value]");
      if (!button) return;
      state.sort = button.dataset.segValue;
      render();
    });
  }

  function row(task) {
    const selected = state.view === "task" && task.id === state.selectedId;
    const budget = Number(task.total_budget_sol) || 0;
    const spent = Number(task.spent_sol) || 0;
    const budgetPct = budget > 0 ? Math.min(100, (spent / budget) * 100) : 0;
    const pnl = pnlOf(task);
    const stateClass = task.enabled ? `is-${task.effective_state}` : "is-paused";
    return `<button type="button" class="copy-row${selected ? " is-selected" : ""}" data-task-id="${task.id}" aria-pressed="${selected}">
      <span class="copy-row-line">
        <span class="copy-row-name">${esc(taskName(task))}</span>
        <span class="copy-row-mode copy-mode-${esc(task.mode)}">${esc(MODE_LABELS[task.mode] || task.mode)}</span>
      </span>
      <span class="copy-row-line">
        <span class="copy-row-state ${stateClass}">${esc(stateText(task))}</span>
        <span class="copy-row-pnl ${toneClass(pnl)}">${esc(signedSol(pnl, 3))}</span>
      </span>
      <span class="copy-row-line copy-row-detail">
        <span class="copy-row-budget"><span class="copy-meter" aria-hidden="true"><span style="width:${budgetPct.toFixed(1)}%"></span></span><span>${esc(`${fixed(spent, 2)} / ${fixed(budget, 2)} SOL`)}</span></span>
        ${sparkline(task.pnl_trend)}
      </span>
    </button>`;
  }

  function render() {
    const tasks = state.overview?.tasks || [];
    const hash = JSON.stringify([tasks, state.sort, state.selectedId, state.view]);
    if (hash === lastHash) return;
    lastHash = hash;
    const active = tasks.filter((task) => task.enabled).length;
    const count = $("#copy-list-count");
    if (count) count.textContent = `${active} active · ${tasks.length} total`;
    const sort = $("#copy-list-sort");
    if (sort) sort.innerHTML = segmented("sort", SORTS, state.sort, esc, "Sort wallets");
    const rows = $("#copy-list-rows");
    if (rows) rows.innerHTML = sorted(tasks, state.sort).map(row).join("");
    const compare = $("#copy-compare-open");
    if (compare) {
      compare.classList.toggle("is-active", state.view === "compare");
      compare.disabled = tasks.length < 1;
    }
  }

  return { setup, render, invalidate: () => (lastHash = "") };
}
