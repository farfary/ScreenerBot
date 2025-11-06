import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import * as Utils from "../core/utils.js";
import { DataTable } from "../ui/data_table.js";
import { TabBar } from "../ui/tab_bar.js";
import { TradeActionDialog } from "../ui/trade_action_dialog.js";
import { TokenDetailsDialog } from "../ui/token_details_dialog.js";

const SUB_TABS = [
  { id: "overview", label: "📊 Overview" },
  { id: "flows", label: "💸 Flows" },
  { id: "holdings", label: "🪙 Holdings" },
  { id: "history", label: "📜 History" },
];

const WINDOW_OPTIONS = [
  { id: "24", label: "24h", hours: 24 },
  { id: "168", label: "7d", hours: 168 },
  { id: "720", label: "30d", hours: 720 },
  { id: "0", label: "All Time", hours: 0 },
];

function createLifecycle() {
  let tabBar = null;
  let poller = null;
  let holdingsTable = null;
  let historyTable = null;
  let tradeDialog = null;
  let detailsDialog = null;
  let balanceChart = null;
  let flowsChart = null;

  const state = {
    view: "overview",
    window: 24,
    dashboardData: null,
    currentSnapshot: null,
    lastUpdate: null,
  };

  // ============================================================================
  // UTILITY FUNCTIONS
  // ============================================================================

  const formatSol = (v) => Utils.formatSol(v, { decimals: 4, fallback: "—" });
  const formatPercent = (v) => Utils.formatPercent(v, { style: "pnl", decimals: 2, fallback: "—" });
  const formatUsd = (v) => Utils.formatCurrencyUSD(v, { fallback: "—" });
  const formatTimeAgo = (seconds) => Utils.formatTimeAgo(seconds, { fallback: "—" });

  const escapeHtml = (str) => Utils.escapeHtml(str);

  const tokenCell = (row) => {
    const logo = row.logo_url || row.image_url || "";
    const symbol = row.symbol || "?";
    const name = row.name || "";
    const logoHtml = logo
      ? `<img class="token-logo" src="${escapeHtml(logo)}" alt="${escapeHtml(symbol)}"/>`
      : '<span class="token-logo">🪙</span>';
    return `<div class="position-token">${logoHtml}<div>
      <div class="token-symbol">${escapeHtml(symbol)}</div>
      <div class="token-name">${escapeHtml(name)}</div>
    </div></div>`;
  };

  const priceCell = (value) =>
    Utils.formatPriceSol(value, { fallback: "—", decimals: 12 });

  // ============================================================================
  // DATA FETCHING
  // ============================================================================

  async function fetchCurrentSnapshot() {
    try {
      const response = await fetch("/api/wallet/current");
      if (!response.ok) return null;
      const data = await response.json();
      state.currentSnapshot = data;
      return data;
    } catch (error) {
      console.error("[Wallet] Failed to fetch current snapshot:", error);
      return null;
    }
  }

  async function fetchDashboardData(windowHours = 24) {
    try {
      const response = await fetch("/api/wallet/dashboard", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          windowHours,
          snapshotLimit: 600,
          maxTokens: 250,
        }),
      });

      if (!response.ok) return null;
      const result = await response.json();
      if (result.error) {
        console.error("[Wallet] Dashboard error:", result.error);
        return null;
      }

      state.dashboardData = result.data;
      state.lastUpdate = Date.now();
      return result.data;
    } catch (error) {
      console.error("[Wallet] Failed to fetch dashboard data:", error);
      return null;
    }
  }

  async function refreshDashboardCache(windowHours = 24) {
    try {
      const response = await fetch("/api/wallet/dashboard/refresh", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ windowHours }),
      });

      if (!response.ok) return null;
      const result = await response.json();
      return result.data;
    } catch (error) {
      console.error("[Wallet] Failed to refresh cache:", error);
      return null;
    }
  }

  // ============================================================================
  // OVERVIEW SUBTAB
  // ============================================================================

  function renderOverview(container, data) {
    if (!data) {
      container.innerHTML = `<div class="empty-state">No wallet data available</div>`;
      return;
    }

    const { summary, balance_trend } = data;

    const changeClass =
      summary.sol_change > 0 ? "value-positive" : summary.sol_change < 0 ? "value-negative" : "";

    container.innerHTML = `
      <div class="wallet-overview">
        <div class="summary-cards">
          <div class="summary-card">
            <div class="summary-label">Current Balance</div>
            <div class="summary-value">${formatSol(summary.current_sol_balance)}</div>
          </div>
          <div class="summary-card">
            <div class="summary-label">Change (${state.window}h)</div>
            <div class="summary-value ${changeClass}">${formatSol(summary.sol_change)}</div>
            <div class="summary-hint">${formatPercent(summary.sol_change_percent)}</div>
          </div>
          <div class="summary-card">
            <div class="summary-label">Token Holdings</div>
            <div class="summary-value">${summary.token_count}</div>
          </div>
          <div class="summary-card">
            <div class="summary-label">Last Update</div>
            <div class="summary-value">${summary.last_snapshot_time ? new Date(summary.last_snapshot_time).toLocaleString() : "—"}</div>
          </div>
        </div>

        <div class="window-selector">
          ${WINDOW_OPTIONS.map(
            (opt) => `
            <button 
              class="btn window-btn ${state.window === opt.hours ? "active" : ""}" 
              data-window="${opt.hours}"
            >
              ${opt.label}
            </button>
          `
          ).join("")}
        </div>

        <div class="chart-container">
          <div class="chart-header">
            <h3>Balance Trend</h3>
            <button class="btn small" id="exportBalanceBtn">Export CSV</button>
          </div>
          <div id="balanceChart" style="height: 300px;"></div>
        </div>

        <div class="actions-bar">
          <button class="btn" id="refreshCacheBtn">🔄 Refresh Cache</button>
        </div>
      </div>
    `;

    // Attach event listeners
    container.querySelectorAll(".window-btn").forEach((btn) => {
      btn.addEventListener("click", async () => {
        const newWindow = parseInt(btn.dataset.window, 10);
        if (newWindow === state.window) return;

        state.window = newWindow;
        const newData = await fetchDashboardData(newWindow);
        if (newData) {
          renderOverview(container, newData);
        }
      });
    });

    const exportBtn = container.querySelector("#exportBalanceBtn");
    if (exportBtn) {
      exportBtn.addEventListener("click", () => exportBalanceTrend(balance_trend));
    }

    const refreshBtn = container.querySelector("#refreshCacheBtn");
    if (refreshBtn) {
      refreshBtn.addEventListener("click", async () => {
        refreshBtn.disabled = true;
        refreshBtn.textContent = "⏳ Refreshing...";
        await refreshDashboardCache(state.window);
        const newData = await fetchDashboardData(state.window);
        if (newData) {
          renderOverview(container, newData);
        }
        refreshBtn.disabled = false;
        refreshBtn.textContent = "🔄 Refresh Cache";
      });
    }

    // Render chart
    renderBalanceChart(balance_trend);
  }

  function renderBalanceChart(trendData) {
    if (!trendData || trendData.length === 0) return;

    const chartContainer = document.getElementById("balanceChart");
    if (!chartContainer) return;

    // Destroy existing chart
    if (balanceChart) {
      balanceChart.remove();
      balanceChart = null;
    }

    // Create new chart
    balanceChart = window.LightweightCharts.createChart(chartContainer, {
      layout: {
        background: { color: "transparent" },
        textColor: window
          .getComputedStyle(document.documentElement)
          .getPropertyValue("--text-primary")
          .trim(),
      },
      grid: {
        vertLines: { color: "rgba(128, 128, 128, 0.1)" },
        horzLines: { color: "rgba(128, 128, 128, 0.1)" },
      },
      height: 300,
      timeScale: {
        timeVisible: true,
        secondsVisible: false,
      },
    });

    const lineSeries = balanceChart.addLineSeries({
      color: "#2962FF",
      lineWidth: 2,
    });

    lineSeries.setData(
      trendData.map((point) => ({
        time: point.timestamp,
        value: point.sol_balance,
      }))
    );

    balanceChart.timeScale().fitContent();
  }

  function exportBalanceTrend(trendData) {
    if (!trendData || trendData.length === 0) return;

    const csvData = trendData.map((point) => ({
      Timestamp: new Date(point.timestamp * 1000).toISOString(),
      "SOL Balance": point.sol_balance.toFixed(6),
    }));

    Utils.exportToCSV(csvData, ["Timestamp", "SOL Balance"], "wallet_balance_trend.csv");
  }

  // ============================================================================
  // FLOWS SUBTAB
  // ============================================================================

  function renderFlows(container, data) {
    if (!data) {
      container.innerHTML = `<div class="empty-state">No flow data available</div>`;
      return;
    }

    const { flows, daily_flows } = data;

    const netClass = flows.net_sol > 0 ? "value-positive" : flows.net_sol < 0 ? "value-negative" : "";

    container.innerHTML = `
      <div class="wallet-flows">
        <div class="window-selector">
          ${WINDOW_OPTIONS.map(
            (opt) => `
            <button 
              class="btn window-btn ${state.window === opt.hours ? "active" : ""}" 
              data-window="${opt.hours}"
            >
              ${opt.label}
            </button>
          `
          ).join("")}
        </div>

        <div class="flow-cards">
          <div class="flow-card inflow">
            <div class="flow-label">Inflow</div>
            <div class="flow-value">${formatSol(flows.inflow_sol)}</div>
          </div>
          <div class="flow-card outflow">
            <div class="flow-label">Outflow</div>
            <div class="flow-value">${formatSol(flows.outflow_sol)}</div>
          </div>
          <div class="flow-card net">
            <div class="flow-label">Net Flow</div>
            <div class="flow-value ${netClass}">${formatSol(flows.net_sol)}</div>
          </div>
          <div class="flow-card">
            <div class="flow-label">Transactions</div>
            <div class="flow-value">${flows.transactions_analyzed}</div>
          </div>
        </div>

        <div class="chart-container">
          <div class="chart-header">
            <h3>Daily Flows</h3>
            <button class="btn small" id="exportFlowsBtn">Export CSV</button>
          </div>
          <div id="flowsChart" style="height: 300px;"></div>
        </div>
      </div>
    `;

    // Attach event listeners
    container.querySelectorAll(".window-btn").forEach((btn) => {
      btn.addEventListener("click", async () => {
        const newWindow = parseInt(btn.dataset.window, 10);
        if (newWindow === state.window) return;

        state.window = newWindow;
        const newData = await fetchDashboardData(newWindow);
        if (newData) {
          renderFlows(container, newData);
        }
      });
    });

    const exportBtn = container.querySelector("#exportFlowsBtn");
    if (exportBtn) {
      exportBtn.addEventListener("click", () => exportDailyFlows(daily_flows));
    }

    // Render chart
    renderFlowsChart(daily_flows);
  }

  function renderFlowsChart(dailyFlows) {
    if (!dailyFlows || dailyFlows.length === 0) return;

    const chartContainer = document.getElementById("flowsChart");
    if (!chartContainer) return;

    // Destroy existing chart
    if (flowsChart) {
      flowsChart.remove();
      flowsChart = null;
    }

    // Create new chart
    flowsChart = window.LightweightCharts.createChart(chartContainer, {
      layout: {
        background: { color: "transparent" },
        textColor: window
          .getComputedStyle(document.documentElement)
          .getPropertyValue("--text-primary")
          .trim(),
      },
      grid: {
        vertLines: { color: "rgba(128, 128, 128, 0.1)" },
        horzLines: { color: "rgba(128, 128, 128, 0.1)" },
      },
      height: 300,
      timeScale: {
        timeVisible: true,
        secondsVisible: false,
      },
    });

    const inflowSeries = flowsChart.addHistogramSeries({
      color: "#26a69a",
      priceFormat: { type: "volume" },
    });

    const outflowSeries = flowsChart.addHistogramSeries({
      color: "#ef5350",
      priceFormat: { type: "volume" },
    });

    inflowSeries.setData(
      dailyFlows.map((point) => ({
        time: point.timestamp,
        value: point.inflow,
      }))
    );

    outflowSeries.setData(
      dailyFlows.map((point) => ({
        time: point.timestamp,
        value: -point.outflow,
      }))
    );

    flowsChart.timeScale().fitContent();
  }

  function exportDailyFlows(dailyFlows) {
    if (!dailyFlows || dailyFlows.length === 0) return;

    const csvData = dailyFlows.map((point) => ({
      Date: point.date,
      Inflow: point.inflow.toFixed(6),
      Outflow: point.outflow.toFixed(6),
      Net: point.net.toFixed(6),
      Transactions: point.tx_count,
    }));

    Utils.exportToCSV(
      csvData,
      ["Date", "Inflow", "Outflow", "Net", "Transactions"],
      "wallet_daily_flows.csv"
    );
  }

  // ============================================================================
  // HOLDINGS SUBTAB
  // ============================================================================

  function renderHoldings(container, data) {
    if (!data) {
      container.innerHTML = `<div class="empty-state">No token holdings available</div>`;
      return;
    }

    const { tokens } = data;

    container.innerHTML = `
      <div class="wallet-holdings">
        <div class="holdings-toolbar">
          <div class="holdings-info">
            <span>${tokens.length} token${tokens.length !== 1 ? "s" : ""}</span>
          </div>
          <button class="btn small" id="exportHoldingsBtn">Export CSV</button>
        </div>
        <div id="holdingsTableContainer"></div>
      </div>
    `;

    const exportBtn = container.querySelector("#exportHoldingsBtn");
    if (exportBtn) {
      exportBtn.addEventListener("click", () => exportHoldings(tokens));
    }

    // Render table
    renderHoldingsTable(tokens);
  }

  function renderHoldingsTable(tokens) {
    const tableContainer = document.querySelector("#holdingsTableContainer");
    if (!tableContainer) return;

    // Destroy existing table
    if (holdingsTable) {
      holdingsTable.destroy();
      holdingsTable = null;
    }

    const columns = [
      {
        id: "token",
        label: "Token",
        sortable: true,
        minWidth: 180,
        render: (_v, row) => tokenCell(row),
      },
      {
        id: "balance_ui",
        label: "Balance",
        sortable: true,
        minWidth: 120,
        render: (v) => (v != null ? v.toFixed(6) : "—"),
      },
      {
        id: "price_sol",
        label: "Price (SOL)",
        sortable: true,
        minWidth: 140,
        render: (v) => priceCell(v),
      },
      {
        id: "value_sol",
        label: "Value (SOL)",
        sortable: true,
        minWidth: 120,
        render: (v) => formatSol(v),
      },
      {
        id: "price_usd",
        label: "Price (USD)",
        sortable: true,
        minWidth: 120,
        render: (v) => formatUsd(v),
      },
      {
        id: "liquidity_usd",
        label: "Liquidity",
        sortable: true,
        minWidth: 120,
        render: (v) => formatUsd(v),
      },
      {
        id: "volume_24h",
        label: "24h Volume",
        sortable: true,
        minWidth: 120,
        render: (v) => formatUsd(v),
      },
      {
        id: "last_updated",
        label: "Updated",
        sortable: true,
        minWidth: 100,
        render: (v) => (v ? formatTimeAgo(Math.floor((Date.now() - new Date(v).getTime()) / 1000)) : "—"),
      },
      {
        id: "actions",
        label: "Actions",
        sortable: false,
        minWidth: 180,
        render: (_v, row) => {
          const mint = row?.mint || "";
          if (!mint) return "—";

          return `
            <div class="row-actions">
              <button class="btn row-action" data-action="details" data-mint="${escapeHtml(mint)}" title="View Details">Details</button>
              <button class="btn row-action" data-action="trade" data-mint="${escapeHtml(mint)}" title="Trade">Trade</button>
            </div>
          `;
        },
      },
    ];

    holdingsTable = new DataTable({
      container: tableContainer,
      columns,
      data: tokens,
      defaultSort: { column: "value_sol", direction: "desc" },
      onRowClick: (row, event) => {
        const button = event.target.closest("[data-action]");
        if (!button) return;

        const action = button.dataset.action;
        const mint = button.dataset.mint;

        if (action === "details") {
          showTokenDetails(mint);
        } else if (action === "trade") {
          showTradeDialog(mint, row.symbol);
        }
      },
    });
  }

  function exportHoldings(tokens) {
    if (!tokens || tokens.length === 0) return;

    const csvData = tokens.map((token) => ({
      Mint: token.mint,
      Symbol: token.symbol,
      Name: token.name || "",
      Balance: token.balance_ui.toFixed(6),
      "Price (SOL)": token.price_sol ? token.price_sol.toFixed(12) : "",
      "Value (SOL)": token.value_sol ? token.value_sol.toFixed(6) : "",
      "Price (USD)": token.price_usd ? token.price_usd.toFixed(6) : "",
      "Liquidity (USD)": token.liquidity_usd ? token.liquidity_usd.toFixed(2) : "",
      "24h Volume": token.volume_24h ? token.volume_24h.toFixed(2) : "",
      "Last Updated": token.last_updated || "",
    }));

    Utils.exportToCSV(
      csvData,
      [
        "Mint",
        "Symbol",
        "Name",
        "Balance",
        "Price (SOL)",
        "Value (SOL)",
        "Price (USD)",
        "Liquidity (USD)",
        "24h Volume",
        "Last Updated",
      ],
      "wallet_holdings.csv"
    );
  }

  // ============================================================================
  // HISTORY SUBTAB
  // ============================================================================

  function renderHistory(container) {
    container.innerHTML = `
      <div class="wallet-history">
        <div class="history-toolbar">
          <div class="history-info">
            <span>Historical Snapshots</span>
          </div>
          <button class="btn small" id="exportHistoryBtn">Export CSV</button>
        </div>
        <div id="historyTableContainer"></div>
      </div>
    `;

    const exportBtn = container.querySelector("#exportHistoryBtn");
    if (exportBtn) {
      exportBtn.addEventListener("click", () => exportHistory());
    }

    // Fetch and render history
    fetchHistory();
  }

  async function fetchHistory() {
    try {
      // Use balance trend data as proxy for history (snapshots)
      if (!state.dashboardData || !state.dashboardData.balance_trend) return;

      const snapshots = state.dashboardData.balance_trend.map((point, idx, arr) => {
        const prev = idx > 0 ? arr[idx - 1] : null;
        const change = prev ? point.sol_balance - prev.sol_balance : 0;
        const changePercent = prev && prev.sol_balance > 0 ? (change / prev.sol_balance) * 100 : null;

        return {
          timestamp: point.timestamp,
          sol_balance: point.sol_balance,
          change,
          change_percent: changePercent,
        };
      });

      renderHistoryTable(snapshots);
    } catch (error) {
      console.error("[Wallet] Failed to fetch history:", error);
    }
  }

  function renderHistoryTable(snapshots) {
    const tableContainer = document.querySelector("#historyTableContainer");
    if (!tableContainer) return;

    // Destroy existing table
    if (historyTable) {
      historyTable.destroy();
      historyTable = null;
    }

    const columns = [
      {
        id: "timestamp",
        label: "Timestamp",
        sortable: true,
        minWidth: 180,
        render: (v) => new Date(v * 1000).toLocaleString(),
      },
      {
        id: "sol_balance",
        label: "SOL Balance",
        sortable: true,
        minWidth: 120,
        render: (v) => formatSol(v),
      },
      {
        id: "change",
        label: "Change",
        sortable: true,
        minWidth: 120,
        render: (v) => {
          const cls = v > 0 ? "value-positive" : v < 0 ? "value-negative" : "";
          return `<span class="${cls}">${formatSol(v)}</span>`;
        },
      },
      {
        id: "change_percent",
        label: "Change %",
        sortable: true,
        minWidth: 100,
        render: (v) => {
          if (v == null) return "—";
          const cls = v > 0 ? "value-positive" : v < 0 ? "value-negative" : "";
          return `<span class="${cls}">${formatPercent(v)}</span>`;
        },
      },
    ];

    historyTable = new DataTable({
      container: tableContainer,
      columns,
      data: snapshots,
      defaultSort: { column: "timestamp", direction: "desc" },
    });
  }

  function exportHistory() {
    if (!state.dashboardData || !state.dashboardData.balance_trend) return;

    const snapshots = state.dashboardData.balance_trend.map((point, idx, arr) => {
      const prev = idx > 0 ? arr[idx - 1] : null;
      const change = prev ? point.sol_balance - prev.sol_balance : 0;
      const changePercent = prev && prev.sol_balance > 0 ? (change / prev.sol_balance) * 100 : null;

      return {
        Timestamp: new Date(point.timestamp * 1000).toISOString(),
        "SOL Balance": point.sol_balance.toFixed(6),
        Change: change.toFixed(6),
        "Change %": changePercent ? changePercent.toFixed(2) : "",
      };
    });

    Utils.exportToCSV(
      snapshots,
      ["Timestamp", "SOL Balance", "Change", "Change %"],
      "wallet_history.csv"
    );
  }

  // ============================================================================
  // DIALOGS
  // ============================================================================

  function showTokenDetails(mint) {
    if (!detailsDialog) {
      detailsDialog = new TokenDetailsDialog();
    }
    detailsDialog.show(mint);
  }

  function showTradeDialog(mint, symbol) {
    if (!tradeDialog) {
      tradeDialog = new TradeActionDialog();
    }
    tradeDialog.show({ mint, symbol, action: "buy" });
  }

  // ============================================================================
  // VIEW SWITCHER
  // ============================================================================

  function switchView(newView) {
    if (state.view === newView) return;
    state.view = newView;

    const root = document.querySelector("#wallet-root");
    if (!root) return;

    // Clear content
    root.innerHTML = "";

    // Render based on view
    switch (newView) {
      case "overview":
        renderOverview(root, state.dashboardData);
        break;
      case "flows":
        renderFlows(root, state.dashboardData);
        break;
      case "holdings":
        renderHoldings(root, state.dashboardData);
        break;
      case "history":
        renderHistory(root);
        break;
    }
  }

  // ============================================================================
  // LIFECYCLE
  // ============================================================================

  return {
    async init(ctx) {
      console.log("[Wallet] Initializing...");

      // Fetch initial data
      await fetchCurrentSnapshot();
      await fetchDashboardData(state.window);
    },

    async activate(ctx) {
      console.log("[Wallet] Activating...");

      // Mount TabBar
      const subTabsContainer = document.querySelector("#subTabsContainer");
      if (subTabsContainer) {
        tabBar = new TabBar({
          container: subTabsContainer,
          tabs: SUB_TABS,
          defaultTab: "overview",
          stateKey: "wallet.activeTab",
          pageName: "wallet",
          onChange: (tabId) => {
            switchView(tabId);
          },
        });

        ctx.manageTabBar(tabBar);
        subTabsContainer.style.display = "flex";

        // Trigger initial view
        const activeTab = tabBar.getActive() || "overview";
        switchView(activeTab);
      }

      // Start polling
      poller = new Poller(async () => {
        await fetchCurrentSnapshot();
        await fetchDashboardData(state.window);

        // Refresh current view
        switchView(state.view);
      }, 10000);

      ctx.managePoller(poller);
    },

    deactivate() {
      console.log("[Wallet] Deactivating...");

      // Poller is managed by lifecycle context
      // TabBar is managed by lifecycle context
    },

    dispose() {
      console.log("[Wallet] Disposing...");

      // Cleanup charts
      if (balanceChart) {
        balanceChart.remove();
        balanceChart = null;
      }

      if (flowsChart) {
        flowsChart.remove();
        flowsChart = null;
      }

      // Cleanup tables
      if (holdingsTable) {
        holdingsTable.destroy();
        holdingsTable = null;
      }

      if (historyTable) {
        historyTable.destroy();
        historyTable = null;
      }

      // Cleanup dialogs
      if (tradeDialog) {
        tradeDialog = null;
      }

      if (detailsDialog) {
        detailsDialog = null;
      }

      // Reset state
      state.view = "overview";
      state.window = 24;
      state.dashboardData = null;
      state.currentSnapshot = null;
      state.lastUpdate = null;
    },
  };
}

// Register page
registerPage("wallet", createLifecycle());
