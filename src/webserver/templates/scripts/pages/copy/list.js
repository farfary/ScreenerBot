// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The sortable wallet list: one row per task with its state (and why it is
// paused), its execution mode, P&L with a trend, and its budget use.
import { renderAddress } from "../../ui/token_identity.js";
import { sparkline } from "./charts.js";
import {
  fixed,
  modeLabel,
  pauseReasonShort,
  segmented,
  signedSol,
  stateLabel,
  taskName,
  toneClass,
} from "./format.js";

const SORTS = [
  {
    id: "pnl",
    get label() {
      return I18n.t("copy-sort-pnl");
    },
  },
  {
    id: "state",
    get label() {
      return I18n.t("copy-sort-state");
    },
  },
  {
    id: "name",
    get label() {
      return I18n.t("copy-sort-name");
    },
  },
];

const pnlOf = (task) =>
  (Number(task.stats?.realized_pnl_native) || 0) + (Number(task.stats?.unrealized_pnl_native) || 0);

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
    return short
      ? I18n.t("copy-state-paused-reason", { reason: short })
      : I18n.t("copy-state-paused");
  }
  return stateLabel(task.effective_state);
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
    const budget = Number(task.total_budget_native) || 0;
    const spent = Number(task.spent_native) || 0;
    const budgetPct = budget > 0 ? Math.min(100, (spent / budget) * 100) : 0;
    const pnl = pnlOf(task);
    const stateClass = task.enabled ? `is-${task.effective_state}` : "is-paused";
    return `<button type="button" class="copy-row${selected ? " is-selected" : ""}" data-task-id="${task.id}" aria-pressed="${selected}">
      <span class="copy-row-line">
        <span class="copy-row-name">${esc(taskName(task))}</span>
        <span class="copy-row-mode copy-mode-${esc(task.mode)}">${esc(modeLabel(task.mode))}</span>
      </span>${
        // An unnamed task is told apart by its wallet, in full on a line of its own;
        // the row is the action, so the address is plain.
        task.label
          ? ""
          : `<span class="copy-row-line copy-row-address">${renderAddress(task.target_address, { plain: true })}</span>`
      }
      <span class="copy-row-line">
        <span class="copy-row-state ${stateClass}">${esc(stateText(task))}</span>
        <span class="copy-row-pnl ${toneClass(pnl)}">${esc(signedSol(pnl))}</span>
      </span>
      <span class="copy-row-line copy-row-detail">
        <span class="copy-row-budget"><span class="copy-meter" aria-hidden="true"><span style="width:${budgetPct.toFixed(1)}%"></span></span><span>${esc(I18n.t("copy-budget-of", { spent: fixed(spent, 2), budget: fixed(budget, 2) }))}</span></span>
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
    if (count) count.textContent = I18n.t("copy-list-count", { active, total: tasks.length });
    const sort = $("#copy-list-sort");
    if (sort)
      sort.innerHTML = segmented("sort", SORTS, state.sort, esc, I18n.t("copy-list-sort-label"));
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
