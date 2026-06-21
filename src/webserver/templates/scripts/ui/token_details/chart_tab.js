/**
 * Token Details Dialog - Chart Tab Mixin
 * Extracted from token_details_dialog.js to reduce file size
 * Handles price chart rendering with lightweight-charts
 */
import * as Utils from "../../core/utils.js";
import { requestManager } from "../../core/request_manager.js";

/**
 * Apply chart tab mixin to TokenDetailsDialog class
 * @param {class} DialogClass - TokenDetailsDialog class
 */
export function applyChartTabMixin(DialogClass) {
  const proto = DialogClass.prototype;

  /**
   * Initialize chart on overview tab
   * @private
   * @param {string} mint - Token mint address
   */
  proto._initializeChart = async function (mint) {
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

    // Guard against duplicate initialization. The overview tab can be (re)loaded
    // more than once in quick succession — it renders with partial row data on
    // open, then again the moment the immediate detail fetch resolves. Recreating
    // the chart would orphan the previous instance (leak + duplicate pollers), so
    // if one already exists just refresh its data for the current timeframe.
    if (this.advancedChart) {
      await this._loadChartData(mint, this.currentTimeframe, false);
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
  };

  /**
   * Update chart position markers
   * @private
   */
  proto._updateChartPositions = function () {
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
  };

  /**
   * Load chart OHLCV data
   * @private
   * @param {string} mint - Token mint address
   * @param {string} timeframe - Timeframe (5m, 15m, etc.)
   * @param {boolean} isInitialLoad - Whether this is first load
   */
  proto._loadChartData = async function (mint, timeframe, isInitialLoad = false) {
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
      // Clear any "no data" backoff so a timeframe that does have candles polls
      // at the normal cadence again.
      this._chartEmptyCount = 0;
      this._chartPollBackedOff = false;

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
    } catch {
      // On error, show waiting message
      if (loadingText) {
        loadingText.textContent = "Waiting for chart data...";
      }
      if (loadingOverlay) {
        loadingOverlay.classList.remove("hidden");
      }
      this.chartDataLoaded = false;
    }
  };

  /**
   * Update OHLCV display with latest candle data
   * @private
   * @param {Array} chartData - Chart data array
   */
  proto._updateOhlcvDisplay = function (chartData) {
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
  };
}
