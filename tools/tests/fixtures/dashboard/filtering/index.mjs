// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the filtering dashboard page.

export const endpoints = [
  {
    method: "GET",
    path: "/api/config/filtering",
    fixture: "config_filtering.json",
    rust: "src/webserver/routes/config/types.rs::ConfigResponse<FilteringConfig>",
  },
  {
    method: "GET",
    path: "/api/config/metadata",
    fixture: "config_metadata.json",
    rust: "src/webserver/routes/config/types.rs::ConfigMetadataResponse",
  },
  {
    method: "GET",
    path: "/api/filtering/stats",
    fixture: "filtering_stats.json",
    empty: "filtering_stats.empty.json",
    rust: "src/webserver/routes/filtering/types.rs::FilteringStatsResponse",
  },
  {
    method: "GET",
    path: "/api/filtering/rejection-stats",
    fixture: "filtering_rejection_stats.json",
    empty: "filtering_rejection_stats.empty.json",
    rust: "src/webserver/routes/filtering/types.rs::RejectionStatsResponse",
  },
  {
    method: "GET",
    path: "/api/filtering/analytics",
    fixture: "filtering_analytics.json",
    empty: "filtering_analytics.empty.json",
    rust: "src/webserver/routes/filtering/types.rs::AnalyticsResponse",
  },
];

const tab = (id) => `#subTabsContainer [data-tab-id="${id}"]`;

/** A settings sub-tab: its parameter groups render from the config metadata. */
const settings = (id, groups) => ({
  name: `${id} settings`,
  click: [tab(id)],
  populated: [
    { selector: "#filtering-config-panels .filtering-group", min: groups },
    { selector: "#filtering-toolbar #filtering-search" },
  ],
});

export const views = [
  {
    name: "status",
    click: [tab("status")],
    populated: [
      { selector: "#filtering-info-bar .table-toolbar-chip", min: 6 },
      { selector: ".status-view .metric-card", min: 7 },
      { selector: ".status-rejection-section .rej-source-pill", min: 5 },
      { selector: ".status-rejection-section .rejection-item", min: 14 },
    ],
    empty: [{ selector: ".status-rejection-empty", text: "No rejection data available" }],
  },
  {
    name: "analytics",
    click: [tab("analytics")],
    populated: [
      { selector: ".analytics-view .kpi-card", min: 3 },
      { selector: ".analytics-view .bar-chart-row", min: 10 },
      { selector: ".analytics-view .reasons-table tbody tr", min: 10 },
    ],
    empty: [{ selector: ".analytics-view .analytics-empty", text: "No category data" }],
  },
  {
    name: "explorer",
    click: [tab("explorer")],
    populated: [
      { selector: "#explorer-tree .tree-category", min: 8 },
      { selector: "#explorer-detail-view .overview-list-item", min: 20 },
    ],
    empty: [{ selector: "#explorer-detail-view .analytics-empty-compact", text: "No data" }],
  },
  settings("meta", 2),
  settings("onchain", 3),
  settings("dexscreener", 5),
  settings("geckoterminal", 4),
  settings("rugcheck", 5),
];
