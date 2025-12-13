/**
 * Position Details Dialog
 * Full-screen dialog showing comprehensive position information with multiple tabs
 */
import * as Utils from "../core/utils.js";
import { Poller } from "../core/poller.js";
import { requestManager } from "../core/request_manager.js";
import { TradeActionDialog } from "./trade_action_dialog.js";

export class PositionDetailsDialog {
  constructor(options = {}) {
    this.onClose = options.onClose || (() => {});
    this.onTradeComplete = options.onTradeComplete || (() => {});
    this.dialogEl = null;
    this.currentTab = "overview";
    this.positionData = null;
    this.fullDetails = null;
    this.isLoading = false;
    this.isOpening = false;
    this.refreshPoller = null;
    this.tradeDialog = null;
    this._tabHandlers = null;
    this._chartTimeframe = "5m";
    this._chartData = null;
    this._tfButtonHandlers = null;
    this._escapeHandler = null;
    this._closeHandler = null;
    this._backdropHandler = null;
    this._copyMintHandler = null;
    this._actionHandlers = null;
    this._filterHandlers = null;
  }

  /**
   * Show dialog with position data
   * @param {Object} positionData - Position data object (at minimum needs id or mint)
   */
  async show(positionData) {
    if (!positionData || (!positionData.id && !positionData.mint)) {
      console.error("Invalid position data provided to PositionDetailsDialog");
      return;
    }

    if (this.isOpening) {
      console.log("Dialog already opening, ignoring duplicate request");
      return;
    }

    if (this.dialogEl) {
      this.close();
      await new Promise((resolve) => setTimeout(resolve, 350));
    }

    this.isOpening = true;

    try {
      this.positionData = positionData;
      this.fullDetails = null;
      this.currentTab = "overview";

      this._createDialog();
      this._attachEventHandlers();

      requestAnimationFrame(() => {
        if (this.dialogEl) {
          this.dialogEl.classList.add("active");
        }
      });

      // Fetch full details
      await this._fetchDetails();

      // Start polling for live price updates
      this._startPolling();
    } finally {
      this.isOpening = false;
    }
  }

  /**
   * Fetch full position details from API
   */
  async _fetchDetails() {
    if (this.isLoading) return;
    this.isLoading = true;

    try {
      const key = this._getPositionKey();
      const data = await requestManager.fetch(`/api/positions/${key}/details`, {
        priority: "high",
      });

      this.fullDetails = data;
      this._updateDialogContent();
    } catch (error) {
      console.error("Error loading position details:", error);
      this._showError("Failed to load position details");
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Get position key for API request (id:123 or mint:address)
   */
  _getPositionKey() {
    if (this.positionData.id) {
      return `id:${this.positionData.id}`;
    }
    return `mint:${this.positionData.mint}`;
  }

  /**
   * Start polling for live updates
   */
  _startPolling() {
    this._stopPolling();

    // Only poll for open positions
    if (this.positionData.position_type === "closed") {
      return;
    }

    this.refreshPoller = new Poller(
      () => {
        this._fetchDetails();
      },
      { label: "PositionDetails", interval: 5000 }
    );
    this.refreshPoller.start();
  }

  /**
   * Stop polling
   */
  _stopPolling() {
    if (this.refreshPoller) {
      this.refreshPoller.stop();
      this.refreshPoller.cleanup();
      this.refreshPoller = null;
    }
  }

  /**
   * Show error message in dialog
   */
  _showError(message) {
    const content = this.dialogEl?.querySelector(".tab-content.active");
    if (content) {
      content.innerHTML = `
        <div class="pdd-error-state">
          <i class="icon-alert-circle"></i>
          <p>${Utils.escapeHtml(message)}</p>
        </div>
      `;
    }
  }

  /**
   * Update dialog content after data fetch
   */
  _updateDialogContent() {
    if (!this.fullDetails) return;
    this._updateHeader();
    this._loadTabContent(this.currentTab);
  }

  /**
   * Close dialog
   */
  close() {
    if (!this.dialogEl) return;

    this._stopPolling();
    this.dialogEl.classList.remove("active");

    setTimeout(() => {
      if (this._escapeHandler) {
        document.removeEventListener("keydown", this._escapeHandler);
        this._escapeHandler = null;
      }

      if (this.dialogEl) {
        if (this._closeHandler) {
          const closeBtn = this.dialogEl.querySelector(".dialog-close");
          if (closeBtn) {
            closeBtn.removeEventListener("click", this._closeHandler);
          }
          this._closeHandler = null;
        }

        if (this._backdropHandler) {
          const backdrop = this.dialogEl.querySelector(".dialog-backdrop");
          if (backdrop) {
            backdrop.removeEventListener("click", this._backdropHandler);
          }
          this._backdropHandler = null;
        }

        if (this._copyMintHandler) {
          const copyBtn = this.dialogEl.querySelector("#pddCopyMintBtn");
          if (copyBtn) {
            copyBtn.removeEventListener("click", this._copyMintHandler);
          }
          this._copyMintHandler = null;
        }

        if (this._tabHandlers) {
          this._tabHandlers.forEach(({ element, handler }) => {
            element.removeEventListener("click", handler);
          });
          this._tabHandlers = null;
        }

        // Clean up chart timeframe handlers
        if (this._tfButtonHandlers) {
          this._tfButtonHandlers.forEach(({ element, handler }) => {
            element.removeEventListener("click", handler);
          });
          this._tfButtonHandlers = null;
        }
        this._chartData = null;

        // Clean up action button handlers
        if (this._actionHandlers) {
          this._actionHandlers.forEach(({ element, handler }) => {
            element.removeEventListener("click", handler);
          });
          this._actionHandlers = null;
        }

        // Clean up filter button handlers
        if (this._filterHandlers) {
          this._filterHandlers.forEach(({ element, handler }) => {
            element.removeEventListener("click", handler);
          });
          this._filterHandlers = null;
        }

        this.dialogEl.remove();
        this.dialogEl = null;
      }

      this.positionData = null;
      this.fullDetails = null;
      this.currentTab = "overview";
      this.isLoading = false;
      this.isOpening = false;

      this.onClose();
    }, 300);
  }

  /**
   * Destroy dialog completely, cleaning up all resources
   */
  destroy() {
    this._stopPolling();

    // Remove event handlers
    if (this._escapeHandler) {
      document.removeEventListener("keydown", this._escapeHandler);
      this._escapeHandler = null;
    }

    if (this.dialogEl) {
      this.dialogEl.remove();
      this.dialogEl = null;
    }

    if (this.tradeDialog) {
      this.tradeDialog = null;
    }

    this._closeHandler = null;
    this._backdropHandler = null;
    this._tabHandlers = null;
    this._copyMintHandler = null;
    this.positionData = null;
    this.fullDetails = null;
  }

  /**
   * Create dialog DOM structure
   */
  _createDialog() {
    this.dialogEl = document.createElement("div");
    this.dialogEl.className = "position-details-dialog";
    this.dialogEl.innerHTML = this._getDialogHTML();
    document.body.appendChild(this.dialogEl);
  }

  /**
   * Get initial dialog HTML
   */
  _getDialogHTML() {
    const pos = this.positionData;
    const symbol = pos.symbol || "Unknown";
    const name = pos.name || "Unknown Token";
    const logoUrl = pos.logo_url || "";
    const isOpen = pos.position_type !== "closed";
    const statusBadge = isOpen
      ? '<span class="pdd-badge pdd-badge-success">Open</span>'
      : '<span class="pdd-badge pdd-badge-secondary">Closed</span>';

    return `
      <div class="dialog-backdrop"></div>
      <div class="dialog-container">
        <div class="dialog-header">
          <div class="header-top-row">
            <div class="header-left">
              <div class="header-logo">
                ${logoUrl ? `<img src="${Utils.escapeHtml(logoUrl)}" alt="${Utils.escapeHtml(symbol)}" onerror="this.parentElement.innerHTML='<div class=\\'logo-placeholder\\'>${Utils.escapeHtml(symbol.charAt(0))}</div>'" />` : `<div class="logo-placeholder">${Utils.escapeHtml(symbol.charAt(0))}</div>`}
              </div>
              <div class="header-title">
                <span class="title-main">${Utils.escapeHtml(symbol)}</span>
                <span class="title-sub">${Utils.escapeHtml(name)}</span>
              </div>
              <div class="header-badges">
                ${statusBadge}
              </div>
            </div>
            <div class="header-center">
              <div class="header-price" id="pddHeaderPrice">
                <div class="price-loading">Loading...</div>
              </div>
            </div>
            <div class="header-right">
              <div class="header-actions">
                <button class="action-btn" id="pddCopyMintBtn" title="Copy Mint Address">
                  <i class="icon-copy"></i>
                </button>
                <a href="https://solscan.io/token/${Utils.escapeHtml(pos.mint)}" target="_blank" class="action-btn" title="View on Solscan">
                  <i class="icon-external-link"></i>
                </a>
              </div>
              <button class="dialog-close" type="button" title="Close (ESC)">
                <i class="icon-x"></i>
              </button>
            </div>
          </div>
        </div>

        <div class="dialog-tabs">
          <button class="tab-button active" data-tab="overview">
            <i class="icon-info"></i>
            Overview
          </button>
          <button class="tab-button" data-tab="chart">
            <i class="icon-bar-chart-2"></i>
            Chart
          </button>
          <button class="tab-button" data-tab="history">
            <i class="icon-clock"></i>
            History
          </button>
          <button class="tab-button" data-tab="transactions">
            <i class="icon-list"></i>
            Transactions
          </button>
          <button class="tab-button" data-tab="token">
            <i class="icon-info"></i>
            Token
          </button>
          <button class="tab-button" data-tab="analytics">
            <i class="icon-trending-up"></i>
            Analytics
          </button>
        </div>

        <div class="dialog-body">
          <div class="tab-content active" data-tab-content="overview">
            <div class="loading-spinner">Loading position details...</div>
          </div>
          <div class="tab-content" data-tab-content="chart">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="history">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="transactions">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="token">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="analytics">
            <div class="loading-spinner">Loading...</div>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Update header with current position data
   */
  _updateHeader() {
    const pos = this.fullDetails?.position;
    if (!pos) return;

    const priceContainer = this.dialogEl?.querySelector("#pddHeaderPrice");
    if (priceContainer) {
      priceContainer.innerHTML = this._buildHeaderPrice(pos);
    }
  }

  /**
   * Build header price section HTML
   */
  _buildHeaderPrice(pos) {
    const currentPrice = pos.summary?.current_price;
    const entryPrice = pos.summary?.average_entry_price || pos.summary?.entry_price;
    const isOpen = pos.summary?.position_type !== "closed";

    let priceHtml = "";
    if (currentPrice !== null && currentPrice !== undefined) {
      priceHtml = `
        <div class="price-block">
          <div class="price-sol-row">
            <span class="price-sol">${this._formatPrice(currentPrice)}</span>
            <span class="price-sol-unit">SOL</span>
          </div>
          <span class="price-label">Current Price</span>
        </div>
      `;
    }

    // P&L display
    let pnlHtml = "";
    if (isOpen && pos.summary?.unrealized_pnl !== undefined) {
      const pnl = pos.summary.unrealized_pnl;
      const pnlPct = pos.summary.unrealized_pnl_percent;
      const pnlClass = (pnl != null && pnl >= 0) ? "pdd-positive" : "pdd-negative";
      const sign = (pnl != null && pnl >= 0) ? "+" : "";
      pnlHtml = `
        <div class="pnl-block ${pnlClass}">
          <span class="pnl-value">${sign}${Utils.formatSol(pnl, { decimals: 4, suffix: "" })}</span>
          <span class="pnl-percent">${sign}${Utils.formatNumber(pnlPct, 2)}%</span>
        </div>
      `;
    } else if (!isOpen && pos.summary?.pnl !== undefined) {
      const pnl = pos.summary.pnl;
      const pnlPct = pos.summary.pnl_percent;
      const pnlClass = (pnl != null && pnl >= 0) ? "pdd-positive" : "pdd-negative";
      const sign = (pnl != null && pnl >= 0) ? "+" : "";
      pnlHtml = `
        <div class="pnl-block ${pnlClass}">
          <span class="pnl-value">${sign}${Utils.formatSol(pnl, { decimals: 4, suffix: "" })}</span>
          <span class="pnl-percent">${sign}${Utils.formatNumber(pnlPct, 2)}%</span>
        </div>
      `;
    }

    return `
      ${priceHtml}
      ${pnlHtml}
      <div class="price-metrics">
        <div class="metric-item">
          <span class="metric-label">Entry</span>
          <span class="metric-value">${this._formatPrice(entryPrice)}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Invested</span>
          <span class="metric-value">${Utils.formatSol(pos.summary?.total_size_sol, { decimals: 4 })}</span>
        </div>
      </div>
    `;
  }

  /**
   * Attach event handlers
   */
  _attachEventHandlers() {
    const closeBtn = this.dialogEl.querySelector(".dialog-close");
    this._closeHandler = () => this.close();
    closeBtn.addEventListener("click", this._closeHandler);

    const backdrop = this.dialogEl.querySelector(".dialog-backdrop");
    this._backdropHandler = () => this.close();
    backdrop.addEventListener("click", this._backdropHandler);

    this._escapeHandler = (e) => {
      if (e.key === "Escape") {
        this.close();
      }
    };
    document.addEventListener("keydown", this._escapeHandler);

    // Copy mint button
    const copyBtn = this.dialogEl.querySelector("#pddCopyMintBtn");
    if (copyBtn) {
      this._copyMintHandler = () => {
        Utils.copyToClipboard(this.positionData.mint);
        Utils.showToast("Mint address copied!", "success");
      };
      copyBtn.addEventListener("click", this._copyMintHandler);
    }

    // Tab buttons
    const tabButtons = this.dialogEl.querySelectorAll(".tab-button");
    this._tabHandlers = [];
    tabButtons.forEach((btn) => {
      const handler = () => {
        const tabId = btn.dataset.tab;
        this._switchTab(tabId);
      };
      btn.addEventListener("click", handler);
      this._tabHandlers.push({ element: btn, handler });
    });
  }

  /**
   * Switch to a different tab
   */
  _switchTab(tabId) {
    if (tabId === this.currentTab) return;

    const tabButtons = this.dialogEl.querySelectorAll(".tab-button");
    tabButtons.forEach((btn) => {
      btn.classList.toggle("active", btn.dataset.tab === tabId);
    });

    const tabContents = this.dialogEl.querySelectorAll(".tab-content");
    tabContents.forEach((content) => {
      content.classList.toggle("active", content.dataset.tabContent === tabId);
    });

    this.currentTab = tabId;
    this._loadTabContent(tabId);
  }

  /**
   * Load content for a specific tab
   */
  _loadTabContent(tabId) {
    const content = this.dialogEl?.querySelector(`[data-tab-content="${tabId}"]`);
    if (!content) return;

    if (!this.fullDetails) {
      content.innerHTML = '<div class="loading-spinner">Loading position details...</div>';
      return;
    }

    switch (tabId) {
      case "overview":
        this._renderOverviewTab(content);
        break;
      case "chart":
        this._renderChartTab(content);
        break;
      case "history":
        this._renderHistoryTab(content);
        break;
      case "transactions":
        this._renderTransactionsTab(content);
        break;
      case "token":
        this._renderTokenTab(content);
        break;
      case "analytics":
        this._renderAnalyticsTab(content);
        break;
    }
  }

  // ===========================================================================
  // OVERVIEW TAB
  // ===========================================================================

  _renderOverviewTab(content) {
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
  }

  /**
   * Build the summary banner at top of overview
   */
  _buildSummaryBanner(pos, isOpen, marketData, solPriceUsd) {
    const logoUrl = pos.logo_url || this.positionData.logo_url;
    const symbol = pos.symbol || "???";
    const name = pos.name || "Unknown Token";
    const currentPrice = pos.current_price;

    // P&L calculation
    const pnl = isOpen ? pos.unrealized_pnl : pos.pnl;
    const pnlPct = isOpen ? pos.unrealized_pnl_percent : pos.pnl_percent;
    const pnlClass = (pnl != null && pnl >= 0) ? "pdd-positive" : "pdd-negative";
    const sign = (pnl != null && pnl >= 0) ? "+" : "";

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
  }

  /**
   * Build entry info card (left column of stats grid)
   */
  _buildEntryInfoCard(pos, positionAge) {
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
          <i class="icon-arrow-down-circle"></i>
          Entry Info
        </h3>
        <div class="pdd-stat-card-content">
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Entry Price</span>
            <span class="pdd-stat-value">${this._formatPrice(entryPrice)} SOL</span>
          </div>
          ${showAvgEntry ? `
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Avg Entry Price</span>
            <span class="pdd-stat-value">${this._formatPrice(avgEntryPrice)} SOL</span>
          </div>
          ` : ""}
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
  }

  /**
   * Build current state card (right column of stats grid)
   */
  _buildCurrentStateCard(pos, isOpen) {
    const currentPrice = pos.current_price;
    const remainingTokens = pos.remaining_token_amount;
    const originalTokens = pos.token_amount;
    const verified = pos.transaction_entry_verified;
    const partialExitCount = pos.partial_exit_count || 0;

    // Calculate holdings percentage
    let holdingsPercent = 100;
    if (originalTokens && remainingTokens) {
      holdingsPercent = (remainingTokens / originalTokens) * 100;
    }

    // Calculate current value
    let currentValue = null;
    if (currentPrice && remainingTokens) {
      currentValue = currentPrice * remainingTokens;
    }

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
                ${verified ? '<span class="pdd-verified"><i class="icon-check-circle"></i> Verified</span>' : '<span class="pdd-unverified"><i class="icon-clock"></i> Pending</span>'}
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
              ${pos.transaction_exit_verified ? '<span class="pdd-verified"><i class="icon-check-circle"></i> Verified</span>' : '<span class="pdd-unverified"><i class="icon-clock"></i> Pending</span>'}
            </span>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Build P&L analysis card (full width)
   */
  _buildPnLAnalysisCard(pos, isOpen, solPriceUsd) {
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
  }

  /**
   * Build market data card
   */
  _buildMarketDataCard(marketData) {
    if (!marketData) {
      return `
        <div class="pdd-stat-card pdd-stat-card-muted">
          <h3 class="pdd-stat-card-title">
            <i class="icon-bar-chart-2"></i>
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
          <i class="icon-bar-chart-2"></i>
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
  }

  /**
   * Build security card
   */
  _buildSecurityCard(security) {
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
          ${warnings.length > 0 ? `
          <div class="pdd-security-warnings">
            ${warnings.map((w) => `<span class="pdd-warning-badge"><i class="icon-alert-triangle"></i> ${w}</span>`).join("")}
          </div>
          ` : ""}
          ${security.top_risks && security.top_risks.length > 0 ? `
          <div class="pdd-security-risks">
            <span class="pdd-risks-label">Top Risks:</span>
            <ul class="pdd-risks-list">
              ${security.top_risks.map((r) => `<li>${Utils.escapeHtml(r)}</li>`).join("")}
            </ul>
          </div>
          ` : ""}
        </div>
      </div>
    `;
  }

  /**
   * Build quick actions card with external links
   */
  _buildQuickActionsCard(pos, isOpen, externalLinks) {
    const actionButtons = isOpen
      ? `
        <button class="pdd-action-btn pdd-action-add" id="pddAddBtn">
          <i class="icon-plus-circle"></i>
          Add to Position
        </button>
        <button class="pdd-action-btn pdd-action-partial" id="pddPartialBtn">
          <i class="icon-scissors"></i>
          Partial Sell
        </button>
        <button class="pdd-action-btn pdd-action-close" id="pddCloseBtn">
          <i class="icon-x-circle"></i>
          Close Position
        </button>
      `
      : `
        <button class="pdd-action-btn pdd-action-view" id="pddViewTokenBtn">
          <i class="icon-external-link"></i>
          View Token Details
        </button>
      `;

    const links = externalLinks || {};

    return `
      <div class="pdd-actions-card">
        <div class="pdd-actions-row">
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
  }

  /**
   * Get CSS class for risk level
   */
  _getRiskLevelClass(level) {
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
  }

  /**
   * Get label for risk level
   */
  _getRiskLevelLabel(level) {
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
  }

  _attachActionHandlers(content, pos, isOpen) {
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
          // Close this dialog and open token details
          this.close();
          // The parent page should handle opening token details
          this.onTradeComplete({ action: "view_token", mint: pos.mint });
        };
        viewBtn.addEventListener("click", handler);
        this._actionHandlers.push({ element: viewBtn, handler });
      }
    }
  }

  async _handleAddPosition(pos, btn) {
    if (!this.tradeDialog) {
      this.tradeDialog = new TradeActionDialog();
    }

    const result = await this.tradeDialog.open("add", {
      mint: pos.mint,
      symbol: pos.symbol,
      entrySize: pos.entry_size_sol,
      currentSize: pos.total_size_sol,
    });

    if (result && result.confirmed) {
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
            amount_sol: result.value,
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
  }

  async _handlePartialSell(pos, btn) {
    if (!this.tradeDialog) {
      this.tradeDialog = new TradeActionDialog();
    }

    const result = await this.tradeDialog.open("sell", {
      mint: pos.mint,
      symbol: pos.symbol,
      tokenAmount: pos.remaining_token_amount || pos.token_amount,
    });

    if (result && result.confirmed) {
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
            percentage: result.value,
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
  }

  async _handleClosePosition(pos, btn) {
    if (!this.tradeDialog) {
      this.tradeDialog = new TradeActionDialog();
    }

    // Pre-select 100% for close position
    const result = await this.tradeDialog.open("sell", {
      mint: pos.mint,
      symbol: pos.symbol,
      tokenAmount: pos.remaining_token_amount || pos.token_amount,
      preselect: 100,
    });

    if (result && result.confirmed && result.value === 100) {
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
  }

  // ===========================================================================
  // CHART TAB
  // ===========================================================================

  async _renderChartTab(content) {
    const pos = this.fullDetails?.position || this.positionData;
    const mint = pos?.mint || this.positionData?.mint;
    const entries = this.fullDetails?.entries || [];
    const exits = this.fullDetails?.exits || [];

    if (!mint) {
      content.innerHTML = '<div class="pdd-empty-state">No mint address available</div>';
      return;
    }

    // Build chart UI structure
    content.innerHTML = `
      <div class="pdd-chart-container">
        <div class="pdd-chart-controls">
          <div class="pdd-tf-buttons">
            <button class="pdd-tf-btn${this._chartTimeframe === "1m" ? " active" : ""}" data-tf="1m">1m</button>
            <button class="pdd-tf-btn${this._chartTimeframe === "5m" ? " active" : ""}" data-tf="5m">5m</button>
            <button class="pdd-tf-btn${this._chartTimeframe === "15m" ? " active" : ""}" data-tf="15m">15m</button>
            <button class="pdd-tf-btn${this._chartTimeframe === "1h" ? " active" : ""}" data-tf="1h">1h</button>
            <button class="pdd-tf-btn${this._chartTimeframe === "4h" ? " active" : ""}" data-tf="4h">4h</button>
            <button class="pdd-tf-btn${this._chartTimeframe === "1d" ? " active" : ""}" data-tf="1d">1d</button>
          </div>
          <div class="pdd-chart-legend">
            <span class="pdd-legend-item pdd-legend-price"><span class="pdd-legend-dot"></span> Price</span>
            <span class="pdd-legend-item pdd-legend-entry"><span class="pdd-legend-marker">▲</span> Entry</span>
            <span class="pdd-legend-item pdd-legend-exit"><span class="pdd-legend-marker">▼</span> Exit</span>
          </div>
        </div>
        <div class="pdd-chart-area" id="pddChartArea">
          <div class="loading-spinner">Loading chart data...</div>
        </div>
        <div class="pdd-chart-stats" id="pddChartStats"></div>
      </div>
    `;

    // Attach timeframe button handlers
    this._attachChartEventHandlers(content, mint, entries, exits);

    // Fetch and render chart data
    await this._fetchAndRenderChart(mint, entries, exits);
  }

  _attachChartEventHandlers(content, mint, entries, exits) {
    // Clean up old handlers
    if (this._tfButtonHandlers) {
      this._tfButtonHandlers.forEach(({ element, handler }) => {
        element.removeEventListener("click", handler);
      });
    }
    this._tfButtonHandlers = [];

    const tfButtons = content.querySelectorAll(".pdd-tf-btn");
    tfButtons.forEach((btn) => {
      const handler = async () => {
        const tf = btn.dataset.tf;
        if (tf === this._chartTimeframe) return;

        // Update active state
        tfButtons.forEach((b) => b.classList.remove("active"));
        btn.classList.add("active");

        this._chartTimeframe = tf;
        await this._fetchAndRenderChart(mint, entries, exits);
      };
      btn.addEventListener("click", handler);
      this._tfButtonHandlers.push({ element: btn, handler });
    });
  }

  async _fetchAndRenderChart(mint, entries, exits) {
    const chartArea = this.dialogEl?.querySelector("#pddChartArea");
    const statsArea = this.dialogEl?.querySelector("#pddChartStats");
    if (!chartArea) return;

    chartArea.innerHTML = '<div class="loading-spinner">Loading chart data...</div>';

    try {
      const data = await requestManager.fetch(
        `/api/ohlcv/${mint}?timeframe=${this._chartTimeframe}&limit=200`,
        { priority: "normal" }
      );

      if (!data?.data || data.data.length === 0) {
        chartArea.innerHTML = `
          <div class="pdd-chart-empty">
            <i class="icon-bar-chart-2"></i>
            <p>No OHLCV data available for this timeframe</p>
            <span class="pdd-chart-empty-hint">Data may not be collected for this token yet</span>
          </div>
        `;
        if (statsArea) statsArea.innerHTML = "";
        return;
      }

      this._chartData = data.data;
      this._renderChartSVG(chartArea, data.data, entries, exits);
      this._renderChartStats(statsArea, data.data);
    } catch (error) {
      console.error("Error fetching OHLCV data:", error);
      chartArea.innerHTML = `
        <div class="pdd-chart-empty pdd-chart-error">
          <i class="icon-alert-circle"></i>
          <p>Failed to load chart data</p>
          <span class="pdd-chart-empty-hint">${Utils.escapeHtml(error?.message || "Unknown error")}</span>
        </div>
      `;
      if (statsArea) statsArea.innerHTML = "";
    }
  }

  _renderChartSVG(container, candles, entries, exits) {
    const width = container.clientWidth || 700;
    const height = 280;
    const padding = { top: 20, right: 60, bottom: 30, left: 10 };
    const chartWidth = width - padding.left - padding.right;
    const chartHeight = height - padding.top - padding.bottom;

    // Get price range
    const prices = candles.flatMap((c) => [c.high, c.low]);
    const minPrice = Math.min(...prices);
    const maxPrice = Math.max(...prices);
    const priceRange = maxPrice - minPrice || 1;
    const pricePadding = priceRange * 0.05;
    const yMin = minPrice - pricePadding;
    const yMax = maxPrice + pricePadding;

    // Time range
    const timestamps = candles.map((c) => c.timestamp);
    const minTime = Math.min(...timestamps);
    const maxTime = Math.max(...timestamps);
    const timeRange = maxTime - minTime || 1;

    // Scale functions
    const scaleX = (t) => padding.left + ((t - minTime) / timeRange) * chartWidth;
    const scaleY = (p) => padding.top + (1 - (p - yMin) / (yMax - yMin)) * chartHeight;

    // Build candlestick bars
    const candleWidth = Math.max(2, (chartWidth / candles.length) * 0.7);
    const candlesHtml = candles
      .map((c) => {
        const x = scaleX(c.timestamp);
        const isGreen = c.close >= c.open;
        const color = isGreen ? "var(--success)" : "var(--error)";
        const yHigh = scaleY(c.high);
        const yLow = scaleY(c.low);
        const yOpen = scaleY(c.open);
        const yClose = scaleY(c.close);
        const bodyTop = Math.min(yOpen, yClose);
        const bodyHeight = Math.max(1, Math.abs(yClose - yOpen));

        return `
          <line x1="${x}" y1="${yHigh}" x2="${x}" y2="${yLow}" stroke="${color}" stroke-width="1" />
          <rect x="${x - candleWidth / 2}" y="${bodyTop}" width="${candleWidth}" height="${bodyHeight}" fill="${color}" />
        `;
      })
      .join("");

    // Build entry markers
    const entryMarkersHtml = entries
      .map((entry) => {
        const ts = entry.timestamp;
        if (ts < minTime || ts > maxTime) return "";
        const x = scaleX(ts);
        const price = entry.price;
        const y = scaleY(price);
        return `
          <polygon points="${x},${y + 8} ${x - 6},${y + 16} ${x + 6},${y + 16}" fill="var(--success)" stroke="#fff" stroke-width="1" />
          <title>Entry: ${this._formatPrice(price)} SOL at ${Utils.formatTimestamp(ts)}</title>
        `;
      })
      .join("");

    // Build exit markers
    const exitMarkersHtml = exits
      .map((exit) => {
        const ts = exit.timestamp;
        if (ts < minTime || ts > maxTime) return "";
        const x = scaleX(ts);
        const price = exit.price;
        const y = scaleY(price);
        return `
          <polygon points="${x},${y - 8} ${x - 6},${y - 16} ${x + 6},${y - 16}" fill="var(--error)" stroke="#fff" stroke-width="1" />
          <title>Exit: ${this._formatPrice(price)} SOL at ${Utils.formatTimestamp(ts)}</title>
        `;
      })
      .join("");

    // Y-axis labels
    const yAxisSteps = 5;
    const yAxisLabels = [];
    for (let i = 0; i <= yAxisSteps; i++) {
      const price = yMin + ((yMax - yMin) * i) / yAxisSteps;
      const y = scaleY(price);
      yAxisLabels.push(`
        <text x="${width - padding.right + 5}" y="${y + 4}" fill="var(--text-secondary)" font-size="10" font-family="JetBrains Mono, monospace">${this._formatPrice(price)}</text>
        <line x1="${padding.left}" y1="${y}" x2="${width - padding.right}" y2="${y}" stroke="var(--border-color)" stroke-width="1" stroke-dasharray="2,2" opacity="0.3" />
      `);
    }

    // X-axis labels (show a few timestamps)
    const xAxisLabels = [];
    const xLabelCount = Math.min(6, candles.length);
    const step = Math.floor(candles.length / xLabelCount) || 1;
    for (let i = 0; i < candles.length; i += step) {
      const c = candles[i];
      const x = scaleX(c.timestamp);
      const date = new Date(c.timestamp * 1000);
      const label = `${date.getHours().toString().padStart(2, "0")}:${date.getMinutes().toString().padStart(2, "0")}`;
      xAxisLabels.push(`
        <text x="${x}" y="${height - 5}" fill="var(--text-secondary)" font-size="10" font-family="JetBrains Mono, monospace" text-anchor="middle">${label}</text>
      `);
    }

    // Current price line
    const currentPrice = candles[candles.length - 1]?.close;
    const currentPriceLine = currentPrice
      ? `<line x1="${padding.left}" y1="${scaleY(currentPrice)}" x2="${width - padding.right}" y2="${scaleY(currentPrice)}" stroke="var(--link-color)" stroke-width="1" stroke-dasharray="4,2" />`
      : "";

    container.innerHTML = `
      <svg width="100%" height="${height}" viewBox="0 0 ${width} ${height}" preserveAspectRatio="xMidYMid meet" class="pdd-chart-svg">
        ${yAxisLabels.join("")}
        ${xAxisLabels.join("")}
        ${candlesHtml}
        ${currentPriceLine}
        <g class="pdd-chart-markers">
          ${entryMarkersHtml}
          ${exitMarkersHtml}
        </g>
      </svg>
    `;
  }

  _renderChartStats(container, candles) {
    if (!container || !candles || candles.length === 0) return;

    const first = candles[0];
    const last = candles[candles.length - 1];
    const high = Math.max(...candles.map((c) => c.high));
    const low = Math.min(...candles.map((c) => c.low));
    const priceChange = last.close - first.open;
    const priceChangePct = first.open !== 0 ? (priceChange / first.open) * 100 : 0;
    const totalVolume = candles.reduce((sum, c) => sum + c.volume, 0);

    const changeClass = priceChange >= 0 ? "pdd-positive" : "pdd-negative";
    const changeSign = priceChange >= 0 ? "+" : "";

    container.innerHTML = `
      <div class="pdd-chart-stat-grid">
        <div class="pdd-chart-stat">
          <span class="label">Open</span>
          <span class="value">${this._formatPrice(first.open)}</span>
        </div>
        <div class="pdd-chart-stat">
          <span class="label">High</span>
          <span class="value pdd-positive">${this._formatPrice(high)}</span>
        </div>
        <div class="pdd-chart-stat">
          <span class="label">Low</span>
          <span class="value pdd-negative">${this._formatPrice(low)}</span>
        </div>
        <div class="pdd-chart-stat">
          <span class="label">Close</span>
          <span class="value">${this._formatPrice(last.close)}</span>
        </div>
        <div class="pdd-chart-stat">
          <span class="label">Change</span>
          <span class="value ${changeClass}">${changeSign}${Utils.formatNumber(priceChangePct, 2)}%</span>
        </div>
        <div class="pdd-chart-stat">
          <span class="label">Volume</span>
          <span class="value">${Utils.formatCompactNumber(totalVolume)}</span>
        </div>
      </div>
    `;
  }

  // ===========================================================================
  // HISTORY TAB
  // ===========================================================================

  _renderHistoryTab(content) {
    const entries = this.fullDetails?.entries || [];
    const exits = this.fullDetails?.exits || [];

    // Combine and sort by timestamp (newest first)
    const timeline = [
      ...entries.map((e) => ({ ...e, type: "entry" })),
      ...exits.map((e) => ({ ...e, type: "exit" })),
    ].sort((a, b) => b.timestamp - a.timestamp);

    if (timeline.length === 0) {
      content.innerHTML = '<div class="pdd-empty-state">No history available</div>';
      return;
    }

    const timelineHtml = timeline
      .map((item) => {
        const isEntry = item.type === "entry";
        const typeClass = isEntry ? "pdd-timeline-entry" : "pdd-timeline-exit";
        const icon = isEntry ? "icon-arrow-down-circle" : "icon-arrow-up-circle";
        const label = isEntry ? "Entry" : "Exit";

        let badges = "";
        if (isEntry && item.is_dca) {
          badges += '<span class="pdd-badge pdd-badge-info">DCA</span>';
        }
        if (!isEntry && item.is_partial) {
          badges += `<span class="pdd-badge pdd-badge-warning">${item.percentage}%</span>`;
        }

        const signature = item.transaction_signature;
        const shortSig = signature ? `${signature.slice(0, 8)}...${signature.slice(-8)}` : "—";

        return `
          <div class="pdd-timeline-item ${typeClass}">
            <div class="pdd-timeline-icon">
              <i class="${icon}"></i>
            </div>
            <div class="pdd-timeline-content">
              <div class="pdd-timeline-header">
                <span class="pdd-timeline-label">${label}</span>
                ${badges}
                <span class="pdd-timeline-time" title="${Utils.formatTimestamp(item.timestamp)}">${Utils.formatTimeAgo(item.timestamp)}</span>
              </div>
              <div class="pdd-timeline-details">
                <div class="pdd-timeline-stat">
                  <span class="label">Amount</span>
                  <span class="value">${Utils.formatCompactNumber(item.amount)} tokens</span>
                </div>
                <div class="pdd-timeline-stat">
                  <span class="label">Price</span>
                  <span class="value">${this._formatPrice(item.price)} SOL</span>
                </div>
                <div class="pdd-timeline-stat">
                  <span class="label">${isEntry ? "SOL Spent" : "SOL Received"}</span>
                  <span class="value">${Utils.formatSol(isEntry ? item.sol_spent : item.sol_received, { decimals: 4 })}</span>
                </div>
              </div>
              <div class="pdd-timeline-signature">
                <span class="pdd-signature-text" data-signature="${signature || ""}" title="Click to copy">${shortSig}</span>
                <a href="https://solscan.io/tx/${signature || ""}" target="_blank" class="pdd-signature-link" title="View on Solscan">
                  <i class="icon-external-link"></i>
                </a>
              </div>
            </div>
          </div>
        `;
      })
      .join("");

    content.innerHTML = `
      <div class="pdd-timeline">
        ${timelineHtml}
      </div>
    `;

    // Attach copy handlers for signatures
    content.querySelectorAll(".pdd-signature-text").forEach((el) => {
      el.addEventListener("click", () => {
        const sig = el.dataset.signature;
        if (sig) {
          Utils.copyToClipboard(sig);
          Utils.showToast("Signature copied!", "success");
        }
      });
    });
  }

  // ===========================================================================
  // TRANSACTIONS TAB
  // ===========================================================================

  _renderTransactionsTab(content) {
    const allTransactions = this.fullDetails?.transactions || [];
    const entries = this.fullDetails?.entries || [];
    const exits = this.fullDetails?.exits || [];
    const stateHistory = this.fullDetails?.state_history || [];

    // Filter out unavailable exit transactions (shown for open positions with no actual exit)
    const transactions = allTransactions.filter((tx) => {
      // Keep all available transactions
      if (tx.available !== false) return true;
      // For unavailable ones, only keep if it's NOT an exit with no actual exits
      if (tx.kind === "exit" && exits.length === 0) return false;
      return true;
    });

    if (transactions.length === 0 && stateHistory.length === 0) {
      content.innerHTML = '<div class="pdd-empty-state">No transactions available</div>';
      return;
    }

    // Merge entry/exit info with transactions for enhanced display
    const entrySignatures = new Set(entries.map((e) => e.transaction_signature));
    const exitSignatures = new Set(exits.map((e) => e.transaction_signature));

    // Build filter buttons
    const filterButtonsHtml = `
      <div class="pdd-tx-filters">
        <button class="pdd-filter-btn active" data-filter="all">All</button>
        <button class="pdd-filter-btn" data-filter="entry">Entries</button>
        <button class="pdd-filter-btn" data-filter="exit">Exits</button>
      </div>
    `;

    // Build transaction cards
    const txCardsHtml = transactions
      .map((tx) => {
        const signature = tx.signature || "";
        const shortSig = signature ? `${signature.slice(0, 8)}...${signature.slice(-8)}` : "—";

        // Determine transaction type
        let txType = "unknown";
        let txTypeLabel = "Transaction";
        let txTypeClass = "";

        if (entrySignatures.has(signature)) {
          const entryRecord = entries.find((e) => e.transaction_signature === signature);
          txType = "entry";
          txTypeLabel = entryRecord?.is_dca ? "DCA Entry" : "Entry";
          txTypeClass = "pdd-tx-type-entry";
        } else if (exitSignatures.has(signature)) {
          const exitRecord = exits.find((e) => e.transaction_signature === signature);
          txType = "exit";
          txTypeLabel = exitRecord?.is_partial ? "Partial Exit" : "Exit";
          txTypeClass = "pdd-tx-type-exit";
        } else if (tx.kind) {
          txType = tx.kind.toLowerCase();
          txTypeLabel = this._formatTransactionType(tx.kind);
        }

        // Status
        const isSuccess = tx.success !== false;
        const isPending = tx.status === "pending";
        const statusClass = isPending ? "pdd-status-pending" : isSuccess ? "pdd-status-success" : "pdd-status-failed";
        const statusLabel = isPending ? "Pending" : isSuccess ? "Confirmed" : "Failed";
        const statusIcon = isPending ? "icon-clock" : isSuccess ? "icon-check-circle" : "icon-x-circle";

        // Amount info
        const solChange = tx.sol_change;
        const solChangeHtml = solChange
          ? `<div class="pdd-tx-sol-change ${solChange > 0 ? "positive" : "negative"}">
              ${solChange > 0 ? "+" : ""}${Utils.formatSol(solChange, { decimals: 6, suffix: "" })} SOL
            </div>`
          : "";

        // Get token amount from entry/exit records
        let tokenAmountHtml = "";
        if (txType === "entry") {
          const entryRecord = entries.find((e) => e.transaction_signature === signature);
          if (entryRecord?.amount) {
            const symbol = this.fullDetails?.position?.symbol || "tokens";
            tokenAmountHtml = `<div class="pdd-tx-token-amount">+${Utils.formatCompactNumber(entryRecord.amount)} ${symbol}</div>`;
          }
        } else if (txType === "exit") {
          const exitRecord = exits.find((e) => e.transaction_signature === signature);
          if (exitRecord?.amount) {
            const symbol = this.fullDetails?.position?.symbol || "tokens";
            tokenAmountHtml = `<div class="pdd-tx-token-amount">-${Utils.formatCompactNumber(exitRecord.amount)} ${symbol}</div>`;
          }
        }

        // Fee display
        const feeSol = tx.fee_sol ? Utils.formatSol(tx.fee_sol, { decimals: 6 }) : null;
        const feeHtml = feeSol ? `<div class="pdd-tx-fee"><span class="label">Fee:</span> ${feeSol}</div>` : "";

        // P&L for exits
        let pnlHtml = "";
        if (txType === "exit") {
          const exitRecord = exits.find((e) => e.transaction_signature === signature);
          if (exitRecord?.sol_received) {
            // Find matching entry to calculate P&L
            const entryPrice = this.fullDetails?.position?.effective_entry_price || 0;
            if (entryPrice && exitRecord.price) {
              const pnlPercent = ((exitRecord.price - entryPrice) / entryPrice) * 100;
              pnlHtml = `<div class="pdd-tx-pnl ${pnlPercent >= 0 ? "positive" : "negative"}">
                P&L: ${pnlPercent >= 0 ? "+" : ""}${pnlPercent.toFixed(2)}%
              </div>`;
            }
          }
        }

        // Router info
        const routerHtml = tx.router ? `<div class="pdd-tx-router">${Utils.escapeHtml(tx.router)}</div>` : "";

        return `
          <div class="pdd-tx-card" data-tx-type="${txType}">
            <div class="pdd-tx-card-header">
              <div class="pdd-tx-type-badge ${txTypeClass}">${txTypeLabel}</div>
              <div class="pdd-tx-status-badge ${statusClass}">
                <i class="${statusIcon}"></i>
                <span>${statusLabel}</span>
              </div>
            </div>
            <div class="pdd-tx-card-body">
              <div class="pdd-tx-amounts">
                ${tokenAmountHtml}
                ${solChangeHtml}
              </div>
              <div class="pdd-tx-details">
                ${feeHtml}
                ${pnlHtml}
                ${routerHtml}
              </div>
            </div>
            <div class="pdd-tx-card-footer">
              <div class="pdd-tx-signature-row">
                <span class="pdd-tx-sig" data-signature="${signature}" title="Click to copy">
                  ${shortSig}
                </span>
                <button class="pdd-tx-copy-btn" data-signature="${signature}" title="Copy signature">
                  <i class="icon-copy"></i>
                </button>
              </div>
              <div class="pdd-tx-time" title="${Utils.formatTimestamp(tx.timestamp)}">
                ${Utils.formatTimeAgo(tx.timestamp)}
              </div>
              <a href="https://solscan.io/tx/${signature}" target="_blank" class="pdd-tx-explorer" title="View on Solscan">
                <i class="icon-external-link"></i>
              </a>
            </div>
          </div>
        `;
      })
      .join("");

    // State history timeline
    const stateHistoryHtml =
      stateHistory.length > 0
        ? `
        <div class="pdd-section-header">State History</div>
        <div class="pdd-state-history">
          ${stateHistory
            .map((state) => {
              return `
              <div class="pdd-state-item">
                <span class="pdd-state-name">${Utils.escapeHtml(state.state)}</span>
                <span class="pdd-state-time" title="${Utils.formatTimestamp(state.changed_at)}">${Utils.formatTimeAgo(state.changed_at)}</span>
                ${state.reason ? `<span class="pdd-state-reason">${Utils.escapeHtml(state.reason)}</span>` : ""}
              </div>
            `;
            })
            .join("")}
        </div>
      `
        : "";

    content.innerHTML = `
      <div class="pdd-transactions-container">
        ${filterButtonsHtml}
        <div class="pdd-tx-cards">
          ${txCardsHtml || '<div class="pdd-empty-state">No transactions</div>'}
        </div>
        ${stateHistoryHtml}
      </div>
    `;

    // Clean up old filter handlers
    if (this._filterHandlers) {
      this._filterHandlers.forEach(({ element, handler }) => {
        element.removeEventListener("click", handler);
      });
    }
    this._filterHandlers = [];

    // Attach filter handlers with tracking
    content.querySelectorAll(".pdd-filter-btn").forEach((btn) => {
      const handler = () => {
        // Update active button
        content.querySelectorAll(".pdd-filter-btn").forEach((b) => b.classList.remove("active"));
        btn.classList.add("active");

        // Filter transactions
        const filter = btn.dataset.filter;
        content.querySelectorAll(".pdd-tx-card").forEach((card) => {
          const txType = card.dataset.txType;
          if (filter === "all") {
            card.style.display = "";
          } else if (filter === "entry" && txType === "entry") {
            card.style.display = "";
          } else if (filter === "exit" && txType === "exit") {
            card.style.display = "";
          } else if (filter !== "all") {
            card.style.display = "none";
          }
        });
      };
      btn.addEventListener("click", handler);
      this._filterHandlers.push({ element: btn, handler });
    });

    // Attach copy handlers for signatures
    content.querySelectorAll(".pdd-tx-sig, .pdd-tx-copy-btn").forEach((el) => {
      el.addEventListener("click", (e) => {
        e.preventDefault();
        const sig = el.dataset.signature;
        if (sig) {
          Utils.copyToClipboard(sig);
          Utils.showToast("Signature copied!", "success");
        }
      });
    });
  }

  // ===========================================================================
  // ANALYTICS TAB
  // ===========================================================================

  _renderAnalyticsTab(content) {
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
        ${this._buildPriceAnalysis(pos)}
        ${this._buildDcaSummary(pos, entries)}
        ${this._buildExitSummary(pos, exits, solPriceUsd)}
        ${this._buildFeeAnalysis(pos, entries, exits)}
      </div>
    `;
  }

  /**
   * Build performance summary card for analytics tab
   */
  _buildPerformanceSummary(pos, isOpen, positionAge, solPriceUsd) {
    // Calculate ROI
    const totalInvested = pos.total_size_sol || 0;
    const currentValue = (pos.current_price || 0) * (pos.remaining_token_amount || 0);
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
  }

  /**
   * Build price analysis card for analytics tab
   */
  _buildPriceAnalysis(pos) {
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
        <h4><i class="icon-bar-chart-2"></i> Price Analysis</h4>
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
  }

  /**
   * Build DCA summary card for analytics tab (only if multiple entries)
   */
  _buildDcaSummary(pos, entries) {
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
          ${minEntry !== null && maxEntry !== null ? `
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Entry Price Range</span>
            <span class="pdd-row-value">${this._formatPrice(minEntry)} - ${this._formatPrice(maxEntry)} SOL</span>
          </div>
          ` : ""}
          ${totalTokensAcquired > 0 ? `
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Total Tokens Acquired</span>
            <span class="pdd-row-value">${Utils.formatCompactNumber(totalTokensAcquired)}</span>
          </div>
          ` : ""}
          ${totalSolSpent > 0 ? `
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Total SOL Spent</span>
            <span class="pdd-row-value">${Utils.formatSol(totalSolSpent, { decimals: 4 })}</span>
          </div>
          ` : ""}
        </div>
      </div>
    `;
  }

  /**
   * Build exit summary card for analytics tab (only if partial exits occurred)
   */
  _buildExitSummary(pos, exits, solPriceUsd) {
    const exitCount = pos.partial_exit_count || exits.length || 0;

    // Only show if there are exits
    if (exitCount === 0 && exits.length === 0) {
      return "";
    }

    const tokensSold = pos.total_exited_amount || exits.reduce((sum, e) => sum + (e.amount || 0), 0);
    const originalTokens = pos.token_amount || 0;
    const pctSold = originalTokens > 0 ? (tokensSold / originalTokens) * 100 : 0;
    const solReceived = pos.sol_received || exits.reduce((sum, e) => sum + (e.sol_received || 0), 0);

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
          ${realizedPnl !== undefined && realizedPnl !== null ? `
          <div class="pdd-analysis-row">
            <span class="pdd-row-label">Realized P&L</span>
            <span class="pdd-row-value ${pnlClass}">${pnlSign}${Utils.formatSol(realizedPnl, { decimals: 4, suffix: "" })} SOL ${realizedPnlPct !== undefined ? `<span class="pdd-row-pct">(${pnlSign}${Utils.formatNumber(realizedPnlPct, 2)}%)</span>` : ""}</span>
          </div>
          ` : ""}
        </div>
      </div>
    `;
  }

  /**
   * Build fee analysis card for analytics tab
   */
  _buildFeeAnalysis(pos, entries, exits) {
    // Calculate entry fees from entries array or position summary
    let entryFees = 0;
    if (pos.entry_fee_lamports) {
      entryFees = pos.entry_fee_lamports / 1e9;
    } else if (entries.length > 0) {
      entryFees = entries.reduce((sum, e) => sum + ((e.fee_lamports || 0) / 1e9), 0);
    }

    // Calculate exit fees from exits array or position summary
    let exitFees = 0;
    if (pos.exit_fee_lamports) {
      exitFees = pos.exit_fee_lamports / 1e9;
    } else if (exits.length > 0) {
      exitFees = exits.reduce((sum, e) => sum + ((e.fee_lamports || 0) / 1e9), 0);
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
  }

  // ===========================================================================
  // TOKEN TAB
  // ===========================================================================

  _renderTokenTab(content) {
    const pos = this.fullDetails?.position;
    const tokenInfo = this.fullDetails?.token_info;
    const security = this.fullDetails?.security;
    const poolInfo = this.fullDetails?.pool_info;
    const marketData = this.fullDetails?.market_data;
    const externalLinks = this.fullDetails?.external_links;

    if (!pos) {
      content.innerHTML = '<div class="pdd-empty-state">No token data available</div>';
      return;
    }

    // Token identity section
    const logoHtml = tokenInfo?.image_url
      ? `<img class="pdd-token-logo-lg" src="${tokenInfo.image_url}" alt="${pos.symbol || ""}" />`
      : `<div class="pdd-token-logo-lg pdd-logo-placeholder">${(pos.symbol || "?").slice(0, 2).toUpperCase()}</div>`;

    const descriptionHtml = tokenInfo?.description
      ? `<p class="pdd-token-desc">${Utils.escapeHtml(tokenInfo.description)}</p>`
      : "";

    // Security assessment section
    const score = security?.score_normalized ?? null;
    const riskLevel = security?.risk_level || "unknown";
    const riskLabelClass = {
      low: "pdd-risk-low",
      medium: "pdd-risk-medium",
      high: "pdd-risk-high",
      unknown: "pdd-risk-unknown",
    }[riskLevel] || "pdd-risk-unknown";

    const riskLabelText = {
      low: "Low Risk",
      medium: "Medium Risk",
      high: "High Risk",
      unknown: "Unknown",
    }[riskLevel] || "Unknown";

    const mintAuthorityHtml = security !== undefined
      ? `<div class="pdd-authority-row">
          <span class="label">Mint Authority:</span>
          <span class="pdd-status ${security.has_mint_authority ? "warning" : "safe"}">
            ${security.has_mint_authority ? "Active ⚠️" : "Revoked ✓"}
          </span>
        </div>`
      : "";

    const freezeAuthorityHtml = security !== undefined
      ? `<div class="pdd-authority-row">
          <span class="label">Freeze Authority:</span>
          <span class="pdd-status ${security.has_freeze_authority ? "warning" : "safe"}">
            ${security.has_freeze_authority ? "Active ⚠️" : "Revoked ✓"}
          </span>
        </div>`
      : "";

    const risksListHtml = security?.top_risks?.length
      ? `<div class="pdd-token-risks">
          <h5>Risk Factors</h5>
          <ul>
            ${security.top_risks.map((risk) => `<li>${Utils.escapeHtml(risk)}</li>`).join("")}
          </ul>
        </div>`
      : "";

    // Social links section
    const socialLinks = [];
    if (tokenInfo?.website) {
      socialLinks.push(`<a href="${tokenInfo.website}" target="_blank" class="pdd-social-link">🌐 Website</a>`);
    }
    if (tokenInfo?.twitter) {
      socialLinks.push(`<a href="${tokenInfo.twitter}" target="_blank" class="pdd-social-link">🐦 Twitter</a>`);
    }
    if (tokenInfo?.telegram) {
      socialLinks.push(`<a href="${tokenInfo.telegram}" target="_blank" class="pdd-social-link">📱 Telegram</a>`);
    }

    const socialLinksHtml =
      socialLinks.length > 0
        ? `<div class="pdd-social-links">
            ${socialLinks.join("")}
          </div>`
        : "";

    // Pool info section
    const poolInfoHtml = poolInfo
      ? `<div class="pdd-pool-info">
          <h4>Pool Information</h4>
          <div class="pdd-pool-grid">
            ${poolInfo.dex_name ? `<div class="pdd-pool-row"><span class="label">DEX:</span><span class="value">${Utils.escapeHtml(poolInfo.dex_name)}</span></div>` : ""}
            ${poolInfo.pool_address ? `<div class="pdd-pool-row"><span class="label">Pool:</span><span class="value pdd-address" data-address="${poolInfo.pool_address}">${poolInfo.pool_address.slice(0, 8)}...${poolInfo.pool_address.slice(-8)}</span></div>` : ""}
            ${poolInfo.liquidity_sol !== null && poolInfo.liquidity_sol !== undefined ? `<div class="pdd-pool-row"><span class="label">Liquidity:</span><span class="value">${Utils.formatSol(poolInfo.liquidity_sol, { suffix: "" })} SOL</span></div>` : ""}
          </div>
        </div>`
      : "";

    // Market data section
    const marketDataHtml = marketData
      ? `<div class="pdd-market-data">
          <h4>Market Data</h4>
          <div class="pdd-market-grid">
            ${marketData.market_cap ? `<div class="pdd-market-item"><span class="label">Market Cap</span><span class="value">${Utils.formatCurrencyUSD(marketData.market_cap)}</span></div>` : ""}
            ${marketData.fdv ? `<div class="pdd-market-item"><span class="label">FDV</span><span class="value">${Utils.formatCurrencyUSD(marketData.fdv)}</span></div>` : ""}
            ${marketData.liquidity_usd ? `<div class="pdd-market-item"><span class="label">Liquidity</span><span class="value">${Utils.formatCurrencyUSD(marketData.liquidity_usd)}</span></div>` : ""}
            ${marketData.volume_24h ? `<div class="pdd-market-item"><span class="label">24h Volume</span><span class="value">${Utils.formatCurrencyUSD(marketData.volume_24h)}</span></div>` : ""}
            ${marketData.price_change_h1 !== null && marketData.price_change_h1 !== undefined ? `<div class="pdd-market-item"><span class="label">1h Change</span><span class="value ${marketData.price_change_h1 >= 0 ? "positive" : "negative"}">${marketData.price_change_h1 >= 0 ? "+" : ""}${marketData.price_change_h1.toFixed(2)}%</span></div>` : ""}
            ${marketData.price_change_h24 !== null && marketData.price_change_h24 !== undefined ? `<div class="pdd-market-item"><span class="label">24h Change</span><span class="value ${marketData.price_change_h24 >= 0 ? "positive" : "negative"}">${marketData.price_change_h24 >= 0 ? "+" : ""}${marketData.price_change_h24.toFixed(2)}%</span></div>` : ""}
            ${marketData.holder_count ? `<div class="pdd-market-item"><span class="label">Holders</span><span class="value">${Utils.formatCompactNumber(marketData.holder_count)}</span></div>` : ""}
          </div>
        </div>`
      : "";

    // External links section
    const explorerLinksHtml = externalLinks
      ? `<div class="pdd-explorer-links">
          <h4>Explore</h4>
          <div class="pdd-explorer-grid">
            <a href="${externalLinks.solscan}" target="_blank" class="pdd-explorer-link">Solscan</a>
            <a href="${externalLinks.dexscreener}" target="_blank" class="pdd-explorer-link">DexScreener</a>
            <a href="${externalLinks.birdeye}" target="_blank" class="pdd-explorer-link">Birdeye</a>
            <a href="${externalLinks.rugcheck}" target="_blank" class="pdd-explorer-link">RugCheck</a>
            <a href="${externalLinks.photon}" target="_blank" class="pdd-explorer-link">Photon</a>
          </div>
        </div>`
      : "";

    content.innerHTML = `
      <div class="pdd-token-tab">
        <div class="pdd-token-identity">
          ${logoHtml}
          <div class="pdd-token-details">
            <h3>${Utils.escapeHtml(pos.name || pos.symbol || "Unknown Token")} ${pos.symbol ? `(${Utils.escapeHtml(pos.symbol)})` : ""}</h3>
            <div class="pdd-token-address-row">
              <span class="pdd-token-address" title="${pos.mint}">${pos.mint.slice(0, 12)}...${pos.mint.slice(-12)}</span>
              <button class="pdd-copy-btn" data-copy="${pos.mint}" title="Copy address">
                <i class="icon-copy"></i>
              </button>
            </div>
            ${descriptionHtml}
          </div>
        </div>

        ${security ? `
        <div class="pdd-security-full">
          <h4>Security Assessment</h4>
          <div class="pdd-security-header">
            <div class="pdd-score-display">
              ${score !== null ? `<div class="pdd-score-circle" style="--score: ${score}; --score-color: ${this._getScoreColor(score)}">${score}</div>` : '<div class="pdd-score-circle pdd-score-unknown">?</div>'}
              <span class="pdd-risk-label ${riskLabelClass}">${riskLabelText}</span>
            </div>
          </div>
          <div class="pdd-security-details">
            ${mintAuthorityHtml}
            ${freezeAuthorityHtml}
          </div>
          ${risksListHtml}
        </div>
        ` : ""}

        ${socialLinksHtml}
        ${marketDataHtml}
        ${poolInfoHtml}
        ${explorerLinksHtml}
      </div>
    `;

    // Attach copy handlers
    content.querySelectorAll(".pdd-copy-btn").forEach((btn) => {
      btn.addEventListener("click", () => {
        const text = btn.dataset.copy;
        if (text) {
          Utils.copyToClipboard(text);
          Utils.showToast("Copied!", "success");
        }
      });
    });

    content.querySelectorAll(".pdd-address").forEach((el) => {
      el.style.cursor = "pointer";
      el.addEventListener("click", () => {
        const address = el.dataset.address;
        if (address) {
          Utils.copyToClipboard(address);
          Utils.showToast("Address copied!", "success");
        }
      });
    });
  }

  /**
   * Get color for security score
   */
  _getScoreColor(score) {
    if (score >= 80) return "var(--success)";
    if (score >= 50) return "var(--warning)";
    return "var(--error)";
  }

  // ===========================================================================
  // UTILITY METHODS
  // ===========================================================================

  /**
   * Format price with appropriate precision
   * Uses subscript notation for very small prices
   */
  _formatPrice(price) {
    if (price === null || price === undefined) return "—";
    return Utils.formatPriceSubscript(price, { precision: 5 });
  }

  /**
   * Format transaction type for display
   */
  _formatTransactionType(type) {
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
  }
}
