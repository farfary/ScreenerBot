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
    content.innerHTML = this._buildOverviewHTML(tokenToUse);

    setTimeout(() => {
      this._initializeChart(tokenToUse.mint);
    }, 100);

    // Only mark as fully loaded if we have complete data
    if (this.fullTokenData && this._initialLoadComplete) {
      content.dataset.loaded = "true";
    }
  }

  _buildOverviewHTML(token) {
    return `
      <div class="overview-split-layout">
        <div class="overview-left">
          ${this._buildQuickStats(token)}
          ${this._buildOverviewContent(token)}
        </div>
        <div class="overview-right">
          <div class="chart-container">
            <div class="chart-header">
              <div class="chart-header-left">
                <span class="chart-title">Price Chart</span>
                ${this._renderHintTrigger("tokenDetails.chart")}
              </div>
              <div class="chart-ohlcv-display" id="chartOhlcvDisplay">
                <span class="ohlcv-item"><span class="ohlcv-label">O</span> <span class="ohlcv-value" id="ohlcvOpen">—</span></span>
                <span class="ohlcv-item"><span class="ohlcv-label">H</span> <span class="ohlcv-value" id="ohlcvHigh">—</span></span>
                <span class="ohlcv-item"><span class="ohlcv-label">L</span> <span class="ohlcv-value" id="ohlcvLow">—</span></span>
                <span class="ohlcv-item"><span class="ohlcv-label">C</span> <span class="ohlcv-value" id="ohlcvClose">—</span></span>
                <span class="ohlcv-change" id="ohlcvChange">—</span>
              </div>
              <div class="chart-controls">
                <div class="timeframe-buttons" id="timeframeButtons">
                  <button class="timeframe-btn" data-tf="1m">1M</button>
                  <button class="timeframe-btn active" data-tf="5m">5M</button>
                  <button class="timeframe-btn" data-tf="15m">15M</button>
                  <button class="timeframe-btn" data-tf="1h">1H</button>
                  <button class="timeframe-btn" data-tf="4h">4H</button>
                  <button class="timeframe-btn" data-tf="12h">12H</button>
                  <button class="timeframe-btn" data-tf="1d">1D</button>
                </div>
              </div>
            </div>
            <div id="tradingview-chart" class="tradingview-chart"></div>
            <div id="chartLoadingOverlay" class="chart-loading-overlay">
              <div class="chart-loading-content">
                <div class="chart-loading-spinner"></div>
                <div class="chart-loading-text">Loading chart data...</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  _buildQuickStats(token) {
    const change24h = token.price_change_periods?.h24;
    const changeClass = change24h >= 0 ? "positive" : "negative";

    return `
      <div class="quick-stats">
        <div class="quick-stat">
          <span class="stat-label">Price</span>
          <span class="stat-value">${token.price_sol ? Utils.formatPriceSol(token.price_sol, { decimals: 9 }) + " SOL" : "—"}</span>
          ${change24h !== undefined ? `<span class="stat-change ${changeClass}">${this._formatChange(change24h)}</span>` : ""}
        </div>
        <div class="quick-stat">
          <span class="stat-label">Market Cap</span>
          <span class="stat-value">${token.market_cap ? Utils.formatCompactNumber(token.market_cap, { prefix: "$" }) : token.fdv ? Utils.formatCompactNumber(token.fdv, { prefix: "$" }) : "—"}</span>
        </div>
        <div class="quick-stat">
          <span class="stat-label">Liquidity</span>
          <span class="stat-value">${token.liquidity_usd ? Utils.formatCompactNumber(token.liquidity_usd, { prefix: "$" }) : token.pool_reserves_sol ? Utils.formatSol(token.pool_reserves_sol, { decimals: 2 }) : "—"}</span>
        </div>
        <div class="quick-stat">
          <span class="stat-label">Vol 24H</span>
          <span class="stat-value">${token.volume_24h ? Utils.formatCompactNumber(token.volume_24h, { prefix: "$" }) : "—"}</span>
        </div>
      </div>
    `;
  }

  _buildOverviewContent(token) {
    return `
      <div class="overview-grid">
        ${this._buildTokenInfoCard(token)}
        ${this._buildLiquidityCard(token)}
        ${this._buildPriceChangesCard(token)}
        ${this._buildVolumeCard(token)}
        ${this._buildActivityCard(token)}
      </div>
    `;
  }

  _buildTokenInfoCard(token) {
    const age = token.pair_created_at
      ? Utils.formatTimeAgo(new Date(token.pair_created_at * 1000))
      : token.created_at
        ? Utils.formatTimeAgo(new Date(token.created_at * 1000))
        : "—";

    // Build tags display with wrapper to prevent height jump
    const tagsContent =
      token.tags && token.tags.length > 0
        ? `<div class="token-tags">${token.tags.map((t) => `<span class="token-tag">${this._escapeHtml(t)}</span>`).join("")}</div>`
        : '<div class="token-tags-placeholder">No tags</div>';
    const tagsHtml = `<div class="token-tags-wrapper">${tagsContent}</div>`;

    // Build filtering status display
    let filteringStatusHtml = "";
    if (token.last_rejection_reason) {
      const displayLabel = this._getRejectionDisplayLabel(token.last_rejection_reason);
      filteringStatusHtml = `
        <div class="info-cell full-width">
          <span class="cell-label">Filter Status</span>
          <span class="cell-value">
            <span class="status-badge rejected" title="${this._escapeHtml(token.last_rejection_reason)}">
              Rejected: ${this._escapeHtml(displayLabel)}
            </span>
          </span>
        </div>
      `;
    }

    return `
      <div class="info-card compact">
        <div class="card-header">
          <span>Token Info</span>
          <div class="card-header-actions">
            ${token.verified ? '<span class="verified-badge"><i class="icon-check"></i> Verified</span>' : ""}
            ${this._renderHintTrigger("tokenDetails.tokenInfo")}
          </div>
        </div>
        <div class="card-body">
          <div class="info-grid-2col">
            <div class="info-cell">
              <span class="cell-label">Mint</span>
              <span class="cell-value mono clickable" onclick="Utils.copyToClipboard('${token.mint}')" title="Click to copy">${this._formatShortAddress(token.mint)}</span>
            </div>
            <div class="info-cell">
              <span class="cell-label">Decimals</span>
              <span class="cell-value">${token.decimals ?? "—"}</span>
            </div>
            <div class="info-cell">
              <span class="cell-label">Age</span>
              <span class="cell-value">${age}</span>
            </div>
            <div class="info-cell">
              <span class="cell-label">DEX</span>
              <span class="cell-value">${token.pool_dex ? this._escapeHtml(token.pool_dex) : "—"}</span>
            </div>
            ${
              token.total_holders
                ? `
            <div class="info-cell">
              <span class="cell-label">Holders</span>
              <span class="cell-value">${Utils.formatNumber(token.total_holders, { decimals: 0 })}</span>
            </div>
            `
                : ""
            }
            ${
              token.top_10_concentration
                ? `
            <div class="info-cell">
              <span class="cell-label">Top 10 Hold</span>
              <span class="cell-value">${token.top_10_concentration.toFixed(1)}%</span>
            </div>
            `
                : ""
            }
            ${filteringStatusHtml}
          </div>
          ${tagsHtml}
          ${token.description ? `<div class="info-description">${this._escapeHtml(token.description)}</div>` : ""}
        </div>
      </div>
    `;
  }

  _buildLiquidityCard(token) {
    return `
      <div class="info-card compact">
        <div class="card-header">
          <span>Liquidity & Market</span>
          ${this._renderHintTrigger("tokenDetails.liquidity")}
        </div>
        <div class="card-body">
          <div class="info-grid-2col">
            <div class="info-cell highlight">
              <span class="cell-label">FDV</span>
              <span class="cell-value large">${token.fdv ? Utils.formatCurrencyUSD(token.fdv) : "—"}</span>
            </div>
            <div class="info-cell highlight">
              <span class="cell-label">Liquidity</span>
              <span class="cell-value large">${token.liquidity_usd ? Utils.formatCurrencyUSD(token.liquidity_usd) : "—"}</span>
            </div>
            <div class="info-cell">
              <span class="cell-label">Pool SOL</span>
              <span class="cell-value">${token.pool_reserves_sol ? Utils.formatNumber(token.pool_reserves_sol, { decimals: 2 }) + " SOL" : "—"}</span>
            </div>
            <div class="info-cell">
              <span class="cell-label">Pool Token</span>
              <span class="cell-value">${token.pool_reserves_token ? Utils.formatCompactNumber(token.pool_reserves_token) : "—"}</span>
            </div>
          </div>
          ${
            token.pool_address
              ? `
          <div class="pool-address">
            <span class="cell-label">Pool</span>
            <a href="https://solscan.io/account/${token.pool_address}" target="_blank" rel="noopener" class="pool-link">${this._formatShortAddress(token.pool_address)}</a>
          </div>
          `
              : ""
          }
        </div>
      </div>
    `;
  }

  _buildPriceChangesCard(token) {
    const changes = token.price_change_periods || {};
    return `
      <div class="info-card compact">
        <div class="card-header">
          <span>Price Changes</span>
          ${this._renderHintTrigger("tokenDetails.priceChanges")}
        </div>
        <div class="card-body">
          <div class="change-grid">
            <div class="change-item">
              <span class="change-label">5M</span>
              <span class="change-value ${this._getChangeClass(changes.m5)}">${this._formatChange(changes.m5)}</span>
            </div>
            <div class="change-item">
              <span class="change-label">1H</span>
              <span class="change-value ${this._getChangeClass(changes.h1)}">${this._formatChange(changes.h1)}</span>
            </div>
            <div class="change-item">
              <span class="change-label">6H</span>
              <span class="change-value ${this._getChangeClass(changes.h6)}">${this._formatChange(changes.h6)}</span>
            </div>
            <div class="change-item">
              <span class="change-label">24H</span>
              <span class="change-value ${this._getChangeClass(changes.h24)}">${this._formatChange(changes.h24)}</span>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  _buildVolumeCard(token) {
    const volumes = token.volume_periods || {};
    return `
      <div class="info-card compact">
        <div class="card-header">
          <span>Trading Volume</span>
          ${this._renderHintTrigger("tokenDetails.volume")}
        </div>
        <div class="card-body">
          <div class="volume-grid-4">
            <div class="volume-item">
              <span class="volume-label">5M</span>
              <span class="volume-value">${volumes.m5 ? Utils.formatCompactNumber(volumes.m5, { prefix: "$" }) : "—"}</span>
            </div>
            <div class="volume-item">
              <span class="volume-label">1H</span>
              <span class="volume-value">${volumes.h1 ? Utils.formatCompactNumber(volumes.h1, { prefix: "$" }) : "—"}</span>
            </div>
            <div class="volume-item">
              <span class="volume-label">6H</span>
              <span class="volume-value">${volumes.h6 ? Utils.formatCompactNumber(volumes.h6, { prefix: "$" }) : "—"}</span>
            </div>
            <div class="volume-item">
              <span class="volume-label">24H</span>
              <span class="volume-value">${volumes.h24 ? Utils.formatCompactNumber(volumes.h24, { prefix: "$" }) : "—"}</span>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  _buildActivityCard(token) {
    const txns = token.txn_periods || {};
    const buySellRatio = token.buy_sell_ratio_24h;
    const ratioClass = buySellRatio
      ? buySellRatio > 1
        ? "bullish"
        : buySellRatio < 1
          ? "bearish"
          : "neutral"
      : "";

    const h24 = txns.h24;
    const buys24 = typeof token.buys_24h === "number" ? token.buys_24h : h24?.buys;
    const sells24 = typeof token.sells_24h === "number" ? token.sells_24h : h24?.sells;
    const total24 =
      (typeof buys24 === "number" ? buys24 : 0) + (typeof sells24 === "number" ? sells24 : 0);
    const buyPct24 = total24 > 0 && typeof buys24 === "number" ? (buys24 / total24) * 100 : null;

    const m5 = txns.m5;
    const h1 = txns.h1;
    const total5m =
      (typeof m5?.buys === "number" ? m5.buys : 0) + (typeof m5?.sells === "number" ? m5.sells : 0);
    const total1h =
      (typeof h1?.buys === "number" ? h1.buys : 0) + (typeof h1?.sells === "number" ? h1.sells : 0);
    const rate5m = total5m / 5;
    const rate1h = total1h / 60;
    const spikeFactor = rate1h > 0 ? rate5m / rate1h : null;

    const netFlow24h = typeof token.net_flow_24h === "number" ? token.net_flow_24h : null;
    const netFlowLabel =
      typeof netFlow24h === "number"
        ? netFlow24h > 0
          ? `+${Utils.formatNumber(netFlow24h, { decimals: 0 })}`
          : Utils.formatNumber(netFlow24h, { decimals: 0 })
        : "—";
    const netFlowClass =
      typeof netFlow24h === "number" ? (netFlow24h >= 0 ? "positive" : "negative") : "";

    return `
      <div class="info-card compact">
        <div class="card-header">
          <span>Transaction Activity</span>
          <div class="card-header-actions">
            ${
              typeof buyPct24 === "number"
                ? `<span class="ratio-badge ${buyPct24 >= 50 ? "bullish" : "bearish"}">${buyPct24.toFixed(0)}% Buy</span>`
                : ""
            }
            ${buySellRatio ? `<span class="ratio-badge ${ratioClass}">${buySellRatio.toFixed(2)} B/S</span>` : ""}
            ${this._renderHintTrigger("tokenDetails.activity")}
          </div>
        </div>
        <div class="card-body">
          <div class="txn-grid">
            ${this._buildTxnRow("5M", txns.m5, { minutes: 5 })}
            ${this._buildTxnRow("1H", txns.h1, { minutes: 60 })}
            ${this._buildTxnRow("6H", txns.h6, { minutes: 360 })}
            ${this._buildTxnRow("24H", txns.h24, { minutes: 1440 })}
          </div>
          ${
            typeof token.buys_24h === "number" || typeof token.sells_24h === "number"
              ? `
          <div class="txn-summary">
            <div class="txn-summary-item buys">
              <span class="summary-icon">↗</span>
              <span class="summary-value">${typeof token.buys_24h === "number" ? Utils.formatNumber(token.buys_24h, { decimals: 0 }) : "—"}</span>
              <span class="summary-label">Buys</span>
            </div>
            <div class="txn-summary-item sells">
              <span class="summary-icon">↘</span>
              <span class="summary-value">${typeof token.sells_24h === "number" ? Utils.formatNumber(token.sells_24h, { decimals: 0 }) : "—"}</span>
              <span class="summary-label">Sells</span>
            </div>
            ${
              typeof token.net_flow_24h === "number"
                ? `
            <div class="txn-summary-item flow ${netFlowClass}">
              <span class="summary-icon">${netFlow24h >= 0 ? "+" : "−"}</span>
              <span class="summary-value">${netFlowLabel}</span>
              <span class="summary-label">Net</span>
            </div>
            `
                : ""
            }
          </div>
          `
              : ""
          }

          <div class="txn-insights">
            <div class="txn-insight">
              <div class="insight-label">24H Total</div>
              <div class="insight-value mono">${total24 > 0 ? Utils.formatNumber(total24, { decimals: 0 }) : "—"}</div>
            </div>
            <div class="txn-insight">
              <div class="insight-label">24H Avg</div>
              <div class="insight-value mono">${
                total24 > 0 ? `${Utils.formatNumber(total24 / 24, { decimals: 1 })}/h` : "—"
              }</div>
            </div>
            <div class="txn-insight">
              <div class="insight-label">5M Spike</div>
              <div class="insight-value mono">${
                typeof spikeFactor === "number" && Number.isFinite(spikeFactor)
                  ? `${Utils.formatNumber(spikeFactor, { decimals: 2 })}×`
                  : "—"
              }</div>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  _buildTxnRow(label, data, { minutes }) {
    const buysRaw = data?.buys;
    const sellsRaw = data?.sells;

    const hasBuys = typeof buysRaw === "number";
    const hasSells = typeof sellsRaw === "number";
    const hasAny = hasBuys || hasSells;

    const buys = hasBuys ? buysRaw : null;
    const sells = hasSells ? sellsRaw : null;
    const total = (typeof buys === "number" ? buys : 0) + (typeof sells === "number" ? sells : 0);

    const buyPct = total > 0 && typeof buys === "number" ? (buys / total) * 100 : 50;
    const sellPct = 100 - buyPct;

    const countsTitle = hasAny
      ? `Buys: ${typeof buys === "number" ? buys : "—"} (${total > 0 ? buyPct.toFixed(0) : "—"}%), Sells: ${typeof sells === "number" ? sells : "—"} (${total > 0 ? sellPct.toFixed(0) : "—"}%), Total: ${total}`
      : "No transaction data";

    const buyText = typeof buys === "number" ? Utils.formatNumber(buys, { decimals: 0 }) : "—";
    const sellText = typeof sells === "number" ? Utils.formatNumber(sells, { decimals: 0 }) : "—";
    const ratePerMin = minutes && total >= 0 ? total / minutes : null;
    const rateText = hasAny ? `${Utils.formatNumber(ratePerMin ?? 0, { decimals: 1 })}/m` : "—";
    const pctText =
      total > 0 ? `${buyPct.toFixed(0)}% / ${sellPct.toFixed(0)}%` : hasAny ? "0% / 0%" : "—";

    return `
      <div class="txn-row" title="${countsTitle}">
        <div class="txn-time">
          <div class="txn-label">${label}</div>
          <div class="txn-rate">${rateText}</div>
        </div>
        <div class="txn-bar-container ${hasAny ? "" : "is-empty"}" aria-label="${countsTitle}">
          <div class="txn-bar buy-bar" style="width: ${buyPct}%"></div>
          <div class="txn-bar sell-bar" style="width: ${sellPct}%"></div>
        </div>
        <div class="txn-counts">
          <div class="txn-counts-main">
            <span class="buy-count">${buyText}</span>
            <span class="separator">/</span>
            <span class="sell-count">${sellText}</span>
          </div>
          <div class="txn-counts-sub">${pctText}</div>
        </div>
      </div>
    `;
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

    // Check if we have security data
    const hasSecurityData =
      tokenToUse.safety_score !== undefined && tokenToUse.safety_score !== null;

    if (!hasSecurityData) {
      // Show partial content with loading indicator for security section
      content.innerHTML = this._buildSecurityLoadingContent(tokenToUse);
      return;
    }

    content.innerHTML = this._buildSecurityContent(tokenToUse);
    content.dataset.loaded = "true";
  }

  /**
   * Build security tab with loading state for missing data
   */
  _buildSecurityLoadingContent(token) {
    return `
      <div class="security-container">
        <div class="security-left-col">
          <div class="security-loading-notice">
            <div class="loading-spinner-small"></div>
            <span>Rugcheck analysis in progress...</span>
          </div>
          <div class="security-header security-loading" style="--i:0">
            <div class="security-header-title">
              <span class="section-title"><i class="icon-shield-check"></i> Security Pulse</span>
              ${this._renderHintTrigger("tokenDetails.security")}
            </div>
            <div class="security-score-container">
              <div class="security-score-circle">
                <svg class="score-progress" width="120" height="120" viewBox="0 0 120 120">
                  <circle class="score-bg" cx="60" cy="60" r="46"></circle>
                </svg>
                <div class="score-content">
                  <span class="score-value" style="opacity: 0.3;">—</span>
                </div>
              </div>
              <div class="safety-badge" style="background: var(--bg-secondary); color: var(--text-muted);">
                Analyzing...
              </div>
            </div>
          </div>
          <div class="security-bento-grid">
             <div class="security-bento-card security-loading" style="--i:1"></div>
             <div class="security-bento-card security-loading" style="--i:2"></div>
          </div>
          ${this._buildAuthoritiesCard(token)}
        </div>
        <div class="security-right-col">
          <div class="security-card security-loading" style="height: 400px; --i:3">
             <div class="loading-spinner" style="margin-top: 150px;"></div>
          </div>
        </div>
      </div>
    `;
  }

  _buildSecurityContent(token) {
    // Use safety_score (0-100, higher = safer) for display
    const safetyScore = token.safety_score;
    const scoreClass = this._getSafetyScoreClass(safetyScore);
    const scoreLabel = this._getSafetyScoreLabel(safetyScore);

    return `
      <div class="security-container">
        <div class="security-left-col">
          ${this._buildSecurityHeader(token, safetyScore, scoreClass, scoreLabel)}
          ${this._buildSecurityOverview(token)}
          ${this._buildAuthoritiesCard(token)}
        </div>
        <div class="security-right-col">
          ${this._buildHoldersCard(token)}
          ${this._buildTransferFeeCard(token)}
          ${this._buildRisksSection(token.security_risks)}
          ${this._buildTopHoldersSection(token)}
        </div>
      </div>
    `;
  }

  _buildSecurityHeader(token, safetyScore, scoreClass, scoreLabel) {
    const lastUpdated = token.security_last_updated
      ? Utils.formatTimestamp(token.security_last_updated)
      : null;

    // Safety score is 0-100 (higher = safer)
    const score = safetyScore ?? 0;
    const scorePercent = Math.min(100, Math.max(0, score));
    const circumference = 2 * Math.PI * 46; // radius = 46 for 120px ring
    const offset = circumference - (scorePercent / 100) * circumference;

    // Risk score for additional info
    const riskScore = token.risk_score;

    // Map score label to safety badge
    const badgeConfigs = {
      "score-good": { label: "Shielded", class: "good" },
      "score-ok": { label: "Safe", class: "good" },
      "score-warn": { label: "Caution", class: "warning" },
      "score-danger": { label: "Vulnerable", class: "critical" },
    };
    const badge = badgeConfigs[scoreClass] || { label: scoreLabel, class: "warning" };

    return `
      <div class="security-header" style="--i:0">
        <div class="security-header-title">
          <span class="section-title"><i class="icon-shield-check"></i> Security Pulse</span>
          ${this._renderHintTrigger("tokenDetails.security")}
        </div>
        <div class="security-score-container">
          <div class="security-score-circle">
            <div class="score-glow ${scoreClass}"></div>
            <svg class="score-progress" width="120" height="120" viewBox="0 0 120 120">
              <circle class="score-bg" cx="60" cy="60" r="46"></circle>
              <circle class="score-ring ${scoreClass}" cx="60" cy="60" r="46" 
                style="stroke-dasharray: ${circumference}; stroke-dashoffset: ${offset};"></circle>
            </svg>
            <div class="score-content">
              <span class="score-value">${score}</span>
              <span class="score-max">SCORE</span>
            </div>
          </div>
          <div class="safety-badge ${badge.class}">
            ${badge.label}
          </div>
        </div>
        ${
          token.rugged
            ? `
        <div class="rugged-warning" style="margin-top: 15px; border-radius: 10px;">
          <i class="icon-skull" style="font-size: 20px;"></i>
          <span style="font-size: 1rem; letter-spacing: 0.1em; font-weight: 800;">RUGGED</span>
        </div>
        `
            : ""
        }
        ${
          lastUpdated && !token.rugged
            ? `<div style="text-align: center; font-size: 0.65rem; color: var(--text-muted); margin-top: 10px;">Updated ${lastUpdated}</div>`
            : ""
        }
      </div>
    `;
  }

  _buildSecurityOverview(token) {
    const items = [];

    if (token.token_type) {
      items.push({
        label: "Token Type",
        value: token.token_type,
        icon: '<i class="icon-box"></i>',
      });
    }

    if (token.total_holders !== null && token.total_holders !== undefined) {
      items.push({
        label: "Total Holders",
        value: Utils.formatCompactNumber(token.total_holders),
        icon: '<i class="icon-users"></i>',
      });
    }

    if (token.lp_provider_count !== null && token.lp_provider_count !== undefined) {
      items.push({
        label: "LP Providers",
        value: Utils.formatNumber(token.lp_provider_count, { decimals: 0 }),
        icon: '<i class="icon-droplet"></i>',
      });
    }

    if (token.graph_insiders_detected !== null && token.graph_insiders_detected !== undefined) {
      const isDangerous = token.graph_insiders_detected > 0;
      items.push({
        label: "Graph Insiders",
        value: `${isDangerous ? "Detected" : "Clean"} ${token.graph_insiders_detected > 0 ? `(${token.graph_insiders_detected})` : ""}`,
        icon: isDangerous ? '<i class="icon-alert-triangle"></i>' : '<i class="icon-search"></i>',
        class: isDangerous ? "warning" : "good",
      });
    }

    if (items.length === 0) return "";

    return `
      <div class="security-bento-grid" style="margin-top: 8px;">
        ${items
          .map(
            (item, idx) => `
          <div class="security-bento-card" style="--i:${idx + 1}">
            <div class="bento-icon">${item.icon}</div>
            <div class="bento-label">${item.label}</div>
            <div class="bento-value">${item.value}</div>
          </div>
        `
          )
          .join("")}
      </div>
    `;
  }

  _buildAuthoritiesCard(token) {
    return `
      <div class="security-card" style="--i:2">
        <div class="card-header">
          <span>Token Control</span>
          <span class="card-subtitle">Authority status</span>
        </div>
        <div class="card-body">
          <div class="authority-grid">
            <div class="authority-item ${token.mint_authority ? "danger" : "safe"}">
              <div class="authority-header">
                <span class="authority-label">Mint <i class="icon-wrench"></i></span>
                ${this._renderAuthorityBadge(token.mint_authority)}
              </div>
              ${
                token.mint_authority
                  ? `
              <div class="authority-address">
                <span class="address-value" title="${token.mint_authority}">${this._formatShortAddress(token.mint_authority)}</span>
                <button class="btn-copy-mini" onclick="Utils.copyToClipboard('${token.mint_authority}')"><i class="icon-copy"></i></button>
              </div>
              <div class="authority-status-text">At Risk</div>
              `
                  : '<div class="authority-status-text">Immutable</div>'
              }
            </div>
            
            <div class="authority-item ${token.freeze_authority ? "danger" : "safe"}">
              <div class="authority-header">
                <span class="authority-label">Freeze <i class="icon-snowflake"></i></span>
                ${this._renderAuthorityBadge(token.freeze_authority)}
              </div>
              ${
                token.freeze_authority
                  ? `
              <div class="authority-address">
                <span class="address-value" title="${token.freeze_authority}">${this._formatShortAddress(token.freeze_authority)}</span>
                <button class="btn-copy-mini" onclick="Utils.copyToClipboard('${token.freeze_authority}')"><i class="icon-copy"></i></button>
              </div>
              <div class="authority-status-text">At Risk</div>
              `
                  : '<div class="authority-status-text">Revoked</div>'
              }
            </div>
          </div>
        </div>
      </div>
    `;
  }

  _buildHoldersCard(token) {
    const top10Pct =
      token.top_10_holders_pct !== undefined
        ? token.top_10_holders_pct
        : token.top_10_concentration;
    const creatorPct = token.creator_balance_pct;

    // Determine concentration risk level
    let concentrationClass = "good";
    let concentrationLabel = "Healthy";
    if (top10Pct !== null && top10Pct !== undefined) {
      if (top10Pct > 80) {
        concentrationClass = "danger";
        concentrationLabel = "Critical";
      } else if (top10Pct > 60) {
        concentrationClass = "warning";
        concentrationLabel = "High";
      } else if (top10Pct > 40) {
        concentrationClass = "moderate";
        concentrationLabel = "Moderate";
      }
    }

    const gaugeHtml = this._buildHolderGauge(top10Pct, concentrationClass);

    return `
      <div class="security-card" style="--i:2">
        <div class="card-header">
          <span>Holder Health</span>
          ${concentrationLabel ? `<span class="concentration-badge ${concentrationClass}" style="margin-left:auto">${concentrationLabel}</span>` : ""}
        </div>
        <div class="card-body" style="padding: 16px;">
           <div style="display: flex; align-items: center; justify-content: space-around; gap: 20px;">
             <div style="text-align: center;">
                ${gaugeHtml}
                <div style="margin-top: 8px; font-size: 10px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 1px; font-weight: 600;">Top 10 Supply</div>
             </div>
             
             <div style="display: flex; flex-direction: column; gap: 14px; flex: 1;">
                <div class="holder-stat-item">
                   <div style="font-size: 10px; color: var(--text-secondary); text-transform: uppercase; margin-bottom: 2px;">Total Holders</div>
                   <div style="font-weight: 700; font-size: 16px; color: var(--text-primary); display: flex; align-items: baseline; gap: 4px;">
                      ${token.total_holders ? Utils.formatNumber(token.total_holders, { decimals: 0 }) : "—"}
                      <span style="font-size: 10px; font-weight: 400; color: var(--text-muted);">unique</span>
                   </div>
                </div>
                
                ${
                  creatorPct !== null && creatorPct !== undefined
                    ? `
                <div class="holder-stat-item">
                   <div style="font-size: 10px; color: var(--text-secondary); text-transform: uppercase; margin-bottom: 2px;">Creator Share</div>
                   <div style="font-weight: 700; font-size: 16px; color: ${creatorPct > 10 ? "var(--error-color)" : "var(--success-color)"};">
                      ${creatorPct.toFixed(2)}%
                   </div>
                </div>
                `
                    : ""
                }
             </div>
           </div>
        </div>
      </div>
    `;
  }

  _buildHolderGauge(percent, colorClass) {
    if (percent === null || percent === undefined)
      return `
        <div class="gauge-placeholder" style="width:90px;height:90px;display:flex;align-items:center;justify-content:center;color:var(--text-secondary);background:var(--bg-secondary);border-radius:50%;border:1px dashed var(--border-color);">
            <span style="font-size: 20px; opacity: 0.5;">?</span>
        </div>`;

    const p = Math.min(100, Math.max(0, percent));
    const r = 38;
    const c = 2 * Math.PI * r;
    const off = c - (p / 100) * c;

    // Color mapping
    let strokeColor = "var(--success-color)";
    if (colorClass === "danger") strokeColor = "var(--error-color)";
    if (colorClass === "warning") strokeColor = "#d29922";
    if (colorClass === "moderate") strokeColor = "#db6d28";

    return `
        <div class="gauge-wrapper" style="position:relative; width:90px; height:90px;">
             <svg width="90" height="90" viewBox="0 0 90 90" style="transform: rotate(-90deg);">
              <circle cx="45" cy="45" r="${r}" fill="none" stroke="var(--bg-secondary)" stroke-width="8"></circle>
              <circle cx="45" cy="45" r="${r}" fill="none" stroke="${strokeColor}" stroke-width="8"
                style="stroke-dasharray: ${c}; stroke-dashoffset: ${off}; transition: stroke-dashoffset 1.5s cubic-bezier(0.4, 0, 0.2, 1); stroke-linecap: round; filter: drop-shadow(0 0 3px ${strokeColor}44);"></circle>
            </svg>
            <div style="position:absolute; top:50%; left:50%; transform:translate(-50%, -50%); text-align:center;">
               <div style="font-size:16px; font-weight:800; color:var(--text-primary); font-family:var(--font-mono);">${p.toFixed(0)}<span style="font-size: 10px; margin-left: 1px;">%</span></div>
            </div>
        </div>
     `;
  }

  _buildTransferFeeCard(token) {
    // Only show if token has transfer fee data
    if (token.transfer_fee_pct === null && token.transfer_fee_pct === undefined) {
      return "";
    }

    const hasFee = token.transfer_fee_pct > 0;

    return `
      <div class="security-card ${hasFee ? "has-fee" : ""}" style="--i:2.5">
        <div class="card-header">
          <span>Transfer Tax</span>
          ${hasFee ? `<span class="fee-badge"><i class="icon-alert-triangle"></i> ${token.transfer_fee_pct}%</span>` : '<span class="no-fee-badge"><i class="icon-check"></i> No Fee</span>'}
        </div>
        <div class="card-body" style="padding: 0;">
          ${
            hasFee
              ? `
          <div class="fee-details">
            <div class="fee-row">
              <span class="fee-label">Fee Percentage</span>
              <span class="fee-value">${token.transfer_fee_pct}%</span>
            </div>
            ${
              token.transfer_fee_max_amount
                ? `
            <div class="fee-row">
              <span class="fee-label">Max Fee Amount</span>
              <span class="fee-value">${Utils.formatNumber(token.transfer_fee_max_amount)}</span>
            </div>
            `
                : ""
            }
            ${
              token.transfer_fee_authority
                ? `
            <div class="fee-row">
              <span class="fee-label">Fee Authority</span>
              <span class="fee-value" title="${token.transfer_fee_authority}">${this._formatShortAddress(token.transfer_fee_authority)}</span>
            </div>
            `
                : ""
            }
          </div>
          <div style="padding: 10px 16px; border-top: 1px solid var(--border-color); font-size: 0.65rem; color: var(--warning-color); background: var(--warning-alpha-10); display: flex; align-items: center; gap: 8px;">
            <i class="icon-alert-circle" style="font-size: 14px;"></i>
            <span>A ${token.transfer_fee_pct}% fee is charged on every transfer</span>
          </div>
          `
              : `
          <div style="padding: 16px; color: var(--success-color); font-size: 0.75rem; font-weight: 600;">
             <i class="icon-shield"></i> No transfer fees detected.
          </div>
          `
          }
        </div>
      </div>
    `;
  }

  _buildTopHoldersSection(token) {
    const topHolders = token.top_holders;
    if (!topHolders || topHolders.length === 0) {
      return "";
    }

    // Calculate concentration (use backend value or fallback sum)
    let concentration = token.top_10_concentration;
    if (concentration === undefined || concentration === null) {
      const limit = Math.min(topHolders.length, 10);
      concentration = topHolders.slice(0, limit).reduce((sum, h) => sum + (h.percentage || 0), 0);
    }

    // Top 3 Podium
    const top3 = topHolders.slice(0, 3);
    const podiumHtml = `
      <div class="holders-podium small" style="margin: 20px 0;">
        ${[1, 0, 2]
          .map((idx) => {
            const h = top3[idx];
            if (!h) return '<div class="podium-spot empty"></div>';
            const name =
              h.owner_type && h.owner_type.length < 15
                ? h.owner_type
                : this._formatShortAddress(h.address);
            return `
            <div class="podium-spot rank-${idx + 1}" title="${h.address}">
              <div class="podium-avatar">${idx + 1 === 1 ? '<i class="icon-crown"></i>' : idx + 1}</div>
              <div class="podium-value">${h.percentage.toFixed(1)}%</div>
              <div class="podium-pedestal"></div>
              <div class="podium-name">${name}</div>
            </div>
          `;
          })
          .join("")}
      </div>
    `;

    const holderRows = topHolders
      .slice(3, 10)
      .map((holder, idx) => {
        const insiderClass = holder.is_insider ? "insider" : "";
        const ownerLabel = holder.owner_type || "";
        const badges = [];
        if (holder.is_insider) badges.push('<span class="insider-badge">Insider</span>');

        if (ownerLabel) {
          const isAddress = ownerLabel.length > 30 && !ownerLabel.includes(" ");
          const displayLabel = isAddress ? this._formatShortAddress(ownerLabel) : ownerLabel;
          const cssClass = isAddress ? "owner-badge address-badge" : "owner-badge";
          badges.push(
            `<span class="${cssClass}" title="${this._escapeHtml(ownerLabel)}">${this._escapeHtml(displayLabel)}</span>`
          );
        }

        return `
          <div class="holder-row ${insiderClass}" style="--i: ${idx + 4}">
            <div class="holder-rank">#${idx + 4}</div>
            <div class="holder-address-container">
              <span class="holder-address mono">${this._formatShortAddress(holder.address)}</span>
              <div class="holder-badges">${badges.join("")}</div>
            </div>
            <div class="holder-share">${holder.percentage.toFixed(2)}%</div>
          </div>
        `;
      })
      .join("");

    return `
      <div class="security-card main-holders-card" style="--i:3.5">
        <div class="card-header">
          <span>Top 10 Holders Concentration</span>
          <span class="concentration-value">${concentration.toFixed(2)}%</span>
        </div>
        <div class="card-body" style="padding: 10px 16px;">
          ${podiumHtml}
          <div class="holders-list-small">
            ${holderRows}
          </div>
        </div>
      </div>
    `;
  }

  _renderAuthorityBadge(value) {
    if (value === null || value === undefined || value === "") {
      return '<span class="status-badge status-safe">Revoked</span>';
    }
    return '<span class="status-badge status-danger">Present</span>';
  }

  _buildRisksSection(risks) {
    if (!risks || risks.length === 0) {
      return `
        <div class="security-card" style="--i:3">
          <div class="card-header">
            <span>Security Risks</span>
          </div>
          <div class="card-body">
            <div class="no-data-message" style="color: var(--success-color); font-weight: 700; display: flex; align-items: center; gap: 8px;">
               <i class="icon-sparkles"></i> No security risks detected.
            </div>
          </div>
        </div>
      `;
    }

    const riskItems = risks
      .map((risk, idx) => {
        const level = risk.level?.toLowerCase() || "info";
        const riskClass = level === "danger" ? "risk-danger" : level === "warn" ? "risk-warn" : "";
        const icon =
          level === "danger" ? '<i class="icon-ban"></i>' : '<i class="icon-alert-triangle"></i>';

        return `
        <div class="risk-row ${riskClass}" style="--i:${idx}">
          <div class="risk-icon">${icon}</div>
          <div class="risk-details">
            <div class="risk-name">${this._escapeHtml(risk.name)}</div>
            <div class="risk-description">${this._escapeHtml(risk.description)}</div>
          </div>
        </div>
      `;
      })
      .join("");

    return `
      <div class="security-card" style="--i:3">
        <div class="card-header">
          <span>Security Risks</span>
          <span class="card-subtitle">${risks.length} incidents found</span>
        </div>
        <div class="card-body" style="padding: 0;">
          <div class="risks-list">
            ${riskItems}
          </div>
        </div>
      </div>
    `;
  }

  // Safety score classification (0-100, higher = safer)
  _getSafetyScoreClass(score) {
    if (score === null || score === undefined) return "";
    if (score >= 70) return "score-safe";
    if (score >= 40) return "score-caution";
    return "score-vulnerable";
  }

  _getSafetyScoreLabel(score) {
    if (score === null || score === undefined) return "Unknown";
    if (score >= 90) return "Shielded";
    if (score >= 70) return "Safe";
    if (score >= 40) return "Caution";
    return "Vulnerable";
  }

  // Deprecated - kept for reference but use _getSafetyScoreClass instead
  _getScoreClass(score) {
    if (score === null || score === undefined) return "";
    if (score >= 700) return "score-good";
    if (score >= 500) return "score-ok";
    if (score >= 300) return "score-warn";
    return "score-danger";
  }

  _getScoreLabel(score) {
    if (score === null || score === undefined) return "Unknown";
    if (score >= 700) return "Good";
    if (score >= 500) return "OK";
    if (score >= 300) return "Risky";
    return "Danger";
  }

  // ========================================================================
  // POSITIONS TAB
  // =========================================================================

  async _loadPositionsTab(content) {
    content.innerHTML = '<div class="loading-spinner">Loading positions...</div>';

    try {
      const response = await fetch(`/api/positions?mint=${this.tokenData.mint}`);
      if (!response.ok) {
        throw new Error("Failed to fetch positions");
      }
      const positions = await response.json();
      content.innerHTML = this._buildPositionsContent(positions);
      content.dataset.loaded = "true";
      this._attachPositionsEventHandlers(content);
    } catch (error) {
      console.error("Error loading positions:", error);
      content.innerHTML = `
        <div class="empty-state">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><line x1="12" y1="20" x2="12" y2="10"></line><line x1="18" y1="20" x2="18" y2="4"></line><line x1="6" y1="20" x2="6" y2="16"></line></svg>
          <h3>No positions found</h3>
          <p>You don't have any positions for this token.</p>
        </div>
      `;
      content.dataset.loaded = "true";
    }
  }

  _attachPositionsEventHandlers(content) {
    // Handle sell buttons in position cards
    content.querySelectorAll(".btn-position-sell").forEach((btn) => {
      btn.addEventListener("click", async (e) => {
        e.preventDefault();
        e.stopPropagation();
        await this._handleSellClick();
      });
    });

    // Handle add buttons in position cards (DCA)
    content.querySelectorAll(".btn-position-add").forEach((btn) => {
      btn.addEventListener("click", async (e) => {
        e.preventDefault();
        e.stopPropagation();
        await this._handleBuyClick();
      });
    });

    // Handle copy buttons
    content.querySelectorAll(".btn-copy-mini").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        e.preventDefault();
        e.stopPropagation();
        const text = btn.dataset.copy;
        if (text) {
          Utils.copyToClipboard(text);
          Utils.showToast("Copied to clipboard", "success");
        }
      });
    });
  }

  _buildPositionsContent(positions) {
    if (!positions || positions.length === 0) {
      return `
        <div class="empty-state">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><line x1="12" y1="20" x2="12" y2="10"></line><line x1="18" y1="20" x2="18" y2="4"></line><line x1="6" y1="20" x2="6" y2="16"></line></svg>
          <h3>No positions found</h3>
          <p>You don't have any positions for this token.</p>
        </div>
      `;
    }

    // Store positions data for sell actions
    this.positionsData = positions;

    // Separate open and closed positions
    const openPositions = positions.filter((p) => !p.exit_time);
    const closedPositions = positions.filter((p) => p.exit_time);

    // Two-column layout when we have both open and closed positions
    if (openPositions.length > 0 && closedPositions.length > 0) {
      return `
        <div class="positions-container">
          <div class="positions-col">
            <div class="positions-section">
              <h3 class="positions-section-title">Open Positions (${openPositions.length})</h3>
              <div class="positions-list">${openPositions.map((pos) => this._buildOpenPositionCard(pos)).join("")}</div>
            </div>
          </div>
          <div class="positions-col">
            <div class="positions-section">
              <h3 class="positions-section-title">Closed Positions (${closedPositions.length})</h3>
              <div class="positions-list">${closedPositions.map((pos) => this._buildClosedPositionCard(pos)).join("")}</div>
            </div>
          </div>
        </div>
      `;
    }

    // Single column for only open or only closed
    let html = '<div class="positions-single-col">';

    if (openPositions.length > 0) {
      html += `<div class="positions-section">
        <h3 class="positions-section-title">Open Positions (${openPositions.length})</h3>
        <div class="positions-list">${openPositions.map((pos) => this._buildOpenPositionCard(pos)).join("")}</div>
      </div>`;
    }

    if (closedPositions.length > 0) {
      html += `<div class="positions-section">
        <h3 class="positions-section-title">Closed Positions (${closedPositions.length})</h3>
        <div class="positions-list">${closedPositions.map((pos) => this._buildClosedPositionCard(pos)).join("")}</div>
      </div>`;
    }

    html += "</div>";
    return html;
  }

  _buildOpenPositionCard(pos) {
    const pnlClass = (pos.unrealized_pnl || 0) >= 0 ? "positive" : "negative";
    const priceChange =
      pos.current_price && pos.entry_price
        ? (((pos.current_price - pos.entry_price) / pos.entry_price) * 100).toFixed(2)
        : null;
    const priceChangeClass = priceChange >= 0 ? "positive" : "negative";

    // Calculate token amount display
    const tokenAmount = pos.remaining_token_amount || pos.token_amount;
    const tokenAmountDisplay = tokenAmount
      ? Utils.formatNumber(tokenAmount / Math.pow(10, 9), { decimals: 4 })
      : "—";

    return `
      <div class="position-card open" data-position-id="${pos.id || pos.mint}">
        <div class="position-header">
          <div class="position-status-group">
            <span class="position-status open">OPEN</span>
            ${pos.dca_count > 0 ? `<span class="position-badge dca">DCA ×${pos.dca_count}</span>` : ""}
            ${pos.partial_exit_count > 0 ? `<span class="position-badge partial">Partial ×${pos.partial_exit_count}</span>` : ""}
            ${pos.transaction_entry_verified ? '<span class="position-badge verified">✓ Verified</span>' : ""}
          </div>
          <div class="position-actions">
            <button class="btn-position-add" data-mint="${pos.mint}" title="Add to position (DCA)">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 8v8M8 12h8"/></svg>
              Add
            </button>
            <button class="btn-position-sell" data-mint="${pos.mint}" title="Sell position">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 19V5M5 12l7-7 7 7"/></svg>
              Sell
            </button>
          </div>
        </div>

        <div class="position-pnl-banner ${pnlClass}">
          <div class="pnl-main">
            <span class="pnl-label">Unrealized P&L</span>
            <span class="pnl-value">${this._formatPnLWithPercent(pos.unrealized_pnl, pos.unrealized_pnl_percent)}</span>
          </div>
          ${priceChange !== null ? `<span class="price-change ${priceChangeClass}">${priceChange >= 0 ? "+" : ""}${priceChange}% from entry</span>` : ""}
        </div>

        <div class="position-body">
          <div class="position-grid">
            <div class="position-cell">
              <span class="cell-label">Entry Price</span>
              <span class="cell-value">${Utils.formatPriceSol(pos.average_entry_price || pos.entry_price, { decimals: 9 })} SOL</span>
              ${pos.dca_count > 0 && pos.entry_price !== pos.average_entry_price ? `<span class="cell-sub">Initial: ${Utils.formatPriceSol(pos.entry_price, { decimals: 9 })}</span>` : ""}
            </div>
            <div class="position-cell">
              <span class="cell-label">Current Price</span>
              <span class="cell-value">${pos.current_price ? Utils.formatPriceSol(pos.current_price, { decimals: 9 }) + " SOL" : "—"}</span>
              ${pos.current_price_updated ? `<span class="cell-sub">${Utils.formatTimestamp(pos.current_price_updated)}</span>` : ""}
            </div>
            <div class="position-cell">
              <span class="cell-label">Total Invested</span>
              <span class="cell-value">${Utils.formatSol(pos.total_size_sol)} SOL</span>
              ${pos.entry_size_sol !== pos.total_size_sol ? `<span class="cell-sub">Initial: ${Utils.formatSol(pos.entry_size_sol)} SOL</span>` : ""}
            </div>
            <div class="position-cell">
              <span class="cell-label">Token Amount</span>
              <span class="cell-value">${tokenAmountDisplay}</span>
              ${pos.partial_exit_count > 0 ? `<span class="cell-sub">Exited: ${Utils.formatNumber(pos.total_exited_amount / Math.pow(10, 9), { decimals: 4 })}</span>` : ""}
            </div>
          </div>

          <div class="position-details">
            <div class="detail-row">
              <span class="detail-label">Price Range</span>
              <span class="detail-value">
                <span class="low">${Utils.formatPriceSol(pos.price_lowest, { decimals: 9 })}</span>
                <span class="range-separator">—</span>
                <span class="high">${Utils.formatPriceSol(pos.price_highest, { decimals: 9 })}</span>
              </span>
            </div>
            <div class="detail-row">
              <span class="detail-label">Entry Time</span>
              <span class="detail-value">${Utils.formatTimestamp(pos.entry_time)} (${Utils.formatDuration(Date.now() / 1000 - pos.entry_time)} ago)</span>
            </div>
            ${
              pos.entry_fee_lamports
                ? `
            <div class="detail-row">
              <span class="detail-label">Entry Fee</span>
              <span class="detail-value">${Utils.formatSol(pos.entry_fee_lamports / 1e9)} SOL</span>
            </div>
            `
                : ""
            }
            ${
              pos.liquidity_tier
                ? `
            <div class="detail-row">
              <span class="detail-label">Liquidity Tier</span>
              <span class="detail-value tier-${pos.liquidity_tier.toLowerCase()}">${pos.liquidity_tier}</span>
            </div>
            `
                : ""
            }
          </div>
          ${this._buildPositionTransactionPair(pos)}
        </div>
      </div>
    `;
  }

  _buildClosedPositionCard(pos) {
    const pnlClass = (pos.pnl || 0) >= 0 ? "positive" : "negative";
    const duration = pos.exit_time && pos.entry_time ? pos.exit_time - pos.entry_time : 0;

    return `
      <div class="position-card closed" data-position-id="${pos.id || pos.mint}">
        <div class="position-header">
          <div class="position-status-group">
            <span class="position-status closed">CLOSED</span>
            ${pos.dca_count > 0 ? `<span class="position-badge dca">DCA ×${pos.dca_count}</span>` : ""}
            ${pos.partial_exit_count > 0 ? `<span class="position-badge partial">Partial ×${pos.partial_exit_count}</span>` : ""}
            ${pos.synthetic_exit ? '<span class="position-badge synthetic">Synthetic</span>' : ""}
            ${pos.closed_reason ? `<span class="position-badge reason">${pos.closed_reason}</span>` : ""}
          </div>
          <span class="position-time">${Utils.formatTimestamp(pos.exit_time)}</span>
        </div>

        <div class="position-pnl-banner ${pnlClass}">
          <div class="pnl-main">
            <span class="pnl-label">Realized P&L</span>
            <span class="pnl-value">${this._formatPnLWithPercent(pos.pnl, pos.pnl_percent)}</span>
          </div>
          ${pos.sol_received ? `<span class="sol-received">Received: ${Utils.formatSol(pos.sol_received)} SOL</span>` : ""}
        </div>

        <div class="position-body">
          <div class="position-grid">
            <div class="position-cell">
              <span class="cell-label">Entry Price</span>
              <span class="cell-value">${Utils.formatPriceSol(pos.average_entry_price || pos.entry_price, { decimals: 9 })} SOL</span>
            </div>
            <div class="position-cell">
              <span class="cell-label">Exit Price</span>
              <span class="cell-value">${Utils.formatPriceSol(pos.average_exit_price || pos.exit_price, { decimals: 9 })} SOL</span>
            </div>
            <div class="position-cell">
              <span class="cell-label">Total Invested</span>
              <span class="cell-value">${Utils.formatSol(pos.total_size_sol)} SOL</span>
            </div>
            <div class="position-cell">
              <span class="cell-label">Duration</span>
              <span class="cell-value">${Utils.formatDuration(duration)}</span>
            </div>
          </div>

          <div class="position-details collapsed">
            <div class="detail-row">
              <span class="detail-label">Price Range</span>
              <span class="detail-value">
                <span class="low">${Utils.formatPriceSol(pos.price_lowest, { decimals: 9 })}</span>
                <span class="range-separator">—</span>
                <span class="high">${Utils.formatPriceSol(pos.price_highest, { decimals: 9 })}</span>
              </span>
            </div>
            <div class="detail-row">
              <span class="detail-label">Entry Time</span>
              <span class="detail-value">${Utils.formatTimestamp(pos.entry_time)}</span>
            </div>
            ${
              pos.entry_fee_lamports || pos.exit_fee_lamports
                ? `
            <div class="detail-row">
              <span class="detail-label">Total Fees</span>
              <span class="detail-value">${Utils.formatSol(((pos.entry_fee_lamports || 0) + (pos.exit_fee_lamports || 0)) / 1e9)} SOL</span>
            </div>
            `
                : ""
            }
          </div>
          ${this._buildPositionTransactionPair(pos)}
        </div>
      </div>
    `;
  }

  /**
   * Build symmetric entry/exit transaction pair display for a position
   * @param {Object} pos - Position with entry/exit transaction signatures
   * @returns {string} HTML for transaction pair
   */
  _buildPositionTransactionPair(pos) {
    const hasEntry = !!pos.entry_transaction_signature;
    const hasExit = !!pos.exit_transaction_signature;

    if (!hasEntry && !hasExit) {
      return "";
    }

    const entryTx = hasEntry
      ? `
        <div class="tx-pair-item entry">
          <div class="tx-pair-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 5v14M5 12l7-7 7 7"/></svg>
          </div>
          <div class="tx-pair-content">
            <div class="tx-pair-label">Entry</div>
            <a href="https://solscan.io/tx/${pos.entry_transaction_signature}" target="_blank" rel="noopener" class="tx-pair-signature">${pos.entry_transaction_signature.slice(0, 6)}...${pos.entry_transaction_signature.slice(-6)}</a>
          </div>
          <button class="btn-copy-mini" data-copy="${pos.entry_transaction_signature}" title="Copy signature">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
          </button>
        </div>
      `
      : '<div class="tx-pair-item entry empty"><span class="tx-pair-empty">—</span></div>';

    const exitTx = hasExit
      ? `
        <div class="tx-pair-item exit">
          <div class="tx-pair-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 19V5M5 12l7 7 7-7"/></svg>
          </div>
          <div class="tx-pair-content">
            <div class="tx-pair-label">Exit</div>
            <a href="https://solscan.io/tx/${pos.exit_transaction_signature}" target="_blank" rel="noopener" class="tx-pair-signature">${pos.exit_transaction_signature.slice(0, 6)}...${pos.exit_transaction_signature.slice(-6)}</a>
          </div>
          <button class="btn-copy-mini" data-copy="${pos.exit_transaction_signature}" title="Copy signature">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
          </button>
        </div>
      `
      : '<div class="tx-pair-item exit empty"><span class="tx-pair-empty">—</span></div>';

    return `
      <div class="tx-pair-section">
        <div class="tx-pair-title">Transactions</div>
        <div class="tx-pair-grid">
          ${entryTx}
          ${exitTx}
        </div>
      </div>
    `;
  }

  /**
   * Build transaction history section with symmetric entry/exit display
   * @param {Object} position - Position data with entries/exits arrays
   * @returns {string} HTML for transaction history
   */
  _buildTransactionHistory(position) {
    const entries = position.entries || [];
    const exits = position.exits || [];

    // Combine and sort by timestamp
    const allTransactions = [
      ...entries.map((e, idx) => ({
        type: idx === 0 ? "entry" : "dca",
        timestamp: e.timestamp,
        price: e.price,
        amount: e.amount,
        sol: e.sol_spent,
        signature: e.transaction_signature,
        fees: e.fees_sol,
      })),
      ...exits.map((e) => ({
        type: "exit",
        timestamp: e.timestamp,
        price: e.price,
        amount: e.amount,
        sol: e.sol_received,
        signature: e.transaction_signature,
        fees: e.fees_sol,
        percentage: e.percentage,
      })),
    ].sort((a, b) => a.timestamp - b.timestamp);

    if (allTransactions.length === 0) {
      return "";
    }

    const transactionRows = allTransactions
      .map((tx) => {
        const isEntry = tx.type === "entry" || tx.type === "dca";
        const typeLabel = tx.type === "entry" ? "Buy" : tx.type === "dca" ? "DCA" : "Sell";
        const typeClass = tx.type;
        const icon = isEntry
          ? '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 5v14M5 12l7-7 7 7"/></svg>'
          : '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 19V5M5 12l7 7 7-7"/></svg>';

        const amountLabel = isEntry ? "Spent" : "Received";
        const solAmount = tx.sol ? Utils.formatSol(tx.sol) : "—";

        return `
        <div class="transaction-row ${typeClass}">
          <div class="transaction-icon">${icon}</div>
          <div class="transaction-type">
            <span class="transaction-type-label">${typeLabel}</span>
            <span class="transaction-type-time">${Utils.formatTimestamp(tx.timestamp * 1000)}</span>
          </div>
          <div class="transaction-price">
            <span class="transaction-price-label">Price</span>
            <span class="transaction-price-value">${tx.price ? Utils.formatPriceSol(tx.price, { decimals: 9 }) + " SOL" : "—"}</span>
          </div>
          <div class="transaction-amount">
            <span class="transaction-amount-label">${amountLabel}</span>
            <span class="transaction-amount-value">${solAmount} SOL</span>
          </div>
          <div class="transaction-sol">
            <span class="transaction-sol-label">Fees</span>
            <span class="transaction-sol-value">${tx.fees ? Utils.formatSol(tx.fees) + " SOL" : "—"}</span>
          </div>
          <div class="transaction-link">
            ${tx.signature ? `<a href="https://solscan.io/tx/${this._escapeHtml(tx.signature)}" target="_blank" rel="noopener" title="View on Solscan"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg></a>` : ""}
          </div>
        </div>
      `;
      })
      .join("");

    return `
      <div class="transaction-history">
        <div class="transaction-history-title">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/></svg>
          Transaction History
        </div>
        <div class="transaction-list">
          ${transactionRows}
        </div>
      </div>
    `;
  }

  // =========================================================================
  // POOLS TAB
  // =========================================================================

  _loadPoolsTab(content) {
    if (!this.fullTokenData) {
      content.innerHTML = '<div class="loading-spinner">Waiting for token data...</div>';
      return;
    }

    content.innerHTML = this._buildPoolsContent(this.fullTokenData);
    content.dataset.loaded = "true";
    this._attachPoolsCopyHandlers(content);
  }

  _attachPoolsCopyHandlers(content) {
    content.querySelectorAll(".copy-btn-mini[data-copy]").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        e.preventDefault();
        e.stopPropagation();
        const text = btn.dataset.copy;
        if (text) {
          Utils.copyToClipboard(text);
        }
      });
    });
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

    content.innerHTML = this._buildLinksContent(this.fullTokenData);
    content.dataset.loaded = "true";
    this._attachLinksCopyHandlers(content);
  }

  _attachLinksCopyHandlers(content) {
    content.querySelectorAll(".copy-btn-mini[data-copy]").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        e.preventDefault();
        e.stopPropagation();
        const text = btn.dataset.copy;
        if (text) {
          Utils.copyToClipboard(text);
        }
      });
    });
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
    if (typeof LightweightCharts === "undefined") {
      chartContainer.innerHTML = "Chart library missing";
      return;
    }

    const chart = LightweightCharts.createChart(chartContainer, {
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
