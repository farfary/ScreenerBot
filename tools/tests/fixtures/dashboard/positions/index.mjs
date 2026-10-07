// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the positions dashboard page.

const ROWS = "#positions-root tr[data-row-id]";
const EMPTY = { selector: "#positions-root .dt-empty-state", text: "No data" };
const DETAILS = {
  trigger: `${ROWS} .token-symbol`,
  dialog: ".position-details-dialog .dialog-container",
  close: ".position-details-dialog .dialog-close",
};

// Positions whose details dialog the views open: the first row of each list.
const DETAIL_POSITIONS = [
  { id: 100, status: "open", mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", symbol: "bonk" },
  { id: 103, status: "closed", mint: "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN", symbol: "jup" },
  {
    id: 106,
    status: "archived",
    mint: "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm",
    symbol: "wif",
  },
];

export const endpoints = [
  {
    method: "GET",
    path: "/api/positions",
    query: { status: "open" },
    fixture: "positions_open.json",
    empty: "positions_list.empty.json",
    rust: "src/webserver/routes/positions/types.rs::PositionResponse",
    record: "/api/positions?status=open&limit=0",
  },
  {
    method: "GET",
    path: "/api/positions",
    query: { status: "closed" },
    fixture: "positions_closed.json",
    empty: "positions_list.empty.json",
    rust: "src/webserver/routes/positions/types.rs::PositionResponse",
    record: "/api/positions?status=closed&limit=0",
  },
  {
    method: "GET",
    path: "/api/positions",
    query: { status: "archived" },
    fixture: "positions_archived.json",
    empty: "positions_list.empty.json",
    rust: "src/webserver/routes/positions/types.rs::PositionResponse",
    record: "/api/positions?status=archived&limit=0",
  },
  ...DETAIL_POSITIONS.flatMap(({ id, status }) => [
    {
      method: "GET",
      path: `/api/positions/id:${id}/details`,
      fixture: `position_details_${status}.json`,
      rust: "src/webserver/routes/positions/types.rs::PositionDetailResponse",
      record: `/api/positions/id:${id}/details`,
    },
    {
      method: "GET",
      path: `/api/positions/id:${id}/activity`,
      fixture: `position_activity_${status}.json`,
      rust: "src/webserver/routes/positions/types.rs::TokenActivityResponse",
      record: `/api/positions/id:${id}/activity`,
    },
  ]),
  ...DETAIL_POSITIONS.map(({ mint, symbol }) => ({
    method: "GET",
    path: `/api/tokens/${mint}/ohlcv`,
    fixture: `token_ohlcv_${symbol}.json`,
    rust: "src/webserver/routes/tokens/types.rs::OhlcvPoint",
    record: `/api/tokens/${mint}/ohlcv?timeframe=5m&limit=0`,
  })),
  {
    method: "GET",
    path: "/api/tokens/{mint}/ohlcv/status",
    fixture: "token_ohlcv_status.json",
    rust: "src/ohlcvs/types.rs::OhlcvStatus",
    record: null,
  },
  {
    method: "GET",
    path: "/api/tokens/favorites",
    fixture: "favorites.json",
    rust: "src/webserver/routes/tokens/types.rs::FavoritesListResponse",
  },
];

export const views = [
  {
    name: "open positions",
    click: ['#subTabsContainer [data-tab-id="open"]'],
    populated: [{ selector: ROWS, min: 3 }],
    empty: [EMPTY],
    dialogs: [DETAILS],
  },
  {
    name: "closed positions",
    click: ['#subTabsContainer [data-tab-id="closed"]'],
    populated: [{ selector: ROWS, min: 3 }],
    empty: [EMPTY],
    dialogs: [DETAILS],
  },
  {
    name: "archived positions",
    click: ['#subTabsContainer [data-tab-id="archived"]'],
    populated: [{ selector: ROWS, min: 2 }],
    empty: [EMPTY],
    dialogs: [
      DETAILS,
      {
        trigger: '#positions-root [data-btn-id="delete-all-archived"]',
        dialog: ".confirmation-dialog",
      },
    ],
  },
];
