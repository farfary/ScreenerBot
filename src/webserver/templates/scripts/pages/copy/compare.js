// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Compare view: every task's closed rounds over one date range, as curves and a
// table. Choosing a wallet opens its workspace.
import { DataTable } from "../../ui/data_table.js";
import { addressFloorWidth, renderNamedAddress } from "../../ui/token_identity.js";
import { comparisonCurves } from "./charts.js";
import {
  RANGES,
  duration,
  fixed,
  modeLabel,
  pct,
  rangeQuery,
  seconds,
  segmented,
  signedPct,
  signedSolCell,
  taskName,
  toneClass,
} from "./format.js";
import { forget } from "./tokens.js";
import { panelMessage } from "./overview.js";

const TABLE_ID = "copy-compare-table";

export function createCompare(page) {
  const { $, Utils, api, state, on, paint } = page;
  const esc = Utils.escapeHtml;
  const data = new Map();
  const errors = new Map();

  function setup() {
    on($("#copy-compare"), "click", (event) => {
      const range = event.target.closest('[data-seg="compare-range"] [data-seg-value]');
      if (range) {
        state.range = range.dataset.segValue;
        render();
        void refresh();
        return;
      }
      if (event.target.closest("[data-compare-close]")) page.openCompare();
    });
  }

  async function refresh() {
    const range = state.range;
    try {
      const response = await api.compare(rangeQuery(range));
      data.set(range, response.tasks || []);
      errors.delete(range);
    } catch (error) {
      errors.set(range, error.detail);
    }
    if (state.view === "compare") render();
  }

  let table = null;

  const columns = () => [
    {
      id: "label",
      label: I18n.t("copy-table-wallet"),
      grow: true,
      sortable: true,
      minWidth: addressFloorWidth() + 24,
      render: (_value, row) => renderNamedAddress(taskName(row), row.target_address),
      sortFn: (a, b) => taskName(a).localeCompare(taskName(b)),
    },
    {
      id: "mode",
      label: I18n.t("copy-table-mode"),
      sortable: true,
      wrap: false,
      minWidth: "content",
      render: (_value, row) =>
        `<span class="copy-row-mode copy-mode-${esc(row.mode)}">${esc(modeLabel(row.mode))}</span>${row.enabled ? "" : `<small class="copy-muted"> ${esc(I18n.t("copy-paused-suffix"))}</small>`}`,
    },
    {
      id: "rounds",
      label: I18n.t("copy-table-rounds"),
      type: "number",
      sortable: true,
      render: (value) => esc(String(value)),
    },
    {
      id: "win_rate_pct",
      label: I18n.t("copy-metric-win-rate"),
      type: "percent",
      sortable: true,
      render: (value, row) => esc(row.rounds ? pct(value, 0) : "—"),
    },
    {
      id: "realized_pnl_native",
      label: I18n.t("copy-table-realized"),
      type: "sol",
      sortable: true,
      render: (value) => `<span class="${toneClass(value)}">${esc(signedSolCell(value))}</span>`,
    },
    {
      id: "profit_factor",
      label: I18n.t("copy-table-profit-factor"),
      type: "number",
      sortable: true,
      render: (value) => esc(fixed(value, 2)),
    },
    {
      id: "average_hold_seconds",
      label: I18n.t("copy-table-average-hold"),
      type: "number",
      sortable: true,
      render: (value) => esc(duration(value)),
    },
    {
      id: "arrival_median_ms",
      label: I18n.t("copy-metric-median-arrival"),
      type: "number",
      sortable: true,
      render: (value) => esc(seconds(value)),
    },
    {
      id: "slippage_median_pct",
      label: I18n.t("copy-table-median-slippage"),
      type: "percent",
      sortable: true,
      render: (value) => esc(signedPct(value, 2)),
    },
    {
      id: "fills",
      label: I18n.t("copy-kind-fills"),
      type: "number",
      sortable: true,
      render: (value) => esc(String(value)),
    },
    {
      id: "skips",
      label: I18n.t("copy-kind-skips"),
      type: "number",
      sortable: true,
      render: (value) => esc(String(value)),
    },
  ];

  function dropTable() {
    table?.destroy();
    table = null;
  }

  /** The shared DataTable in its own slot, created once and refreshed in place. */
  function showTable(slot, rows) {
    if (table && table.elements.container !== slot) dropTable();
    if (!table) {
      table = new DataTable({
        container: slot,
        columns: columns(),
        rowIdField: "task_id",
        stateKey: "copy.compare-table",
        compact: true,
        stickyHeader: true,
        zebra: true,
        fitToContainer: true,
        sorting: { mode: "client", column: "realized_pnl_native", direction: "desc" },
        emptyTitle: I18n.t("copy-compare-empty"),
        emptyMessage: I18n.t("copy-compare-empty-message"),
        onRowClick: (row) => page.select(Number(row.task_id)),
      });
    }
    table.setData(rows);
  }

  function render() {
    const root = $("#copy-compare");
    if (!root) return;
    if (state.view !== "compare") {
      root.hidden = true;
      return;
    }
    root.hidden = false;
    const workspace = $("#copy-workspace");
    if (workspace) workspace.hidden = true;
    const rows = data.get(state.range);
    const error = errors.get(state.range);
    const head = `<div class="copy-panel-head"><h2>${esc(I18n.t("copy-compare-title"))}</h2><div class="copy-panel-tools">${segmented("compare-range", RANGES, state.range, esc, I18n.attr("copy-range-label", "aria-label"))}<button class="btn btn-ghost btn-sm" type="button" data-compare-close>${esc(I18n.t("copy-compare-back"))}</button></div></div>`;
    let body;
    if (!rows)
      body = error
        ? panelMessage(I18n.t("copy-compare-load-failed", { error }), esc, "is-error")
        : panelMessage(I18n.t("copy-compare-loading"), esc);
    else if (!rows.length) body = panelMessage(I18n.t("copy-compare-empty"), esc);
    else {
      body = `<section class="copy-card"><h4>${esc(I18n.t("copy-compare-curve-title"))}</h4>${comparisonCurves(
        rows.map((row) => ({
          // The table's name for the task; its wallet is the entry's tooltip.
          nameHtml: `<span title="${esc(row.target_address)}">${esc(taskName(row))}</span>`,
          points: row.pnl_curve,
        })),
        { escapeHtml: esc }
      )}</section>`;
    }
    if (!root.querySelector(`#${TABLE_ID}`)) {
      dropTable();
      forget(root);
      root.innerHTML = `<div data-compare-view></div><div class="copy-compare-table" id="${TABLE_ID}"></div>`;
    }
    paint(root.querySelector("[data-compare-view]"), head + body);
    const slot = root.querySelector(`#${TABLE_ID}`);
    slot.hidden = !rows?.length;
    if (rows?.length) showTable(slot, rows);
  }

  return { setup, refresh, render, dispose: dropTable };
}
