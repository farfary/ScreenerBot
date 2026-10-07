// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the config dashboard page.

export const endpoints = [
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
    path: "/api/telegram/status",
    fixture: "telegram_status.json",
    rust: "src/webserver/routes/telegram/types.rs::TelegramStatusResponse",
  },
  {
    // `{ enabled, configured, commands_require_2fa }` built inline by the handler.
    method: "GET",
    path: "/api/telegram/totp/status",
    fixture: "telegram_totp_status.json",
    rust: "src/webserver/routes/telegram/handlers.rs::get_totp_status",
  },
];

const sectionItem = (icon) => `#configSectionsList .config-section-item:has(i.${icon})`;

export const views = [
  {
    name: "trader section",
    populated: [
      { selector: "#configSectionsList .config-section-item", min: 26 },
      { selector: "#configSectionsList .config-section-item.active", min: 1 },
      { selector: "#configCategories .config-category", min: 3 },
      { selector: "#configCategories .config-field", min: 5 },
    ],
    dialogs: [
      {
        trigger: "#configExportButton",
        dialog: ".config-export-dialog",
        close: ".config-export-dialog .config-dialog-close",
      },
      {
        trigger: "#configImportButton",
        dialog: ".config-import-dialog",
        close: ".config-import-dialog .config-dialog-close",
        defect: "The import dialog close button has no click handler on its first step",
      },
      {
        trigger: "#configResetButton",
        dialog: ".confirmation-dialog",
        close: ".confirmation-dialog [data-action='cancel']",
      },
    ],
  },
  {
    name: "filtering section",
    click: [sectionItem("icon-target")],
    populated: [
      { selector: `${sectionItem("icon-target")}.active`, min: 1 },
      { selector: "#configCategories .config-field", min: 5 },
    ],
  },
  {
    name: "telegram section",
    click: [sectionItem("icon-send")],
    populated: [
      { selector: `${sectionItem("icon-send")}.active`, min: 1 },
      { selector: "#configCategories #telegram-test-btn", min: 1 },
      { selector: "#configCategories .telegram-auth-section", min: 1 },
    ],
  },
  {
    name: "chains section",
    click: [sectionItem("icon-blocks")],
    populated: [
      { selector: `${sectionItem("icon-blocks")}.active`, min: 1 },
      { selector: "#configCategories .config-field", min: 2 },
    ],
  },
];
