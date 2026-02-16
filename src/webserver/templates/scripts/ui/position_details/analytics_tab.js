/**
 * Analytics Tab Mixin for Position Details Dialog
 * Adds analytics tab rendering functionality to PositionDetailsDialog class
 */
import * as Utils from "../../core/utils.js";

/**
 * Apply analytics tab methods to PositionDetailsDialog prototype
 * @param {class} PositionDetailsDialog - The PositionDetailsDialog class
 */
export function applyAnalyticsTabMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

  // ===========================================================================
  // ANALYTICS TAB
  // ===========================================================================

  proto._renderAnalyticsTab = function (content) {
    const pos = this.fullDetails?.position;
    const entries = this.fullDetails?.entries || [];
    const exits = this.fullDetails?.exits || [];
    const positionAge = this.fullDetails?.position_age_seconds;
    const solPriceUsd = this.fullDetails?.sol_price_usd;

    if (!pos) {
      content.innerHTML = '<div class="pdd-empty-state">No analytics data available</div>';
      return;
    }

    const isOpen = pos.position_type !== "closed";

    content.innerHTML = `
      <div class="pdd-analytics-tab">
        ${this._buildPerformanceSummary(pos, isOpen, positionAge, solPriceUsd)}
        <div class="pdd-analytics-grid">
          ${this._buildPriceAnalysis(pos)}
          ${this._buildFeeAnalysis(pos, entries, exits)}
        </div>
        ${this._buildDcaSummary(pos, entries)}
        ${this._buildExitSummary(pos, exits, solPriceUsd)}
      </div>
    `;
  };

  /**
   * Build performance summary card for analytics tab
   */
  proto._buildPerformanceSummary = function (pos, isOpen, positionAge, solPriceUsd) {
    // Calculate ROI using proper value conversion
    const totalInvested = pos.total_size_sol || 0;
    const currentValue = this._calculateCurrentValue(pos) || 0;
    const solReceived = pos.sol_received || 0;
    const totalValue = isOpen ? currentValue : solReceived;
    const roi = totalInvested > 0 ? ((totalValue - totalInvested) / totalInvested) * 100 : 0;
    const roiClass = roi >= 0 ? "pdd-positive" : "pdd-negative";
    const roiSign = roi >= 0 ? "+" : "";

    // Format duration
    const durationFormatted = positionAge ? Utils.formatUptime(positionAge) : "—";

    // Calculate USD values if available
    let investedUsd = "";
    let valueUsd = "";
    if (solPriceUsd && totalInvested > 0) {
      investedUsd = `<span class="pdd-metric-usd">${Utils.formatCurrencyUSD(totalInvested * solPriceUsd)}</span>`;
    }
    if (solPriceUsd && totalValue > 0) {
      valueUsd = `<span class="pdd-metric-usd">${Utils.formatCurrencyUSD(totalValue * solPriceUsd)}</span>`;
    }

    return `
      <div class="pdd-analytics-card pdd-perf-summary">
        <h4><i class="icon-activity"></i> Performance Summary</h4>
        <div class="pdd-perf-metrics">
          <div class="pdd-perf-metric">
            <span class="pdd-metric-label">ROI</span>
            <span class="pdd-metric-value ${roiClass}">${roiSign}${Utils.formatNumber(roi, 2)}%</span>
          </div>
          <div class="pdd-perf-metric">
            <span class="pdd-metric-label">Duration</span>
            <span class="pdd-metric-value">${durationFormatted}</span>
          </div>
          <div class="pdd-perf-metric">
            <span class="pdd-metric-label">Cost Basis</span>
            <span class="pdd-metric-value">${Utils.formatSol(totalInvested, { decimals: 4 })}</span>
            ${investedUsd}
          </div>
          <div class="pdd-perf-metric">
            <span class="pdd-metric-label">${isOpen ? "Current Value" : "Final Value"}</span>
            <span class="pdd-metric-value">${Utils.formatSol(totalValue, { decimals: 4 })}</span>
            ${valueUsd}
          </div>
        </div>
      </div>
    `;
  };

  /**
   * Build price analysis card for analytics tab
   */
  proto._buildPriceAnalysis = function (pos) {
    const entryPrice = pos.average_entry_price || pos.entry_price;
    const currentPrice = pos.current_price;
    const peakPrice = pos.price_highest;
    const lowPrice = pos.price_lowest;

    // Calculate peak from entry percentage
    let peakFromEntry = null;
    if (peakPrice && entryPrice && entryPrice > 0) {
      peakFromEntry = ((peakPrice - entryPrice) / entryPrice) * 100;
    }

    // Calculate low from entry percentage
    let lowFromEntry = null;
    if (lowPrice && entryPrice && entryPrice > 0) {
      lowFromEntry = ((lowPrice - entryPrice) / entryPrice) * 100;
    }

    // Calculate drawdown (current vs peak)
    let drawdown = null;
    if (currentPrice && peakPrice && peakPrice > 0) {
      drawdown = ((currentPrice - peakPrice) / peakPrice) * 100;
    }

    const formatPriceWithPct = (price, pct, showPositive = true) => {
      if (price === null || price === undefined) return "—";
      const priceStr = this._formatPrice(price) + " SOL";
      if (pct === null || pct === undefined) return priceStr;
      const pctClass = pct >= 0 ? "pdd-positive" : "pdd-negative";
      const sign = pct >= 0 && showPositive ? "+" : "";
      return `${priceStr} <span class="${pctClass}">(${sign}${Utils.formatNumber(pct, 1)}%)</span>`;
    };

    return `
      <div class="pdd-analytics-card pdd-price-analysis">
        <h4><i class="icon-chart-bar"></i> Price Analysis</h4>
        <div class="pdd-analysis-rows">
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Entry Price</span>
            <span class="pdd-row-value">${entryPrice ? this._formatPrice(entryPrice) + " SOL" : "—"}</span>
          </div>
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Current Price</span>
            <span class="pdd-row-value">${currentPrice ? this._formatPrice(currentPrice) + " SOL" : "—"}</span>
          </div>
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Peak Price</span>
            <span class="pdd-row-value">${formatPriceWithPct(peakPrice, peakFromEntry)}</span>
          </div>
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Low Price</span>
            <span class="pdd-row-value">${formatPriceWithPct(lowPrice, lowFromEntry, false)}</span>
          </div>
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Current vs Peak</span>
            <span class="pdd-row-value ${drawdown !== null && drawdown < 0 ? "pdd-negative" : ""}">${drawdown !== null ? Utils.formatNumber(drawdown, 1) + "%" : "—"}</span>
          </div>
        </div>
      </div>
    `;
  };

  /**
   * Build DCA summary card for analytics tab (only if multiple entries)
   */
  proto._buildDcaSummary = function (pos, entries) {
    const dcaCount = pos.dca_count || entries.length || 0;

    // Only show if there are multiple entries (DCA)
    if (dcaCount <= 1 && entries.length <= 1) {
      return "";
    }

    const avgEntry = pos.average_entry_price;

    // Calculate entry price range from entries array
    let minEntry = null;
    let maxEntry = null;
    if (entries.length > 0) {
      const prices = entries.map((e) => e.price).filter((p) => p != null && p > 0);
      if (prices.length > 0) {
        minEntry = Math.min(...prices);
        maxEntry = Math.max(...prices);
      }
    }

    // Calculate total tokens from all entries
    const totalTokensAcquired = entries.reduce((sum, e) => sum + (e.amount || 0), 0);
    const totalSolSpent = entries.reduce((sum, e) => sum + (e.sol_spent || 0), 0);

    return `
      <div class="pdd-analytics-card pdd-dca-summary">
        <h4><i class="icon-layers"></i> DCA Summary</h4>
        <div class="pdd-analysis-rows">
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Number of Entries</span>
            <span class="pdd-row-value">${dcaCount}</span>
          </div>
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Average Entry Price</span>
            <span class="pdd-row-value">${avgEntry ? this._formatPrice(avgEntry) + " SOL" : "—"}</span>
          </div>
          ${
            minEntry !== null && maxEntry !== null
              ? `
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Entry Price Range</span>
            <span class="pdd-row-value">${this._formatPrice(minEntry)} - ${this._formatPrice(maxEntry)} SOL</span>
          </div>
          `
              : ""
          }
          ${
            totalTokensAcquired > 0
              ? `
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Total Tokens Acquired</span>
            <span class="pdd-row-value">${Utils.formatCompactNumber(totalTokensAcquired)}</span>
          </div>
          `
              : ""
          }
          ${
            totalSolSpent > 0
              ? `
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Total SOL Spent</span>
            <span class="pdd-row-value">${Utils.formatSol(totalSolSpent, { decimals: 4 })}</span>
          </div>
          `
              : ""
          }
        </div>
      </div>
    `;
  };

  /**
   * Build exit summary card for analytics tab (only if partial exits occurred)
   */
  proto._buildExitSummary = function (pos, exits, solPriceUsd) {
    const exitCount = pos.partial_exit_count || exits.length || 0;

    // Only show if there are exits
    if (exitCount === 0 && exits.length === 0) {
      return "";
    }

    const tokensSold =
      pos.total_exited_amount || exits.reduce((sum, e) => sum + (e.amount || 0), 0);
    const originalTokens = pos.token_amount || 0;
    const pctSold = originalTokens > 0 ? (tokensSold / originalTokens) * 100 : 0;
    const solReceived =
      pos.sol_received || exits.reduce((sum, e) => sum + (e.sol_received || 0), 0);

    // Realized P&L
    const realizedPnl = pos.pnl;
    const realizedPnlPct = pos.pnl_percent;
    const pnlClass = realizedPnl >= 0 ? "pdd-positive" : "pdd-negative";
    const pnlSign = realizedPnl >= 0 ? "+" : "";

    // USD value if available
    let solReceivedUsd = "";
    if (solPriceUsd && solReceived > 0) {
      solReceivedUsd = `<span class="pdd-row-usd">(${Utils.formatCurrencyUSD(solReceived * solPriceUsd)})</span>`;
    }

    return `
      <div class="pdd-analytics-card pdd-exit-summary">
        <h4><i class="icon-log-out"></i> Exit Summary</h4>
        <div class="pdd-analysis-rows">
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Partial Exits</span>
            <span class="pdd-row-value">${exitCount}</span>
          </div>
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Tokens Sold</span>
            <span class="pdd-row-value">${Utils.formatCompactNumber(tokensSold)} <span class="pdd-row-pct">(${Utils.formatNumber(pctSold, 1)}%)</span></span>
          </div>
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">SOL Recovered</span>
            <span class="pdd-row-value">${Utils.formatSol(solReceived, { decimals: 4 })} ${solReceivedUsd}</span>
          </div>
          ${
            realizedPnl !== undefined && realizedPnl !== null
              ? `
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Realized P&L</span>
            <span class="pdd-row-value ${pnlClass}">${pnlSign}${Utils.formatSol(realizedPnl, { decimals: 4, suffix: "" })} SOL ${realizedPnlPct !== undefined ? `<span class="pdd-row-pct">(${pnlSign}${Utils.formatNumber(realizedPnlPct, 2)}%)</span>` : ""}</span>
          </div>
          `
              : ""
          }
        </div>
      </div>
    `;
  };

  /**
   * Build fee analysis card for analytics tab
   */
  proto._buildFeeAnalysis = function (pos, entries, exits) {
    // Calculate entry fees from entries array or position summary
    let entryFees = 0;
    if (pos.entry_fee_lamports) {
      entryFees = pos.entry_fee_lamports / 1e9;
    } else if (entries.length > 0) {
      entryFees = entries.reduce((sum, e) => sum + (e.fee_lamports || 0) / 1e9, 0);
    }

    // Calculate exit fees from exits array or position summary
    let exitFees = 0;
    if (pos.exit_fee_lamports) {
      exitFees = pos.exit_fee_lamports / 1e9;
    } else if (exits.length > 0) {
      exitFees = exits.reduce((sum, e) => sum + (e.fee_lamports || 0) / 1e9, 0);
    }

    const totalFees = entryFees + exitFees;
    const totalInvested = pos.total_size_sol || 0;
    const feePct = totalInvested > 0 ? (totalFees / totalInvested) * 100 : 0;

    // Only show if there are fees to display
    if (totalFees <= 0) {
      return "";
    }

    return `
      <div class="pdd-analytics-card pdd-fee-analysis">
        <h4><i class="icon-dollar-sign"></i> Fee Analysis</h4>
        <div class="pdd-analysis-rows">
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Entry Fees</span>
            <span class="pdd-row-value">${Utils.formatSol(entryFees, { decimals: 6 })}</span>
          </div>
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Exit Fees</span>
            <span class="pdd-row-value">${exitFees > 0 ? Utils.formatSol(exitFees, { decimals: 6 }) : "—"}</span>
          </div>
          <div class="pdd-analysis-row pdd-row-total">
            <span class="pdd-row-label">Total Fees</span>
            <span class="pdd-row-value">${Utils.formatSol(totalFees, { decimals: 6 })}</span>
          </div>
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Fees as % of Investment</span>
            <span class="pdd-row-value">${Utils.formatNumber(feePct, 3)}%</span>
          </div>
        </div>
      </div>
    `;
  };
}
