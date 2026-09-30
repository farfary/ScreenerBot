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

const DIRECTION_STYLES = {
  Incoming: { variant: "success", glyph: "↓ " },
  Outgoing: { variant: "error", glyph: "↑ " },
  Internal: { variant: "secondary", glyph: "⟲ " },
  // Only rows written before the wallet-relative direction landed can still be
  // Unknown; the reclassification sweep clears them.
  Unknown: { variant: "secondary", glyph: "" },
};

export function directionLabel(direction) {
  return I18n.label(DIRECTION_LABELS, direction);
}

/** Badge markup for a direction; `empty` is returned when there is none. */
export function directionBadge(direction, empty = "") {
  if (!direction) return empty;
  const style = DIRECTION_STYLES[direction];
  if (!style) return escapeHtml(direction);
  return `<span class="badge ${style.variant}">${style.glyph}${escapeHtml(directionLabel(direction))}</span>`;
}
