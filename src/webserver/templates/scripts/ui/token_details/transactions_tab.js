/**
 * Token Details Dialog - Transactions Tab Mixin
 * Extracted from token_details_dialog.js to reduce file size
 * Handles transaction history display and chart
 */
import * as Utils from "../../core/utils.js";
import { requestManager } from "../../core/request_manager.js";

/**
 * Apply transactions tab mixin to TokenDetailsDialog class
 * @param {class} DialogClass - TokenDetailsDialog class
 */
export function applyTransactionsTabMixin(DialogClass) {
  const proto = DialogClass.prototype;

  /**
   * Load transactions tab content
   * @private
   * @param {HTMLElement} content - Tab content container
   */
  proto._loadTransactionsTab = async function (content) {
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
  };

  /**
   * Build transactions tab HTML structure
   * @private
   * @returns {string} HTML string
   */
  proto._buildTransactionsHTML = function () {
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
  };

  /**
   * Render transactions chart (hourly histogram)
   * @private
   * @param {Array} transactions - Transaction data
   */
  proto._renderTransactionsChart = function (transactions) {
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
  };

  /**
   * Render transactions list table
   * @private
   * @param {Array} transactions - Transaction data
   */
  proto._renderTransactionsList = function (transactions) {
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
        const price = tx.price_sol ? Utils.formatPriceSubscript(tx.price_sol, { precision: 5 }) : "—";

        // Total SOL (use sol_delta absolute value)
        const amount = tx.amount_sol !== undefined ? tx.amount_sol : Math.abs(tx.sol_delta || 0);
        const total = Utils.formatNumber(amount, { decimals: 2 });

        const rowInner = `
           <span style="color: var(--text-secondary);">${timeDisplay}</span>
           <span style="font-weight: 600; ${typeStyle}">${typeLabel}</span>
           <span style="font-family: monospace;">${price}</span>
           <span>${total} SOL</span>
           <span style="color: var(--text-muted); text-align: right;"><i class="icon-external-link"></i></span>`;

        // Whole row links to the transaction on Solscan (clearer than a tiny icon).
        return tx.signature
          ? `
         <a class="txn-row" href="${Utils.solscanTxUrl(tx.signature)}" target="_blank" rel="noopener noreferrer" title="View transaction on Solscan" style="display: grid; grid-template-columns: 1fr 0.8fr 1.2fr 1fr 0.5fr; gap: 8px; padding: 8px 12px; border-bottom: 1px solid var(--border-color); font-size: 13px; color: inherit; text-decoration: none;">
           ${rowInner}
         </a>
       `
          : `
         <div style="display: grid; grid-template-columns: 1fr 0.8fr 1.2fr 1fr 0.5fr; gap: 8px; padding: 8px 12px; border-bottom: 1px solid var(--border-color); font-size: 13px;">
           ${rowInner}
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
  };

  /**
   * Build explorer link HTML
   * @private
   * @param {string} url - Explorer URL
   * @param {string} name - Display name
   * @returns {string} HTML string
   */
  proto._buildExplorerLink = function (url, name) {
    return `
      <a href="${this._escapeHtml(url)}" target="_blank" rel="noopener noreferrer" class="links-explorer-item">
        <span>${this._escapeHtml(name)}</span>
        <i class="icon-external-link link-external-icon"></i>
      </a>
    `;
  };

  /**
   * Build official link HTML
   * @private
   * @param {string} url - Link URL
   * @param {string} label - Display label
   * @returns {string} HTML string
   */
  proto._buildOfficialLink = function (url, label) {
    return `
      <a href="${this._escapeHtml(url)}" target="_blank" rel="noopener noreferrer" class="links-official-item">
        <div class="links-official-content">
          <span class="links-official-label">${this._escapeHtml(label)}</span>
          <span class="links-official-url">${this._escapeHtml(this._formatUrl(url))}</span>
        </div>
        <i class="icon-external-link link-external-icon"></i>
      </a>
    `;
  };

  /**
   * Build social link HTML
   * @private
   * @param {string} url - Social link URL
   * @param {string} label - Platform label
   * @returns {string} HTML string
   */
  proto._buildSocialLink = function (url, label) {
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
  };

  /**
   * Get social platform metadata
   * @private
   * @param {string} platform - Platform name
   * @returns {Object} Icon and label
   */
  proto._getSocialMeta = function (platform) {
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
  };

  /**
   * Get social platform color class
   * @private
   * @param {string} platform - Platform name
   * @returns {string} CSS class
   */
  proto._getSocialColorClass = function (platform) {
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
  };

  /**
   * Extract domain name from URL
   * @private
   * @param {string} url - URL to parse
   * @returns {string|null} Domain name
   */
  proto._extractDomainName = function (url) {
    try {
      const domain = new URL(url).hostname;
      return domain.replace(/^www\./, "");
    } catch {
      return null;
    }
  };

  /**
   * Format URL for display
   * @private
   * @param {string} url - URL to format
   * @returns {string} Formatted URL
   */
  proto._formatUrl = function (url) {
    try {
      const parsed = new URL(url);
      return parsed.hostname + (parsed.pathname !== "/" ? parsed.pathname : "");
    } catch {
      return url;
    }
  };

  /**
   * Extract username from social URL
   * @private
   * @param {string} url - Social URL
   * @returns {string|null} Username with @ prefix
   */
  proto._extractSocialUsername = function (url) {
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
  };
}
