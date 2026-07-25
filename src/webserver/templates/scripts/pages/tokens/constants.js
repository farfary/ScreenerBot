/**
 * Constants and configuration for tokens page
 * Lines extracted from tokens.js (16-116, 185-271)
 */

// Sub-tabs (views) configuration with hint references
export const TOKEN_VIEWS = [
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
export const DEFAULT_VIEW = "all";
export const DEFAULT_SERVER_SORT = { by: "symbol", direction: "asc" };
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

// Rejection reason label mapping (machine code -> human-readable)
export const REJECTION_LABELS = {
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
  dex_vol5m_low: "5m volume too low",
  dex_vol5m_missing: "5m volume missing",
  dex_vol1h_low: "1h volume too low",
  dex_vol1h_missing: "1h volume missing",
  dex_vol6h_low: "6h volume too low",
  dex_vol6h_missing: "6h volume missing",
  dex_price_change_5m_low: "5m price change too low",
  dex_price_change_5m_high: "5m price change too high",
  dex_price_change_low: "Price change too low",
  dex_price_change_high: "Price change too high",
  dex_price_change_6h_low: "6h price change too low",
  dex_price_change_6h_high: "6h price change too high",
  dex_price_change_24h_low: "24h price change too low",
  dex_price_change_24h_high: "24h price change too high",
  gecko_liq_low: "Liquidity too low",
  gecko_liq_high: "Liquidity too high",
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
  gecko_price_change_1h_low: "1h price change too low",
  gecko_price_change_1h_high: "1h price change too high",
  gecko_price_change_24h_low: "24h price change too low",
  gecko_price_change_24h_high: "24h price change too high",
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
  rug_graph_insiders: "Graph insiders too high",
  rug_lp_providers_low: "LP providers too low",
  rug_lp_providers_missing: "LP providers missing",
  rug_lp_lock_low: "LP lock too low",
  rug_lp_lock_missing: "LP lock missing",
};
