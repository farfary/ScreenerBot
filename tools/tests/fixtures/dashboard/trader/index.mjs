// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the trader dashboard page, including its embedded strategies roster and editor.

export const endpoints = [
  {
    method: "GET",
    path: "/api/features",
    fixture: "features.json",
    rust: "src/features/types.rs::Features",
  },
  {
    method: "GET",
    path: "/api/config/metadata",
    fixture: "config_metadata.json",
    rust: "src/webserver/routes/config/types.rs::ConfigMetadataResponse",
  },
  {
    method: "GET",
    path: "/api/config",
    fixture: "config_full.json",
    rust: "src/webserver/routes/config/types.rs::FullConfigResponse",
  },
  {
    method: "GET",
    path: "/api/trader/status",
    fixture: "trader_status.json",
    rust: "src/trader/status.rs::TraderStatus",
  },
  {
    method: "GET",
    path: "/api/trader/stats",
    query: { window: "30d" },
    fixture: "trader_stats.json",
    empty: "trader_stats.empty.json",
    rust: "src/trader/stats/types.rs::TraderStats",
    record: "/api/trader/stats?window=30d",
  },
  {
    // The recent closes resolve their token names and logos.
    method: "GET",
    path: "/api/tokens/identities",
    fixture: "token_identities.json",
    rust: "src/webserver/routes/tokens/identity.rs::IdentitiesResponse",
    record: null,
  },
  {
    method: "GET",
    path: "/api/trader/force-stop/status",
    fixture: "trader_force_stop_status.json",
    rust: "src/global/mod.rs::ForceStopStatus",
  },
  {
    method: "GET",
    path: "/api/trader/monitors/status",
    fixture: "trader_monitors_status.json",
    rust: "src/trader/controller.rs::MonitorsStatus",
  },
  {
    method: "GET",
    path: "/api/trader/loss-limit/status",
    fixture: "trader_loss_limit_status.json",
    rust: "src/trader/controller.rs::LossLimitSnapshot",
  },
  {
    method: "GET",
    path: "/api/strategies/overview",
    fixture: "strategies_overview.json",
    empty: "strategies_overview.empty.json",
    rust: "src/webserver/routes/strategies/types.rs::StrategiesOverviewResponse",
  },
  {
    method: "GET",
    path: "/api/strategies/conditions/schemas",
    fixture: "strategies_condition_schemas.json",
    rust: "src/webserver/routes/strategies/types.rs::ConditionSchemasResponse",
  },
  {
    method: "GET",
    path: "/api/positions",
    query: { status: "open" },
    fixture: "positions_open.json",
    empty: "positions_open.empty.json",
    rust: "src/webserver/routes/positions/types.rs::PositionResponse",
    record: "/api/positions?status=open&limit=0",
  },
  {
    method: "GET",
    path: "/api/trader/time-exits",
    fixture: "trader_time_exits.json",
    empty: "trader_time_exits.empty.json",
    rust: "src/webserver/routes/trader/types.rs::PositionTimeExit",
  },
];

const tab = (id) => `#subTabsContainer [data-tab-id="${id}"]`;

/** A settings sub-tab: its inputs carry the label the config metadata supplies. */
const settings = (id, panel, fields, defect) => ({
  name: `${id} settings`,
  click: [tab(id)],
  populated: [
    { selector: `#${panel} input[aria-label], #${panel} select[aria-label]`, min: fields, defect },
  ],
});

export const views = [
  {
    name: "stats",
    click: [tab("stats")],
    populated: [
      { selector: '#stats-state [data-summary-id="run"]' },
      { selector: "#stats-figures .metric-card", min: 8 },
      // One bar per day with a closed trade: 16 of the fixture's 30 days.
      { selector: "#daily-pnl svg .inline-chart-bar", min: 16 },
      { selector: "#outcome-distribution .inline-histogram-col", min: 9 },
      { selector: "#stats-extremes #best-trade .pnl-positive" },
      { selector: "#exit-breakdown tbody tr", min: 5 },
      { selector: "#source-breakdown tbody tr", min: 2 },
      { selector: "#recent-rounds tbody tr", min: 10 },
    ],
    empty: [
      {
        selector: "#stats-empty:not([hidden]) .state-view-empty",
        text: "No closed trades in the last \\W*30\\W* days",
      },
    ],
    dialogs: [
      {
        trigger: "#force-stop-btn",
        dialog: ".confirmation-dialog",
        close: '.confirmation-dialog [data-action="cancel"]',
      },
    ],
  },
  {
    name: "strategies",
    click: [tab("strategies")],
    populated: [{ selector: "#strategies-roster tbody tr[data-row-id]", min: 3 }],
    empty: [{ selector: "#strategies-roster .state-view-empty", text: "No strategies yet" }],
    dialogs: [
      {
        trigger: '#strategies-roster [data-btn-id="create-strategy"]',
        dialog: "#create-strategy-modal",
        close: "#create-strategy-close",
      },
    ],
  },
  settings("stop-loss", "stop-loss-tab", 4),
  settings("trailing-stop", "trailing-stop-tab", 3),
  settings("take-profit", "take-profit-tab", 3),
  {
    ...settings("time-rules", "time-rules-tab", 4),
    populated: [
      ...settings("time-rules", "time-rules-tab", 4).populated,
      { selector: "#time-positions-status tbody .ti-row-cell", min: 3 },
    ],
    empty: [
      {
        selector: "#time-positions-status .dt-state-cell > .state-view-empty",
        text: "No open positions",
      },
    ],
  },
  settings("dca", "dca-tab", 5),
  settings("general-settings", "general-settings-tab", 5),
];
