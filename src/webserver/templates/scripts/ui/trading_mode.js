// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Labels for the paper or live mode of a copy task or a strategy, in one place.
 *
 * Ids are the serialized values of `TradingMode` (src/trader/paper/mode.rs).
 */

/** Message key of each mode's label; keep in step with `TradingMode`. */
export const TRADING_MODE_LABELS = Object.freeze({
  paper: "common-mode-paper",
  live: "common-mode-live",
});

export const tradingModeLabel = (mode) => I18n.label(TRADING_MODE_LABELS, mode);
