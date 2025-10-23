/* global prompt */
import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import * as Utils from "../core/utils.js";
import { DataTable } from "../ui/data_table.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";

const SUB_TABS = [
  { id: "open", label: "📈 Open" },
  { id: "closed", label: "📉 Closed" },
];

function createLifecycle() {
  let table = null;
  let poller = null;
  let tabBar = null;

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
      : '<span class="token-logo">🪙</span>';
    return `<div class="position-token">${logoHtml}<div>
      <div class="token-symbol">${Utils.escapeHtml(symbol)}</div>
      <div class="token-name">${Utils.escapeHtml(name)}</div>
    </div></div>`;
  };

  const priceCell = (value) => Utils.formatPriceSol(value, { fallback: "—", decimals: 9 });
  const solCell = (v) => Utils.formatSol(v, { decimals: 4 });
  const pnlCell = (v) => Utils.formatPnL(v, { decimals: 4 });
  const percentCell = (v) => Utils.formatPercent(v, { style: "pnl", decimals: 2, fallback: "—" });
  const timeCell = (v) => Utils.formatTimeFromSeconds(v, { includeSeconds: false });

  const columnsOpen = [
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
      id: "entry_price",
      label: "Entry Price (SOL)",
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
      id: "entry_size_sol",
      label: "Entry Size",
      sortable: true,
      minWidth: 120,
      render: (v) => solCell(v),
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
    {
      id: "entry_time",
      label: "Entry Time",
      sortable: true,
      minWidth: 140,
      render: (v) => timeCell(v),
    },
  ];

  const columnsClosed = [
    {
      id: "token",
      label: "Token",
      sortable: true,
      minWidth: 180,
      wrap: false,
      render: (_v, r) => tokenCell(r),
    },
    {
      id: "effective_entry_price",
      label: "Eff. Entry (SOL)",
      sortable: true,
      minWidth: 140,
      render: (v, r) => priceCell(v ?? r.entry_price),
    },
    {
      id: "effective_exit_price",
      label: "Eff. Exit (SOL)",
      sortable: true,
      minWidth: 140,
      render: (v, r) => (v == null ? priceCell(r.exit_price) : priceCell(v)),
    },
    {
      id: "entry_size_sol",
      label: "Size",
      sortable: true,
      minWidth: 100,
      render: (v) => solCell(v),
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
    {
      id: "exit_time",
      label: "Exit Time",
      sortable: true,
      minWidth: 140,
      render: (v) => (v == null ? "—" : timeCell(v)),
    },
  ];

  function selectedColumns() {
    return state.view === "open" ? columnsOpen : columnsClosed;
  }

  const updateToolbar = () => {
    if (!table) return;

    table.updateToolbarSummary([
      {
        id: "positions-total",
        label: state.view === "open" ? "Open" : "Closed",
        value: Utils.formatNumber(state.total),
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
          Utils.showToast("⚠️ Failed to refresh positions", "warning");
        }
      }
      throw err;
    }
  };

  const switchView = (view) => {
    if (view !== "open" && view !== "closed") return;
    state.view = view;
    if (table) {
      table.setColumns(selectedColumns());
      table.setStateKey(`positions-table.${view}`, { render: false });
      table.refresh({ reason: "view-switch", preserveScroll: false, resetScroll: true });
    }
    updateToolbar();
  };

  return {
    init(ctx) {
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

      const columns = selectedColumns();

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
            icon: "📊",
            text: "Positions",
            meta: [{ id: "positions-last-update", lines: ["Last Update", "—"] }],
          },
          summary: [{ id: "positions-total", label: "Total", value: "0", variant: "secondary" }],
          search: {
            enabled: true,
            mode: "client",
            placeholder: "Search by symbol or mint...",
          },
          buttons: [
            {
              id: "refresh",
              label: "Refresh",
              variant: "primary",
              onClick: () => table.refresh({ reason: "manual" }),
            },
          ],
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

        try {
          if (action === "add") {
            let sizeStr = prompt("Add size (SOL) — leave empty for default (50%)");
            let size = sizeStr != null && String(sizeStr).trim() !== "" ? Number(sizeStr) : null;
            if (size != null && (!Number.isFinite(size) || size <= 0)) {
              return Utils.showToast("Invalid size", "error");
            }
            btn.disabled = true;
            const res = await fetch("/api/trader/manual/add", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ mint, ...(size ? { size_sol: size } : {}) }),
            });
            const data = await res.json();
            btn.disabled = false;
            if (!res.ok) throw new Error(data?.error?.message || "Add failed");
            Utils.showToast("✅ Added to position", "success");
            table.refresh({ reason: "manual", preserveScroll: true });
          } else if (action === "sell") {
            let pctStr = prompt("Sell percentage (1-100). Leave empty to sell 100%.");
            let body;
            if (pctStr == null || String(pctStr).trim() === "") {
              body = { mint, close_all: true };
            } else {
              const pct = Number(pctStr);
              if (!Number.isFinite(pct) || pct <= 0 || pct > 100) {
                return Utils.showToast("Invalid percentage", "error");
              }
              body = { mint, percentage: pct };
            }
            btn.disabled = true;
            const res = await fetch("/api/trader/manual/sell", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify(body),
            });
            const data = await res.json();
            btn.disabled = false;
            if (!res.ok) throw new Error(data?.error?.message || "Sell failed");
            Utils.showToast("✅ Sell placed", "success");
            table.refresh({ reason: "manual", preserveScroll: true });
          }
        } catch (err) {
          btn.disabled = false;
          Utils.showToast(err?.message || "Action failed", "error");
        }
      };

      if (containerEl) {
        containerEl.addEventListener("click", handleRowActionClick);
        ctx.addCleanup(() => containerEl.removeEventListener("click", handleRowActionClick));
      }
    },

    activate(ctx) {
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
    },
  };
}

registerPage("positions", createLifecycle());
