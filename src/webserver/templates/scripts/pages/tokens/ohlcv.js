/**
 * OHLCV table module for tokens page
 * Lines extracted from tokens.js (1578-1968)
 * Factory function pattern for shared state access
 */

import { timeAgoCell } from "./formatters.js";

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
        label: "Token",
        sortable: true,
        minWidth: 180,
        maxWidth: 200,
        wrap: false,
        render: (value, row) => {
          const short = value ? `${value.slice(0, 6)}...${value.slice(-4)}` : "—";
          return `<span class="ohlcv-token-cell">
            <span class="mint-cell" title="${Utils.escapeHtml(value)}">${short}</span>
            <span class="ohlcv-token-actions">
              <button class="btn btn-sm btn-danger ohlcv-delete-btn" data-mint="${Utils.escapeHtml(row.mint)}" title="Delete OHLCV data" aria-label="Delete OHLCV data">
                <i class="icon-trash-2"></i>
              </button>
            </span>
          </span>`;
        },
      },
      {
        id: "status",
        label: "Status",
        sortable: true,
        minWidth: 90,
        wrap: false,
        render: (value) => {
          const isActive = value === "active";
          const cls = isActive ? "status-active" : "status-inactive";
          const icon = isActive ? "icon-activity" : "icon-pause";
          return `<span class="status-badge ${cls}"><i class="${icon}"></i> ${value}</span>`;
        },
      },
      {
        id: "priority",
        label: "Priority",
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
          return `<span class="priority-badge ${priorityClass}">${value || "—"}</span>`;
        },
      },
      {
        id: "candle_count",
        label: "Candles",
        sortable: true,
        minWidth: 90,
        wrap: false,
        align: "right",
        render: (value) => Utils.formatNumber(value, { fallback: "0" }),
      },
      {
        id: "backfill_progress",
        label: "Backfill",
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
              return `<span class="tf-indicator ${cls}" title="${tf.label}: ${tf.done ? "Complete" : "Pending"}">${tf.label.charAt(0)}</span>`;
            })
            .join("");

          return `<div class="backfill-cell">
            <div class="backfill-bar ${progressCls}" style="--progress: ${pct}%"></div>
            <span class="backfill-text">${completed}/${total}</span>
            <div class="tf-indicators">${tfIcons}</div>
          </div>`;
        },
      },
      {
        id: "data_span_hours",
        label: "Data Span",
        sortable: true,
        minWidth: 90,
        wrap: false,
        align: "right",
        render: (value) => {
          if (!value || value <= 0) return "—";
          if (value < 24) return `${value.toFixed(1)}h`;
          const days = value / 24;
          return `${days.toFixed(1)}d`;
        },
      },
      {
        id: "open_gaps",
        label: "Gaps",
        sortable: true,
        minWidth: 70,
        wrap: false,
        align: "right",
        render: (value) => {
          if (!value || value === 0) return '<span class="value-positive">0</span>';
          return `<span class="value-warning">${value}</span>`;
        },
      },
      {
        id: "pool_count",
        label: "Pools",
        sortable: true,
        minWidth: 70,
        wrap: false,
        align: "right",
        render: (value) => Utils.formatNumber(value, { fallback: "0" }),
      },
      {
        id: "last_fetch",
        label: "Last Fetch",
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
    ohlcvState.isLoading = true;
    try {
      const response = await requestManager.fetch("/api/ohlcv/tokens", { priority: "normal" });
      if (response && response.tokens) {
        ohlcvState.tokens = response.tokens;
        ohlcvState.stats = response.stats;
      }
    } catch (err) {
      console.error("Failed to fetch OHLCV data:", err);
      Utils.showToast({ key: "ohlcv-load", type: "error", title: "Could not load OHLCV data" });
    } finally {
      ohlcvState.isLoading = false;
    }
  };

  const updateOhlcvTable = () => {
    if (!deps.ohlcvTable) return;
    deps.ohlcvTable.setData(ohlcvState.tokens, { preserveScroll: true });
    updateOhlcvToolbar();
  };

  const updateOhlcvToolbar = () => {
    if (!deps.ohlcvTable) return;
    const stats = ohlcvState.stats || {};

    deps.ohlcvTable.updateToolbarSummary([
      {
        id: "ohlcv-total",
        label: "Total Tokens",
        value: Utils.formatNumber(stats.total_tokens ?? 0, 0),
      },
      {
        id: "ohlcv-active",
        label: "Active",
        value: Utils.formatNumber(stats.active_tokens ?? 0, 0),
        variant: "success",
      },
      {
        id: "ohlcv-candles",
        label: "Candles",
        value: Utils.formatCompactNumber(stats.total_candles ?? 0),
        variant: "info",
      },
      {
        id: "ohlcv-size",
        label: "DB Size",
        value: `${(stats.database_size_mb ?? 0).toFixed(1)} MB`,
        variant: "secondary",
      },
    ]);
  };

  const handleOhlcvDelete = async (mint) => {
    const result = await ConfirmationDialog.show({
      title: "Delete OHLCV Data",
      message: `Delete all OHLCV data for ${mint.slice(0, 8)}...?`,
      confirmLabel: "Delete",
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
          `Deleted: ${response.candles_deleted} candles, ${response.pools_deleted} pools`,
          "success"
        );
        await fetchOhlcvData();
        updateOhlcvTable();
      }
    } catch (err) {
      console.error("Failed to delete OHLCV data:", err);
      Utils.showToast("Failed to delete OHLCV data", "error");
    }
  };

  const handleOhlcvCleanup = async () => {
    const result = await InputDialog.show({
      title: "Delete Inactive Tokens",
      message: "Delete inactive tokens older than specified hours",
      placeholder: "Hours...",
      defaultValue: "24",
      confirmLabel: "Delete",
      type: "number",
      validate: (value) => {
        const num = parseInt(value, 10);
        if (isNaN(num) || num < 1) return "Please enter a positive number";
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
        Utils.showToast(`Cleaned up ${response.deleted_count} inactive tokens`, "success");
        await fetchOhlcvData();
        updateOhlcvTable();
      }
    } catch (err) {
      console.error("Failed to cleanup OHLCV data:", err);
      Utils.showToast("Failed to cleanup OHLCV data", "error");
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
      ohlcvContainer.style.display = "none";
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
          { id: "ohlcv-total", label: "Total Tokens", value: "0" },
          { id: "ohlcv-active", label: "Active", value: "0", variant: "success" },
          { id: "ohlcv-candles", label: "Candles", value: "0", variant: "info" },
          { id: "ohlcv-size", label: "DB Size", value: "0 MB", variant: "secondary" },
        ],
        actions: [
          {
            id: "cleanup",
            label: "Cleanup Inactive",
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

  const showOhlcvView = () => {
    const tokensRoot = document.querySelector("#tokens-root");
    const ohlcvContainer = document.querySelector("#ohlcv-table-container");

    if (tokensRoot) tokensRoot.style.display = "none";
    if (ohlcvContainer) ohlcvContainer.style.display = "";

    // Pause main table poller, start OHLCV poller
    if (deps.poller) deps.poller.pause();
    if (deps.lastUpdatePoller) deps.lastUpdatePoller.pause();

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
    deps.ohlcvPoller.start();

    // Initial load
    fetchOhlcvData().then(() => updateOhlcvTable());
  };

  const hideOhlcvView = () => {
    const tokensRoot = document.querySelector("#tokens-root");
    const ohlcvContainer = document.querySelector("#ohlcv-table-container");

    if (tokensRoot) tokensRoot.style.display = "";
    if (ohlcvContainer) ohlcvContainer.style.display = "none";

    // Pause OHLCV poller, resume main poller
    if (deps.ohlcvPoller) deps.ohlcvPoller.pause();
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
