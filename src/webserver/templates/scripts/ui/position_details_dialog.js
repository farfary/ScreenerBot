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
    this._escapeHandler = null;
    this._closeHandler = null;
    this._backdropHandler = null;
    this._copyMintHandler = null;
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
          <button class="tab-button" data-tab="history">
            <i class="icon-clock"></i>
            History
          </button>
          <button class="tab-button" data-tab="transactions">
            <i class="icon-list"></i>
            Transactions
          </button>
        </div>

        <div class="dialog-body">
          <div class="tab-content active" data-tab-content="overview">
            <div class="loading-spinner">Loading position details...</div>
          </div>
          <div class="tab-content" data-tab-content="history">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="transactions">
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
      const pnlClass = pnl >= 0 ? "pdd-positive" : "pdd-negative";
      const sign = pnl >= 0 ? "+" : "";
      pnlHtml = `
        <div class="pnl-block ${pnlClass}">
          <span class="pnl-value">${sign}${Utils.formatSol(pnl, { decimals: 4, suffix: "" })}</span>
          <span class="pnl-percent">${sign}${Utils.formatNumber(pnlPct, 2)}%</span>
        </div>
      `;
    } else if (!isOpen && pos.summary?.pnl !== undefined) {
      const pnl = pos.summary.pnl;
      const pnlPct = pos.summary.pnl_percent;
      const pnlClass = pnl >= 0 ? "pdd-positive" : "pdd-negative";
      const sign = pnl >= 0 ? "+" : "";
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
      case "history":
        this._renderHistoryTab(content);
        break;
      case "transactions":
        this._renderTransactionsTab(content);
        break;
    }
  }

  // ===========================================================================
  // OVERVIEW TAB
  // ===========================================================================

  _renderOverviewTab(content) {
    const pos = this.fullDetails?.position?.summary;
    if (!pos) {
      content.innerHTML = '<div class="pdd-empty-state">No position data available</div>';
      return;
    }

    const isOpen = pos.position_type !== "closed";

    content.innerHTML = `
      <div class="pdd-overview-grid">
        ${this._buildEntryDetailsCard(pos)}
        ${this._buildCurrentStateCard(pos, isOpen)}
        ${this._buildPnLSummaryCard(pos, isOpen)}
        ${this._buildActionsCard(pos, isOpen)}
      </div>
    `;

    // Attach action button handlers
    this._attachActionHandlers(content, pos, isOpen);
  }

  _buildEntryDetailsCard(pos) {
    const avgEntryPrice = pos.average_entry_price || pos.entry_price;
    const totalInvested = pos.total_size_sol;
    const dcaCount = pos.dca_count || 0;
    const initialEntryTime = pos.entry_time;

    return `
      <div class="pdd-card">
        <h3 class="pdd-card-title">
          <i class="icon-arrow-down-circle"></i>
          Entry Details
        </h3>
        <div class="pdd-card-content">
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Avg Entry Price</span>
            <span class="pdd-stat-value">${this._formatPrice(avgEntryPrice)} SOL</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Total Invested</span>
            <span class="pdd-stat-value">${Utils.formatSol(totalInvested, { decimals: 4 })}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">DCA Count</span>
            <span class="pdd-stat-value">${dcaCount} ${dcaCount > 0 ? '<span class="pdd-badge pdd-badge-info">DCA</span>' : ""}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Initial Entry</span>
            <span class="pdd-stat-value" title="${Utils.formatTimestamp(initialEntryTime)}">${Utils.formatTimeAgo(initialEntryTime)}</span>
          </div>
        </div>
      </div>
    `;
  }

  _buildCurrentStateCard(pos, isOpen) {
    const currentPrice = pos.current_price;
    const tokenAmount = pos.remaining_token_amount || pos.token_amount;
    const verified = pos.transaction_entry_verified;

    let stateContent = "";
    if (isOpen) {
      stateContent = `
        <div class="pdd-stat-row">
          <span class="pdd-stat-label">Current Price</span>
          <span class="pdd-stat-value">${currentPrice ? this._formatPrice(currentPrice) + " SOL" : "—"}</span>
        </div>
        <div class="pdd-stat-row">
          <span class="pdd-stat-label">Token Amount</span>
          <span class="pdd-stat-value">${tokenAmount ? Utils.formatCompactNumber(tokenAmount) : "—"}</span>
        </div>
        <div class="pdd-stat-row">
          <span class="pdd-stat-label">Entry Verified</span>
          <span class="pdd-stat-value">${verified ? '<span class="pdd-verified">✓ Verified</span>' : '<span class="pdd-unverified">Pending</span>'}</span>
        </div>
      `;
    } else {
      const exitPrice = pos.average_exit_price || pos.exit_price;
      const solReceived = pos.sol_received;
      const closedReason = pos.closed_reason;

      stateContent = `
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
      `;
    }

    return `
      <div class="pdd-card">
        <h3 class="pdd-card-title">
          <i class="icon-activity"></i>
          ${isOpen ? "Current State" : "Exit Details"}
        </h3>
        <div class="pdd-card-content">
          ${stateContent}
        </div>
      </div>
    `;
  }

  _buildPnLSummaryCard(pos, isOpen) {
    const pnl = isOpen ? pos.unrealized_pnl : pos.pnl;
    const pnlPct = isOpen ? pos.unrealized_pnl_percent : pos.pnl_percent;
    const highestPrice = pos.price_highest;
    const lowestPrice = pos.price_lowest;

    const pnlClass = pnl >= 0 ? "pdd-positive" : "pdd-negative";
    const sign = pnl >= 0 ? "+" : "";

    return `
      <div class="pdd-card">
        <h3 class="pdd-card-title">
          <i class="icon-trending-up"></i>
          P&L Summary
        </h3>
        <div class="pdd-card-content">
          <div class="pdd-pnl-display ${pnlClass}">
            <span class="pdd-pnl-value">${pnl !== undefined ? sign + Utils.formatSol(pnl, { decimals: 4 }) : "—"}</span>
            <span class="pdd-pnl-percent">${pnlPct !== undefined ? sign + Utils.formatNumber(pnlPct, 2) + "%" : ""}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Peak Price</span>
            <span class="pdd-stat-value">${highestPrice ? this._formatPrice(highestPrice) + " SOL" : "—"}</span>
          </div>
          <div class="pdd-stat-row">
            <span class="pdd-stat-label">Low Price</span>
            <span class="pdd-stat-value">${lowestPrice ? this._formatPrice(lowestPrice) + " SOL" : "—"}</span>
          </div>
        </div>
      </div>
    `;
  }

  _buildActionsCard(pos, isOpen) {
    if (isOpen) {
      return `
        <div class="pdd-card pdd-actions-card">
          <h3 class="pdd-card-title">
            <i class="icon-zap"></i>
            Actions
          </h3>
          <div class="pdd-card-content">
            <div class="pdd-action-buttons">
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
            </div>
          </div>
        </div>
      `;
    }

    return `
      <div class="pdd-card pdd-actions-card">
        <h3 class="pdd-card-title">
          <i class="icon-info"></i>
          Position Closed
        </h3>
        <div class="pdd-card-content">
          <div class="pdd-action-buttons">
            <button class="pdd-action-btn pdd-action-view" id="pddViewTokenBtn">
              <i class="icon-external-link"></i>
              View Token Details
            </button>
          </div>
        </div>
      </div>
    `;
  }

  _attachActionHandlers(content, pos, isOpen) {
    if (isOpen) {
      const addBtn = content.querySelector("#pddAddBtn");
      const partialBtn = content.querySelector("#pddPartialBtn");
      const closeBtn = content.querySelector("#pddCloseBtn");

      if (addBtn) {
        addBtn.addEventListener("click", () => this._handleAddPosition(pos));
      }
      if (partialBtn) {
        partialBtn.addEventListener("click", () => this._handlePartialSell(pos));
      }
      if (closeBtn) {
        closeBtn.addEventListener("click", () => this._handleClosePosition(pos));
      }
    } else {
      const viewBtn = content.querySelector("#pddViewTokenBtn");
      if (viewBtn) {
        viewBtn.addEventListener("click", () => {
          // Close this dialog and open token details
          this.close();
          // The parent page should handle opening token details
          this.onTradeComplete({ action: "view_token", mint: pos.mint });
        });
      }
    }
  }

  async _handleAddPosition(pos) {
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
      }
    }
  }

  async _handlePartialSell(pos) {
    if (!this.tradeDialog) {
      this.tradeDialog = new TradeActionDialog();
    }

    const result = await this.tradeDialog.open("sell", {
      mint: pos.mint,
      symbol: pos.symbol,
      tokenAmount: pos.remaining_token_amount || pos.token_amount,
    });

    if (result && result.confirmed) {
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
      }
    }
  }

  async _handleClosePosition(pos) {
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
      }
    }
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
    const transactions = this.fullDetails?.transactions || [];
    const stateHistory = this.fullDetails?.state_history || [];

    if (transactions.length === 0 && stateHistory.length === 0) {
      content.innerHTML = '<div class="pdd-empty-state">No transactions available</div>';
      return;
    }

    // Transactions list
    const txListHtml = transactions
      .map((tx) => {
        const signature = tx.signature || "";
        const shortSig = signature ? `${signature.slice(0, 8)}...${signature.slice(-8)}` : "—";
        const statusClass = tx.success ? "pdd-status-success" : "pdd-status-failed";
        const statusIcon = tx.success ? "icon-check-circle" : "icon-x-circle";
        const typeLabel = this._formatTransactionType(tx.transaction_type || tx.kind);
        const feeSol = tx.fee_sol ? Utils.formatSol(tx.fee_sol, { decimals: 6 }) : "—";

        return `
          <div class="pdd-transaction-item">
            <div class="pdd-tx-status ${statusClass}">
              <i class="${statusIcon}"></i>
            </div>
            <div class="pdd-tx-info">
              <div class="pdd-tx-type">${typeLabel}</div>
              <div class="pdd-tx-signature" data-signature="${signature}" title="Click to copy">
                ${shortSig}
              </div>
            </div>
            <div class="pdd-tx-meta">
              <div class="pdd-tx-fee">Fee: ${feeSol}</div>
              <div class="pdd-tx-time" title="${Utils.formatTimestamp(tx.timestamp)}">${Utils.formatTimeAgo(tx.timestamp)}</div>
            </div>
            <div class="pdd-tx-actions">
              <a href="https://solscan.io/tx/${signature}" target="_blank" class="action-btn" title="View on Solscan">
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
        ${transactions.length > 0 ? "<div class=\"pdd-section-header\">Transactions</div>" : ""}
        <div class="pdd-transactions-list">
          ${txListHtml || "<div class=\"pdd-empty-state\">No transactions</div>"}
        </div>
        ${stateHistoryHtml}
      </div>
    `;

    // Attach copy handlers for signatures
    content.querySelectorAll(".pdd-tx-signature").forEach((el) => {
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
      "entry": "Entry",
      "exit": "Exit",
      "buy": "Buy",
      "sell": "Sell",
      "swap": "Swap",
      "transfer": "Transfer",
      "unknown": "Unknown",
    };

    return typeMap[type.toLowerCase()] || Utils.escapeHtml(type);
  }
}
