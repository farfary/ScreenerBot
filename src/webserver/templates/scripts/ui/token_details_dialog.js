/**
 * Token Details Dialog
 * Full-screen dialog showing comprehensive token information with multiple tabs
 */
/* global LightweightCharts, ResizeObserver */
import * as Utils from "../core/utils.js";

export class TokenDetailsDialog {
  constructor(options = {}) {
    this.onClose = options.onClose || (() => {});
    this.dialogEl = null;
    this.currentTab = "overview";
    this.tokenData = null;
    this.tabHandlers = new Map();
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

    this.tokenData = tokenData;
    this._createDialog();
    this._attachEventHandlers();

    // Fetch full token details from API
    try {
      const response = await fetch(`/api/tokens/${tokenData.mint}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch token details: ${response.statusText}`);
      }
      this.fullTokenData = await response.json();

      // Update header with full data
      this._updateHeader(this.fullTokenData);

      // Load overview tab with full data
      this._loadTabContent(this.currentTab);
    } catch (error) {
      console.error("Error loading token details:", error);
      const headerMetrics = this.dialogEl.querySelector(".header-metrics");
      if (headerMetrics) {
        headerMetrics.innerHTML = `<div class="error-text">Failed to load details</div>`;
      }
    }

    // Animate in
    requestAnimationFrame(() => {
      this.dialogEl.classList.add("active");
    });
  }

  /**
   * Close dialog
   */
  close() {
    if (!this.dialogEl) return;

    this.dialogEl.classList.remove("active");
    setTimeout(() => {
      if (this.dialogEl) {
        this.dialogEl.remove();
        this.dialogEl = null;
      }
      this.onClose();
    }, 300);
  }

  /**
   * Create dialog DOM structure
   */
  _createDialog() {
    this.dialogEl = document.createElement("div");
    this.dialogEl.className = "token-details-dialog";
    this.dialogEl.innerHTML = this._getDialogHTML();
    document.body.appendChild(this.dialogEl);
  }

  /**
   * Generate dialog HTML structure
   */
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
              ${logoUrl ? `<img src="${this._escapeHtml(logoUrl)}" alt="${this._escapeHtml(symbol)}" />` : '<div class="logo-placeholder">?</div>'}
            </div>
            <div class="header-title">
              <div class="title-main">${this._escapeHtml(symbol)}</div>
              <div class="title-sub">${this._escapeHtml(name)}</div>
            </div>
          </div>
          <div class="header-metrics">
            <div class="loading-spinner-inline">Loading metrics...</div>
          </div>
          <button class="dialog-close" type="button" title="Close (ESC)">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>

        <div class="dialog-tabs">
          <button class="tab-button active" data-tab="overview">Overview</button>
          <button class="tab-button" data-tab="positions">Positions</button>
          <button class="tab-button" data-tab="pools">Pools</button>
          <button class="tab-button" data-tab="dexscreener">DexScreener</button>
          <button class="tab-button" data-tab="gmgn">GMGN</button>
          <button class="tab-button" data-tab="rugcheck">RugCheck</button>
        </div>

        <div class="dialog-body">
          <div class="tab-content active" data-tab-content="overview">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="positions">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="pools">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="dexscreener">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="gmgn">
            <div class="loading-spinner">Loading...</div>
          </div>
          <div class="tab-content" data-tab-content="rugcheck">
            <div class="loading-spinner">Loading...</div>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Update header with full token data
   */
  _updateHeader(token) {
    const metricsContainer = this.dialogEl.querySelector(".header-metrics");
    if (!metricsContainer) return;

    const metrics = [];

    // Liquidity
    if (token.liquidity_usd !== null && token.liquidity_usd !== undefined) {
      metrics.push(
        `<div class="metric"><div class="metric-label">Liquidity</div><div class="metric-value">${Utils.formatCurrencyUSD(token.liquidity_usd)}</div></div>`
      );
    }

    // Market Cap
    if (token.market_cap !== null && token.market_cap !== undefined) {
      metrics.push(
        `<div class="metric"><div class="metric-label">MCap</div><div class="metric-value">${Utils.formatCurrencyUSD(token.market_cap)}</div></div>`
      );
    }

    // Holders
    if (token.total_holders !== null && token.total_holders !== undefined) {
      metrics.push(
        `<div class="metric"><div class="metric-label">Holders</div><div class="metric-value">${Utils.formatNumber(token.total_holders)}</div></div>`
      );
    }

    // 24H Volume
    if (token.volume_24h !== null && token.volume_24h !== undefined) {
      metrics.push(
        `<div class="metric"><div class="metric-label">Vol 24H</div><div class="metric-value">${Utils.formatCurrencyUSD(token.volume_24h)}</div></div>`
      );
    }

    metricsContainer.innerHTML =
      metrics.length > 0
        ? metrics.join("")
        : '<div class="metric-empty">No metrics available</div>';
  }

  /**
   * Attach event handlers
   */
  _attachEventHandlers() {
    // Close button
    const closeBtn = this.dialogEl.querySelector(".dialog-close");
    closeBtn.addEventListener("click", () => this.close());

    // Backdrop click
    const backdrop = this.dialogEl.querySelector(".dialog-backdrop");
    backdrop.addEventListener("click", () => this.close());

    // ESC key
    this._escapeHandler = (e) => {
      if (e.key === "Escape") {
        this.close();
      }
    };
    document.addEventListener("keydown", this._escapeHandler);

    // Tab buttons
    const tabButtons = this.dialogEl.querySelectorAll(".tab-button");
    tabButtons.forEach((btn) => {
      btn.addEventListener("click", () => {
        const tabId = btn.dataset.tab;
        this._switchTab(tabId);
      });
    });
  }

  /**
   * Switch to different tab
   */
  _switchTab(tabId) {
    if (tabId === this.currentTab) return;

    // Update button states
    const tabButtons = this.dialogEl.querySelectorAll(".tab-button");
    tabButtons.forEach((btn) => {
      if (btn.dataset.tab === tabId) {
        btn.classList.add("active");
      } else {
        btn.classList.remove("active");
      }
    });

    // Update content visibility
    const tabContents = this.dialogEl.querySelectorAll(".tab-content");
    tabContents.forEach((content) => {
      if (content.dataset.tabContent === tabId) {
        content.classList.add("active");
      } else {
        content.classList.remove("active");
      }
    });

    this.currentTab = tabId;
    this._loadTabContent(tabId);
  }

  /**
   * Load content for specific tab
   */
  _loadTabContent(tabId) {
    const content = this.dialogEl.querySelector(`[data-tab-content="${tabId}"]`);
    if (!content) return;

    // Check if already loaded
    if (content.dataset.loaded === "true") return;

    // Load tab-specific content
    switch (tabId) {
      case "overview":
        this._loadOverviewTab(content);
        break;
      case "positions":
        this._loadPositionsTab(content);
        break;
      case "pools":
        this._loadPoolsTab(content);
        break;
      case "dexscreener":
        this._loadDexScreenerTab(content);
        break;
      case "gmgn":
        this._loadGmgnTab(content);
        break;
      case "rugcheck":
        this._loadRugCheckTab(content);
        break;
    }

    content.dataset.loaded = "true";
  }

  /**
   * Load Overview tab content
   */
  _loadOverviewTab(content) {
    if (!this.fullTokenData) {
      content.innerHTML = `<div class="loading-spinner">Waiting for token data...</div>`;
      return;
    }

    content.innerHTML = this._buildOverviewHTML(this.fullTokenData);

    // Initialize chart after DOM is ready
    setTimeout(() => {
      this._initializeChart(this.fullTokenData.mint);
    }, 100);
  }

  /**
   * Build comprehensive Overview tab HTML
   */
  _buildOverviewHTML(token) {
    return `
      <div class="overview-split-layout">
        <div class="overview-left">
          ${this._buildOverviewTable(token)}
        </div>
        <div class="overview-right">
          <div class="chart-container">
            <div class="chart-header">
              <div class="chart-title">Price Chart</div>
              <div class="chart-controls">
                <select class="chart-timeframe" id="chartTimeframe">
                  <option value="1m">1 Minute</option>
                  <option value="5m" selected>5 Minutes</option>
                  <option value="15m">15 Minutes</option>
                  <option value="1h">1 Hour</option>
                  <option value="4h">4 Hours</option>
                  <option value="12h">12 Hours</option>
                  <option value="1d">1 Day</option>
                </select>
              </div>
            </div>
            <div id="tradingview-chart" class="tradingview-chart"></div>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Build comprehensive overview data table
   */
  _buildOverviewTable(token) {
    const rows = [];

    // === BASIC INFORMATION ===
    rows.push(this._buildSectionHeader("Basic Information"));
    rows.push(this._buildDataRow("Mint Address", token.mint || "—", "mono"));
    rows.push(this._buildDataRow("Symbol", token.symbol || "—"));
    rows.push(this._buildDataRow("Name", token.name || "—"));
    if (token.decimals !== null && token.decimals !== undefined) {
      rows.push(this._buildDataRow("Decimals", token.decimals.toString()));
    }
    if (token.pair_created_at) {
      const birthDate = new Date(token.pair_created_at * 1000);
      rows.push(this._buildDataRow("Token Age", Utils.formatTimeAgo(birthDate)));
      rows.push(this._buildDataRow("Birth Date", birthDate.toLocaleString()));
    } else if (token.created_at) {
      const createdDate = new Date(token.created_at * 1000);
      rows.push(this._buildDataRow("First Seen", createdDate.toLocaleString()));
    }
    if (token.last_updated) {
      const updatedDate = new Date(token.last_updated * 1000);
      rows.push(this._buildDataRow("Last Updated", Utils.formatTimeAgo(updatedDate)));
    }

    // Status flags
    const badges = [];
    if (token.verified) badges.push(`<span class="badge-success">✓ Verified</span>`);
    if (token.has_open_position) badges.push(`<span class="badge-info">📊 Position Open</span>`);
    if (token.blacklisted) badges.push(`<span class="badge-danger">🚫 Blacklisted</span>`);
    if (token.has_ohlcv) badges.push(`<span class="badge-success">📈 Chart Data</span>`);
    if (badges.length > 0) {
      rows.push(this._buildDataRow("Status", badges.join(" ")));
    }

    // === PRICE DATA ===
    rows.push(this._buildSectionHeader("Price Data"));
    if (token.price_sol !== null && token.price_sol !== undefined) {
      rows.push(
        this._buildDataRow("Price (SOL)", Utils.formatPriceSol(token.price_sol, { decimals: 12 }))
      );
    }
    if (token.price_usd !== null && token.price_usd !== undefined) {
      rows.push(this._buildDataRow("Price (USD)", Utils.formatCurrencyUSD(token.price_usd)));
    }
    if (token.price_confidence) {
      rows.push(this._buildDataRow("Price Confidence", token.price_confidence));
    }
    if (token.price_updated_at) {
      const priceAge = Math.floor(Date.now() / 1000) - token.price_updated_at;
      rows.push(this._buildDataRow("Price Age", Utils.formatTimeAgo(priceAge)));
    }

    // Price changes
    if (token.price_change_periods) {
      if (token.price_change_periods.m5 !== null && token.price_change_periods.m5 !== undefined) {
        rows.push(
          this._buildDataRow(
            "Price Change (5M)",
            this._formatPercentChange(token.price_change_periods.m5)
          )
        );
      }
      if (token.price_change_periods.h1 !== null && token.price_change_periods.h1 !== undefined) {
        rows.push(
          this._buildDataRow(
            "Price Change (1H)",
            this._formatPercentChange(token.price_change_periods.h1)
          )
        );
      }
      if (token.price_change_periods.h6 !== null && token.price_change_periods.h6 !== undefined) {
        rows.push(
          this._buildDataRow(
            "Price Change (6H)",
            this._formatPercentChange(token.price_change_periods.h6)
          )
        );
      }
      if (token.price_change_periods.h24 !== null && token.price_change_periods.h24 !== undefined) {
        rows.push(
          this._buildDataRow(
            "Price Change (24H)",
            this._formatPercentChange(token.price_change_periods.h24)
          )
        );
      }
    }

    // === MARKET DATA ===
    rows.push(this._buildSectionHeader("Market Data"));
    if (token.market_cap !== null && token.market_cap !== undefined) {
      rows.push(this._buildDataRow("Market Cap", Utils.formatCurrencyUSD(token.market_cap)));
    }
    if (token.fdv !== null && token.fdv !== undefined) {
      rows.push(this._buildDataRow("FDV", Utils.formatCurrencyUSD(token.fdv)));
    }

    // === LIQUIDITY & VOLUME ===
    rows.push(this._buildSectionHeader("Liquidity & Volume"));
    if (token.liquidity_usd !== null && token.liquidity_usd !== undefined) {
      rows.push(
        this._buildDataRow("Liquidity (USD)", Utils.formatCurrencyUSD(token.liquidity_usd))
      );
    }
    if (token.pool_reserves_sol !== null && token.pool_reserves_sol !== undefined) {
      rows.push(
        this._buildDataRow(
          "Pool Reserves (SOL)",
          Utils.formatNumber(token.pool_reserves_sol, { decimals: 4 })
        )
      );
    }
    if (token.pool_reserves_token !== null && token.pool_reserves_token !== undefined) {
      rows.push(
        this._buildDataRow(
          "Pool Reserves (Token)",
          Utils.formatNumber(token.pool_reserves_token, { decimals: 2 })
        )
      );
    }

    // Volume periods
    if (token.volume_periods) {
      if (token.volume_periods.m5 !== null && token.volume_periods.m5 !== undefined) {
        rows.push(
          this._buildDataRow("Volume (5M)", Utils.formatCurrencyUSD(token.volume_periods.m5))
        );
      }
      if (token.volume_periods.h1 !== null && token.volume_periods.h1 !== undefined) {
        rows.push(
          this._buildDataRow("Volume (1H)", Utils.formatCurrencyUSD(token.volume_periods.h1))
        );
      }
      if (token.volume_periods.h6 !== null && token.volume_periods.h6 !== undefined) {
        rows.push(
          this._buildDataRow("Volume (6H)", Utils.formatCurrencyUSD(token.volume_periods.h6))
        );
      }
      if (token.volume_periods.h24 !== null && token.volume_periods.h24 !== undefined) {
        rows.push(
          this._buildDataRow("Volume (24H)", Utils.formatCurrencyUSD(token.volume_periods.h24))
        );
      }
    }

    // === POOL INFO ===
    if (token.pool_dex || token.pool_address) {
      rows.push(this._buildSectionHeader("Pool Information"));
      if (token.pool_dex) {
        rows.push(this._buildDataRow("DEX", token.pool_dex));
      }
      if (token.pool_address) {
        rows.push(this._buildDataRow("Pool Address", token.pool_address, "mono"));
      }
    }

    // === TRANSACTION ACTIVITY ===
    rows.push(this._buildSectionHeader("Transaction Activity"));

    // Transaction periods
    if (token.txn_periods) {
      if (token.txn_periods.m5) {
        rows.push(
          this._buildDataRow(
            "Txns (5M)",
            `${token.txn_periods.m5.buys || 0}B / ${token.txn_periods.m5.sells || 0}S`
          )
        );
      }
      if (token.txn_periods.h1) {
        rows.push(
          this._buildDataRow(
            "Txns (1H)",
            `${token.txn_periods.h1.buys || 0}B / ${token.txn_periods.h1.sells || 0}S`
          )
        );
      }
      if (token.txn_periods.h6) {
        rows.push(
          this._buildDataRow(
            "Txns (6H)",
            `${token.txn_periods.h6.buys || 0}B / ${token.txn_periods.h6.sells || 0}S`
          )
        );
      }
      if (token.txn_periods.h24) {
        rows.push(
          this._buildDataRow(
            "Txns (24H)",
            `${token.txn_periods.h24.buys || 0}B / ${token.txn_periods.h24.sells || 0}S`
          )
        );
      }
    }

    if (token.buys_24h !== null && token.buys_24h !== undefined) {
      rows.push(this._buildDataRow("Buys (24H)", token.buys_24h.toString()));
    }
    if (token.sells_24h !== null && token.sells_24h !== undefined) {
      rows.push(this._buildDataRow("Sells (24H)", token.sells_24h.toString()));
    }
    if (token.net_flow_24h !== null && token.net_flow_24h !== undefined) {
      const cls = token.net_flow_24h > 0 ? "positive" : token.net_flow_24h < 0 ? "negative" : "";
      rows.push(
        this._buildDataRow("Net Flow (24H)", `<span class="${cls}">${token.net_flow_24h}</span>`)
      );
    }
    if (token.buy_sell_ratio_24h !== null && token.buy_sell_ratio_24h !== undefined) {
      rows.push(this._buildDataRow("Buy/Sell Ratio", token.buy_sell_ratio_24h.toFixed(2)));
    }

    // === SECURITY & RISK ===
    rows.push(this._buildSectionHeader("Security & Risk"));
    if (token.risk_score !== null && token.risk_score !== undefined) {
      let badgeClass = "badge-success";
      if (token.risk_score < 300) badgeClass = "badge-danger";
      else if (token.risk_score < 500) badgeClass = "badge-warning";
      else if (token.risk_score < 700) badgeClass = "badge-info";
      rows.push(
        this._buildDataRow("Risk Score", `<span class="${badgeClass}">${token.risk_score}</span>`)
      );
    }
    if (token.mint_authority !== null && token.mint_authority !== undefined) {
      const value = token.mint_authority
        ? `<span class="badge-warning">Present</span>`
        : `<span class="badge-success">Revoked</span>`;
      rows.push(this._buildDataRow("Mint Authority", value));
    }
    if (token.freeze_authority !== null && token.freeze_authority !== undefined) {
      const value = token.freeze_authority
        ? `<span class="badge-warning">Present</span>`
        : `<span class="badge-success">Revoked</span>`;
      rows.push(this._buildDataRow("Freeze Authority", value));
    }
    if (token.total_holders !== null && token.total_holders !== undefined) {
      rows.push(this._buildDataRow("Total Holders", Utils.formatNumber(token.total_holders)));
    }
    if (token.top_10_concentration !== null && token.top_10_concentration !== undefined) {
      rows.push(
        this._buildDataRow(
          "Top 10 Concentration",
          Utils.formatPercentValue(token.top_10_concentration)
        )
      );
    }
    if (token.rugged) {
      rows.push(
        this._buildDataRow("Status", `<span class="badge-danger">⚠️ Flagged as Rugged</span>`)
      );
    }
    if (token.security_summary) {
      rows.push(this._buildDataRow("Security Summary", this._escapeHtml(token.security_summary)));
    }
    if (token.security_risks && token.security_risks.length > 0) {
      const risks = token.security_risks
        .map((r) => this._escapeHtml(r.name || r.description || "Unknown"))
        .join(", ");
      rows.push(this._buildDataRow("Security Risks", risks));
    }

    // === DESCRIPTION ===
    if (token.description) {
      rows.push(this._buildSectionHeader("Description"));
      rows.push(
        `<tr><td colspan="2" class="data-description">${this._escapeHtml(token.description)}</td></tr>`
      );
    }

    // === LINKS ===
    if (
      (token.websites && token.websites.length > 0) ||
      (token.socials && token.socials.length > 0)
    ) {
      rows.push(this._buildSectionHeader("Links"));

      if (token.websites && token.websites.length > 0) {
        const links = token.websites
          .map(
            (site) =>
              `<a href="${this._escapeHtml(site.url)}" target="_blank" rel="noopener noreferrer" class="data-link">🌐 ${this._escapeHtml(site.label || "Website")}</a>`
          )
          .join(" ");
        rows.push(this._buildDataRow("Websites", links));
      }

      if (token.socials && token.socials.length > 0) {
        const links = token.socials
          .map((social) => {
            const icon = this._getSocialIcon(social.platform);
            return `<a href="${this._escapeHtml(social.url)}" target="_blank" rel="noopener noreferrer" class="data-link">${icon} ${this._escapeHtml(social.platform)}</a>`;
          })
          .join(" ");
        rows.push(this._buildDataRow("Socials", links));
      }
    }

    // === POOLS ===
    if (token.pools && token.pools.length > 0) {
      rows.push(this._buildSectionHeader(`Liquidity Pools (${token.pools.length})`));
      token.pools.forEach((pool, idx) => {
        rows.push(
          this._buildDataRow(
            `Pool #${idx + 1}`,
            `<span class="mono">${pool.pool_id}</span> - ${pool.program}${pool.is_canonical ? " <span class='badge-info'>Canonical</span>" : ""}`
          )
        );
        if (pool.liquidity_usd !== null && pool.liquidity_usd !== undefined) {
          rows.push(this._buildDataRow(`  Liquidity`, Utils.formatCurrencyUSD(pool.liquidity_usd)));
        }
        if (pool.volume_h24_usd !== null && pool.volume_h24_usd !== undefined) {
          rows.push(
            this._buildDataRow(`  Volume 24H`, Utils.formatCurrencyUSD(pool.volume_h24_usd))
          );
        }
      });
    }

    return `<table class="overview-table">${rows.join("")}</table>`;
  }

  /**
   * Build section header row
   */
  _buildSectionHeader(title) {
    return `<tr class="section-header"><td colspan="2">${title}</td></tr>`;
  }

  /**
   * Build data row
   */
  _buildDataRow(label, value, valueClass = "") {
    const cls = valueClass ? ` class="${valueClass}"` : "";
    return `<tr class="data-row"><td class="data-label">${label}</td><td class="data-value"${cls}>${value}</td></tr>`;
  }

  /**
   * Format mint address (shortened with copy button)
   */
  _formatMintAddress(address) {
    if (!address || address.length < 16) return address || "—";
    const short = `${address.substring(0, 8)}...${address.substring(address.length - 8)}`;
    return `<span title="${this._escapeHtml(address)}">${short}</span>`;
  }

  /**
   * Format short address
   */
  _formatShortAddress(address) {
    if (!address || address.length < 16) return address || "—";
    return `${address.substring(0, 6)}...${address.substring(address.length - 6)}`;
  }

  /**
   * Format percent change with color
   */
  _formatPercentChange(value) {
    if (value === null || value === undefined || !Number.isFinite(value)) return "—";
    const cls = value > 0 ? "positive" : value < 0 ? "negative" : "";
    return `<span class="${cls}">${Utils.formatPercentValue(value, { includeSign: true })}</span>`;
  }

  /**
   * Get social platform icon
   */
  _getSocialIcon(platform) {
    const icons = {
      twitter: "🐦",
      telegram: "✈️",
      discord: "💬",
      website: "🌐",
    };
    return icons[platform.toLowerCase()] || "🔗";
  }

  /**
   * Initialize TradingView chart
   */
  async _initializeChart(mint) {
    const chartContainer = this.dialogEl.querySelector("#tradingview-chart");
    const timeframeSelect = this.dialogEl.querySelector("#chartTimeframe");

    if (!chartContainer || !window.LightweightCharts) {
      console.error("Chart container or LightweightCharts library not found");
      return;
    }

    // Create chart
    const chart = window.LightweightCharts.createChart(chartContainer, {
      layout: {
        background: { color: "#1a1a1a" },
        textColor: "#d1d4dc",
      },
      grid: {
        vertLines: { color: "#2b2b2b" },
        horzLines: { color: "#2b2b2b" },
      },
      crosshair: {
        mode: window.LightweightCharts.CrosshairMode.Normal,
      },
      rightPriceScale: {
        borderColor: "#2b2b2b",
        scaleMargins: {
          top: 0.1,
          bottom: 0.2,
        },
      },
      localization: {
        priceFormatter: (price) => {
          // Format SOL price with 12 decimals
          if (price === 0) return "0";
          // For very small prices, use scientific notation
          if (Math.abs(price) < 0.000001) {
            return price.toExponential(6);
          }
          // For normal small prices, show 12 decimals, trim trailing zeros
          const formatted = price.toFixed(12);
          return formatted.replace(/\.?0+$/, "");
        },
      },
      timeScale: {
        borderColor: "#2b2b2b",
        timeVisible: true,
        secondsVisible: false,
        barSpacing: 12, // Fixed spacing for consistent candle width across timeframes
        minBarSpacing: 4,
      },
      width: chartContainer.clientWidth,
      height: chartContainer.clientHeight,
    });

    // Create candlestick series
    const candlestickSeries = chart.addCandlestickSeries({
      upColor: "#26a69a",
      downColor: "#ef5350",
      borderVisible: false,
      wickUpColor: "#26a69a",
      wickDownColor: "#ef5350",
      priceFormat: {
        type: "custom",
        formatter: (price) => {
          // Format SOL price with 12 decimals
          if (price === 0) return "0";
          // For very small prices, use scientific notation
          if (Math.abs(price) < 0.000001) {
            return price.toExponential(6);
          }
          // For normal small prices, show 12 decimals, trim trailing zeros
          const formatted = price.toFixed(12);
          return formatted.replace(/\.?0+$/, "");
        },
      },
    });

    // Store chart instance for cleanup
    this.chart = chart;
    this.candlestickSeries = candlestickSeries;

    // Load initial data
    await this._loadChartData(mint, timeframeSelect.value);

    // Handle timeframe changes
    timeframeSelect.addEventListener("change", async (e) => {
      await this._loadChartData(mint, e.target.value);
    });

    // Handle resize
    const resizeObserver = new ResizeObserver(() => {
      chart.applyOptions({
        width: chartContainer.clientWidth,
        height: chartContainer.clientHeight,
      });
    });
    resizeObserver.observe(chartContainer);
    this.chartResizeObserver = resizeObserver;
  }

  /**
   * Load chart data from API
   */
  async _loadChartData(mint, timeframe) {
    try {
      const response = await fetch(`/api/tokens/${mint}/ohlcv?timeframe=${timeframe}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch OHLCV data: ${response.statusText}`);
      }

      const data = await response.json();

      // Backend returns flat array of OhlcvPoint objects
      if (!Array.isArray(data) || data.length === 0) {
        console.warn("No OHLCV data available for this token");
        return;
      }

      // Convert data to LightweightCharts format
      const chartData = data.map((candle) => ({
        time: candle.timestamp,
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
      }));

      // Update chart
      this.candlestickSeries.setData(chartData);

      // Use logical range to maintain consistent bar spacing across timeframes
      // Always show 80 bars worth of space, even if we have fewer data points
      if (chartData.length > 0) {
        const targetVisibleBars = 80;
        const lastIndex = chartData.length - 1;
        
        // Set logical range: show from (lastIndex - 80) to lastIndex
        // This maintains consistent spacing regardless of actual data count
        this.chart.timeScale().setVisibleLogicalRange({
          from: lastIndex - targetVisibleBars,
          to: lastIndex,
        });
      }
    } catch (error) {
      console.error("Error loading chart data:", error);
    }
  }

  /**
   * Load Positions tab content
   */
  _loadPositionsTab(content) {
    content.innerHTML = `<div class="tab-placeholder">Positions content will be loaded here</div>`;
  }

  /**
   * Load Pools tab content
   */
  _loadPoolsTab(content) {
    content.innerHTML = `<div class="tab-placeholder">Pools content will be loaded here</div>`;
  }

  /**
   * Load DexScreener tab content
   */
  _loadDexScreenerTab(content) {
    content.innerHTML = `<div class="tab-placeholder">DexScreener iframe will be loaded here</div>`;
  }

  /**
   * Load GMGN tab content
   */
  _loadGmgnTab(content) {
    content.innerHTML = `<div class="tab-placeholder">GMGN iframe will be loaded here</div>`;
  }

  /**
   * Load RugCheck tab content
   */
  _loadRugCheckTab(content) {
    content.innerHTML = `<div class="tab-placeholder">RugCheck content will be loaded here</div>`;
  }

  /**
   * Escape HTML
   */
  _escapeHtml(text) {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }

  /**
   * Destroy dialog and cleanup
   */
  destroy() {
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
