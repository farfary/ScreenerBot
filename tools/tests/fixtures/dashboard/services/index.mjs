// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the services dashboard page.

export const endpoints = [
  {
    method: "GET",
    path: "/api/services/overview",
    fixture: "services_overview.json",
    empty: "services_overview.empty.json",
    rust: "src/webserver/routes/services/types.rs::ServicesOverviewResponse",
  },
];

export const views = [
  {
    name: "services table",
    populated: [
      { selector: "#services-root tbody tr[data-row-id]", min: 19 },
      { selector: "#services-root tbody .badge.warning", min: 3 },
      { selector: "#services-root tbody .badge.error", min: 1 },
    ],
    empty: [
      {
        selector: "#services-root .dt-state-cell > .state-view-empty",
        text: "No services running",
      },
    ],
  },
];
