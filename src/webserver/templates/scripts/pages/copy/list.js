// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The sortable wallet list: one row per task with its name (an unnamed task's short
// wallet address), its execution mode, its state (and why it is paused), its net P&L
// with a trend, its budget use, and a warning while its exit overrides no longer fit
// the Trader's exit rules.
import { renderAddress } from "../../ui/token_identity.js";
import { sparkline } from "../../ui/inline_chart.js";
import { exitPolicyIssueHtml, signedSol, solNumber, stateText, toneClass } from "./format.js";
import { tradingModeMarker } from "../../ui/trading_mode.js";
import { keepSegmentFocus, renderSegmented } from "../../ui/segmented.js";

const SORTS = [
  {
    value: "pnl",
    get label() {
      return I18n.t("copy-sort-pnl");
    },
  },
  {
    value: "state",
    get label() {
      return I18n.t("copy-sort-state");
    },
  },
  {
    value: "name",
    get label() {
      return I18n.t("copy-sort-name");
    },
  },
];

/** The name a row sorts by: the task's label, else its wallet address. */
const sortName = (task) => task.label || task.target_address || "";

function sorted(tasks, sort) {
  const list = [...tasks];
  if (sort === "name") return list.sort((a, b) => sortName(a).localeCompare(sortName(b)));
  if (sort === "state") {
    return list.sort(
      (a, b) =>
        Number(b.enabled) - Number(a.enabled) ||
        Number(b.mode === "live") - Number(a.mode === "live") ||
        sortName(a).localeCompare(sortName(b))
    );
  }
  return list.sort((a, b) => (Number(b.net_pnl_native) || 0) - (Number(a.net_pnl_native) || 0));
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
    on($("#copy-list-sort"), "change", (event) => {
      const input = event.target.closest("input[data-seg-value]");
      if (!input) return;
      state.sort = input.value;
      render();
    });
  }

  function row(task) {
    const selected = state.view === "task" && task.id === state.selectedId;
    const budget = Number(task.total_budget_native) || 0;
    const spent = Number(task.spent_native) || 0;
    const budgetPct = budget > 0 ? Math.min(100, (spent / budget) * 100) : 0;
    const stateClass = task.enabled ? `is-${task.effective_state}` : "is-paused";
    // An unnamed task is told apart by its wallet, in its short form in the name slot;
    // the row is the action, so the address is plain and its tooltip carries it in full.
    const name = task.label
      ? esc(task.label)
      : renderAddress(task.target_address, { plain: true, compact: true });
    return `<button type="button" class="copy-row" data-task-id="${task.id}"${selected ? ' aria-current="true"' : ""}>
      <span class="copy-row-line">
        <span class="copy-row-name">${name}</span>
        ${tradingModeMarker(task.mode, esc)}
      </span>
      <span class="copy-row-line">
        <span class="copy-row-state ${stateClass}">${esc(stateText(task))}</span>
        <span class="copy-row-pnl ${toneClass(task.net_pnl_native)}">${esc(signedSol(task.net_pnl_native))}</span>
      </span>
      <span class="copy-row-line copy-row-detail">
        <span class="copy-row-budget"><span class="copy-meter" aria-hidden="true"><span style="width:${budgetPct.toFixed(1)}%"></span></span><span>${esc(I18n.t("copy-budget-of", { spent: solNumber(spent, 2), budget: solNumber(budget, 2) }))}</span></span>
        ${sparkline(task.pnl_trend)}
      </span>${exitPolicyIssueHtml(task, esc, "copy-row-line copy-row-warning")}
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
    if (sort) {
      keepSegmentFocus(sort, () => {
        sort.innerHTML = renderSegmented(
          {
            name: "sort",
            options: SORTS,
            value: state.sort,
            ariaLabel: I18n.t("copy-list-sort-label"),
          },
          esc
        );
      });
    }
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
