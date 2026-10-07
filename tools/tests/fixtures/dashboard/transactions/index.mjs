// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the transactions dashboard page.

const ROWS = "#transactions-root tr[data-row-id]";
// The newest transaction: the first row, whose details dialog the view opens.
const NEWEST =
  "5Synth5uqdh3DhVVh35Hu7q51Xs91jqmDH1dXfXTfFh9s5TMmouhoKh5sMqFM53TP3Hh3mTd7qKs11M9o1oTjHFb";

export const endpoints = [
  {
    method: "POST",
    path: "/api/transactions/list",
    fixture: "transactions_list.json",
    empty: "transactions_list.empty.json",
    rust: "src/webserver/routes/transactions/types.rs::ListTransactionsResponse",
    record: null,
  },
  {
    method: "POST",
    path: "/api/transactions/summary",
    fixture: "transactions_summary.json",
    empty: "transactions_summary.empty.json",
    rust: "src/webserver/routes/transactions/types.rs::TransactionSummaryResponse",
    record: null,
  },
  {
    method: "GET",
    path: `/api/transactions/${NEWEST}`,
    fixture: "transaction_detail.json",
    rust: "src/webserver/routes/transactions/types.rs::TransactionDetailResponse",
    record: `/api/transactions/${NEWEST}`,
  },
  {
    method: "GET",
    path: "/api/tokens/identities",
    fixture: "token_identities.json",
    rust: "src/webserver/routes/tokens/identity.rs::IdentitiesResponse",
    record: "/api/tokens/identities?mints=DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
  },
  {
    method: "GET",
    path: "/api/wallets/watch",
    fixture: "wallets_watch.json",
    empty: "wallets_watch.empty.json",
    rust: "src/webserver/routes/wallets/watch.rs::TargetListResponse",
  },
];

export const views = [
  {
    name: "wallet transactions",
    populated: [{ selector: ROWS, min: 7 }],
    empty: [{ selector: "#transactions-root .dt-empty-state", text: "No data" }],
    dialogs: [
      {
        trigger: `${ROWS} td`,
        dialog: ".transaction-details-dialog",
        close: ".transaction-details-dialog .dialog-close",
      },
    ],
  },
];
