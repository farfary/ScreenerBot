// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the wallets dashboard page.

// The empty variant keeps the main wallet (a running bot always has one) with no
// holdings, no secondary or archived wallets and no watched addresses.
const tab = (id) => `#subTabsContainer [data-tab-id="${id}"]`;
const MAIN_ROWS = "#tokens-datatable-root tr[data-row-id]";
const SECONDARY_ROWS = "#secondaries-table-container tr[data-row-id]";
// Import and export sit in the table toolbar's overflow menu.
const SECONDARY_MENU = "#secondaries-table-container .table-toolbar-overflow__trigger";
const ARCHIVE_ROWS = "#archive-table-container tr[data-row-id]";
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
    path: "/api/wallet/current",
    fixture: "wallet_current.json",
    empty: "wallet_current.empty.json",
    rust: "src/webserver/routes/wallet/types.rs::WalletCurrentResponse",
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
    empty: [{ selector: "#tokens-datatable-root .dt-empty-state", text: "No token holdings" }],
    dialogs: [dialog('[data-btn-id="wt-export-key"]', "export-modal", "export-modal-close")],
  },
  {
    name: "secondary wallets",
    click: [tab("secondaries")],
    populated: [{ selector: SECONDARY_ROWS, min: 2 }],
    empty: [
      { selector: "#secondaries-table-container .dt-empty-state", text: "No secondary wallets" },
    ],
    dialogs: [dialog('[data-btn-id="secondaries-add"]', "add-wallet-modal", "modal-close-btn")],
  },
  {
    name: "secondary wallets bulk import",
    click: [tab("secondaries"), SECONDARY_MENU],
    populated: [{ selector: SECONDARY_ROWS, min: 2 }],
    dialogs: [
      dialog('[data-btn-id="secondaries-import"]', "bulk-import-modal", "bulk-import-modal-close"),
    ],
  },
  {
    name: "secondary wallets bulk export",
    click: [tab("secondaries"), SECONDARY_MENU],
    populated: [{ selector: SECONDARY_ROWS, min: 2 }],
    dialogs: [
      dialog('[data-btn-id="secondaries-export"]', "bulk-export-modal", "bulk-export-modal-close"),
    ],
  },
  {
    name: "archived wallets",
    click: [tab("archive")],
    populated: [{ selector: ARCHIVE_ROWS, min: 2 }],
    empty: [{ selector: "#archive-table-container .dt-empty-state", text: "No archived wallets" }],
  },
  {
    name: "watched addresses",
    click: [tab("watched")],
    populated: [{ selector: WATCHED_ROWS, min: 3 }],
    empty: [{ selector: "#watched-wallets-root .dt-empty-state", text: "No watched addresses" }],
    dialogs: [
      dialog('[data-btn-id="watched-add"]', "watch-wallet-modal", "watch-modal-close"),
      dialog(
        `${WATCHED_ROWS} [data-watch-action="budget"]`,
        "watch-budget-modal",
        "watch-budget-close"
      ),
    ],
  },
  {
    name: "secondary wallet row actions",
    click: [tab("secondaries")],
    populated: [{ selector: `${SECONDARY_ROWS} .dt-action-btn`, min: 4 }],
    dialogs: [
      dialog(
        `${SECONDARY_ROWS} .dt-action-btn[data-action-id="export"]`,
        "export-modal",
        "export-modal-close"
      ),
      dialog(
        `${SECONDARY_ROWS} .dt-action-btn[data-action-id="archive"]`,
        "archive-modal",
        "archive-modal-close"
      ),
    ],
  },
  {
    name: "archived wallet row actions",
    click: [tab("archive")],
    populated: [{ selector: `${ARCHIVE_ROWS} .dt-action-btn`, min: 6 }],
    dialogs: [
      dialog(
        `${ARCHIVE_ROWS} .dt-action-btn[data-action-id="export"]`,
        "export-modal",
        "export-modal-close"
      ),
      dialog(
        `${ARCHIVE_ROWS} .dt-action-btn[data-action-id="delete"]`,
        "delete-modal",
        "delete-modal-close"
      ),
    ],
  },
];
