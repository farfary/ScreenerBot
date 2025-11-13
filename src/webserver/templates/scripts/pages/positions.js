import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import * as Utils from "../core/utils.js";
import { DataTable } from "../ui/data_table.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import { TradeActionDialog } from "../ui/trade_action_dialog.js";

const SUB_TABS = [
  { id: "open", label: '<i class="icon-trending-up"></i> Open' },
  { id: "closed", label: '<i class="icon-trending-down"></i> Closed' },
];

function createLifecycle() {
  let table = null;
  let poller = null;
  let tabBar = null;
  let tradeDialog = null;
  let walletBalance = 0;

  const state = {
    view: "open", // 'open' | 'closed'
    total: 0,
    lastUpdate: null,
  };

  const tokenCell = (row) => {
    const logo = row.logo_url || row.image_url || "";
    const symbol = row.symbol || "?";
    const name = row.name || "";
    const logoHtml = logo
      ? `<img class="token-logo" src="${Utils.escapeHtml(logo)}" alt="${Utils.escapeHtml(
          symbol
        )}"/>`
      : '<i class="token-logo icon-coins"></i>';
    return `<div class="position-token">${logoHtml}<div>
      <div class="token-symbol">${Utils.escapeHtml(symbol)}</div>
      <div class="token-name">${Utils.escapeHtml(name)}</div>
    </div></div>`;
  };

  const priceCell = (value) => Utils.formatPriceSol(value, { fallback: "—", decimals: 12 });
  const solCell = (v) => Utils.formatSol(v, { decimals: 4 });
  const pnlCell = (v) => Utils.formatPnL(v, { decimals: 4 });
  const percentCell = (v) => Utils.formatPercent(v, { style: "pnl", decimals: 2, fallback: "—" });
  const timeCell = (v) => Utils.formatTimeFromSeconds(v, { includeSeconds: false });

  const dcaCell = (count) => {
    if (!count || count === 0) return "—";
    return `<span class="chip">${count} DCA${count > 1 ? "s" : ""}</span>`;
  };

  const partialExitsCell = (count) => {
    if (!count || count === 0) return "—";
    return `<span class="chip warning">${count} exit${count > 1 ? "s" : ""}</span>`;
  };

  const currentSizeCell = (remaining, original) => {
    if (!remaining || !original || original === 0) return "—";
    const pct = Math.round((remaining / original) * 100);
    const cls = pct === 100 ? "success" : pct >= 50 ? "warning" : "danger";
    return `<span class="chip ${cls}">${pct}%</span>`;
  };

  /**
   * Build columns array based on current view (open/closed)
   * Different views show different columns (open has unrealized PnL, closed has exit data)
   */
  const buildColumns = () => {
    if (state.view === "open") {
      return [
        {
          id: "token",
          label: "Token",
          sortable: true,
          minWidth: 180,
          wrap: false,
          render: (_v, r) => tokenCell(r),
        },
        {
          id: "actions",
          label: "Actions",
          sortable: false,
          minWidth: 180,
          wrap: false,
          render: (_v, row) => {
            const mint = row?.mint || "";
            const isOpen = !row?.transaction_exit_verified;

            if (!mint || !isOpen) return "—";

            return `
              <div class="row-actions">
                <button class="btn row-action" data-action="add" data-mint="${Utils.escapeHtml(
                  mint
                )}" title="Add to position (DCA)">Add</button>
                <button class="btn warning row-action" data-action="sell" data-mint="${Utils.escapeHtml(
                  mint
                )}" title="Sell (full or % partial)">Sell</button>
              </div>
            `;
          },
        },
        {
          id: "entry_time",
          label: "Entry Time",
          sortable: true,
          minWidth: 140,
          render: (v) => timeCell(v),
        },
        {
          id: "dca_count",
          label: "DCA",
          sortable: true,
          minWidth: 80,
          render: (v) => dcaCell(v),
        },
        {
          id: "average_entry_price",
          label: "Avg Entry (SOL)",
          sortable: true,
          minWidth: 140,
          render: (v) => priceCell(v),
        },
        {
          id: "current_price",
          label: "Current (SOL)",
          sortable: true,
          minWidth: 140,
          render: (v) => (v == null ? "—" : priceCell(v)),
        },
        {
          id: "total_size_sol",
          label: "Total Invested",
          sortable: true,
          minWidth: 120,
          render: (v) => solCell(v),
        },
        {
          id: "current_size",
          label: "Size",
          sortable: true,
          minWidth: 80,
          render: (_v, r) => currentSizeCell(r.remaining_token_amount, r.token_amount),
        },
        {
          id: "partial_exit_count",
          label: "Exits",
          sortable: true,
          minWidth: 90,
          render: (v) => partialExitsCell(v),
        },
        {
          id: "unrealized_pnl",
          label: "Unrealized PnL",
          sortable: true,
          minWidth: 130,
          render: (v) => pnlCell(v),
        },
        {
          id: "unrealized_pnl_percent",
          label: "Unrealized %",
          sortable: true,
          minWidth: 110,
          render: (v) => percentCell(v),
        },
      ];
    } else {
      // closed view
      return [
        {
          id: "token",
          label: "Token",
          sortable: true,
          minWidth: 180,
          wrap: false,
          render: (_v, r) => tokenCell(r),
        },
        {
          id: "exit_time",
          label: "Exit Time",
          sortable: true,
          minWidth: 140,
          render: (v) => (v == null ? "—" : timeCell(v)),
        },
        {
          id: "dca_count",
          label: "DCA",
          sortable: true,
          minWidth: 80,
          render: (v) => dcaCell(v),
        },
        {
          id: "average_entry_price",
          label: "Avg Entry (SOL)",
          sortable: true,
          minWidth: 140,
          render: (v, r) => priceCell(v || r.entry_price),
        },
        {
          id: "average_exit_price",
          label: "Avg Exit (SOL)",
          sortable: true,
          minWidth: 140,
          render: (v, r) => (v == null ? priceCell(r.exit_price) : priceCell(v)),
        },
        {
          id: "total_size_sol",
          label: "Total Invested",
          sortable: true,
          minWidth: 120,
          render: (v) => solCell(v),
        },
        {
          id: "partial_exit_count",
          label: "Exits",
          sortable: true,
          minWidth: 90,
          render: (v) => partialExitsCell(v),
        },
        {
          id: "sol_received",
          label: "Proceeds",
          sortable: true,
          minWidth: 110,
          render: (v) => (v == null ? "—" : solCell(v)),
        },
        { id: "pnl", label: "PnL", sortable: true, minWidth: 110, render: (v) => pnlCell(v) },
        {
          id: "pnl_percent",
          label: "PnL %",
          sortable: true,
          minWidth: 100,
          render: (v) => percentCell(v),
        },
      ];
    }
  };

  const updateToolbar = () => {
    if (!table) return;

    table.updateToolbarSummary([
      {
        id: "positions-total",
        label: state.view === "open" ? "Open" : "Closed",
        value: Utils.formatNumber(state.total, 0),
      },
    ]);

    table.updateToolbarMeta([
      {
        id: "positions-last-update",
        text: state.lastUpdate
          ? `Last update ${new Date(state.lastUpdate).toLocaleTimeString()}`
          : "",
      },
    ]);
  };

  const loadPositionsPage = async ({ reason, signal }) => {
    const status = state.view;
    const url = `/api/positions?status=${encodeURIComponent(status)}&limit=500`;
    try {
      const res = await fetch(url, { cache: "no-store", signal });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const rows = await res.json();
      state.total = Array.isArray(rows) ? rows.length : 0;
      state.lastUpdate = Date.now();

      const mapped = rows.map((row) => ({
        ...row,
        token: `${row.symbol} (${row.mint.slice(0, 4)}…${row.mint.slice(-4)})`,
      }));

      return {
        rows: mapped,
        cursorNext: null,
        cursorPrev: null,
        hasMoreNext: false,
        hasMorePrev: false,
        total: mapped.length,
        preserveScroll: reason === "poll",
      };
    } catch (err) {
      if (err?.name !== "AbortError") {
        console.error("[Positions] fetch failed:", err);
        if (reason !== "poll") {
          Utils.showToast("Failed to refresh positions", "warning");
        }
      }
      throw err;
    }
  };

  const switchView = (view) => {
    if (view !== "open" && view !== "closed") return;
    state.view = view;
    if (table) {
      // Update columns for the new view using setColumns (systematic approach)
      const newColumns = buildColumns();
      table.setColumns(newColumns, {
        preserveData: false, // Clear data to force reload
        preserveScroll: false, // Reset scroll when switching views
        resetState: false, // Keep column widths/visibility preferences within each view
      });

      // Update stateKey for per-view persistence
      table.setStateKey(`positions-table.${view}`, { render: false });

      // Update toolbar title
      const viewLabel = view === "open" ? "Open" : "Closed";
      if (table.toolbarView) {
        const titleConfig = table.options.toolbar?.title;
        if (titleConfig) {
          titleConfig.text = `Positions: ${viewLabel}`;
        }
      }

      // Update sorting for the new view
      const sortColumn = view === "open" ? "entry_time" : "exit_time";
      table.setSort(sortColumn, "desc");
    }
    updateToolbar();
  };

  return {
    init(ctx) {
      // Initialize trade dialog
      tradeDialog = new TradeActionDialog();

      // Sub-tabs
      tabBar = new TabBar({
        container: "#subTabsContainer",
        tabs: SUB_TABS,
        defaultTab: state.view,
        stateKey: "positions.activeTab",
        pageName: "positions",
        onChange: (tabId) => switchView(tabId),
      });
      TabBarManager.register("positions", tabBar);
      ctx.manageTabBar(tabBar);
      tabBar.show();

      // Sync state.view with TabBar's restored state (from localStorage or URL)
      state.view = tabBar.getActiveTab() || state.view;

      // Build columns based on current view
      const columns = buildColumns();
      const viewLabel = state.view === "open" ? "Open" : "Closed";

      table = new DataTable({
        container: "#positions-root",
        columns,
        rowIdField: "mint",
        stateKey: `positions-table.${state.view}`,
        enableLogging: false,
        sorting: {
          mode: "client",
          column: state.view === "open" ? "entry_time" : "exit_time",
          direction: "desc",
        },
        compact: true,
        stickyHeader: true,
        zebra: true,
        fitToContainer: true,
        uniformRowHeight: 2,
        pagination: {
          threshold: 160,
          maxRows: 5000,
          loadPage: loadPositionsPage,
          dedupeKey: (row) => row?.mint ?? null,
          rowIdField: "mint",
          onPageLoaded: () => updateToolbar(),
        },
        toolbar: {
          title: {
            icon: "icon-bar-chart-2",
            text: `Positions: ${viewLabel}`,
            meta: [{ id: "positions-last-update", lines: ["Last Update", "—"] }],
          },
          summary: [{ id: "positions-total", label: "Total", value: "0", variant: "secondary" }],
          search: {
            enabled: true,
            mode: "client",
            placeholder: "Search by symbol or mint...",
          },
        },
      });

      updateToolbar();

      // Row actions: delegate clicks on the table container
      const containerEl = document.querySelector("#positions-root");
      const handleRowActionClick = async (e) => {
        const btn = e.target?.closest?.(".row-action");
        if (!btn) return;
        const action = btn.getAttribute("data-action");
        const mint = btn.getAttribute("data-mint");
        if (!action || !mint) return;

        // Find row data
        const row = table.getData().find((r) => r.mint === mint);
        if (!row) {
          Utils.showToast("Position data not found", "error");
          return;
        }

        try {
          if (action === "add") {
            // Fetch config for entry sizes
            let entrySizes = [0.005, 0.01, 0.02, 0.05];
            try {
              const configRes = await fetch("/api/config/trader");
              if (configRes.ok) {
                const configData = await configRes.json();
                if (Array.isArray(configData?.data?.entry_sizes)) {
                  entrySizes = configData.data.entry_sizes;
                }
              }
            } catch (err) {
              console.warn("Failed to fetch entry_sizes config:", err);
            }

            // Open dialog
            const result = await tradeDialog.open({
              action: "add",
              mint: mint,
              symbol: row.symbol || "?",
              context: {
                balance: walletBalance,
                entrySize: row.entry_sol || row.sol_size,
                entrySizes: entrySizes,
                currentSize: row.sol_size,
              },
            });

            if (!result) return; // User cancelled

            // Proceed with API call
            btn.disabled = true;
            const res = await fetch("/api/trader/manual/add", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                mint,
                ...(result.amount ? { size_sol: result.amount } : {}),
              }),
            });
            const data = await res.json();
            btn.disabled = false;
            if (!res.ok) throw new Error(data?.error?.message || "Add failed");
            Utils.showToast("Added to position", "success");
            table.refresh({ reason: "manual", preserveScroll: true });
          } else if (action === "sell") {
            // Open dialog
            const result = await tradeDialog.open({
              action: "sell",
              mint: mint,
              symbol: row.symbol || "?",
              context: {
                holdings: row.token_amount,
              },
            });

            if (!result) return; // User cancelled

            // Build request body
            const body =
              result.percentage === 100
                ? { mint, close_all: true }
                : { mint, percentage: result.percentage };

            btn.disabled = true;
            const res = await fetch("/api/trader/manual/sell", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify(body),
            });
            const data = await res.json();
            btn.disabled = false;
            if (!res.ok) throw new Error(data?.error?.message || "Sell failed");
            Utils.showToast("Sell placed", "success");
            table.refresh({ reason: "manual", preserveScroll: true });
          }
        } catch (err) {
          btn.disabled = false;
          Utils.showToast(err?.message || "Action failed", "error");
        }
      };

      if (containerEl) {
        containerEl.addEventListener("click", handleRowActionClick);
        ctx.onDispose(() => containerEl.removeEventListener("click", handleRowActionClick));
      }
    },

    activate(ctx) {
      // Fetch wallet balance for dialog context
      fetch("/api/wallet/balance")
        .then((res) => (res.ok ? res.json() : null))
        .then((data) => {
          if (data?.sol_balance != null) {
            walletBalance = data.sol_balance;
          }
        })
        .catch(() => {
          console.warn("[Positions] Failed to fetch wallet balance");
        });

      if (!poller) {
        poller = ctx.managePoller(
          new Poller(() => table?.refresh({ reason: "poll", preserveScroll: true }), {
            label: "Positions",
          })
        );
      }
      poller.start();
      if ((table?.getData?.() ?? []).length === 0) {
        table.refresh({ reason: "initial" });
      }
    },

    deactivate() {
      table?.cancelPendingLoad?.();
    },

    dispose() {
      if (tradeDialog) {
        tradeDialog.destroy();
        tradeDialog = null;
      }
      if (table) {
        table.destroy();
        table = null;
      }
      poller = null;
      tabBar = null;
      TabBarManager.unregister("positions");
      state.view = "open";
      state.total = 0;
      state.lastUpdate = null;
      walletBalance = 0;
    },
  };
}

registerPage("positions", createLifecycle());
