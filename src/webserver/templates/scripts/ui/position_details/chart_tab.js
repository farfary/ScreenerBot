/**
 * Chart Tab Mixin for Position Details Dialog
 * Adds chart tab rendering functionality with OHLCV candlestick charts
 */
import * as Utils from "../../core/utils.js";
import { requestManager } from "../../core/request_manager.js";

/**
 * Apply chart tab methods to PositionDetailsDialog prototype
 * @param {class} PositionDetailsDialog - The PositionDetailsDialog class
 */
export function applyChartTabMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

  // ===========================================================================
  // CHART TAB
  // ===========================================================================

  proto._renderChartTab = async function (content) {
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
  };

  /**
   * Attach event handlers for timeframe buttons
   */
  proto._attachChartEventHandlers = function (content, mint, entries, exits) {
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
  };

  /**
   * Fetch OHLCV data and render chart
   */
  proto._fetchAndRenderChart = async function (mint, entries, exits) {
    const chartArea = this.dialogEl?.querySelector("#pddChartArea");
    const statsArea = this.dialogEl?.querySelector("#pddChartStats");
    if (!chartArea) return;

    // Only show loading state if we don't have cached chart data
    // This prevents the annoying flash when switching back to chart tab
    const hasCachedData = this._chartData && this._chartData.length > 0;
    if (!hasCachedData) {
      chartArea.innerHTML = '<div class="loading-spinner">Loading chart data...</div>';
    }

    try {
      const data = await requestManager.fetch(
        `/api/ohlcv/${mint}?timeframe=${this._chartTimeframe}&limit=200`,
        { priority: "normal" }
      );

      if (!data?.data || data.data.length === 0) {
        chartArea.innerHTML = `
          <div class="pdd-chart-empty">
            <i class="icon-chart-bar"></i>
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
          <i class="icon-circle-alert"></i>
          <p>Failed to load chart data</p>
          <span class="pdd-chart-empty-hint">${Utils.escapeHtml(error?.message || "Unknown error")}</span>
        </div>
      `;
      if (statsArea) statsArea.innerHTML = "";
    }
  };

  /**
   * Render candlestick chart as SVG with entry/exit markers
   */
  proto._renderChartSVG = function (container, candles, entries, exits) {
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
  };

  /**
   * Render chart statistics below chart
   */
  proto._renderChartStats = function (container, candles) {
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
  };
}
