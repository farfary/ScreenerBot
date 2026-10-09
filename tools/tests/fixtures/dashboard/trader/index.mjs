// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the trader dashboard page, including its embedded strategies editor.

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
    rust: "src/trader/controller.rs::TraderStatus",
  },
  {
    method: "GET",
    path: "/api/trader/stats",
    query: { days: 30 },
    fixture: "trader_stats.json",
    empty: "trader_stats.empty.json",
    rust: "src/trader/stats.rs::TraderStats",
    record: "/api/trader/stats?days=30",
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
    path: "/api/strategies",
    query: { type: "ENTRY" },
    fixture: "strategies_entry.json",
    empty: "strategies.empty.json",
    rust: "src/webserver/routes/strategies/types.rs::StrategyListResponse",
    record: "/api/strategies?type=ENTRY",
  },
  {
    method: "GET",
    path: "/api/strategies",
    query: { type: "EXIT" },
    fixture: "strategies_exit.json",
    empty: "strategies.empty.json",
    rust: "src/webserver/routes/strategies/types.rs::StrategyListResponse",
    record: "/api/strategies?type=EXIT",
  },
  {
    method: "GET",
    path: "/api/strategies",
    fixture: "strategies_all.json",
    empty: "strategies.empty.json",
    rust: "src/webserver/routes/strategies/types.rs::StrategyListResponse",
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
      { selector: '#trader-status-bar[data-status="running"]' },
      { selector: "#daily-pnl svg .daily-pnl-bar", min: 30 },
      { selector: "#exit-breakdown .exit-breakdown-row", min: 5 },
      { selector: "#stats-extremes:not([hidden]) #best-trade.positive" },
    ],
    empty: [
      { selector: "#daily-pnl .state-view-empty", text: "No closed trades in this window" },
      {
        selector: "#exit-breakdown .state-view-empty",
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
    name: "strategy control",
    click: [tab("strategy-control")],
    populated: [
      { selector: "#entry-strategies .strategy-control-item", min: 3 },
      { selector: "#exit-strategies .strategy-control-item", min: 2 },
    ],
    empty: [
      {
        selector: "#entry-strategies .state-view-empty",
        text: "No strategies defined",
      },
      { selector: "#exit-strategies .state-view-empty", text: "No strategies defined" },
    ],
  },
  {
    name: "strategies editor",
    click: [tab("strategies")],
    populated: [{ selector: "#strategy-list .strategy-item", min: 5 }],
    empty: [{ selector: "#strategy-list .state-view-empty", text: "No strategies yet" }],
    dialogs: [
      {
        trigger: "#create-strategy",
        dialog: "#create-strategy-modal",
        close: "#create-strategy-close",
      },
    ],
  },
  settings("stop-loss", "stop-loss-tab", 4),
  settings("trailing-stop", "trailing-stop-tab", 3),
  settings("roi", "roi-tab", 2),
  {
    ...settings("time-rules", "time-rules-tab", 4),
    populated: [
      ...settings("time-rules", "time-rules-tab", 4).populated,
      { selector: "#time-positions-status tbody .ti-row-cell", min: 3 },
    ],
    empty: [{ selector: "#time-positions-status .dt-state-cell > .state-view-empty", text: "No open positions" }],
  },
  settings("dca", "dca-tab", 5),
  settings(
    "general-settings",
    "general-settings-tab",
    5,
  ),
];
