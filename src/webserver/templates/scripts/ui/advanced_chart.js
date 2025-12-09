/**
 * AdvancedChart - A comprehensive charting component for ScreenerBot
 *
 * Features:
 * - Candlestick, Line, Area, Bar charts
 * - Multiple indicators (RSI, MACD, EMA, SMA, Bollinger Bands, Volume)
 * - Position markers (entry/exit points, DCA levels)
 * - Price formatting with subscript notation for tiny prices
 * - Overlay annotations (text, icons, horizontal lines)
 * - Zoom/pan with mouse wheel, trackpad, and touch gestures
 * - Light/dark theme support
 * - Live data updates with smooth transitions
 * - Multi-chart comparison mode
 * - Responsive sizing with ResizeObserver
 * - Crosshair with synchronized tooltip
 *
 * Dependencies: lightweight-charts (TradingView)
 */

(function () {
  "use strict";

  // ==========================================================================
  // CONSTANTS & DEFAULTS
  // ==========================================================================

  const CHART_THEMES = {
    dark: {
      background: "#0d1117",
      textColor: "#8b949e",
      gridColor: "#21262d",
      borderColor: "#30363d",
      crosshairColor: "#58a6ff",
      upColor: "#3fb950",
      downColor: "#f85149",
      volumeUpColor: "rgba(63, 185, 80, 0.3)",
      volumeDownColor: "rgba(248, 81, 73, 0.3)",
      wickUpColor: "#3fb950",
      wickDownColor: "#f85149",
      overlayBackground: "rgba(13, 17, 23, 0.9)",
      tooltipBackground: "#161b22",
      tooltipBorder: "#30363d",
      indicatorColors: {
        ema9: "#f59e0b",
        ema21: "#8b5cf6",
        sma50: "#06b6d4",
        sma200: "#ec4899",
        rsi: "#58a6ff",
        macdLine: "#3fb950",
        macdSignal: "#f85149",
        macdHistogramUp: "rgba(63, 185, 80, 0.5)",
        macdHistogramDown: "rgba(248, 81, 73, 0.5)",
        bollingerUpper: "rgba(88, 166, 255, 0.5)",
        bollingerLower: "rgba(88, 166, 255, 0.5)",
        bollingerMiddle: "#58a6ff",
      },
      positionColors: {
        entry: "#3fb950",
        exit: "#f85149",
        dca: "#f59e0b",
        stopLoss: "#ef4444",
        takeProfit: "#10b981",
      },
    },
    light: {
      background: "#ffffff",
      textColor: "#374151",
      gridColor: "#e5e7eb",
      borderColor: "#d1d5db",
      crosshairColor: "#1565c0",
      upColor: "#10b981",
      downColor: "#ef4444",
      volumeUpColor: "rgba(16, 185, 129, 0.3)",
      volumeDownColor: "rgba(239, 68, 68, 0.3)",
      wickUpColor: "#10b981",
      wickDownColor: "#ef4444",
      overlayBackground: "rgba(255, 255, 255, 0.95)",
      tooltipBackground: "#ffffff",
      tooltipBorder: "#d1d5db",
      indicatorColors: {
        ema9: "#d97706",
        ema21: "#7c3aed",
        sma50: "#0891b2",
        sma200: "#db2777",
        rsi: "#1565c0",
        macdLine: "#059669",
        macdSignal: "#dc2626",
        macdHistogramUp: "rgba(16, 185, 129, 0.5)",
        macdHistogramDown: "rgba(239, 68, 68, 0.5)",
        bollingerUpper: "rgba(21, 101, 192, 0.4)",
        bollingerLower: "rgba(21, 101, 192, 0.4)",
        bollingerMiddle: "#1565c0",
      },
      positionColors: {
        entry: "#059669",
        exit: "#dc2626",
        dca: "#d97706",
        stopLoss: "#dc2626",
        takeProfit: "#059669",
      },
    },
  };

  const DEFAULT_OPTIONS = {
    theme: "dark",
    chartType: "candlestick", // candlestick, line, area, bar
    showVolume: true,
    showGrid: true,
    showCrosshair: true,
    showLegend: true,
    showTooltip: true,
    showTimeScale: true,
    showPriceScale: true,
    priceScalePosition: "right", // left, right, both
    autoScale: true,
    barSpacing: 12,
    minBarSpacing: 4,
    rightOffset: 5,
    indicators: [], // ['ema9', 'ema21', 'sma50', 'rsi', 'macd', 'bollinger']
    overlays: [], // Array of { type, value, label, color, style }
    positions: [], // Array of { type, price, timestamp, label }
    comparisonData: [], // Array of { symbol, data, color }
    timeframe: "5m",
    locale: "en-US",
    priceFormat: "auto", // auto, fixed, scientific, subscript
    pricePrecision: 9,
    volumePrecision: 2,
    animateData: true,
    watermark: null, // { text, fontSize, color }
    height: null, // null for auto
    minHeight: 300,
  };

  const TIMEFRAMES = {
    "1m": { label: "1M", seconds: 60 },
    "5m": { label: "5M", seconds: 300 },
    "15m": { label: "15M", seconds: 900 },
    "30m": { label: "30M", seconds: 1800 },
    "1h": { label: "1H", seconds: 3600 },
    "4h": { label: "4H", seconds: 14400 },
    "12h": { label: "12H", seconds: 43200 },
    "1d": { label: "1D", seconds: 86400 },
  };

  // ==========================================================================
  // PRICE FORMATTING UTILITIES
  // ==========================================================================

  /**
   * Format price with intelligent notation for very small numbers
   * Uses subscript notation: 0.0₉12345 means 0.000000000012345 (9 zeros after decimal)
   */
  function formatPriceSubscript(price, precision = 9) {
    if (price === 0) return "0";
    if (!Number.isFinite(price)) return "—";

    const absPrice = Math.abs(price);
    const sign = price < 0 ? "-" : "";

    // Handle normal-sized numbers (>= 0.0001)
    if (absPrice >= 0.0001) {
      const formatted = absPrice.toFixed(precision);
      return sign + formatted.replace(/\.?0+$/, "");
    }

    // Count leading zeros after decimal
    const str = absPrice.toFixed(20);
    const match = str.match(/^0\.0*/);
    if (!match) return sign + absPrice.toPrecision(precision);

    const leadingZeros = match[0].length - 2; // Subtract "0."

    // Get significant digits after zeros
    const significantPart = str.substring(match[0].length);
    const significant = significantPart.substring(0, Math.min(5, significantPart.length));

    // Use subscript for zero count
    const subscriptDigits = "₀₁₂₃₄₅₆₇₈₉";
    let subscript = "";
    const zeroStr = leadingZeros.toString();
    for (const char of zeroStr) {
      subscript += subscriptDigits[parseInt(char, 10)];
    }

    return `${sign}0.0${subscript}${significant}`;
  }

  /**
   * Format price based on magnitude for chart display
   */
  function formatPriceAuto(price, precision = 9) {
    if (price === 0) return "0";
    if (!Number.isFinite(price)) return "—";

    const absPrice = Math.abs(price);

    // Very small prices - use subscript
    if (absPrice < 0.000001) {
      return formatPriceSubscript(price, precision);
    }

    // Small prices - use scientific notation
    if (absPrice < 0.0001) {
      return price.toExponential(4);
    }

    // Normal prices
    if (absPrice < 1) {
      const formatted = price.toFixed(precision);
      return formatted.replace(/\.?0+$/, "");
    }

    // Larger prices
    if (absPrice < 1000) {
      return price.toFixed(Math.min(4, precision));
    }

    // Very large prices
    return price.toLocaleString("en-US", {
      maximumFractionDigits: 2,
    });
  }

  // ==========================================================================
  // INDICATOR CALCULATIONS
  // ==========================================================================

  const Indicators = {
    /**
     * Calculate Simple Moving Average
     */
    sma(data, period) {
      const result = [];
      for (let i = 0; i < data.length; i++) {
        if (i < period - 1) {
          result.push({ time: data[i].time, value: null });
          continue;
        }
        let sum = 0;
        for (let j = 0; j < period; j++) {
          sum += data[i - j].close;
        }
        result.push({ time: data[i].time, value: sum / period });
      }
      return result;
    },

    /**
     * Calculate Exponential Moving Average
     */
    ema(data, period) {
      const result = [];
      const multiplier = 2 / (period + 1);
      let ema = null;

      for (let i = 0; i < data.length; i++) {
        if (i < period - 1) {
          result.push({ time: data[i].time, value: null });
          continue;
        }

        if (ema === null) {
          // Initialize with SMA
          let sum = 0;
          for (let j = 0; j < period; j++) {
            sum += data[i - j].close;
          }
          ema = sum / period;
        } else {
          ema = (data[i].close - ema) * multiplier + ema;
        }

        result.push({ time: data[i].time, value: ema });
      }
      return result;
    },

    /**
     * Calculate Relative Strength Index
     */
    rsi(data, period = 14) {
      const result = [];
      const gains = [];
      const losses = [];

      for (let i = 0; i < data.length; i++) {
        if (i === 0) {
          result.push({ time: data[i].time, value: null });
          continue;
        }

        const change = data[i].close - data[i - 1].close;
        gains.push(change > 0 ? change : 0);
        losses.push(change < 0 ? Math.abs(change) : 0);

        if (i < period) {
          result.push({ time: data[i].time, value: null });
          continue;
        }

        const avgGain =
          gains.slice(-period).reduce((a, b) => a + b, 0) / period;
        const avgLoss =
          losses.slice(-period).reduce((a, b) => a + b, 0) / period;

        const rs = avgLoss === 0 ? 100 : avgGain / avgLoss;
        const rsi = 100 - 100 / (1 + rs);

        result.push({ time: data[i].time, value: rsi });
      }
      return result;
    },

    /**
     * Calculate MACD (Moving Average Convergence Divergence)
     */
    macd(data, fastPeriod = 12, slowPeriod = 26, signalPeriod = 9) {
      const fastEMA = this.ema(data, fastPeriod);
      const slowEMA = this.ema(data, slowPeriod);

      const macdLine = [];
      for (let i = 0; i < data.length; i++) {
        const fast = fastEMA[i].value;
        const slow = slowEMA[i].value;
        macdLine.push({
          time: data[i].time,
          value: fast !== null && slow !== null ? fast - slow : null,
          close: fast !== null && slow !== null ? fast - slow : 0,
        });
      }

      const signalLine = this.ema(macdLine, signalPeriod);
      const histogram = [];

      for (let i = 0; i < data.length; i++) {
        const macd = macdLine[i].value;
        const signal = signalLine[i].value;
        histogram.push({
          time: data[i].time,
          value: macd !== null && signal !== null ? macd - signal : null,
        });
      }

      return { macdLine, signalLine, histogram };
    },

    /**
     * Calculate Bollinger Bands
     */
    bollinger(data, period = 20, stdDev = 2) {
      const sma = this.sma(data, period);
      const upper = [];
      const lower = [];

      for (let i = 0; i < data.length; i++) {
        if (sma[i].value === null) {
          upper.push({ time: data[i].time, value: null });
          lower.push({ time: data[i].time, value: null });
          continue;
        }

        // Calculate standard deviation
        let sumSquared = 0;
        for (let j = 0; j < period; j++) {
          const diff = data[i - j].close - sma[i].value;
          sumSquared += diff * diff;
        }
        const std = Math.sqrt(sumSquared / period);

        upper.push({ time: data[i].time, value: sma[i].value + stdDev * std });
        lower.push({ time: data[i].time, value: sma[i].value - stdDev * std });
      }

      return { upper, middle: sma, lower };
    },
  };

  // ==========================================================================
  // ADVANCED CHART CLASS
  // ==========================================================================

  class AdvancedChart {
    constructor(container, options = {}) {
      this.container =
        typeof container === "string"
          ? document.querySelector(container)
          : container;

      if (!this.container) {
        throw new Error("AdvancedChart: Container element not found");
      }

      this.options = { ...DEFAULT_OPTIONS, ...options };
      this.theme = CHART_THEMES[this.options.theme] || CHART_THEMES.dark;

      // Internal state
      this.chart = null;
      this.mainSeries = null;
      this.volumeSeries = null;
      this.indicatorPanes = [];

      // User interaction tracking - respects user zoom/pan actions
      this._userHasInteracted = false;
      this._isFirstDataLoad = true;
      this._lastVisibleRange = null;
      this._interactionTimeout = null;
      this.indicatorSeries = {};
      this.overlaySeries = [];
      this.positionMarkers = [];
      this.comparisonSeries = [];
      this.data = [];
      this.volumeData = [];

      // UI elements
      this.tooltipEl = null;
      this.legendEl = null;
      this.controlsEl = null;

      // Observers
      this.resizeObserver = null;

      // Callbacks
      this.onCrosshairMove = null;
      this.onTimeRangeChange = null;
      this.onDataUpdate = null;

      this._init();
    }

    // ========================================================================
    // INITIALIZATION
    // ========================================================================

    _init() {
      this._createWrapper();
      this._createChart();
      this._createMainSeries();

      if (this.options.showVolume) {
        this._createVolumeSeries();
      }

      if (this.options.showLegend) {
        this._createLegend();
      }

      if (this.options.showTooltip) {
        this._createTooltip();
      }

      this._setupEventHandlers();
      this._setupResizeObserver();
    }

    _createWrapper() {
      // Wrap container content
      this.wrapper = document.createElement("div");
      this.wrapper.className = "advanced-chart-wrapper";
      this.container.appendChild(this.wrapper);

      // Chart area
      this.chartArea = document.createElement("div");
      this.chartArea.className = "advanced-chart-area";
      this.wrapper.appendChild(this.chartArea);
    }

    _createChart() {
      if (!window.LightweightCharts) {
        console.error("AdvancedChart: LightweightCharts library not loaded");
        return;
      }

      // Use chartArea dimensions - it uses flex: 1 to fill available space
      const width = this.chartArea.clientWidth || this.container.clientWidth || 400;
      const height = this.chartArea.clientHeight || this.container.clientHeight || 300;

      this.chart = window.LightweightCharts.createChart(this.chartArea, {
        width: width,
        height: height,
        autoSize: true,
        layout: {
          background: { color: this.theme.background },
          textColor: this.theme.textColor,
          fontFamily: "'JetBrains Mono', monospace",
          fontSize: 11,
        },
        grid: {
          vertLines: {
            color: this.options.showGrid ? this.theme.gridColor : "transparent",
          },
          horzLines: {
            color: this.options.showGrid ? this.theme.gridColor : "transparent",
          },
        },
        crosshair: {
          mode: this.options.showCrosshair
            ? window.LightweightCharts.CrosshairMode.Normal
            : window.LightweightCharts.CrosshairMode.Hidden,
          vertLine: {
            color: this.theme.crosshairColor,
            width: 1,
            style: 2,
            labelBackgroundColor: this.theme.tooltipBackground,
          },
          horzLine: {
            color: this.theme.crosshairColor,
            width: 1,
            style: 2,
            labelBackgroundColor: this.theme.tooltipBackground,
          },
        },
        rightPriceScale: {
          visible:
            this.options.showPriceScale &&
            ["right", "both"].includes(this.options.priceScalePosition),
          borderColor: this.theme.borderColor,
          scaleMargins: { top: 0.1, bottom: 0.2 },
        },
        leftPriceScale: {
          visible:
            this.options.showPriceScale &&
            ["left", "both"].includes(this.options.priceScalePosition),
          borderColor: this.theme.borderColor,
          scaleMargins: { top: 0.1, bottom: 0.2 },
        },
        timeScale: {
          visible: this.options.showTimeScale,
          borderColor: this.theme.borderColor,
          timeVisible: true,
          secondsVisible: false,
          barSpacing: this.options.barSpacing,
          minBarSpacing: this.options.minBarSpacing,
          rightOffset: this.options.rightOffset,
        },
        localization: {
          priceFormatter: (price) => this._formatPrice(price),
          locale: this.options.locale,
        },
        handleScroll: {
          mouseWheel: true,
          pressedMouseMove: true,
          horzTouchDrag: true,
          vertTouchDrag: true,
        },
        handleScale: {
          mouseWheel: true,
          pinch: true,
          axisPressedMouseMove: { time: true, price: true },
        },
      });

      // Add watermark if specified
      if (this.options.watermark) {
        this.chart.applyOptions({
          watermark: {
            visible: true,
            text: this.options.watermark.text,
            fontSize: this.options.watermark.fontSize || 48,
            color: this.options.watermark.color || "rgba(128, 128, 128, 0.15)",
            horzAlign: "center",
            vertAlign: "center",
          },
        });
      }
    }

    _createMainSeries() {
      const priceFormatOptions = {
        type: "custom",
        formatter: (price) => this._formatPrice(price),
        minMove: 0.000000001,
      };

      switch (this.options.chartType) {
        case "line":
          this.mainSeries = this.chart.addLineSeries({
            color: this.theme.upColor,
            lineWidth: 2,
            priceFormat: priceFormatOptions,
          });
          break;

        case "area":
          this.mainSeries = this.chart.addAreaSeries({
            topColor: `${this.theme.upColor}40`,
            bottomColor: `${this.theme.upColor}05`,
            lineColor: this.theme.upColor,
            lineWidth: 2,
            priceFormat: priceFormatOptions,
          });
          break;

        case "bar":
          this.mainSeries = this.chart.addBarSeries({
            upColor: this.theme.upColor,
            downColor: this.theme.downColor,
            priceFormat: priceFormatOptions,
          });
          break;

        case "candlestick":
        default:
          this.mainSeries = this.chart.addCandlestickSeries({
            upColor: this.theme.upColor,
            downColor: this.theme.downColor,
            borderVisible: false,
            wickUpColor: this.theme.wickUpColor,
            wickDownColor: this.theme.wickDownColor,
            priceFormat: priceFormatOptions,
          });
          break;
      }
    }

    _createVolumeSeries() {
      this.volumeSeries = this.chart.addHistogramSeries({
        priceFormat: {
          type: "volume",
        },
        priceScaleId: "volume",
      });

      this.chart.priceScale("volume").applyOptions({
        scaleMargins: {
          top: 0.85,
          bottom: 0,
        },
      });
    }

    _createLegend() {
      this.legendEl = document.createElement("div");
      this.legendEl.className = "advanced-chart-legend";
      this.wrapper.insertBefore(this.legendEl, this.chartArea);
      this._updateLegend();
    }

    _createTooltip() {
      this.tooltipEl = document.createElement("div");
      this.tooltipEl.className = "advanced-chart-tooltip";
      this.tooltipEl.style.display = "none";
      this.wrapper.appendChild(this.tooltipEl);
    }

    // ========================================================================
    // DATA MANAGEMENT
    // ========================================================================

    /**
     * Set chart data
     * @param {Array} data - Array of OHLCV objects { time, open, high, low, close, volume }
     */
    setData(data) {
      if (!data || !Array.isArray(data) || data.length === 0) {
        console.warn("AdvancedChart: No data provided");
        return;
      }

      // Normalize and sort data
      this.data = data
        .map((d) => ({
          time: typeof d.time === "number" ? d.time : d.timestamp,
          open: d.open,
          high: d.high,
          low: d.low,
          close: d.close,
          volume: d.volume || 0,
        }))
        .sort((a, b) => a.time - b.time);

      // Set main series data
      if (this.options.chartType === "line" || this.options.chartType === "area") {
        this.mainSeries.setData(
          this.data.map((d) => ({ time: d.time, value: d.close }))
        );
      } else {
        this.mainSeries.setData(this.data);
      }

      // Set volume data
      if (this.volumeSeries) {
        this.volumeData = this.data.map((d) => ({
          time: d.time,
          value: d.volume,
          color: d.close >= d.open ? this.theme.volumeUpColor : this.theme.volumeDownColor,
        }));
        this.volumeSeries.setData(this.volumeData);
      }

      // Update indicators
      this._updateIndicators();

      // Update legend
      this._updateLegend();

      // Only fit content on first load or if user hasn't interacted
      if (this._isFirstDataLoad) {
        this.fitContent();
        this._isFirstDataLoad = false;
      } else if (!this._userHasInteracted && this._lastVisibleRange) {
        // User hasn't interacted - scroll to show latest data while maintaining zoom level
        this._scrollToLatestPreserveZoom();
      }
      // If user HAS interacted, don't touch their view at all

      // Callback
      if (this.onDataUpdate) {
        this.onDataUpdate(this.data);
      }
    }

    /**
     * Update with new data point (for live updates)
     * @param {Object} point - OHLCV data point
     */
    updateData(point) {
      if (!point) return;

      const normalizedPoint = {
        time: typeof point.time === "number" ? point.time : point.timestamp,
        open: point.open,
        high: point.high,
        low: point.low,
        close: point.close,
        volume: point.volume || 0,
      };

      // Update or add to data array
      const existingIndex = this.data.findIndex(
        (d) => d.time === normalizedPoint.time
      );
      if (existingIndex >= 0) {
        this.data[existingIndex] = normalizedPoint;
      } else {
        this.data.push(normalizedPoint);
        this.data.sort((a, b) => a.time - b.time);
      }

      // Update main series
      if (this.options.chartType === "line" || this.options.chartType === "area") {
        this.mainSeries.update({
          time: normalizedPoint.time,
          value: normalizedPoint.close,
        });
      } else {
        this.mainSeries.update(normalizedPoint);
      }

      // Update volume
      if (this.volumeSeries) {
        this.volumeSeries.update({
          time: normalizedPoint.time,
          value: normalizedPoint.volume,
          color:
            normalizedPoint.close >= normalizedPoint.open
              ? this.theme.volumeUpColor
              : this.theme.volumeDownColor,
        });
      }

      // Update indicators with latest data
      this._updateIndicators();
      this._updateLegend();
    }

    // ========================================================================
    // INDICATORS
    // ========================================================================

    /**
     * Add indicator to chart
     * @param {string} type - Indicator type (ema9, ema21, sma50, sma200, rsi, macd, bollinger)
     * @param {Object} options - Indicator-specific options
     */
    addIndicator(type, options = {}) {
      if (!this.data.length) {
        console.warn("AdvancedChart: No data for indicator calculation");
        return;
      }

      const indicatorColors = this.theme.indicatorColors;

      switch (type) {
        case "ema9":
          this._addOverlayIndicator("ema9", Indicators.ema(this.data, 9), {
            color: indicatorColors.ema9,
            lineWidth: 1,
            ...options,
          });
          break;

        case "ema21":
          this._addOverlayIndicator("ema21", Indicators.ema(this.data, 21), {
            color: indicatorColors.ema21,
            lineWidth: 1,
            ...options,
          });
          break;

        case "sma50":
          this._addOverlayIndicator("sma50", Indicators.sma(this.data, 50), {
            color: indicatorColors.sma50,
            lineWidth: 1,
            ...options,
          });
          break;

        case "sma200":
          this._addOverlayIndicator("sma200", Indicators.sma(this.data, 200), {
            color: indicatorColors.sma200,
            lineWidth: 1,
            ...options,
          });
          break;

        case "rsi":
          this._addSeparatePaneIndicator("rsi", Indicators.rsi(this.data, 14), {
            color: indicatorColors.rsi,
            lineWidth: 1,
            paneHeight: 100,
            levels: [30, 70],
            ...options,
          });
          break;

        case "macd": {
          const macdData = Indicators.macd(this.data);
          this._addMACDIndicator(macdData, options);
          break;
        }

        case "bollinger": {
          const bollingerData = Indicators.bollinger(this.data, 20, 2);
          this._addBollingerIndicator(bollingerData, options);
          break;
        }

        default:
          console.warn(`AdvancedChart: Unknown indicator type: ${type}`);
      }

      // Track indicator
      if (!this.options.indicators.includes(type)) {
        this.options.indicators.push(type);
      }

      this._updateLegend();
    }

    /**
     * Remove indicator from chart
     * @param {string} type - Indicator type to remove
     */
    removeIndicator(type) {
      if (this.indicatorSeries[type]) {
        if (this.chart) {
          if (Array.isArray(this.indicatorSeries[type])) {
            this.indicatorSeries[type].forEach((s) => this.chart.removeSeries(s));
          } else {
            this.chart.removeSeries(this.indicatorSeries[type]);
          }
        }
        delete this.indicatorSeries[type];
      }

      const idx = this.options.indicators.indexOf(type);
      if (idx >= 0) {
        this.options.indicators.splice(idx, 1);
      }

      this._updateLegend();
    }

    _addOverlayIndicator(name, data, options) {
      const series = this.chart.addLineSeries({
        color: options.color,
        lineWidth: options.lineWidth || 1,
        priceLineVisible: false,
        lastValueVisible: false,
        crosshairMarkerVisible: false,
      });

      series.setData(data.filter((d) => d.value !== null));
      this.indicatorSeries[name] = series;
    }

    _addSeparatePaneIndicator(name, data, options) {
      // For now, add as overlay - separate panes require different approach
      const series = this.chart.addLineSeries({
        color: options.color,
        lineWidth: options.lineWidth || 1,
        priceLineVisible: false,
        lastValueVisible: true,
        crosshairMarkerVisible: false,
        priceScaleId: name,
      });

      this.chart.priceScale(name).applyOptions({
        scaleMargins: {
          top: 0.8,
          bottom: 0.05,
        },
        autoScale: true,
      });

      series.setData(data.filter((d) => d.value !== null));
      this.indicatorSeries[name] = series;

      // Add level lines
      if (options.levels) {
        options.levels.forEach((level) => {
          series.createPriceLine({
            price: level,
            color: this.theme.gridColor,
            lineWidth: 1,
            lineStyle: 2, // Dashed
            axisLabelVisible: false,
          });
        });
      }
    }

    _addMACDIndicator(macdData, options) {
      const colors = this.theme.indicatorColors;

      // MACD Line
      const macdLine = this.chart.addLineSeries({
        color: colors.macdLine,
        lineWidth: 1,
        priceLineVisible: false,
        lastValueVisible: true,
        crosshairMarkerVisible: false,
        priceScaleId: "macd",
      });

      // Signal Line
      const signalLine = this.chart.addLineSeries({
        color: colors.macdSignal,
        lineWidth: 1,
        priceLineVisible: false,
        lastValueVisible: true,
        crosshairMarkerVisible: false,
        priceScaleId: "macd",
      });

      // Histogram
      const histogram = this.chart.addHistogramSeries({
        priceScaleId: "macd",
        priceLineVisible: false,
        lastValueVisible: false,
      });

      this.chart.priceScale("macd").applyOptions({
        scaleMargins: {
          top: 0.85,
          bottom: 0.0,
        },
      });

      macdLine.setData(
        macdData.macdLine.filter((d) => d.value !== null)
      );
      signalLine.setData(
        macdData.signalLine.filter((d) => d.value !== null)
      );
      histogram.setData(
        macdData.histogram
          .filter((d) => d.value !== null)
          .map((d) => ({
            time: d.time,
            value: d.value,
            color:
              d.value >= 0
                ? colors.macdHistogramUp
                : colors.macdHistogramDown,
          }))
      );

      this.indicatorSeries.macd = [macdLine, signalLine, histogram];
    }

    _addBollingerIndicator(bollingerData, options) {
      const colors = this.theme.indicatorColors;

      // Upper band
      const upperBand = this.chart.addLineSeries({
        color: colors.bollingerUpper,
        lineWidth: 1,
        priceLineVisible: false,
        lastValueVisible: false,
        crosshairMarkerVisible: false,
      });

      // Middle band (SMA)
      const middleBand = this.chart.addLineSeries({
        color: colors.bollingerMiddle,
        lineWidth: 1,
        priceLineVisible: false,
        lastValueVisible: false,
        crosshairMarkerVisible: false,
      });

      // Lower band
      const lowerBand = this.chart.addLineSeries({
        color: colors.bollingerLower,
        lineWidth: 1,
        priceLineVisible: false,
        lastValueVisible: false,
        crosshairMarkerVisible: false,
      });

      upperBand.setData(bollingerData.upper.filter((d) => d.value !== null));
      middleBand.setData(bollingerData.middle.filter((d) => d.value !== null));
      lowerBand.setData(bollingerData.lower.filter((d) => d.value !== null));

      this.indicatorSeries.bollinger = [upperBand, middleBand, lowerBand];
    }

    _updateIndicators() {
      const indicators = [...this.options.indicators];
      indicators.forEach((type) => {
        this.removeIndicator(type);
        this.addIndicator(type);
      });
    }

    // ========================================================================
    // POSITION MARKERS
    // ========================================================================

    /**
     * Add position marker to chart
     * @param {Object} position - { type: 'entry'|'exit'|'dca'|'stopLoss'|'takeProfit', price, timestamp, label }
     */
    addPositionMarker(position) {
      if (!this.mainSeries) return;

      const colors = this.theme.positionColors;
      const color = colors[position.type] || colors.entry;

      // Add horizontal price line
      const priceLine = this.mainSeries.createPriceLine({
        price: position.price,
        color: color,
        lineWidth: 1,
        lineStyle: 2, // Dashed
        axisLabelVisible: true,
        title: position.label || position.type.toUpperCase(),
      });

      // Add marker on chart if timestamp provided
      if (position.timestamp) {
        const markerShape = {
          entry: "arrowUp",
          exit: "arrowDown",
          dca: "circle",
          stopLoss: "square",
          takeProfit: "square",
        };

        const marker = {
          time: position.timestamp,
          position: position.type === "exit" ? "aboveBar" : "belowBar",
          color: color,
          shape: markerShape[position.type] || "circle",
          text: position.label || "",
          size: 1,
        };

        // Add to markers array
        const existingMarkers = this.mainSeries.markers() || [];
        this.mainSeries.setMarkers([...existingMarkers, marker]);
      }

      this.positionMarkers.push({ ...position, priceLine });
    }

    /**
     * Clear all position markers
     */
    clearPositionMarkers() {
      if (!this.mainSeries) return;

      this.positionMarkers.forEach((p) => {
        if (p.priceLine) {
          this.mainSeries.removePriceLine(p.priceLine);
        }
      });
      this.positionMarkers = [];
      this.mainSeries.setMarkers([]);
    }

    /**
     * Set multiple positions at once
     * @param {Array} positions - Array of position objects
     */
    setPositions(positions) {
      this.clearPositionMarkers();
      positions.forEach((p) => this.addPositionMarker(p));
    }

    // ========================================================================
    // OVERLAYS & ANNOTATIONS
    // ========================================================================

    /**
     * Add horizontal line overlay
     * @param {Object} options - { price, color, label, style }
     */
    addHorizontalLine(options) {
      if (!this.mainSeries) return null;

      const line = this.mainSeries.createPriceLine({
        price: options.price,
        color: options.color || this.theme.crosshairColor,
        lineWidth: options.lineWidth || 1,
        lineStyle: options.style || 0, // 0=solid, 1=dotted, 2=dashed
        axisLabelVisible: options.showLabel !== false,
        title: options.label || "",
      });

      this.overlaySeries.push({ type: "hline", line, options });
      return line;
    }

    /**
     * Remove horizontal line
     * @param {Object} line - Line object returned from addHorizontalLine
     */
    removeHorizontalLine(line) {
      if (!this.mainSeries || !line) return;
      this.mainSeries.removePriceLine(line);
      this.overlaySeries = this.overlaySeries.filter((o) => o.line !== line);
    }

    /**
     * Clear all overlays
     */
    clearOverlays() {
      if (!this.mainSeries) {
        this.overlaySeries = [];
        return;
      }

      this.overlaySeries.forEach((overlay) => {
        if (overlay.type === "hline" && overlay.line) {
          this.mainSeries.removePriceLine(overlay.line);
        }
      });
      this.overlaySeries = [];
    }

    // ========================================================================
    // COMPARISON MODE
    // ========================================================================

    /**
     * Add comparison series
     * @param {Object} options - { symbol, data, color }
     */
    addComparison(options) {
      if (!options.data || !options.data.length) return;

      const series = this.chart.addLineSeries({
        color: options.color || "#8b949e",
        lineWidth: 2,
        priceLineVisible: false,
        lastValueVisible: true,
        crosshairMarkerVisible: true,
        priceScaleId: "comparison",
      });

      // Normalize data to percentage change from first point
      const firstPrice = options.data[0].close;
      const normalizedData = options.data.map((d) => ({
        time: d.time || d.timestamp,
        value: ((d.close - firstPrice) / firstPrice) * 100,
      }));

      series.setData(normalizedData);

      this.comparisonSeries.push({
        symbol: options.symbol,
        series,
        color: options.color,
      });

      this._updateLegend();
    }

    /**
     * Remove comparison series
     * @param {string} symbol - Symbol to remove
     */
    removeComparison(symbol) {
      const idx = this.comparisonSeries.findIndex((c) => c.symbol === symbol);
      if (idx >= 0) {
        this.chart.removeSeries(this.comparisonSeries[idx].series);
        this.comparisonSeries.splice(idx, 1);
        this._updateLegend();
      }
    }

    /**
     * Clear all comparison series
     */
    clearComparisons() {
      if (this.chart) {
        this.comparisonSeries.forEach((c) => this.chart.removeSeries(c.series));
      }
      this.comparisonSeries = [];
      this._updateLegend();
    }

    // ========================================================================
    // UI UPDATES
    // ========================================================================

    _updateLegend() {
      if (!this.legendEl) return;

      const lastData = this.data[this.data.length - 1];
      if (!lastData) {
        this.legendEl.innerHTML = "";
        return;
      }

      const priceChange = lastData.close - lastData.open;
      const priceChangePercent = (priceChange / lastData.open) * 100;
      const changeClass = priceChange >= 0 ? "positive" : "negative";

      let html = `
        <div class="legend-item main">
          <span class="legend-label">O</span>
          <span class="legend-value">${this._formatPrice(lastData.open)}</span>
          <span class="legend-label">H</span>
          <span class="legend-value">${this._formatPrice(lastData.high)}</span>
          <span class="legend-label">L</span>
          <span class="legend-value">${this._formatPrice(lastData.low)}</span>
          <span class="legend-label">C</span>
          <span class="legend-value ${changeClass}">${this._formatPrice(lastData.close)}</span>
          <span class="legend-change ${changeClass}">${priceChange >= 0 ? "+" : ""}${priceChangePercent.toFixed(2)}%</span>
        </div>
      `;

      // Add indicator values
      this.options.indicators.forEach((ind) => {
        if (this.indicatorSeries[ind]) {
          const series = Array.isArray(this.indicatorSeries[ind])
            ? this.indicatorSeries[ind][0]
            : this.indicatorSeries[ind];
          // Note: Getting last value requires data tracking
          html += `<div class="legend-item indicator"><span class="legend-indicator-name">${ind.toUpperCase()}</span></div>`;
        }
      });

      // Add comparison series
      this.comparisonSeries.forEach((c) => {
        html += `
          <div class="legend-item comparison" style="border-color: ${c.color}">
            <span class="legend-symbol">${c.symbol}</span>
          </div>
        `;
      });

      this.legendEl.innerHTML = html;
    }

    _updateTooltip(param) {
      if (!this.tooltipEl) return;

      if (
        !param ||
        param.time === undefined ||
        !param.point ||
        param.point.x < 0 ||
        param.point.y < 0
      ) {
        this.tooltipEl.style.display = "none";
        return;
      }

      const dataPoint = this.data.find((d) => d.time === param.time);
      if (!dataPoint) {
        this.tooltipEl.style.display = "none";
        return;
      }

      const date = new Date(dataPoint.time * 1000);
      const dateStr = date.toLocaleDateString(this.options.locale, {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });

      const change = dataPoint.close - dataPoint.open;
      const changePercent = (change / dataPoint.open) * 100;
      const changeClass = change >= 0 ? "positive" : "negative";

      this.tooltipEl.innerHTML = `
        <div class="tooltip-time">${dateStr}</div>
        <div class="tooltip-row">
          <span class="tooltip-label">Open</span>
          <span class="tooltip-value">${this._formatPrice(dataPoint.open)}</span>
        </div>
        <div class="tooltip-row">
          <span class="tooltip-label">High</span>
          <span class="tooltip-value">${this._formatPrice(dataPoint.high)}</span>
        </div>
        <div class="tooltip-row">
          <span class="tooltip-label">Low</span>
          <span class="tooltip-value">${this._formatPrice(dataPoint.low)}</span>
        </div>
        <div class="tooltip-row">
          <span class="tooltip-label">Close</span>
          <span class="tooltip-value ${changeClass}">${this._formatPrice(dataPoint.close)}</span>
        </div>
        <div class="tooltip-row">
          <span class="tooltip-label">Change</span>
          <span class="tooltip-value ${changeClass}">${change >= 0 ? "+" : ""}${changePercent.toFixed(2)}%</span>
        </div>
        ${dataPoint.volume ? `
        <div class="tooltip-row">
          <span class="tooltip-label">Volume</span>
          <span class="tooltip-value">${this._formatVolume(dataPoint.volume)}</span>
        </div>
        ` : ""}
      `;

      // Position tooltip
      const containerRect = this.container.getBoundingClientRect();
      let x = param.point.x + 20;
      let y = param.point.y;

      // Keep tooltip within bounds
      const tooltipRect = this.tooltipEl.getBoundingClientRect();
      if (x + tooltipRect.width > containerRect.width - 20) {
        x = param.point.x - tooltipRect.width - 20;
      }
      if (y + tooltipRect.height > containerRect.height - 20) {
        y = containerRect.height - tooltipRect.height - 20;
      }

      this.tooltipEl.style.left = `${x}px`;
      this.tooltipEl.style.top = `${y}px`;
      this.tooltipEl.style.display = "block";
    }

    // ========================================================================
    // EVENT HANDLERS
    // ========================================================================

    _setupEventHandlers() {
      // Crosshair move
      this.chart.subscribeCrosshairMove((param) => {
        this._updateTooltip(param);
        if (this.onCrosshairMove) {
          this.onCrosshairMove(param);
        }
      });

      // Time range change - track user interactions
      this.chart.timeScale().subscribeVisibleLogicalRangeChange((range) => {
        // Store last visible range for restoration
        this._lastVisibleRange = range;

        if (this.onTimeRangeChange) {
          this.onTimeRangeChange(range);
        }
      });

      // Track user scroll/zoom interactions
      this.chartArea.addEventListener('wheel', () => {
        this._markUserInteraction();
      });

      this.chartArea.addEventListener('mousedown', () => {
        this._markUserInteraction();
      });

      this.chartArea.addEventListener('touchstart', () => {
        this._markUserInteraction();
      });

      // Mouse leave - hide tooltip
      this.chartArea.addEventListener("mouseleave", () => {
        if (this.tooltipEl) {
          this.tooltipEl.style.display = "none";
        }
      });
    }

    /**
     * Mark that user has interacted with chart (zoom/pan)
     * User interaction flag decays after 30 seconds of no interaction
     */
    _markUserInteraction() {
      this._userHasInteracted = true;

      // Clear existing timeout
      if (this._interactionTimeout) {
        clearTimeout(this._interactionTimeout);
      }

      // Decay after 30 seconds of no interaction - then auto-follow newest data again
      this._interactionTimeout = setTimeout(() => {
        this._userHasInteracted = false;
      }, 30000);
    }

    /**
     * Reset user interaction flag (call to re-enable auto-fit)
     */
    resetUserInteraction() {
      this._userHasInteracted = false;
      if (this._interactionTimeout) {
        clearTimeout(this._interactionTimeout);
        this._interactionTimeout = null;
      }
    }

    _setupResizeObserver() {
      this.resizeObserver = new ResizeObserver(() => {
        if (this.chart && this.chartArea) {
          const width = this.chartArea.clientWidth;
          const height = this.chartArea.clientHeight;

          if (width > 0 && height > 0) {
            this.chart.applyOptions({ width, height });
          }
        }
      });

      // Observe the chartArea which has flex: 1
      this.resizeObserver.observe(this.chartArea);
    }

    // ========================================================================
    // FORMATTING HELPERS
    // ========================================================================

    _formatPrice(price) {
      switch (this.options.priceFormat) {
        case "subscript":
          return formatPriceSubscript(price, this.options.pricePrecision);
        case "scientific":
          if (price === 0) return "0";
          if (Math.abs(price) < 0.0001) return price.toExponential(4);
          return price.toFixed(this.options.pricePrecision);
        case "fixed":
          return price.toFixed(this.options.pricePrecision);
        case "auto":
        default:
          return formatPriceAuto(price, this.options.pricePrecision);
      }
    }

    _formatVolume(volume) {
      if (volume >= 1e9) return (volume / 1e9).toFixed(2) + "B";
      if (volume >= 1e6) return (volume / 1e6).toFixed(2) + "M";
      if (volume >= 1e3) return (volume / 1e3).toFixed(2) + "K";
      return volume.toFixed(this.options.volumePrecision);
    }

    // ========================================================================
    // PUBLIC API
    // ========================================================================

    /**
     * Set theme
     * @param {string} themeName - 'dark' or 'light'
     */
    setTheme(themeName) {
      this.theme = CHART_THEMES[themeName] || CHART_THEMES.dark;
      this.options.theme = themeName;

      if (this.chart) {
        this.chart.applyOptions({
          layout: {
            background: { color: this.theme.background },
            textColor: this.theme.textColor,
          },
          grid: {
            vertLines: { color: this.theme.gridColor },
            horzLines: { color: this.theme.gridColor },
          },
          crosshair: {
            vertLine: {
              color: this.theme.crosshairColor,
              labelBackgroundColor: this.theme.tooltipBackground,
            },
            horzLine: {
              color: this.theme.crosshairColor,
              labelBackgroundColor: this.theme.tooltipBackground,
            },
          },
          rightPriceScale: { borderColor: this.theme.borderColor },
          leftPriceScale: { borderColor: this.theme.borderColor },
          timeScale: { borderColor: this.theme.borderColor },
        });

        // Update series colors
        if (this.mainSeries) {
          if (this.options.chartType === "candlestick") {
            this.mainSeries.applyOptions({
              upColor: this.theme.upColor,
              downColor: this.theme.downColor,
              wickUpColor: this.theme.wickUpColor,
              wickDownColor: this.theme.wickDownColor,
            });
          } else if (this.options.chartType === "line") {
            this.mainSeries.applyOptions({ color: this.theme.upColor });
          } else if (this.options.chartType === "area") {
            this.mainSeries.applyOptions({
              topColor: `${this.theme.upColor}40`,
              bottomColor: `${this.theme.upColor}05`,
              lineColor: this.theme.upColor,
            });
          }
        }

        // Update volume colors
        if (this.volumeSeries && this.volumeData.length) {
          const updatedVolumeData = this.data.map((d) => ({
            time: d.time,
            value: d.volume,
            color:
              d.close >= d.open
                ? this.theme.volumeUpColor
                : this.theme.volumeDownColor,
          }));
          this.volumeSeries.setData(updatedVolumeData);
        }

        // Refresh indicators with new colors
        this._updateIndicators();
      }
    }

    /**
     * Set chart type
     * @param {string} type - 'candlestick', 'line', 'area', 'bar'
     */
    setChartType(type) {
      if (type === this.options.chartType) return;

      // Remove old series
      if (this.mainSeries) {
        this.chart.removeSeries(this.mainSeries);
      }

      this.options.chartType = type;
      this._createMainSeries();

      // Reload data
      if (this.data.length) {
        if (type === "line" || type === "area") {
          this.mainSeries.setData(
            this.data.map((d) => ({ time: d.time, value: d.close }))
          );
        } else {
          this.mainSeries.setData(this.data);
        }
      }

      // Re-add position markers
      const positions = this.positionMarkers.map((p) => ({
        type: p.type,
        price: p.price,
        timestamp: p.timestamp,
        label: p.label,
      }));
      this.clearPositionMarkers();
      positions.forEach((p) => this.addPositionMarker(p));
    }

    /**
     * Toggle volume visibility
     * @param {boolean} visible
     */
    setVolumeVisible(visible) {
      this.options.showVolume = visible;

      if (visible && !this.volumeSeries) {
        this._createVolumeSeries();
        if (this.data.length) {
          const volumeData = this.data.map((d) => ({
            time: d.time,
            value: d.volume,
            color:
              d.close >= d.open
                ? this.theme.volumeUpColor
                : this.theme.volumeDownColor,
          }));
          this.volumeSeries.setData(volumeData);
        }
      } else if (!visible && this.volumeSeries) {
        this.chart.removeSeries(this.volumeSeries);
        this.volumeSeries = null;
      }
    }

    /**
     * Toggle grid visibility
     * @param {boolean} visible
     */
    setGridVisible(visible) {
      this.options.showGrid = visible;
      this.chart.applyOptions({
        grid: {
          vertLines: { color: visible ? this.theme.gridColor : "transparent" },
          horzLines: { color: visible ? this.theme.gridColor : "transparent" },
        },
      });
    }

    /**
     * Fit chart to show all data
     */
    fitContent() {
      if (this.chart) {
        this.chart.timeScale().fitContent();
      }
    }

    /**
     * Scroll to specific time
     * @param {number} timestamp - Unix timestamp in seconds
     * @param {boolean} animate - Whether to animate the scroll
     */
    scrollToTime(timestamp, animate = true) {
      if (!this.chart || !this.data.length) return;

      // Find the logical index for this timestamp
      const index = this.data.findIndex(d => d.time >= timestamp);
      if (index < 0) return;

      // Use scrollToRealTime for proper time-based scrolling
      this.chart.timeScale().scrollToRealTime();
    }

    /**
     * Scroll to show latest data while preserving current zoom level
     * Called during data updates when user hasn't interacted
     */
    _scrollToLatestPreserveZoom() {
      if (!this.chart || !this.data.length || !this._lastVisibleRange) return;

      // Calculate how many bars were visible
      const visibleBars = this._lastVisibleRange.to - this._lastVisibleRange.from;

      // Set new range ending at latest data
      const lastIndex = this.data.length - 1;
      this.chart.timeScale().setVisibleLogicalRange({
        from: Math.max(0, lastIndex - visibleBars + this.options.rightOffset),
        to: lastIndex + this.options.rightOffset,
      });
    }

    /**
     * Set visible range
     * @param {number} barsCount - Number of bars to show
     */
    setVisibleRange(barsCount) {
      if (this.chart && this.data.length) {
        const lastIndex = this.data.length - 1;
        this.chart.timeScale().setVisibleLogicalRange({
          from: Math.max(0, lastIndex - barsCount),
          to: lastIndex + this.options.rightOffset,
        });
      }
    }

    /**
     * Take screenshot of chart
     * @returns {string} Base64 encoded PNG
     */
    takeScreenshot() {
      if (this.chart) {
        return this.chart.takeScreenshot().toDataURL("image/png");
      }
      return null;
    }

    /**
     * Get current visible data
     * @returns {Array} Visible data points
     */
    getVisibleData() {
      if (!this.chart || !this.data.length) return [];

      const range = this.chart.timeScale().getVisibleLogicalRange();
      if (!range) return this.data;

      const startIdx = Math.max(0, Math.floor(range.from));
      const endIdx = Math.min(this.data.length - 1, Math.ceil(range.to));

      return this.data.slice(startIdx, endIdx + 1);
    }

    /**
     * Destroy chart and cleanup
     */
    destroy() {
      // Stop observers
      if (this.resizeObserver) {
        this.resizeObserver.disconnect();
        this.resizeObserver = null;
      }

      // Clear all series and indicators
      this.clearPositionMarkers();
      this.clearOverlays();
      this.clearComparisons();

      Object.keys(this.indicatorSeries).forEach((key) => {
        this.removeIndicator(key);
      });

      // Remove chart
      if (this.chart) {
        this.chart.remove();
        this.chart = null;
      }

      // Remove UI elements
      if (this.wrapper) {
        this.wrapper.remove();
        this.wrapper = null;
      }

      // Clear references
      this.mainSeries = null;
      this.volumeSeries = null;
      this.data = [];
      this.volumeData = [];
    }
  }

  // ==========================================================================
  // FACTORY FUNCTION FOR EASY CREATION
  // ==========================================================================

  /**
   * Create an AdvancedChart instance
   * @param {HTMLElement|string} container - Container element or selector
   * @param {Object} options - Chart options
   * @returns {AdvancedChart}
   */
  function createAdvancedChart(container, options = {}) {
    return new AdvancedChart(container, options);
  }

  // ==========================================================================
  // EXPORTS
  // ==========================================================================

  // Export to window for global access
  window.AdvancedChart = AdvancedChart;
  window.createAdvancedChart = createAdvancedChart;

  // Also export formatting utilities
  window.ChartUtils = {
    formatPriceSubscript,
    formatPriceAuto,
    CHART_THEMES,
    TIMEFRAMES,
    Indicators,
  };
})();
