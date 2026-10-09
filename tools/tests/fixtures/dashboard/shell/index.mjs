// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions shared by every dashboard page (header, status bar, notifications, bootstrap).

export const endpoints = [
  {
    method: "GET",
    path: "/api/system/bootstrap",
    fixture: "system_bootstrap.json",
    rust: "src/webserver/routes/system/types.rs::BootStatusResponse",
  },
  {
    method: "GET",
    path: "/api/initialization/status",
    fixture: "initialization_status.json",
    rust: "src/webserver/routes/initialization/types.rs::InitializationStatusResponse",
  },
  {
    method: "GET",
    path: "/api/version",
    fixture: "version.json",
    rust: "src/webserver/routes/updates/types.rs::VersionResponse",
  },
  {
    method: "GET",
    path: "/api/config/gui",
    fixture: "config_gui.json",
    rust: "src/webserver/routes/config/types.rs::ConfigResponse<GuiConfig>",
  },
  {
    method: "GET",
    path: "/api/ui-state/all",
    fixture: "ui_state_all.json",
    rust: "src/webserver/routes/ui_state/types.rs::UiStateStore",
  },
  {
    method: "POST",
    path: "/api/ui-state/load",
    fixture: "ui_state_load.json",
    rust: "src/webserver/routes/ui_state/types.rs::LoadStateResponse",
    record: null,
  },
  {
    method: "POST",
    path: "/api/ui-state/save",
    fixture: "ui_state_save.json",
    rust: "src/webserver/routes/ui_state/types.rs::SaveStateResponse",
    record: null,
  },
  {
    method: "POST",
    path: "/api/ui-state/batch-save",
    fixture: "ui_state_batch_save.json",
    rust: "src/webserver/routes/ui_state/types.rs::BatchSaveResponse",
    record: null,
  },
  {
    method: "GET",
    path: "/api/actions/stream",
    fixture: "actions_stream.txt",
    rust: "src/actions/types.rs::ActionUpdate",
    contentType: "text/event-stream",
    record: null,
  },
  {
    method: "GET",
    path: "/api/actions/active",
    fixture: "actions_active.json",
    rust: "src/webserver/routes/actions/types.rs::ActiveActionsResponse",
  },
  {
    method: "GET",
    path: "/api/actions/history",
    fixture: "actions_history.json",
    empty: "actions_history.empty.json",
    rust: "src/webserver/routes/actions/types.rs::ActionHistoryResponse",
    record: "/api/actions/history?limit=30&offset=0",
  },
  {
    method: "GET",
    path: "/api/header/metrics",
    fixture: "header_metrics.json",
    empty: "header_metrics.empty.json",
    rust: "src/webserver/routes/header/types.rs::HeaderMetricsResponse",
  },
  {
    method: "GET",
    path: "/api/status",
    fixture: "status.json",
    rust: "src/webserver/snapshot/types.rs::StatusSnapshot",
  },
  {
    method: "GET",
    path: "/api/health",
    fixture: "health.json",
    rust: "src/webserver/routes/status/types.rs::HealthResponse",
  },
  {
    method: "GET",
    path: "/api/lockscreen/status",
    fixture: "lockscreen_status.json",
    rust: "src/webserver/routes/lockscreen/types.rs::LockscreenStatusResponse",
  },
  {
    method: "GET",
    path: "/api/updates/status",
    fixture: "updates_status.json",
    rust: "src/webserver/routes/updates/types.rs::UpdateStatusResponse",
  },
  {
    method: "GET",
    path: "/api/agent-control/approvals",
    fixture: "agent_control_approvals.json",
    rust: "src/agent_control/approvals.rs::Vec<PendingApproval>",
  },
  {
    method: "POST",
    path: "/api/system/client-ready",
    fixture: "system_client_ready.json",
    rust: "src/webserver/routes/system/types.rs::ClientReadyResponse",
    record: null,
  },
  {
    method: "POST",
    path: "/api/actions/read-all",
    fixture: "actions_read_all.json",
    rust: "src/webserver/routes/actions/types.rs::ActionMutationResponse",
    record: null,
  },
  {
    method: "GET",
    path: "/api/system/paths",
    fixture: "system_paths.json",
    rust: "src/webserver/routes/system/types.rs::PathsResponse",
  },
  {
    method: "GET",
    path: "/api/i18n/locales",
    fixture: "i18n_locales.json",
    rust: "src/webserver/routes/i18n/types.rs::LocalesResponse",
  },
];

export const views = [];
