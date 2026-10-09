// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the tools dashboard page.

export const endpoints = [
  {
    method: "GET",
    path: "/api/features",
    fixture: "features.json",
    rust: "src/features/types.rs::Features",
  },
  {
    method: "GET",
    path: "/api/tools/ata-scan",
    fixture: "ata_scan.json",
    empty: "ata_scan.empty.json",
    rust: "src/webserver/routes/tools/types.rs::AtaScanResponse",
  },
];

export const views = [
  {
    name: "tool navigation",
    click: ['#tools-nav .nav-item[data-tool="wallet-cleanup"]'],
    populated: [
      { selector: '#tools-nav .nav-item.active[data-tool="wallet-cleanup"][data-status="ready"]' },
      { selector: "#tools-nav .nav-item.feature-coming-soon .status-badge.coming-soon", min: 10 },
      { selector: "#tool-actions #scan-atas-btn" },
    ],
    empty: [{ selector: "#ata-list .state-view", text: 'Click "Scan Wallet" to find empty ATAs' }],
    dialogs: [
      { trigger: "#tool-hint .hint-trigger", dialog: ".hint-popover", close: ".hint-popover__close" },
    ],
  },
  {
    name: "wallet cleanup scan",
    click: ['#tools-nav .nav-item[data-tool="wallet-cleanup"]', "#scan-atas-btn"],
    populated: [
      { selector: "#ata-list .state-view-success" },
      { selector: "#tool-actions #cleanup-atas-btn:not([disabled])" },
    ],
    empty: [{ selector: "#ata-list .state-view", text: "No empty ATAs found - wallet is clean!" }],
  },
];
