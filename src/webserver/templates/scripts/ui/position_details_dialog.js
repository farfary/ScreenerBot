/**
 * Position Details Dialog
 * Full-screen dialog showing comprehensive position information with multiple tabs
 */
import * as Utils from "../core/utils.js";
import { createFocusTrap } from "../core/utils.js";
import { Poller } from "../core/poller.js";
import { requestManager } from "../core/request_manager.js";
import * as Hints from "../core/hints.js";
import { HintTrigger } from "./hint_popover.js";
import { applyOverviewTabMixin } from "./position_details/overview_tab.js";
import { applyChartTabMixin } from "./position_details/chart_tab.js";
import { applyAnalyticsTabMixin } from "./position_details/analytics_tab.js";
import { applySecondaryTabsMixin } from "./position_details/secondary_tabs.js";
import { applyUtilitiesMixin } from "./position_details/utilities.js";

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
    this._manualToggleHandler = null;
    this._managementChangedHandler = null;
    this._focusTrap = null;
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
          // Add ARIA attributes for accessibility
          const container = this.dialogEl.querySelector(".dialog-container");
          if (container) {
            container.setAttribute("role", "dialog");
            container.setAttribute("aria-modal", "true");
            container.setAttribute("aria-labelledby", "pdd-dialog-title");
          }
          // Activate focus trap
          this._focusTrap = createFocusTrap(this.dialogEl);
          this._focusTrap.activate();
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
          <i class="icon-circle-alert"></i>
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

    // Deactivate focus trap
    if (this._focusTrap) {
      this._focusTrap.deactivate();
      this._focusTrap = null;
    }

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

        if (this._manualToggleHandler) {
          const manualToggle = this.dialogEl.querySelector("#pddManualToggle");
          if (manualToggle) {
            manualToggle.removeEventListener("click", this._manualToggleHandler);
          }
          this._manualToggleHandler = null;
        }

        if (this._managementChangedHandler) {
          window.removeEventListener(
            "screenerbot:position-management-changed",
            this._managementChangedHandler
          );
          this._managementChangedHandler = null;
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
    this._destroyPositionChart?.();

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
    // Manual management: open positions get an interactive toggle (enable/disable the
    // auto-trader for this position); closed positions just show the historical state.
    const manualHint = this._renderManualHint();
    const manualBadge = isOpen
      ? `<button type="button" id="pddManualToggle" class="pdd-badge pdd-manual-toggle ${
          pos.manual_management ? "is-on" : "is-off"
        }" aria-pressed="${pos.manual_management ? "true" : "false"}" title="${
          pos.manual_management
            ? "Manual management ON — auto-trader will not sell this position. Click to hand it back to the auto-trader."
            : "Manual management OFF — auto-trader manages this position. Click to take manual control."
        }"><i class="icon-shield"></i> ${
          pos.manual_management ? "Manual" : "Auto-managed"
        }</button>${manualHint}`
      : pos.manual_management
        ? '<span class="pdd-badge pdd-badge-manual" title="Manually bought — the auto-trader did not auto-sell or DCA this position"><i class="icon-shield"></i> Manual</span>'
        : "";

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
                ${manualBadge}
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
            <i class="icon-chart-bar"></i>
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
   * Note: Position data is flattened (no summary wrapper) due to serde(flatten) on backend
   */
  _buildHeaderPrice(pos) {
    const currentPrice = pos?.current_price;
    const entryPrice = pos?.average_entry_price || pos?.entry_price;
    const isOpen = pos?.position_type !== "closed";

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
    if (isOpen && pos?.unrealized_pnl !== undefined) {
      const pnl = pos.unrealized_pnl;
      const pnlPct = pos.unrealized_pnl_percent;
      const pnlClass = pnl != null && pnl >= 0 ? "pdd-positive" : "pdd-negative";
      const sign = pnl != null && pnl >= 0 ? "+" : "";
      pnlHtml = `
        <div class="pnl-block ${pnlClass}">
          <span class="pnl-value">${sign}${Utils.formatSol(pnl, { decimals: 4, suffix: "" })}</span>
          <span class="pnl-percent">${sign}${Utils.formatNumber(pnlPct, 2)}%</span>
        </div>
      `;
    } else if (!isOpen && pos?.pnl !== undefined) {
      const pnl = pos.pnl;
      const pnlPct = pos.pnl_percent;
      const pnlClass = pnl != null && pnl >= 0 ? "pdd-positive" : "pdd-negative";
      const sign = pnl != null && pnl >= 0 ? "+" : "";
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
          <span class="metric-value">${Utils.formatSol(pos?.total_size_sol, { decimals: 4 })}</span>
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

    // Manual-management toggle (open positions). Dispatches the shared toggle event;
    // the global handler does the POST + toast and broadcasts the change back.
    const manualToggle = this.dialogEl.querySelector("#pddManualToggle");
    if (manualToggle) {
      this._manualToggleHandler = () => {
        const next = !this.positionData.manual_management;
        window.dispatchEvent(
          new CustomEvent("screenerbot:toggle-position-management", {
            detail: { id: this.positionData.id, mint: this.positionData.mint, enabled: next },
          })
        );
      };
      manualToggle.addEventListener("click", this._manualToggleHandler);
    }

    // Reflect management changes (from here or the row context menu) in this dialog.
    this._managementChangedHandler = (event) => {
      const { id, enabled } = event.detail || {};
      if (id == null || id !== this.positionData?.id) return;
      this.positionData.manual_management = !!enabled;
      this._refreshManualToggle();
    };
    window.addEventListener(
      "screenerbot:position-management-changed",
      this._managementChangedHandler
    );

    // Activate the hint trigger's delegated click handler.
    HintTrigger.initAll();
  }

  /** Render the manual-management hint trigger HTML (empty if hints are off). */
  _renderManualHint() {
    const hint = Hints.getHint("positions.manualManagement");
    if (!hint) return "";
    return HintTrigger.render(hint, "positions.manualManagement", {
      size: "sm",
      position: "bottom",
    });
  }

  /** Update the toggle button's state/label in place after a management change. */
  _refreshManualToggle() {
    const btn = this.dialogEl?.querySelector("#pddManualToggle");
    if (!btn) return;
    const on = !!this.positionData.manual_management;
    btn.classList.toggle("is-on", on);
    btn.classList.toggle("is-off", !on);
    btn.setAttribute("aria-pressed", on ? "true" : "false");
    btn.title = on
      ? "Manual management ON — auto-trader will not sell this position. Click to hand it back to the auto-trader."
      : "Manual management OFF — auto-trader manages this position. Click to take manual control.";
    btn.innerHTML = `<i class="icon-shield"></i> ${on ? "Manual" : "Auto-managed"}`;
  }

  /**
   * Switch to a different tab
   */
  _switchTab(tabId) {
    if (tabId === this.currentTab) return;

    // Leaving the chart tab — release the chart, poller and observers.
    if (this.currentTab === "chart" && tabId !== "chart") {
      this._destroyPositionChart?.();
    }

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
      case "analytics":
        this._renderAnalyticsTab(content);
        break;
    }
  }

  // ===========================================================================
  // OVERVIEW TAB
  // ===========================================================================
  // Methods are added via applyOverviewTabMixin()

  // ===========================================================================
  // CHART TAB
  // ===========================================================================
  // Methods are added via applyChartTabMixin()

  // ===========================================================================
  // ANALYTICS TAB
  // ===========================================================================
  // Methods are added via applyAnalyticsTabMixin()

  // ===========================================================================
  // TOKEN TAB
  // ===========================================================================
  // Methods are added via applySecondaryTabsMixin()

  // ===========================================================================
  // UTILITY METHODS
  // ===========================================================================
  // Methods are added via applyUtilitiesMixin()
}

// Apply mixins to add tab rendering methods
applyOverviewTabMixin(PositionDetailsDialog);
applyChartTabMixin(PositionDetailsDialog);
applyAnalyticsTabMixin(PositionDetailsDialog);
applySecondaryTabsMixin(PositionDetailsDialog);
applyUtilitiesMixin(PositionDetailsDialog);

// ============================================================================
// Global Event Listener for Context Menu "View Details" Action (Positions)
// ============================================================================
// This listener allows any page to open the PositionDetailsDialog via custom event
// dispatched from context_menu.js when user clicks "View Details" on a position row

let globalPositionDialogInstance = null;

window.addEventListener("screenerbot:open-position-details", async (event) => {
  const { id, mint, symbol, position_type } = event.detail || {};

  if (!id && !mint) {
    console.error("[PositionDetailsDialog] Event received without id or mint");
    return;
  }

  console.log(`[PositionDetailsDialog] Opening details for position ${id || mint}`);

  // Close existing dialog if open for a different position
  if (globalPositionDialogInstance && globalPositionDialogInstance.dialogEl) {
    const currentId = globalPositionDialogInstance.positionData?.id;
    const currentMint = globalPositionDialogInstance.positionData?.mint;
    if ((id && currentId === id) || (!id && currentMint === mint)) {
      // Already open for this position, do nothing
      console.log("[PositionDetailsDialog] Dialog already open for this position");
      return;
    }
    globalPositionDialogInstance.close();
    await new Promise((resolve) => setTimeout(resolve, 350));
  }

  // Create new dialog instance if needed
  if (!globalPositionDialogInstance) {
    globalPositionDialogInstance = new PositionDetailsDialog({
      onClose: () => {
        // Keep instance for reuse, just clean up state
      },
    });
  }

  // Open dialog with position data (dialog will fetch full details)
  await globalPositionDialogInstance.show({
    id,
    mint,
    symbol: symbol || "",
    position_type: position_type || "open",
  });
});

// ============================================================================
// Global Manual-Management Toggle Handler
// ============================================================================
// Single place that performs the toggle so it works from any page (the positions
// context menu and this dialog both dispatch `screenerbot:toggle-position-management`).
// On success it broadcasts `screenerbot:position-management-changed` so an open
// positions table / dialog can refresh.
window.addEventListener("screenerbot:toggle-position-management", async (event) => {
  const { id, enabled } = event.detail || {};
  if (id == null) return;

  try {
    const data = await requestManager.fetch(
      `/api/positions/${encodeURIComponent(id)}/manual-management`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled: !!enabled }),
        priority: "high",
      }
    );
    if (data && data.success === false) {
      throw new Error(data.message || "Request failed");
    }
    Utils.showToast(
      data?.message ||
        (enabled
          ? "Manual management enabled — auto-trader won't sell this position"
          : "Manual management disabled — auto-trader now manages this position"),
      "success"
    );
    window.dispatchEvent(
      new CustomEvent("screenerbot:position-management-changed", {
        detail: { id, enabled: !!enabled },
      })
    );
  } catch (err) {
    Utils.showToast(err?.message || "Failed to update manual management", "error");
  }
});
