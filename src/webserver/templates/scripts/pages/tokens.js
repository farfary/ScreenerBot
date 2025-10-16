import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import * as Utils from "../core/utils.js";
import { DataTable } from "../ui/data_table.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";

// Sub-tabs (views) configuration
const TOKEN_VIEWS = [
  { id: "pool", label: "💧 Pool Service" },
  { id: "all", label: "📋 All Tokens" },
  { id: "passed", label: "✅ Passed" },
  { id: "rejected", label: "⛔ Rejected" },
  { id: "blacklisted", label: "🚫 Blacklisted" },
  { id: "positions", label: "📊 Positions" },
  { id: "secure", label: "🛡️ Secure" },
  { id: "recent", label: "🆕 Recent" },
];

// Constants
const DEFAULT_VIEW = "all";
const DEFAULT_SERVER_SORT = { by: "symbol", direction: "asc" };
const DEFAULT_FILTERS = { priced: "all", positions: "all" };
const DEFAULT_SUMMARY = { priced: 0, positions: 0, blacklisted: 0 };

const COLUMN_TO_SORT_KEY = {
  token: "symbol",
  price_sol: "price_sol",
  liquidity_usd: "liquidity_usd",
  volume_24h: "volume_24h",
  fdv: "fdv",
  market_cap: "market_cap",
  price_change_h1: "price_change_h1",
  price_change_h24: "price_change_h24",
  security_score: "security_score",
  updated_at: "updated_at",
};

const SORT_KEY_TO_COLUMN = Object.entries(COLUMN_TO_SORT_KEY).reduce(
  (acc, [columnId, sortKey]) => {
    acc[sortKey] = columnId;
    return acc;
  },
  {}
);

const PAGE_LIMIT = 100; // chunked fetch size for incremental scrolling

function priceCell(value) {
  return Utils.formatPriceSol(value, { fallback: "—", decimals: 9 });
}

function usdCell(value) {
  return Utils.formatCurrencyUSD(value, { fallback: "—" });
}

function percentCell(value) {
  if (value === null || value === undefined) return "—";
  const num = Number(value);
  if (!Number.isFinite(num)) return "—";
  const cls = num > 0 ? "value-positive" : num < 0 ? "value-negative" : "";
  const text = Utils.formatPercentValue(num, { includeSign: true, decimals: 2 });
  return `<span class="${cls}">${text}</span>`;
}

function timeAgoCell(seconds) {
  return Utils.formatTimeAgo(seconds, { fallback: "—" });
}

function tokenCell(row) {
  const logo = row.logo_url
    ? `<img class="token-logo" alt="" src="${Utils.escapeHtml(row.logo_url)}" />`
    : "<span class=\"token-logo\" style=\"display:inline-block;background:var(--bg-secondary);\"></span>";
  const sym = Utils.escapeHtml(row.symbol || "—");
  const name = row.name ? `<div class="token-name">${Utils.escapeHtml(row.name)}</div>` : "";
  return `<div class="token-cell">${logo}<div><div class="token-symbol">${sym}</div>${name}</div></div>`;
}

function resolveSortColumn(sortKey) {
  if (!sortKey) {
    return null;
  }
  return SORT_KEY_TO_COLUMN[sortKey] ?? null;
}

function resolveSortKey(columnId) {
  if (!columnId) {
    return null;
  }
  return COLUMN_TO_SORT_KEY[columnId] ?? null;
}

function normalizeSortDirection(direction) {
  return direction === "desc" ? "desc" : "asc";
}

function createLifecycle() {
  let table = null;
  let poller = null;
  let tabBar = null;

  const state = {
    view: DEFAULT_VIEW,
    search: "",
    totalCount: null,
    lastUpdate: null,
    sort: { ...DEFAULT_SERVER_SORT },
    filters: { ...DEFAULT_FILTERS },
    summary: { ...DEFAULT_SUMMARY },
  };

  const updateToolbar = () => {
    if (!table) return;
    const rows = table.getData();

    const loaded = rows.length;
    const totalGlobal =
      typeof state.totalCount === "number" && Number.isFinite(state.totalCount)
        ? state.totalCount
        : loaded;

    const summaryPriced =
      typeof state.summary?.priced === "number" && Number.isFinite(state.summary.priced)
        ? state.summary.priced
        : rows.filter((r) => r.has_pool_price).length;
    const summaryPositions =
      typeof state.summary?.positions === "number" && Number.isFinite(state.summary.positions)
        ? state.summary.positions
        : rows.filter((r) => r.has_open_position).length;
    const summaryBlacklisted =
      typeof state.summary?.blacklisted === "number" && Number.isFinite(state.summary.blacklisted)
        ? state.summary.blacklisted
        : rows.filter((r) => r.blacklisted).length;

    table.updateToolbarSummary([
      {
        id: "tokens-total",
        label: "Total",
        value: Utils.formatNumber(totalGlobal),
      },
      {
        id: "tokens-priced",
        label: "With Price",
        value: Utils.formatNumber(summaryPriced),
        variant: "info",
      },
      {
        id: "tokens-positions",
        label: "Positions",
        value: Utils.formatNumber(summaryPositions),
        variant: summaryPositions > 0 ? "success" : "secondary",
      },
      {
        id: "tokens-blacklisted",
        label: "Blacklisted",
        value: Utils.formatNumber(summaryBlacklisted),
        variant: summaryBlacklisted > 0 ? "warning" : "success",
      },
    ]);

    const metaEntries = [];
    metaEntries.push({
      id: "tokens-last-update",
      text: state.lastUpdate
        ? `Last update ${Utils.formatTimestamp(state.lastUpdate, { includeSeconds: true })}`
        : "Last update —",
    });

    const loadedLabel = Utils.formatNumber(loaded, { decimals: 0 });
    const hasTotalCount =
      typeof state.totalCount === "number" && Number.isFinite(state.totalCount);
    const totalLabel = hasTotalCount
      ? `Loaded ${loadedLabel} / ${Utils.formatNumber(state.totalCount, { decimals: 0 })}`
      : `Loaded ${loadedLabel}`;

    metaEntries.push({
      id: "tokens-loaded",
      text: `${totalLabel} tokens`,
    });

    table.updateToolbarMeta(metaEntries);
  };

  const buildQuery = ({ cursor = null } = {}) => {
    const params = new URLSearchParams();
    params.set("view", state.view);
    if (state.search) params.set("search", state.search);
    const sort = state.sort ?? DEFAULT_SERVER_SORT;
    const sortBy = sort?.by ?? DEFAULT_SERVER_SORT.by;
    const sortDir = sort?.direction ?? DEFAULT_SERVER_SORT.direction;
    params.set("sort_by", sortBy);
    params.set("sort_dir", sortDir);
    if (state.filters.priced === "priced") {
      params.set("has_pool_price", "true");
    } else if (state.filters.priced === "noprice") {
      params.set("has_pool_price", "false");
    }
    if (state.filters.positions === "open") {
      params.set("has_open_position", "true");
    }
    params.set("limit", String(PAGE_LIMIT));
    if (cursor !== null && cursor !== undefined) {
      params.set("cursor", String(cursor));
    }
    return params;
  };

  const shouldSkipPollReload = () => {
    if (!table) return false;

    const paginationState =
      typeof table.getPaginationState === "function"
        ? table.getPaginationState()
        : null;
    if (paginationState?.loadingNext || paginationState?.loadingPrev) {
      return true;
    }

    const container = table?.elements?.scrollContainer;
    if (!container) {
      return false;
    }

    const hasScrollableContent =
      container.scrollHeight > container.clientHeight + 16;
    if (!hasScrollableContent) {
      return false;
    }

    const nearTop = container.scrollTop <= 120;
    return !nearTop;
  };

  const syncTableSortState = (options = {}) => {
    if (!table) {
      return;
    }
    const columnId = resolveSortColumn(state.sort.by);
    table.setSortState(columnId, state.sort.direction, options);
  };

  const syncToolbarFilters = () => {
    if (!table) {
      return;
    }
    table.setToolbarFilterValue("priced", state.filters.priced, {
      apply: false,
    });
    table.setToolbarFilterValue("positions", state.filters.positions, {
      apply: false,
    });
  };

  const handleSortChange = ({ column, direction }) => {
    const sortKey = resolveSortKey(column);
    if (!sortKey) {
      syncTableSortState({ render: true });
      return;
    }

    const nextDirection = normalizeSortDirection(direction);
    state.sort = { by: sortKey, direction: nextDirection };
    state.totalCount = null;
    state.lastUpdate = null;
    updateToolbar();
    requestReload("sort", {
      silent: false,
      resetScroll: true,
    }).catch(() => {});
  };

  const loadTokensPage = async ({ direction = "initial", cursor, reason, signal }) => {
    if (direction === "prev") {
      const currentTotal = state.totalCount ?? table?.getData?.().length ?? 0;
      return {
        rows: [],
        cursorPrev: null,
        hasMorePrev: false,
        total: currentTotal,
        meta: { timestamp: state.lastUpdate },
        preserveScroll: true,
      };
    }

    const params = buildQuery({ cursor });
    const url = `/api/tokens/list?${params.toString()}`;

    try {
      const response = await fetch(url, {
        headers: { "X-Requested-With": "fetch" },
        cache: "no-store",
        signal,
      });
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();
      const items = Array.isArray(data?.items) ? data.items : [];

      if (typeof data?.total === "number" && Number.isFinite(data.total)) {
        state.totalCount = data.total;
      } else {
        state.totalCount = null;
      }

      state.lastUpdate = data?.timestamp ?? null;
      const pricedTotal =
        typeof data?.priced_total === "number" && Number.isFinite(data.priced_total)
          ? data.priced_total
          : null;
      const positionsTotal =
        typeof data?.positions_total === "number" && Number.isFinite(data.positions_total)
          ? data.positions_total
          : null;
      const blacklistedTotal =
        typeof data?.blacklisted_total === "number" && Number.isFinite(data.blacklisted_total)
          ? data.blacklisted_total
          : null;

      state.summary = {
        priced:
          pricedTotal !== null
            ? pricedTotal
            : items.filter((row) => row.has_pool_price).length,
        positions:
          positionsTotal !== null
            ? positionsTotal
            : items.filter((row) => row.has_open_position).length,
        blacklisted:
          blacklistedTotal !== null
            ? blacklistedTotal
            : items.filter((row) => row.blacklisted).length,
      };

      return {
        rows: items,
        cursorNext: data?.next_cursor ?? null,
        cursorPrev: null,
        hasMoreNext: Boolean(data?.next_cursor),
        hasMorePrev: false,
        total: state.totalCount ?? items.length,
        meta: { timestamp: state.lastUpdate },
        preserveScroll: reason === "poll",
      };
    } catch (error) {
      if (error?.name === "AbortError") {
        throw error;
      }
      console.error("[Tokens] Failed to load tokens list:", error);
      if (reason !== "poll") {
        Utils.showToast("⚠️ Failed to load tokens", "warning");
      }
      throw error;
    }
  };

  const requestReload = (reason = "manual", options = {}) => {
    if (!table) return Promise.resolve(null);
    if (reason === "poll" && shouldSkipPollReload()) {
      return Promise.resolve(null);
    }
    return table.reload({
      reason,
      silent: options.silent ?? false,
      preserveScroll: options.preserveScroll ?? false,
      resetScroll: options.resetScroll ?? false,
    });
  };

  const switchView = (view) => {
    if (!TOKEN_VIEWS.some((v) => v.id === view)) return;
    state.view = view;
    state.totalCount = null;
    state.lastUpdate = null;
    state.sort = { ...DEFAULT_SERVER_SORT };
    state.filters = { ...DEFAULT_FILTERS };
    state.summary = { ...DEFAULT_SUMMARY };
    syncTableSortState({ render: true });
    syncToolbarFilters();
    updateToolbar();
    requestReload("view", { silent: false, resetScroll: true }).catch(() => {});
  };

  return {
    init(ctx) {
      // Initialize tab bar for tokens page
      tabBar = new TabBar({
        container: "#subTabsContainer",
        tabs: TOKEN_VIEWS,
        defaultTab: DEFAULT_VIEW,
        stateKey: "tokens.activeTab",
        pageName: "tokens",
        onChange: (tabId) => {
          switchView(tabId);
        },
      });

      // Register with TabBarManager for page-switch coordination
      TabBarManager.register("tokens", tabBar);

      // Integrate with lifecycle for auto-cleanup
      ctx.manageTabBar(tabBar);

      // Show the tab bar
      tabBar.show();

      // Get the active tab after state restoration
      const activeTab = tabBar.getActiveTab();
      if (activeTab && activeTab !== state.view) {
        // Sync state with tab bar's restored state (e.g., from URL hash)
        state.view = activeTab;
      }

      const initialSortColumn = resolveSortColumn(state.sort.by);

      const columns = [
        {
          id: "token",
          label: "Token",
          sortable: true,
          minWidth: 180,
          wrap: false,
          render: (_v, row) => tokenCell(row),
        },
        {
          id: "price_sol",
          label: "Price (SOL)",
          sortable: true,
          minWidth: 120,
          wrap: false,
          render: (v) => priceCell(v),
        },
        {
          id: "liquidity_usd",
          label: "Liquidity",
          sortable: true,
          minWidth: 110,
          wrap: false,
          render: (v) => usdCell(v),
        },
        {
          id: "volume_24h",
          label: "24h Vol",
          sortable: true,
          minWidth: 110,
          wrap: false,
          render: (v) => usdCell(v),
        },
        {
          id: "fdv",
          label: "FDV",
          sortable: true,
          minWidth: 110,
          wrap: false,
          render: (v) => usdCell(v),
        },
        {
          id: "market_cap",
          label: "Mkt Cap",
          sortable: true,
          minWidth: 110,
          wrap: false,
          render: (v) => usdCell(v),
        },
        {
          id: "price_change_h1",
          label: "1h",
          sortable: true,
          minWidth: 90,
          wrap: false,
          render: (v) => percentCell(v),
        },
        {
          id: "price_change_h24",
          label: "24h",
          sortable: true,
          minWidth: 90,
          wrap: false,
          render: (v) => percentCell(v),
        },
        {
          id: "security_score",
          label: "Security",
          sortable: true,
          minWidth: 90,
          wrap: false,
          render: (v) => Utils.formatNumber(v, 0),
        },
        {
          id: "status",
          label: "Status",
          sortable: false,
          minWidth: 140,
          wrap: false,
          render: (_v, row) => {
            const flags = [];
            if (row.has_pool_price) flags.push("<span class=\"badge info\">Price</span>");
            if (row.has_ohlcv) flags.push("<span class=\"badge\">OHLCV</span>");
            if (row.has_open_position) flags.push("<span class=\"badge success\">Position</span>");
            if (row.blacklisted) flags.push("<span class=\"badge warning\">Blacklisted</span>");
            return flags.join(" ") || "—";
          },
        },
        {
          id: "updated_at",
          label: "Updated",
          sortable: true,
          minWidth: 100,
          wrap: false,
          render: (v) => timeAgoCell(v),
        },
      ];

      table = new DataTable({
        container: "#tokens-root",
        columns,
        rowIdField: "mint",
        stateKey: "tokens-table.v2",
        enableLogging: false,
        sorting: {
          mode: "server",
          column: initialSortColumn,
          direction: state.sort.direction,
          onChange: handleSortChange,
        },
        compact: true,
        stickyHeader: true,
        zebra: true,
        fitToContainer: true,
        autoSizeColumns: false,
        uniformRowHeight: 2,
        pagination: {
          threshold: 160,
          maxRows: 5000,
          loadPage: loadTokensPage,
          dedupeKey: (row) => row?.mint ?? null,
          rowIdField: "mint",
          onPageLoaded: () => updateToolbar(),
        },
        toolbar: {
          title: {
            icon: "🪙",
            text: "Tokens",
            meta: [{ id: "tokens-last-update", text: "Last update —" }],
          },
          summary: [
            { id: "tokens-total", label: "Total", value: "0" },
            { id: "tokens-priced", label: "With Price", value: "0", variant: "info" },
            { id: "tokens-positions", label: "Positions", value: "0", variant: "secondary" },
            { id: "tokens-blacklisted", label: "Blacklisted", value: "0", variant: "warning" },
          ],
          search: {
            enabled: true,
            mode: "server",
            placeholder: "Search by symbol or mint...",
            onChange: (value) => {
              state.search = (value || "").trim();
            },
            onSubmit: () => {
              state.totalCount = null;
              state.lastUpdate = null;
              updateToolbar();
              requestReload("search", {
                silent: false,
                resetScroll: true,
              }).catch(() => {});
            },
          },
          filters: [
            {
              id: "priced",
              label: "With Price",
              mode: "server",
              autoApply: false,
              defaultValue: DEFAULT_FILTERS.priced,
              options: [
                { value: "all", label: "All" },
                { value: "priced", label: "With Price" },
                { value: "noprice", label: "No Price" },
              ],
              onChange: (value) => {
                state.filters.priced = value;
                state.totalCount = null;
                state.lastUpdate = null;
                updateToolbar();
                requestReload("filters", {
                  silent: false,
                  resetScroll: true,
                }).catch(() => {});
              },
            },
            {
              id: "positions",
              label: "Positions",
              mode: "server",
              autoApply: false,
              defaultValue: DEFAULT_FILTERS.positions,
              options: [
                { value: "all", label: "All" },
                { value: "open", label: "Open Only" },
              ],
              onChange: (value) => {
                state.filters.positions = value;
                state.totalCount = null;
                state.lastUpdate = null;
                updateToolbar();
                requestReload("filters", {
                  silent: false,
                  resetScroll: true,
                }).catch(() => {});
              },
            },
          ],
          buttons: [
            {
              id: "refresh",
              label: "Refresh",
              variant: "primary",
              onClick: () =>
                requestReload("manual", {
                  silent: false,
                  preserveScroll: false,
                }).catch(() => {}),
            },
          ],
        },
      });

      syncTableSortState({ render: false });
      syncToolbarFilters();
      table.setToolbarSearchValue(state.search, { apply: false });
      updateToolbar();

      // Trigger initial data load for the active view
      // This is needed because TabBar restoration with { silent: true } doesn't trigger onChange
      requestReload("initial", { silent: false, resetScroll: true }).catch(() => {});
    },

    activate(ctx) {
      if (!poller) {
        poller = ctx.managePoller(
          new Poller(() => requestReload("poll", { silent: true, preserveScroll: true }), {
            label: "Tokens",
          })
        );
      }
      poller.start();
      if ((table?.getData?.() ?? []).length === 0) {
        requestReload("initial", { silent: false, resetScroll: true }).catch(() => {});
      }
    },

    deactivate() {
      table?.cancelPendingLoad();
    },

    dispose() {
      if (table) {
        table.destroy();
        table = null;
      }
      poller = null;
      tabBar = null; // Cleaned up automatically by manageTabBar
      TabBarManager.unregister("tokens");
      state.view = DEFAULT_VIEW;
      state.search = "";
      state.totalCount = null;
      state.lastUpdate = null;
      state.sort = { ...DEFAULT_SERVER_SORT };
      state.filters = { ...DEFAULT_FILTERS };
      state.summary = { ...DEFAULT_SUMMARY };
    },
  };
}

registerPage("tokens", createLifecycle());
