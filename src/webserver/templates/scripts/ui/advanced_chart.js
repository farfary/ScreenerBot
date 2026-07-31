/**
 * AdvancedChart - the shared OHLCV chart engine for ScreenerBot dialogs
 *
 * Features:
 * - Candlestick, line, area and bar series over one dataset
 * - EMA overlays, position markers, horizontal reference lines
 * - Price formatting delegated to Utils.formatPriceSubscript (one policy)
 * - Framing: anchorLatest() for "what is happening now", anchorRange() for
 *   "show me this span" (the position chart frames a position's lifetime)
 * - Zoom/pan with mouse wheel, trackpad and touch, with an optional decay back
 *   to the owner's frame
 * - Light/dark theming, ResizeObserver sizing, crosshair tooltip
 *
 * Dependencies:
 * - lightweight-charts (TradingView) v4
 * - advanced_chart/themes.js (CHART_THEMES)
 * - advanced_chart/indicators.js (technical indicator calculations)
 * - core/utils.js (Utils.formatPriceSubscript — the one price format policy)
 */

(function () {
  "use strict";

  // ==========================================================================
  // IMPORT FROM EXTERNAL MODULES
  // ==========================================================================

  // These are loaded from separate script files and exposed on window
  const { CHART_THEMES } = window.ChartThemes || {};
  const Indicators = window.ChartIndicators || {};

  // Validate dependencies are loaded
  if (!CHART_THEMES || !Indicators) {
    console.error(
      "AdvancedChart: Missing dependencies. Ensure themes.js and indicators.js are loaded first."
    );
  }

  // ==========================================================================
  // CONSTANTS & DEFAULTS
  // ==========================================================================

  /**
   * Floor on how few bars a range frame may show. setVisibleRange() derives
   * candle WIDTH from the span, so framing a position that lived three candles
   * would stretch those three into pane-wide slabs. Widening the window to at
   * least this many bars keeps candles readable at any position duration.
   */
  const MIN_FRAME_BARS = 40;

  const DEFAULT_OPTIONS = {
    theme: "dark",
    chartType: "candlestick", // candlestick, line, area, bar
    showVolume: true,
    showGrid: true,
    showCrosshair: true,
    showTooltip: true,
    barSpacing: 12,
    minBarSpacing: 4,
    rightOffset: 5,
    indicators: [], // ['ema9', 'ema21']
    locale: "en-US",
    // Significant digits for every price this chart prints (axis, tooltip).
    // Not decimal places — see Utils.formatPriceSubscript.
    pricePrecision: 5,
    volumePrecision: 2,
    // Optional (bar) => [{ label, value, cls }] hook: extra tooltip rows for the
    // surface that owns the chart (e.g. a position's entry and P&L at that bar).
    tooltipExtraRows: null,
    // How long a user zoom/pan suppresses re-framing on the next data refresh.
    // null = never re-frame again until the owner explicitly re-anchors, which
    // is what a chart the user pans into the PAST needs (the position chart).
    interactionDecayMs: 30000,
    watermark: null, // { text, fontSize, color }
  };

  // ==========================================================================
  // ADVANCED CHART CLASS
  // ==========================================================================

  class AdvancedChart {
    constructor(container, options = {}) {
      this.container =
        typeof container === "string" ? document.querySelector(container) : container;

      if (!this.container) {
        throw new Error("AdvancedChart: Container element not found");
      }

      this.options = { ...DEFAULT_OPTIONS, ...options };
      // `indicators` is an array inside DEFAULT_OPTIONS. Without this clone every
      // chart that did not pass its own array SHARED that one instance, so
      // enabling EMA on one chart silently enabled it on every chart created
      // afterwards — with their toggles still reading "off".
      this.options.indicators = [...this.options.indicators];
      this.theme = CHART_THEMES[this.options.theme] || CHART_THEMES.dark;

      // Internal state
      this.chart = null;
      this.mainSeries = null;
      this.volumeSeries = null;

      // User interaction tracking - respects user zoom/pan actions
      this._userHasInteracted = false;
      this._isFirstDataLoad = true;
      this._interactionTimeout = null;
      this._maybeDragging = false;
      this.indicatorSeries = {};
      this.overlayLines = [];
      this.positionMarkers = [];
      this.data = [];
      this.volumeData = [];
      // Time -> bar index for O(1) crosshair lookups, and the detected bar
      // interval (seconds) used to label the hovered bar.
      this._barByTime = new Map();
      this._barSeconds = null;
      // How the view is framed when data lands and the user has not taken over.
      this._frame = { mode: "latest" };
      // Prices that must stay inside the visible price range even though they
      // are drawn as price lines (which do not extend the scale on their own).
      this._autoscalePrices = [];

      // UI elements
      this.tooltipEl = null;
      this.tooltipRefs = null;
      this._tooltipFrame = null;
      this._tooltipParam = null;

      // Observers
      this.resizeObserver = null;

      // Callbacks
      this.onCrosshairMove = null;

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
          visible: true,
          borderColor: this.theme.borderColor,
          scaleMargins: { top: 0.1, bottom: 0.2 },
        },
        timeScale: {
          visible: true,
          borderColor: this.theme.borderColor,
          timeVisible: true,
          secondsVisible: false,
          barSpacing: this.options.barSpacing,
          minBarSpacing: this.options.minBarSpacing,
          rightOffset: this.options.rightOffset,
          // lightweight-charts renders axis ticks in UTC by default, but the
          // crosshair tooltip formats in local time — that mismatch made the
          // axis label and the popup show different times for the same candle.
          // Format ticks in local time so both agree.
          tickMarkFormatter: (time, tickMarkType, locale) => {
            const d = new Date(time * 1000);
            switch (tickMarkType) {
              case 0: // Year
                return d.toLocaleDateString(locale, { year: "numeric" });
              case 1: // Month
                return d.toLocaleDateString(locale, { month: "short" });
              case 2: // DayOfMonth
                return d.toLocaleDateString(locale, {
                  day: "numeric",
                  month: "short",
                });
              case 3: // Time
                return d.toLocaleTimeString(locale, {
                  hour: "2-digit",
                  minute: "2-digit",
                });
              default: // TimeWithSeconds
                return d.toLocaleTimeString(locale, {
                  hour: "2-digit",
                  minute: "2-digit",
                  second: "2-digit",
                });
            }
          },
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
      const common = {
        priceFormat: priceFormatOptions,
        autoscaleInfoProvider: (base) => this._autoscaleInfo(base),
      };

      switch (this.options.chartType) {
        case "line":
          this.mainSeries = this.chart.addLineSeries({
            color: this.theme.upColor,
            lineWidth: 2,
            ...common,
          });
          break;

        case "area":
          this.mainSeries = this.chart.addAreaSeries({
            topColor: `${this.theme.upColor}40`,
            bottomColor: `${this.theme.upColor}05`,
            lineColor: this.theme.upColor,
            lineWidth: 2,
            ...common,
          });
          break;

        case "bar":
          this.mainSeries = this.chart.addBarSeries({
            upColor: this.theme.upColor,
            downColor: this.theme.downColor,
            ...common,
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
            ...common,
          });
          break;
      }
    }

    /**
     * Widen the auto price range so flagged reference lines stay on screen.
     *
     * lightweight-charts price lines are paint-only: they never extend the price
     * scale. A position whose average entry sits well outside the visible candle
     * range therefore drew its "Avg Entry" line off-pane and the chart looked
     * like it had no position on it at all. Prices registered through
     * setOverlayLines({ autoscale: true }) are folded into the range here.
     */
    _autoscaleInfo(baseImplementation) {
      const base = baseImplementation();
      if (!this._autoscalePrices.length) return base;

      const values = this._autoscalePrices.filter((p) => Number.isFinite(p) && p > 0);
      if (!values.length) return base;

      let minValue = Math.min(...values);
      let maxValue = Math.max(...values);
      if (base?.priceRange) {
        minValue = Math.min(minValue, base.priceRange.minValue);
        maxValue = Math.max(maxValue, base.priceRange.maxValue);
      }
      if (minValue === maxValue) return base;

      return { ...(base || {}), priceRange: { minValue, maxValue } };
    }

    /** Re-run autoscale after the registered reference prices changed. */
    _invalidateAutoscale() {
      if (!this.mainSeries) return;
      this.mainSeries.applyOptions({
        autoscaleInfoProvider: (base) => this._autoscaleInfo(base),
      });
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

    _createTooltip() {
      // The skeleton is built ONCE and the crosshair handler only writes
      // textContent into it. Rebuilding innerHTML per pointer sample re-parsed
      // the whole card and forced a synchronous layout on every mouse move.
      this.tooltipEl = document.createElement("div");
      this.tooltipEl.className = "advanced-chart-tooltip";
      this.tooltipEl.innerHTML = `
        <div class="tooltip-head">
          <span class="tooltip-date"></span>
          <span class="tooltip-interval"></span>
        </div>
        <div class="tooltip-headline">
          <span class="tooltip-price"></span>
          <span class="tooltip-change"></span>
        </div>
        <div class="tooltip-grid"></div>
        <div class="tooltip-grid tooltip-section tooltip-indicators is-empty"></div>
        <div class="tooltip-grid tooltip-section tooltip-extra is-empty"></div>
      `;
      // Anchored to the chart AREA (not the wrapper): crosshair points are
      // canvas-relative, so any other wrapper row would offset the whole card.
      this.chartArea.appendChild(this.tooltipEl);

      const grid = this.tooltipEl.querySelector(".tooltip-grid");
      this.tooltipRefs = {
        date: this.tooltipEl.querySelector(".tooltip-date"),
        interval: this.tooltipEl.querySelector(".tooltip-interval"),
        price: this.tooltipEl.querySelector(".tooltip-price"),
        change: this.tooltipEl.querySelector(".tooltip-change"),
        open: this._appendTooltipRow(grid, "Open"),
        high: this._appendTooltipRow(grid, "High"),
        low: this._appendTooltipRow(grid, "Low"),
        delta: this._appendTooltipRow(grid, "Chg"),
        range: this._appendTooltipRow(grid, "Range"),
        volume: this._appendTooltipRow(grid, "Vol"),
        indicators: this.tooltipEl.querySelector(".tooltip-indicators"),
        extra: this.tooltipEl.querySelector(".tooltip-extra"),
      };
      this._indicatorRowCache = { signature: null, values: [] };
      this._extraRowCache = { signature: null, values: [] };
    }

    /** Append a label/value pair to a tooltip grid and return the value node. */
    _appendTooltipRow(grid, label) {
      const labelEl = document.createElement("span");
      labelEl.className = "tooltip-label";
      labelEl.textContent = label;
      const valueEl = document.createElement("span");
      valueEl.className = "tooltip-value";
      grid.append(labelEl, valueEl);
      return valueEl;
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

      // Crosshair lookup index + detected bar interval. The interval is the
      // smallest gap between consecutive bars, so missing bars (no-trade candles
      // are never stored) cannot inflate it.
      this._barByTime = new Map(this.data.map((d) => [d.time, d]));
      this._barSeconds = this.data.reduce((smallest, bar, idx) => {
        if (idx === 0) return smallest;
        const gap = bar.time - this.data[idx - 1].time;
        return gap > 0 && (smallest === null || gap < smallest) ? gap : smallest;
      }, null);
      // The bars this card was describing have just been replaced. Re-render it
      // against the new set instead of hiding: the chart data poller calls
      // setData every few seconds, and blanking the tooltip on each tick would
      // make it flicker under a resting cursor. A hovered bar that no longer
      // exists (timeframe switch) makes the re-render hide it anyway.
      if (this.tooltipEl?.classList.contains("visible")) {
        this._renderTooltip(this._tooltipParam);
      } else {
        this._hideTooltip();
      }

      // Set main series data
      if (this.options.chartType === "line" || this.options.chartType === "area") {
        this.mainSeries.setData(this.data.map((d) => ({ time: d.time, value: d.close })));
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

      // Re-apply the owner's frame on first load AND on every non-interacted
      // refresh, so a poll landing under the cursor cannot drift the view. Never
      // fitContent() here — it derives candle WIDTH from the data span, so a
      // token with a handful of candles gets stretched into giant bars. If the
      // user HAS taken over the view, leave it untouched.
      if (this._isFirstDataLoad || !this._userHasInteracted) {
        this._applyFrame();
        this._isFirstDataLoad = false;
      }
    }

    // ========================================================================
    // INDICATORS
    // ========================================================================

    /**
     * Add indicator to chart
     * @param {string} type - Indicator type (ema9, ema21)
     */
    addIndicator(type) {
      if (!this.data.length) return;

      const colors = this.theme.indicatorColors;

      switch (type) {
        case "ema9":
          this._addOverlayIndicator("ema9", Indicators.ema(this.data, 9), colors.ema9);
          break;

        case "ema21":
          this._addOverlayIndicator("ema21", Indicators.ema(this.data, 21), colors.ema21);
          break;

        default:
          console.warn(`AdvancedChart: Unknown indicator type: ${type}`);
          return;
      }

      // Track indicator
      if (!this.options.indicators.includes(type)) {
        this.options.indicators.push(type);
      }
    }

    /**
     * Remove indicator from chart
     * @param {string} type - Indicator type to remove
     */
    removeIndicator(type) {
      this._dropIndicatorSeries(type);

      const idx = this.options.indicators.indexOf(type);
      if (idx >= 0) {
        this.options.indicators.splice(idx, 1);
      }
    }

    /** Detach an indicator's series without touching the enabled-indicator list. */
    _dropIndicatorSeries(type) {
      const series = this.indicatorSeries[type];
      if (!series) return;
      if (this.chart) {
        this.chart.removeSeries(series);
      }
      delete this.indicatorSeries[type];
    }

    _addOverlayIndicator(name, data, color) {
      // Re-adding an already-drawn indicator (chart-type switch, theme change)
      // must retire the previous series first. Overwriting the map entry alone
      // orphaned the old line on the chart, where nothing could ever remove it.
      this._dropIndicatorSeries(name);

      const series = this.chart.addLineSeries({
        color,
        lineWidth: 1,
        priceLineVisible: false,
        lastValueVisible: false,
        crosshairMarkerVisible: false,
      });

      series.setData(data.filter((d) => d.value !== null));
      this.indicatorSeries[name] = series;
    }

    _updateIndicators() {
      [...this.options.indicators].forEach((type) => this.addIndicator(type));
    }

    // ========================================================================
    // POSITION MARKERS & REFERENCE LINES
    // ========================================================================

    /**
     * Replace every position marker on the chart in one pass.
     *
     * Bar markers go in a SINGLE setMarkers() call (adding them one at a time
     * re-copied the whole array per marker), and only entries that ask for one
     * get a price line — a dashed line plus an axis label per DCA turns the
     * price scale into an unreadable stack.
     *
     * @param {Array} markers - [{ type: 'entry'|'exit'|'dca', price, timestamp, label, showLine }]
     */
    setPositionMarkers(markers = []) {
      if (!this.mainSeries) return;

      this.positionMarkers.forEach((m) => {
        if (m.priceLine) this.mainSeries.removePriceLine(m.priceLine);
      });
      this.positionMarkers = [];

      const colors = this.theme.positionColors;
      const shapes = { entry: "arrowUp", exit: "arrowDown", dca: "circle" };
      const bars = [];

      markers
        .slice()
        .sort((a, b) => (a.timestamp || 0) - (b.timestamp || 0))
        .forEach((marker) => {
          const color = colors[marker.type] || colors.entry;

          if (marker.timestamp) {
            bars.push({
              time: marker.timestamp,
              position: marker.type === "exit" ? "aboveBar" : "belowBar",
              color,
              shape: shapes[marker.type] || "circle",
              text: marker.label || "",
              size: 1,
            });
          }

          const priceLine = marker.showLine
            ? this.mainSeries.createPriceLine({
                price: marker.price,
                color,
                lineWidth: 1,
                lineStyle: 2, // Dashed
                axisLabelVisible: true,
                title: marker.label || marker.type.toUpperCase(),
              })
            : null;

          this.positionMarkers.push({ ...marker, priceLine });
        });

      this.mainSeries.setMarkers(bars);
    }

    /**
     * Replace every horizontal reference line in one pass.
     * @param {Array} lines - [{ price, color, label, style, lineWidth, showLabel, autoscale }]
     */
    setOverlayLines(lines = []) {
      if (!this.mainSeries) {
        this.overlayLines = [];
        this._autoscalePrices = [];
        return;
      }

      this.overlayLines.forEach((o) => this.mainSeries.removePriceLine(o.line));
      this.overlayLines = [];

      lines.forEach((options) => {
        if (!Number.isFinite(options.price)) return;
        const line = this.mainSeries.createPriceLine({
          price: options.price,
          color: options.color || this.theme.crosshairColor,
          lineWidth: options.lineWidth || 1,
          lineStyle: options.style ?? 0, // 0=solid, 1=dotted, 2=dashed
          axisLabelVisible: options.showLabel !== false,
          title: options.label || "",
        });
        this.overlayLines.push({ line, options });
      });

      this._autoscalePrices = lines.filter((o) => o.autoscale).map((o) => o.price);
      this._invalidateAutoscale();
    }

    // ========================================================================
    // UI UPDATES
    // ========================================================================

    /**
     * Crosshair moves fire far faster than the screen repaints, so renders are
     * coalesced into one animation frame. Each frame does exactly one DOM write
     * pass and one measurement, never a re-parse.
     */
    _scheduleTooltip(param) {
      if (!this.tooltipEl) return;
      this._tooltipParam = param;
      if (this._tooltipFrame) return;
      this._tooltipFrame = requestAnimationFrame(() => {
        this._tooltipFrame = null;
        this._renderTooltip(this._tooltipParam);
      });
    }

    _hideTooltip() {
      if (this._tooltipFrame) {
        cancelAnimationFrame(this._tooltipFrame);
        this._tooltipFrame = null;
      }
      this._tooltipParam = null;
      if (this.tooltipEl) this.tooltipEl.classList.remove("visible");
    }

    _renderTooltip(param) {
      if (!this.tooltipEl) return;

      if (
        !param ||
        param.time === undefined ||
        !param.point ||
        param.point.x < 0 ||
        param.point.y < 0
      ) {
        this._hideTooltip();
        return;
      }

      // O(1) — a linear scan over every candle ran on each pointer sample before.
      const bar = this._barByTime.get(param.time);
      if (!bar) {
        this._hideTooltip();
        return;
      }

      const refs = this.tooltipRefs;
      const delta = bar.close - bar.open;
      const changeClass = delta >= 0 ? "positive" : "negative";
      const changePercent = bar.open ? (delta / bar.open) * 100 : null;
      const rangePercent = bar.low ? ((bar.high - bar.low) / bar.low) * 100 : null;

      refs.date.textContent = this._formatBarTime(bar.time);
      refs.interval.textContent = this._formatBarInterval();
      refs.price.textContent = this._formatPrice(bar.close);
      refs.price.className = `tooltip-price ${changeClass}`;
      refs.change.textContent = this._formatSignedPercent(changePercent);
      refs.change.className = `tooltip-change ${changeClass}`;

      refs.open.textContent = this._formatPrice(bar.open);
      refs.high.textContent = this._formatPrice(bar.high);
      refs.low.textContent = this._formatPrice(bar.low);
      refs.delta.textContent = `${delta >= 0 ? "+" : "-"}${this._formatPrice(Math.abs(delta))}`;
      refs.delta.className = `tooltip-value ${changeClass}`;
      refs.range.textContent = rangePercent === null ? "—" : `${rangePercent.toFixed(2)}%`;
      // Always rendered, including 0: a row that appears and disappears between
      // candles made the card change height under the cursor.
      refs.volume.textContent = this._formatVolume(bar.volume || 0);

      this._syncTooltipRows(refs.indicators, this._indicatorRowCache, this._indicatorRows(param));
      this._syncTooltipRows(
        refs.extra,
        this._extraRowCache,
        typeof this.options.tooltipExtraRows === "function"
          ? this.options.tooltipExtraRows(bar) || []
          : []
      );

      this._positionTooltip(param.point);
      this.tooltipEl.classList.add("visible");
    }

    /** Values of every active indicator at the hovered bar, in series colour. */
    _indicatorRows(param) {
      if (!param.seriesData) return [];

      const rows = [];
      this.options.indicators.forEach((name) => {
        const series = this.indicatorSeries[name];
        if (!series) return;
        const value = param.seriesData.get(series)?.value;
        if (!Number.isFinite(value)) return;
        rows.push({
          label: name.toUpperCase(),
          value: this._formatPrice(value),
          color: series.options?.().color || "",
        });
      });

      return rows;
    }

    /**
     * Reconcile a dynamic row group. The DOM is rebuilt only when the row set
     * itself changes; otherwise this is a textContent write per row.
     */
    _syncTooltipRows(container, cache, rows) {
      const signature = rows.map((row) => row.label).join("|");

      if (cache.signature !== signature) {
        container.textContent = "";
        cache.values = rows.map((row) => this._appendTooltipRow(container, row.label));
        cache.signature = signature;
      }

      rows.forEach((row, idx) => {
        const el = cache.values[idx];
        el.textContent = row.value;
        el.className = `tooltip-value${row.cls ? ` ${row.cls}` : ""}`;
        el.style.color = row.color || "";
      });

      container.classList.toggle("is-empty", rows.length === 0);
    }

    /**
     * Place the card beside the crosshair: flipped to the other side when it
     * would cross the right edge, vertically centred on the cursor, and always
     * clamped inside the chart area so no edge can clip it.
     */
    _positionTooltip(point) {
      const gap = 16;
      const inset = 8;
      const areaWidth = this.chartArea.clientWidth;
      const areaHeight = this.chartArea.clientHeight;
      const width = this.tooltipEl.offsetWidth;
      const height = this.tooltipEl.offsetHeight;

      let x = point.x + gap;
      if (x + width > areaWidth - inset) {
        x = point.x - width - gap;
      }
      x = Math.max(inset, Math.min(x, Math.max(inset, areaWidth - width - inset)));

      let y = point.y - height / 2;
      y = Math.max(inset, Math.min(y, Math.max(inset, areaHeight - height - inset)));

      this.tooltipEl.style.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px)`;
    }

    // ========================================================================
    // EVENT HANDLERS
    // ========================================================================

    _setupEventHandlers() {
      // Crosshair move. The hovered bar is resolved once and handed to the
      // consumer so a surface that mirrors OHLC in its own header never has to
      // scan the data again — or disagree with this tooltip about which bar it is.
      this.chart.subscribeCrosshairMove((param) => {
        this._scheduleTooltip(param);
        if (this.onCrosshairMove) {
          const bar = param?.time === undefined ? null : this._barByTime.get(param.time) || null;
          this.onCrosshairMove(param, bar);
        }
      });

      // Track real view changes. A bare click must NOT count: with decay
      // disabled it would freeze the frame forever, so a drag is only recorded
      // once the pointer actually moves with the button down.
      this.chartArea.addEventListener("wheel", () => this._markUserInteraction());
      this.chartArea.addEventListener("touchstart", () => this._markUserInteraction());
      this.chartArea.addEventListener("mousedown", () => {
        this._maybeDragging = true;
      });
      this.chartArea.addEventListener("mousemove", () => {
        if (!this._maybeDragging) return;
        this._maybeDragging = false;
        this._markUserInteraction();
      });
      this.chartArea.addEventListener("mouseup", () => {
        this._maybeDragging = false;
      });

      // Pointer leaving the chart (mouse out, or a touch ending) hides the card.
      // Touch never fires mouseleave, so the tooltip used to stay pinned over the
      // candles after a tap-drag.
      this.chartArea.addEventListener("mouseleave", () => {
        this._maybeDragging = false;
        this._hideTooltip();
      });
      this.chartArea.addEventListener("touchend", () => this._hideTooltip());
      this.chartArea.addEventListener("touchcancel", () => this._hideTooltip());
    }

    /**
     * Record that the user took over the view. The flag decays back to
     * auto-framing after `interactionDecayMs`; when that option is null the
     * view stays exactly where the user left it until the owner re-anchors.
     */
    _markUserInteraction() {
      this._userHasInteracted = true;

      if (this._interactionTimeout) {
        clearTimeout(this._interactionTimeout);
        this._interactionTimeout = null;
      }

      const decay = this.options.interactionDecayMs;
      if (decay === null || decay === undefined) return;

      this._interactionTimeout = setTimeout(() => {
        this._userHasInteracted = false;
      }, decay);
    }

    /**
     * Hand the view back to the chart's frame (call before re-anchoring).
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

    /**
     * ONE price policy for the whole chart — axis and tooltip — and it is the
     * same function every table and dialog in the dashboard uses, so the same
     * number can never render two ways on one screen.
     */
    _formatPrice(price) {
      return window.Utils.formatPriceSubscript(price, {
        precision: this.options.pricePrecision,
      });
    }

    /** Bar open time; the clock is dropped once bars are a day or wider. */
    _formatBarTime(time) {
      const date = new Date(time * 1000);
      const daily = (this._barSeconds || 0) >= 86400;
      const now = new Date();

      return date.toLocaleString(this.options.locale, {
        month: "short",
        day: "numeric",
        ...(date.getFullYear() === now.getFullYear() ? {} : { year: "numeric" }),
        ...(daily ? {} : { hour: "2-digit", minute: "2-digit" }),
      });
    }

    /** Detected bar interval as a compact label (5m, 4h, 1d). */
    _formatBarInterval() {
      const seconds = this._barSeconds;
      if (!seconds) return "";
      if (seconds < 3600) return `${Math.round(seconds / 60)}m`;
      if (seconds < 86400) return `${Math.round(seconds / 3600)}h`;
      return `${Math.round(seconds / 86400)}d`;
    }

    _formatSignedPercent(percent) {
      if (percent === null || !Number.isFinite(percent)) return "—";
      return `${percent >= 0 ? "+" : "-"}${Math.abs(percent).toFixed(2)}%`;
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

      if (!this.chart) return;

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
        timeScale: { borderColor: this.theme.borderColor },
      });

      // Update series colors
      if (this.mainSeries) {
        if (this.options.chartType === "candlestick" || this.options.chartType === "bar") {
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
        this.volumeSeries.setData(
          this.data.map((d) => ({
            time: d.time,
            value: d.volume,
            color: d.close >= d.open ? this.theme.volumeUpColor : this.theme.volumeDownColor,
          }))
        );
      }

      // Refresh indicators with new colors
      this._updateIndicators();
    }

    /**
     * Set chart type. Position markers and reference lines are re-created on the
     * new series — price lines belong to the series that made them, so removing
     * the old series takes its lines with it.
     * @param {string} type - 'candlestick', 'line', 'area', 'bar'
     */
    setChartType(type) {
      if (type === this.options.chartType) return;

      const markers = this.positionMarkers.map(({ priceLine: _priceLine, ...m }) => m);
      const lines = this.overlayLines.map((o) => o.options);

      if (this.mainSeries) {
        this.chart.removeSeries(this.mainSeries);
      }
      this.positionMarkers = [];
      this.overlayLines = [];

      this.options.chartType = type;
      this._createMainSeries();

      if (this.data.length) {
        if (type === "line" || type === "area") {
          this.mainSeries.setData(this.data.map((d) => ({ time: d.time, value: d.close })));
        } else {
          this.mainSeries.setData(this.data);
        }
      }

      this.setPositionMarkers(markers);
      this.setOverlayLines(lines);
    }

    /**
     * Anchor the view to the newest candle at a FIXED bar width, showing as many
     * candles as fit the pane.
     *
     * This is the deliberate alternative to fitContent(): that derives candle
     * WIDTH from the data span, so a token with only a handful of candles gets a
     * few enormous bars stretched across the pane. anchorLatest keeps candle
     * width CONSTANT no matter how many candles exist.
     */
    anchorLatest() {
      this._frame = { mode: "latest" };
      this._applyFrame();
    }

    /**
     * Frame a specific span — the position chart uses this to show a position's
     * lifetime instead of "the last pane-worth of candles", which for anything
     * older than a few hours contained none of the position's events at all.
     * The frame survives data refreshes until the user takes over the view.
     * @param {number} from - Unix seconds
     * @param {number} to - Unix seconds
     */
    anchorRange(from, to) {
      this._frame = { mode: "range", from, to };
      this._applyFrame();
    }

    _applyFrame() {
      if (!this.chart || !this.data.length) return;
      const ts = this.chart.timeScale();

      if (this._frame.mode === "range") {
        this._applyRangeFrame(ts, this._frame.from, this._frame.to);
        return;
      }

      // Reset any bar spacing a previous range frame applied, then pin the
      // newest candle to the right edge. scrollToRealTime keeps that anchor as
      // live candles arrive.
      ts.applyOptions({
        barSpacing: this.options.barSpacing,
        rightOffset: this.options.rightOffset,
      });
      ts.scrollToRealTime();
    }

    /**
     * Show the bars covering [from, to] plus a margin, in LOGICAL coordinates so
     * a span that starts before the loaded data still frames correctly (the
     * leading whitespace is the honest answer: those candles do not exist here).
     */
    _applyRangeFrame(ts, from, to) {
      const bars = this.data;
      let startIdx = bars.findIndex((b) => b.time >= from);
      if (startIdx < 0) startIdx = bars.length - 1;

      let endIdx = startIdx;
      for (let i = bars.length - 1; i >= startIdx; i--) {
        if (bars[i].time <= to) {
          endIdx = i;
          break;
        }
      }
      if (endIdx < startIdx) endIdx = startIdx;

      const pad = Math.max(2, Math.round((endIdx - startIdx + 1) * 0.08));
      let lo = startIdx - pad;
      let hi = endIdx + pad;

      const shortfall = MIN_FRAME_BARS - (hi - lo + 1);
      if (shortfall > 0) {
        const grow = Math.ceil(shortfall / 2);
        lo -= grow;
        hi += grow;
      }

      ts.setVisibleLogicalRange({ from: lo, to: hi });
    }

    /**
     * Destroy chart and cleanup
     */
    destroy() {
      // Drop any pending tooltip frame before the chart goes away
      this._hideTooltip();
      this.tooltipEl = null;
      this.tooltipRefs = null;

      if (this._interactionTimeout) {
        clearTimeout(this._interactionTimeout);
        this._interactionTimeout = null;
      }

      // Stop observers
      if (this.resizeObserver) {
        this.resizeObserver.disconnect();
        this.resizeObserver = null;
      }

      // Remove chart (this takes every series and price line with it)
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
      this.indicatorSeries = {};
      this.positionMarkers = [];
      this.overlayLines = [];
      this.data = [];
      this.volumeData = [];
      this._barByTime = new Map();
      this._barSeconds = null;
      this._autoscalePrices = [];
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
  window.createAdvancedChart = createAdvancedChart;
})();
