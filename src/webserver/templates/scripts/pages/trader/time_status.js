// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
//! The line an open position shows under the time exits, formatted from the watch `/api/trader/time-exits` computes for it.

import { formatPercentValue, formatUptime } from "../../core/format.js";

const span = (seconds) => formatUptime(seconds, { style: "trimmed" });

/** The loss rule's line: the time left before it sells, or the next check once due. */
function losingLine(watch, left) {
  const pnl = formatPercentValue(watch.pnl_pct, { decimals: 1, trim: true });
  return left > 0
    ? I18n.t("trader-time-status-losing", { pnl, remaining: span(left) })
    : I18n.t("trader-time-status-losing-due", { pnl });
}

/** The flat rule's line: how long the position has been held and the time left before it sells. */
function flatLine(heldSeconds, left) {
  const held = span(heldSeconds);
  return left > 0
    ? I18n.t("trader-time-status-flat", { held, remaining: span(left) })
    : I18n.t("trader-time-status-flat-due", { held });
}

/**
 * The line a position shows under the time exits. The route decides which rule applies
 * and when it is due; this only words it.
 *
 * @param {Object|undefined} watch The position's `time_exit` from `/api/trader/time-exits`;
 *   absent when the route has no watch for it.
 * @param {number|null} heldSeconds How long the position has been held.
 * @param {number} now Unix seconds.
 * @returns {string} Plain text.
 */
export function timeExitStatus(watch, heldSeconds, now) {
  switch (watch?.status) {
    case "not_managed":
      return I18n.t("trader-time-status-not-managed");
    case "off":
      return I18n.t("trader-time-status-off");
    case "clear":
      return I18n.t("trader-time-status-none");
    case "due": {
      const left = watch.due_at - now;
      if (watch.rule === "time_loss") return losingLine(watch, left);
      if (watch.rule === "time_stagnation") return flatLine(heldSeconds ?? 0, left);
      return "—";
    }
    default:
      return "—";
  }
}
