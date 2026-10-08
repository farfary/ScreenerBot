// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * OHLCV table module for tokens page
 * Lines extracted from tokens.js (1578-1968)
 * Factory function pattern for shared state access
 */

import { formatTimeSpan } from "../../core/format.js";
import { timeAgoCell } from "./formatters.js";
import {
  getIdentity,
  renderTokenCell,
  resolveTokenCells,
  TOKEN_CELL_MIN_WIDTH,
} from "../../ui/token_identity.js";

// Ids are the `status` values of /api/ohlcv/tokens.
const OHLCV_STATUS_LABELS = Object.freeze({
  active: "tokens-ohlcv-status-active",
  inactive: "tokens-ohlcv-status-inactive",
});

// Ids are the monitoring priorities of the OHLCV service (`Priority::as_str`).
const OHLCV_PRIORITY_LABELS = Object.freeze({
  critical: "tokens-ohlcv-priority-critical",
  high: "tokens-ohlcv-priority-high",
  medium: "tokens-ohlcv-priority-medium",
  low: "tokens-ohlcv-priority-low",
});

/**
 * Create OHLCV module with access to page state and dependencies
 * @param {Object} deps - Dependencies and state
 * @returns {Object} OHLCV module functions
 */
export function createOhlcvModule(deps) {
  const { ohlcvState, requestManager, DataTable, Utils, Poller, ConfirmationDialog, InputDialog } =
    deps;

  const buildOhlcvColumns = () => {
    return [
      {
        id: "mint",
        label: I18n.t("tokens-column-token"),
        sortable: true,
        minWidth: TOKEN_CELL_MIN_WIDTH + 34,
        wrap: false,
        render: (value, row) => {
          return `<span class="ohlcv-token-cell">
            ${renderTokenCell(value)}
            <span class="ohlcv-token-actions">
              <button class="btn btn-sm btn-danger ohlcv-delete-btn" data-mint="${Utils.escapeHtml(row.mint)}" title="${Utils.escapeHtml(I18n.attr("tokens-ohlcv-delete", "title"))}" aria-label="${Utils.escapeHtml(I18n.attr("tokens-ohlcv-delete", "aria-label"))}">
                <i class="icon-trash-2"></i>
              </button>
            </span>
          </span>`;
        },
      },
      {
        id: "status",
        label: I18n.t("tokens-column-status"),
        sortable: true,
        minWidth: 90,
        wrap: false,
        render: (value) => {
          const isActive = value === "active";
          const cls = isActive ? "status-active" : "status-inactive";
          const icon = isActive ? "icon-activity" : "icon-pause";
          return `<span class="status-badge ${cls}"><i class="${icon}"></i> ${Utils.escapeHtml(I18n.label(OHLCV_STATUS_LABELS, value))}</span>`;
        },
      },
      {
        id: "priority",
        label: I18n.t("tokens-ohlcv-column-priority"),
        sortable: true,
        minWidth: 80,
        wrap: false,
        render: (value) => {
          const priorityClass =
            {
              critical: "priority-critical",
              high: "priority-high",
              medium: "priority-medium",
              low: "priority-low",
            }[value?.toLowerCase()] || "priority-medium";
          return `<span class="priority-badge ${priorityClass}">${value ? Utils.escapeHtml(I18n.label(OHLCV_PRIORITY_LABELS, value)) : "—"}</span>`;
        },
      },
      {
        id: "candle_count",
        label: I18n.t("chart-candles"),
        type: "number",
        sortable: true,
        minWidth: 90,
        wrap: false,
        render: (value) => Utils.formatNumber(value, { fallback: "0" }),
      },
      {
        id: "backfill_progress",
        label: I18n.t("tokens-ohlcv-column-backfill"),
        // column-type-ok: a progress bar with timeframe marks, laid out as a text column
        sortable: false,
        minWidth: 120,
        wrap: false,
        render: (value) => {
          if (!value) return "—";
          const { completed, total, percent, timeframes } = value;
          const pct = Math.round(percent);
          const progressCls =
            pct === 100 ? "progress-complete" : pct > 50 ? "progress-partial" : "progress-low";

          // Build timeframe icons
          const tfIcons = [
            { key: "m1", label: "1m", done: timeframes?.["1m"] ?? timeframes?.m1 },
            { key: "m5", label: "5m", done: timeframes?.["5m"] ?? timeframes?.m5 },
            { key: "m15", label: "15m", done: timeframes?.["15m"] ?? timeframes?.m15 },
            { key: "h1", label: "1h", done: timeframes?.["1h"] ?? timeframes?.h1 },
            { key: "h4", label: "4h", done: timeframes?.["4h"] ?? timeframes?.h4 },
            { key: "h12", label: "12h", done: timeframes?.["12h"] ?? timeframes?.h12 },
            { key: "d1", label: "1d", done: timeframes?.["1d"] ?? timeframes?.d1 },
          ]
            .map((tf) => {
              const cls = tf.done ? "tf-done" : "tf-pending";
              return `<span class="tf-indicator ${cls}" title="${Utils.escapeHtml(tf.done ? I18n.t("tokens-ohlcv-timeframe-complete", { timeframe: tf.label }) : I18n.t("tokens-ohlcv-timeframe-pending", { timeframe: tf.label }))}">${tf.label.charAt(0)}</span>`;
            })
            .join("");

          return `<div class="backfill-cell">
            <div class="backfill-bar ${progressCls}" style="--progress: ${pct}%"></div>
            <span class="backfill-text">${Utils.formatNumber(completed, 0)}/${Utils.formatNumber(total, 0)}</span>
            <div class="tf-indicators">${tfIcons}</div>
          </div>`;
        },
      },
      {
        id: "data_span_hours",
        label: I18n.t("tokens-ohlcv-column-data-span"),
        type: "number",
        sortable: true,
        minWidth: 90,
        wrap: false,
        render: (value) => {
          if (!value || value <= 0) return "—";
          if (value < 24) return formatTimeSpan(value, { unit: "hour", decimals: 1 });
          return formatTimeSpan(value / 24, { unit: "day", decimals: 1 });
        },
      },
      {
        id: "open_gaps",
        label: I18n.t("tokens-ohlcv-column-gaps"),
        type: "number",
        sortable: true,
        minWidth: 70,
        wrap: false,
        render: (value) => {
          if (!value || value === 0) return '<span class="value-positive">0</span>';
          return `<span class="value-warning">${Utils.formatNumber(value, 0)}</span>`;
        },
      },
      {
        id: "pool_count",
        label: I18n.t("tokens-ohlcv-column-pools"),
        type: "number",
        sortable: true,
        minWidth: 70,
        wrap: false,
        render: (value) => Utils.formatNumber(value, { fallback: "0" }),
      },
      {
        id: "last_fetch",
        label: I18n.t("tokens-ohlcv-column-last-fetch"),
        sortable: true,
        minWidth: 100,
        wrap: false,
        render: (value) => {
          if (!value) return "—";
          const timestamp =
            typeof value === "string" ? Math.floor(new Date(value).getTime() / 1000) : value;
          return timeAgoCell(timestamp);
        },
      },
    ];
  };

  const fetchOhlcvData = async () => {
    const table = deps.ohlcvTable;
    ohlcvState.isLoading = true;
    if (!ohlcvState.hasLoadedOnce) {
      table?.showBlockingState?.({
        variant: "loading",
        title: I18n.t("tokens-table-loading-title"),
        description: I18n.t("tokens-table-loading-description"),
      });
    }
    try {
      const response = await requestManager.fetch("/api/ohlcv/tokens", { priority: "normal" });
      if (response && response.tokens) {
        ohlcvState.tokens = response.tokens;
        ohlcvState.stats = response.stats;
      }
      ohlcvState.hasLoadedOnce = true;
      table?.hideBlockingState?.();
    } catch (err) {
      console.error("Failed to fetch OHLCV data:", err);
      if (!ohlcvState.hasLoadedOnce) {
        table?.showBlockingState?.({
          variant: "error",
          title: I18n.t("tokens-ohlcv-load-failed-title"),
          description: I18n.t("tokens-table-retry-hint"),
        });
      } else {
        Utils.showToast({
          key: "ohlcv-load",
          type: "error",
          title: I18n.t("tokens-ohlcv-load-failed-toast"),
        });
      }
    } finally {
      ohlcvState.isLoading = false;
    }
  };

  const updateOhlcvTable = () => {
    if (!deps.ohlcvTable) return;
    deps.ohlcvTable.setData(ohlcvState.tokens, { preserveScroll: true });
    resolveTokenCells(
      ohlcvState.tokens.map((token) => token.mint),
      () => deps.ohlcvTable?.repaintRows()
    );
    updateOhlcvToolbar();
  };

  const updateOhlcvToolbar = () => {
    if (!deps.ohlcvTable) return;
    const stats = ohlcvState.stats || {};

    deps.ohlcvTable.updateToolbarSummary([
      {
        id: "ohlcv-total",
        label: I18n.t("tokens-ohlcv-total"),
        value: Utils.formatNumber(stats.total_tokens ?? 0, 0),
      },
      {
        id: "ohlcv-active",
        label: I18n.t("tokens-ohlcv-active"),
        value: Utils.formatNumber(stats.active_tokens ?? 0, 0),
        variant: "success",
      },
      {
        id: "ohlcv-candles",
        label: I18n.t("chart-candles"),
        value: Utils.formatCompactNumber(stats.total_candles ?? 0),
        variant: "info",
      },
      {
        id: "ohlcv-size",
        label: I18n.t("tokens-ohlcv-db-size"),
        value: Utils.formatBytes((stats.database_size_mb ?? 0) * 1_048_576),
        variant: "secondary",
      },
    ]);
  };

  const handleOhlcvDelete = async (mint) => {
    const result = await ConfirmationDialog.show({
      title: I18n.t("tokens-ohlcv-delete-title"),
      message: I18n.t("tokens-ohlcv-delete-token-message", {
        token: getIdentity(mint).symbol || I18n.t("format-unknown"),
      }),
      confirmLabel: I18n.t("common-action-delete"),
      cancelLabel: I18n.t("common-action-cancel"),
      variant: "danger",
    });
    if (!result.confirmed) return;

    try {
      const response = await requestManager.fetch(`/api/ohlcv/${mint}/delete`, {
        method: "DELETE",
        priority: "high",
      });

      if (response) {
        Utils.showToast(
          I18n.t("tokens-ohlcv-delete-done", {
            candles: response.candles_deleted,
            pools: response.pools_deleted,
          }),
          "success"
        );
        await fetchOhlcvData();
        updateOhlcvTable();
      }
    } catch (err) {
      console.error("Failed to delete OHLCV data:", err);
      Utils.showToast(I18n.t("tokens-ohlcv-delete-failed"), "error");
    }
  };

  const handleOhlcvCleanup = async () => {
    const result = await InputDialog.show({
      title: I18n.t("tokens-ohlcv-cleanup-title"),
      message: I18n.t("tokens-ohlcv-cleanup-message"),
      placeholder: I18n.t("tokens-ohlcv-cleanup-placeholder"),
      defaultValue: "24",
      confirmLabel: I18n.t("common-action-delete"),
      cancelLabel: I18n.t("common-action-cancel"),
      type: "number",
      validate: (value) => {
        const num = parseInt(value, 10);
        if (isNaN(num) || num < 1) return I18n.t("tokens-ohlcv-cleanup-invalid");
        return null;
      },
    });
    if (!result) return;

    const inactiveHours = parseInt(result.value, 10);

    try {
      const response = await requestManager.fetch("/api/ohlcv/cleanup", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ inactive_hours: inactiveHours }),
        priority: "high",
      });

      if (response) {
        Utils.showToast(
          I18n.t("tokens-ohlcv-cleanup-done", { count: response.deleted_count }),
          "success"
        );
        await fetchOhlcvData();
        updateOhlcvTable();
      }
    } catch (err) {
      console.error("Failed to cleanup OHLCV data:", err);
      Utils.showToast(I18n.t("tokens-ohlcv-cleanup-failed"), "error");
    }
  };

  const initOhlcvTable = () => {
    if (deps.ohlcvTable) return; // Already initialized

    // Create container for OHLCV table
    const rootEl = document.querySelector("#tokens-root");
    if (!rootEl) return;

    // Create OHLCV container
    let ohlcvContainer = document.querySelector("#ohlcv-table-container");
    if (!ohlcvContainer) {
      ohlcvContainer = document.createElement("div");
      ohlcvContainer.id = "ohlcv-table-container";
      ohlcvContainer.className = "ohlcv-table-container";
      // This table is created only when OHLCV becomes active. Keep it laid out
      // for DataTable's initial measurement instead of constructing it hidden.
      ohlcvContainer.style.display = "";
      rootEl.parentNode.insertBefore(ohlcvContainer, rootEl.nextSibling);
    }

    deps.ohlcvTable = new DataTable({
      container: "#ohlcv-table-container",
      columns: buildOhlcvColumns(),
      rowIdField: "mint",
      stateKey: "ohlcv-table",
      enableLogging: false,
      sorting: {
        mode: "client",
        column: "candle_count",
        direction: "desc",
      },
      clientPagination: {
        enabled: true,
        pageSizes: [10, 20, 50, 100, "all"],
        defaultPageSize: 50,
        stateKey: "tokens.ohlcv.pageSize",
      },
      compact: true,
      stickyHeader: true,
      zebra: true,
      fitToContainer: true,
      autoSizeColumns: false,
      uniformRowHeight: 2,
      toolbar: {
        summary: [
          { id: "ohlcv-total", label: I18n.t("tokens-ohlcv-total"), value: "0" },
          {
            id: "ohlcv-active",
            label: I18n.t("tokens-ohlcv-active"),
            value: "0",
            variant: "success",
          },
          {
            id: "ohlcv-candles",
            label: I18n.t("chart-candles"),
            value: "0",
            variant: "info",
          },
          {
            id: "ohlcv-size",
            label: I18n.t("tokens-ohlcv-db-size"),
            value: Utils.formatBytes(0),
            variant: "secondary",
          },
        ],
        actions: [
          {
            id: "cleanup",
            label: I18n.t("tokens-ohlcv-cleanup"),
            icon: "icon-trash-2",
            variant: "warning",
            onClick: handleOhlcvCleanup,
          },
        ],
      },
    });

    // Add click handler for delete buttons
    ohlcvContainer.addEventListener("click", (e) => {
      const deleteBtn = e.target.closest(".ohlcv-delete-btn");
      if (deleteBtn) {
        const mint = deleteBtn.getAttribute("data-mint");
        if (mint) handleOhlcvDelete(mint);
      }
    });
  };

  const showOhlcvView = ({ load = true } = {}) => {
    const tokensRoot = document.querySelector("#tokens-root");

    if (tokensRoot) tokensRoot.style.display = "none";
    if (!deps.ohlcvTable) initOhlcvTable();
    const ohlcvContainer = document.querySelector("#ohlcv-table-container");
    if (ohlcvContainer) ohlcvContainer.style.display = "";

    // Pause main table poller, start OHLCV poller
    if (deps.poller) deps.poller.stop({ silent: true });
    if (deps.lastUpdatePoller) deps.lastUpdatePoller.stop({ silent: true });

    if (ohlcvState.hasLoadedOnce) {
      updateOhlcvTable();
    } else {
      deps.ohlcvTable?.showBlockingState?.({
        variant: "loading",
        title: I18n.t("tokens-table-loading-title"),
        description: I18n.t("tokens-table-loading-description"),
      });
    }
    if (!load) return;

    if (!deps.ohlcvPoller) {
      deps.ohlcvPoller = new Poller(
        async () => {
          await fetchOhlcvData();
          updateOhlcvTable();
        },
        {
          label: "OHLCV",
          getInterval: () => 30000,
          pauseWhenHidden: true,
        }
      );
    }
    deps.managePoller?.(deps.ohlcvPoller);
    deps.ohlcvPoller.start();

    if (!ohlcvState.isLoading) {
      void fetchOhlcvData().then(() => updateOhlcvTable());
    }
  };

  const hideOhlcvView = () => {
    const tokensRoot = document.querySelector("#tokens-root");
    const ohlcvContainer = document.querySelector("#ohlcv-table-container");

    if (tokensRoot) tokensRoot.style.display = "";
    if (ohlcvContainer) ohlcvContainer.style.display = "none";

    // Pause OHLCV poller, resume main poller
    if (deps.ohlcvPoller) deps.ohlcvPoller.stop({ silent: true });
    if (deps.poller) deps.poller.start();
    if (deps.lastUpdatePoller) deps.lastUpdatePoller.start();
  };

  return {
    buildOhlcvColumns,
    fetchOhlcvData,
    updateOhlcvTable,
    updateOhlcvToolbar,
    handleOhlcvDelete,
    handleOhlcvCleanup,
    initOhlcvTable,
    showOhlcvView,
    hideOhlcvView,
  };
}
