/**
 * Token Details Dialog
 * Full-screen dialog showing comprehensive token information with multiple tabs
 */
/* global createAdvancedChart, MutationObserver */
import * as Utils from "../core/utils.js";
import { createFocusTrap } from "../core/utils.js";
import { Poller } from "../core/poller.js";
import { requestManager } from "../core/request_manager.js";
import { TradeActionDialog } from "./trade_action_dialog.js";
import * as Hints from "../core/hints.js";
import { HintTrigger } from "./hint_popover.js";
import { renderOverviewTab } from "./token_details/overview_tab.js";
import { renderSecurityTab } from "./token_details/security_tab.js";
import {
  renderPoolsTab,
  initPoolsTabEvents,
  renderLinksTab,
  initLinksTabEvents,
} from "./token_details/pools_links_tab.js";

// Data source status constants
const DATA_SOURCE_STATUS = {
  PENDING: "pending",
  LOADING: "loading",
  SUCCESS: "success",
  ERROR: "error",
  CACHED: "cached",
};

export class TokenDetailsDialog {
  constructor(options = {}) {
    this.onClose = options.onClose || (() => {});
    this.onTradeComplete = options.onTradeComplete || (() => {});
    this.dialogEl = null;
    this.currentTab = "overview";
    this.tokenData = null;
    this.tabHandlers = new Map();
    this.refreshPoller = null;
    this.chartPoller = null;
    this.isRefreshing = false;
    this.currentTimeframe = "5m";
    this.isOpening = false;
    this.tradeDialog = null;
    this.positionsData = null;
    this.walletBalance = 0;
    this.walletBalanceFetchedAt = 0;
    this.advancedChart = null;
    this.chartDataLoaded = false; // Track whether OHLCV data has been loaded
    this._focusTrap = null;
    // Data source status tracking
    this._dataSourceStatus = {
      token: DATA_SOURCE_STATUS.PENDING,
      dexscreener: DATA_SOURCE_STATUS.PENDING,
      rugcheck: DATA_SOURCE_STATUS.PENDING,
      ohlcv: DATA_SOURCE_STATUS.PENDING,
    };
    this._initialLoadComplete = false;
    this._retryCount = 0;
    this._maxRetries = 3;
  }

  /**
   * Show dialog with token data
   * @param {Object} tokenData - Complete token data object (minimal - just mint required)
   */
  async show(tokenData) {
    if (!tokenData || !tokenData.mint) {
      console.error("Invalid token data provided to TokenDetailsDialog");
      return;
    }

    if (this.isOpening) {
      console.log("Dialog already opening, ignoring duplicate request");
      return;
    }

    if (this.dialogEl && this.tokenData && this.tokenData.mint !== tokenData.mint) {
      console.log("Closing existing dialog to open new token");
      this.close();
      await new Promise((resolve) => setTimeout(resolve, 350));
    }

    if (this.dialogEl && this.tokenData && this.tokenData.mint === tokenData.mint) {
      console.log("Dialog already open for this token, ignoring");
      return;
    }

    this.isOpening = true;

    try {
      this.tokenData = tokenData;
      // Use initial tokenData for immediate display (don't wait for API)
      // fullTokenData will be updated by polling when fresh data arrives
      this.fullTokenData = tokenData;
      // Reset chart data state for new token
      this.chartDataLoaded = false;

      // Initialize hints system before creating dialog
      await Hints.init();

      this._createDialog();
      this._attachEventHandlers();
      this._loadTabContent("overview");

      // Initialize hint triggers after content is loaded
      HintTrigger.initAll();

      requestAnimationFrame(() => {
        if (this.dialogEl) {
          this.dialogEl.classList.add("active");
          // Add ARIA attributes for accessibility
          const container = this.dialogEl.querySelector(".dialog-container");
          if (container) {
            container.setAttribute("role", "dialog");
            container.setAttribute("aria-modal", "true");
            container.setAttribute("aria-labelledby", "tdd-dialog-title");
          }
          // Activate focus trap
          this._focusTrap = createFocusTrap(this.dialogEl);
          this._focusTrap.activate();
        }
      });

      // Set this token as dashboard-active for priority data fetching (fire and forget)
      this._focusToken().catch(() => {
        // Silent - focus is best-effort
      });

      // Trigger backend refresh endpoints in parallel (fire and forget)
      // These trigger high-priority data fetching on backend
      // Don't await - let them run in background while dialog shows
      this._triggerTokenRefresh().catch((err) => {
        console.warn("Token refresh failed:", err);
      });
      this._triggerOhlcvRefresh().catch(() => {
        // Silent - expected for new tokens without OHLCV
      });

      // Start polling after a short delay to give refresh time to start
      // The poller will fetch fresh data as it becomes available
      setTimeout(() => {
        if (this.dialogEl) {
          this._startPolling();
        }
      }, 500);
    } finally {
      this.isOpening = false;
    }
  }

  async _triggerTokenRefresh() {
    try {
      // Use requestManager with high priority for immediate refresh
      const response = await requestManager.fetch(`/api/tokens/${this.tokenData.mint}/refresh`, {
        method: "POST",
        priority: "high",
      });
      if (response.success !== false) {
        console.log("Token data refresh triggered:", response);
        return response;
      }
    } catch (error) {
      console.warn("Failed to trigger token refresh:", error);
    }
    return null;
  }

  async _triggerOhlcvRefresh() {
    try {
      // Use requestManager with high priority for immediate OHLCV refresh
      const response = await requestManager.fetch(
        `/api/tokens/${this.tokenData.mint}/ohlcv/refresh`,
        {
          method: "POST",
          priority: "high",
        }
      );
      if (response.success !== false) {
        console.log("OHLCV data refresh triggered:", response);
        return response;
      }
    } catch (error) {
      // Silently ignore - OHLCV may not be available for new tokens
    }
    return null;
  }

  /**
   * Set this token as dashboard-active for priority data fetching
   * Background batch updates will skip this token while it's focused
   */
  async _focusToken() {
    try {
      const response = await requestManager.fetch(`/api/tokens/${this.tokenData.mint}/focus`, {
        method: "POST",
        priority: "high",
      });
      if (response.success) {
        console.log("Token focused for priority updates:", response);
      }
      return response;
    } catch (error) {
      console.warn("Failed to focus token:", error);
    }
    return null;
  }

  /**
   * Clear dashboard focus when dialog closes
   * Resets OHLCV priority unless token has an open position
   */
  async _unfocusToken() {
    try {
      const response = await requestManager.fetch(`/api/tokens/${this.tokenData.mint}/unfocus`, {
        method: "POST",
        priority: "low",
      });
      if (response.success) {
        console.log("Token unfocused:", response);
      }
      return response;
    } catch (error) {
      // Silent - unfocus is best-effort
    }
    return null;
  }

  async _fetchTokenData() {
    if (this.isRefreshing) return;
    this.isRefreshing = true;

    // Update status to loading on first fetch
    if (!this._initialLoadComplete) {
      this._updateDataSourceStatus("token", DATA_SOURCE_STATUS.LOADING);
    }

    try {
      // Use requestManager with high priority for token detail fetch
      const newData = await requestManager.fetch(`/api/tokens/${this.tokenData.mint}`, {
        priority: "high",
      });

      if (newData) {
        const isInitialLoad = !this._initialLoadComplete;
        this.fullTokenData = newData;
        this._updateHeader(this.fullTokenData);
        this._initialLoadComplete = true;
        this._retryCount = 0;

        // Update data source statuses based on what data we have
        this._updateDataSourceStatus("token", DATA_SOURCE_STATUS.SUCCESS);
        this._updateDataSourceFromToken(newData);

        if (isInitialLoad && this.currentTab === "overview") {
          this._loadTabContent(this.currentTab);
        } else if (!isInitialLoad && this.currentTab === "overview") {
          this._refreshOverviewTab();
        }

        // Also refresh security tab if it's active and was waiting for data
        if (this.currentTab === "security") {
          const content = this.dialogEl?.querySelector('[data-tab-content="security"]');
          if (content && content.dataset.loaded !== "true") {
            this._loadSecurityTab(content);
          }
        }
      }
    } catch (error) {
      console.error("Error loading token details:", error);
      this._updateDataSourceStatus("token", DATA_SOURCE_STATUS.ERROR);

      // Retry with exponential backoff for initial load
      if (!this._initialLoadComplete && this._retryCount < this._maxRetries) {
        this._retryCount++;
        const delay = 1000 * Math.pow(2, this._retryCount - 1); // 1s, 2s, 4s
        console.log(`Retrying token fetch (${this._retryCount}/${this._maxRetries}) in ${delay}ms`);
        setTimeout(() => {
          this.isRefreshing = false;
          this._fetchTokenData();
        }, delay);
        return;
      }

      const headerMetrics = this.dialogEl?.querySelector(".header-metrics");
      if (headerMetrics) {
        headerMetrics.innerHTML = '<div class="error-text">Failed to load details</div>';
      }
    } finally {
      this.isRefreshing = false;
    }
  }

  /**
   * Update data source statuses based on token data fields
   */
  _updateDataSourceFromToken(token) {
    // Check DexScreener data
    if (token.market_cap || token.volume_24h || token.liquidity_usd) {
      this._updateDataSourceStatus("dexscreener", DATA_SOURCE_STATUS.SUCCESS);
    } else if (this._dataSourceStatus.dexscreener === DATA_SOURCE_STATUS.PENDING) {
      this._updateDataSourceStatus("dexscreener", DATA_SOURCE_STATUS.LOADING);
    }

    // Check Rugcheck data
    if (token.safety_score !== undefined && token.safety_score !== null) {
      this._updateDataSourceStatus("rugcheck", DATA_SOURCE_STATUS.SUCCESS);
    } else if (this._dataSourceStatus.rugcheck === DATA_SOURCE_STATUS.PENDING) {
      this._updateDataSourceStatus("rugcheck", DATA_SOURCE_STATUS.LOADING);
    }

    // Check OHLCV availability
    if (token.has_ohlcv) {
      this._updateDataSourceStatus("ohlcv", DATA_SOURCE_STATUS.SUCCESS);
    } else if (this._dataSourceStatus.ohlcv === DATA_SOURCE_STATUS.PENDING) {
      this._updateDataSourceStatus("ohlcv", DATA_SOURCE_STATUS.LOADING);
    }
  }

  /**
   * Update data source status and UI indicator
   * @param {string} source - 'token' | 'dexscreener' | 'rugcheck' | 'ohlcv'
   * @param {string} status - DATA_SOURCE_STATUS constant
   */
  _updateDataSourceStatus(source, status) {
    this._dataSourceStatus[source] = status;

    // Update UI indicator
    const indicator = this.dialogEl?.querySelector(`.source-status[data-source="${source}"]`);
    if (indicator) {
      const icon = indicator.querySelector(".status-icon");
      if (icon) {
        icon.className = `status-icon ${status}`;
      }
    }

    // Check if all sources are done loading
    const allDone = Object.values(this._dataSourceStatus).every(
      (s) =>
        s === DATA_SOURCE_STATUS.SUCCESS ||
        s === DATA_SOURCE_STATUS.ERROR ||
        s === DATA_SOURCE_STATUS.CACHED
    );

    // Hide status bar when all sources are loaded successfully
    if (allDone) {
      const statusBar = this.dialogEl?.querySelector(".data-sources-status");
      if (statusBar) {
        statusBar.classList.add("all-loaded");
      }
    }
  }

  _startPolling() {
    this._stopPolling();
    // Use 5 second polling interval (reduced from 1 second)
    this.refreshPoller = new Poller(
      () => {
        this._fetchTokenData();
      },
      { label: "TokenRefresh", interval: 5000 }
    );
    this.refreshPoller.start();
  }

  _stopPolling() {
    if (this.refreshPoller) {
      this.refreshPoller.stop();
      this.refreshPoller.cleanup();
      this.refreshPoller = null;
    }
  }

  _startChartPolling() {
    this._stopChartPolling();
    // Use 3s polling when waiting for data, 10s when data loaded (reduced from 1.5s/5s)
    const interval = this.chartDataLoaded ? 10000 : 3000;
    this.chartPoller = new Poller(
      () => {
        this._refreshChartData();
      },
      { label: "ChartRefresh", interval }
    );
    this.chartPoller.start();
  }

  _stopChartPolling() {
    if (this.chartPoller) {
      this.chartPoller.stop();
      this.chartPoller.cleanup();
      this.chartPoller = null;
    }
  }

  async _refreshChartData() {
    if (!this.advancedChart || !this.tokenData || this.currentTab !== "overview") {
      return;
    }

    const loadingOverlay = this.dialogEl?.querySelector("#chartLoadingOverlay");
    const loadingText = loadingOverlay?.querySelector(".chart-loading-text");
    const wasDataLoaded = this.chartDataLoaded;

    // Update OHLCV status to loading if not yet loaded
    if (!this.chartDataLoaded && this._dataSourceStatus.ohlcv !== DATA_SOURCE_STATUS.SUCCESS) {
      this._updateDataSourceStatus("ohlcv", DATA_SOURCE_STATUS.LOADING);
    }

    try {
      // Use requestManager with normal priority for periodic chart refresh
      const data = await requestManager.fetch(
        `/api/tokens/${this.tokenData.mint}/ohlcv?timeframe=${this.currentTimeframe}&limit=200`,
        { priority: "normal" }
      );

      if (!Array.isArray(data) || data.length === 0) {
        // No data yet - keep showing waiting message
        if (loadingText) {
          loadingText.textContent = "Waiting for chart data...";
        }
        if (loadingOverlay) {
          loadingOverlay.classList.remove("hidden");
        }
        this.chartDataLoaded = false;
        return;
      }

      const chartData = data.map((candle) => ({
        time: candle.timestamp,
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
        volume: candle.volume || 0,
      }));

      this.advancedChart.setData(chartData);

      // Hide loading overlay and mark data as loaded
      if (loadingOverlay) {
        loadingOverlay.classList.add("hidden");
      }
      this.chartDataLoaded = true;
      this._chartErrorCount = 0; // Reset error counter on success

      // Update OHLCV status to success
      this._updateDataSourceStatus("ohlcv", DATA_SOURCE_STATUS.SUCCESS);

      // Update OHLCV display
      this._updateOhlcvDisplay(chartData);

      // If data just loaded (was not loaded before), restart polling with slower interval
      if (!wasDataLoaded && this.chartDataLoaded) {
        this._startChartPolling();
      }
    } catch (error) {
      // On error when no data yet, keep showing waiting message
      if (!this.chartDataLoaded && loadingText) {
        loadingText.textContent = "Waiting for chart data...";
      }
      if (!this.chartDataLoaded && loadingOverlay) {
        loadingOverlay.classList.remove("hidden");
      }
      // Track chart fetch failures - after multiple failures, mark as error
      if (!this.chartDataLoaded) {
        this._chartErrorCount = (this._chartErrorCount || 0) + 1;
        // After 5 consecutive failures (~15-50s depending on interval), mark as error
        if (this._chartErrorCount >= 5) {
          this._updateDataSourceStatus("ohlcv", DATA_SOURCE_STATUS.ERROR);
        }
      }
    }
  }

  _refreshOverviewTab() {
    const content = this.dialogEl?.querySelector('[data-tab-content="overview"]');
    if (!content || !this.fullTokenData) return;
    if (content.dataset.loaded !== "true") return;

    const overviewTable = content.querySelector(".overview-left");
    if (overviewTable) {
      overviewTable.innerHTML = this._buildOverviewContent(this.fullTokenData);
    }
  }

  close() {
    if (!this.dialogEl) return;

    // Deactivate focus trap
    if (this._focusTrap) {
      this._focusTrap.deactivate();
      this._focusTrap = null;
    }

    // Clear dashboard focus and deprioritize OHLCV when dialog closes (fire and forget)
    // Store mint in local variable before tokenData is nulled
    const mintToUnfocus = this.tokenData?.mint;
    if (mintToUnfocus) {
      this._unfocusToken().catch(() => {
        // Silent - unfocus is best-effort
      });
    }

    this._stopPolling();
    this._stopChartPolling();

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

        if (this._tabHandlers) {
          this._tabHandlers.forEach(({ element, handler }) => {
            element.removeEventListener("click", handler);
          });
          this._tabHandlers = null;
        }

        // Clean up buy/sell button handlers
        if (this._buyHandler) {
          const buyBtn = this.dialogEl.querySelector("#headerBuyBtn");
          if (buyBtn) {
            buyBtn.removeEventListener("click", this._buyHandler);
          }
          this._buyHandler = null;
        }

        if (this._sellHandler) {
          const sellBtn = this.dialogEl.querySelector("#headerSellBtn");
          if (sellBtn) {
            sellBtn.removeEventListener("click", this._sellHandler);
          }
          this._sellHandler = null;
        }
      }

      if (this.chartResizeObserver) {
        this.chartResizeObserver.disconnect();
        this.chartResizeObserver = null;
      }

      // Clean up theme observer
      if (this._themeObserver) {
        this._themeObserver.disconnect();
        this._themeObserver = null;
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

      this.tokenData = null;
      this.fullTokenData = null;
      this.currentTab = "overview";
      this.currentTimeframe = "5m";
      this.isRefreshing = false;
      this.isOpening = false;
      this.tabHandlers.clear();

      // Reset data source tracking
      this._dataSourceStatus = {
        token: DATA_SOURCE_STATUS.PENDING,
        dexscreener: DATA_SOURCE_STATUS.PENDING,
        rugcheck: DATA_SOURCE_STATUS.PENDING,
        ohlcv: DATA_SOURCE_STATUS.PENDING,
      };
      this._initialLoadComplete = false;
      this._retryCount = 0;
      this._chartErrorCount = 0;

      this.onClose();
    }, 300);
  }

  _createDialog() {
    this.dialogEl = document.createElement("div");
    this.dialogEl.className = "token-details-dialog";
    this.dialogEl.innerHTML = this._getDialogHTML();
    document.body.appendChild(this.dialogEl);
  }

  _getDialogHTML() {
    const symbol = this.tokenData.symbol || "Unknown";
    const name = this.tokenData.name || "Unknown Token";
    const logoUrl = this.tokenData.logo_url || this.tokenData.image_url || "";

    return `
      <div class="dialog-backdrop"></div>
      <div class="dialog-container">
        <div class="dialog-header">
          <div class="header-top-row">
            <div class="header-left">
              <div class="header-logo">
                ${logoUrl ? `<img src="${this._escapeHtml(logoUrl)}" alt="${this._escapeHtml(symbol)}" onerror="this.parentElement.innerHTML='<div class=\\'logo-placeholder\\'>${this._escapeHtml(symbol.charAt(0))}</div>'" />` : `<div class="logo-placeholder">${this._escapeHtml(symbol.charAt(0))}</div>`}
              </div>
              <div class="header-title">
                <span class="title-main">${this._escapeHtml(symbol)}</span>
                <span class="title-sub">${this._escapeHtml(name)}</span>
              </div>
            </div>
            <div class="header-center">
              <div class="header-price" id="headerPrice">
                <div class="price-loading">Loading...</div>
              </div>
            </div>
            <div class="header-right">
              <div class="header-trade-actions">
                <button class="trade-btn buy-btn" id="headerBuyBtn" title="Buy this token">
                  <i class="icon-shopping-cart"></i>
                  Buy
                </button>
                <button class="trade-btn sell-btn" id="headerSellBtn" title="Sell position" disabled>
                  <i class="icon-dollar-sign"></i>
                  Sell
                </button>
              </div>
              <div class="header-actions">
                <button class="action-btn" id="copyMintBtn" title="Copy Mint Address">
                  <i class="icon-copy"></i>
                </button>
                <a href="https://solscan.io/token/${this._escapeHtml(this.tokenData.mint)}" target="_blank" class="action-btn" title="View on Solscan">
                  <i class="icon-external-link"></i>
                </a>
              </div>
              <button class="dialog-close" type="button" title="Close (ESC)">
                <i class="icon-x"></i>
              </button>
            </div>
          </div>
          <div class="header-badges-row" id="headerBadgesRow">
            <div class="title-badges" id="headerBadges"></div>
            <div class="header-status-area">
              <div class="data-sources-status" role="status" aria-label="Data loading status">
                <span class="source-status" data-source="token" title="Token info">
                  <span class="status-icon pending" aria-hidden="true"></span>
                  <span class="status-label">Token</span>
                </span>
                <span class="source-status" data-source="dexscreener" title="Market data">
                  <span class="status-icon pending" aria-hidden="true"></span>
                  <span class="status-label">Market</span>
                </span>
                <span class="source-status" data-source="rugcheck" title="Security analysis">
                  <span class="status-icon pending" aria-hidden="true"></span>
                  <span class="status-label">Security</span>
                </span>
                <span class="source-status" data-source="ohlcv" title="Chart data">
                  <span class="status-icon pending" aria-hidden="true"></span>
                  <span class="status-label">Chart</span>
                </span>
              </div>
              <div class="last-updated" id="lastUpdatedTime"></div>
            </div>
          </div>
        </div>

        <div class="dialog-tabs">
          <button class="tab-button active" data-tab="overview">
            <i class="icon-info"></i>
            Overview
          </button>
          <button class="tab-button" data-tab="security">
            <i class="icon-shield"></i>
            Security
          </button>
          <button class="tab-button" data-tab="positions">
            <i class="icon-chart-bar"></i>
            Positions
          </button>
          <button class="tab-button" data-tab="pools">
            <i class="icon-droplet"></i>
            Pools
          </button>
          <button class="tab-button" data-tab="links">
            <i class="icon-link"></i>
            Links
          </button>
          <button class="tab-button" data-tab="transactions">
            <i class="icon-activity"></i>
            Txns
          </button>
        </div>

        <div class="dialog-body">
          <div class="tab-content active" data-tab-content="overview">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="security">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="positions">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="pools">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="links">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="transactions">
            <div class="loading-spinner">Loading...</div>
          </div>
        </div>
      </div>
    `;
  }

  _updateHeader(token) {
    // Update badges in separate row
    const badgesContainer = this.dialogEl.querySelector("#headerBadges");
    const badgesRow = this.dialogEl.querySelector("#headerBadgesRow");
    if (badgesContainer && badgesRow) {
      const badges = [];
      if (token.verified) badges.push('<span class="badge badge-success">Verified</span>');

      // Mutable/Immutable badge
      if (token.is_mutable === false) {
        badges.push('<span class="badge badge-success">Immutable</span>');
      } else if (token.is_mutable === true) {
        badges.push('<span class="badge badge-warning">Mutable</span>');
      }

      // Update Authority badge
      if (token.update_authority) {
        const auth = token.update_authority;
        const trunc = auth.slice(0, 4) + "..." + auth.slice(-4);
        badges.push(
          `<span class="badge badge-secondary" title="Update Authority: ${auth}">Auth: ${trunc}</span>`
        );
      }

      if (token.has_open_position) badges.push('<span class="badge badge-info">Position</span>');
      if (token.blacklisted) badges.push('<span class="badge badge-danger">Blacklisted</span>');
      if (token.has_ohlcv) badges.push('<span class="badge badge-secondary">OHLCV</span>');

      badgesContainer.innerHTML = badges.join("");
      // Always show badges row for layout consistency (contains status now)
      badgesRow.style.display = "flex";
    }

    // Update Last Updated time
    const lastUpdatedEl = this.dialogEl.querySelector("#lastUpdatedTime");
    if (lastUpdatedEl) {
      const marketFetchedAt = token.market_data_last_fetched_at;
      const poolFetchedAt = token.pool_price_last_calculated_at;

      // Use the most recent timestamp
      let lastTs = 0;
      if (marketFetchedAt && marketFetchedAt > lastTs) lastTs = marketFetchedAt;
      if (poolFetchedAt && poolFetchedAt > lastTs) lastTs = poolFetchedAt;

      if (lastTs > 0) {
        // Convert timestamp (seconds or milliseconds) to relative time
        // If it's very large, assume ms, but DB usually stores ms or sec.
        // Rust types.rs says i64 for ts, usually ms in JS land if passed directly.
        // Assuming backend sends milliseconds or seconds. Let's check Utils.formatTimestamp or relative
        const now = Date.now();
        // Check if timestamp is likely seconds (less than 2030 in s)
        const tsMs = lastTs < 2000000000 ? lastTs * 1000 : lastTs;
        const diff = Math.max(0, now - tsMs);

        let timeStr = "";
        if (diff < 60000) {
          timeStr = "Just now";
        } else if (diff < 3600000) {
          timeStr = Math.floor(diff / 60000) + "m ago";
        } else {
          timeStr = new Date(tsMs).toLocaleTimeString();
        }

        lastUpdatedEl.textContent = `Updated: ${timeStr}`;
      } else {
        lastUpdatedEl.textContent = "";
      }
    }

    // Update price section
    const priceContainer = this.dialogEl.querySelector("#headerPrice");
    if (priceContainer) {
      const priceHtml = this._buildHeaderPrice(token);
      priceContainer.innerHTML = priceHtml;
    }

    // Update sell button state based on open positions
    const sellBtn = this.dialogEl.querySelector("#headerSellBtn");
    if (sellBtn) {
      sellBtn.disabled = !token.has_open_position;
    }

    // Setup copy mint button
    const copyBtn = this.dialogEl.querySelector("#copyMintBtn");
    if (copyBtn && !copyBtn._hasListener) {
      copyBtn._hasListener = true;
      copyBtn.addEventListener("click", () => {
        Utils.copyToClipboard(token.mint);
        Utils.showToast("Mint address copied!", "success");
      });
    }
  }

  _buildHeaderPrice(token) {
    const priceSol =
      token.price_sol !== null && token.price_sol !== undefined
        ? Utils.formatPriceSol(token.price_sol, { decimals: 12 })
        : "—";
    const priceUsd =
      token.price_usd !== null && token.price_usd !== undefined
        ? Utils.formatCurrencyUSD(token.price_usd)
        : "";

    // Price change badge
    let changeHtml = "";
    if (token.price_change_periods) {
      const change24h = token.price_change_periods.h24;
      if (change24h !== null && change24h !== undefined) {
        const changeClass = change24h >= 0 ? "positive" : "negative";
        const sign = change24h >= 0 ? "+" : "";
        changeHtml = `<span class="price-change ${changeClass}">${sign}${change24h.toFixed(2)}%</span>`;
      }
    }

    return `
      <div class="price-block">
        <div class="price-sol-row">
          <span class="price-sol">${priceSol}</span>
          <span class="price-sol-unit">SOL</span>
        </div>
        ${priceUsd ? `<span class="price-usd">${priceUsd}</span>` : ""}
      </div>
      ${changeHtml}
      <div class="price-metrics">
        <div class="metric-item">
          <span class="metric-label">MCap</span>
          <span class="metric-value">${token.market_cap ? Utils.formatCompactNumber(token.market_cap, { prefix: "$" }) : "—"}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Liq</span>
          <span class="metric-value">${token.liquidity_usd ? Utils.formatCompactNumber(token.liquidity_usd, { prefix: "$" }) : "—"}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Vol 24H</span>
          <span class="metric-value">${token.volume_24h ? Utils.formatCompactNumber(token.volume_24h, { prefix: "$" }) : "—"}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Holders</span>
          <span class="metric-value">${token.total_holders ? Utils.formatCompactNumber(token.total_holders) : "—"}</span>
        </div>
      </div>
    `;
  }

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

    // Trade action buttons
    const buyBtn = this.dialogEl.querySelector("#headerBuyBtn");
    if (buyBtn) {
      this._buyHandler = () => this._handleBuyClick();
      buyBtn.addEventListener("click", this._buyHandler);
    }

    const sellBtn = this.dialogEl.querySelector("#headerSellBtn");
    if (sellBtn) {
      this._sellHandler = () => this._handleSellClick();
      sellBtn.addEventListener("click", this._sellHandler);
    }

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

  // =========================================================================
  // TRADE ACTIONS
  // =========================================================================

  _ensureTradeDialog() {
    if (!this.tradeDialog) {
      this.tradeDialog = new TradeActionDialog();
    }
    return this.tradeDialog;
  }

  async _getWalletBalance() {
    const now = Date.now();
    if (this.walletBalance != null && now - this.walletBalanceFetchedAt < 10000) {
      return this.walletBalance;
    }

    try {
      const data = await requestManager.fetch("/api/wallet/balance", { priority: "low" });
      const parsedBalance = Number(data?.sol_balance);
      if (Number.isFinite(parsedBalance)) {
        this.walletBalance = parsedBalance;
        this.walletBalanceFetchedAt = now;
        return this.walletBalance;
      }
    } catch (error) {
      console.warn("[TokenDetailsDialog] Failed to fetch wallet balance", error);
    }

    this.walletBalance = 0;
    this.walletBalanceFetchedAt = now;
    return this.walletBalance;
  }

  async _handleBuyClick() {
    const dialog = this._ensureTradeDialog();
    const symbol = this.fullTokenData?.symbol || this.tokenData?.symbol || "?";
    const mint = this.tokenData?.mint;
    const balance = await this._getWalletBalance();

    if (!mint) {
      Utils.showToast("No mint address available", "error");
      return;
    }

    try {
      const result = await dialog.open({
        action: "buy",
        symbol,
        context: { balance, mint },
      });

      if (!result) return; // User cancelled

      const buyBtn = this.dialogEl.querySelector("#headerBuyBtn");
      if (buyBtn) buyBtn.disabled = true;

      const response = await fetch("/api/trader/manual/buy", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          mint: this.tokenData.mint,
          ...(result.amount ? { size_sol: result.amount } : {}),
        }),
      });

      if (buyBtn) buyBtn.disabled = false;

      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.message || "Buy failed");
      }

      Utils.showToast("Buy order placed!", "success");
      this._refreshPositionsData();
      this.onTradeComplete("buy", this.tokenData.mint);
    } catch (error) {
      Utils.showToast(error.message || "Buy failed", "error");
    }
  }

  async _handleSellClick() {
    const dialog = this._ensureTradeDialog();
    const symbol = this.fullTokenData?.symbol || this.tokenData?.symbol || "?";
    const mint = this.tokenData?.mint;

    if (!mint) {
      Utils.showToast("No mint address available", "error");
      return;
    }

    // Get holdings for sell percentage calculation
    const holdings = this.fullTokenData?.holdings || 0;

    try {
      const result = await dialog.open({
        action: "sell",
        symbol,
        context: { mint, holdings },
      });

      if (!result) return; // User cancelled

      const sellBtn = this.dialogEl.querySelector("#headerSellBtn");
      if (sellBtn) sellBtn.disabled = true;

      const body =
        result.percentage === 100
          ? { mint: this.tokenData.mint, close_all: true }
          : { mint: this.tokenData.mint, percentage: result.percentage };

      const response = await fetch("/api/trader/manual/sell", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });

      if (sellBtn) sellBtn.disabled = false;

      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.message || "Sell failed");
      }

      Utils.showToast("Sell order placed!", "success");
      this._refreshPositionsData();
      this.onTradeComplete("sell", this.tokenData.mint);
    } catch (error) {
      Utils.showToast(error.message || "Sell failed", "error");
    }
  }

  async _refreshPositionsData() {
    // Refresh positions tab if loaded
    const positionsContent = this.dialogEl?.querySelector('[data-tab-content="positions"]');
    if (positionsContent) {
      positionsContent.dataset.loaded = "false";
      if (this.currentTab === "positions") {
        this._loadPositionsTab(positionsContent);
      }
    }
    // Refresh token data to update has_open_position
    await this._fetchTokenData();
  }

  _switchTab(tabId) {
    if (tabId === this.currentTab) return;

    const tabButtons = this.dialogEl.querySelectorAll(".tab-button");
    tabButtons.forEach((btn) => {
      if (btn.dataset.tab === tabId) {
        btn.classList.add("active");
      } else {
        btn.classList.remove("active");
      }
    });

    const tabContents = this.dialogEl.querySelectorAll(".tab-content");
    tabContents.forEach((content) => {
      if (content.dataset.tabContent === tabId) {
        content.classList.add("active");
      } else {
        content.classList.remove("active");
      }
    });

    if (this.currentTab === "overview" && tabId !== "overview") {
      this._stopChartPolling();
    }

    if (tabId === "overview" && this.advancedChart) {
      this._startChartPolling();
    }

    this.currentTab = tabId;
    this._loadTabContent(tabId);
  }

  _loadTabContent(tabId) {
    const content = this.dialogEl.querySelector(`[data-tab-content="${tabId}"]`);
    if (!content) return;

    if (content.dataset.loaded === "true") return;

    switch (tabId) {
      case "overview":
        this._loadOverviewTab(content);
        break;
      case "security":
        this._loadSecurityTab(content);
        break;
      case "positions":
        this._loadPositionsTab(content);
        break;
      case "pools":
        this._loadPoolsTab(content);
        break;
      case "links":
        this._loadLinksTab(content);
        break;
      case "transactions":
        this._loadTransactionsTab(content);
        break;
    }
  }

  // =========================================================================
  // OVERVIEW TAB
  // =========================================================================

  _loadOverviewTab(content) {
    // Use whatever data we have - show partial content rather than blocking
    const tokenToUse = this.fullTokenData || this.tokenData;

    if (!tokenToUse || !tokenToUse.mint) {
      content.innerHTML = '<div class="loading-spinner">Waiting for token data...</div>';
      return;
    }

    // Build overview with available data - placeholders for missing fields
    content.innerHTML = renderOverviewTab(tokenToUse, {
      renderHintTrigger: this._renderHintTrigger.bind(this),
      escapeHtml: this._escapeHtml.bind(this),
      formatShortAddress: this._formatShortAddress.bind(this),
      getRejectionDisplayLabel: this._getRejectionDisplayLabel.bind(this),
    });

    setTimeout(() => {
      this._initializeChart(tokenToUse.mint);
    }, 100);

    // Only mark as fully loaded if we have complete data
    if (this.fullTokenData && this._initialLoadComplete) {
      content.dataset.loaded = "true";
    }
  }

  // =========================================================================
  // SECURITY TAB
  // =========================================================================

  _loadSecurityTab(content) {
    // Use whatever data we have - show partial content rather than blocking
    const tokenToUse = this.fullTokenData || this.tokenData;

    if (!tokenToUse || !tokenToUse.mint) {
      content.innerHTML = '<div class="loading-spinner">Waiting for token data...</div>';
      return;
    }

    content.innerHTML = renderSecurityTab(tokenToUse, {
      renderHintTrigger: this._renderHintTrigger.bind(this),
      escapeHtml: this._escapeHtml.bind(this),
      formatShortAddress: this._formatShortAddress.bind(this),
    });

    // Check if we have security data
    const hasSecurityData =
      tokenToUse.safety_score !== undefined && tokenToUse.safety_score !== null;

    if (hasSecurityData) {
      content.dataset.loaded = "true";
    }
  }

  // =========================================================================
  // POOLS TAB
  // =========================================================================

  _loadPoolsTab(content) {
    if (!this.fullTokenData) {
      content.innerHTML = '<div class="loading-spinner">Waiting for token data...</div>';
      return;
    }

    content.innerHTML = renderPoolsTab(this.fullTokenData, {
      renderHintTrigger: this._renderHintTrigger.bind(this),
      escapeHtml: this._escapeHtml.bind(this),
      formatShortAddress: this._formatShortAddress.bind(this),
    });
    content.dataset.loaded = "true";
    initPoolsTabEvents(content);
  }

  _buildPoolsContent(token) {
    const pools = token.pools || [];

    if (pools.length === 0) {
      return `
        <div class="empty-state">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M12 2.69l5.66 5.66a8 8 0 1 1-11.31 0z"></path></svg>
          <h3>No pools found</h3>
          <p>No liquidity pools have been detected for this token.</p>
        </div>
      `;
    }

    // Calculate summary stats
    const totalLiquidity = pools.reduce((sum, p) => sum + (p.liquidity_usd || 0), 0);
    const totalVolume24h = pools.reduce((sum, p) => sum + (p.volume_h24_usd || 0), 0);
    const canonicalPool = pools.find((p) => p.is_canonical);
    const programCounts = pools.reduce((acc, p) => {
      acc[p.program] = (acc[p.program] || 0) + 1;
      return acc;
    }, {});
    const baseRoleCount = pools.filter((p) => p.token_role === "base").length;
    const quoteRoleCount = pools.filter((p) => p.token_role === "quote").length;

    // Build left column - Summary stats
    const leftCol = `
      <div class="pools-left-col">
        <div class="pools-summary-card">
          <div class="pools-summary-title">
            <span>Pool Summary</span>
            ${this._renderHintTrigger("tokenDetails.pools")}
          </div>
          <div class="pools-summary-stats">
            <div class="pools-stat">
              <span class="pools-stat-label">Total Pools</span>
              <span class="pools-stat-value">${pools.length}</span>
            </div>
            <div class="pools-stat">
              <span class="pools-stat-label">Total Liquidity</span>
              <span class="pools-stat-value">${Utils.formatCurrencyUSD(totalLiquidity)}</span>
            </div>
            <div class="pools-stat">
              <span class="pools-stat-label">Total 24h Volume</span>
              <span class="pools-stat-value">${Utils.formatCurrencyUSD(totalVolume24h)}</span>
            </div>
            <div class="pools-stat">
              <span class="pools-stat-label">Base Role</span>
              <span class="pools-stat-value">${baseRoleCount}</span>
            </div>
            <div class="pools-stat">
              <span class="pools-stat-label">Quote Role</span>
              <span class="pools-stat-value">${quoteRoleCount}</span>
            </div>
          </div>
        </div>

        <div class="pools-summary-card">
          <div class="pools-summary-title">DEX Breakdown</div>
          <div class="pools-dex-list">
            ${Object.entries(programCounts)
              .sort((a, b) => b[1] - a[1])
              .map(
                ([program, count]) => `
                <div class="pools-dex-row">
                  <span class="pools-dex-name">${this._escapeHtml(program)}</span>
                  <span class="pools-dex-count">${count}</span>
                </div>
              `
              )
              .join("")}
          </div>
        </div>

        ${
          canonicalPool
            ? `
        <div class="pools-summary-card canonical-highlight">
          <div class="pools-summary-title">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"></path></svg>
            Canonical Pool
          </div>
          <div class="pools-canonical-info">
            <div class="pools-stat">
              <span class="pools-stat-label">DEX</span>
              <span class="pools-stat-value">${this._escapeHtml(canonicalPool.program)}</span>
            </div>
            <div class="pools-stat">
              <span class="pools-stat-label">Liquidity</span>
              <span class="pools-stat-value">${canonicalPool.liquidity_usd ? Utils.formatCurrencyUSD(canonicalPool.liquidity_usd) : "—"}</span>
            </div>
            <div class="pools-stat">
              <span class="pools-stat-label">Volume 24h</span>
              <span class="pools-stat-value">${canonicalPool.volume_h24_usd ? Utils.formatCurrencyUSD(canonicalPool.volume_h24_usd) : "—"}</span>
            </div>
          </div>
        </div>
        `
            : ""
        }
      </div>
    `;

    // Build right column - Pool details
    const poolCards = pools.map((pool) => this._buildPoolDetailCard(pool)).join("");
    const rightCol = `
      <div class="pools-right-col">
        <div class="pools-list-header">
          <span class="pools-list-title">All Pools (${pools.length})</span>
        </div>
        <div class="pools-list">
          ${poolCards}
        </div>
      </div>
    `;

    return `<div class="pools-container">${leftCol}${rightCol}</div>`;
  }

  _buildPoolDetailCard(pool) {
    const lastUpdated = pool.last_updated_unix
      ? Utils.formatTimestamp(pool.last_updated_unix * 1000)
      : "—";

    const reserveAccountsHtml =
      pool.reserve_accounts && pool.reserve_accounts.length > 0
        ? pool.reserve_accounts
            .map(
              (addr) => `
            <div class="pool-reserve-item">
              <span class="pool-reserve-addr" title="${this._escapeHtml(addr)}">${this._formatShortAddress(addr)}</span>
              <button class="copy-btn-mini" data-copy="${this._escapeHtml(addr)}" title="Copy address">
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
              </button>
            </div>
          `
            )
            .join("")
        : '<span class="pool-no-data">No reserve accounts</span>';

    return `
      <div class="pool-detail-card ${pool.is_canonical ? "canonical" : ""}">
        <div class="pool-detail-header">
          <div class="pool-detail-left">
            <span class="pool-detail-program">${this._escapeHtml(pool.program)}</span>
            ${pool.is_canonical ? '<span class="pool-canonical-badge">★ Canonical</span>' : ""}
          </div>
          <div class="pool-detail-role ${pool.token_role}">${this._escapeHtml(pool.token_role)}</div>
        </div>

        <div class="pool-detail-body">
          <div class="pool-detail-section">
            <div class="pool-detail-row">
              <span class="pool-detail-label">Pool Address</span>
              <div class="pool-detail-value-group">
                <span class="pool-detail-value mono" title="${this._escapeHtml(pool.pool_id)}">${this._formatShortAddress(pool.pool_id)}</span>
                <button class="copy-btn-mini" data-copy="${this._escapeHtml(pool.pool_id)}" title="Copy pool address">
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
                </button>
              </div>
            </div>

            <div class="pool-detail-row">
              <span class="pool-detail-label">Base Mint</span>
              <div class="pool-detail-value-group">
                <span class="pool-detail-value mono" title="${this._escapeHtml(pool.base_mint)}">${this._formatShortAddress(pool.base_mint)}</span>
                <button class="copy-btn-mini" data-copy="${this._escapeHtml(pool.base_mint)}" title="Copy base mint">
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
                </button>
              </div>
            </div>

            <div class="pool-detail-row">
              <span class="pool-detail-label">Quote Mint</span>
              <div class="pool-detail-value-group">
                <span class="pool-detail-value mono" title="${this._escapeHtml(pool.quote_mint)}">${this._formatShortAddress(pool.quote_mint)}</span>
                <button class="copy-btn-mini" data-copy="${this._escapeHtml(pool.quote_mint)}" title="Copy quote mint">
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
                </button>
              </div>
            </div>

            <div class="pool-detail-row">
              <span class="pool-detail-label">Paired Mint</span>
              <div class="pool-detail-value-group">
                <span class="pool-detail-value mono" title="${this._escapeHtml(pool.paired_mint)}">${this._formatShortAddress(pool.paired_mint)}</span>
                <button class="copy-btn-mini" data-copy="${this._escapeHtml(pool.paired_mint)}" title="Copy paired mint">
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
                </button>
              </div>
            </div>
          </div>

          <div class="pool-detail-divider"></div>

          <div class="pool-detail-section">
            <div class="pool-detail-row">
              <span class="pool-detail-label">Liquidity</span>
              <span class="pool-detail-value highlight">${pool.liquidity_usd ? Utils.formatCurrencyUSD(pool.liquidity_usd) : "—"}</span>
            </div>

            <div class="pool-detail-row">
              <span class="pool-detail-label">Volume 24h</span>
              <span class="pool-detail-value highlight">${pool.volume_h24_usd ? Utils.formatCurrencyUSD(pool.volume_h24_usd) : "—"}</span>
            </div>

            <div class="pool-detail-row">
              <span class="pool-detail-label">Last Updated</span>
              <span class="pool-detail-value muted">${lastUpdated}</span>
            </div>
          </div>

          <div class="pool-detail-divider"></div>

          <div class="pool-detail-section">
            <div class="pool-detail-row reserves-row">
              <span class="pool-detail-label">Reserve Accounts (${pool.reserve_accounts?.length || 0})</span>
            </div>
            <div class="pool-reserves-list">
              ${reserveAccountsHtml}
            </div>
          </div>
        </div>
      </div>
    `;
  }

  // =========================================================================
  // LINKS TAB
  // =========================================================================

  _loadLinksTab(content) {
    if (!this.fullTokenData) {
      content.innerHTML = '<div class="loading-spinner">Waiting for token data...</div>';
      return;
    }

    content.innerHTML = renderLinksTab(this.fullTokenData, {
      escapeHtml: this._escapeHtml.bind(this),
      formatShortAddress: this._formatShortAddress.bind(this),
    });
    content.dataset.loaded = "true";
    initLinksTabEvents(content);
  }

  _buildLinksContent(token) {
    const mint = token.mint;
    const hasWebsites = token.websites && token.websites.length > 0;
    const hasSocials = token.socials && token.socials.length > 0;
    const hasLogo = !!token.logo_url;
    const hasHeader = !!token.header_image_url;
    const hasDescription = !!token.description;

    // Build left column - Media & Info
    const leftCol = this._buildLinksLeftColumn(token, hasLogo, hasHeader, hasDescription);

    // Build right column - All links organized
    const rightCol = this._buildLinksRightColumn(token, mint, hasWebsites, hasSocials);

    return `<div class="links-container">${leftCol}${rightCol}</div>`;
  }

  _buildLinksLeftColumn(token, hasLogo, hasHeader, hasDescription) {
    const mint = token.mint;

    // Token info section
    const tokenInfoSection = `
      <div class="links-info-card">
        <div class="links-info-title">
          <i class="icon-info"></i>
          Token Info
        </div>
        <div class="links-info-content">
          <div class="links-info-row">
            <span class="links-info-label">Mint Address</span>
            <div class="links-info-value-group">
              <span class="links-info-value mono" title="${this._escapeHtml(mint)}">${this._formatShortAddress(mint)}</span>
              <button class="copy-btn-mini" data-copy="${this._escapeHtml(mint)}" title="Copy mint address">
                <i class="icon-copy"></i>
              </button>
            </div>
          </div>
          ${
            token.data_source
              ? `
          <div class="links-info-row">
            <span class="links-info-label">Data Source</span>
            <span class="links-info-value badge">${this._escapeHtml(token.data_source)}</span>
          </div>
          `
              : ""
          }
          ${
            token.verified
              ? `
          <div class="links-info-row">
            <span class="links-info-label">Status</span>
            <span class="links-info-value badge success"><i class="icon-shield-check"></i> Verified</span>
          </div>
          `
              : ""
          }
        </div>
      </div>
    `;

    // Media section - logo and header
    let mediaSection = "";
    if (hasLogo || hasHeader) {
      const logoHtml = hasLogo
        ? `
        <div class="links-media-item">
          <div class="links-media-label">Logo</div>
          <div class="links-media-preview logo">
            <img src="${this._escapeHtml(token.logo_url)}" alt="Token Logo" onerror="this.parentElement.innerHTML='<i class=\\'icon-image-off\\'></i>'" />
          </div>
          <a href="${this._escapeHtml(token.logo_url)}" target="_blank" class="links-media-link">
            <i class="icon-external-link"></i> Open Image
          </a>
        </div>
      `
        : "";

      const headerHtml = hasHeader
        ? `
        <div class="links-media-item">
          <div class="links-media-label">Header Image</div>
          <div class="links-media-preview header">
            <img src="${this._escapeHtml(token.header_image_url)}" alt="Header Image" onerror="this.parentElement.innerHTML='<i class=\\'icon-image-off\\'></i>'" />
          </div>
          <a href="${this._escapeHtml(token.header_image_url)}" target="_blank" class="links-media-link">
            <i class="icon-external-link"></i> Open Image
          </a>
        </div>
      `
        : "";

      mediaSection = `
        <div class="links-info-card">
          <div class="links-info-title">
            <i class="icon-image"></i>
            Media Assets
          </div>
          <div class="links-media-grid">
            ${logoHtml}
            ${headerHtml}
          </div>
        </div>
      `;
    }

    // Description section
    let descriptionSection = "";
    if (hasDescription) {
      descriptionSection = `
        <div class="links-info-card">
          <div class="links-info-title">
            <i class="icon-file-text"></i>
            Description
          </div>
          <div class="links-description">
            ${this._escapeHtml(token.description)}
          </div>
        </div>
      `;
    }

    return `
      <div class="links-left-col">
        ${tokenInfoSection}
        ${mediaSection}
        ${descriptionSection}
      </div>
    `;
  }

  _buildLinksRightColumn(token, mint, hasWebsites, hasSocials) {
    // Explorers section - comprehensive list
    const explorersSection = `
      <div class="links-section-card">
        <div class="links-section-title">
          <i class="icon-search"></i>
          Explorers & Analytics
        </div>
        <div class="links-grid-compact">
          ${this._buildExplorerLink("https://solscan.io/token/" + mint, "Solscan")}
          ${this._buildExplorerLink("https://explorer.solana.com/address/" + mint, "Solana Explorer")}
          ${this._buildExplorerLink("https://birdeye.so/token/" + mint + "?chain=solana", "Birdeye")}
          ${this._buildExplorerLink("https://dexscreener.com/solana/" + mint, "DEX Screener")}
          ${this._buildExplorerLink("https://www.geckoterminal.com/solana/tokens/" + mint, "GeckoTerminal")}
          ${this._buildExplorerLink("https://www.dextools.io/app/en/solana/pair-explorer/" + mint, "DexTools")}
          ${this._buildExplorerLink("https://gmgn.ai/sol/token/" + mint, "GMGN")}
          ${this._buildExplorerLink("https://photon-sol.tinyastro.io/en/lp/" + mint, "Photon")}
          ${this._buildExplorerLink("https://rugcheck.xyz/tokens/" + mint, "RugCheck")}
          ${this._buildExplorerLink("https://app.bubblemaps.io/sol/token/" + mint, "Bubblemaps")}
          ${this._buildExplorerLink("https://www.coingecko.com/en/coins/" + mint, "CoinGecko")}
          ${this._buildExplorerLink("https://jup.ag/swap/SOL-" + mint, "Jupiter Swap")}
        </div>
      </div>
    `;

    // Official websites section
    let websitesSection = "";
    if (hasWebsites) {
      const websiteLinks = token.websites
        .map((site) => {
          const label = site.label || this._extractDomainName(site.url) || "Website";
          return this._buildOfficialLink(site.url, label);
        })
        .join("");

      websitesSection = `
        <div class="links-section-card">
          <div class="links-section-title">
            <i class="icon-globe"></i>
            Official Websites
          </div>
          <div class="links-list">
            ${websiteLinks}
          </div>
        </div>
      `;
    }

    // Social links section
    let socialsSection = "";
    if (hasSocials) {
      const socialLinks = token.socials
        .map((social) => {
          const { label } = this._getSocialMeta(social.platform);
          return this._buildSocialLink(social.url, label);
        })
        .join("");

      socialsSection = `
        <div class="links-section-card">
          <div class="links-section-title">
            <i class="icon-share-2"></i>
            Social Media
          </div>
          <div class="links-list">
            ${socialLinks}
          </div>
        </div>
      `;
    }

    // No links message
    let noLinksMessage = "";
    if (!hasWebsites && !hasSocials) {
      noLinksMessage = `
        <div class="links-empty-notice">
          <i class="icon-link-2-off"></i>
          <span>No official website or social links available for this token.</span>
        </div>
      `;
    }

    return `
      <div class="links-right-col">
        ${explorersSection}
        ${websitesSection}
        ${socialsSection}
        ${noLinksMessage}
      </div>
    `;
  }

  // =========================================================================
  // TRANSACTIONS TAB
  // =========================================================================

  async _loadTransactionsTab(content) {
    if (content.dataset.loaded === "true") return;

    content.innerHTML = '<div class="loading-spinner">Loading transactions...</div>';

    try {
      // Fetch 24h of transactions (limit 1000 usually enough for chart unless very high volume)
      const response = await requestManager.fetch(
        `/api/tokens/${this.tokenData.mint}/transactions?limit=1000`
      );

      if (response && response.success) {
        const transactions = response.data || [];
        content.innerHTML = this._buildTransactionsHTML();

        // Wait for DOM
        setTimeout(() => {
          this._renderTransactionsChart(transactions);
          this._renderTransactionsList(transactions);
        }, 50);

        content.dataset.loaded = "true";
      } else {
        content.innerHTML = '<div class="empty-state">No transaction data available</div>';
      }
    } catch (err) {
      console.error("Failed to load transactions:", err);
      content.innerHTML = '<div class="error-state">Failed to load transactions</div>';
    }
  }

  _buildTransactionsHTML() {
    return `
      <div class="transactions-container" style="display: flex; flex-direction: column; gap: 16px; padding: 16px; height: 100%; overflow: auto;">
        <div class="transactions-chart-section" style="background: var(--bg-surface); padding: 12px; border-radius: 8px; border: 1px solid var(--border-color);">
          <div class="section-header" style="margin-bottom: 8px; font-size: 14px; color: var(--text-secondary);">
             <span style="font-weight: 600; color: var(--text-primary);">24h Transaction Activity</span>
             <span style="font-size: 12px; opacity: 0.7;">(Hourly Count)</span>
          </div>
          <div id="txns-chart" style="width: 100%; height: 200px;"></div>
        </div>
        <div class="transactions-list-section" style="background: var(--bg-surface); border-radius: 8px; border: 1px solid var(--border-color); flex: 1; display: flex; flex-direction: column; overflow: hidden;">
          <div class="section-header" style="padding: 12px; border-bottom: 1px solid var(--border-color); font-weight: 600; color: var(--text-primary);">Last 100 Transactions</div>
          <div id="txns-list" class="simple-table-container" style="overflow-y: auto; flex: 1;"></div>
        </div>
      </div>
    `;
  }

  _renderTransactionsChart(transactions) {
    const chartContainer = this.dialogEl.querySelector("#txns-chart");
    if (!chartContainer) return;

    // Aggregate by hour buckets
    const buckets = {};
    const now = Math.floor(Date.now() / 1000);
    const start = now - 24 * 3600;

    // Initialize buckets
    for (let i = 0; i < 24; i++) {
      const ts = start + i * 3600;
      const hourKey = Math.floor(ts / 3600) * 3600;
      buckets[hourKey] = { time: hourKey, value: 0 };
    }

    transactions.forEach((tx) => {
      // Use timestamp (ISO string from backend)
      const date = new Date(tx.timestamp);
      const ts = date.getTime() / 1000;

      if (ts < start) return;
      const hourKey = Math.floor(ts / 3600) * 3600;
      if (!buckets[hourKey]) buckets[hourKey] = { time: hourKey, value: 0 };
      buckets[hourKey].value += 1;
    });

    const data = Object.values(buckets).sort((a, b) => a.time - b.time);

    // Create Chart
    // Ensure LightweightCharts is available
    if (!window.LightweightCharts) {
      chartContainer.innerHTML = "Chart library missing";
      return;
    }

    const chart = window.LightweightCharts.createChart(chartContainer, {
      layout: { background: { type: "solid", color: "transparent" }, textColor: "#8b949e" },
      grid: { vertLines: { visible: false }, horzLines: { color: "#30363d" } },
      rightPriceScale: { borderVisible: false, scaleMargins: { top: 0.1, bottom: 0 } },
      timeScale: { borderVisible: false, timeVisible: true, secondsVisible: false },
      crosshair: { vertLine: { labelVisible: false }, horzLine: { labelVisible: false } }, // minimal
    });

    const series = chart.addHistogramSeries({ color: "#238636" });
    series.setData(data);
    chart.timeScale().fitContent();

    // Auto-resize
    const resizeObserver = new ResizeObserver((entries) => {
      if (entries.length === 0 || !entries[0].contentRect) return;
      const { width, height } = entries[0].contentRect;
      chart.applyOptions({ width, height });
    });
    resizeObserver.observe(chartContainer);

    // Save reference for cleanup if needed
    this.txChart = chart;
  }

  _renderTransactionsList(transactions) {
    const container = this.dialogEl.querySelector("#txns-list");
    if (!container) return;

    const recent = transactions.slice(0, 100);

    // Simple HTML table for speed
    const rows = recent
      .map((tx) => {
        // Use safe field access (backend returns transaction_type, sol_delta)
        const txType = (tx.transaction_type || tx.type || "UNKNOWN").toLowerCase();
        const isBuy =
          txType.includes("buy") ||
          (txType === "swap" && (tx.direction || "").toLowerCase() === "incoming");

        const typeLabel = txType.toUpperCase();

        // Style based on type
        let typeStyle = "color: var(--text-secondary);";
        if (isBuy) typeStyle = "color: var(--success-color, #3fb950);";
        else if (
          txType.includes("sell") ||
          (txType === "swap" && (tx.direction || "").toLowerCase() === "outgoing")
        )
          typeStyle = "color: var(--error-color, #f85149);";

        const timeDisplay = new Date(tx.timestamp).toLocaleTimeString();

        // Price (if available) - generic transactions typically don't have price
        const price = tx.price_sol ? Utils.formatPriceSol(tx.price_sol) : "—";

        // Total SOL (use sol_delta absolute value)
        const amount = tx.amount_sol !== undefined ? tx.amount_sol : Math.abs(tx.sol_delta || 0);
        const total = Utils.formatNumber(amount, { decimals: 2 });

        return `
         <div style="display: grid; grid-template-columns: 1fr 0.8fr 1.2fr 1fr 0.5fr; gap: 8px; padding: 8px 12px; border-bottom: 1px solid var(--border-color); font-size: 13px;">
           <span style="color: var(--text-secondary);">${timeDisplay}</span>
           <span style="font-weight: 600; ${typeStyle}">${typeLabel}</span>
           <span style="font-family: monospace;">${price}</span>
           <span>${total} SOL</span>
           <a href="https://solscan.io/tx/${tx.signature}" target="_blank" style="color: var(--text-secondary); text-align: right;"><i class="icon-external-link"></i></a>
         </div>
       `;
      })
      .join("");

    container.innerHTML = `
      <div style="display: flex; flex-direction: column;">
         <div style="display: grid; grid-template-columns: 1fr 0.8fr 1.2fr 1fr 0.5fr; gap: 8px; padding: 8px 12px; background: var(--bg-input, #0d1117); font-size: 12px; color: var(--text-secondary); font-weight: 600; position: sticky; top: 0;">
           <span>Time</span>
           <span>Type</span>
           <span>Price</span>
           <span>Total</span>
           <span>Link</span>
         </div>
         <div style="flex: 1;">
           ${rows}
         </div>
      </div>
    `;
  }

  _buildExplorerLink(url, name) {
    return `
      <a href="${this._escapeHtml(url)}" target="_blank" rel="noopener noreferrer" class="links-explorer-item">
        <span>${this._escapeHtml(name)}</span>
        <i class="icon-external-link link-external-icon"></i>
      </a>
    `;
  }

  _buildOfficialLink(url, label) {
    return `
      <a href="${this._escapeHtml(url)}" target="_blank" rel="noopener noreferrer" class="links-official-item">
        <div class="links-official-content">
          <span class="links-official-label">${this._escapeHtml(label)}</span>
          <span class="links-official-url">${this._escapeHtml(this._formatUrl(url))}</span>
        </div>
        <i class="icon-external-link link-external-icon"></i>
      </a>
    `;
  }

  _buildSocialLink(url, label) {
    const username = this._extractSocialUsername(url);
    return `
      <a href="${this._escapeHtml(url)}" target="_blank" rel="noopener noreferrer" class="links-social-item">
        <div class="links-social-content">
          <span class="links-social-platform">${this._escapeHtml(label)}</span>
          ${username ? `<span class="links-social-handle">${this._escapeHtml(username)}</span>` : ""}
        </div>
        <i class="icon-external-link link-external-icon"></i>
      </a>
    `;
  }

  _getSocialMeta(platform) {
    const platformLower = platform?.toLowerCase() || "";
    const socialMap = {
      twitter: { icon: "icon-twitter", label: "Twitter / X" },
      x: { icon: "icon-twitter", label: "X (Twitter)" },
      telegram: { icon: "icon-send", label: "Telegram" },
      discord: { icon: "icon-message-circle", label: "Discord" },
      medium: { icon: "icon-book-open", label: "Medium" },
      github: { icon: "icon-github", label: "GitHub" },
      youtube: { icon: "icon-youtube", label: "YouTube" },
      reddit: { icon: "icon-message-square", label: "Reddit" },
      facebook: { icon: "icon-facebook", label: "Facebook" },
      instagram: { icon: "icon-instagram", label: "Instagram" },
      linkedin: { icon: "icon-linkedin", label: "LinkedIn" },
      tiktok: { icon: "icon-music", label: "TikTok" },
    };
    return socialMap[platformLower] || { icon: "icon-link", label: platform || "Link" };
  }

  _getSocialColorClass(platform) {
    const platformLower = platform?.toLowerCase() || "";
    const colorMap = {
      "twitter / x": "social-twitter",
      "x (twitter)": "social-twitter",
      telegram: "social-telegram",
      discord: "social-discord",
      youtube: "social-youtube",
      github: "social-github",
      medium: "social-medium",
      reddit: "social-reddit",
    };
    return colorMap[platformLower] || "social-default";
  }

  _extractDomainName(url) {
    try {
      const domain = new URL(url).hostname;
      return domain.replace(/^www\./, "");
    } catch {
      return null;
    }
  }

  _formatUrl(url) {
    try {
      const parsed = new URL(url);
      return parsed.hostname + (parsed.pathname !== "/" ? parsed.pathname : "");
    } catch {
      return url;
    }
  }

  _extractSocialUsername(url) {
    try {
      const parsed = new URL(url);
      const path = parsed.pathname.replace(/^\/+|\/+$/g, "");
      if (path && !path.includes("/")) {
        return "@" + path;
      }
      return null;
    } catch {
      return null;
    }
  }

  // =========================================================================
  // CHART
  // =========================================================================

  async _initializeChart(mint) {
    const chartContainer = this.dialogEl.querySelector("#tradingview-chart");
    const timeframeButtons = this.dialogEl.querySelector("#timeframeButtons");

    if (!chartContainer) {
      console.error("Chart container not found");
      return;
    }

    if (!window.createAdvancedChart) {
      console.error("AdvancedChart not available");
      return;
    }

    // Determine current theme
    const isDarkMode = document.documentElement.getAttribute("data-theme") === "dark";

    // Create advanced chart instance
    this.advancedChart = window.createAdvancedChart(chartContainer, {
      theme: isDarkMode ? "dark" : "light",
      chartType: "candlestick",
      showVolume: true,
      showGrid: true,
      showCrosshair: true,
      showLegend: false, // We have our own OHLCV display in header
      showTooltip: true,
      priceFormat: "auto",
      pricePrecision: 12,
      barSpacing: 12,
      minBarSpacing: 4,
      indicators: [],
      watermark: {
        text: this.tokenData?.symbol || "",
        fontSize: 32,
        color: isDarkMode ? "rgba(128, 128, 128, 0.1)" : "rgba(128, 128, 128, 0.08)",
      },
    });

    // Store reference for cleanup
    this.chart = this.advancedChart;

    // Get initial timeframe from active button
    const activeBtn = timeframeButtons?.querySelector(".timeframe-btn.active");
    this.currentTimeframe = activeBtn?.dataset.tf || "5m";

    await this._loadChartData(mint, this.currentTimeframe, true); // Initial load - set view

    this._startChartPolling();

    // Handle timeframe button clicks
    if (timeframeButtons) {
      timeframeButtons.addEventListener("click", async (e) => {
        const btn = e.target.closest(".timeframe-btn");
        if (!btn) return;

        // Update active state
        timeframeButtons
          .querySelectorAll(".timeframe-btn")
          .forEach((b) => b.classList.remove("active"));
        btn.classList.add("active");

        this.currentTimeframe = btn.dataset.tf;
        await this._triggerOhlcvRefresh();
        await new Promise((resolve) => setTimeout(resolve, 500));
        await this._loadChartData(mint, this.currentTimeframe, true); // Timeframe change - reset view
      });
    }

    // Listen for theme changes and update chart
    this._themeObserver = new MutationObserver((mutations) => {
      mutations.forEach((mutation) => {
        if (mutation.type === "attributes" && mutation.attributeName === "data-theme") {
          const newTheme = document.documentElement.getAttribute("data-theme") || "dark";
          if (this.advancedChart) {
            this.advancedChart.setTheme(newTheme);
          }
        }
      });
    });
    this._themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });

    // Add position markers if we have position data
    this._updateChartPositions();
  }

  _updateChartPositions() {
    if (!this.advancedChart || !this.positionsData) return;

    this.advancedChart.clearPositionMarkers();

    // Add entry markers for each position entry
    if (this.positionsData.entries && this.positionsData.entries.length > 0) {
      this.positionsData.entries.forEach((entry, idx) => {
        if (entry.price_sol && entry.timestamp) {
          this.advancedChart.addPositionMarker({
            type: idx === 0 ? "entry" : "dca",
            price: entry.price_sol,
            timestamp: Math.floor(new Date(entry.timestamp).getTime() / 1000),
            label: idx === 0 ? "Entry" : `DCA ${idx}`,
          });
        }
      });
    }

    // Add exit markers for closed positions
    if (this.positionsData.exits && this.positionsData.exits.length > 0) {
      this.positionsData.exits.forEach((exit, idx) => {
        if (exit.price_sol && exit.timestamp) {
          this.advancedChart.addPositionMarker({
            type: "exit",
            price: exit.price_sol,
            timestamp: Math.floor(new Date(exit.timestamp).getTime() / 1000),
            label: `Exit ${idx + 1}`,
          });
        }
      });
    }

    // Add stop loss / take profit lines from current position
    if (this.positionsData.stop_loss_price) {
      this.advancedChart.addHorizontalLine({
        price: this.positionsData.stop_loss_price,
        color: "#ef4444",
        label: "Stop Loss",
        style: 2,
      });
    }

    if (this.positionsData.take_profit_price) {
      this.advancedChart.addHorizontalLine({
        price: this.positionsData.take_profit_price,
        color: "#10b981",
        label: "Take Profit",
        style: 2,
      });
    }
  }

  async _loadChartData(mint, timeframe, isInitialLoad = false) {
    const loadingOverlay = this.dialogEl?.querySelector("#chartLoadingOverlay");
    const loadingText = loadingOverlay?.querySelector(".chart-loading-text");

    try {
      // Use requestManager with high priority for initial chart data load
      const data = await requestManager.fetch(`/api/tokens/${mint}/ohlcv?timeframe=${timeframe}`, {
        priority: isInitialLoad ? "high" : "normal",
      });

      if (!Array.isArray(data) || data.length === 0) {
        // No data yet - show waiting message
        if (loadingText) {
          loadingText.textContent = "Waiting for chart data...";
        }
        if (loadingOverlay) {
          loadingOverlay.classList.remove("hidden");
        }
        this.chartDataLoaded = false;
        return;
      }

      if (!this.advancedChart) return;

      const chartData = data.map((candle) => ({
        time: candle.timestamp,
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
        volume: candle.volume || 0,
      }));

      // setData now respects user interactions - only fits on first load
      this.advancedChart.setData(chartData);

      // Hide loading overlay when data arrives
      if (loadingOverlay) {
        loadingOverlay.classList.add("hidden");
      }
      this.chartDataLoaded = true;

      // Update OHLCV display with latest candle
      this._updateOhlcvDisplay(chartData);

      // Update position markers after loading data
      this._updateChartPositions();

      // Only set initial visible range on first load of this timeframe
      // Chart will auto-preserve user's zoom/pan on subsequent updates
      if (isInitialLoad && chartData.length > 0) {
        // Reset interaction flag and set initial view
        this.advancedChart.resetUserInteraction();
        this.advancedChart.setVisibleRange(80);
      }
    } catch (error) {
      // On error, show waiting message
      if (loadingText) {
        loadingText.textContent = "Waiting for chart data...";
      }
      if (loadingOverlay) {
        loadingOverlay.classList.remove("hidden");
      }
      this.chartDataLoaded = false;
    }
  }

  _updateOhlcvDisplay(chartData) {
    if (!chartData || chartData.length === 0) return;

    const latest = chartData[chartData.length - 1];
    const ohlcvOpen = this.dialogEl?.querySelector("#ohlcvOpen");
    const ohlcvHigh = this.dialogEl?.querySelector("#ohlcvHigh");
    const ohlcvLow = this.dialogEl?.querySelector("#ohlcvLow");
    const ohlcvClose = this.dialogEl?.querySelector("#ohlcvClose");
    const ohlcvChange = this.dialogEl?.querySelector("#ohlcvChange");

    if (ohlcvOpen) ohlcvOpen.textContent = Utils.formatPriceSol(latest.open, { decimals: 9 });
    if (ohlcvHigh) ohlcvHigh.textContent = Utils.formatPriceSol(latest.high, { decimals: 9 });
    if (ohlcvLow) ohlcvLow.textContent = Utils.formatPriceSol(latest.low, { decimals: 9 });
    if (ohlcvClose) ohlcvClose.textContent = Utils.formatPriceSol(latest.close, { decimals: 9 });

    if (ohlcvChange && latest.open && latest.close) {
      const changePercent = ((latest.close - latest.open) / latest.open) * 100;
      const sign = changePercent >= 0 ? "+" : "";
      ohlcvChange.textContent = `${sign}${changePercent.toFixed(2)}%`;
      ohlcvChange.className = `ohlcv-change ${changePercent >= 0 ? "positive" : "negative"}`;
    }
  }

  // =========================================================================
  // UTILITIES
  // =========================================================================

  _formatShortAddress(address) {
    if (!address || address.length < 16) return address || "—";
    return `${address.substring(0, 6)}...${address.substring(address.length - 6)}`;
  }

  _formatPnLWithPercent(solValue, percentValue) {
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
  }

  _formatChange(value) {
    if (value === null || value === undefined) return "—";
    const sign = value >= 0 ? "+" : "";
    return `${sign}${value.toFixed(2)}%`;
  }

  _getChangeClass(value) {
    if (value === null || value === undefined) return "";
    return value >= 0 ? "positive" : "negative";
  }

  /**
   * Render a hint trigger for card headers
   */
  _renderHintTrigger(hintKey) {
    const hint = Hints.getHint(hintKey);
    if (!hint) return "";
    return HintTrigger.render(hint, hintKey, { size: "sm", position: "bottom" });
  }

  _escapeHtml(text) {
    if (!text) return "";
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }

  // Rejection reason label mapping (machine code -> human-readable)
  _getRejectionDisplayLabel(reasonCode) {
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
  }

  destroy() {
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
  }
}

// ============================================================================
// Global Event Listener for Context Menu "View Details" Action
// ============================================================================
// This listener allows any page to open the TokenDetailsDialog via custom event
// dispatched from context_menu.js when user clicks "View Details"

let globalDialogInstance = null;

window.addEventListener("screenerbot:open-token-details", async (event) => {
  const { mint, symbol } = event.detail || {};

  if (!mint) {
    console.error("[TokenDetailsDialog] Event received without mint address");
    return;
  }

  console.log(`[TokenDetailsDialog] Opening details for ${symbol || mint}`);

  // Close existing dialog if open for a different token
  if (globalDialogInstance && globalDialogInstance.dialogEl) {
    if (globalDialogInstance.tokenData?.mint === mint) {
      // Already open for this token, do nothing
      console.log("[TokenDetailsDialog] Dialog already open for this token");
      return;
    }
    globalDialogInstance.close();
    await new Promise((resolve) => setTimeout(resolve, 350));
  }

  // Create new dialog instance if needed
  if (!globalDialogInstance) {
    globalDialogInstance = new TokenDetailsDialog({
      onClose: () => {
        // Keep instance for reuse, just clean up state
      },
    });
  }

  // Open dialog with minimal token data (dialog will fetch full details)
  await globalDialogInstance.show({ mint, symbol: symbol || "" });
});
