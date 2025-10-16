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

const DEFAULT_VIEW = "pool";
const DEFAULT_SORT = { column: "symbol", direction: "asc" };
const PAGE_SIZE = 100; // client-page chunking; server also pages

function normalizePageNumber(value, fallback = null) {
  if (value === null || value === undefined) {
    return fallback;
  }
  const num = Number(value);
  if (!Number.isFinite(num)) {
    return fallback;
  }
  const int = Math.floor(num);
  if (int < 1) {
    return fallback;
  }
  return int;
}

function normalizeNonNegativeInt(value, fallback = null, { min = 0 } = {}) {
  if (value === null || value === undefined) {
    return fallback;
  }
  const num = Number(value);
  if (!Number.isFinite(num)) {
    return fallback;
  }
  const int = Math.floor(num);
  if (int < min) {
    return fallback;
  }
  return int;
}

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

function createLifecycle() {
  let table = null;
  let poller = null;
  let tabBar = null;

  const state = {
    view: DEFAULT_VIEW,
    search: "",
    summary: null,
    pageMeta: null,
  };

  const updateToolbar = () => {
    if (!table) return;
    const rows = table.getData();

    const total = rows.length;
    const withPrice = rows.filter((r) => r.has_pool_price).length;
    const blacklisted = rows.filter((r) => r.blacklisted).length;
    const openPos = rows.filter((r) => r.has_open_position).length;

    table.updateToolbarSummary([
      { id: "tokens-total", label: "Total", value: Utils.formatNumber(total) },
      { id: "tokens-priced", label: "With Price", value: Utils.formatNumber(withPrice), variant: "info" },
      { id: "tokens-positions", label: "Positions", value: Utils.formatNumber(openPos), variant: openPos > 0 ? "success" : "secondary" },
      { id: "tokens-blacklisted", label: "Blacklisted", value: Utils.formatNumber(blacklisted), variant: blacklisted > 0 ? "warning" : "success" },
    ]);

    const metaEntries = [];
    if (state.pageMeta?.timestamp) {
      metaEntries.push({
        id: "tokens-last-update",
        text: `Last update ${Utils.formatTimestamp(state.pageMeta.timestamp, { includeSeconds: true })}`,
      });
    } else {
      metaEntries.push({
        id: "tokens-last-update",
        text: "Last update —",
      });
    }

    if (state.pageMeta?.page && state.pageMeta?.totalPages) {
      metaEntries.push({
        id: "tokens-page",
        text: `Page ${state.pageMeta.page}/${state.pageMeta.totalPages}`,
      });
    }

    if (typeof state.pageMeta?.total === "number" && Number.isFinite(state.pageMeta.total)) {
      metaEntries.push({
        id: "tokens-total-count",
        text: `${Utils.formatNumber(state.pageMeta.total, { decimals: 0 })} tokens`,
      });
    }

    table.updateToolbarMeta(metaEntries);
  };

  const buildQuery = ({ page = 1 } = {}) => {
    const params = new URLSearchParams();
    params.set("view", state.view);
    if (state.search) params.set("search", state.search);
    params.set("sort_by", "symbol");
    params.set("sort_dir", "asc");
    const pageNumber = normalizePageNumber(page, 1) ?? 1;
    params.set("page", String(pageNumber));
    params.set("page_size", String(PAGE_SIZE));
    return params;
  };

  const loadTokensPage = async ({ direction, cursor, reason, signal, table }) => {
    const paginationState = typeof table?.getPaginationState === "function" ? table.getPaginationState() : null;
    const currentMeta = paginationState?.meta ?? {};

    let targetPage = normalizePageNumber(cursor, null);
    const currentPage = normalizePageNumber(currentMeta.page, null);
    const totalPages = normalizePageNumber(currentMeta.totalPages, null);

    if (targetPage === null) {
      if (direction === "next" && currentPage !== null) {
        const candidate = currentPage + 1;
        if (totalPages !== null && candidate > totalPages) {
          return { rows: [], cursorNext: null, hasMoreNext: false };
        }
        targetPage = candidate;
      } else if (direction === "prev" && currentPage !== null) {
        const candidate = Math.max(1, currentPage - 1);
        if (candidate === currentPage) {
          return { rows: [], cursorPrev: null, hasMorePrev: false };
        }
        targetPage = candidate;
      } else {
        targetPage = 1;
      }
    }

    if (direction === "next" && totalPages !== null && targetPage > totalPages) {
      return { rows: [], cursorNext: null, hasMoreNext: false };
    }
    if (direction === "prev" && targetPage < 1) {
      return { rows: [], cursorPrev: null, hasMorePrev: false };
    }

    const params = buildQuery({ page: targetPage });
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

      const responsePage = normalizePageNumber(data?.page, targetPage) ?? targetPage;
      const responseTotalPages = normalizePageNumber(data?.total_pages, totalPages ?? null) ?? responsePage;
  const totalCount = normalizeNonNegativeInt(data?.total, null);
  const pageSize = normalizeNonNegativeInt(data?.page_size, PAGE_SIZE, { min: 1 }) ?? PAGE_SIZE;

      const nextPage = responsePage < responseTotalPages ? responsePage + 1 : null;
      const prevPage = responsePage > 1 ? responsePage - 1 : null;

      state.pageMeta = {
        page: responsePage,
        totalPages: responseTotalPages,
        total: totalCount,
        pageSize,
        timestamp: data?.timestamp ?? null,
      };

      return {
        rows: items,
        cursorNext: nextPage,
        cursorPrev: prevPage,
        hasMoreNext: nextPage !== null,
        hasMorePrev: prevPage !== null,
        total: totalCount ?? items.length,
        meta: { ...state.pageMeta },
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
    state.pageMeta = null;
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

      const columns = [
        {
          id: "token",
          label: "Token",
          sortable: true,
          minWidth: 200,
          wrap: false,
          render: (_v, row) => tokenCell(row),
          sortFn: (a, b) => (a.symbol || "").localeCompare(b.symbol || ""),
        },
        { id: "price_sol", label: "Price (SOL)", sortable: true, minWidth: 120, wrap: false, render: (v) => priceCell(v), sortFn: (a, b) => (a.price_sol ?? -Infinity) - (b.price_sol ?? -Infinity) },
        { id: "liquidity_usd", label: "Liquidity", sortable: true, minWidth: 110, wrap: false, render: (v) => usdCell(v), sortFn: (a, b) => (a.liquidity_usd ?? 0) - (b.liquidity_usd ?? 0) },
        { id: "volume_24h", label: "24h Vol", sortable: true, minWidth: 110, wrap: false, render: (v) => usdCell(v), sortFn: (a, b) => (a.volume_24h ?? 0) - (b.volume_24h ?? 0) },
        { id: "fdv", label: "FDV", sortable: true, minWidth: 110, wrap: false, render: (v) => usdCell(v), sortFn: (a, b) => (a.fdv ?? 0) - (b.fdv ?? 0) },
        { id: "market_cap", label: "Mkt Cap", sortable: true, minWidth: 110, wrap: false, render: (v) => usdCell(v), sortFn: (a, b) => (a.market_cap ?? 0) - (b.market_cap ?? 0) },
        { id: "price_change_h1", label: "1h", sortable: true, minWidth: 90, wrap: false, render: (v) => percentCell(v), sortFn: (a, b) => (a.price_change_h1 ?? 0) - (b.price_change_h1 ?? 0) },
        { id: "price_change_h24", label: "24h", sortable: true, minWidth: 90, wrap: false, render: (v) => percentCell(v), sortFn: (a, b) => (a.price_change_h24 ?? 0) - (b.price_change_h24 ?? 0) },
        { id: "security_score", label: "Security", sortable: true, minWidth: 90, wrap: false, render: (v) => Utils.formatNumber(v, 0), sortFn: (a, b) => (a.security_score ?? 0) - (b.security_score ?? 0) },
        { id: "status", label: "Status", sortable: false, minWidth: 140, wrap: false, render: (_v, row) => {
            const flags = [];
            if (row.has_pool_price) flags.push("<span class=\"badge info\">Price</span>");
            if (row.has_ohlcv) flags.push("<span class=\"badge\">OHLCV</span>");
            if (row.has_open_position) flags.push("<span class=\"badge success\">Position</span>");
            if (row.blacklisted) flags.push("<span class=\"badge warning\">Blacklisted</span>");
            return flags.join(" ") || "—";
          }
        },
        { id: "price_updated_at", label: "Updated", sortable: true, minWidth: 110, wrap: false, render: (v) => timeAgoCell(v), sortFn: (a, b) => (a.price_updated_at ?? 0) - (b.price_updated_at ?? 0) },
      ];

      table = new DataTable({
        container: "#tokens-root",
        columns,
        rowIdField: "mint",
        stateKey: "tokens-table",
        enableLogging: false,
        sorting: DEFAULT_SORT,
        compact: true,
        stickyHeader: true,
        zebra: true,
        fitToContainer: true,
        autoSizeColumns: true,
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
            placeholder: "Search by symbol or mint...",
            onChange: (value) => {
              state.search = (value || "").trim();
            },
            onSubmit: () => {
              state.pageMeta = null;
              updateToolbar();
              requestReload("search", { silent: false, resetScroll: true }).catch(() => {});
            },
          },
          filters: [
            {
              id: "priced",
              label: "With Price",
              options: [
                { value: "all", label: "All" },
                { value: "priced", label: "With Price" },
                { value: "noprice", label: "No Price" },
              ],
              filterFn: (row, value) =>
                value === "all" || (value === "priced" ? row.has_pool_price : !row.has_pool_price),
            },
            {
              id: "positions",
              label: "Positions",
              options: [
                { value: "all", label: "All" },
                { value: "open", label: "Open Only" },
              ],
              filterFn: (row, value) => value === "all" || (value === "open" && row.has_open_position),
            },
          ],
          buttons: [
            {
              id: "refresh",
              label: "Refresh",
              variant: "primary",
              onClick: () => requestReload("manual", { silent: false, preserveScroll: false }).catch(() => {}),
            },
          ],
        },
      });

      updateToolbar();
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
    },
  };
}

registerPage("tokens", createLifecycle());
