/**
 * Token Details Dialog - Utilities Mixin
 * Extracted from token_details_dialog.js to reduce file size
 * Helper functions for formatting and display
 */
import * as Hints from "../../core/hints.js";
import { HintTrigger } from "../hint_popover.js";

/**
 * Apply utilities mixin to TokenDetailsDialog class
 * @param {class} DialogClass - TokenDetailsDialog class
 */
export function applyUtilitiesMixin(DialogClass) {
  const proto = DialogClass.prototype;

  /**
   * Format address to short form (first 6 + last 6 chars)
   * @private
   * @param {string} address - Address to format
   * @returns {string} Formatted address
   */
  proto._formatShortAddress = function (address) {
    if (!address || address.length < 16) return address || "—";
    return `${address.substring(0, 6)}...${address.substring(address.length - 6)}`;
  };

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

    const sign = solNum >= 0 ? "+" : "-";
    const absVal = Math.abs(solNum).toFixed(4);
    let result = `${sign}${absVal} SOL`;

    if (Number.isFinite(percentNum)) {
      result += ` (${percentNum >= 0 ? "+" : ""}${percentNum.toFixed(2)}%)`;
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
    const sign = value >= 0 ? "+" : "";
    return `${sign}${value.toFixed(2)}%`;
  };

  /**
   * Get CSS class for change value (positive/negative)
   * @private
   * @param {number} value - Change value
   * @returns {string} CSS class name
   */
  proto._getChangeClass = function (value) {
    if (value === null || value === undefined) return "";
    return value >= 0 ? "positive" : "negative";
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
   * Get human-readable rejection reason label
   * @private
   * @param {string} reasonCode - Machine rejection code
   * @returns {string} Human-readable label
   */
  proto._getRejectionDisplayLabel = function (reasonCode) {
    const labels = {
      no_decimals: "No decimals in database",
      token_too_new: "Token too new",
      cooldown_filtered: "Cooldown filtered",
      dex_data_missing: "DexScreener data missing",
      gecko_data_missing: "GeckoTerminal data missing",
      rug_data_missing: "Rugcheck data missing",
      dex_empty_name: "Empty name",
      dex_empty_symbol: "Empty symbol",
      dex_empty_logo: "Empty logo URL",
      dex_empty_website: "Empty website URL",
      dex_txn_5m: "Low 5m transactions",
      dex_txn_1h: "Low 1h transactions",
      dex_zero_liq: "Zero liquidity",
      dex_liq_low: "Liquidity too low",
      dex_liq_high: "Liquidity too high",
      dex_mcap_low: "Market cap too low",
      dex_mcap_high: "Market cap too high",
      dex_vol_low: "Volume too low",
      dex_vol_missing: "Volume missing",
      dex_fdv_low: "FDV too low",
      dex_fdv_high: "FDV too high",
      dex_fdv_missing: "FDV missing",
      dex_vol5m_low: "5m volume too low",
      dex_vol5m_missing: "5m volume missing",
      dex_vol1h_low: "1h volume too low",
      dex_vol1h_missing: "1h volume missing",
      dex_vol6h_low: "6h volume too low",
      dex_vol6h_missing: "6h volume missing",
      dex_price_change_5m_low: "5m price change too low",
      dex_price_change_5m_high: "5m price change too high",
      dex_price_change_5m_missing: "5m price change missing",
      dex_price_change_low: "Price change too low",
      dex_price_change_high: "Price change too high",
      dex_price_change_missing: "Price change missing",
      dex_price_change_6h_low: "6h price change too low",
      dex_price_change_6h_high: "6h price change too high",
      dex_price_change_6h_missing: "6h price change missing",
      dex_price_change_24h_low: "24h price change too low",
      dex_price_change_24h_high: "24h price change too high",
      dex_price_change_24h_missing: "24h price change missing",
      gecko_liq_missing: "Liquidity missing",
      gecko_liq_low: "Liquidity too low",
      gecko_liq_high: "Liquidity too high",
      gecko_mcap_missing: "Market cap missing",
      gecko_mcap_low: "Market cap too low",
      gecko_mcap_high: "Market cap too high",
      gecko_vol5m_low: "5m volume too low",
      gecko_vol5m_missing: "5m volume missing",
      gecko_vol1h_low: "1h volume too low",
      gecko_vol1h_missing: "1h volume missing",
      gecko_vol24h_low: "24h volume too low",
      gecko_vol24h_missing: "24h volume missing",
      gecko_price_change_5m_low: "5m price change too low",
      gecko_price_change_5m_high: "5m price change too high",
      gecko_price_change_5m_missing: "5m price change missing",
      gecko_price_change_1h_low: "1h price change too low",
      gecko_price_change_1h_high: "1h price change too high",
      gecko_price_change_1h_missing: "1h price change missing",
      gecko_price_change_24h_low: "24h price change too low",
      gecko_price_change_24h_high: "24h price change too high",
      gecko_price_change_24h_missing: "24h price change missing",
      gecko_pool_count_low: "Pool count too low",
      gecko_pool_count_high: "Pool count too high",
      gecko_pool_count_missing: "Pool count missing",
      gecko_reserve_low: "Reserve too low",
      gecko_reserve_missing: "Reserve missing",
      rug_rugged: "Rugged token",
      rug_score: "Risk score too high",
      rug_level_danger: "Danger risk level",
      rug_mint_authority: "Mint authority present",
      rug_freeze_authority: "Freeze authority present",
      rug_top_holder: "Top holder % too high",
      rug_top3_holders: "Top 3 holders % too high",
      rug_min_holders: "Not enough holders",
      rug_insider_count: "Too many insider holders",
      rug_insider_pct: "Insider % too high",
      rug_creator_pct: "Creator balance too high",
      rug_transfer_fee_present: "Transfer fee present",
      rug_transfer_fee_high: "Transfer fee too high",
      rug_transfer_fee_missing: "Transfer fee data missing",
      rug_graph_insiders: "Graph insiders too high",
      rug_lp_providers_low: "LP providers too low",
      rug_lp_providers_missing: "LP providers missing",
      rug_lp_lock_low: "LP lock too low",
      rug_lp_lock_missing: "LP lock missing",
    };
    return labels[reasonCode] || reasonCode;
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
    this.chart = null;

    if (this.dialogEl) {
      this.dialogEl.remove();
      this.dialogEl = null;
    }
    this.tabHandlers.clear();
  };
}
