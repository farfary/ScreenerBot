// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Recorded API responses and view assertions for the home dashboard page.

export const endpoints = [
  {
    method: "GET",
    path: "/api/dashboard/home",
    fixture: "dashboard_home.json",
    empty: "dashboard_home.empty.json",
    rust: "src/webserver/routes/dashboard/types.rs::HomeDashboardResponse",
  },
  {
    // The page asks for the current UTC month; the recording is October 2026 and
    // the grid renders whichever month the response names.
    method: "GET",
    path: "/api/dashboard/portfolio-calendar",
    fixture: "portfolio_calendar.json",
    empty: "portfolio_calendar.empty.json",
    rust: "src/webserver/routes/dashboard/types.rs::PortfolioCalendarResponse",
    record: "/api/dashboard/portfolio-calendar?year=2026&month=10",
  },
  {
    // `{ success, boosted, jupiter_organic, jupiter_traded, dexscreener_trending }`,
    // each a list of cards.
    method: "GET",
    path: "/api/featured/all",
    fixture: "featured_all.json",
    empty: "featured_all.empty.json",
    rust: "src/webserver/routes/featured/types.rs::FeaturedCard",
  },
  {
    method: "GET",
    path: "/api/wallet/qr/{address}",
    fixture: "wallet_qr.json",
    rust: "src/webserver/routes/wallet/types.rs::WalletQrResponse",
    record: null,
  },
];

export const views = [
  {
    name: "portfolio overview",
    populated: [
      { selector: "#portfolioCalendarGrid .calendar-cell[data-date]", min: 6 },
      { selector: "#heroSparkLine[points*=',']", min: 1 },
      { selector: "#homeWalletIdentity:not([hidden])", min: 1 },
      { selector: "#featured-row .featured-row-card[data-mint]", min: 7 },
      { selector: "#featured-row .featured-row-card.boosted", min: 2 },
    ],
    empty: [
      { selector: "#featured-row .featured-row-skeleton-caption", text: "No featured tokens" },
      { selector: "#walletWorth", text: "^—$" },
      { selector: "#walletSol", text: "^—$" },
      { selector: "#walletTokenCount", text: "^—$" },
      { selector: "#calendarMonthTrades", text: "^0$" },
      { selector: "#positionsBest", text: "^—$" },
    ],
    dialogs: [
      {
        trigger: "#homeWalletQrButton",
        dialog: "#homeWalletQrPopover",
        close: "#homeWalletQrClose",
      },
      {
        trigger: "#featured-row .featured-row-view-all",
        dialog: "#featured-dialog .featured-container",
        close: "#featured-dialog .dialog-close",
      },
    ],
  },
];
