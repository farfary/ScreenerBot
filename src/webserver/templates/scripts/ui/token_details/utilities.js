// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Token Details Dialog - Utilities Mixin
 * Extracted from token_details_dialog.js to reduce file size
 * Helper functions for formatting and display
 */
import * as Hints from "../../core/hints.js";
import { formatPercentValue, formatSignedSol } from "../../core/format.js";
import { HintTrigger } from "../hint_popover.js";

/**
 * Apply utilities mixin to TokenDetailsDialog class
 * @param {class} DialogClass - TokenDetailsDialog class
 */
export function applyUtilitiesMixin(DialogClass) {
  const proto = DialogClass.prototype;

  /**
   * Format PnL with both SOL value and percentage
   * @private
   * @param {number} solValue - SOL value
   * @param {number} percentValue - Percentage value
   * @returns {string} Formatted PnL string
   */
  proto._formatPnLWithPercent = function (solValue, percentValue) {
    const solNum = parseFloat(solValue);
    const percentNum = parseFloat(percentValue);

    if (!Number.isFinite(solNum)) return "—";

    let result = formatSignedSol(solNum, { decimals: 4 });

    if (Number.isFinite(percentNum)) {
      result += ` (${formatPercentValue(percentNum, { decimals: 2, signZero: true })})`;
    }

    return result;
  };

  /**
   * Format percentage change with sign
   * @private
   * @param {number} value - Change value
   * @returns {string} Formatted change
   */
  proto._formatChange = function (value) {
    if (value === null || value === undefined) return "—";
    return formatPercentValue(value, { decimals: 2, signZero: true });
  };

  /**
   * Render a hint trigger for card headers
   * @private
   * @param {string} hintKey - Hint key identifier
   * @returns {string} HTML string for hint trigger
   */
  proto._renderHintTrigger = function (hintKey) {
    const hint = Hints.getHint(hintKey);
    if (!hint) return "";
    return HintTrigger.render(hint, hintKey, { size: "sm", position: "bottom" });
  };

  /**
   * Escape HTML to prevent XSS
   * @private
   * @param {string} text - Text to escape
   * @returns {string} Escaped HTML
   */
  proto._escapeHtml = function (text) {
    if (!text) return "";
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  };

  /**
   * Cleanup dialog resources
   */
  proto.destroy = function () {
    this._stopPolling();
    this._stopChartPolling();

    if (this._escapeHandler) {
      document.removeEventListener("keydown", this._escapeHandler);
    }

    // Clean up theme observer
    if (this._themeObserver) {
      this._themeObserver.disconnect();
      this._themeObserver = null;
    }

    // Clean up chart resize observer
    if (this.chartResizeObserver) {
      this.chartResizeObserver.disconnect();
      this.chartResizeObserver = null;
    }

    // Clean up advanced chart
    if (this.advancedChart) {
      this.advancedChart.destroy();
      this.advancedChart = null;
    }
    this._chartSeries = null;
    this.chart = null;

    if (this.dialogEl) {
      this.dialogEl.remove();
      this.dialogEl = null;
    }
    this.tabHandlers.clear();
  };
}
