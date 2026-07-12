/**
 * Overview Tab Mixin for Position Details Dialog
 * Adds overview tab rendering functionality to PositionDetailsDialog class
 */
import * as Utils from "../../core/utils.js";
import { manualTrade } from "../manual_trade.js";

const LAMPORTS_PER_SOL = 1e9;

const lamportsToSol = (lamports) => (lamports ? lamports / LAMPORTS_PER_SOL : 0);

/**
 * Apply overview tab methods to PositionDetailsDialog prototype
 * @param {class} PositionDetailsDialog - The PositionDetailsDialog class
 */
export function applyOverviewTabMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

  // ===========================================================================
  // OVERVIEW TAB
  // ===========================================================================

  proto._renderOverviewTab = function (content) {
    const pos = this.fullDetails?.position;
    if (!pos) {
      content.innerHTML = '<div class="pdd-empty-state">No position data available</div>';
      return;
    }

    // Security, links, token info and pool info are rendered by the header meta row, not
    // here — reading them into locals this function never used was left over from an
    // earlier layout.
    const isOpen = pos.position_type !== "closed";
    const marketData = this.fullDetails?.market_data;
    const positionAge = this.fullDetails?.position_age_seconds;
    const solPriceUsd = this.fullDetails?.sol_price_usd;
    const entries = this.fullDetails?.entries || [];
    const exits = this.fullDetails?.exits || [];

    content.innerHTML = `
      <div class="pdd-overview-layout">
        <div class="pdd-cards-grid">
          ${this._buildEntryInfoCard(pos, positionAge, entries)}
          ${this._buildCurrentStateCard(pos, isOpen, exits)}
          ${this._buildMarketDataCard(marketData)}
        </div>

        ${this._buildPnLAnalysisCard(pos, isOpen, solPriceUsd, entries, exits)}
      </div>
    `;
  };

  /**
   * Build entry info card (left column of stats grid)
   */
  proto._buildEntryInfoCard = function (pos, positionAge, entries = []) {
    const entryPrice = pos.entry_price;
    const avgEntryPrice = pos.average_entry_price;
    const totalInvested = pos.total_size_sol;
    const tokenAmount = pos.token_amount;
    const dcaCount = pos.dca_count || 0;

    // Format position age
    const ageFormatted = positionAge ? Utils.formatUptime(positionAge) : "—";

    // Show both entry and average if DCA
    const showAvgEntry = dcaCount > 0 && avgEntryPrice && avgEntryPrice !== entryPrice;

    // DCA extra detail: price range and total tokens (only when multiple entries)
    let minEntry = null;
    let maxEntry = null;
    let totalTokensAcquired = 0;
    if (entries.length > 1) {
      const prices = entries.map((e) => e.price).filter((p) => p != null && p > 0);
      if (prices.length > 0) {
        minEntry = Math.min(...prices);
        maxEntry = Math.max(...prices);
      }
      totalTokensAcquired = entries.reduce((sum, e) => sum + (e.amount || 0), 0);
    }

    return `
      <div class="pdd-stat-card">
        <h3 class="pdd-stat-card-title">
          <i class="icon-circle-arrow-down"></i>
          Entry Info
        </h3>
        <div class="pdd-stat-card-content">
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Entry Price</span>
            <span class="pdd-stat-value">${this._formatPrice(entryPrice)} SOL</span>
          </div>
          ${
            showAvgEntry
              ? `
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Avg Entry Price</span>
            <span class="pdd-stat-value">${this._formatPrice(avgEntryPrice)} SOL</span>
          </div>
          `
              : ""
          }
          ${
            minEntry !== null && maxEntry !== null && minEntry !== maxEntry
              ? `
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Entry Price Range</span>
            <span class="pdd-stat-value">${this._formatPrice(minEntry)} – ${this._formatPrice(maxEntry)} SOL</span>
          </div>
          `
              : ""
          }
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Total Invested</span>
            <span class="pdd-stat-value">${Utils.formatSol(totalInvested, { decimals: 4 })}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Position Size</span>
            <span class="pdd-stat-value">${tokenAmount ? Utils.formatCompactNumber(this._toUiAmount(tokenAmount)) + " tokens" : "—"}</span>
          </div>
          ${
            totalTokensAcquired > 0
              ? `
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Total Acquired</span>
            <span class="pdd-stat-value">${Utils.formatCompactNumber(this._toUiAmount(totalTokensAcquired))} tokens</span>
          </div>
          `
              : ""
          }
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Position Age</span>
            <span class="pdd-stat-value">${ageFormatted}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">DCA Count</span>
            <span class="pdd-stat-value">
              ${dcaCount}
              ${dcaCount > 0 ? '<span class="pdd-badge pdd-badge-info">DCA</span>' : ""}
            </span>
          </div>
        </div>
      </div>
    `;
  };

  /**
   * Build current state card (right column of stats grid)
   */
  proto._buildCurrentStateCard = function (pos, isOpen, exits = []) {
    const currentPrice = pos.current_price;
    const remainingTokens = pos.remaining_token_amount;
    const verified = pos.transaction_entry_verified;
    const partialExitCount = pos.partial_exit_count || 0;

    // Tokens ever acquired = still held + already exited. NOT `token_amount`, which is
    // only the ENTRY buy and never grows on a DCA — measuring against it reported a
    // DCA'd position as >100% held and understated the percentage sold.
    const acquiredTokens = (remainingTokens || 0) + (pos.total_exited_amount || 0);

    // Calculate holdings percentage (both values are raw u64, so ratio works directly)
    let holdingsPercent = 100;
    if (acquiredTokens > 0 && remainingTokens != null) {
      holdingsPercent = (remainingTokens / acquiredTokens) * 100;
    }

    // Calculate current value in SOL using proper decimals conversion
    const currentValue = this._calculateCurrentValue(pos);

    if (isOpen) {
      // Partial exit stats (tokens sold + SOL recovered so far)
      const tokensSold = exits.reduce((sum, e) => sum + (e.amount || 0), 0);
      const pctSold = acquiredTokens > 0 ? (tokensSold / acquiredTokens) * 100 : 0;
      const solRecoveredSoFar = exits.reduce((sum, e) => sum + (e.sol_received || 0), 0);

      return `
        <div class="pdd-stat-card">
          <h3 class="pdd-stat-card-title">
            <i class="icon-activity"></i>
            Current State
          </h3>
          <div class="pdd-stat-card-content">
            <div class="pdd-stat-row">
              <span class="pdd-stat-label">Current Price</span>
              <span class="pdd-stat-value">${currentPrice ? this._formatPrice(currentPrice) + " SOL" : "—"}</span>
            </div>
            <div class="pdd-stat-row">
              <span class="pdd-stat-label">Current Value</span>
              <span class="pdd-stat-value">${currentValue ? Utils.formatSol(currentValue, { decimals: 4 }) : "—"}</span>
            </div>
            <div class="pdd-stat-row">
              <span class="pdd-stat-label">Remaining Tokens</span>
              <span class="pdd-stat-value">${remainingTokens ? Utils.formatCompactNumber(this._toUiAmount(remainingTokens)) : "—"}</span>
            </div>
            <div class="pdd-stat-row">
              <span class="pdd-stat-label">Holdings %</span>
              <span class="pdd-stat-value">
                ${Utils.formatNumber(holdingsPercent, 1)}%
                ${partialExitCount > 0 ? `<span class="pdd-badge pdd-badge-warning">${partialExitCount} exits</span>` : ""}
              </span>
            </div>
            ${
              partialExitCount > 0 && tokensSold > 0
                ? `
            <div class="pdd-stat-row">
              <span class="pdd-stat-label">Tokens Sold</span>
              <span class="pdd-stat-value">${Utils.formatCompactNumber(this._toUiAmount(tokensSold))} <span class="pdd-stat-pct">(${Utils.formatNumber(pctSold, 1)}%)</span></span>
            </div>
            <div class="pdd-stat-row">
              <span class="pdd-stat-label">SOL Recovered</span>
              <span class="pdd-stat-value">${Utils.formatSol(solRecoveredSoFar, { decimals: 4 })}</span>
            </div>
            `
                : ""
            }
            <div class="pdd-stat-row">
              <span class="pdd-stat-label">Verification</span>
              <span class="pdd-stat-value">
                ${verified ? '<span class="pdd-verified"><i class="icon-circle-check"></i> Verified</span>' : '<span class="pdd-unverified"><i class="icon-clock"></i> Pending</span>'}
              </span>
            </div>
          </div>
        </div>
      `;
    }

    // Closed position
    const exitPrice = pos.average_exit_price || pos.exit_price;
    const solReceived = pos.sol_received;
    const closedReason = pos.closed_reason;
    const totalExited = pos.total_exited_amount || exits.reduce((sum, e) => sum + (e.amount || 0), 0);
    const pctExited = acquiredTokens > 0 ? (totalExited / acquiredTokens) * 100 : 0;

    return `
      <div class="pdd-stat-card">
        <h3 class="pdd-stat-card-title">
          <i class="icon-log-out"></i>
          Exit Details
        </h3>
        <div class="pdd-stat-card-content">
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Exit Price</span>
            <span class="pdd-stat-value">${exitPrice ? this._formatPrice(exitPrice) + " SOL" : "—"}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Tokens Sold</span>
            <span class="pdd-stat-value">${totalExited ? Utils.formatCompactNumber(this._toUiAmount(totalExited)) + (pctExited > 0 ? ` <span class="pdd-stat-pct">(${Utils.formatNumber(pctExited, 1)}%)</span>` : "") : "—"}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">SOL Received</span>
            <span class="pdd-stat-value">${solReceived ? Utils.formatSol(solReceived, { decimals: 4 }) : "—"}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Close Reason</span>
            <span class="pdd-stat-value">${closedReason ? Utils.escapeHtml(closedReason) : "Manual"}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Exit Verified</span>
            <span class="pdd-stat-value">
              ${pos.transaction_exit_verified ? '<span class="pdd-verified"><i class="icon-circle-check"></i> Verified</span>' : '<span class="pdd-unverified"><i class="icon-clock"></i> Pending</span>'}
            </span>
          </div>
        </div>
      </div>
    `;
  };

  /**
   * Build P&L analysis card (full width)
   */
  proto._buildPnLAnalysisCard = function (pos, isOpen, solPriceUsd, entries = [], exits = []) {
    const unrealizedPnl = pos.unrealized_pnl;
    const unrealizedPnlPct = pos.unrealized_pnl_percent;
    const realizedPnl = pos.pnl;
    const realizedPnlPct = pos.pnl_percent;
    const highestPrice = pos.price_highest;
    const lowestPrice = pos.price_lowest;
    const currentPrice = pos.current_price;
    const entryPrice = pos.average_entry_price || pos.entry_price;

    // ROI (same calculation as unrealized/realized pnl_percent, explicit label)
    const roiPct = isOpen ? unrealizedPnlPct : realizedPnlPct;

    // Peak deviation from entry %
    let peakDeviation = null;
    if (highestPrice && entryPrice) {
      peakDeviation = ((highestPrice - entryPrice) / entryPrice) * 100;
    }

    // Low deviation from entry %
    let lowDeviation = null;
    if (lowestPrice && entryPrice && entryPrice > 0) {
      lowDeviation = ((lowestPrice - entryPrice) / entryPrice) * 100;
    }

    // Drawdown: current vs peak
    let drawdown = null;
    if (isOpen && currentPrice && highestPrice && highestPrice > 0) {
      drawdown = ((currentPrice - highestPrice) / highestPrice) * 100;
    }

    // Fee breakdown. The position's own fee fields cover only the entry and the final
    // close; every DCA add and partial exit carries its own fee on its RECORD, and the
    // records report it in SOL (`fees_sol`) — the fallback used to read `fee_lamports`,
    // a field the API does not send, so it silently summed to zero on every DCA'd or
    // partially-exited position.
    const recordFees = (records) => records.reduce((sum, r) => sum + (r.fees_sol || 0), 0);
    const entryFees = recordFees(entries) || lamportsToSol(pos.entry_fee_lamports);
    const exitFees = recordFees(exits) || lamportsToSol(pos.exit_fee_lamports);
    const totalFees = entryFees + exitFees;
    const totalInvested = pos.total_size_sol || 0;
    const feePct = totalInvested > 0 && totalFees > 0 ? (totalFees / totalInvested) * 100 : 0;

    const formatPnlRow = (label, pnl, pct, solPrice) => {
      if (pnl === null || pnl === undefined) return "";
      const cls = pnl >= 0 ? "pdd-positive" : "pdd-negative";
      const sign = pnl >= 0 ? "+" : "";
      let usdPart = "";
      if (solPrice && pnl !== 0) {
        const usd = pnl * solPrice;
        usdPart = `<span class="pdd-pnl-usd-inline">(${sign}${Utils.formatCurrencyUSD(Math.abs(usd))})</span>`;
      }
      return `
        <div class="pdd-pnl-row ${cls}">
          <span class="pdd-pnl-row-label">${label}</span>
          <span class="pdd-pnl-row-value">
            ${sign}${Utils.formatSol(pnl, { decimals: 4, suffix: "" })} SOL
            ${pct !== undefined ? `<span class="pdd-pnl-row-pct">${sign}${Utils.formatNumber(pct, 2)}%</span>` : ""}
            ${usdPart}
          </span>
        </div>
      `;
    };

    return `
      <div class="pdd-pnl-card">
        <h3 class="pdd-stat-card-title">
          <i class="icon-trending-up"></i>
          P&L Analysis
        </h3>
        <div class="pdd-pnl-card-content">
          <div class="pdd-pnl-rows">
            ${isOpen ? formatPnlRow("Unrealized P&L", unrealizedPnl, unrealizedPnlPct, solPriceUsd) : ""}
            ${!isOpen || pos.total_exited_amount ? formatPnlRow("Realized P&L", realizedPnl, realizedPnlPct, solPriceUsd) : ""}
          </div>
          <div class="pdd-pnl-stats">
            ${
              roiPct !== null && roiPct !== undefined
                ? `<div class="pdd-pnl-stat">
              <span class="pdd-pnl-stat-label">ROI</span>
              <span class="pdd-pnl-stat-value ${roiPct >= 0 ? "pdd-positive" : "pdd-negative"}">${roiPct >= 0 ? "+" : ""}${Utils.formatNumber(roiPct, 2)}%</span>
            </div>`
                : ""
            }
            <div class="pdd-pnl-stat">
              <span class="pdd-pnl-stat-label">Peak Price</span>
              <span class="pdd-pnl-stat-value">${highestPrice ? this._formatPrice(highestPrice) + " SOL" : "—"}</span>
            </div>
            <div class="pdd-pnl-stat">
              <span class="pdd-pnl-stat-label">Peak from Entry</span>
              <span class="pdd-pnl-stat-value ${peakDeviation && peakDeviation > 0 ? "pdd-positive" : ""}">${peakDeviation !== null ? "+" + Utils.formatNumber(peakDeviation, 1) + "%" : "—"}</span>
            </div>
            <div class="pdd-pnl-stat">
              <span class="pdd-pnl-stat-label">Low Price</span>
              <span class="pdd-pnl-stat-value">${lowestPrice ? this._formatPrice(lowestPrice) + " SOL" : "—"}</span>
            </div>
            ${
              lowDeviation !== null
                ? `<div class="pdd-pnl-stat">
              <span class="pdd-pnl-stat-label">Low from Entry</span>
              <span class="pdd-pnl-stat-value ${lowDeviation < 0 ? "pdd-negative" : "pdd-positive"}">${lowDeviation >= 0 ? "+" : ""}${Utils.formatNumber(lowDeviation, 1)}%</span>
            </div>`
                : ""
            }
            ${
              drawdown !== null
                ? `<div class="pdd-pnl-stat">
              <span class="pdd-pnl-stat-label">Drawdown (vs Peak)</span>
              <span class="pdd-pnl-stat-value ${drawdown < 0 ? "pdd-negative" : ""}">${Utils.formatNumber(drawdown, 1)}%</span>
            </div>`
                : ""
            }
            ${
              entryFees > 0
                ? `<div class="pdd-pnl-stat">
              <span class="pdd-pnl-stat-label">Entry Fees</span>
              <span class="pdd-pnl-stat-value">${Utils.formatSol(entryFees, { decimals: 6 })}</span>
            </div>`
                : ""
            }
            ${
              exitFees > 0
                ? `<div class="pdd-pnl-stat">
              <span class="pdd-pnl-stat-label">Exit Fees</span>
              <span class="pdd-pnl-stat-value">${Utils.formatSol(exitFees, { decimals: 6 })}</span>
            </div>`
                : ""
            }
            <div class="pdd-pnl-stat">
              <span class="pdd-pnl-stat-label">Total Fees</span>
              <span class="pdd-pnl-stat-value">${totalFees > 0 ? Utils.formatSol(totalFees, { decimals: 6 }) : "—"}</span>
            </div>
            ${
              feePct > 0
                ? `<div class="pdd-pnl-stat">
              <span class="pdd-pnl-stat-label">Fees / Investment</span>
              <span class="pdd-pnl-stat-value">${Utils.formatNumber(feePct, 3)}%</span>
            </div>`
                : ""
            }
          </div>
        </div>
      </div>
    `;
  };

  /**
   * Build market data card
   */
  proto._buildMarketDataCard = function (marketData) {
    if (!marketData) {
      return `
        <div class="pdd-stat-card pdd-stat-card-muted">
          <h3 class="pdd-stat-card-title">
            <i class="icon-chart-bar"></i>
            Market Data
          </h3>
          <div class="pdd-stat-card-empty">
            <span>Market data not available</span>
          </div>
        </div>
      `;
    }

    return `
      <div class="pdd-stat-card">
        <h3 class="pdd-stat-card-title">
          <i class="icon-chart-bar"></i>
          Market Data
        </h3>
        <div class="pdd-stat-card-content">
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Market Cap</span>
            <span class="pdd-stat-value">${marketData.market_cap ? Utils.formatCurrencyUSD(marketData.market_cap) : "—"}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">FDV</span>
            <span class="pdd-stat-value">${marketData.fdv ? Utils.formatCurrencyUSD(marketData.fdv) : "—"}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Liquidity</span>
            <span class="pdd-stat-value">${marketData.liquidity_usd ? Utils.formatCurrencyUSD(marketData.liquidity_usd) : "—"}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">24h Volume</span>
            <span class="pdd-stat-value">${marketData.volume_24h ? Utils.formatCurrencyUSD(marketData.volume_24h) : "—"}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Holders</span>
            <span class="pdd-stat-value">${marketData.holder_count ? Utils.formatCompactNumber(marketData.holder_count) : "—"}</span>
          </div>
        </div>
      </div>
    `;
  };



  /**
   * Get CSS class for risk level
   */
  proto._getRiskLevelClass = function (level) {
    switch (level?.toLowerCase()) {
      case "low":
        return "pdd-risk-low";
      case "medium":
        return "pdd-risk-medium";
      case "high":
        return "pdd-risk-high";
      default:
        return "pdd-risk-unknown";
    }
  };

  /**
   * Get label for risk level
   */
  proto._getRiskLevelLabel = function (level) {
    switch (level?.toLowerCase()) {
      case "low":
        return "Low Risk";
      case "medium":
        return "Medium Risk";
      case "high":
        return "High Risk";
      default:
        return "Unknown";
    }
  };

  /**
   * Bind the top trade-action buttons (in the tab bar). Bound once at dialog
   * creation; `pos` is resolved fresh at click time. Buttons live in the header
   * so they are reachable from every tab, not just Overview.
   */
  proto._attachTradeButtons = function () {
    if (this._actionHandlers) {
      this._actionHandlers.forEach(({ element, handler }) => {
        element.removeEventListener("click", handler);
      });
    }
    this._actionHandlers = [];

    const bind = (id, fn) => {
      const btn = this.dialogEl?.querySelector(id);
      if (!btn) return;
      const handler = () => fn(btn);
      btn.addEventListener("click", handler);
      this._actionHandlers.push({ element: btn, handler });
    };
    const pos = () => this.fullDetails?.position || this.positionData;

    bind("#pddAddBtn", (btn) => this._handleAddPosition(pos(), btn));
    bind("#pddPartialBtn", (btn) => this._handlePartialSell(pos(), btn));
    bind("#pddCloseBtn", (btn) => this._handleClosePosition(pos(), btn));
    bind("#pddViewTokenBtn", () => {
      const p = pos();
      const { mint, symbol } = p;
      this.close();
      window.dispatchEvent(
        new CustomEvent("screenerbot:open-token-details", { detail: { mint, symbol } })
      );
    });
  };

  /**
   * Add / sell / close all run through the shared manual-trade flow
   * (ui/manual_trade.js), which owns the dialog, payload, endpoints and toasts, and
   * resolves the position itself.
   *
   * The local copies these replace had two silent bugs:
   *   - they tested `response.success`, but the manual endpoints return RAW JSON with
   *     no {success,data} envelope, so `success` was always undefined and a
   *     SUCCESSFUL trade reported "Failed to ..." and never refreshed the view;
   *   - "close position" POSTed percentage: 100 rather than close_all, which can
   *     leave dust behind instead of fully closing.
   */
  proto._runTrade = async function (action, pos, btn, context = {}) {
    const originalText = btn?.innerHTML;
    if (btn) btn.innerHTML = '<i class="icon-loader"></i> Loading...';

    const placed = await manualTrade({
      action: action === "close" ? "sell" : action,
      mint: pos.mint,
      symbol: pos.symbol,
      btn,
      context,
    });

    // Restore the label; the disabled state is reasserted from the live in-flight
    // state so the button stays disabled while the trade processes.
    if (btn) btn.innerHTML = originalText;
    this._updateTradeButtonsState();

    return placed;
  };

  /**
   * Handle add to position action
   */
  proto._handleAddPosition = async function (pos, btn) {
    const placed = await this._runTrade("add", pos, btn, {
      entrySize: pos.entry_size_sol,
    });

    if (!placed) return;
    this.onTradeComplete({ action: "add", mint: pos.mint });
    await this._fetchDetails();
  };

  /**
   * Handle partial sell action
   */
  proto._handlePartialSell = async function (pos, btn) {
    const placed = await this._runTrade("sell", pos, btn);

    if (!placed) return;
    this.onTradeComplete({ action: "sell", mint: pos.mint });
    await this._fetchDetails();
  };

  /**
   * Handle close position action — the sell dialog pre-selected at 100%, which the
   * shared flow submits as `close_all` (no dust left behind).
   */
  proto._handleClosePosition = async function (pos, btn) {
    const placed = await this._runTrade("close", pos, btn, { preselect: 100 });

    if (!placed) return;
    this.onTradeComplete({ action: "close", mint: pos.mint });
    this.close();
  };
}
