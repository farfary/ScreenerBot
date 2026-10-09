// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the events dashboard page.

export const endpoints = [
  {
    method: "GET",
    path: "/api/events/head",
    fixture: "events_head.json",
    empty: "events_head.empty.json",
    rust: "src/webserver/routes/events/types.rs::EventsListResponse",
    record: "/api/events/head?limit=100",
  },
  {
    // The poller asks for rows newer than the newest one shown; none arrived.
    method: "GET",
    path: "/api/events/since",
    fixture: "events_since.json",
    rust: "src/webserver/routes/events/types.rs::EventsListResponse",
    record: null,
  },
  {
    method: "GET",
    path: "/api/events/before",
    fixture: "events_before.json",
    rust: "src/webserver/routes/events/types.rs::EventsListResponse",
    record: null,
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
    name: "event log",
    populated: [
      { selector: "#events-root tbody tr[data-row-id]", min: 22 },
      { selector: "#events-root tbody .badge.error", min: 2 },
      { selector: "#events-root tbody .badge.warning", min: 4 },
    ],
    empty: [{ selector: "#events-root .dt-state-cell > .state-view-empty", text: "No events yet" }],
    dialogs: [
      {
        trigger: "#events-root tbody tr[data-row-id]",
        dialog: ".events-dialog-overlay.is-visible",
        close: ".events-dialog-overlay.is-visible .modal-close",
      },
    ],
  },
];
