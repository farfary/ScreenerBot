// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Labels and the marker for the paper or live mode of a copy task or a strategy, in one place.
 *
 * Ids are the serialized values of `TradingMode` (src/trader/paper/mode.rs).
 */

/** Message key of each mode's label; keep in step with `TradingMode`. */
export const TRADING_MODE_LABELS = Object.freeze({
  paper: "common-mode-paper",
  live: "common-mode-live",
});

/** The bare glyph a mode carries beside its label; Live carries none. */
export const TRADING_MODE_ICONS = Object.freeze({ paper: "icon-test-tube-diagonal" });

/** The mode's label as plain text, for a sentence or a field value. */
export const tradingModeLabel = (mode) => I18n.label(TRADING_MODE_LABELS, mode);

/**
 * The mode as a standalone marker: Paper is its bare glyph inline with its label, Live is
 * the label alone. `escapeHtml` escapes the mode id and the label.
 */
export function tradingModeMarker(mode, escapeHtml) {
  const icon = Object.hasOwn(TRADING_MODE_ICONS, mode) ? TRADING_MODE_ICONS[mode] : "";
  const glyph = icon ? `<i class="${icon}" aria-hidden="true"></i>` : "";
  return `<span class="mode-marker" data-mode="${escapeHtml(String(mode))}">${glyph}<span>${escapeHtml(
    tradingModeLabel(mode)
  )}</span></span>`;
}
