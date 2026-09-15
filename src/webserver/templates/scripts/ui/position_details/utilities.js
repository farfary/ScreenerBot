/**
 * Utilities Mixin for the Position Details dialog: the position's lifecycle, the number
 * formatting shared by the header, summary, chart and activity, and guarded region painting.
 */
import * as Utils from "../../core/utils.js";

const LAMPORTS_PER_SOL = 1e9;

export function applyUtilitiesMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

  /** The position as last reported: full details once loaded, the opening row before. */
  proto._position = function () {
    return this.fullDetails?.position || this.positionData || null;
  };

  /**
   * `open` | `closed` | `archived`, exactly as the API reports it — null only before the
   * first response of a dialog opened without a row. Never inferred from `position_type`,
   * which is the trade side and reads "buy" for every position, closed ones included.
   */
  proto._status = function () {
    return this._position()?.status ?? null;
  };

  /**
   * Whether the round has finished trading. An archived position can still hold tokens (it
   * was only removed from the lists), so the headline figures follow this, not the status.
   */
  proto._isSettled = function () {
    const pos = this._position();
    return Boolean(pos && (pos.transaction_exit_verified || pos.exit_time != null));
  };

  /**
   * Repaint one region only when its markup changed. The dialog repaints on every poll, and
   * an unguarded innerHTML write resets scroll and wipes any text selection each tick.
   */
  proto._paintRegion = function (selector, html, afterPaint) {
    const el = this.dialogEl?.querySelector(selector);
    if (!el || this._renderKeys[selector] === html) return;
    this._renderKeys[selector] = html;
    el.innerHTML = html;
    afterPaint?.(el);
  };

  proto._formatPrice = function (price) {
    if (price === null || price === undefined) return "—";
    return Utils.formatPriceSubscript(price, { precision: 5 });
  };

  /**
   * A SOL amount at a precision that keeps small positions readable: the P&L of a 0.005 SOL
   * position is a few millionths of a SOL, which four flat decimals printed as 0.0000.
   */
  proto._formatSol = function (value, { sign = false, unit = true } = {}) {
    const num = Number(value);
    if (value === null || value === undefined || !Number.isFinite(num)) return "—";
    const abs = Math.abs(num);
    const suffix = unit ? " SOL" : "";
    const prefix = num < 0 ? "-" : sign && num > 0 ? "+" : "";
    if (abs > 0 && abs < 0.00000001) return `${prefix}<0.00000001${suffix}`;
    const decimals = abs === 0 || abs >= 0.01 ? 4 : abs >= 0.0001 ? 6 : 8;
    const text = Utils.formatSol(abs, { decimals, suffix: "" });
    // Zeros past the fourth decimal are noise: 0.005000 printed beside 0.0198 in one column.
    const trimmed = decimals > 4 ? text.replace(/(\.\d{4}\d*?)0+$/, "$1") : text;
    return `${prefix}${trimmed}${suffix}`;
  };

  proto._formatPct = function (value, decimals = 2) {
    const num = Number(value);
    if (value === null || value === undefined || !Number.isFinite(num)) return "—";
    return `${num > 0 ? "+" : ""}${Utils.formatNumber(num, decimals)}%`;
  };

  /** A signed SOL amount in USD at today's SOL price, or "" when that price is unknown. */
  proto._formatUsd = function (sol) {
    const solPrice = this.fullDetails?.sol_price_usd;
    const num = Number(sol);
    if (!solPrice || sol === null || sol === undefined || !Number.isFinite(num) || num === 0) {
      return "";
    }
    return `${num < 0 ? "-" : "+"}${Utils.formatCurrencyUSD(Math.abs(num * solPrice))}`;
  };

  proto._toneClass = function (value) {
    const num = Number(value);
    if (value === null || value === undefined || !Number.isFinite(num) || num === 0) return "";
    return num > 0 ? "pdd-positive" : "pdd-negative";
  };

  proto._plural = function (count, word) {
    return `${count} ${word}${count === 1 ? "" : "s"}`;
  };

  proto._lamportsToSol = function (lamports) {
    return lamports ? lamports / LAMPORTS_PER_SOL : 0;
  };

  /**
   * Token decimals. The position's own `token_decimals` comes from the stable on-chain
   * decimals column, so it survives market-data loss; `token_info` is null once full-token
   * assembly fails for a delisted token. Defaults to 9 (most SPL tokens).
   */
  proto._getDecimals = function () {
    return (
      this.fullDetails?.position?.token_decimals ?? this.fullDetails?.token_info?.decimals ?? 9
    );
  };

  /** Raw token amount (u64, smallest units) to whole tokens. */
  proto._toUiAmount = function (rawAmount) {
    if (!rawAmount) return 0;
    return rawAmount / Math.pow(10, this._getDecimals());
  };

  /** Remaining holding valued at the pool price, or null when either side is unknown. */
  proto._calculateCurrentValue = function (pos) {
    const currentPrice = pos?.current_price;
    const remainingTokens = pos?.remaining_token_amount;
    if (!currentPrice || !remainingTokens) return null;
    return currentPrice * this._toUiAmount(remainingTokens);
  };
}
