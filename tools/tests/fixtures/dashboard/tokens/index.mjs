// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the tokens dashboard page.

const LIST_RUST = "src/webserver/routes/tokens/types.rs::TokenListResponse";
const ROWS = "#tokens-root tr[data-row-id]";
const EMPTY = { selector: "#tokens-root .dt-empty-state", text: "No data" };
const tab = (id) => `#subTabsContainer [data-tab-id="${id}"]`;
const DETAILS = {
  trigger: `${ROWS} .token-symbol`,
  dialog: ".token-details-dialog",
  close: ".token-details-dialog .dialog-close",
};
const FEATURED = {
  trigger: ".featured-row-view-all",
  dialog: ".featured-dialog",
  close: ".featured-dialog .dialog-close",
};

// Views served by `/api/tokens/list`, with the rows each fixture holds.
const LIST_VIEWS = [
  { view: "all", name: "all tokens", rows: 8 },
  { view: "pool", name: "pool service tokens", rows: 6 },
  { view: "no_market", name: "tokens without market data", rows: 3 },
  { view: "passed", name: "passed tokens", rows: 5 },
  { view: "rejected", name: "rejected tokens", rows: 3 },
  { view: "blacklisted", name: "blacklisted tokens", rows: 2 },
  { view: "positions", name: "tokens with positions", rows: 3 },
  { view: "recent", name: "recent tokens", rows: 6 },
];

// Every token the lists hold; the details dialog opens on whichever row a view sorts first.
const TOKENS = [
  { mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", symbol: "bonk" },
  { mint: "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm", symbol: "wif" },
  { mint: "7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr", symbol: "popcat" },
  { mint: "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN", symbol: "jup" },
  { mint: "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3", symbol: "pyth" },
  { mint: "orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE", symbol: "orca" },
  { mint: "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R", symbol: "ray" },
  { mint: "jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL", symbol: "jto" },
];

export const endpoints = [
  ...LIST_VIEWS.map(({ view }) => ({
    method: "GET",
    path: "/api/tokens/list",
    query: { view },
    fixture: `tokens_list_${view}.json`,
    empty: "tokens_list.empty.json",
    rust: LIST_RUST,
    record: `/api/tokens/list?view=${view}&sort_by=liquidity_usd&sort_dir=desc&limit=100`,
  })),
  {
    method: "GET",
    path: "/api/tokens/favorites",
    fixture: "favorites.json",
    empty: "favorites.empty.json",
    rust: "src/webserver/routes/tokens/types.rs::FavoritesListResponse",
  },
  {
    method: "GET",
    path: "/api/ohlcv/tokens",
    fixture: "ohlcv_tokens.json",
    empty: "ohlcv_tokens.empty.json",
    rust: "src/webserver/routes/ohlcv/types.rs::OhlcvTokenListResponse",
  },
  ...TOKENS.map(({ mint, symbol }) => ({
    method: "GET",
    path: `/api/tokens/${mint}`,
    fixture: `token_detail_${symbol}.json`,
    rust: "src/webserver/routes/tokens/types.rs::TokenDetailResponse",
    record: `/api/tokens/${mint}`,
  })),
  {
    method: "GET",
    path: "/api/tokens/{mint}/ohlcv",
    fixture: "token_ohlcv.json",
    rust: "src/webserver/routes/tokens/types.rs::OhlcvPoint",
    record: null,
  },
  {
    method: "GET",
    path: "/api/tokens/{mint}/ohlcv/status",
    fixture: "token_ohlcv_status.json",
    rust: "src/ohlcvs/types.rs::OhlcvStatus",
    record: null,
  },
  {
    method: "POST",
    path: "/api/tokens/{mint}/ohlcv/refresh",
    fixture: "token_ohlcv_refresh.json",
    rust: "src/webserver/routes/tokens/ohlcv.rs::refresh_token_ohlcv",
    record: null,
  },
  {
    method: "POST",
    path: "/api/tokens/{mint}/refresh",
    fixture: "token_refresh.json",
    rust: "src/webserver/routes/tokens/detail.rs::refresh_token_data",
    record: null,
  },
  {
    method: "POST",
    path: "/api/tokens/{mint}/focus",
    fixture: "token_focus.json",
    rust: "src/webserver/routes/tokens/types.rs::FocusResponse",
    record: null,
  },
  {
    method: "POST",
    path: "/api/tokens/{mint}/unfocus",
    fixture: "token_unfocus.json",
    rust: "src/webserver/routes/tokens/types.rs::FocusResponse",
    record: null,
  },
  {
    method: "GET",
    path: "/api/boosts",
    fixture: "boosts.json",
    empty: "boosts.empty.json",
    rust: "src/webserver/routes/boosts/types.rs::BoostStanding",
  },
  {
    method: "GET",
    path: "/api/featured/all",
    fixture: "featured_all.json",
    empty: "featured_all.empty.json",
    rust: "src/webserver/routes/featured/types.rs::FeaturedCard",
  },
];

export const views = [
  ...LIST_VIEWS.map(({ view, name, rows }) => ({
    name,
    click: [tab(view)],
    populated: [{ selector: ROWS, min: rows }],
    empty: [EMPTY],
    dialogs: view === "all" ? [DETAILS, FEATURED] : [DETAILS],
  })),
  {
    name: "favorite tokens",
    click: [tab("favorites")],
    populated: [{ selector: "#favorites-table-container tr[data-row-id]", min: 3 }],
    empty: [
      {
        selector: "#favorites-empty-state",
        text: "No Favorites Yet",
        defect:
          "The favorites empty state is rendered inside a container the table replaces, so it never shows",
      },
    ],
  },
  {
    name: "tokens with candle data",
    click: [tab("ohlcv")],
    populated: [{ selector: "#ohlcv-table-container tr[data-row-id]", min: 5 }],
    empty: [{ selector: "#ohlcv-table-container .dt-empty-state", text: "No data" }],
    dialogs: [
      { trigger: '#ohlcv-table-container [data-btn-id="cleanup"]', dialog: ".input-dialog" },
    ],
  },
];
