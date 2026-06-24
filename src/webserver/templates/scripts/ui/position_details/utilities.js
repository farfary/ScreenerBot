/**
 * Utilities Mixin for Position Details Dialog
 * Adds utility methods to PositionDetailsDialog class
 */
import * as Utils from "../../core/utils.js";

/**
 * Apply utility methods to PositionDetailsDialog prototype
 * @param {class} PositionDetailsDialog - The PositionDetailsDialog class
 */
export function applyUtilitiesMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

  // ===========================================================================
  // UTILITY METHODS
  // ===========================================================================

  /**
   * Format price with appropriate precision
   * Uses subscript notation for very small prices
   */
  proto._formatPrice = function (price) {
    if (price === null || price === undefined) return "—";
    return Utils.formatPriceSubscript(price, { precision: 5 });
  };

  /**
   * Format transaction type for display
   */
  proto._formatTransactionType = function (type) {
    if (!type) return "Unknown";

    const typeMap = {
      entry: "Entry",
      exit: "Exit",
      buy: "Buy",
      sell: "Sell",
      swap: "Swap",
      transfer: "Transfer",
      unknown: "Unknown",
    };

    return typeMap[type.toLowerCase()] || Utils.escapeHtml(type);
  };

  /**
   * Get token decimals from fullDetails or position data
   * Defaults to 9 (most Solana SPL tokens) if not available
   */
  proto._getDecimals = function () {
    // Prefer the position summary's token_decimals (sourced from the stable
    // on-chain decimals column, so it survives market-data loss) over token_info,
    // which is null when full-token assembly fails for a delisted/rugged token.
    return (
      this.fullDetails?.position?.token_decimals ??
      this.fullDetails?.token_info?.decimals ??
      9
    );
  };

  /**
   * Convert raw token amount (u64) to UI amount using decimals
   * @param {number} rawAmount - Raw token amount in smallest units
   * @returns {number} UI amount (human-readable)
   */
  proto._toUiAmount = function (rawAmount) {
    if (!rawAmount) return 0;
    const decimals = this._getDecimals();
    return rawAmount / Math.pow(10, decimals);
  };

  /**
   * Calculate current position value in SOL
   * @param {Object} pos - Position object with current_price and remaining_token_amount
   * @returns {number|null} Current value in SOL or null if not calculable
   */
  proto._calculateCurrentValue = function (pos) {
    const currentPrice = pos?.current_price;
    const remainingTokens = pos?.remaining_token_amount;
    if (!currentPrice || !remainingTokens) return null;
    const uiAmount = this._toUiAmount(remainingTokens);
    return currentPrice * uiAmount;
  };
}
