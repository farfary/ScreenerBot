// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Presentation for the reasons a new entry was blocked.
 *
 * Ids are the `kind` values serialized by `EntryBlock` (src/trader/admission.rs), shared
 * by every source that enters a position: the Auto Trader, copy trading and manual buys.
 */

/** Message key of each block's label; keep in step with `EntryBlock`. */
export const ENTRY_BLOCK_LABELS = Object.freeze({
  force_stopped: "trade-entry-block-force-stopped",
  loss_limit: "trade-entry-block-loss-limit",
  connectivity: "trade-entry-block-connectivity",
  storage_unavailable: "trade-entry-block-storage-unavailable",
  position_limit: "trade-entry-block-position-limit",
  already_open: "trade-entry-block-already-open",
  reentry_cooldown: "trade-entry-block-reentry-cooldown",
  open_cooldown: "trade-entry-block-open-cooldown",
  entry_reserved: "trade-entry-block-entry-reserved",
  blacklisted: "trade-entry-block-blacklisted",
  paper_balance_exhausted: "trade-entry-block-paper-balance-exhausted",
  check_failed: "trade-entry-block-check-failed",
});

/** Label of an entry block `kind`. */
export function entryBlockText(kind) {
  return I18n.label(ENTRY_BLOCK_LABELS, kind);
}
