// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the wallets dashboard page.

// The empty variant keeps the main wallet (a running bot always has one) with no
// holdings, no other wallets and no watched addresses.
const tab = (id) => `#subTabsContainer [data-tab-id="${id}"]`;
const MAIN_ROWS = "#tokens-datatable-root tr[data-row-id]";
const OTHER_ROWS = "#other-wallets-table-container tr[data-row-id]";
const WATCHED_ROWS = "#watched-wallets-root tr[data-row-id]";
const dialog = (trigger, id, close) => ({ trigger, dialog: `#${id}`, close: `#${close}` });

export const endpoints = [
  {
    method: "GET",
    path: "/api/wallets",
    query: { include_inactive: "true" },
    fixture: "wallets_list.json",
    empty: "wallets_list.empty.json",
    rust: "src/webserver/routes/wallets/types.rs::WalletListResponse",
    record: "/api/wallets?include_inactive=true",
  },
  {
    method: "GET",
    path: "/api/wallet/tokens",
    fixture: "wallet_tokens.json",
    empty: "wallet_tokens.empty.json",
    rust: "src/webserver/routes/wallet/types.rs::WalletTokensResponse",
  },
  {
    method: "GET",
    path: "/api/wallets/watch",
    fixture: "wallets_watch.json",
    empty: "wallets_watch.empty.json",
    rust: "src/webserver/routes/wallets/watch.rs::TargetListResponse",
  },
  {
    method: "GET",
    path: "/api/wallets/backups",
    fixture: "wallets_backups.json",
    empty: "wallets_backups.empty.json",
    rust: "src/webserver/routes/wallets/types.rs::WalletBackupsResponse",
  },
  {
    method: "GET",
    path: "/api/wallets/watch/{id}/status",
    fixture: "wallets_watch_status.json",
    rust: "src/wallets/watch/types.rs::WatchStatus",
    record: "/api/wallets/watch/1/status",
  },
];

export const views = [
  {
    name: "main wallet holdings",
    click: [tab("main")],
    populated: [{ selector: MAIN_ROWS, min: 5 }],
    empty: [
      {
        selector: "#tokens-datatable-root .dt-state-cell > .state-view-empty",
        text: "No token holdings",
      },
    ],
    dialogs: [dialog('[data-btn-id="wt-export-key"]', "export-modal", "export-modal-close")],
  },
  {
    name: "other wallets",
    click: [tab("other")],
    populated: [{ selector: OTHER_ROWS, min: 4 }],
    empty: [
      {
        selector: "#other-wallets-table-container .dt-state-cell > .state-view-empty",
        text: "No other wallets",
      },
    ],
  },
  {
    name: "watched addresses",
    click: [tab("watched")],
    populated: [{ selector: WATCHED_ROWS, min: 3 }],
    empty: [
      {
        selector: "#watched-wallets-root .dt-state-cell > .state-view-empty",
        text: "No watched addresses",
      },
    ],
    dialogs: [
      dialog('[data-btn-id="watched-add"]', "watch-wallet-modal", "watch-modal-close"),
      dialog(
        `${WATCHED_ROWS} .dt-action-btn[data-action-id="budget"]`,
        "watch-budget-modal",
        "watch-budget-close"
      ),
    ],
  },
  {
    name: "other wallet row actions",
    click: [tab("other")],
    populated: [{ selector: `${OTHER_ROWS} .dt-action-btn`, min: 4 }],
    dialogs: [
      dialog(
        `${OTHER_ROWS} .dt-action-btn[data-action-id="export"]`,
        "export-modal",
        "export-modal-close"
      ),
    ],
  },
];
