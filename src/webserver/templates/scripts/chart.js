/**
 * ScreenerBot Chart System
 *
 * A modular, production-grade charting system built on LightweightCharts.
 * Supports OHLCV candles, volume, overlays (API price, pool price), indicators (MA, EMA),
 * and is fully responsive with theme support.
 *
 * Architecture:
 * - ChartConfig: Themes, defaults, responsive breakpoints
 * - ChartDataService: API calls, data transformation, caching
 * - ChartIndicators: Indicator calculations and registry
 * - ChartRenderer: LightweightCharts wrapper
 * - ChartManager: Public API and orchestration
 *
 * @module ChartSystem
 * @version 1.0.0
 */

// ============================================================================
// CHART CONFIG - Themes, Defaults, Constants
// ============================================================================

const ChartConfig = {
  // Theme configurations
  themes: {
    dark: {
      layout: {
        background: { color: "#1a1a1a" },
        textColor: "#d1d4dc",
      },
      grid: {
        vertLines: { color: "#2a2a2a" },
        horzLines: { color: "#2a2a2a" },
      },
      crosshair: {
        mode: 1, // Normal
        vertLine: {
          color: "#758696",
          width: 1,
          style: 3, // Dashed
          labelBackgroundColor: "#363c4e",
        },
        horzLine: {
          color: "#758696",
          width: 1,
          style: 3,
          labelBackgroundColor: "#363c4e",
        },
      },
      watermark: {
        color: "rgba(255, 255, 255, 0.05)",
        visible: true,
        text: "ScreenerBot",
        fontSize: 48,
        horzAlign: "center",
        vertAlign: "center",
      },
      candlestick: {
        upColor: "#26a69a",
        downColor: "#ef5350",
        borderUpColor: "#26a69a",
        borderDownColor: "#ef5350",
        wickUpColor: "#26a69a",
        wickDownColor: "#ef5350",
      },
      volume: {
        upColor: "rgba(38, 166, 154, 0.5)",
        downColor: "rgba(239, 83, 80, 0.5)",
      },
      overlays: {
        apiPrice: { color: "#2196F3", lineWidth: 2, lineStyle: 0 },
        poolPrice: { color: "#FF9800", lineWidth: 2, lineStyle: 0 },
      },
    },
    light: {
      layout: {
        background: { color: "#FFFFFF" },
        textColor: "#191919",
      },
      grid: {
        vertLines: { color: "#e0e0e0" },
        horzLines: { color: "#e0e0e0" },
      },
      crosshair: {
        mode: 1,
        vertLine: {
          color: "#9598a1",
          width: 1,
          style: 3,
          labelBackgroundColor: "#f0f3fa",
        },
        horzLine: {
          color: "#9598a1",
          width: 1,
          style: 3,
          labelBackgroundColor: "#f0f3fa",
        },
      },
      watermark: {
        color: "rgba(0, 0, 0, 0.05)",
        visible: true,
        text: "ScreenerBot",
        fontSize: 48,
        horzAlign: "center",
        vertAlign: "center",
      },
      candlestick: {
        upColor: "#26a69a",
        downColor: "#ef5350",
        borderUpColor: "#26a69a",
        borderDownColor: "#ef5350",
        wickUpColor: "#26a69a",
        wickDownColor: "#ef5350",
      },
      volume: {
        upColor: "rgba(38, 166, 154, 0.5)",
        downColor: "rgba(239, 83, 80, 0.5)",
      },
      overlays: {
        apiPrice: { color: "#1976D2", lineWidth: 2, lineStyle: 0 },
        poolPrice: { color: "#F57C00", lineWidth: 2, lineStyle: 0 },
      },
    },
  },

  // Responsive breakpoints
  breakpoints: {
    mobile: 768,
    tablet: 1024,
  },

  // Timeframe configurations
  timeframes: {
    "1m": { label: "1m", seconds: 60 },
    "5m": { label: "5m", seconds: 300 },
    "15m": { label: "15m", seconds: 900 },
    "1h": { label: "1h", seconds: 3600 },
    "4h": { label: "4h", seconds: 14400 },
    "1D": { label: "1D", seconds: 86400 },
  },

  // Default chart options
  defaults: {
    height: 500,
    responsive: true,
    locale: "en-US",
    timeScale: {
      timeVisible: true,
      secondsVisible: false,
      borderColor: "#2a2a2a",
    },
    rightPriceScale: {
      borderColor: "#2a2a2a",
      scaleMargins: {
        top: 0.1,
        bottom: 0.2,
      },
    },
    handleScroll: {
      mouseWheel: true,
      pressedMouseMove: true,
      horzTouchDrag: true,
      vertTouchDrag: true,
    },
    handleScale: {
      axisPressedMouseMove: true,
      mouseWheel: true,
      pinch: true,
    },
  },

  // Volume pane configuration
  volumePane: {
    height: 20, // Percentage of main chart
    scaleMargins: {
      top: 0.8,
      bottom: 0,
    },
  },
};

// ============================================================================
// CHART DATA SERVICE - API Calls, Transformation, Caching
// ============================================================================

class ChartDataService {
  constructor() {
    this.cache = new Map();
    this.cacheExpiry = 30000; // 30 seconds
  }

  /**
   * Fetch OHLCV data from backend
   * @param {string} mint - Token mint address
   * @param {string} timeframe - Timeframe (1m, 5m, etc.)
   * @returns {Promise<Array>} OHLCV data points
   */
  async fetchOHLCV(mint, timeframe) {
    console.log("[ChartDataService] fetchOHLCV() called:", { mint, timeframe });

    const cacheKey = `ohlcv:${mint}:${timeframe}`;
    const cached = this.cache.get(cacheKey);

    if (cached && Date.now() - cached.timestamp < this.cacheExpiry) {
      console.log("[ChartDataService] Returning cached data");
      return cached.data;
    }

    try {
      const url = `/api/tokens/${mint}/ohlcv?timeframe=${timeframe}`;
      console.log("[ChartDataService] Fetching from:", url);

      const response = await fetch(url);
      console.log("[ChartDataService] Response status:", response.status);

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();
      console.log("[ChartDataService] Raw data points:", data.length);

      // Transform to LightweightCharts format
      const transformed = this.transformOHLCV(data);
      console.log("[ChartDataService] Transformed:", {
        candles: transformed.candles.length,
        volume: transformed.volume.length,
      });

      this.cache.set(cacheKey, {
        data: transformed,
        timestamp: Date.now(),
      });

      return transformed;
    } catch (error) {
      console.error("[ChartDataService] fetchOHLCV error:", error);
      throw error;
    }
  }

  /**
   * Transform OHLCV data to LightweightCharts format
   * @param {Array} data - Raw OHLCV data
   * @returns {Object} { candles, volume }
   */
  transformOHLCV(data) {
    if (!data || !Array.isArray(data) || data.length === 0) {
      return { candles: [], volume: [] };
    }

    const candles = [];
    const volume = [];

    for (const point of data) {
      const time = point.timestamp || point.time;

      candles.push({
        time: time,
        open: parseFloat(point.open),
        high: parseFloat(point.high),
        low: parseFloat(point.low),
        close: parseFloat(point.close),
      });

      if (point.volume !== undefined) {
        volume.push({
          time: time,
          value: parseFloat(point.volume),
          color:
            parseFloat(point.close) >= parseFloat(point.open)
              ? "rgba(38, 166, 154, 0.5)"
              : "rgba(239, 83, 80, 0.5)",
        });
      }
    }

    return { candles, volume };
  }

  /**
   * Fetch pool price history (placeholder - needs backend implementation)
   * @param {string} mint - Token mint address
   * @param {string} timeframe - Timeframe
   * @returns {Promise<Array>} Pool price history
   */
  async fetchPoolPriceHistory(mint, timeframe) {
    // TODO: Implement backend endpoint /api/tokens/:mint/pool-price-history
    // For now, return empty array
    console.warn(
      "[ChartDataService] Pool price history endpoint not yet implemented"
    );
    return [];
  }

  /**
   * Transform pool price data to line series format
   * @param {Array} data - Raw pool price data
   * @returns {Array} Line series data
   */
  transformPoolPrice(data) {
    if (!data || !Array.isArray(data)) {
      return [];
    }

    return data.map((point) => ({
      time: point.timestamp || point.time,
      value: parseFloat(point.price),
    }));
  }

  /**
   * Clear cache for specific token or all
   * @param {string} mint - Optional mint to clear specific cache
   */
  clearCache(mint = null) {
    if (mint) {
      for (const key of this.cache.keys()) {
        if (key.includes(mint)) {
          this.cache.delete(key);
        }
      }
    } else {
      this.cache.clear();
    }
  }
}

// ============================================================================
// CHART INDICATORS - Calculations and Registry
// ============================================================================

class ChartIndicators {
  constructor() {
    this.registry = new Map();
    this.registerDefaults();
  }

  /**
   * Register default indicators
   */
  registerDefaults() {
    // Simple Moving Average
    this.register("SMA", {
      name: "Simple Moving Average",
      calculate: (data, params) => {
        const period = params.period || 20;
        const result = [];

        for (let i = 0; i < data.length; i++) {
          if (i < period - 1) {
            continue;
          }

          const slice = data.slice(i - period + 1, i + 1);
          const sum = slice.reduce((acc, point) => acc + point.close, 0);
          const avg = sum / period;

          result.push({
            time: data[i].time,
            value: avg,
          });
        }

        return result;
      },
      defaultParams: { period: 20 },
      defaultColor: "#2196F3",
      lineStyle: 0, // Solid
      lineWidth: 2,
    });

    // Exponential Moving Average
    this.register("EMA", {
      name: "Exponential Moving Average",
      calculate: (data, params) => {
        const period = params.period || 20;
        const multiplier = 2 / (period + 1);
        const result = [];

        if (data.length < period) {
          return result;
        }

        // Calculate initial SMA
        let ema = 0;
        for (let i = 0; i < period; i++) {
          ema += data[i].close;
        }
        ema = ema / period;

        result.push({
          time: data[period - 1].time,
          value: ema,
        });

        // Calculate EMA
        for (let i = period; i < data.length; i++) {
          ema = (data[i].close - ema) * multiplier + ema;
          result.push({
            time: data[i].time,
            value: ema,
          });
        }

        return result;
      },
      defaultParams: { period: 20 },
      defaultColor: "#FF9800",
      lineStyle: 0,
      lineWidth: 2,
    });
  }

  /**
   * Register a new indicator
   * @param {string} id - Indicator ID
   * @param {Object} config - Indicator configuration
   */
  register(id, config) {
    this.registry.set(id, config);
  }

  /**
   * Calculate indicator data
   * @param {string} id - Indicator ID
   * @param {Array} data - OHLCV data
   * @param {Object} params - Indicator parameters
   * @returns {Array} Calculated indicator data
   */
  calculate(id, data, params = {}) {
    const indicator = this.registry.get(id);
    if (!indicator) {
      throw new Error(`Indicator ${id} not found`);
    }

    const mergedParams = { ...indicator.defaultParams, ...params };
    return indicator.calculate(data, mergedParams);
  }

  /**
   * Get indicator configuration
   * @param {string} id - Indicator ID
   * @returns {Object} Indicator config
   */
  getConfig(id) {
    return this.registry.get(id);
  }

  /**
   * List all registered indicators
   * @returns {Array} Array of indicator IDs
   */
  list() {
    return Array.from(this.registry.keys());
  }
}

// ============================================================================
// CHART RENDERER - LightweightCharts Wrapper
// ============================================================================

class ChartRenderer {
  constructor(container, options = {}) {
    this.container = container;
    this.options = options;
    this.chart = null;
    this.series = new Map();
    this.currentTheme = options.theme || "dark";
  }

  /**
   * Initialize chart
   */
  initialize() {
    if (this.chart) {
      this.destroy();
    }

    const theme = ChartConfig.themes[this.currentTheme];
    const chartOptions = {
      ...ChartConfig.defaults,
      ...theme,
      width: this.container.clientWidth,
      height: this.options.height || ChartConfig.defaults.height,
    };

    this.chart = LightweightCharts.createChart(this.container, chartOptions);

    // Setup resize observer
    this.setupResizeObserver();
  }

  /**
   * Add candlestick series
   * @param {Array} data - Candlestick data
   * @returns {Object} Series reference
   */
  addCandlestickSeries(data) {
    const theme = ChartConfig.themes[this.currentTheme];
    const series = this.chart.addCandlestickSeries({
      ...theme.candlestick,
    });

    if (data && data.length > 0) {
      series.setData(data);
    }

    this.series.set("candlestick", series);
    return series;
  }

  /**
   * Add volume histogram
   * @param {Array} data - Volume data
   * @returns {Object} Series reference
   */
  addVolumeHistogram(data) {
    const series = this.chart.addHistogramSeries({
      priceFormat: {
        type: "volume",
      },
      priceScaleId: "volume",
      scaleMargins: ChartConfig.volumePane.scaleMargins,
    });

    if (data && data.length > 0) {
      series.setData(data);
    }

    this.series.set("volume", series);
    return series;
  }

  /**
   * Add line series (for overlays like API price, pool price)
   * @param {string} id - Series ID
   * @param {Array} data - Line data
   * @param {Object} options - Line options
   * @returns {Object} Series reference
   */
  addLineSeries(id, data, options = {}) {
    const series = this.chart.addLineSeries({
      color: options.color || "#2196F3",
      lineWidth: options.lineWidth || 2,
      lineStyle: options.lineStyle || 0,
      priceLineVisible: false,
      lastValueVisible: true,
      title: options.title || id,
    });

    if (data && data.length > 0) {
      series.setData(data);
    }

    this.series.set(id, series);
    return series;
  }

  /**
   * Remove series
   * @param {string} id - Series ID
   */
  removeSeries(id) {
    const series = this.series.get(id);
    if (series) {
      this.chart.removeSeries(series);
      this.series.delete(id);
    }
  }

  /**
   * Update series data
   * @param {string} id - Series ID
   * @param {Array} data - New data
   */
  updateSeries(id, data) {
    const series = this.series.get(id);
    if (series && data && data.length > 0) {
      series.setData(data);
    }
  }

  /**
   * Set theme
   * @param {string} theme - Theme name ('dark' or 'light')
   */
  setTheme(theme) {
    this.currentTheme = theme;
    if (this.chart) {
      const themeConfig = ChartConfig.themes[theme];
      this.chart.applyOptions({
        ...themeConfig,
      });
    }
  }

  /**
   * Resize chart to fit container
   */
  resize() {
    if (this.chart && this.container) {
      this.chart.applyOptions({
        width: this.container.clientWidth,
        height: this.options.height || this.container.clientHeight,
      });
    }
  }

  /**
   * Setup resize observer for responsive behavior
   */
  setupResizeObserver() {
    if (typeof ResizeObserver !== "undefined") {
      const resizeObserver = new ResizeObserver(() => {
        this.resize();
      });
      resizeObserver.observe(this.container);
      this.resizeObserver = resizeObserver;
    }
  }

  /**
   * Fit content to chart
   */
  fitContent() {
    if (this.chart) {
      this.chart.timeScale().fitContent();
    }
  }

  /**
   * Destroy chart and cleanup
   */
  destroy() {
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
    }

    if (this.chart) {
      this.chart.remove();
      this.chart = null;
    }

    this.series.clear();
  }
}

// ============================================================================
// CHART MANAGER - Public API and Orchestration
// ============================================================================

class ChartManager {
  /**
   * @param {string|HTMLElement} container - Container ID or element
   * @param {Object} options - Chart options
   */
  constructor(container, options = {}) {
    this.container =
      typeof container === "string"
        ? document.getElementById(container)
        : container;

    if (!this.container) {
      throw new Error("Chart container not found");
    }

    this.options = {
      theme: options.theme || "dark",
      height: options.height || 500,
      responsive: options.responsive !== false,
      indicators: options.indicators || [],
      overlays: options.overlays || [],
      showVolume: options.showVolume !== false,
    };

    // Find loading/error elements - they're siblings of the chart container
    const parentContainer = this.container.parentElement;
    this.ui = {
      loading:
        parentContainer?.querySelector("#chart-loading") ||
        document.querySelector("#chart-loading"),
      error:
        parentContainer?.querySelector("#chart-error") ||
        document.querySelector("#chart-error"),
    };

    console.log("[ChartManager] UI elements:", {
      loading: this.ui.loading ? "found" : "NOT FOUND",
      error: this.ui.error ? "found" : "NOT FOUND",
      container: this.container.id,
      parent: parentContainer?.id,
    });

    this.dataService = new ChartDataService();
    this.indicatorService = new ChartIndicators();
    this.renderer = null;

    this.currentMint = null;
    this.currentTimeframe = "5m";
    this.currentData = null;

    this.boundResizeHandler = null;

    this.state = {
      loading: false,
      error: null,
      indicators: new Map(),
      overlays: new Set(),
      overlayPreferences: new Map(),
    };

    this.initialize();
  }

  /**
   * Initialize chart system
   */
  initialize() {
    try {
      this.renderer = new ChartRenderer(this.container, {
        theme: this.options.theme,
        height: this.options.height,
      });
      this.renderer.initialize();

      // Apply responsive behavior
      if (this.options.responsive) {
        this.setupResponsive();
      }
    } catch (error) {
      console.error("[ChartManager] Initialization error:", error);
      this.showError("Failed to initialize chart");
    }
  }

  /**
   * Load token data and render chart
   * @param {string} mint - Token mint address
   * @param {string} timeframe - Timeframe (default: 5m)
   * @returns {Promise<void>}
   */
  async loadToken(mint, timeframe = "5m") {
    console.log("[ChartManager] loadToken() called with:", { mint, timeframe });

    if (this.state.loading) {
      console.warn("[ChartManager] Already loading data");
      return;
    }

    const volumePreference = this.state.overlayPreferences.has("volume")
      ? this.state.overlayPreferences.get("volume")
      : this.options.showVolume;
    const shouldRenderVolume = Boolean(volumePreference);
    if (!this.state.overlayPreferences.has("volume")) {
      this.state.overlayPreferences.set("volume", shouldRenderVolume);
    }

    this.currentMint = mint;
    this.currentTimeframe = timeframe;
    this.state.loading = true;
    this.state.error = null;

    console.log("[ChartManager] State updated, showing loading...");
    this.showLoading();

    try {
      console.log("[ChartManager] Fetching OHLCV data...");
      const ohlcvData = await this.dataService.fetchOHLCV(mint, timeframe);
      console.log("[ChartManager] OHLCV data received:", {
        candles: ohlcvData.candles.length,
        volume: ohlcvData.volume.length,
      });

      this.currentData = ohlcvData;

      // Render candlestick chart
      console.log("[ChartManager] Adding candlestick series...");
      this.renderer.addCandlestickSeries(ohlcvData.candles);

      // Render volume if enabled by default or previously selected
      if (shouldRenderVolume && ohlcvData.volume.length > 0) {
        console.log("[ChartManager] Adding volume histogram...");
        this.renderer.addVolumeHistogram(ohlcvData.volume);
        this.state.overlays.add("volume");
      } else {
        this.renderer.removeSeries("volume");
        this.state.overlays.delete("volume");
      }

      // Apply indicators, preserving any user selections
      const persistedIndicators = this.state.indicators.size
        ? Array.from(this.state.indicators.values())
        : null;

      if (persistedIndicators) {
        this.state.indicators.clear();
        for (const indicator of persistedIndicators) {
          await this.addIndicator(indicator.id, {
            ...indicator.params,
            color: indicator.color,
          });
        }
      } else {
        console.log(
          "[ChartManager] Applying indicators:",
          this.options.indicators
        );
        for (const indicatorConfig of this.options.indicators) {
          if (typeof indicatorConfig === "string") {
            await this.addIndicator(indicatorConfig);
          } else {
            await this.addIndicator(indicatorConfig.id, {
              ...indicatorConfig.params,
              color: indicatorConfig.color,
            });
          }
        }
      }

      // Fit content
      console.log("[ChartManager] Fitting content...");
      this.renderer.fitContent();

      console.log("[ChartManager] Hiding loading, chart complete!");
      this.hideLoading();
      this.state.loading = false;
    } catch (error) {
      console.error("[ChartManager] Load error:", error);
      this.showError(`Failed to load chart data: ${error.message}`);
      this.state.loading = false;
    }
  }

  /**
   * Add indicator to chart
   * @param {string} indicatorId - Indicator ID (SMA, EMA, etc.)
   * @param {Object} params - Indicator parameters
   * @returns {Promise<void>}
   */
  async addIndicator(indicatorId, params = {}) {
    try {
      if (!this.currentData || !this.currentData.candles) {
        throw new Error("No data loaded");
      }

      const config = this.indicatorService.getConfig(indicatorId);
      if (!config) {
        throw new Error(`Indicator ${indicatorId} not found`);
      }

      const { color: paramColor, ...indicatorParams } = params;
      const mergedParams = { ...config.defaultParams, ...indicatorParams };
      const indicatorData = this.indicatorService.calculate(
        indicatorId,
        this.currentData.candles,
        mergedParams
      );

      const seriesId = `indicator:${indicatorId}:${
        mergedParams.period || "default"
      }`;
      const color = paramColor || config.defaultColor;

      this.renderer.addLineSeries(seriesId, indicatorData, {
        color: color,
        lineWidth: config.lineWidth,
        lineStyle: config.lineStyle,
        title: `${indicatorId}(${mergedParams.period || ""})`,
      });

      this.state.indicators.set(seriesId, {
        id: indicatorId,
        params: mergedParams,
        color: color,
      });
    } catch (error) {
      console.error("[ChartManager] Add indicator error:", error);
    }
  }

  /**
   * Remove indicator from chart
   * @param {string} seriesId - Series ID
   */
  removeIndicator(seriesId) {
    this.renderer.removeSeries(seriesId);
    this.state.indicators.delete(seriesId);
  }

  /**
   * Toggle overlay (volume, API price, pool price)
   * @param {string} overlayId - Overlay ID
   */
  toggleOverlay(overlayId) {
    if (!this.renderer) {
      return;
    }

    if (overlayId === "volume") {
      if (this.state.overlays.has("volume")) {
        this.renderer.removeSeries("volume");
        this.state.overlays.delete("volume");
        this.state.overlayPreferences.set("volume", false);
        return;
      }

      this.state.overlayPreferences.set("volume", true);
      if (this.currentData && this.currentData.volume.length > 0) {
        this.renderer.addVolumeHistogram(this.currentData.volume);
        this.state.overlays.add("volume");
      } else {
        console.warn("[ChartManager] No volume data available to render");
      }
      return;
    }

    if (this.state.overlays.has(overlayId)) {
      this.renderer.removeSeries(overlayId);
      this.state.overlays.delete(overlayId);
    } else {
      this.state.overlays.add(overlayId);
    }
  }

  /**
   * Change timeframe and reload data
   * @param {string} timeframe - New timeframe
   * @returns {Promise<void>}
   */
  async setTimeframe(timeframe) {
    if (!this.currentMint) {
      return;
    }

    this.currentTimeframe = timeframe;

    // Clear current chart
    this.renderer.destroy();
    this.renderer.initialize();

    // Reload with new timeframe
    await this.loadToken(this.currentMint, timeframe);
  }

  /**
   * Change theme
   * @param {string} theme - Theme name ('dark' or 'light')
   */
  setTheme(theme) {
    this.options.theme = theme;
    if (this.renderer) {
      this.renderer.setTheme(theme);
    }
  }

  /**
   * Resize chart
   */
  resize() {
    if (this.renderer) {
      this.renderer.resize();
    }
  }

  /**
   * Setup responsive behavior
   */
  setupResponsive() {
    if (this.boundResizeHandler) {
      window.removeEventListener("resize", this.boundResizeHandler);
    }

    this.boundResizeHandler = () => this.resize();
    window.addEventListener("resize", this.boundResizeHandler);
  }

  /**
   * Show loading state
   */
  showLoading() {
    console.log("[ChartManager] showLoading() called");
    const loading = this.ui.loading;
    const error = this.ui.error;

    console.log("[ChartManager] Loading element:", loading);
    if (loading) {
      loading.style.display = "flex";
      console.log(
        "[ChartManager] Loading shown, display:",
        loading.style.display
      );
    } else {
      console.error("[ChartManager] Loading element not found!");
    }
    if (error) error.style.display = "none";
  }

  /**
   * Hide loading state
   */
  hideLoading() {
    console.log("[ChartManager] hideLoading() called");
    const loading = this.ui.loading;
    console.log("[ChartManager] Loading element:", loading);
    if (loading) {
      loading.style.display = "none";
      console.log(
        "[ChartManager] Loading hidden, display:",
        loading.style.display
      );
    } else {
      console.error("[ChartManager] Loading element not found!");
    }
  }

  /**
   * Show error message
   * @param {string} message - Error message
   */
  showError(message) {
    const loading = this.ui.loading;
    const error = this.ui.error;

    if (loading) loading.style.display = "none";
    if (error) {
      error.style.display = "flex";
      const errorText = error.querySelector("p");
      if (errorText) {
        errorText.textContent = message;
      }
    }
  }

  /**
   * Get current state
   * @returns {Object} Current state
   */
  getState() {
    return {
      loading: this.state.loading,
      error: this.state.error,
      mint: this.currentMint,
      timeframe: this.currentTimeframe,
      indicators: Array.from(this.state.indicators.entries()),
      overlays: Array.from(this.state.overlays),
      overlayPreferences: Array.from(this.state.overlayPreferences.entries()),
    };
  }

  /**
   * Destroy chart and cleanup
   */
  destroy() {
    if (this.boundResizeHandler) {
      window.removeEventListener("resize", this.boundResizeHandler);
      this.boundResizeHandler = null;
    }

    if (this.renderer) {
      this.renderer.destroy();
    }

    this.dataService.clearCache();
    this.state.indicators.clear();
    this.state.overlays.clear();
    this.state.overlayPreferences.clear();
  }
}

// ============================================================================
// EXPORTS
// ============================================================================

// Make available globally
window.ChartManager = ChartManager;
window.ChartConfig = ChartConfig;
window.ChartDataService = ChartDataService;
window.ChartIndicators = ChartIndicators;
window.ChartRenderer = ChartRenderer;

console.log("[ChartSystem] Loaded successfully");
