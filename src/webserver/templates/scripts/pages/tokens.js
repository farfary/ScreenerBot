import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import * as Utils from "../core/utils.js";
import { DataTable } from "../ui/data_table.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import { TradeActionDialog } from "../ui/trade_action_dialog.js";
import { TokenDetailsDialog } from "../ui/token_details_dialog.js";

// Sub-tabs (views) configuration
const TOKEN_VIEWS = [
  { id: "pool", label: "💧 Pool Service" },
  { id: "no_market", label: "📉 No Market Data" },
  { id: "all", label: "📋 All Tokens" },
  { id: "passed", label: "✅ Passed" },
  { id: "rejected", label: "⛔ Rejected" },
  { id: "blacklisted", label: "🚫 Blacklisted" },
  { id: "positions", label: "📊 Positions" },
  { id: "recent", label: "🆕 Recent" },
];

// Constants
const DEFAULT_VIEW = "all";
const DEFAULT_SERVER_SORT = { by: "symbol", direction: "asc" };
const DEFAULT_FILTERS = {
  pool_price: false,
  positions: false,
  rejection_reason: "all",
};
const DEFAULT_SUMMARY = { priced: 0, positions: 0, blacklisted: 0 };

const getDefaultFiltersForView = (view) => {
  const filters = { ...DEFAULT_FILTERS };
  if (view === "positions") {
    filters.positions = true;
  }
  return filters;
};

const COLUMN_TO_SORT_KEY = {
  token: "symbol",
  price_sol: "price_sol",
  liquidity_usd: "liquidity_usd",
  volume_24h: "volume_24h",
  fdv: "fdv",
  market_cap: "market_cap",
  price_change_h1: "price_change_h1",
  price_change_h24: "price_change_h24",
  txns_5m: "txns_5m",
  txns_1h: "txns_1h",
  txns_6h: "txns_6h",
  txns_24h: "txns_24h",
  risk_score: "risk_score",
  updated_at: "updated_at",
  first_seen_at: "first_seen_at",
  token_birth_at: "token_birth_at",
};

const SORT_KEY_TO_COLUMN = Object.entries(COLUMN_TO_SORT_KEY).reduce((acc, [columnId, sortKey]) => {
  acc[sortKey] = columnId;
  return acc;
}, {});

const PAGE_LIMIT = 100; // chunked fetch size for incremental scrolling

function priceCell(value) {
  return Utils.formatPriceSol(value, { fallback: "—", decimals: 12 });
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
  const src = row.logo_url || row.image_url;
  const logo = src
    ? `<img class="token-logo clickable-logo" alt="" src="${Utils.escapeHtml(src)}" data-logo-url="${Utils.escapeHtml(src)}" data-token-symbol="${Utils.escapeHtml(row.symbol || "")}" data-token-name="${Utils.escapeHtml(row.name || "")}" data-token-mint="${Utils.escapeHtml(row.mint || "")}" title="Click to enlarge" />`
    : '<span class="token-logo">N/A</span>';
  const sym = Utils.escapeHtml(row.symbol || "—");
  const name = row.name ? `<div class="token-name">${Utils.escapeHtml(row.name)}</div>` : "";
  return `<div class="token-cell">${logo}<div><div class="token-symbol">${sym}</div>${name}</div></div>`;
}

function normalizeBlacklistReasons(mint, sourcesMap) {
  if (!mint || typeof mint !== "string") return [];
  if (!sourcesMap || typeof sourcesMap !== "object") return [];
  const raw = sourcesMap[mint];
  if (!Array.isArray(raw) || raw.length === 0) return [];
  return raw
    .filter((entry) => entry && typeof entry === "object")
    .map((entry) => {
      const category = typeof entry.category === "string" && entry.category.trim().length > 0
        ? entry.category.trim()
        : "unknown";
      const reason = typeof entry.reason === "string" && entry.reason.trim().length > 0
        ? entry.reason.trim()
        : "unknown_reason";
      const detail = typeof entry.detail === "string" && entry.detail.trim().length > 0
        ? entry.detail.trim()
        : null;
      return { category, reason, detail };
    });
}
function summarizeBlacklistReasons(sourceList) {
  if (!Array.isArray(sourceList) || sourceList.length === 0) return "";
  return sourceList
    .map((source) => {
      if (!source || typeof source !== "object") return "unknown";
      const category = source.category || "unknown";
      const reason = source.reason || "unknown_reason";
      const detail = source.detail ? ` (${source.detail})` : "";
      return `${category}:${reason}${detail}`;
    })
    .join(", ");
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
  let tradeDialog = null;
  let tokenDetailsDialog = null;
  let walletBalance = 0;
  let lastUpdateInterval = null; // Track interval for updating "Last Update" display

  const state = {
    view: DEFAULT_VIEW,
    search: "",
    totalCount: null,
    lastUpdate: null,
    sort: { ...DEFAULT_SERVER_SORT },
    filters: getDefaultFiltersForView(DEFAULT_VIEW),
    summary: { ...DEFAULT_SUMMARY },
    availableRejectionReasons: [],
  };

  const parseToggleValue = (value, fallback = false) => {
    if (typeof value === "boolean") {
      return value;
    }
    if (typeof value === "string") {
      const normalized = value.trim().toLowerCase();
      if (normalized === "true" || normalized === "1" || normalized === "on") {
        return true;
      }
      if (normalized === "false" || normalized === "0" || normalized === "off") {
        return false;
      }
    }
    return fallback;
  };

  const updateFilterVisibility = () => {
    if (!table?.elements?.toolbar) {
      return;
    }
    const toolbar = table.elements.toolbar;

    // Only show rejection_reason filter in "rejected" tab
    const reasonField = toolbar.querySelector(
      '.table-toolbar-field[data-filter-id="rejection_reason"]'
    );
    if (reasonField) {
      const shouldShow = state.view === "rejected";
      reasonField.style.display = shouldShow ? "" : "none";
    }
  };

  const updateRejectionReasonOptions = () => {
    if (!table?.elements?.toolbar) {
      return;
    }

    const select = table.elements.toolbar.querySelector(
      '.dt-filter[data-filter-id="rejection_reason"]'
    );
    if (!select) {
      return;
    }

    const reasons = Array.isArray(state.availableRejectionReasons)
      ? Array.from(new Set(state.availableRejectionReasons.filter((item) => item && item.trim())))
      : [];

    reasons.sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase()));

    const currentValue = state.filters.rejection_reason || "all";
    const optionMarkup = [
      '<option value="all">All</option>',
      ...reasons.map((reason) => {
        const escaped = Utils.escapeHtml(reason);
        return `<option value="${escaped}">${escaped}</option>`;
      }),
    ].join("");

    if (select.innerHTML !== optionMarkup) {
      select.innerHTML = optionMarkup;
    }

    const normalizedCurrent = reasons.some((reason) => reason === currentValue)
      ? currentValue
      : "all";

    if (normalizedCurrent !== currentValue) {
      state.filters.rejection_reason = normalizedCurrent;
    }

    select.value = normalizedCurrent;
  };

  const applyViewPreferences = () => {
    if (!table) return;
    const showRejectReason = state.view === "rejected";
    if (table.state && typeof table.state === "object") {
      table.state.visibleColumns = table.state.visibleColumns || {};
      table.state.visibleColumns.reject_reason = showRejectReason;
    }
    const column = Array.isArray(table.options?.columns)
      ? table.options.columns.find((col) => col.id === "reject_reason")
      : null;
    if (column) {
      column.visible = showRejectReason;
    }
    if (typeof table._renderTable === "function") {
      table._renderTable();
    }
    updateFilterVisibility();
    updateRejectionReasonOptions();
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
        value: Utils.formatNumber(totalGlobal, 0),
      },
      {
        id: "tokens-priced",
        label: "With Price",
        value: Utils.formatNumber(summaryPriced, 0),
        variant: "info",
      },
      {
        id: "tokens-positions",
        label: "Positions",
        value: Utils.formatNumber(summaryPositions, 0),
        variant: summaryPositions > 0 ? "success" : "secondary",
      },
      {
        id: "tokens-blacklisted",
        label: "Blacklisted",
        value: Utils.formatNumber(summaryBlacklisted, 0),
        variant: summaryBlacklisted > 0 ? "warning" : "success",
      },
    ]);

    const metaEntries = [];

    let lastUpdateLines = ["Last Update", "—"];
    if (state.lastUpdate) {
      const parsed = new Date(state.lastUpdate);
      if (!Number.isNaN(parsed.getTime())) {
        const dateLine = parsed.toLocaleDateString(undefined, {
          year: "numeric",
          month: "short",
          day: "numeric",
        });
        const timeLine = parsed.toLocaleTimeString(undefined, {
          hour: "2-digit",
          minute: "2-digit",
          second: "2-digit",
        });
        lastUpdateLines = ["Last Update", `${dateLine} · ${timeLine}`];
      }
    }

    metaEntries.push({
      id: "tokens-last-update",
      lines: lastUpdateLines,
    });

    const loadedLabel = Utils.formatNumber(loaded, { decimals: 0 });
    const hasTotalCount = typeof state.totalCount === "number" && Number.isFinite(state.totalCount);
    const loadedValue = hasTotalCount
      ? `${loadedLabel} / ${Utils.formatNumber(state.totalCount, { decimals: 0 })}`
      : `${loadedLabel}`;

    metaEntries.push({
      id: "tokens-loaded",
      lines: ["Loaded", `${loadedValue} tokens`],
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
    if (state.filters.pool_price) {
      params.set("has_pool_price", "true");
    }
    if (state.filters.positions) {
      params.set("has_open_position", "true");
    }
    if (
      state.view === "rejected" &&
      state.filters.rejection_reason &&
      state.filters.rejection_reason !== "all"
    ) {
      params.set("rejection_reason", state.filters.rejection_reason);
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
      typeof table.getPaginationState === "function" ? table.getPaginationState() : null;
    if (paginationState?.loadingNext || paginationState?.loadingPrev) {
      return true;
    }

    const container = table?.elements?.scrollContainer;
    if (!container) {
      return false;
    }

    const hasScrollableContent = container.scrollHeight > container.clientHeight + 16;
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
    table.setToolbarFilterValue("rejection_reason", state.filters.rejection_reason, {
      apply: false,
    });
    updateFilterVisibility();
    updateRejectionReasonOptions();
  };

  const handleSortChange = ({ column, direction, restored }) => {
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

    // For restored state, we still need to load data, but silently
    requestReload(restored ? "restored" : "sort", {
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

      const rejectionReasons =
        data &&
        typeof data === "object" &&
        data.rejection_reasons &&
        typeof data.rejection_reasons === "object"
          ? data.rejection_reasons
          : null;

      const blacklistSourcesMap =
        data &&
        typeof data === "object" &&
        data.blacklist_reasons &&
        typeof data.blacklist_reasons === "object"
          ? data.blacklist_reasons
          : null;

      const normalizedItems = items.map((row) => {
        if (!row || typeof row !== "object") return row;
        const hasServerReason =
          rejectionReasons &&
          row.mint &&
          Object.prototype.hasOwnProperty.call(rejectionReasons, row.mint);
        const resolvedReason = hasServerReason
          ? rejectionReasons[row.mint]
          : (row.reject_reason ?? null);
        const normalizedSources = normalizeBlacklistReasons(row.mint, blacklistSourcesMap);
        return {
          ...row,
          reject_reason: resolvedReason ?? null,
          blacklist_reasons: normalizedSources,
        };
      });

      if (Array.isArray(data?.available_rejection_reasons)) {
        state.availableRejectionReasons = data.available_rejection_reasons.filter(
          (reason) => typeof reason === "string" && reason.trim().length > 0
        );
      } else {
        state.availableRejectionReasons = [];
      }

      if (state.view === "rejected") {
        updateRejectionReasonOptions();
        if (table) {
          table.setToolbarFilterValue("rejection_reason", state.filters.rejection_reason, {
            apply: false,
          });
        }
      }

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
            : normalizedItems.filter((row) => row.has_pool_price).length,
        positions:
          positionsTotal !== null
            ? positionsTotal
            : normalizedItems.filter((row) => row.has_open_position).length,
        blacklisted:
          blacklistedTotal !== null
            ? blacklistedTotal
            : normalizedItems.filter((row) => row.blacklisted).length,
      };

      return {
        rows: normalizedItems,
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

  /**
   * Build columns array based on current view
   * Different views show different conditional columns (Actions, reject_reason, blacklist_reason)
   */
  const buildColumns = () => {
    return [
      {
        id: "token",
        label: "Token",
        sortable: true,
        minWidth: 180,
        wrap: false,
        render: (_v, row) => tokenCell(row),
      },
      // Conditionally add Actions column for Pool view
      ...(state.view === "pool"
        ? [
            {
              id: "actions",
              label: "Actions",
              sortable: false,
              minWidth: 100,
              wrap: false,
              render: (_v, row) => {
                const mint = row?.mint || "";
                const isBlacklisted = Boolean(row?.blacklisted);
                const hasOpen = Boolean(row?.has_open_position);
                const disabledAttr = isBlacklisted ? ' disabled aria-disabled="true"' : "";

                if (!mint) return "—";

                if (hasOpen) {
                  return `
                    <div class="row-actions">
                      <button class="btn row-action" data-action="add" data-mint="${Utils.escapeHtml(
                        mint
                      )}" title="Add to position (DCA)"${disabledAttr}>Add</button>
                      <button class="btn warning row-action" data-action="sell" data-mint="${Utils.escapeHtml(
                        mint
                      )}" title="Sell (full or % partial)"${disabledAttr}>Sell</button>
                    </div>
                  `;
                }

                return `
                  <div class="row-actions">
                    <button class="btn success row-action" data-action="buy" data-mint="${Utils.escapeHtml(
                      mint
                    )}" title="Buy position"${disabledAttr}>Buy</button>
                  </div>
                `;
              },
            },
          ]
        : []),
      {
        id: "links",
        label: "Links",
        sortable: false,
        minWidth: 70,
        wrap: false,
        render: (_v, row) => {
          const mint = row?.mint || "";
          if (!mint) return "—";
          return `
            <button 
              class="btn btn-sm links-dropdown-trigger" 
              data-mint="${Utils.escapeHtml(mint)}"
              title="External links"
              type="button"
            >
              🔗
            </button>
          `;
        },
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
        id: "txns_5m",
        label: "Txns 5m",
        sortable: true,
        minWidth: 80,
        wrap: false,
        render: (_v, row) => {
          const buys = row.txns_5m_buys || 0;
          const sells = row.txns_5m_sells || 0;
          if (buys === 0 && sells === 0) return "—";
          return `${Utils.formatNumber(buys, 0)}/${Utils.formatNumber(sells, 0)}`;
        },
      },
      {
        id: "txns_1h",
        label: "Txns 1h",
        sortable: true,
        minWidth: 80,
        wrap: false,
        render: (_v, row) => {
          const buys = row.txns_1h_buys || 0;
          const sells = row.txns_1h_sells || 0;
          if (buys === 0 && sells === 0) return "—";
          return `${Utils.formatNumber(buys, 0)}/${Utils.formatNumber(sells, 0)}`;
        },
      },
      {
        id: "txns_6h",
        label: "Txns 6h",
        sortable: true,
        minWidth: 80,
        wrap: false,
        render: (_v, row) => {
          const buys = row.txns_6h_buys || 0;
          const sells = row.txns_6h_sells || 0;
          if (buys === 0 && sells === 0) return "—";
          return `${Utils.formatNumber(buys, 0)}/${Utils.formatNumber(sells, 0)}`;
        },
      },
      {
        id: "txns_24h",
        label: "Txns 24h",
        sortable: true,
        minWidth: 90,
        wrap: false,
        render: (_v, row) => {
          const buys = row.txns_24h_buys || 0;
          const sells = row.txns_24h_sells || 0;
          if (buys === 0 && sells === 0) return "—";
          return `${Utils.formatNumber(buys, 0)}/${Utils.formatNumber(sells, 0)}`;
        },
      },
      {
        id: "risk_score",
        label: "Risk Score",
        sortable: true,
        minWidth: 90,
        wrap: false,
        render: (v) => {
          if (v == null) return "—";
          // Raw rugcheck score: lower = safer, higher = more risky
          const num = Utils.formatNumber(v, 0);
          if (v <= 1000) return `<span style="color: var(--success)">${num}</span>`;
          if (v <= 10000) return `<span style="color: var(--warning)">${num}</span>`;
          return `<span style="color: var(--error)">${num}</span>`;
        },
      },
      // Conditionally add reject_reason column only for rejected view
      ...(state.view === "rejected"
        ? [
            {
              id: "reject_reason",
              label: "Reject Reason",
              sortable: false,
              minWidth: 220,
              wrap: true,
              render: (value) => {
                if (!value) return "—";
                return Utils.escapeHtml(String(value));
              },
            },
          ]
        : []),
      // Conditionally add blacklist_reason column only for blacklisted view
      ...(state.view === "blacklisted"
        ? [
            {
              id: "blacklist_reason",
              label: "Blacklist Reason",
              sortable: false,
              minWidth: 250,
              wrap: true,
              render: (_v, row) => {
                const summary = summarizeBlacklistReasons(row.blacklist_reasons);
                if (!summary) return "—";
                return Utils.escapeHtml(summary);
              },
            },
          ]
        : []),
      {
        id: "status",
        label: "Status",
        sortable: false,
        minWidth: 140,
        wrap: false,
        render: (_v, row) => {
          const flags = [];
          if (row.has_pool_price) flags.push('<span class="badge info">Price</span>');
          if (row.has_ohlcv) flags.push('<span class="badge">OHLCV</span>');
          if (row.has_open_position) flags.push('<span class="badge success">Position</span>');
          if (row.blacklisted) {
            const summary = summarizeBlacklistReasons(row.blacklist_reasons);
            const tooltip = summary ? `Blacklisted: ${summary}` : "Blacklisted token";
            flags.push(
              `<span class="badge warning" title="${Utils.escapeHtml(tooltip)}">Blacklisted</span>`
            );
          }
          return flags.join(" ") || "—";
        },
      },
      {
        id: "updated_at",
        label: "Updated",
        sortable: true,
        minWidth: 100,
        wrap: false,
        render: (v, row) => {
          const source =
            typeof row?.data_source === "string" ? row.data_source.toLowerCase() : "";
          if (!v || source === "unknown") {
            return "—";
          }
          return timeAgoCell(v);
        },
      },
      {
        id: "token_birth_at",
        label: "Birth",
        sortable: true,
        minWidth: 110,
        wrap: false,
        render: (_v, row) => {
          const value = row.token_birth_at || row.first_seen_at;
          return timeAgoCell(value);
        },
      },
      {
        id: "first_seen_at",
        label: "First Seen",
        sortable: true,
        minWidth: 110,
        wrap: false,
        render: (v) => timeAgoCell(v),
      },
    ];
  };

  const switchView = (view) => {
    if (!TOKEN_VIEWS.some((v) => v.id === view)) return;
    state.view = view;
    state.totalCount = null;
    state.lastUpdate = null;
    state.filters = getDefaultFiltersForView(view);
    state.summary = { ...DEFAULT_SUMMARY };
    state.availableRejectionReasons = [];

    // Update table stateKey for per-tab state persistence
    if (table) {
      table.setStateKey(`tokens-table-v2.${view}`, { render: false }); // v2: fixed column order

      // Update columns for the new view (different views have different conditional columns)
      const newColumns = buildColumns();
      table.setColumns(newColumns, {
        preserveData: true,
        preserveScroll: false, // Reset scroll when switching views
        resetState: false, // Keep column widths/visibility preferences within each view
      });

      // Read restored sort state from table and sync to state.sort
      const restoredTableState = table.getServerState();
      if (restoredTableState?.sortColumn) {
        const sortKey = resolveSortKey(restoredTableState.sortColumn);
        if (sortKey) {
          state.sort = {
            by: sortKey,
            direction: restoredTableState.sortDirection || "asc",
          };
        } else {
          state.sort = { ...DEFAULT_SERVER_SORT };
        }
      } else {
        state.sort = { ...DEFAULT_SERVER_SORT };
      }
    } else {
      state.sort = { ...DEFAULT_SERVER_SORT };
    }

    applyViewPreferences();
    syncTableSortState({ render: true });
    syncToolbarFilters();
    updateToolbar();
    requestReload("view", { silent: false, resetScroll: true }).catch(() => {});
  };

  const handleLinkAction = (actionId, mint) => {
    const urlMap = {
      dexscreener: `https://dexscreener.com/solana/${mint}`,
      gmgn: `https://gmgn.ai/sol/token/${mint}`,
      solscan: `https://solscan.io/token/${mint}`,
      birdeye: `https://birdeye.so/token/${mint}`,
      rugcheck: `https://rugcheck.xyz/tokens/${mint}`,
      pumpfun: `https://pump.fun/${mint}`,
    };

    if (actionId === "copy") {
      navigator.clipboard
        .writeText(mint)
        .then(() => {
          Utils.showToast("📋 Mint address copied", "success");
        })
        .catch((err) => {
          console.error("Failed to copy mint:", err);
          Utils.showToast("⚠️ Failed to copy mint", "warning");
        });
    } else if (urlMap[actionId]) {
      window.open(urlMap[actionId], "_blank", "noopener,noreferrer");
    }
  };

  const initializeLinkDropdowns = () => {
    // Use event delegation instead of creating Dropdown instances
    // This works better with dynamic table data that re-renders
    if (!table?.elements?.scrollContainer) return;

    // Remove existing listener if any
    const container = table.elements.scrollContainer;
    if (container._linksClickHandler) {
      container.removeEventListener("click", container._linksClickHandler);
    }

    // Add delegated click handler
    const clickHandler = (e) => {
      const trigger = e.target.closest(".links-dropdown-trigger");
      if (!trigger) return;

      e.preventDefault();
      e.stopPropagation();

      const mint = trigger.dataset.mint;
      if (!mint) return;

      // Close any existing dropdown
      const existingMenu = document.querySelector(".links-dropdown-menu.open");
      if (existingMenu) {
        existingMenu.remove();
      }

      // Create dropdown menu
      const menu = document.createElement("div");
      menu.className = "links-dropdown-menu dropdown-menu open";
      menu.setAttribute("data-align", "left");
      menu.innerHTML = `
        <button class="dropdown-item" data-action="dexscreener" type="button">
          <span class="icon">📊</span>
          <span class="label">DexScreener</span>
        </button>
        <button class="dropdown-item" data-action="gmgn" type="button">
          <span class="icon">📈</span>
          <span class="label">GMGN</span>
        </button>
        <button class="dropdown-item" data-action="solscan" type="button">
          <span class="icon">🔍</span>
          <span class="label">Solscan</span>
        </button>
        <button class="dropdown-item" data-action="birdeye" type="button">
          <span class="icon">🦅</span>
          <span class="label">Birdeye</span>
        </button>
        <button class="dropdown-item" data-action="rugcheck" type="button">
          <span class="icon">🛡️</span>
          <span class="label">RugCheck</span>
        </button>
        <button class="dropdown-item" data-action="pumpfun" type="button">
          <span class="icon">🚀</span>
          <span class="label">Pump.fun</span>
        </button>
        <div class="dropdown-divider"></div>
        <button class="dropdown-item" data-action="copy" type="button">
          <span class="icon">📋</span>
          <span class="label">Copy Mint</span>
        </button>
      `;

      // Position menu relative to trigger
      const rect = trigger.getBoundingClientRect();
      const containerRect = container.getBoundingClientRect();
      menu.style.position = "absolute";
      menu.style.top = `${rect.bottom - containerRect.top + container.scrollTop + 6}px`;
      menu.style.left = `${rect.left - containerRect.left + container.scrollLeft}px`;
      menu.style.zIndex = "9999";

      container.appendChild(menu);

      // Handle menu item clicks
      menu.addEventListener("click", (e) => {
        const item = e.target.closest(".dropdown-item");
        if (!item) return;

        const action = item.dataset.action;
        if (action) {
          handleLinkAction(action, mint);
        }
        menu.remove();
      });

      // Close on outside click
      const closeHandler = (e) => {
        if (!menu.contains(e.target) && e.target !== trigger) {
          menu.remove();
          document.removeEventListener("click", closeHandler);
        }
      };
      setTimeout(() => document.addEventListener("click", closeHandler), 0);

      // Close on escape
      const escapeHandler = (e) => {
        if (e.key === "Escape") {
          menu.remove();
          document.removeEventListener("keydown", escapeHandler);
        }
      };
      document.addEventListener("keydown", escapeHandler);
    };

    container._linksClickHandler = clickHandler;
    container.addEventListener("click", clickHandler);
  };

  const initializeImageLightbox = () => {
    // Use event delegation for logo clicks
    if (!table?.elements?.scrollContainer) return;

    const container = table.elements.scrollContainer;
    if (container._logoClickHandler) {
      container.removeEventListener("click", container._logoClickHandler);
    }

    const clickHandler = (e) => {
      const logo = e.target.closest(".clickable-logo");
      if (!logo) return;

      e.preventDefault();
      e.stopPropagation();

      const imageUrl = logo.dataset.logoUrl;
      const symbol = logo.dataset.tokenSymbol || "";
      const name = logo.dataset.tokenName || "";
      const mint = logo.dataset.tokenMint || "";

      if (!imageUrl) return;

      // Look up age from current table data instead of stale data attribute
      let ageText = "Unknown";
      if (mint && table) {
        const tableData = table.getData();
        const rowData = tableData.find((row) => row.mint === mint);
        if (rowData) {
          const ageSeconds = rowData.token_birth_at || rowData.first_seen_at;
          if (ageSeconds !== null && ageSeconds !== undefined) {
            ageText = Utils.formatTimeAgo(ageSeconds, { fallback: "Unknown" });
          }
        }
      }

      // Create lightbox overlay
      const lightbox = document.createElement("div");
      lightbox.className = "image-lightbox";

      const symbolHtml = symbol
        ? `<div class="lightbox-token-symbol">${Utils.escapeHtml(symbol)}</div>`
        : "";
      const nameHtml = name
        ? `<div class="lightbox-token-name">${Utils.escapeHtml(name)}</div>`
        : "";

      lightbox.innerHTML = `
        <div class="lightbox-backdrop"></div>
        <div class="lightbox-container">
          <div class="lightbox-header">
            ${symbolHtml}
            ${nameHtml}
            <button class="lightbox-close" type="button" title="Close (ESC)">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
              </svg>
            </button>
          </div>
          <div class="lightbox-body">
            <div class="lightbox-image-wrapper">
              <img src="${Utils.escapeHtml(imageUrl)}" alt="Token logo" class="lightbox-image" />
            </div>
          </div>
          <div class="lightbox-footer">
            <div class="lightbox-stat">
              <div class="stat-label">Token Age</div>
              <div class="stat-value">${Utils.escapeHtml(ageText)}</div>
            </div>
          </div>
        </div>
      `;

      document.body.appendChild(lightbox);

      // Animate in
      requestAnimationFrame(() => {
        lightbox.classList.add("active");
      });

      // Close handlers
      const close = () => {
        lightbox.classList.remove("active");
        setTimeout(() => lightbox.remove(), 300);
      };

      lightbox.querySelector(".lightbox-close").addEventListener("click", close);
      lightbox.querySelector(".lightbox-backdrop").addEventListener("click", close);

      const escapeHandler = (e) => {
        if (e.key === "Escape") {
          close();
          document.removeEventListener("keydown", escapeHandler);
        }
      };
      document.addEventListener("keydown", escapeHandler);
    };

    container._logoClickHandler = clickHandler;
    container.addEventListener("click", clickHandler);
  };

  const initializeRowClickHandler = () => {
    // Use event delegation for row clicks
    if (!table?.elements?.scrollContainer) return;

    const container = table.elements.scrollContainer;
    if (container._rowClickHandler) {
      container.removeEventListener("click", container._rowClickHandler);
    }

    const clickHandler = (e) => {
      // Ignore clicks on logos, buttons, and interactive elements
      if (
        e.target.closest(".clickable-logo") ||
        e.target.closest(".row-action") ||
        e.target.closest(".links-dropdown-trigger") ||
        e.target.closest("button") ||
        e.target.closest("a")
      ) {
        return;
      }

      // Find the row element
      const row = e.target.closest("tr");
      if (!row) return;

      // Get the mint from row dataset or find it in the cells
      const mintCell = row.querySelector("[data-token-mint]");
      if (!mintCell) return;

      const mint = mintCell.dataset.tokenMint;
      if (!mint) return;

      // Find token data in table
      const tableData = table.getData();
      const tokenData = tableData.find((token) => token.mint === mint);
      if (!tokenData) return;

      // Show token details dialog
      if (!tokenDetailsDialog) {
        tokenDetailsDialog = new TokenDetailsDialog();
      }
      tokenDetailsDialog.show(tokenData);
    };

    container._rowClickHandler = clickHandler;
    container.addEventListener("click", clickHandler);
  };

  return {
    init(ctx) {
      // Initialize trade dialog
      tradeDialog = new TradeActionDialog();

      // Initialize token details dialog
      tokenDetailsDialog = new TokenDetailsDialog();

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

      state.filters = getDefaultFiltersForView(state.view);

      const initialSortColumn = resolveSortColumn(state.sort.by);

      // Build columns based on current view
      const columns = buildColumns();

      table = new DataTable({
        container: "#tokens-root",
        columns,
        rowIdField: "mint",
        stateKey: `tokens-table-v2.${state.view}`, // v2: fixed column order (Actions before Links)
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
          onPageLoaded: () => {
            updateToolbar();
            initializeLinkDropdowns();
            initializeImageLightbox();
            initializeRowClickHandler();
          },
        },
        toolbar: {
          title: {
            icon: "🪙",
            text: "Tokens",
            meta: [
              {
                id: "tokens-last-update",
                lines: ["Last Update", "—"],
              },
            ],
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
              // onChange just updates state, onSubmit triggers reload
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
              id: "rejection_reason",
              label: "Reject Reason",
              mode: "server",
              autoApply: true,
              defaultValue: DEFAULT_FILTERS.rejection_reason,
              options: [{ value: "all", label: "All" }],
              onChange: (value, _el, options) => {
                state.filters.rejection_reason = value || "all";
                if (options?.restored) {
                  return;
                }
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
        },
      });

      // Hide rejection_reason filter immediately if not in rejected tab
      updateFilterVisibility();

      // Row actions: delegate clicks on the table container
      const containerEl = document.querySelector("#tokens-root");
      const handleRowActionClick = async (e) => {
        const btn = e.target?.closest?.(".row-action");
        if (!btn) return;
        const action = btn.getAttribute("data-action");
        const mint = btn.getAttribute("data-mint");
        if (!action || !mint) return;

        // Find row data
        const row = table.getData().find((r) => r.mint === mint);
        if (!row) {
          Utils.showToast("Token data not found", "error");
          return;
        }

        try {
          if (action === "buy") {
            // Open dialog
            const result = await tradeDialog.open({
              action: "buy",
              mint: mint,
              symbol: row.symbol || "?",
              context: {
                balance: walletBalance,
              },
            });

            if (!result) return; // User cancelled

            // Proceed with API call
            btn.disabled = true;
            const res = await fetch("/api/trader/manual/buy", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                mint,
                ...(result.amount ? { size_sol: result.amount } : {}),
              }),
            });
            const data = await res.json();
            btn.disabled = false;
            if (!res.ok) throw new Error(data?.error?.message || "Buy failed");
            Utils.showToast("✅ Buy placed", "success");
            requestReload("manual", { silent: false, preserveScroll: true }).catch(() => {});
          } else if (action === "add") {
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
                entrySize: row.entry_sol || 0.005,
                entrySizes: entrySizes,
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
            Utils.showToast("✅ Added to position", "success");
            requestReload("manual", { silent: false, preserveScroll: true }).catch(() => {});
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
            Utils.showToast("✅ Sell placed", "success");
            requestReload("manual", { silent: false, preserveScroll: true }).catch(() => {});
          }
        } catch (err) {
          btn.disabled = false;
          Utils.showToast(err?.message || "Action failed", "error");
        }
      };

      if (containerEl) {
        containerEl.addEventListener("click", handleRowActionClick);
      }
      applyViewPreferences();

      // Sync state from DataTable's restored server state
      const restoredState = table.getServerState();
      const serverState = restoredState || { filters: {} };
      let hasSortRestored = false;

      if (serverState.sortColumn) {
        const sortKey = resolveSortKey(serverState.sortColumn);
        if (sortKey) {
          state.sort = {
            by: sortKey,
            direction: serverState.sortDirection || "asc",
          };
          hasSortRestored = true;
        }
      }
      if (serverState.searchQuery) {
        state.search = serverState.searchQuery;
      }

      const serverFilters = serverState.filters || {};
      if (Object.prototype.hasOwnProperty.call(serverFilters, "pool_price")) {
        state.filters.pool_price = parseToggleValue(serverFilters.pool_price, false);
      } else if (Object.prototype.hasOwnProperty.call(serverFilters, "priced")) {
        const legacy = serverFilters.priced;
        state.filters.pool_price = legacy === "priced" || parseToggleValue(legacy, false);
      }

      if (Object.prototype.hasOwnProperty.call(serverFilters, "positions")) {
        const value = serverFilters.positions;
        if (typeof value === "string") {
          state.filters.positions = value === "open";
        } else {
          state.filters.positions = parseToggleValue(value, false);
        }
      }

      if (Object.prototype.hasOwnProperty.call(serverFilters, "rejection_reason")) {
        const reason = serverFilters.rejection_reason;
        state.filters.rejection_reason =
          typeof reason === "string" && reason.trim().length > 0 ? reason : "all";
      }

      if (state.view === "positions") {
        state.filters.positions = true;
      }
      if (state.view === "no_market") {
        state.filters.pool_price = false;
      }
      if (state.view !== "rejected") {
        state.filters.rejection_reason = "all";
      }

      syncTableSortState({ render: false });
      syncToolbarFilters();
      table.setToolbarSearchValue(state.search, { apply: false });
      updateToolbar();

      // Trigger initial data load if no sort state was restored
      // (sort restoration triggers reload via handleSortChange)
      if (!hasSortRestored) {
        requestReload("initial", { silent: false, resetScroll: true }).catch(() => {});
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
          console.warn("[Tokens] Failed to fetch wallet balance");
        });

      // Start interval to update "Last Update" display every second
      if (!lastUpdateInterval) {
        lastUpdateInterval = setInterval(() => {
          // Only update if we have a lastUpdate timestamp
          if (state.lastUpdate && table) {
            updateToolbar();
          }
        }, 1000);
      }

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
      // Stop the interval when page is deactivated
      if (lastUpdateInterval) {
        clearInterval(lastUpdateInterval);
        lastUpdateInterval = null;
      }
    },

    dispose() {
      // Clean up interval
      if (lastUpdateInterval) {
        clearInterval(lastUpdateInterval);
        lastUpdateInterval = null;
      }
      // Clean up any open dropdown menus
      const existingMenu = document.querySelector(".links-dropdown-menu");
      if (existingMenu) {
        existingMenu.remove();
      }
      // Clean up any open image lightbox
      const existingLightbox = document.querySelector(".image-lightbox");
      if (existingLightbox) {
        existingLightbox.remove();
      }
      if (tradeDialog) {
        tradeDialog.destroy();
        tradeDialog = null;
      }
      if (tokenDetailsDialog) {
        tokenDetailsDialog.destroy();
        tokenDetailsDialog = null;
      }
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
      state.filters = getDefaultFiltersForView(DEFAULT_VIEW);
      state.summary = { ...DEFAULT_SUMMARY };
      walletBalance = 0;
    },
  };
}

registerPage("tokens", createLifecycle());
