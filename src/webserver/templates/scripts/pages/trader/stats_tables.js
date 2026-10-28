// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
//! The Trader Stats tab's tables: closed trades by exit reason and by entry source, and the most recent closes of the window.

import { $ } from "../../core/dom.js";
import { originKindText } from "../../ui/position_origin.js";
import { renderTokenRowCell, resolveTokenCells } from "../../ui/token_identity.js";

const DASH = "—";

/**
 * Builds the three tables of the Stats tab. Each table is created on its first paint with
 * rows, once its section is on screen, and rows are replaced only when they changed.
 * `emptyText(periodDays)` names an empty window, as the tab's own empty state does.
 */
export function createStatsTables({ DataTable, Utils, emptyText }) {
  const esc = Utils.escapeHtml;
  const tables = new Map();
  const painted = new Map();
  // The book the shown rows belong to; only a live row opens the position details.
  let shownBook = "live";
  // The window of the shown rows, which an emptied table names.
  let shownPeriod = null;
  const emptyState = {
    emptyTitle: () => emptyText(shownPeriod),
    emptyMessage: () => I18n.t("trader-stats-empty-hint"),
  };

  /** The figures every group table shares after its first column. */
  function groupColumns(first) {
    return [
      first,
      {
        id: "count",
        label: I18n.t("trader-column-trades"),
        type: "number",
        sortable: true,
        render: (value) => esc(String(value)),
      },
      {
        id: "win_rate_pct",
        label: I18n.t("trader-column-win-rate"),
        type: "percent",
        sortable: true,
        render: (value) =>
          esc(Utils.formatPercentValue(value, { decimals: 1, includeSign: false, fallback: DASH })),
      },
      {
        id: "avg_profit_pct",
        label: I18n.t("trader-column-avg-return"),
        type: "percent",
        sortable: true,
        render: (value) =>
          Utils.formatPercent(value, { style: "pnl", decimals: 1, fallback: DASH }),
      },
      {
        id: "net_pnl_native",
        label: I18n.t("trader-column-net-pnl"),
        type: "number",
        sortable: true,
        render: (value) => Utils.formatPnL(value, { trim: false, fallback: DASH }),
      },
    ];
  }

  function groupTable(container, { first, rowIdField, stateKey }) {
    return new DataTable({
      container,
      columns: groupColumns(first),
      rowIdField,
      stateKey,
      compact: true,
      zebra: true,
      fitToContainer: true,
      sorting: { mode: "client", column: "count", direction: "desc" },
      emptyTitle: emptyState.emptyTitle,
      emptyMessage: emptyState.emptyMessage,
    });
  }

  /** Open the details of a live round; the dialog reads the live book only. */
  async function openDetails(row) {
    if (shownBook !== "live" || row.position_id == null) return;
    // The dialog module installs the listener this event reaches; it loads on first use.
    await import("../../ui/position_details_dialog.js");
    window.dispatchEvent(
      new CustomEvent("screenerbot:open-position-details", {
        detail: { id: row.position_id, mint: row.mint, symbol: row.symbol },
      })
    );
  }

  const BUILDERS = Object.freeze({
    exits: (container) =>
      groupTable(container, {
        first: {
          id: "exit_type",
          label: I18n.t("trader-column-exit"),
          grow: true,
          sortable: true,
          render: (_value, row) => esc(I18n.text(row.label)),
        },
        rowIdField: "exit_type",
        stateKey: "trader.stats-exits",
      }),
    sources: (container) =>
      groupTable(container, {
        first: {
          id: "source",
          label: I18n.t("trader-column-source"),
          grow: true,
          sortable: true,
          render: (value) => esc(originKindText(value)),
        },
        rowIdField: "source",
        stateKey: "trader.stats-sources",
      }),
    recent: (container) => {
      const table = new DataTable({
        container,
        columns: [
          {
            id: "symbol",
            label: I18n.t("trader-column-token"),
            grow: true,
            maxWidth: 240,
            sortable: true,
            wrap: false,
            render: (_value, row) =>
              renderTokenRowCell(row.mint, {
                symbol: row.symbol,
                name: row.name,
              }),
          },
          {
            id: "pnl_native",
            label: I18n.t("trader-column-pnl"),
            type: "number",
            sortable: true,
            render: (value) => Utils.formatPnL(value, { trim: false, fallback: DASH }),
          },
          {
            id: "pnl_percent",
            label: I18n.t("trader-column-return"),
            type: "percent",
            sortable: true,
            render: (value) =>
              Utils.formatPercent(value, { style: "pnl", decimals: 1, fallback: DASH }),
          },
          {
            id: "hold_secs",
            label: I18n.t("trader-column-held"),
            type: "number",
            sortable: true,
            // A compound duration is several isolated runs; the page direction orders
            // them inside the left-to-right numeric cell.
            render: (value) =>
              `<span dir="${I18n.dir}">${esc(
                Utils.formatUptime(value, { style: "trimmed", fallback: DASH })
              )}</span>`,
          },
          {
            id: "exit_time",
            label: I18n.t("trader-column-closed"),
            type: "number",
            sortable: true,
            render: (value) => esc(Utils.formatTimeAgo(value, { fallback: DASH })),
          },
          {
            id: "source",
            label: I18n.t("trader-column-source"),
            sortable: true,
            render: (value) => esc(originKindText(value)),
          },
          {
            id: "reason",
            label: I18n.t("trader-column-exit"),
            sortable: false,
            wrap: false,
            render: (value) => esc(I18n.text(value)),
          },
        ],
        rowIdField: "position_id",
        stateKey: "trader.stats-recent",
        compact: true,
        zebra: true,
        fitToContainer: true,
        sorting: { mode: "client", column: "exit_time", direction: "desc" },
        emptyTitle: emptyState.emptyTitle,
        emptyMessage: emptyState.emptyMessage,
        onRowClick: (row) => void openDetails(row),
      });
      return table;
    },
  });

  const CONTAINERS = Object.freeze({
    exits: "#exit-breakdown",
    sources: "#source-breakdown",
    recent: "#recent-rounds",
  });

  function paintTable(name, rows) {
    const hash = JSON.stringify(rows);
    if (painted.get(name) === hash) return null;
    let table = tables.get(name);
    if (!table) {
      if (!rows.length) return null;
      const container = $(CONTAINERS[name]);
      if (!container) return null;
      table = BUILDERS[name](container);
      tables.set(name, table);
    }
    painted.set(name, hash);
    table.setData(rows);
    return table;
  }

  return {
    /** Show the rows of `data` (a `/api/trader/stats` response) from `book`. */
    paint(data, book) {
      shownBook = book;
      shownPeriod = data.period_days ?? null;
      paintTable("exits", data.exit_breakdown || []);
      paintTable("sources", data.source_breakdown || []);
      const recentRows = data.recent_rounds || [];
      const recent = paintTable("recent", recentRows);
      if (recent) {
        resolveTokenCells(
          recentRows.map((row) => row.mint),
          () => recent.repaintRows()
        );
      }
    },

    destroy() {
      tables.forEach((table) => table.destroy());
      tables.clear();
      painted.clear();
    },
  };
}
