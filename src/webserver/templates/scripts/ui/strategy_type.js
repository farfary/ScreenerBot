// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Label, glyph and marker of a strategy type, in one place.
 *
 * Ids are the `strategy_type` values of the strategies API.
 */

/** Message key of each strategy type's label. */
export const STRATEGY_TYPE_LABELS = Object.freeze({
  ENTRY: "strategies-type-entry",
  EXIT: "strategies-type-exit",
});

/** The bare glyph each strategy type carries beside its label. */
export const STRATEGY_TYPE_ICONS = Object.freeze({
  ENTRY: "icon-log-in",
  EXIT: "icon-log-out",
});

/** The type's label as plain text. */
export const strategyTypeLabel = (type) => I18n.label(STRATEGY_TYPE_LABELS, type);

/**
 * The type as a standalone marker: its bare glyph in the kind colour inline with its
 * label. `escapeHtml` escapes the type id and the label.
 */
export function strategyTypeMarker(type, escapeHtml) {
  const icon = Object.hasOwn(STRATEGY_TYPE_ICONS, type) ? STRATEGY_TYPE_ICONS[type] : "";
  const glyph = icon ? `<i class="${icon}" aria-hidden="true"></i>` : "";
  return `<span class="strategy-type-badge ${escapeHtml(String(type).toLowerCase())}">${glyph}<span>${escapeHtml(
    strategyTypeLabel(type)
  )}</span></span>`;
}
