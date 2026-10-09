// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the copy dashboard page.

// The populated book holds three tasks (paper, live, paused); the empty book holds
// one task created a moment ago, so every workspace tab shows its own empty state.
const TASK_ROW = "#copy-list-rows .copy-row";

export const endpoints = [
  {
    method: "GET",
    path: "/api/copy-trading/overview",
    fixture: "copy_overview.json",
    empty: "copy_overview.empty.json",
    rust: "src/trader/copy/control.rs::CopyTradingOverview",
  },
  {
    method: "GET",
    path: "/api/copy-trading/defaults",
    fixture: "copy_defaults.json",
    rust: "src/trader/copy/workspace.rs::CopyDefaults",
  },
  {
    method: "GET",
    path: "/api/copy-trading/tasks/{id}/workspace",
    fixture: "copy_workspace.json",
    empty: "copy_workspace.empty.json",
    rust: "src/trader/copy/workspace.rs::CopyTaskWorkspace",
    record: "/api/copy-trading/tasks/1/workspace",
  },
  {
    method: "GET",
    path: "/api/copy-trading/tasks/{id}/activity",
    fixture: "copy_activity.json",
    empty: "copy_activity.empty.json",
    rust: "src/trader/copy/workspace.rs::ActivityPage",
    record: "/api/copy-trading/tasks/1/activity?filter=all&limit=50",
  },
  {
    method: "GET",
    path: "/api/copy-trading/tasks/{id}/insights",
    fixture: "copy_insights.json",
    empty: "copy_insights.empty.json",
    rust: "src/trader/copy/insights.rs::CopyInsights",
    record: "/api/copy-trading/tasks/1/insights",
  },
  {
    // `compare_tasks` wraps the rows as `{ "tasks": [...] }`.
    method: "GET",
    path: "/api/copy-trading/insights",
    fixture: "copy_compare.json",
    empty: "copy_compare.empty.json",
    rust: "src/trader/copy/workspace.rs::TaskComparison",
  },
  {
    method: "GET",
    path: "/api/copy-trading/wallets/{address}",
    fixture: "copy_wallet_profile.json",
    empty: "copy_wallet_profile.empty.json",
    rust: "src/trader/copy/workspace/profile.rs::WalletProfile",
    record: null,
  },
  {
    method: "GET",
    path: "/api/config/copy_trading",
    fixture: "config_copy_trading.json",
    rust: "src/webserver/routes/config/types.rs::ConfigResponse<CopyTradingConfig>",
  },
  {
    method: "GET",
    path: "/api/features",
    fixture: "features.json",
    rust: "src/features/types.rs::Features",
  },
  {
    method: "GET",
    path: "/api/wallets/watch",
    fixture: "wallets_watch.json",
    rust: "src/webserver/routes/wallets/watch.rs::TargetListResponse",
  },
  {
    method: "GET",
    path: "/api/wallets/watch/{id}/status",
    fixture: "wallets_watch_status.json",
    rust: "src/wallets/watch/types.rs::WatchStatus",
    record: "/api/wallets/watch/1/status",
  },
  {
    method: "GET",
    path: "/api/tokens/identities",
    fixture: "token_identities.json",
    rust: "src/webserver/routes/tokens/identity.rs::IdentitiesResponse",
    record: null,
  },
];

export const views = [
  {
    name: "overview",
    click: [TASK_ROW, "#copy-tab-overview"],
    populated: [
      { selector: TASK_ROW, min: 3 },
      { selector: "#copy-figures .copy-figure", min: 6 },
      { selector: "#copy-ws-panel .copy-check", min: 5 },
      { selector: "#copy-ws-panel .copy-chart-plot", min: 1 },
    ],
    empty: [
      { selector: "#copy-ws-panel .copy-chart-empty", text: "No closed rounds in this range yet" },
    ],
    dialogs: [
      { trigger: "#copy-add", dialog: "#copy-editor", close: "#copy-editor .modal-close" },
      {
        trigger: "#copy-settings-open",
        dialog: "#copy-settings",
        close: "#copy-settings .modal-close",
      },
      {
        trigger: "#copy-ws-actions [data-ws-action='profile']",
        dialog: "#copy-profile",
        close: "#copy-profile .modal-close",
      },
      {
        trigger: "#copy-ws-panel [data-ws-action='arm']",
        dialog: "#copy-arm",
        close: "#copy-arm .modal-close",
      },
      {
        trigger: "#copy-ws-actions [data-ws-action='edit']",
        dialog: "#copy-editor",
        close: "#copy-editor .modal-close",
      },
    ],
  },
  {
    name: "holdings",
    click: [TASK_ROW, "#copy-tab-holdings"],
    populated: [{ selector: "#copy-ws-panel .copy-table tbody tr", min: 3 }],
    empty: [{ selector: "#copy-ws-panel .copy-panel-message", text: "No open paper holdings" }],
  },
  {
    name: "activity",
    click: [TASK_ROW, "#copy-tab-activity"],
    populated: [{ selector: "#copy-ws-panel .copy-events .copy-event", min: 6 }],
    empty: [{ selector: "#copy-ws-panel .copy-panel-message", text: "No decisions yet" }],
  },
  {
    name: "rules",
    click: [TASK_ROW, "#copy-tab-rules"],
    populated: [{ selector: "#copy-ws-panel .copy-rules tbody tr", min: 10 }],
  },
  {
    name: "execution",
    click: [TASK_ROW, "#copy-tab-execution"],
    populated: [{ selector: "#copy-ws-panel .copy-metric", min: 3 }],
    empty: [
      { selector: "#copy-ws-panel .copy-chart-empty", text: "No arrival samples in this range" },
    ],
  },
  {
    name: "compare",
    click: ["#copy-compare-open"],
    populated: [
      { selector: "#copy-compare-table tbody tr[data-row-id]", min: 3 },
      { selector: "#copy-compare .copy-chart-plot", min: 1 },
    ],
    empty: [{ selector: "#copy-compare .copy-chart-empty", text: "No closed rounds to compare" }],
  },
];
