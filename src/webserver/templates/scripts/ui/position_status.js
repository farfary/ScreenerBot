// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Presentation for the lifecycle status of a position, in one place.
 *
 * Ids are the `status` values of the positions API (`open`, `closed`, `archived`).
 */

/** Message key of each status label. */
export const POSITION_STATUS_LABELS = Object.freeze({
  open: "positions-status-open",
  closed: "positions-status-closed",
  archived: "positions-status-archived",
});

/** Empty state of the positions table per status: the title, with the reason as `.message`. */
export const POSITION_EMPTY_LABELS = Object.freeze({
  open: "positions-open-empty",
  closed: "positions-closed-empty",
  archived: "positions-archived-empty",
});
