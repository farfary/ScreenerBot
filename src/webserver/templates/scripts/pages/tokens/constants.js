// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/** Constants and configuration for the Tokens page. */

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

/** Empty state of the token table per view: the title, with the reason as `.message`. */
export const TOKEN_VIEW_EMPTY_LABELS = Object.freeze({
  pool: "tokens-view-pool-empty",
  no_market: "tokens-view-no-market-empty",
  all: "tokens-view-all-empty",
  passed: "tokens-view-passed-empty",
  rejected: "tokens-view-rejected-empty",
  blacklisted: "tokens-view-blacklisted-empty",
  positions: "tokens-view-positions-empty",
  recent: "tokens-view-recent-empty",
});

const view = (id, icon, hintKey) => ({
  id,
  icon,
  label: I18n.label(TOKEN_VIEW_LABELS, id),
  hintKey,
});

// Sub-tabs (views) configuration with hint references
export const TOKEN_VIEWS = [
  view("favorites", "icon-star", "tokens.favorites"),
  view("pool", "icon-droplet", "tokens.poolService"),
  view("no_market", "icon-trending-down", "tokens.noMarketData"),
  view("all", "icon-list", "tokens.allTokens"),
  view("passed", "icon-check", "tokens.passedTokens"),
  view("rejected", "icon-circle-x", "tokens.rejectedTokens"),
  view("blacklisted", "icon-ban", "tokens.blacklistedTokens"),
  view("positions", "icon-chart-bar", "tokens.positionsTokens"),
  view("recent", "icon-clock", "tokens.recentTokens"),
  view("ohlcv", "icon-chart-candlestick", "tokens.ohlcvData"),
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
