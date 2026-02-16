/**
 * Overview Tab Mixin for Position Details Dialog
 * Adds overview tab rendering functionality to PositionDetailsDialog class
 */
import * as Utils from "../../core/utils.js";
import { requestManager } from "../../core/request_manager.js";
import { TradeActionDialog } from "../trade_action_dialog.js";

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

    const isOpen = pos.position_type !== "closed";
    const marketData = this.fullDetails?.market_data;
    const security = this.fullDetails?.security;
    const externalLinks = this.fullDetails?.external_links;
    const positionAge = this.fullDetails?.position_age_seconds;
    const solPriceUsd = this.fullDetails?.sol_price_usd;

    content.innerHTML = `
      <div class="pdd-overview-layout">
        ${this._buildSummaryBanner(pos, isOpen, marketData, solPriceUsd)}
        
        <div class="pdd-overview-grid">
          ${this._buildEntryInfoCard(pos, positionAge)}
          ${this._buildCurrentStateCard(pos, isOpen)}
        </div>
        
        ${this._buildPnLAnalysisCard(pos, isOpen, solPriceUsd)}
        
        <div class="pdd-overview-grid">
          ${this._buildMarketDataCard(marketData)}
          ${this._buildSecurityCard(security)}
        </div>
        
        ${this._buildQuickActionsCard(pos, isOpen, externalLinks)}
      </div>
    `;

    // Attach action button handlers
    this._attachActionHandlers(content, pos, isOpen);
  };

  /**
   * Build the summary banner at top of overview
   */
  proto._buildSummaryBanner = function (pos, isOpen, marketData, solPriceUsd) {
    const logoUrl = pos.logo_url || this.positionData.logo_url;
    const symbol = pos.symbol || "???";
    const name = pos.name || "Unknown Token";
    const currentPrice = pos.current_price;

    // P&L calculation
    const pnl = isOpen ? pos.unrealized_pnl : pos.pnl;
    const pnlPct = isOpen ? pos.unrealized_pnl_percent : pos.pnl_percent;
    const pnlClass = pnl != null && pnl >= 0 ? "pdd-positive" : "pdd-negative";
    const sign = pnl != null && pnl >= 0 ? "+" : "";

    // Price changes
    const priceChange1h = marketData?.price_change_h1;
    const priceChange24h = marketData?.price_change_h24;

    const formatPriceChange = (val) => {
      if (val === null || val === undefined) return null;
      const num = Number(val);
      if (!Number.isFinite(num)) return null;
      const cls = num >= 0 ? "pdd-change-positive" : "pdd-change-negative";
      const s = num >= 0 ? "+" : "";
      return `<span class="pdd-change-badge ${cls}">${s}${num.toFixed(2)}%</span>`;
    };

    const change1hHtml = formatPriceChange(priceChange1h);
    const change24hHtml = formatPriceChange(priceChange24h);

    // USD value if available
    let pnlUsdHtml = "";
    if (pnl !== undefined && solPriceUsd) {
      const pnlUsd = pnl * solPriceUsd;
      pnlUsdHtml = `<span class="pdd-pnl-usd">${sign}${Utils.formatCurrencyUSD(Math.abs(pnlUsd))}</span>`;
    }

    return `
      <div class="pdd-overview-banner">
        <div class="pdd-banner-left">
          <div class="pdd-banner-logo">
            ${logoUrl ? `<img src="${Utils.escapeHtml(logoUrl)}" alt="${Utils.escapeHtml(symbol)}" onerror="this.style.display='none'; this.nextElementSibling.style.display='flex';" /><div class="pdd-logo-fallback" style="display:none">${Utils.escapeHtml(symbol.charAt(0))}</div>` : `<div class="pdd-logo-fallback">${Utils.escapeHtml(symbol.charAt(0))}</div>`}
          </div>
          <div class="pdd-banner-token">
            <span class="pdd-banner-symbol">${Utils.escapeHtml(symbol)}</span>
            <span class="pdd-banner-name">${Utils.escapeHtml(name)}</span>
          </div>
        </div>
        
        <div class="pdd-banner-center">
          <div class="pdd-banner-price">
            <span class="pdd-price-value">${currentPrice ? this._formatPrice(currentPrice) : "—"}</span>
            <span class="pdd-price-unit">SOL</span>
          </div>
          <div class="pdd-price-changes">
            ${change1hHtml ? `<span class="pdd-change-item"><span class="pdd-change-label">1H</span>${change1hHtml}</span>` : ""}
            ${change24hHtml ? `<span class="pdd-change-item"><span class="pdd-change-label">24H</span>${change24hHtml}</span>` : ""}
          </div>
        </div>
        
        <div class="pdd-banner-right">
          <div class="pdd-banner-pnl ${pnlClass}">
            <span class="pdd-pnl-label">${isOpen ? "Unrealized P&L" : "Realized P&L"}</span>
            <span class="pdd-pnl-main">${pnl !== undefined ? sign + Utils.formatSol(pnl, { decimals: 4, suffix: "" }) : "—"} <span class="pdd-pnl-sol">SOL</span></span>
            <div class="pdd-pnl-secondary">
              <span class="pdd-pnl-pct">${pnlPct !== undefined ? sign + Utils.formatNumber(pnlPct, 2) + "%" : ""}</span>
              ${pnlUsdHtml}
            </div>
          </div>
        </div>
      </div>
    `;
  };

  /**
   * Build entry info card (left column of stats grid)
   */
  proto._buildEntryInfoCard = function (pos, positionAge) {
    const entryPrice = pos.entry_price;
    const avgEntryPrice = pos.average_entry_price;
    const totalInvested = pos.total_size_sol;
    const tokenAmount = pos.token_amount;
    const dcaCount = pos.dca_count || 0;

    // Format position age
    const ageFormatted = positionAge ? Utils.formatUptime(positionAge) : "—";

    // Show both entry and average if DCA
    const showAvgEntry = dcaCount > 0 && avgEntryPrice && avgEntryPrice !== entryPrice;

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
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Total Invested</span>
            <span class="pdd-stat-value">${Utils.formatSol(totalInvested, { decimals: 4 })}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Position Size</span>
            <span class="pdd-stat-value">${tokenAmount ? Utils.formatCompactNumber(tokenAmount) + " tokens" : "—"}</span>
          </div>
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
  proto._buildCurrentStateCard = function (pos, isOpen) {
    const currentPrice = pos.current_price;
    const remainingTokens = pos.remaining_token_amount;
    const originalTokens = pos.token_amount;
    const verified = pos.transaction_entry_verified;
    const partialExitCount = pos.partial_exit_count || 0;

    // Calculate holdings percentage (both values are raw u64, so ratio works directly)
    let holdingsPercent = 100;
    if (originalTokens && remainingTokens) {
      holdingsPercent = (remainingTokens / originalTokens) * 100;
    }

    // Calculate current value in SOL using proper decimals conversion
    const currentValue = this._calculateCurrentValue(pos);

    if (isOpen) {
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
              <span class="pdd-stat-value">${remainingTokens ? Utils.formatCompactNumber(remainingTokens) : "—"}</span>
            </div>
            <div class="pdd-stat-row">
              <span class="pdd-stat-label">Holdings %</span>
              <span class="pdd-stat-value">
                ${Utils.formatNumber(holdingsPercent, 1)}%
                ${partialExitCount > 0 ? `<span class="pdd-badge pdd-badge-warning">${partialExitCount} exits</span>` : ""}
              </span>
            </div>
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
  proto._buildPnLAnalysisCard = function (pos, isOpen, solPriceUsd) {
    const unrealizedPnl = pos.unrealized_pnl;
    const unrealizedPnlPct = pos.unrealized_pnl_percent;
    const realizedPnl = pos.pnl;
    const realizedPnlPct = pos.pnl_percent;
    const highestPrice = pos.price_highest;
    const lowestPrice = pos.price_lowest;
    const entryPrice = pos.average_entry_price || pos.entry_price;

    // Calculate peak deviation from entry
    let peakDeviation = null;
    if (highestPrice && entryPrice) {
      peakDeviation = ((highestPrice - entryPrice) / entryPrice) * 100;
    }

    // Entry/exit fees
    const entryFeeSol = pos.entry_fee_lamports ? pos.entry_fee_lamports / 1e9 : null;
    const exitFeeSol = pos.exit_fee_lamports ? pos.exit_fee_lamports / 1e9 : null;
    const totalFees = (entryFeeSol || 0) + (exitFeeSol || 0);

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
            <div class="pdd-pnl-stat">
              <span class="pdd-pnl-stat-label">Peak Price</span>
              <span class="pdd-pnl-stat-value">${highestPrice ? this._formatPrice(highestPrice) + " SOL" : "—"}</span>
            </div>
            <div class="pdd-pnl-stat">
              <span class="pdd-pnl-stat-label">Low Price</span>
              <span class="pdd-pnl-stat-value">${lowestPrice ? this._formatPrice(lowestPrice) + " SOL" : "—"}</span>
            </div>
            <div class="pdd-pnl-stat">
              <span class="pdd-pnl-stat-label">Peak from Entry</span>
              <span class="pdd-pnl-stat-value ${peakDeviation && peakDeviation > 0 ? "pdd-positive" : ""}">${peakDeviation !== null ? "+" + Utils.formatNumber(peakDeviation, 1) + "%" : "—"}</span>
            </div>
            <div class="pdd-pnl-stat">
              <span class="pdd-pnl-stat-label">Total Fees</span>
              <span class="pdd-pnl-stat-value">${totalFees > 0 ? Utils.formatSol(totalFees, { decimals: 6 }) : "—"}</span>
            </div>
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
   * Build security card
   */
  proto._buildSecurityCard = function (security) {
    if (!security) {
      return `
        <div class="pdd-stat-card pdd-stat-card-muted">
          <h3 class="pdd-stat-card-title">
            <i class="icon-shield"></i>
            Security
          </h3>
          <div class="pdd-stat-card-empty">
            <span>Security data not available</span>
          </div>
        </div>
      `;
    }

    const score = security.score_normalized;
    const riskLevel = security.risk_level || "unknown";
    const scoreClass = this._getRiskLevelClass(riskLevel);
    const scoreLabel = this._getRiskLevelLabel(riskLevel);

    // Authority warnings
    const warnings = [];
    if (security.has_mint_authority) warnings.push("Mint Authority");
    if (security.has_freeze_authority) warnings.push("Freeze Authority");

    return `
      <div class="pdd-stat-card">
        <h3 class="pdd-stat-card-title">
          <i class="icon-shield"></i>
          Security
        </h3>
        <div class="pdd-stat-card-content">
          <div class="pdd-security-score ${scoreClass}">
            <span class="pdd-score-value">${score !== null && score !== undefined ? score : "?"}</span>
            <span class="pdd-score-label">${scoreLabel}</span>
          </div>
          ${
            warnings.length > 0
              ? `
          <div class="pdd-security-warnings">
            ${warnings.map((w) => `<span class="pdd-warning-badge"><i class="icon-triangle-alert"></i> ${w}</span>`).join("")}
          </div>
          `
              : ""
          }
          ${
            security.top_risks && security.top_risks.length > 0
              ? `
          <div class="pdd-security-risks">
            <span class="pdd-risks-label">Top Risks:</span>
            <ul class="pdd-risks-list">
              ${security.top_risks.map((r) => `<li>${Utils.escapeHtml(r)}</li>`).join("")}
            </ul>
          </div>
          `
              : ""
          }
        </div>
      </div>
    `;
  };

  /**
   * Build quick actions card with external links
   */
  proto._buildQuickActionsCard = function (pos, isOpen, externalLinks) {
    const actionButtons = isOpen
      ? `
        <button class="pdd-action-btn pdd-action-add" id="pddAddBtn">
          <i class="icon-circle-plus"></i>
          <span>Add to Position</span>
        </button>
        <button class="pdd-action-btn pdd-action-partial" id="pddPartialBtn">
          <i class="icon-scissors"></i>
          <span>Partial Sell</span>
        </button>
        <button class="pdd-action-btn pdd-action-close" id="pddCloseBtn">
          <i class="icon-circle-x"></i>
          <span>Close Position</span>
        </button>
      `
      : `
        <button class="pdd-action-btn pdd-action-view pdd-action-full-width" id="pddViewTokenBtn">
          <i class="icon-external-link"></i>
          <span>View Token Details</span>
        </button>
      `;

    const links = externalLinks || {};

    return `
      <div class="pdd-actions-card">
        <h3 class="pdd-actions-title">
          <i class="icon-zap"></i>
          Quick Actions
        </h3>
        <div class="pdd-actions-row${!isOpen ? " pdd-actions-single" : ""}">
          ${actionButtons}
        </div>
        <div class="pdd-external-links">
          <span class="pdd-links-label">View on:</span>
          <a href="${links.solscan || "#"}" target="_blank" class="pdd-external-link" title="Solscan">
            <i class="icon-external-link"></i> Solscan
          </a>
          <a href="${links.dexscreener || "#"}" target="_blank" class="pdd-external-link" title="DexScreener">
            <i class="icon-external-link"></i> DexScreener
          </a>
          <a href="${links.birdeye || "#"}" target="_blank" class="pdd-external-link" title="Birdeye">
            <i class="icon-external-link"></i> Birdeye
          </a>
          <a href="${links.rugcheck || "#"}" target="_blank" class="pdd-external-link" title="Rugcheck">
            <i class="icon-external-link"></i> Rugcheck
          </a>
          <a href="${links.photon || "#"}" target="_blank" class="pdd-external-link" title="Photon">
            <i class="icon-external-link"></i> Photon
          </a>
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
   * Attach event handlers for action buttons
   */
  proto._attachActionHandlers = function (content, pos, isOpen) {
    // Clean up old handlers first
    if (this._actionHandlers) {
      this._actionHandlers.forEach(({ element, handler }) => {
        element.removeEventListener("click", handler);
      });
    }
    this._actionHandlers = [];

    if (isOpen) {
      const addBtn = content.querySelector("#pddAddBtn");
      const partialBtn = content.querySelector("#pddPartialBtn");
      const closeBtn = content.querySelector("#pddCloseBtn");

      if (addBtn) {
        const handler = () => this._handleAddPosition(pos, addBtn);
        addBtn.addEventListener("click", handler);
        this._actionHandlers.push({ element: addBtn, handler });
      }
      if (partialBtn) {
        const handler = () => this._handlePartialSell(pos, partialBtn);
        partialBtn.addEventListener("click", handler);
        this._actionHandlers.push({ element: partialBtn, handler });
      }
      if (closeBtn) {
        const handler = () => this._handleClosePosition(pos, closeBtn);
        closeBtn.addEventListener("click", handler);
        this._actionHandlers.push({ element: closeBtn, handler });
      }
    } else {
      const viewBtn = content.querySelector("#pddViewTokenBtn");
      if (viewBtn) {
        const handler = () => {
          // Close this dialog and open token details via global event
          const mint = pos.mint;
          const symbol = pos.symbol;
          this.close();
          // Dispatch global event to open token details dialog
          window.dispatchEvent(
            new CustomEvent("screenerbot:open-token-details", {
              detail: { mint, symbol },
            })
          );
        };
        viewBtn.addEventListener("click", handler);
        this._actionHandlers.push({ element: viewBtn, handler });
      }
    }
  };

  /**
   * Handle add to position action
   */
  proto._handleAddPosition = async function (pos, btn) {
    if (!this.tradeDialog) {
      this.tradeDialog = new TradeActionDialog();
    }

    const result = await this.tradeDialog.open({
      action: "add",
      symbol: pos.symbol,
      context: {
        mint: pos.mint,
        balance: pos.wallet_balance || 0,
        entrySize: pos.entry_size_sol,
        currentSize: pos.total_size_sol,
      },
    });

    if (result) {
      // Set loading state
      const originalText = btn?.innerHTML;
      if (btn) {
        btn.disabled = true;
        btn.innerHTML = '<i class="icon-loader"></i> Loading...';
      }

      try {
        const response = await requestManager.fetch("/api/trader/add-to-position", {
          method: "POST",
          body: JSON.stringify({
            mint: pos.mint,
            amount_sol: result.amount,
          }),
          priority: "high",
        });

        if (response.success) {
          Utils.showToast("Added to position successfully!", "success");
          this.onTradeComplete({ action: "add", mint: pos.mint });
          await this._fetchDetails();
        } else {
          Utils.showToast(response.error || "Failed to add to position", "error");
        }
      } catch (error) {
        Utils.showToast(error?.message || "Failed to add to position", "error");
      } finally {
        // Reset button state
        if (btn) {
          btn.disabled = false;
          btn.innerHTML = originalText;
        }
      }
    }
  };

  /**
   * Handle partial sell action
   */
  proto._handlePartialSell = async function (pos, btn) {
    if (!this.tradeDialog) {
      this.tradeDialog = new TradeActionDialog();
    }

    const result = await this.tradeDialog.open({
      action: "sell",
      symbol: pos.symbol,
      context: {
        mint: pos.mint,
        holdings: pos.remaining_token_amount || pos.token_amount,
      },
    });

    if (result) {
      // Set loading state
      const originalText = btn?.innerHTML;
      if (btn) {
        btn.disabled = true;
        btn.innerHTML = '<i class="icon-loader"></i> Loading...';
      }

      try {
        const response = await requestManager.fetch("/api/trader/sell", {
          method: "POST",
          body: JSON.stringify({
            mint: pos.mint,
            percentage: result.percentage,
          }),
          priority: "high",
        });

        if (response.success) {
          Utils.showToast("Sell order executed!", "success");
          this.onTradeComplete({ action: "sell", mint: pos.mint });
          await this._fetchDetails();
        } else {
          Utils.showToast(response.error || "Failed to execute sell", "error");
        }
      } catch (error) {
        Utils.showToast(error?.message || "Failed to execute sell", "error");
      } finally {
        // Reset button state
        if (btn) {
          btn.disabled = false;
          btn.innerHTML = originalText;
        }
      }
    }
  };

  /**
   * Handle close position action
   */
  proto._handleClosePosition = async function (pos, btn) {
    if (!this.tradeDialog) {
      this.tradeDialog = new TradeActionDialog();
    }

    // Pre-select 100% for close position
    const result = await this.tradeDialog.open({
      action: "sell",
      symbol: pos.symbol,
      context: {
        mint: pos.mint,
        holdings: pos.remaining_token_amount || pos.token_amount,
        preselect: 100,
      },
    });

    if (result && result.percentage === 100) {
      // Set loading state
      const originalText = btn?.innerHTML;
      if (btn) {
        btn.disabled = true;
        btn.innerHTML = '<i class="icon-loader"></i> Loading...';
      }

      try {
        const response = await requestManager.fetch("/api/trader/sell", {
          method: "POST",
          body: JSON.stringify({
            mint: pos.mint,
            percentage: 100,
          }),
          priority: "high",
        });

        if (response.success) {
          Utils.showToast("Position closed!", "success");
          this.onTradeComplete({ action: "close", mint: pos.mint });
          this.close();
        } else {
          Utils.showToast(response.error || "Failed to close position", "error");
        }
      } catch (error) {
        Utils.showToast(error?.message || "Failed to close position", "error");
      } finally {
        // Reset button state (in case close fails)
        if (btn) {
          btn.disabled = false;
          btn.innerHTML = originalText;
        }
      }
    }
  };
}
