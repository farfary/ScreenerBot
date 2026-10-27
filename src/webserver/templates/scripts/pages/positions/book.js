// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
//! The Live / Paper book switch of the Positions table: where each book's rows come from, the one action a paper row offers, and the paper book's empty state.

import { TRADING_MODE_ICONS, tradingModeLabel } from "../../ui/trading_mode.js";

/** The books the table can show, one at a time; never mixed into one list. */
export const POSITION_BOOKS = Object.freeze(["live", "paper"]);

/**
 * The rows URL of `status` in `book`. The paper book keeps every round and archives
 * none, so its archived view has no source and reads empty.
 */
export function positionsUrl(book, status) {
  const query = `status=${encodeURIComponent(status)}&limit=0`;
  if (book !== "paper") return `/api/positions?${query}`;
  return status === "archived" ? null : `/api/positions/paper?${query}`;
}

/** The toolbar switch between the books; Paper carries its bare glyph inline. */
export function bookViews(book, onChange) {
  return {
    id: "book",
    value: book,
    ariaLabel: I18n.t("positions-book"),
    options: [
      { value: "live", label: tradingModeLabel("live") },
      { value: "paper", label: tradingModeLabel("paper"), icon: TRADING_MODE_ICONS.paper },
    ],
    onChange,
  };
}

/**
 * The actions of a paper row: an open round can be closed at a paper fill; it has no
 * manual add or sell, and a closed round has nothing to archive.
 */
export function paperActionCell(row, esc) {
  if (row?.id == null || row.transaction_exit_verified) return "";
  const title = esc(I18n.attr("positions-action-paper-close", "title"));
  const label = esc(I18n.attr("positions-action-paper-close", "aria-label"));
  return `<button class="btn-icon row-action" data-action="paper-close" data-id="${esc(
    String(row.id)
  )}" title="${title}" aria-label="${label}"><i class="icon-circle-x"></i></button>`;
}

/** The empty state of the paper book per status: title, reason, and where paper starts. */
export function paperEmptyState(status) {
  if (status === "archived") {
    return {
      title: I18n.t("positions-paper-archived-empty"),
      message: I18n.attr("positions-paper-archived-empty", "message"),
    };
  }
  return {
    title: I18n.t("positions-paper-empty"),
    message: I18n.attr("positions-paper-empty", "message"),
  };
}
