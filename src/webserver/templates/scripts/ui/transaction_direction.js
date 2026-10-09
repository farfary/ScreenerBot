// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Presentation for a transaction's wallet-relative direction, shared by the
 * transaction list, the details dialog and the position activity feed.
 */
import { escapeHtml } from "../core/utils.js";

// `TransactionDirection` (src/transactions/types.rs) as sent by the transaction and activity endpoints.
export const DIRECTION_LABELS = Object.freeze({
  Incoming: "transactions-direction-incoming",
  Outgoing: "transactions-direction-outgoing",
  Internal: "transactions-direction-internal",
  Unknown: "transactions-direction-unknown",
});

// A direction is a neutral fact about the wallet, not an outcome: every direction
// shares the neutral badge and differs only by its glyph, so a normal buy (SOL out)
// never reads as an error beside its status.
const DIRECTION_GLYPHS = {
  Incoming: "↓ ",
  Outgoing: "↑ ",
  Internal: "⟲ ",
  // Only rows written before the wallet-relative direction landed can still be
  // Unknown; the reclassification sweep clears them.
  Unknown: "",
};

export function directionLabel(direction) {
  return I18n.label(DIRECTION_LABELS, direction);
}

/** Badge markup for a direction; `empty` is returned when there is none. */
export function directionBadge(direction, empty = "") {
  if (!direction) return empty;
  if (!Object.hasOwn(DIRECTION_GLYPHS, direction)) return escapeHtml(direction);
  return `<span class="badge secondary">${DIRECTION_GLYPHS[direction]}${escapeHtml(directionLabel(direction))}</span>`;
}
