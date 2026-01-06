import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { requestManager } from "../core/request_manager.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";
import { DataTable } from "../ui/data_table.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import { TradeActionDialog } from "../ui/trade_action_dialog.js";
import { TokenDetailsDialog } from "../ui/token_details_dialog.js";
import * as Hints from "../core/hints.js";
import { HintTrigger } from "../ui/hint_popover.js";
import { showBillboardRow, hideBillboardRow } from "../ui/billboard_row.js";

// Sub-tabs (views) configuration with hint references
const TOKEN_VIEWS = [
  {
    id: "favorites",
    label: '<i class="icon-star"></i> Favorites',
    hintKey: "tokens.favorites",
  },
  { id: "pool", label: '<i class="icon-droplet"></i> Pool Service', hintKey: "tokens.poolService" },
  {
    id: "no_market",
    label: '<i class="icon-trending-down"></i> No Market Data',
    hintKey: "tokens.noMarketData",
  },
  { id: "all", label: '<i class="icon-list"></i> All Tokens', hintKey: "tokens.allTokens" },
  { id: "passed", label: '<i class="icon-check"></i> Passed', hintKey: "tokens.passedTokens" },
  {
    id: "rejected",
    label: '<i class="icon-circle-x"></i> Rejected',
    hintKey: "tokens.rejectedTokens",
  },
  {
    id: "blacklisted",
    label: '<i class="icon-ban"></i> Blacklisted',
    hintKey: "tokens.blacklistedTokens",
  },
  {
    id: "positions",
    label: '<i class="icon-chart-bar"></i> Positions',
    hintKey: "tokens.positionsTokens",
  },
  { id: "recent", label: '<i class="icon-clock"></i> Recent', hintKey: "tokens.recentTokens" },
  {
    id: "ohlcv",
    label: '<i class="icon-chart-candlestick"></i> OHLCV Data',
    hintKey: "tokens.ohlcvData",
  },
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
  updated_at: "market_data_last_fetched_at",
  first_seen_at: "first_discovered_at",
  token_birth_at: "blockchain_created_at",
};

// View-aware sort key resolver for "updated_at" column
const getServerSortKey = (columnId, currentView) => {
  if (columnId === "updated_at") {
    // Different views show different timestamps in "Updated" column
    if (currentView === "pool" || currentView === "positions") {
      return "pool_price_last_calculated_at";
    } else if (currentView === "no_market") {
      return "metadata_last_fetched_at";
    } else {
      return "market_data_last_fetched_at";
    }
  }

  // Use mapping for other columns
  return COLUMN_TO_SORT_KEY[columnId] || columnId;
};

const SORT_KEY_TO_COLUMN = Object.entries(COLUMN_TO_SORT_KEY).reduce((acc, [columnId, sortKey]) => {
  acc[sortKey] = columnId;
  return acc;
}, {});

const getTokensTableStateKey = (view) => `tokens-table.${view}`;

const PAGE_LIMIT = 100; // chunked fetch size for incremental scrolling
const PRICE_HIGHLIGHT_DURATION_MS = 10_000;

function findFirstDifferenceIndex(a, b) {
  if (typeof a !== "string" || typeof b !== "string") {
    return -1;
  }

  const minLength = Math.min(a.length, b.length);
  for (let i = 0; i < minLength; i += 1) {
    if (a[i] !== b[i]) {
      return i;
    }
  }

  if (a.length !== b.length) {
    return minLength;
  }

  return -1;
}

function priceCell(value, row = null) {
  const formatted = Utils.formatPriceSol(value, { fallback: "—", decimals: 12 });
  const baseValue = Utils.escapeHtml(formatted);

  let directionClass = "price-change--neutral";
  let arrowSymbol = "▲";
  let arrowClass = "price-change-arrow price-change-arrow--placeholder";
  let valueHtml = baseValue;

  if (row && row.price_change_meta) {
    const { direction, currentFormatted, changeStartIndex } = row.price_change_meta;
    if (
      direction &&
      typeof changeStartIndex === "number" &&
      currentFormatted &&
      formatted !== "—"
    ) {
      const boundedIndex = Math.max(0, Math.min(changeStartIndex, currentFormatted.length));
      const leadingPart = Utils.escapeHtml(currentFormatted.slice(0, boundedIndex));
      const changedPart = Utils.escapeHtml(currentFormatted.slice(boundedIndex));
      valueHtml = `${leadingPart}<span class="price-change-diff">${changedPart}</span>`;
      directionClass = direction === "up" ? "price-change--up" : "price-change--down";
      arrowSymbol = direction === "up" ? "▲" : "▼";
      arrowClass = "price-change-arrow";
    }
  }

  return `<span class="price-change ${directionClass}"><span class="${arrowClass}" aria-hidden="true">${arrowSymbol}</span><span class="price-change-value">${valueHtml}</span></span>`;
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

// Rejection reason label mapping (machine code -> human-readable)
const REJECTION_LABELS = {
  no_decimals: "No decimals in database",
  token_too_new: "Token too new",
  cooldown_filtered: "Cooldown filtered",
  dex_data_missing: "DexScreener data missing",
  gecko_data_missing: "GeckoTerminal data missing",
  rug_data_missing: "Rugcheck data missing",
  dex_empty_name: "Empty name",
  dex_empty_symbol: "Empty symbol",
  dex_empty_logo: "Empty logo URL",
  dex_empty_website: "Empty website URL",
  dex_txn_5m: "Low 5m transactions",
  dex_txn_1h: "Low 1h transactions",
  dex_zero_liq: "Zero liquidity",
  dex_liq_low: "Liquidity too low",
  dex_liq_high: "Liquidity too high",
  dex_mcap_low: "Market cap too low",
  dex_mcap_high: "Market cap too high",
  dex_vol_low: "Volume too low",
  dex_vol_missing: "Volume missing",
  dex_fdv_low: "FDV too low",
  dex_fdv_high: "FDV too high",
  dex_fdv_missing: "FDV missing",
  dex_vol5m_low: "5m volume too low",
  dex_vol5m_missing: "5m volume missing",
  dex_vol1h_low: "1h volume too low",
  dex_vol1h_missing: "1h volume missing",
  dex_vol6h_low: "6h volume too low",
  dex_vol6h_missing: "6h volume missing",
  dex_price_change_5m_low: "5m price change too low",
  dex_price_change_5m_high: "5m price change too high",
  dex_price_change_5m_missing: "5m price change missing",
  dex_price_change_low: "Price change too low",
  dex_price_change_high: "Price change too high",
  dex_price_change_missing: "Price change missing",
  dex_price_change_6h_low: "6h price change too low",
  dex_price_change_6h_high: "6h price change too high",
  dex_price_change_6h_missing: "6h price change missing",
  dex_price_change_24h_low: "24h price change too low",
  dex_price_change_24h_high: "24h price change too high",
  dex_price_change_24h_missing: "24h price change missing",
  gecko_liq_missing: "Liquidity missing",
  gecko_liq_low: "Liquidity too low",
  gecko_liq_high: "Liquidity too high",
  gecko_mcap_missing: "Market cap missing",
  gecko_mcap_low: "Market cap too low",
  gecko_mcap_high: "Market cap too high",
  gecko_vol5m_low: "5m volume too low",
  gecko_vol5m_missing: "5m volume missing",
  gecko_vol1h_low: "1h volume too low",
  gecko_vol1h_missing: "1h volume missing",
  gecko_vol24h_low: "24h volume too low",
  gecko_vol24h_missing: "24h volume missing",
  gecko_price_change_5m_low: "5m price change too low",
  gecko_price_change_5m_high: "5m price change too high",
  gecko_price_change_5m_missing: "5m price change missing",
  gecko_price_change_1h_low: "1h price change too low",
  gecko_price_change_1h_high: "1h price change too high",
  gecko_price_change_1h_missing: "1h price change missing",
  gecko_price_change_24h_low: "24h price change too low",
  gecko_price_change_24h_high: "24h price change too high",
  gecko_price_change_24h_missing: "24h price change missing",
  gecko_pool_count_low: "Pool count too low",
  gecko_pool_count_high: "Pool count too high",
  gecko_pool_count_missing: "Pool count missing",
  gecko_reserve_low: "Reserve too low",
  gecko_reserve_missing: "Reserve missing",
  rug_rugged: "Rugged token",
  rug_score: "Risk score too high",
  rug_level_danger: "Danger risk level",
  rug_mint_authority: "Mint authority present",
  rug_freeze_authority: "Freeze authority present",
  rug_top_holder: "Top holder % too high",
  rug_top3_holders: "Top 3 holders % too high",
  rug_min_holders: "Not enough holders",
  rug_insider_count: "Too many insider holders",
  rug_insider_pct: "Insider % too high",
  rug_creator_pct: "Creator balance too high",
  rug_transfer_fee_present: "Transfer fee present",
  rug_transfer_fee_high: "Transfer fee too high",
  rug_transfer_fee_missing: "Transfer fee data missing",
  rug_graph_insiders: "Graph insiders too high",
  rug_lp_providers_low: "LP providers too low",
  rug_lp_providers_missing: "LP providers missing",
  rug_lp_lock_low: "LP lock too low",
  rug_lp_lock_missing: "LP lock missing",
};

function getRejectionDisplayLabel(reasonCode) {
  if (!reasonCode) return null;
  return REJECTION_LABELS[reasonCode] || reasonCode;
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
      const category =
        typeof entry.category === "string" && entry.category.trim().length > 0
          ? entry.category.trim()
          : "unknown";
      const reason =
        typeof entry.reason === "string" && entry.reason.trim().length > 0
          ? entry.reason.trim()
          : "unknown_reason";
      const detail =
        typeof entry.detail === "string" && entry.detail.trim().length > 0
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
  // Dynamic keys mapping back to 'updated_at'
  if (
    [
      "pool_price_last_calculated_at",
      "metadata_last_fetched_at",
      "market_data_last_fetched_at",
    ].includes(sortKey)
  ) {
    return "updated_at";
  }
  return SORT_KEY_TO_COLUMN[sortKey] ?? null;
}

function normalizeSortDirection(direction) {
  return direction === "desc" ? "desc" : "asc";
}

function loadPersistedSort(stateKey) {
  if (!stateKey) return null;
  const saved = AppState.load(stateKey);
  if (saved && typeof saved === "object" && saved.sortColumn) {
    return {
      column: saved.sortColumn,
      direction: normalizeSortDirection(saved.sortDirection),
    };
  }
  return null;
}

/**
 * Attach hint triggers to tab buttons
 * Must be called after tab bar is rendered
 */
async function attachHintsToTabs() {
  // Initialize hints system (loads settings and dismissed state)
  await Hints.init();

  if (!Hints.isEnabled()) return;

  const container = document.querySelector("#subTabsContainer");
  if (!container) return;

  // Find all tab buttons and attach hints
  TOKEN_VIEWS.forEach((view) => {
    if (!view.hintKey) return;

    const hint = Hints.getHint(view.hintKey);
    if (!hint || Hints.isDismissed(hint.id)) return;

    const tabButton = container.querySelector(`[data-tab-id="${view.id}"]`);
    if (!tabButton) return;

    // Check if hint already attached
    if (tabButton.querySelector(".hint-trigger")) return;

    // Append hint trigger to tab button (pass hintKey as path)
    const triggerHtml = HintTrigger.render(hint, view.hintKey, { size: "sm" });
    if (triggerHtml) {
      tabButton.insertAdjacentHTML("beforeend", triggerHtml);
    }
  });

  // Initialize hint trigger handlers
  HintTrigger.initAll();
}

function createLifecycle() {
  let table = null;
  let ohlcvTable = null; // Separate table for OHLCV data view
  let poller = null;
  let ohlcvPoller = null; // Separate poller for OHLCV data
  let tabBar = null;
  let tradeDialog = null;
  let tokenDetailsDialog = null;
  let walletBalance = 0;
  let lastUpdatePoller = null; // Poller for updating "Last Update" display

  // Event cleanup tracking
  const eventCleanups = [];

  const priceHistory = new Map();
  let priceBaselineReady = false;

  // OHLCV state
  const ohlcvState = {
    tokens: [],
    stats: null,
    isLoading: false,
  };

  // Favorites state
  let favoritesTable = null;
  const favoritesState = {
    favorites: [],
    isLoading: false,
  };

  const state = {
    view: DEFAULT_VIEW,
    search: "",
    totalCount: null,
    lastUpdate: null,
    sort: { ...DEFAULT_SERVER_SORT },
    filters: getDefaultFiltersForView(DEFAULT_VIEW),
    summary: { ...DEFAULT_SUMMARY },
    availableRejectionReasons: [],
    hasLoadedOnce: false,
  };

  /**
   * Add tracked event listener for cleanup
   */
  function addTrackedListener(element, event, handler) {
    if (!element) return;
    element.addEventListener(event, handler);
    eventCleanups.push(() => element.removeEventListener(event, handler));
  }

  const resetPriceTracking = () => {
    priceHistory.clear();
    priceBaselineReady = false;
    if (table?.getData) {
      const currentRows = table.getData();
      if (Array.isArray(currentRows)) {
        currentRows.forEach((row) => {
          if (
            row &&
            typeof row === "object" &&
            Object.prototype.hasOwnProperty.call(row, "price_change_meta")
          ) {
            row.price_change_meta = null;
          }
        });
      }
    }
  };

  const annotatePriceChange = (row) => {
    if (!row || typeof row !== "object" || state.view !== "pool") {
      return row;
    }

    const mint = row.mint;
    if (!mint) {
      row.price_change_meta = null;
      return row;
    }

    const numericPrice = Number(row.price_sol);
    if (!Number.isFinite(numericPrice)) {
      priceHistory.delete(mint);
      row.price_change_meta = null;
      return row;
    }

    const formattedCurrent = Utils.formatPriceSol(numericPrice, { fallback: "", decimals: 12 });
    const now = Date.now();
    const record = priceHistory.get(mint);

    if (
      priceBaselineReady &&
      record &&
      Number.isFinite(record.lastPrice) &&
      numericPrice !== record.lastPrice
    ) {
      const direction = numericPrice > record.lastPrice ? "up" : "down";
      const previousFormatted = record.lastFormatted ?? "";
      const changeStartIndex = findFirstDifferenceIndex(previousFormatted, formattedCurrent);
      if (direction && changeStartIndex !== -1) {
        const meta = {
          direction,
          currentFormatted: formattedCurrent,
          changeStartIndex,
        };
        priceHistory.set(mint, {
          lastPrice: numericPrice,
          lastFormatted: formattedCurrent,
          highlightUntil: now + PRICE_HIGHLIGHT_DURATION_MS,
          meta,
        });
        row.price_change_meta = meta;
        return row;
      }
    }

    if (record && record.highlightUntil > now && record.meta) {
      const refreshedMeta = {
        ...record.meta,
        currentFormatted: formattedCurrent,
      };
      priceHistory.set(mint, {
        lastPrice: numericPrice,
        lastFormatted: formattedCurrent,
        highlightUntil: record.highlightUntil,
        meta: refreshedMeta,
      });
      row.price_change_meta = refreshedMeta;
      return row;
    }

    priceHistory.set(mint, {
      lastPrice: numericPrice,
      lastFormatted: formattedCurrent,
      highlightUntil: 0,
      meta: null,
    });
    row.price_change_meta = null;
    return row;
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

    // Sort by display label for user-friendly ordering
    reasons.sort((a, b) => {
      const labelA = getRejectionDisplayLabel(a) || a;
      const labelB = getRejectionDisplayLabel(b) || b;
      return labelA.toLowerCase().localeCompare(labelB.toLowerCase());
    });

    const currentValue = state.filters.rejection_reason || "all";

    // Build options array for CustomSelect with human-readable labels
    const newOptions = [
      { value: "all", label: "All" },
      ...reasons.map((reason) => ({
        value: reason,
        label: getRejectionDisplayLabel(reason) || reason,
      })),
    ];

    // Check if the select has a CustomSelect instance attached
    if (select._customSelectInstance && typeof select._customSelectInstance.setOptions === "function") {
      select._customSelectInstance.setOptions(newOptions);
      select._customSelectInstance.setValue(currentValue);
    } else {
      // Fallback: update native select options
      const optionMarkup = [
        '<option value="all">All</option>',
        ...reasons.map((reason) => {
          const escaped = Utils.escapeHtml(reason);
          const label = Utils.escapeHtml(getRejectionDisplayLabel(reason) || reason);
          return `<option value="${escaped}">${label}</option>`;
        }),
      ].join("");

      if (select.innerHTML !== optionMarkup) {
        select.innerHTML = optionMarkup;
      }
      select.value = currentValue;
    }

    const normalizedCurrent = reasons.some((reason) => reason === currentValue)
      ? currentValue
      : "all";

    if (normalizedCurrent !== currentValue) {
      state.filters.rejection_reason = normalizedCurrent;
    }
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

  const buildQuery = ({ cursor = null, page = null, pageSize = null } = {}) => {
    const params = new URLSearchParams();
    params.set("view", state.view);
    if (state.search) params.set("search", state.search);
    const sort = state.sort ?? DEFAULT_SERVER_SORT;
    const sortBy = sort?.by ?? DEFAULT_SERVER_SORT.by;
    const sortDir = sort?.direction ?? DEFAULT_SERVER_SORT.direction;
    const sortColumnId = resolveSortColumn(sortBy) ?? sortBy;
    // Use view-aware sort key resolver with column identifier
    const serverSortKey = getServerSortKey(sortColumnId, state.view);
    params.set("sort_by", serverSortKey);
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
    // Page-based pagination (for 'pages' mode)
    if (page !== null && page !== undefined) {
      params.set("page", String(page));
      params.set("limit", String(pageSize || PAGE_LIMIT));
    } else {
      // Cursor-based pagination (for 'scroll' mode)
      params.set("limit", String(PAGE_LIMIT));
      if (cursor !== null && cursor !== undefined) {
        params.set("cursor", String(cursor));
      }
    }
    return params;
  };

  // Prevent poll reloads from racing with user actions (sort/page/search/etc.)
  // Poll reload uses DataTable.reload(), which cancels in-flight pagination requests.
  // If poll fires during a user-triggered load, it can repeatedly abort and re-queue
  // requests, making even small page sizes feel "stuck".
  let lastUserReloadAt = 0;

  const shouldSkipPollReload = () => {
    if (!table) return false;

    // Only the Pool Service view needs real-time table refreshes.
    // Other views are either static or user-driven and auto-reloading every poll interval
    // can spam /api/tokens/list and interfere with sorting/pagination responsiveness.
    if (state.view !== "pool") {
      return true;
    }

    // In server-backed "Pages" mode we should not auto-reload the table.
    // It triggers a full page fetch every interval (default 1s) which can
    // overwhelm the UI and interfere with user-driven sorts/page navigation.
    if (typeof table.getServerPaginationMode === "function" && table.getServerPaginationMode() === "pages") {
      return true;
    }

    // If the table is already loading, never start a poll reload.
    // This avoids poll->abort->reload loops that delay UI updates.
    if (table.state?.isLoading) {
      return true;
    }

    // Skip polls shortly after a user-triggered reload (sort/search/page size/view).
    // Keeps the UI responsive by letting the latest user action complete.
    if (lastUserReloadAt && Date.now() - lastUserReloadAt < 2500) {
      return true;
    }

    const paginationState =
      typeof table.getPaginationState === "function" ? table.getPaginationState() : null;
    if (paginationState?.loadingNext || paginationState?.loadingPrev || paginationState?.loadingInitial) {
      return true;
    }

    // Skip if links dropdown menu is open
    const linksDropdown = document.querySelector(".links-dropdown-menu");
    if (linksDropdown) {
      return true;
    }

    // Skip if page size select is focused or dropdown is open
    const container = table?.elements?.container;
    if (container) {
      const pageSizeSelect = container.querySelector("[data-pagination-size]");
      if (pageSizeSelect && document.activeElement === pageSizeSelect) {
        return true;
      }
      
      // Skip if any row is currently being hovered (prevents hover state loss)
      const hoveredRow = container.querySelector("tr.dt-row:hover");
      if (hoveredRow) {
        return true;
      }
      
      // Skip if any input, select, or button in table is focused
      const focusedElement = document.activeElement;
      if (focusedElement && container.contains(focusedElement)) {
        const tagName = focusedElement.tagName?.toLowerCase();
        if (tagName === "input" || tagName === "select" || tagName === "button") {
          return true;
        }
      }
    }

    const scrollContainer = table?.elements?.scrollContainer;
    if (!scrollContainer) {
      return false;
    }

    const hasScrollableContent = scrollContainer.scrollHeight > scrollContainer.clientHeight + 16;
    if (!hasScrollableContent) {
      return false;
    }

    const nearTop = scrollContainer.scrollTop <= 120;
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
    const sortKey = getServerSortKey(column, state.view);
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

  const loadTokensPage = async ({
    direction = "initial",
    cursor,
    page,
    pageSize,
    reason,
    signal,
  }) => {
    // Handle page-based pagination (direction === 'page')
    if (direction === "page" && page !== undefined) {
      return loadTokensPageBased({ page, pageSize, reason, signal });
    }

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

    if (!state.hasLoadedOnce && reason !== "poll" && table?.showBlockingState) {
      table.showBlockingState({
        variant: "loading",
        title: "Loading tokens snapshot...",
        description: "Large token databases can take a few seconds to warm up during startup.",
      });
    }

    const params = buildQuery({ cursor });
    const url = `/api/tokens/list?${params.toString()}`;

    try {
      const requestPriority = reason === "poll" ? "normal" : "high";
      const data = await requestManager.fetch(url, {
        priority: requestPriority,
        signal,
        skipQueue: reason !== "poll",
      });
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

      let rowsWithPriceMeta = normalizedItems;
      if (state.view === "pool") {
        rowsWithPriceMeta = normalizedItems.map((row) => annotatePriceChange(row));
        if (!priceBaselineReady) {
          priceBaselineReady = true;
        }
      } else if (priceHistory.size > 0) {
        resetPriceTracking();
      }

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
            : rowsWithPriceMeta.filter((row) => row.has_pool_price).length,
        positions:
          positionsTotal !== null
            ? positionsTotal
            : rowsWithPriceMeta.filter((row) => row.has_open_position).length,
        blacklisted:
          blacklistedTotal !== null
            ? blacklistedTotal
            : rowsWithPriceMeta.filter((row) => row.blacklisted).length,
      };

      state.hasLoadedOnce = true;
      if (table?.hideBlockingState) {
        table.hideBlockingState();
      }

      return {
        rows: rowsWithPriceMeta,
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
      if (!state.hasLoadedOnce) {
        table?.showBlockingState?.({
          variant: "error",
          title: "Still loading tokens...",
          description: "Waiting for the backend to respond. We will retry automatically.",
        });
      } else if (reason !== "poll") {
        Utils.showToast("Failed to load tokens", "warning");
      }
      throw error;
    }
  };

  /**
   * Load tokens using page-based pagination (for 'pages' mode).
   * Returns data in a format DataTable expects for server page navigation.
   */
  const loadTokensPageBased = async ({ page, pageSize, reason, signal }) => {
    if (!state.hasLoadedOnce && reason !== "poll" && table?.showBlockingState) {
      table.showBlockingState({
        variant: "loading",
        title: "Loading tokens page...",
        description: "Fetching page data from server.",
      });
    }

    const params = buildQuery({ page, pageSize });
    const url = `/api/tokens/list?${params.toString()}`;

    try {
      const requestPriority = reason === "poll" ? "normal" : "high";
      const data = await requestManager.fetch(url, {
        priority: requestPriority,
        signal,
      });
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

      let rowsWithPriceMeta = normalizedItems;
      if (state.view === "pool") {
        rowsWithPriceMeta = normalizedItems.map((row) => annotatePriceChange(row));
        if (!priceBaselineReady) {
          priceBaselineReady = true;
        }
      }

      // Update available rejection reasons
      if (Array.isArray(data?.available_rejection_reasons)) {
        state.availableRejectionReasons = data.available_rejection_reasons.filter(
          (r) => typeof r === "string" && r.trim().length > 0
        );
      }

      if (state.view === "rejected") {
        updateRejectionReasonOptions();
        if (table) {
          table.setToolbarFilterValue("rejection_reason", state.filters.rejection_reason, {
            apply: false,
          });
        }
      }

      // Update totals
      if (typeof data?.total === "number" && Number.isFinite(data.total)) {
        state.totalCount = data.total;
      }

      state.lastUpdate = data?.timestamp ?? null;
      const pricedTotal = typeof data?.priced_total === "number" ? data.priced_total : null;
      const positionsTotal =
        typeof data?.positions_total === "number" ? data.positions_total : null;
      const blacklistedTotal =
        typeof data?.blacklisted_total === "number" ? data.blacklisted_total : null;

      state.summary = {
        priced:
          pricedTotal !== null
            ? pricedTotal
            : rowsWithPriceMeta.filter((row) => row.has_pool_price).length,
        positions:
          positionsTotal !== null
            ? positionsTotal
            : rowsWithPriceMeta.filter((row) => row.has_open_position).length,
        blacklisted:
          blacklistedTotal !== null
            ? blacklistedTotal
            : rowsWithPriceMeta.filter((row) => row.blacklisted).length,
      };

      state.hasLoadedOnce = true;
      if (table?.hideBlockingState) {
        table.hideBlockingState();
      }

      // Return page-based response with serverPage info
      return {
        rows: rowsWithPriceMeta,
        total: state.totalCount ?? items.length,
        meta: { timestamp: state.lastUpdate },
        // Server page info for DataTable
        serverPage: {
          page: data?.page ?? page,
          pageSize: data?.page_size ?? pageSize,
          totalPages: data?.total_pages ?? Math.ceil((state.totalCount || items.length) / pageSize),
        },
      };
    } catch (error) {
      if (error?.name === "AbortError") {
        throw error;
      }
      console.error("[Tokens] Failed to load tokens page:", error);
      if (!state.hasLoadedOnce) {
        table?.showBlockingState?.({
          variant: "error",
          title: "Still loading tokens...",
          description: "Waiting for the backend to respond. We will retry automatically.",
        });
      } else if (reason !== "poll") {
        Utils.showToast("Failed to load tokens page", "warning");
      }
      throw error;
    }
  };

  const requestReload = (reason = "manual", options = {}) => {
    if (!table) return Promise.resolve(null);
    if (reason === "poll" && shouldSkipPollReload()) {
      return Promise.resolve(null);
    }

    if (reason !== "poll") {
      lastUserReloadAt = Date.now();
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
                      )}" title="Add to position (DCA)"${disabledAttr}><i class="icon-circle-plus"></i> Add</button>
                      <button class="btn row-action" data-action="sell" data-mint="${Utils.escapeHtml(
                        mint
                      )}" title="Sell (full or % partial)"${disabledAttr}><i class="icon-trending-down"></i> Sell</button>
                    </div>
                  `;
                }

                return `
                  <div class="row-actions">
                    <button class="btn row-action" data-action="buy" data-mint="${Utils.escapeHtml(
                      mint
                    )}" title="Buy position"${disabledAttr}><i class="icon-shopping-cart"></i> Buy</button>
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
              <i class="icon-external-link"></i>
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
        render: (v, row) => priceCell(v, row),
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
                const displayLabel = getRejectionDisplayLabel(value);
                // Show display label with original code as tooltip
                return `<span title="${Utils.escapeHtml(String(value))}">${Utils.escapeHtml(displayLabel)}</span>`;
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
        render: (_v, row) => {
          // Select timestamp based on current view
          let timestamp;

          if (state.view === "pool" || state.view === "positions") {
            // Pool/Positions: show pool price calculation time
            timestamp = row.pool_price_last_calculated_at;
          } else if (state.view === "no_market") {
            // No Market: show metadata fetch time
            timestamp = row.metadata_last_fetched_at;
          } else {
            // All others: show market data fetch time
            timestamp = row.market_data_last_fetched_at;
          }

          if (!timestamp) {
            return "—";
          }

          // timestamp is DateTime<Utc> serialized, convert to seconds
          const timestampSeconds =
            typeof timestamp === "string"
              ? Math.floor(new Date(timestamp).getTime() / 1000)
              : timestamp;

          return timeAgoCell(timestampSeconds);
        },
      },
      {
        id: "token_birth_at",
        label: "Birth",
        sortable: true,
        minWidth: 110,
        wrap: false,
        render: (_v, row) => {
          // Blockchain creation time, fallback to bot discovery
          const timestamp = row.blockchain_created_at || row.first_discovered_at;
          if (!timestamp) return "—";

          const timestampSeconds =
            typeof timestamp === "string"
              ? Math.floor(new Date(timestamp).getTime() / 1000)
              : timestamp;

          return timeAgoCell(timestampSeconds);
        },
      },
      {
        id: "first_seen_at",
        label: "First Seen",
        sortable: true,
        minWidth: 110,
        wrap: false,
        render: (_v, row) => {
          const timestamp = row.first_discovered_at;
          if (!timestamp) return "—";

          const timestampSeconds =
            typeof timestamp === "string"
              ? Math.floor(new Date(timestamp).getTime() / 1000)
              : timestamp;

          return timeAgoCell(timestampSeconds);
        },
      },
    ];
  };

  // ═══════════════════════════════════════════════════════════════════════════
  // OHLCV TABLE FUNCTIONS
  // ═══════════════════════════════════════════════════════════════════════════

  const buildOhlcvColumns = () => {
    return [
      {
        id: "mint",
        label: "Token",
        sortable: true,
        minWidth: 120,
        maxWidth: 200,
        wrap: false,
        render: (value) => {
          const short = value ? `${value.slice(0, 6)}...${value.slice(-4)}` : "—";
          return `<span class="mint-cell" title="${Utils.escapeHtml(value)}">${short}</span>`;
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
      {
        id: "actions",
        label: "",
        sortable: false,
        minWidth: 80,
        maxWidth: 80,
        wrap: false,
        render: (_value, row) => {
          return `<button class="btn btn-small btn-danger ohlcv-delete-btn" data-mint="${Utils.escapeHtml(row.mint)}" title="Delete OHLCV data">
            <i class="icon-trash-2"></i>
          </button>`;
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
      Utils.showToast("Failed to load OHLCV data", "error");
    } finally {
      ohlcvState.isLoading = false;
    }
  };

  const updateOhlcvTable = () => {
    if (!ohlcvTable) return;
    ohlcvTable.setData(ohlcvState.tokens, { preserveScroll: true });
    updateOhlcvToolbar();
  };

  const updateOhlcvToolbar = () => {
    if (!ohlcvTable) return;
    const stats = ohlcvState.stats || {};

    ohlcvTable.updateToolbarSummary([
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
    if (!window.confirm(`Delete all OHLCV data for ${mint.slice(0, 8)}...?`)) return;

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
    const hours = window.prompt("Delete inactive tokens older than (hours):", "24");
    if (!hours) return;

    const inactiveHours = parseInt(hours, 10);
    if (isNaN(inactiveHours) || inactiveHours < 1) {
      Utils.showToast("Invalid hours value", "error");
      return;
    }

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
    if (ohlcvTable) return; // Already initialized

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

    ohlcvTable = new DataTable({
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
        title: {
          icon: "icon-chart-candlestick",
          text: "OHLCV Data",
        },
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
    if (poller) poller.pause();
    if (lastUpdatePoller) lastUpdatePoller.pause();

    if (!ohlcvPoller) {
      ohlcvPoller = new Poller(
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
    ohlcvPoller.start();

    // Initial load
    fetchOhlcvData().then(() => updateOhlcvTable());
  };

  const hideOhlcvView = () => {
    const tokensRoot = document.querySelector("#tokens-root");
    const ohlcvContainer = document.querySelector("#ohlcv-table-container");

    if (tokensRoot) tokensRoot.style.display = "";
    if (ohlcvContainer) ohlcvContainer.style.display = "none";

    // Pause OHLCV poller, resume main poller
    if (ohlcvPoller) ohlcvPoller.pause();
    if (poller) poller.start();
    if (lastUpdatePoller) lastUpdatePoller.start();
  };

  // ═══════════════════════════════════════════════════════════════════════════
  // FAVORITES TABLE FUNCTIONS
  // ═══════════════════════════════════════════════════════════════════════════

  const buildFavoritesColumns = () => {
    return [
      {
        id: "token",
        label: "Token",
        sortable: true,
        minWidth: 200,
        wrap: false,
        render: (_v, row) => {
          const logo = row.logo_url
            ? `<img class="token-logo" alt="" src="${Utils.escapeHtml(row.logo_url)}" />`
            : '<span class="token-logo">N/A</span>';
          const sym = Utils.escapeHtml(row.symbol || "???");
          const name = row.name
            ? `<div class="token-name">${Utils.escapeHtml(row.name)}</div>`
            : "";
          return `<div class="token-cell">${logo}<div><div class="token-symbol">${sym}</div>${name}</div></div>`;
        },
      },
      {
        id: "mint",
        label: "Mint",
        sortable: true,
        minWidth: 150,
        wrap: false,
        render: (value) => {
          const short = value ? `${value.slice(0, 8)}...${value.slice(-4)}` : "—";
          return `<code class="mint-code" title="${Utils.escapeHtml(value)}">${short}</code>`;
        },
      },
      {
        id: "notes",
        label: "Notes",
        sortable: false,
        minWidth: 200,
        wrap: true,
        render: (value) => {
          if (!value) return '<span class="text-muted">—</span>';
          return Utils.escapeHtml(value);
        },
      },
      {
        id: "created_at",
        label: "Added",
        sortable: true,
        minWidth: 120,
        wrap: false,
        render: (value) => {
          if (!value) return "—";
          const timestamp =
            typeof value === "string" ? Math.floor(new Date(value).getTime() / 1000) : value;
          return timeAgoCell(timestamp);
        },
      },
      {
        id: "actions",
        label: "",
        sortable: false,
        minWidth: 120,
        maxWidth: 120,
        wrap: false,
        render: (_value, row) => {
          return `
            <div class="favorites-actions">
              <button class="btn btn-small btn-icon favorites-action-btn" data-action="copy" data-mint="${Utils.escapeHtml(row.mint)}" title="Copy Mint">
                <i class="icon-copy"></i>
              </button>
              <button class="btn btn-small btn-icon favorites-action-btn" data-action="external" data-mint="${Utils.escapeHtml(row.mint)}" title="View on DexScreener">
                <i class="icon-external-link"></i>
              </button>
              <button class="btn btn-small btn-icon btn-danger favorites-action-btn" data-action="remove" data-mint="${Utils.escapeHtml(row.mint)}" title="Remove from favorites">
                <i class="icon-trash-2"></i>
              </button>
            </div>
          `;
        },
      },
    ];
  };

  const fetchFavorites = async () => {
    favoritesState.isLoading = true;
    try {
      const response = await requestManager.fetch("/api/tokens/favorites", { priority: "normal" });
      if (response && response.favorites) {
        favoritesState.favorites = response.favorites;
      }
    } catch (err) {
      console.error("Failed to fetch favorites:", err);
      Utils.showToast("Failed to load favorites", "error");
    } finally {
      favoritesState.isLoading = false;
    }
  };

  const updateFavoritesTable = () => {
    if (!favoritesTable) return;
    const isEmpty = favoritesState.favorites.length === 0;
    const emptyState = document.querySelector("#favorites-empty-state");

    if (isEmpty) {
      favoritesTable.setData([], { preserveScroll: false });
      if (emptyState) emptyState.style.display = "";
    } else {
      if (emptyState) emptyState.style.display = "none";
      favoritesTable.setData(favoritesState.favorites, { preserveScroll: true });
    }
    updateFavoritesToolbar();
  };

  const updateFavoritesToolbar = () => {
    if (!favoritesTable) return;
    const count = favoritesState.favorites.length;

    favoritesTable.updateToolbarSummary([
      {
        id: "favorites-total",
        label: "Total Favorites",
        value: Utils.formatNumber(count, 0),
        variant: count > 0 ? "info" : "secondary",
      },
    ]);
  };

  const handleFavoriteAction = async (action, mint) => {
    switch (action) {
      case "copy":
        try {
          await navigator.clipboard.writeText(mint);
          Utils.showToast("Mint address copied", "success");
        } catch (err) {
          console.error("Failed to copy mint:", err);
          Utils.showToast("Failed to copy mint", "warning");
        }
        break;

      case "external":
        Utils.openExternal(`https://dexscreener.com/solana/${mint}`);
        break;

      case "remove":
        try {
          await requestManager.fetch(`/api/tokens/favorites/${encodeURIComponent(mint)}`, {
            method: "DELETE",
            priority: "high",
          });
          // Remove from local state
          favoritesState.favorites = favoritesState.favorites.filter((f) => f.mint !== mint);
          updateFavoritesTable();
          Utils.showToast("Removed from favorites", "success");
        } catch (err) {
          console.error("Failed to remove favorite:", err);
          Utils.showToast("Failed to remove favorite", "error");
        }
        break;
    }
  };

  const initFavoritesTable = () => {
    if (favoritesTable) return; // Already initialized

    // Create container for favorites table
    const rootEl = document.querySelector("#tokens-root");
    if (!rootEl) return;

    // Create favorites container
    let favoritesContainer = document.querySelector("#favorites-table-container");
    if (!favoritesContainer) {
      favoritesContainer = document.createElement("div");
      favoritesContainer.id = "favorites-table-container";
      favoritesContainer.className = "favorites-table-container";
      favoritesContainer.style.display = "none";
      rootEl.parentNode.insertBefore(favoritesContainer, rootEl.nextSibling);
    }

    // Create empty state element
    let emptyState = document.querySelector("#favorites-empty-state");
    if (!emptyState) {
      emptyState = document.createElement("div");
      emptyState.id = "favorites-empty-state";
      emptyState.className = "empty-state";
      emptyState.style.display = "none";
      emptyState.innerHTML = `
        <div class="empty-state-icon">⭐</div>
        <h3 class="empty-state-title">No Favorites Yet</h3>
        <p class="empty-state-description">
          Use the search (<kbd>⌘K</kbd>) to find tokens and add them to your favorites.
        </p>
      `;
      favoritesContainer.appendChild(emptyState);
    }

    favoritesTable = new DataTable({
      container: "#favorites-table-container",
      columns: buildFavoritesColumns(),
      rowIdField: "mint",
      stateKey: "favorites-table",
      enableLogging: false,
      sorting: {
        mode: "client",
        column: "created_at",
        direction: "desc",
      },
      clientPagination: {
        enabled: true,
        pageSizes: [10, 20, 50, 100, "all"],
        defaultPageSize: 50,
        stateKey: "tokens.favorites.pageSize",
      },
      compact: true,
      stickyHeader: true,
      zebra: true,
      fitToContainer: true,
      autoSizeColumns: false,
      uniformRowHeight: 2,
      toolbar: {
        title: {
          icon: "icon-star",
          text: "Favorite Tokens",
        },
        summary: [
          { id: "favorites-total", label: "Total Favorites", value: "0", variant: "secondary" },
        ],
      },
    });

    // Add click handler for action buttons
    favoritesContainer.addEventListener("click", (e) => {
      const actionBtn = e.target.closest(".favorites-action-btn");
      if (actionBtn) {
        const action = actionBtn.getAttribute("data-action");
        const mint = actionBtn.getAttribute("data-mint");
        if (action && mint) handleFavoriteAction(action, mint);
      }
    });
  };

  const showFavoritesView = () => {
    const tokensRoot = document.querySelector("#tokens-root");
    const favoritesContainer = document.querySelector("#favorites-table-container");
    const ohlcvContainer = document.querySelector("#ohlcv-table-container");

    if (tokensRoot) tokensRoot.style.display = "none";
    if (favoritesContainer) favoritesContainer.style.display = "";
    if (ohlcvContainer) ohlcvContainer.style.display = "none";

    // Pause main table poller
    if (poller) poller.pause();
    if (lastUpdatePoller) lastUpdatePoller.pause();
    if (ohlcvPoller) ohlcvPoller.pause();

    // Initial load
    fetchFavorites().then(() => updateFavoritesTable());
  };

  const hideFavoritesView = () => {
    const tokensRoot = document.querySelector("#tokens-root");
    const favoritesContainer = document.querySelector("#favorites-table-container");

    if (tokensRoot) tokensRoot.style.display = "";
    if (favoritesContainer) favoritesContainer.style.display = "none";

    // Resume main poller
    if (poller) poller.start();
    if (lastUpdatePoller) lastUpdatePoller.start();
  };

  // ═══════════════════════════════════════════════════════════════════════════
  // VIEW SWITCHING
  // ═══════════════════════════════════════════════════════════════════════════

  const switchView = (view) => {
    if (!TOKEN_VIEWS.some((v) => v.id === view)) return;
    const previousView = state.view;
    state.view = view;

    // Handle Favorites view specially - it has its own table
    if (view === "favorites") {
      initFavoritesTable();
      showFavoritesView();
      return;
    }

    // Leaving Favorites view - hide it
    if (previousView === "favorites") {
      hideFavoritesView();
    }

    // Handle OHLCV view specially - it has its own table
    if (view === "ohlcv") {
      initOhlcvTable();
      showOhlcvView();
      return;
    }

    // Leaving OHLCV view - hide it
    if (previousView === "ohlcv") {
      hideOhlcvView();
    }

    state.totalCount = null;
    state.lastUpdate = null;
    state.filters = getDefaultFiltersForView(view);
    state.summary = { ...DEFAULT_SUMMARY };
    state.availableRejectionReasons = [];

    if (view === "pool" && previousView !== "pool") {
      resetPriceTracking();
    } else if (previousView === "pool" && view !== "pool") {
      resetPriceTracking();
    }

    // Update table stateKey for per-tab state persistence
    if (table) {
      const nextStateKey = getTokensTableStateKey(view);
      table.setStateKey(nextStateKey, { render: false });

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
        const sortKey = getServerSortKey(restoredTableState.sortColumn, view);
        if (sortKey) {
          state.sort = {
            by: sortKey,
            direction: restoredTableState.sortDirection || "asc",
          };
        } else {
          state.sort = { ...DEFAULT_SERVER_SORT };
        }
      } else {
        const persistedSort = loadPersistedSort(nextStateKey);
        if (persistedSort) {
          const persistedKey = getServerSortKey(persistedSort.column, view);
          if (persistedKey) {
            state.sort = { by: persistedKey, direction: persistedSort.direction };
          } else {
            state.sort = { ...DEFAULT_SERVER_SORT };
          }
        } else {
          state.sort = { ...DEFAULT_SERVER_SORT };
        }
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
      birdeye: `https://birdeye.so/solana/token/${mint}`,
      rugcheck: `https://rugcheck.xyz/tokens/${mint}`,
      pumpfun: `https://pump.fun/${mint}`,
    };

    if (actionId === "copy") {
      navigator.clipboard
        .writeText(mint)
        .then(() => {
          Utils.showToast("Mint address copied", "success");
        })
        .catch((err) => {
          console.error("Failed to copy mint:", err);
          Utils.showToast("Failed to copy mint", "warning");
        });
    } else if (urlMap[actionId]) {
      Utils.openExternal(urlMap[actionId]);
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
          <i class="icon-chart-bar"></i>
          <span class="label">DexScreener</span>
        </button>
        <button class="dropdown-item" data-action="gmgn" type="button">
          <i class="icon-trending-up"></i>
          <span class="label">GMGN</span>
        </button>
        <button class="dropdown-item" data-action="solscan" type="button">
          <i class="icon-search"></i>
          <span class="label">Solscan</span>
        </button>
        <button class="dropdown-item" data-action="birdeye" type="button">
          <span class="icon"><i class="icon-chart-bar"></i></span>
          <span class="label">Birdeye</span>
        </button>
        <button class="dropdown-item" data-action="rugcheck" type="button">
          <span class="icon"><i class="icon-shield"></i></span>
          <span class="label">RugCheck</span>
        </button>
        <button class="dropdown-item" data-action="pumpfun" type="button">
          <span class="icon"><i class="icon-rocket"></i></span>
          <span class="label">Pump.fun</span>
        </button>
        <div class="dropdown-divider"></div>
        <button class="dropdown-item" data-action="copy" type="button">
          <span class="icon"><i class="icon-copy"></i></span>
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
      const menuClickHandler = (e) => {
        const item = e.target.closest(".dropdown-item");
        if (!item) return;

        const action = item.dataset.action;
        if (action) {
          handleLinkAction(action, mint);
        }
        menu.remove();
        document.removeEventListener("click", closeHandler);
        document.removeEventListener("keydown", escapeHandler);
      };
      menu.addEventListener("click", menuClickHandler);

      // Close on outside click
      const closeHandler = (e) => {
        if (!menu.contains(e.target) && e.target !== trigger) {
          menu.remove();
          document.removeEventListener("click", closeHandler);
          document.removeEventListener("keydown", escapeHandler);
        }
      };
      setTimeout(() => document.addEventListener("click", closeHandler), 0);

      // Close on escape
      const escapeHandler = (e) => {
        if (e.key === "Escape") {
          menu.remove();
          document.removeEventListener("click", closeHandler);
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
          const timestamp = rowData.blockchain_created_at || rowData.first_discovered_at;
          if (timestamp) {
            const ageSeconds =
              typeof timestamp === "string"
                ? Math.floor(new Date(timestamp).getTime() / 1000)
                : timestamp;
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

      // Close handlers with proper cleanup
      const escapeHandler = (e) => {
        if (e.key === "Escape") {
          close();
        }
      };
      document.addEventListener("keydown", escapeHandler);

      const close = () => {
        lightbox.classList.remove("active");
        document.removeEventListener("keydown", escapeHandler);
        setTimeout(() => lightbox.remove(), 300);
      };

      lightbox.querySelector(".lightbox-close").addEventListener("click", close);
      lightbox.querySelector(".lightbox-backdrop").addEventListener("click", close);
    };

    container._logoClickHandler = clickHandler;
    container.addEventListener("click", clickHandler);
  };

  const initializeRowClickHandler = () => {
    // Use event delegation for row clicks
    if (!table?.elements?.scrollContainer) {
      return;
    }

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
      const row = e.target.closest("tr[data-row-id]");
      if (!row) return;

      // Get the mint from the row's data-row-id attribute (set by DataTable)
      const mint = row.dataset.rowId;
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

      // Attach hint triggers to tabs
      attachHintsToTabs();

      // Get the active tab after state restoration
      const activeTab = tabBar.getActiveTab();
      if (activeTab && activeTab !== state.view) {
        // Sync state with tab bar's restored state (e.g., from URL hash)
        state.view = activeTab;
      }

      const tableStateKey = getTokensTableStateKey(state.view);
      const persistedSort = loadPersistedSort(tableStateKey);
      if (persistedSort) {
        const persistedSortKey = getServerSortKey(persistedSort.column, state.view);
        if (persistedSortKey) {
          state.sort = {
            by: persistedSortKey,
            direction: persistedSort.direction,
          };
        }
      }

      state.filters = getDefaultFiltersForView(state.view);
      state.hasLoadedOnce = false;

      const initialSortColumn = resolveSortColumn(state.sort.by);

      // Build columns based on current view
      const columns = buildColumns();

      table = new DataTable({
        container: "#tokens-root",
        columns,
        rowIdField: "mint",
        stateKey: tableStateKey,
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
          // Hybrid pagination modes: enable both scroll and pages
          modes: ["scroll", "pages"],
          defaultMode: "scroll",
          modeStateKey: `tokens.${state.view}.paginationMode`,
          defaultPageSize: 50,
          pageSizes: [10, 20, 50, 100],
          // Scroll mode settings
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
            icon: "icon-coins",
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

      table.showBlockingState?.({
        variant: "loading",
        title: "Loading tokens snapshot...",
        description: "Large token databases can take a few seconds to warm up during startup.",
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
            const data = await requestManager.fetch("/api/trader/manual/buy", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                mint,
                ...(result.amount ? { size_sol: result.amount } : {}),
              }),
              priority: "high",
            });
            btn.disabled = false;
            Utils.showToast("Buy placed", "success");
            requestReload("manual", { silent: false, preserveScroll: true }).catch(() => {});
          } else if (action === "add") {
            // Fetch config for entry sizes
            let entrySizes = [0.005, 0.01, 0.02, 0.05];
            try {
              const configData = await requestManager.fetch("/api/config/trader", {
                priority: "normal",
              });
              if (Array.isArray(configData?.data?.entry_sizes)) {
                entrySizes = configData.data.entry_sizes;
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
            const data = await requestManager.fetch("/api/trader/manual/add", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                mint,
                ...(result.amount ? { size_sol: result.amount } : {}),
              }),
              priority: "high",
            });
            btn.disabled = false;
            Utils.showToast("Added to position", "success");
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
            const data = await requestManager.fetch("/api/trader/manual/sell", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify(body),
              priority: "high",
            });
            btn.disabled = false;
            Utils.showToast("Sell placed", "success");
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
        const sortKey = getServerSortKey(serverState.sortColumn, state.view);
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

      // Handle OHLCV view specially - it has its own table
      if (state.view === "ohlcv") {
        initOhlcvTable();
        showOhlcvView();
      } else if (!hasSortRestored) {
        // Trigger initial data load if no sort state was restored
        // (sort restoration triggers reload via handleSortChange)
        requestReload("initial", { silent: false, resetScroll: true }).catch(() => {});
      }
    },

    activate(ctx) {
      // Re-register deactivate cleanup (cleanups are cleared after each deactivate)
      // and force-show tab bar to handle race conditions with TabBarManager
      if (tabBar) {
        ctx.manageTabBar(tabBar);
        tabBar.show({ force: true });
      }

      // Re-attach hints to tabs after TabBar remounts buttons on show()
      // (hints are lost when hide() clears innerHTML and show() rebuilds tabs)
      attachHintsToTabs();

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

      // Handle OHLCV view specially - it has its own poller
      if (state.view === "ohlcv") {
        if (ohlcvPoller) {
          ohlcvPoller.start();
        } else {
          // OHLCV poller not initialized yet, fetch data
          fetchOhlcvData().catch(() => {});
        }
        return;
      }

      // Handle Favorites view specially - refresh favorites data
      if (state.view === "favorites") {
        fetchFavorites()
          .then(() => updateFavoritesTable())
          .catch(() => {});
        return;
      }

      // Start poller to update "Last Update" display every second
      if (!lastUpdatePoller) {
        lastUpdatePoller = ctx.managePoller(
          new Poller(
            () => {
              // Only update if we have a lastUpdate timestamp
              if (state.lastUpdate && table) {
                updateToolbar();
              }
            },
            {
              label: "TokensLastUpdate",
              getInterval: () => 1000, // 1 second fixed interval
              pauseWhenHidden: true,
            }
          )
        );
        lastUpdatePoller.start();
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

      // Show billboard promotional row on tokens page
      showBillboardRow();
    },

    deactivate() {
      // Hide billboard row when leaving page
      hideBillboardRow();

      table?.cancelPendingLoad();
      // Lifecycle automatically stops managed pollers
      // Pause OHLCV poller if active
      if (ohlcvPoller) ohlcvPoller.pause();
    },

    dispose() {
      // Clean up all tracked event listeners
      eventCleanups.forEach((cleanup) => cleanup());
      eventCleanups.length = 0;

      // Lifecycle automatically cleans up managed pollers
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
        table.hideBlockingState?.();
        table.destroy();
        table = null;
      }
      // Clean up OHLCV table
      if (ohlcvTable) {
        ohlcvTable.destroy();
        ohlcvTable = null;
      }
      if (ohlcvPoller) {
        ohlcvPoller.stop();
        ohlcvPoller = null;
      }
      // Remove OHLCV container
      const ohlcvContainer = document.querySelector("#ohlcv-table-container");
      if (ohlcvContainer) {
        ohlcvContainer.remove();
      }
      // Clean up Favorites table
      if (favoritesTable) {
        favoritesTable.destroy();
        favoritesTable = null;
      }
      // Remove Favorites container
      const favoritesContainer = document.querySelector("#favorites-table-container");
      if (favoritesContainer) {
        favoritesContainer.remove();
      }
      poller = null;
      tabBar = null; // Cleaned up automatically by manageTabBar
      TabBarManager.unregister("tokens");
      resetPriceTracking();
      state.view = DEFAULT_VIEW;
      state.search = "";
      state.totalCount = null;
      state.lastUpdate = null;
      state.sort = { ...DEFAULT_SERVER_SORT };
      state.filters = getDefaultFiltersForView(DEFAULT_VIEW);
      state.summary = { ...DEFAULT_SUMMARY };
      state.hasLoadedOnce = false;
      walletBalance = 0;
      // Reset OHLCV state
      ohlcvState.tokens = [];
      ohlcvState.stats = null;
      ohlcvState.isLoading = false;
      // Reset Favorites state
      favoritesState.favorites = [];
      favoritesState.isLoading = false;
    },
  };
}

registerPage("tokens", createLifecycle());
