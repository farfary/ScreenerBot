/**
 * Constants and configuration for tokens page
 * Lines extracted from tokens.js (16-116, 185-271)
 */

import { escapeHtml } from "../../core/utils.js";

// Ids are the `view` query values of /api/tokens/list.
const TOKEN_VIEW_LABELS = Object.freeze({
  favorites: "tokens-view-favorites",
  pool: "tokens-view-pool",
  no_market: "tokens-view-no-market",
  all: "tokens-view-all",
  passed: "tokens-view-passed",
  rejected: "tokens-view-rejected",
  blacklisted: "tokens-view-blacklisted",
  positions: "tokens-view-positions",
  recent: "tokens-view-recent",
  ohlcv: "tokens-view-ohlcv",
});

const viewLabel = (id, icon) =>
  `<i class="${icon}"></i> ${escapeHtml(I18n.label(TOKEN_VIEW_LABELS, id))}`;

// Sub-tabs (views) configuration with hint references
export const TOKEN_VIEWS = [
  { id: "favorites", label: viewLabel("favorites", "icon-star"), hintKey: "tokens.favorites" },
  { id: "pool", label: viewLabel("pool", "icon-droplet"), hintKey: "tokens.poolService" },
  {
    id: "no_market",
    label: viewLabel("no_market", "icon-trending-down"),
    hintKey: "tokens.noMarketData",
  },
  { id: "all", label: viewLabel("all", "icon-list"), hintKey: "tokens.allTokens" },
  { id: "passed", label: viewLabel("passed", "icon-check"), hintKey: "tokens.passedTokens" },
  {
    id: "rejected",
    label: viewLabel("rejected", "icon-circle-x"),
    hintKey: "tokens.rejectedTokens",
  },
  {
    id: "blacklisted",
    label: viewLabel("blacklisted", "icon-ban"),
    hintKey: "tokens.blacklistedTokens",
  },
  {
    id: "positions",
    label: viewLabel("positions", "icon-chart-bar"),
    hintKey: "tokens.positionsTokens",
  },
  { id: "recent", label: viewLabel("recent", "icon-clock"), hintKey: "tokens.recentTokens" },
  {
    id: "ohlcv",
    label: viewLabel("ohlcv", "icon-chart-candlestick"),
    hintKey: "tokens.ohlcvData",
  },
];

// Constants
export const DEFAULT_VIEW = "all";
// Matches `FilteringQuery::default()` on the backend. Alphabetical was never a
// useful first view of a token screener — it ranks by the first character of a
// ticker — and it disagreed with the order the server sorts by when the client
// sends no preference.
export const DEFAULT_SERVER_SORT = { by: "liquidity_usd", direction: "desc" };
export const DEFAULT_FILTERS = {
  pool_price: false,
  positions: false,
  rejection_reason: "all",
};
export const DEFAULT_SUMMARY = { priced: 0, positions: 0, blacklisted: 0 };

export const getDefaultFiltersForView = (view) => {
  const filters = { ...DEFAULT_FILTERS };
  if (view === "positions") {
    filters.positions = true;
  }
  return filters;
};

export const COLUMN_TO_SORT_KEY = {
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
export const getServerSortKey = (columnId, currentView) => {
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

export const SORT_KEY_TO_COLUMN = Object.entries(COLUMN_TO_SORT_KEY).reduce(
  (acc, [columnId, sortKey]) => {
    acc[sortKey] = columnId;
    return acc;
  },
  {},
);

export const getTokensTableStateKey = (view) => `tokens-table.${view}`;

export const PAGE_LIMIT = 100; // chunked fetch size for incremental scrolling
export const PRICE_HIGHLIGHT_DURATION_MS = 10_000;
