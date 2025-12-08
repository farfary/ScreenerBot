/**
 * Token Details Dialog
 * Full-screen dialog showing comprehensive token information with multiple tabs
 */
/* global LightweightCharts */
import * as Utils from "../core/utils.js";
import { Poller } from "../core/poller.js";
import { TradeActionDialog } from "./trade_action_dialog.js";

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
      this._createDialog();
      this._attachEventHandlers();
      this._loadTabContent("overview");

      requestAnimationFrame(() => {
        if (this.dialogEl) {
          this.dialogEl.classList.add("active");
        }
      });

      this._triggerTokenRefresh().catch((err) => {
        console.warn("Token refresh failed:", err);
      });

      this._triggerOhlcvRefresh().catch((err) => {
        // Silent - expected for new tokens
      });

      this._fetchTokenData().catch((err) => {
        console.error("Failed to fetch token data:", err);
      });

      this._startPolling();
    } finally {
      this.isOpening = false;
    }
  }

  async _triggerTokenRefresh() {
    try {
      const response = await fetch(`/api/tokens/${this.tokenData.mint}/refresh`, {
        method: "POST",
      });
      if (response.ok) {
        const result = await response.json();
        console.log("Token data refresh triggered:", result);
      }
    } catch (error) {
      console.warn("Failed to trigger token refresh:", error);
    }
  }

  async _triggerOhlcvRefresh() {
    try {
      const response = await fetch(`/api/tokens/${this.tokenData.mint}/ohlcv/refresh`, {
        method: "POST",
      });
      if (response.ok) {
        const result = await response.json();
        console.log("OHLCV data refresh triggered:", result);
      }
    } catch (error) {
      // Silently ignore - OHLCV may not be available for new tokens
    }
  }

  async _fetchTokenData() {
    if (this.isRefreshing) return;
    this.isRefreshing = true;

    try {
      const response = await fetch(`/api/tokens/${this.tokenData.mint}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch token details: ${response.statusText}`);
      }
      const newData = await response.json();
      const isInitialLoad = !this.fullTokenData;
      this.fullTokenData = newData;
      this._updateHeader(this.fullTokenData);

      if (isInitialLoad && this.currentTab === "overview") {
        this._loadTabContent(this.currentTab);
      } else if (!isInitialLoad && this.currentTab === "overview") {
        this._refreshOverviewTab();
      }
    } catch (error) {
      console.error("Error loading token details:", error);
      const headerMetrics = this.dialogEl?.querySelector(".header-metrics");
      if (headerMetrics) {
        headerMetrics.innerHTML = '<div class="error-text">Failed to load details</div>';
      }
    } finally {
      this.isRefreshing = false;
    }
  }

  _startPolling() {
    this._stopPolling();
    this.refreshPoller = new Poller(
      () => {
        this._fetchTokenData();
      },
      { label: "TokenRefresh", interval: 1000 }
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
    this.chartPoller = new Poller(
      () => {
        this._refreshChartData();
      },
      { label: "ChartRefresh", interval: 5000 }
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
    if (!this.candlestickSeries || !this.tokenData || this.currentTab !== "overview") {
      return;
    }

    try {
      const response = await fetch(
        `/api/tokens/${this.tokenData.mint}/ohlcv?timeframe=${this.currentTimeframe}&limit=200`
      );
      if (!response.ok) return;

      const data = await response.json();
      if (!Array.isArray(data) || data.length === 0) return;

      const chartData = data.map((candle) => ({
        time: candle.timestamp,
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
      }));

      this.candlestickSeries.setData(chartData);
    } catch (error) {
      // Silently fail
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
      }

      if (this.chartResizeObserver) {
        this.chartResizeObserver.disconnect();
        this.chartResizeObserver = null;
      }
      if (this.chart) {
        this.chart.remove();
        this.chart = null;
      }
      if (this.candlestickSeries) {
        this.candlestickSeries = null;
      }

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
          <div class="header-left">
            <div class="header-logo">
              ${logoUrl ? `<img src="${this._escapeHtml(logoUrl)}" alt="${this._escapeHtml(symbol)}" onerror="this.parentElement.innerHTML='<div class=\\'logo-placeholder\\'>${this._escapeHtml(symbol.charAt(0))}</div>'" />` : `<div class="logo-placeholder">${this._escapeHtml(symbol.charAt(0))}</div>`}
            </div>
            <div class="header-title">
              <div class="title-row">
                <span class="title-main">${this._escapeHtml(symbol)}</span>
                <div class="title-badges" id="headerBadges"></div>
              </div>
              <div class="title-sub">${this._escapeHtml(name)}</div>
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
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="9" cy="21" r="1"></circle><circle cx="20" cy="21" r="1"></circle><path d="M1 1h4l2.68 13.39a2 2 0 0 0 2 1.61h9.72a2 2 0 0 0 2-1.61L23 6H6"></path></svg>
                Buy
              </button>
              <button class="trade-btn sell-btn" id="headerSellBtn" title="Sell position" disabled>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="1" x2="12" y2="23"></line><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"></path></svg>
                Sell
              </button>
            </div>
            <div class="header-actions">
              <button class="action-btn" id="copyMintBtn" title="Copy Mint Address">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                </svg>
              </button>
              <a href="https://solscan.io/token/${this._escapeHtml(this.tokenData.mint)}" target="_blank" class="action-btn" title="View on Solscan">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
                  <polyline points="15 3 21 3 21 9"></polyline>
                  <line x1="10" y1="14" x2="21" y2="3"></line>
                </svg>
              </a>
            </div>
            <button class="dialog-close" type="button" title="Close (ESC)">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
              </svg>
            </button>
          </div>
        </div>

        <div class="dialog-tabs">
          <button class="tab-button active" data-tab="overview">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="16" x2="12" y2="12"></line><line x1="12" y1="8" x2="12.01" y2="8"></line></svg>
            Overview
          </button>
          <button class="tab-button" data-tab="security">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path></svg>
            Security
          </button>
          <button class="tab-button" data-tab="positions">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="20" x2="12" y2="10"></line><line x1="18" y1="20" x2="18" y2="4"></line><line x1="6" y1="20" x2="6" y2="16"></line></svg>
            Positions
          </button>
          <button class="tab-button" data-tab="pools">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2.69l5.66 5.66a8 8 0 1 1-11.31 0z"></path></svg>
            Pools
          </button>
          <button class="tab-button" data-tab="links">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path></svg>
            Links
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
        </div>
      </div>
    `;
  }

  _updateHeader(token) {
    // Update badges
    const badgesContainer = this.dialogEl.querySelector("#headerBadges");
    if (badgesContainer) {
      const badges = [];
      if (token.verified) badges.push('<span class="badge badge-success">Verified</span>');
      if (token.has_open_position) badges.push('<span class="badge badge-info">Position</span>');
      if (token.blacklisted) badges.push('<span class="badge badge-danger">Blacklisted</span>');
      badgesContainer.innerHTML = badges.join("");
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
    const priceSol = token.price_sol !== null && token.price_sol !== undefined
      ? Utils.formatPriceSol(token.price_sol, { decimals: 12 })
      : "—";
    const priceUsd = token.price_usd !== null && token.price_usd !== undefined
      ? Utils.formatCurrencyUSD(token.price_usd)
      : "—";

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
      <div class="price-main">
        <span class="price-sol">${priceSol} SOL</span>
        <span class="price-usd">${priceUsd}</span>
        ${changeHtml}
      </div>
      <div class="price-metrics">
        <div class="metric-item">
          <span class="metric-label">MCap</span>
          <span class="metric-value">${token.market_cap ? Utils.formatCurrencyUSD(token.market_cap) : "—"}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Liq</span>
          <span class="metric-value">${token.liquidity_usd ? Utils.formatCurrencyUSD(token.liquidity_usd) : "—"}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Vol 24H</span>
          <span class="metric-value">${token.volume_24h ? Utils.formatCurrencyUSD(token.volume_24h) : "—"}</span>
        </div>
        <div class="metric-item">
          <span class="metric-label">Holders</span>
          <span class="metric-value">${token.total_holders ? Utils.formatNumber(token.total_holders) : "—"}</span>
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

  async _handleBuyClick() {
    const dialog = this._ensureTradeDialog();
    const symbol = this.fullTokenData?.symbol || this.tokenData?.symbol || "?";

    try {
      const result = await dialog.open({
        action: "buy",
        symbol,
        context: {},
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

    try {
      const result = await dialog.open({
        action: "sell",
        symbol,
        context: {},
      });

      if (!result) return; // User cancelled

      const sellBtn = this.dialogEl.querySelector("#headerSellBtn");
      if (sellBtn) sellBtn.disabled = true;

      const body = result.percentage === 100
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

    if (tabId === "overview" && this.candlestickSeries) {
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
    }
  }

  // =========================================================================
  // OVERVIEW TAB
  // =========================================================================

  _loadOverviewTab(content) {
    if (!this.fullTokenData) {
      content.innerHTML = '<div class="loading-spinner">Waiting for token data...</div>';
      return;
    }

    content.innerHTML = this._buildOverviewHTML(this.fullTokenData);

    setTimeout(() => {
      this._initializeChart(this.fullTokenData.mint);
    }, 100);

    content.dataset.loaded = "true";
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
              <div class="chart-title">Price Chart (SOL)</div>
              <div class="chart-controls">
                <select class="chart-timeframe" id="chartTimeframe">
                  <option value="1m">1M</option>
                  <option value="5m" selected>5M</option>
                  <option value="15m">15M</option>
                  <option value="1h">1H</option>
                  <option value="4h">4H</option>
                  <option value="12h">12H</option>
                  <option value="1d">1D</option>
                </select>
              </div>
            </div>
            <div id="tradingview-chart" class="tradingview-chart"></div>
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

    // Build tags display
    const tagsHtml = token.tags && token.tags.length > 0
      ? `<div class="token-tags">${token.tags.map((t) => `<span class="token-tag">${this._escapeHtml(t)}</span>`).join("")}</div>`
      : "";

    return `
      <div class="info-card compact">
        <div class="card-header">
          <span>Token Info</span>
          ${token.verified ? '<span class="verified-badge">✓ Verified</span>' : ""}
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
            ${token.total_holders ? `
            <div class="info-cell">
              <span class="cell-label">Holders</span>
              <span class="cell-value">${Utils.formatNumber(token.total_holders, { decimals: 0 })}</span>
            </div>
            ` : ""}
            ${token.top_10_concentration ? `
            <div class="info-cell">
              <span class="cell-label">Top 10 Hold</span>
              <span class="cell-value">${token.top_10_concentration.toFixed(1)}%</span>
            </div>
            ` : ""}
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
        <div class="card-header">Liquidity & Market</div>
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
          ${token.pool_address ? `
          <div class="pool-address">
            <span class="cell-label">Pool</span>
            <a href="https://solscan.io/account/${token.pool_address}" target="_blank" rel="noopener" class="pool-link">${this._formatShortAddress(token.pool_address)}</a>
          </div>
          ` : ""}
        </div>
      </div>
    `;
  }

  _buildPriceChangesCard(token) {
    const changes = token.price_change_periods || {};
    return `
      <div class="info-card compact">
        <div class="card-header">Price Changes</div>
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
        <div class="card-header">Trading Volume</div>
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
    const ratioClass = buySellRatio ? (buySellRatio > 1 ? "bullish" : buySellRatio < 1 ? "bearish" : "neutral") : "";

    return `
      <div class="info-card compact">
        <div class="card-header">
          <span>Transaction Activity</span>
          ${buySellRatio ? `<span class="ratio-badge ${ratioClass}">${buySellRatio.toFixed(2)} B/S</span>` : ""}
        </div>
        <div class="card-body">
          <div class="txn-grid">
            ${this._buildTxnRow("5M", txns.m5)}
            ${this._buildTxnRow("1H", txns.h1)}
            ${this._buildTxnRow("6H", txns.h6)}
            ${this._buildTxnRow("24H", txns.h24)}
          </div>
          ${token.buys_24h !== undefined || token.sells_24h !== undefined ? `
          <div class="txn-summary">
            <div class="txn-summary-item buys">
              <span class="summary-icon">↗</span>
              <span class="summary-value">${token.buys_24h ?? 0}</span>
              <span class="summary-label">Buys</span>
            </div>
            <div class="txn-summary-item sells">
              <span class="summary-icon">↘</span>
              <span class="summary-value">${token.sells_24h ?? 0}</span>
              <span class="summary-label">Sells</span>
            </div>
            ${token.net_flow_24h !== undefined ? `
            <div class="txn-summary-item flow ${token.net_flow_24h >= 0 ? "positive" : "negative"}">
              <span class="summary-icon">${token.net_flow_24h >= 0 ? "+" : "−"}</span>
              <span class="summary-value">${Math.abs(token.net_flow_24h)}</span>
              <span class="summary-label">Net</span>
            </div>
            ` : ""}
          </div>
          ` : ""}
        </div>
      </div>
    `;
  }

  _buildTxnRow(label, data) {
    if (!data) return "";
    const buys = data.buys ?? 0;
    const sells = data.sells ?? 0;
    const total = buys + sells;
    const buyPct = total > 0 ? (buys / total * 100) : 50;
    return `
      <div class="txn-row">
        <span class="txn-label">${label}</span>
        <div class="txn-bar-container">
          <div class="txn-bar buy-bar" style="width: ${buyPct}%"></div>
          <div class="txn-bar sell-bar" style="width: ${100 - buyPct}%"></div>
        </div>
        <span class="txn-counts">
          <span class="buy-count">${buys}</span>
          <span class="separator">/</span>
          <span class="sell-count">${sells}</span>
        </span>
      </div>
    `;
  }

  // =========================================================================
  // SECURITY TAB
  // =========================================================================

  _loadSecurityTab(content) {
    if (!this.fullTokenData) {
      content.innerHTML = '<div class="loading-spinner">Waiting for token data...</div>';
      return;
    }

    content.innerHTML = this._buildSecurityContent(this.fullTokenData);
    content.dataset.loaded = "true";
  }

  _buildSecurityContent(token) {
    const scoreClass = this._getScoreClass(token.risk_score);
    const scoreLabel = this._getScoreLabel(token.risk_score);

    return `
      <div class="security-container">
        <div class="security-header">
          <div class="security-score ${scoreClass}">
            <span class="score-value">${token.risk_score ?? "—"}</span>
            <span class="score-label">${scoreLabel}</span>
          </div>
          <div class="security-summary">
            ${token.security_summary ? `<p>${this._escapeHtml(token.security_summary)}</p>` : ""}
          </div>
        </div>

        <div class="security-grid">
          <div class="security-card">
            <div class="card-header">Authorities</div>
            <div class="card-body">
              <div class="auth-row">
                <span class="auth-label">Mint Authority</span>
                ${this._renderAuthority(token.mint_authority)}
              </div>
              <div class="auth-row">
                <span class="auth-label">Freeze Authority</span>
                ${this._renderAuthority(token.freeze_authority)}
              </div>
            </div>
          </div>

          <div class="security-card">
            <div class="card-header">Distribution</div>
            <div class="card-body">
              <div class="info-row">
                <span class="info-label">Total Holders</span>
                <span class="info-value">${token.total_holders ? Utils.formatNumber(token.total_holders) : "—"}</span>
              </div>
              <div class="info-row">
                <span class="info-label">Top 10 Concentration</span>
                <span class="info-value">${token.top_10_concentration ? Utils.formatPercentValue(token.top_10_concentration) : "—"}</span>
              </div>
            </div>
          </div>
        </div>

        ${this._buildRisksSection(token.security_risks)}
      </div>
    `;
  }

  _renderAuthority(value) {
    if (value === null || value === undefined || value === "") {
      return '<span class="auth-status revoked"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="20 6 9 17 4 12"></polyline></svg> Revoked</span>';
    }
    return '<span class="auth-status present"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg> Present</span>';
  }

  _buildRisksSection(risks) {
    if (!risks || risks.length === 0) {
      return `
        <div class="security-card full-width">
          <div class="card-header">Security Risks</div>
          <div class="card-body">
            <div class="no-risks">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path><polyline points="9 12 12 15 16 10"></polyline></svg>
              <span>No security risks detected</span>
            </div>
          </div>
        </div>
      `;
    }

    const riskItems = risks.map((risk) => {
      const levelClass = risk.level?.toLowerCase() || "info";
      return `
        <div class="risk-item ${levelClass}">
          <div class="risk-header">
            <span class="risk-name">${this._escapeHtml(risk.name)}</span>
            <span class="risk-level ${levelClass}">${this._escapeHtml(risk.level || "Info")}</span>
          </div>
          <div class="risk-description">${this._escapeHtml(risk.description)}</div>
          ${risk.value ? `<div class="risk-value">${this._escapeHtml(risk.value)}</div>` : ""}
        </div>
      `;
    }).join("");

    return `
      <div class="security-card full-width">
        <div class="card-header">Security Risks (${risks.length})</div>
        <div class="card-body risks-list">
          ${riskItems}
        </div>
      </div>
    `;
  }

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

  // =========================================================================
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

    let html = "";

    // Open positions section
    if (openPositions.length > 0) {
      html += `<div class="positions-section">
        <h3 class="positions-section-title">Open Positions (${openPositions.length})</h3>
        <div class="positions-list">${openPositions.map((pos) => this._buildOpenPositionCard(pos)).join("")}</div>
      </div>`;
    }

    // Closed positions section
    if (closedPositions.length > 0) {
      html += `<div class="positions-section">
        <h3 class="positions-section-title">Closed Positions (${closedPositions.length})</h3>
        <div class="positions-list">${closedPositions.map((pos) => this._buildClosedPositionCard(pos)).join("")}</div>
      </div>`;
    }

    return html;
  }

  _buildOpenPositionCard(pos) {
    const pnlClass = (pos.unrealized_pnl || 0) >= 0 ? "positive" : "negative";
    const priceChange = pos.current_price && pos.entry_price ? ((pos.current_price - pos.entry_price) / pos.entry_price * 100).toFixed(2) : null;
    const priceChangeClass = priceChange >= 0 ? "positive" : "negative";

    // Calculate token amount display
    const tokenAmount = pos.remaining_token_amount || pos.token_amount;
    const tokenAmountDisplay = tokenAmount ? Utils.formatNumber(tokenAmount / Math.pow(10, 9), { decimals: 4 }) : "—";

    return `
      <div class="position-card open" data-position-id="${pos.id || pos.mint}">
        <div class="position-header">
          <div class="position-status-group">
            <span class="position-status open">OPEN</span>
            ${pos.dca_count > 0 ? `<span class="position-badge dca">DCA ×${pos.dca_count}</span>` : ""}
            ${pos.partial_exit_count > 0 ? `<span class="position-badge partial">Partial ×${pos.partial_exit_count}</span>` : ""}
            ${pos.transaction_entry_verified ? `<span class="position-badge verified">✓ Verified</span>` : ""}
          </div>
          <div class="position-actions">
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
            ${pos.entry_fee_lamports ? `
            <div class="detail-row">
              <span class="detail-label">Entry Fee</span>
              <span class="detail-value">${Utils.formatSol(pos.entry_fee_lamports / 1e9)} SOL</span>
            </div>
            ` : ""}
            ${pos.liquidity_tier ? `
            <div class="detail-row">
              <span class="detail-label">Liquidity Tier</span>
              <span class="detail-value tier-${pos.liquidity_tier.toLowerCase()}">${pos.liquidity_tier}</span>
            </div>
            ` : ""}
            ${pos.entry_transaction_signature ? `
            <div class="detail-row">
              <span class="detail-label">Entry TX</span>
              <span class="detail-value signature">
                <a href="https://solscan.io/tx/${pos.entry_transaction_signature}" target="_blank" rel="noopener">${pos.entry_transaction_signature.slice(0, 8)}...${pos.entry_transaction_signature.slice(-8)}</a>
                <button class="btn-copy-mini" data-copy="${pos.entry_transaction_signature}" title="Copy signature">📋</button>
              </span>
            </div>
            ` : ""}
          </div>
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
            ${pos.synthetic_exit ? `<span class="position-badge synthetic">Synthetic</span>` : ""}
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
            ${pos.entry_fee_lamports || pos.exit_fee_lamports ? `
            <div class="detail-row">
              <span class="detail-label">Total Fees</span>
              <span class="detail-value">${Utils.formatSol(((pos.entry_fee_lamports || 0) + (pos.exit_fee_lamports || 0)) / 1e9)} SOL</span>
            </div>
            ` : ""}
            ${pos.entry_transaction_signature ? `
            <div class="detail-row">
              <span class="detail-label">Entry TX</span>
              <span class="detail-value signature">
                <a href="https://solscan.io/tx/${pos.entry_transaction_signature}" target="_blank" rel="noopener">${pos.entry_transaction_signature.slice(0, 8)}...${pos.entry_transaction_signature.slice(-8)}</a>
              </span>
            </div>
            ` : ""}
            ${pos.exit_transaction_signature ? `
            <div class="detail-row">
              <span class="detail-label">Exit TX</span>
              <span class="detail-value signature">
                <a href="https://solscan.io/tx/${pos.exit_transaction_signature}" target="_blank" rel="noopener">${pos.exit_transaction_signature.slice(0, 8)}...${pos.exit_transaction_signature.slice(-8)}</a>
              </span>
            </div>
            ` : ""}
          </div>
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

    const poolCards = pools.map((pool, idx) => {
      return `
        <div class="pool-card ${pool.is_canonical ? "canonical" : ""}">
          <div class="pool-header">
            <span class="pool-program">${this._escapeHtml(pool.program)}</span>
            ${pool.is_canonical ? '<span class="canonical-badge">Canonical</span>' : ""}
          </div>
          <div class="pool-body">
            <div class="pool-row">
              <span class="pool-label">Pool Address</span>
              <span class="pool-value mono" title="${this._escapeHtml(pool.pool_id)}">${this._formatShortAddress(pool.pool_id)}</span>
            </div>
            <div class="pool-row">
              <span class="pool-label">Liquidity</span>
              <span class="pool-value">${pool.liquidity_usd ? Utils.formatCurrencyUSD(pool.liquidity_usd) : "—"}</span>
            </div>
            <div class="pool-row">
              <span class="pool-label">Volume 24H</span>
              <span class="pool-value">${pool.volume_h24_usd ? Utils.formatCurrencyUSD(pool.volume_h24_usd) : "—"}</span>
            </div>
            <div class="pool-row">
              <span class="pool-label">Token Role</span>
              <span class="pool-value">${this._escapeHtml(pool.token_role)}</span>
            </div>
          </div>
        </div>
      `;
    }).join("");

    return `<div class="pools-grid">${poolCards}</div>`;
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
  }

  _buildLinksContent(token) {
    const mint = token.mint;
    const hasWebsites = token.websites && token.websites.length > 0;
    const hasSocials = token.socials && token.socials.length > 0;

    // External explorer/analytics links - comprehensive list
    const explorerLinks = `
      <div class="links-section">
        <div class="section-title">Explorers & Analytics</div>
        <div class="links-grid">
          <a href="https://solscan.io/token/${mint}" target="_blank" rel="noopener noreferrer" class="link-card explorer">
            <div class="link-icon">🔍</div>
            <div class="link-name">Solscan</div>
          </a>
          <a href="https://explorer.solana.com/address/${mint}" target="_blank" rel="noopener noreferrer" class="link-card explorer">
            <div class="link-icon">◎</div>
            <div class="link-name">Solana Explorer</div>
          </a>
          <a href="https://birdeye.so/token/${mint}?chain=solana" target="_blank" rel="noopener noreferrer" class="link-card explorer">
            <div class="link-icon">🦅</div>
            <div class="link-name">Birdeye</div>
          </a>
          <a href="https://dexscreener.com/solana/${mint}" target="_blank" rel="noopener noreferrer" class="link-card explorer">
            <div class="link-icon">📊</div>
            <div class="link-name">DEX Screener</div>
          </a>
          <a href="https://www.geckoterminal.com/solana/tokens/${mint}" target="_blank" rel="noopener noreferrer" class="link-card explorer">
            <div class="link-icon">🦎</div>
            <div class="link-name">GeckoTerminal</div>
          </a>
          <a href="https://www.dextools.io/app/en/solana/pair-explorer/${mint}" target="_blank" rel="noopener noreferrer" class="link-card explorer">
            <div class="link-icon">🛠️</div>
            <div class="link-name">DexTools</div>
          </a>
          <a href="https://gmgn.ai/sol/token/${mint}" target="_blank" rel="noopener noreferrer" class="link-card explorer">
            <div class="link-icon">🤖</div>
            <div class="link-name">GMGN</div>
          </a>
          <a href="https://photon-sol.tinyastro.io/en/lp/${mint}" target="_blank" rel="noopener noreferrer" class="link-card explorer">
            <div class="link-icon">⚡</div>
            <div class="link-name">Photon</div>
          </a>
          <a href="https://rugcheck.xyz/tokens/${mint}" target="_blank" rel="noopener noreferrer" class="link-card explorer">
            <div class="link-icon">🛡️</div>
            <div class="link-name">RugCheck</div>
          </a>
          <a href="https://app.bubblemaps.io/sol/token/${mint}" target="_blank" rel="noopener noreferrer" class="link-card explorer">
            <div class="link-icon">🫧</div>
            <div class="link-name">Bubblemaps</div>
          </a>
        </div>
      </div>
    `;

    // Website links
    let websiteLinks = "";
    if (hasWebsites) {
      const websiteItems = token.websites.map((site) => `
        <a href="${this._escapeHtml(site.url)}" target="_blank" class="link-card website">
          <div class="link-icon">🌐</div>
          <div class="link-name">${this._escapeHtml(site.label || "Website")}</div>
        </a>
      `).join("");

      websiteLinks = `
        <div class="links-section">
          <div class="section-title">Websites</div>
          <div class="links-grid">${websiteItems}</div>
        </div>
      `;
    }

    // Social links
    let socialLinks = "";
    if (hasSocials) {
      const socialItems = token.socials.map((social) => {
        const icon = this._getSocialIcon(social.platform);
        return `
          <a href="${this._escapeHtml(social.url)}" target="_blank" class="link-card social">
            <div class="link-icon">${icon}</div>
            <div class="link-name">${this._escapeHtml(social.platform)}</div>
          </a>
        `;
      }).join("");

      socialLinks = `
        <div class="links-section">
          <div class="section-title">Socials</div>
          <div class="links-grid">${socialItems}</div>
        </div>
      `;
    }

    if (!hasWebsites && !hasSocials) {
      return explorerLinks + `
        <div class="empty-state small">
          <p>No official website or social links available for this token.</p>
        </div>
      `;
    }

    return explorerLinks + websiteLinks + socialLinks;
  }

  _getSocialIcon(platform) {
    const icons = {
      twitter: "𝕏",
      telegram: "✈️",
      discord: "💬",
      medium: "📝",
      github: "🐙",
      youtube: "▶️",
      reddit: "🔴",
    };
    return icons[platform?.toLowerCase()] || "🔗";
  }

  // =========================================================================
  // CHART
  // =========================================================================

  async _initializeChart(mint) {
    const chartContainer = this.dialogEl.querySelector("#tradingview-chart");
    const timeframeSelect = this.dialogEl.querySelector("#chartTimeframe");

    if (!chartContainer || !window.LightweightCharts) {
      console.error("Chart container or LightweightCharts library not found");
      return;
    }

    const chart = window.LightweightCharts.createChart(chartContainer, {
      layout: {
        background: { color: "#0d1117" },
        textColor: "#8b949e",
      },
      grid: {
        vertLines: { color: "#21262d" },
        horzLines: { color: "#21262d" },
      },
      crosshair: {
        mode: window.LightweightCharts.CrosshairMode.Normal,
      },
      rightPriceScale: {
        borderColor: "#30363d",
        scaleMargins: { top: 0.1, bottom: 0.2 },
      },
      localization: {
        priceFormatter: (price) => {
          if (price === 0) return "0";
          if (Math.abs(price) < 0.000001) return price.toExponential(6);
          const formatted = price.toFixed(12);
          return formatted.replace(/\.?0+$/, "");
        },
      },
      timeScale: {
        borderColor: "#30363d",
        timeVisible: true,
        secondsVisible: false,
        barSpacing: 12,
        minBarSpacing: 4,
      },
      width: chartContainer.clientWidth,
      height: chartContainer.clientHeight,
    });

    const candlestickSeries = chart.addCandlestickSeries({
      upColor: "#3fb950",
      downColor: "#f85149",
      borderVisible: false,
      wickUpColor: "#3fb950",
      wickDownColor: "#f85149",
      priceFormat: {
        type: "custom",
        formatter: (price) => {
          if (price === 0) return "0";
          if (Math.abs(price) < 0.000001) return price.toExponential(6);
          const formatted = price.toFixed(12);
          return formatted.replace(/\.?0+$/, "");
        },
      },
    });

    this.chart = chart;
    this.candlestickSeries = candlestickSeries;

    this.currentTimeframe = timeframeSelect.value;
    await this._loadChartData(mint, this.currentTimeframe);

    this._startChartPolling();

    timeframeSelect.addEventListener("change", async (e) => {
      this.currentTimeframe = e.target.value;
      await this._triggerOhlcvRefresh();
      await new Promise((resolve) => setTimeout(resolve, 500));
      await this._loadChartData(mint, this.currentTimeframe);
    });

    const resizeObserver = new ResizeObserver(() => {
      chart.applyOptions({
        width: chartContainer.clientWidth,
        height: chartContainer.clientHeight,
      });
    });
    resizeObserver.observe(chartContainer);
    this.chartResizeObserver = resizeObserver;
  }

  async _loadChartData(mint, timeframe) {
    try {
      const response = await fetch(`/api/tokens/${mint}/ohlcv?timeframe=${timeframe}`);
      if (!response.ok) return;

      const data = await response.json();
      if (!Array.isArray(data) || data.length === 0) return;

      const chartData = data.map((candle) => ({
        time: candle.timestamp,
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
      }));

      this.candlestickSeries.setData(chartData);

      if (chartData.length > 0) {
        const targetVisibleBars = 80;
        const lastIndex = chartData.length - 1;
        this.chart.timeScale().setVisibleLogicalRange({
          from: lastIndex - targetVisibleBars,
          to: lastIndex,
        });
      }
    } catch (error) {
      // Silently fail
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

  _escapeHtml(text) {
    if (!text) return "";
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }

  destroy() {
    this._stopPolling();
    this._stopChartPolling();

    if (this._escapeHandler) {
      document.removeEventListener("keydown", this._escapeHandler);
    }
    if (this.chartResizeObserver) {
      this.chartResizeObserver.disconnect();
    }
    if (this.chart) {
      this.chart.remove();
      this.chart = null;
    }
    if (this.dialogEl) {
      this.dialogEl.remove();
      this.dialogEl = null;
    }
    this.tabHandlers.clear();
  }
}
