// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Presentation for a transaction's wallet-relative direction, shared by the
 * transaction list, the details dialog and the position activity feed.
 */
import { escapeHtml } from "../core/utils.js";

// `TransactionDirection` (src/transactions/types.rs) as sent by the transaction and activity endpoints.
// A direction names what moved ("Tokens in", "SOL out"), so it never reads as a
// contradiction of the SOL delta beside it.
export const DIRECTION_LABELS = Object.freeze({
  TokensIn: "transactions-direction-tokens-in",
  TokensOut: "transactions-direction-tokens-out",
  SolIn: "transactions-direction-sol-in",
  SolOut: "transactions-direction-sol-out",
  Internal: "transactions-direction-internal",
  Unknown: "transactions-direction-unknown",
});

// A direction is a neutral fact about the wallet, not an outcome: every direction
// shares the neutral badge and differs only by its glyph, so a normal buy (SOL out)
// never reads as an error beside its status.
const DIRECTION_GLYPHS = {
  TokensIn: "↓ ",
  TokensOut: "↑ ",
  SolIn: "↓ ",
  SolOut: "↑ ",
  Internal: "⟲ ",
  // Only rows written before the direction named its subject can still be Unknown;
  // the reclassification sweep clears them.
  Unknown: "",
};

/** Directions in the order the transaction list filter offers them. */
export const DIRECTION_FILTER_VALUES = Object.freeze([
  "TokensIn",
  "TokensOut",
  "SolIn",
  "SolOut",
  "Internal",
]);

/** "in" when something arrived in the wallet, "out" when something left, else null. */
export function directionFlow(direction) {
  if (direction === "TokensIn" || direction === "SolIn") return "in";
  if (direction === "TokensOut" || direction === "SolOut") return "out";
  return null;
}

export function directionLabel(direction) {
  return I18n.label(DIRECTION_LABELS, direction);
}

/** Badge markup for a direction; `empty` is returned when there is none. */
export function directionBadge(direction, empty = "") {
  if (!direction) return empty;
  if (!Object.hasOwn(DIRECTION_GLYPHS, direction)) return escapeHtml(direction);
  return `<span class="badge secondary">${DIRECTION_GLYPHS[direction]}${escapeHtml(directionLabel(direction))}</span>`;
}
