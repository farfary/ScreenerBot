/**
 * Chart Tab Mixin for Position Details Dialog
 *
 * Uses the same advanced lightweight-charts engine as the Token Details
 * overview chart (window.createAdvancedChart), and overlays this position's
 * entries, DCAs and exits as markers + price lines so the chart doubles as a
 * full trade-analysis view. Fills the whole subtab.
 */
import * as Utils from "../../core/utils.js";
import { requestManager } from "../../core/request_manager.js";

export function applyChartTabMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

  const TIMEFRAMES = ["1m", "5m", "15m", "1h", "4h", "1d"];

  // ===========================================================================
  // CHART TAB
  // ===========================================================================

  proto._renderChartTab = async function (content) {
    const pos = this.fullDetails?.position || this.positionData;
    const mint = pos?.mint || this.positionData?.mint;

    if (!mint) {
      content.innerHTML = '<div class="pdd-empty-state">No mint address available</div>';
      return;
    }

    // The dialog's 5s refresh poller re-invokes this on every tick. If the chart
    // is already built and still in the DOM, refresh data/markers/stats IN PLACE
    // instead of tearing it down and redrawing — that flicker + zoom reset was
    // the "keeps refreshing every few seconds" bug.
    if (this._pddChart && this.dialogEl?.querySelector("#pddChart")) {
      await this._loadPositionChartData(mint, this._chartTimeframe, false);
      return;
    }

    // First build (or returning to the tab) — start clean.
    this._destroyPositionChart();

    this._chartTimeframe = this._chartTimeframe || "5m";
    this._pddChartType = this._pddChartType || "candlestick";

    content.innerHTML = `
      <div class="pdd-chart-wrap">
        <div class="chart-container pdd-chart-container">
          <div class="chart-header pdd-chart-header">
            <div class="chart-header-left">
              <span class="chart-title">Price Chart</span>
              <div class="chart-ohlcv-display" id="pddOhlcv">
                <span class="ohlcv-item"><span class="ohlcv-label">O</span> <span class="ohlcv-value" id="pddO">—</span></span>
                <span class="ohlcv-item"><span class="ohlcv-label">H</span> <span class="ohlcv-value" id="pddH">—</span></span>
                <span class="ohlcv-item"><span class="ohlcv-label">L</span> <span class="ohlcv-value" id="pddL">—</span></span>
                <span class="ohlcv-item"><span class="ohlcv-label">C</span> <span class="ohlcv-value" id="pddC">—</span></span>
                <span class="ohlcv-change" id="pddChg">—</span>
              </div>
            </div>
            <div class="chart-controls pdd-chart-controls-row">
              <div class="pdd-chart-type" id="pddChartType">
                <button class="pdd-ct-btn${this._pddChartType === "candlestick" ? " active" : ""}" data-ct="candlestick" title="Candles"><i class="icon-chart-candlestick"></i></button>
                <button class="pdd-ct-btn${this._pddChartType === "line" ? " active" : ""}" data-ct="line" title="Line"><i class="icon-chart-line"></i></button>
                <button class="pdd-ct-btn${this._pddChartType === "area" ? " active" : ""}" data-ct="area" title="Area"><i class="icon-chart-area"></i></button>
              </div>
              <button class="pdd-ema-toggle${this._pddEma ? " active" : ""}" id="pddEmaToggle" title="Toggle EMA 9/21">EMA</button>
              <div class="timeframe-buttons" id="pddTimeframes">
                ${TIMEFRAMES.map(
                  (tf) =>
                    `<button class="timeframe-btn${tf === this._chartTimeframe ? " active" : ""}" data-tf="${tf}">${tf.toUpperCase()}</button>`
                ).join("")}
              </div>
            </div>
          </div>
          <div id="pddChart" class="tradingview-chart pdd-chart-canvas"></div>
          <div class="pdd-chart-legend" id="pddChartLegend">
            <span class="pdd-legend-item"><span class="pdd-legend-dot entry"></span> Entry</span>
            <span class="pdd-legend-item"><span class="pdd-legend-dot dca"></span> DCA</span>
            <span class="pdd-legend-item"><span class="pdd-legend-dot exit"></span> Exit</span>
            <span class="pdd-legend-item"><span class="pdd-legend-line avg"></span> Avg Entry</span>
          </div>
          <div id="pddChartLoading" class="chart-loading-overlay">
            <div class="chart-loading-content">
              <div class="chart-loading-spinner"></div>
              <div class="chart-loading-text">Loading chart data...</div>
            </div>
          </div>
        </div>
        <div class="pdd-chart-posbar" id="pddPosBar"></div>
      </div>
    `;

    this._renderPositionChartStats();
    await this._initPositionChart(mint);
  };

  /** Create the advanced chart and wire controls. */
  proto._initPositionChart = async function (mint) {
    const container = this.dialogEl?.querySelector("#pddChart");
    if (!container) return;

    if (!window.createAdvancedChart) {
      container.innerHTML =
        '<div class="pdd-chart-empty"><i class="icon-circle-alert"></i><p>Chart engine unavailable</p></div>';
      return;
    }

    const isDark = document.documentElement.getAttribute("data-theme") !== "light";
    const pos = this.fullDetails?.position || this.positionData || {};

    this._pddChart = window.createAdvancedChart(container, {
      theme: isDark ? "dark" : "light",
      chartType: this._pddChartType,
      showVolume: true,
      showGrid: true,
      showCrosshair: true,
      showLegend: false,
      showTooltip: true,
      barSpacing: 10,
      minBarSpacing: 3,
      // This chart belongs to a position, so the hovered bar answers the only
      // question that matters here: what the position was worth at that price.
      tooltipExtraRows: (bar) => this._positionTooltipRows(bar),
      watermark: {
        text: pos.symbol || "",
        fontSize: 34,
        color: isDark ? "rgba(128,128,128,0.10)" : "rgba(128,128,128,0.08)",
      },
    });

    // Header O/H/L/C follows the crosshair, falling back to the newest candle.
    this._pddChart.onCrosshairMove = (_param, bar) => {
      this._updatePddOhlc(bar || this._pddLatestCandle);
    };

    await this._loadPositionChartData(mint, this._chartTimeframe, true);

    // Timeframe buttons
    const tfWrap = this.dialogEl.querySelector("#pddTimeframes");
    this._pddTfHandler = async (e) => {
      const btn = e.target.closest(".timeframe-btn");
      if (!btn || btn.dataset.tf === this._chartTimeframe) return;
      tfWrap.querySelectorAll(".timeframe-btn").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      this._chartTimeframe = btn.dataset.tf;
      await this._loadPositionChartData(mint, this._chartTimeframe, true);
    };
    tfWrap?.addEventListener("click", this._pddTfHandler);

    // Chart-type toggle
    const ctWrap = this.dialogEl.querySelector("#pddChartType");
    this._pddCtHandler = (e) => {
      const btn = e.target.closest(".pdd-ct-btn");
      if (!btn || !this._pddChart) return;
      ctWrap.querySelectorAll(".pdd-ct-btn").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      this._pddChartType = btn.dataset.ct;
      this._pddChart.setChartType(this._pddChartType);
      this._updatePositionChartMarkers();
      if (this._pddEma) {
        this._pddChart.addIndicator("ema9");
        this._pddChart.addIndicator("ema21");
      }
    };
    ctWrap?.addEventListener("click", this._pddCtHandler);

    // EMA indicator toggle
    const emaBtn = this.dialogEl.querySelector("#pddEmaToggle");
    this._pddEmaHandler = () => {
      if (!this._pddChart) return;
      this._pddEma = !this._pddEma;
      emaBtn.classList.toggle("active", this._pddEma);
      if (this._pddEma) {
        this._pddChart.addIndicator("ema9");
        this._pddChart.addIndicator("ema21");
      } else {
        this._pddChart.removeIndicator("ema9");
        this._pddChart.removeIndicator("ema21");
      }
    };
    emaBtn?.addEventListener("click", this._pddEmaHandler);
    if (this._pddEma) {
      this._pddChart.addIndicator("ema9");
      this._pddChart.addIndicator("ema21");
    }

    // Theme sync
    this._pddThemeObserver = new MutationObserver(() => {
      const t = document.documentElement.getAttribute("data-theme") === "light" ? "light" : "dark";
      this._pddChart?.setTheme(t);
    });
    this._pddThemeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
    // Live updates are driven by the dialog's existing 5s refresh poller, which
    // re-enters _renderChartTab and refreshes this chart in place. No separate
    // interval here (that caused double refreshes).
  };

  /** Fetch OHLCV and push into the chart. */
  proto._loadPositionChartData = async function (mint, timeframe, isInitial) {
    const overlay = this.dialogEl?.querySelector("#pddChartLoading");
    const loadingText = overlay?.querySelector(".chart-loading-text");

    try {
      const data = await requestManager.fetch(`/api/tokens/${mint}/ohlcv?timeframe=${timeframe}`, {
        priority: isInitial ? "high" : "normal",
      });

      if (!Array.isArray(data) || data.length === 0) {
        if (loadingText) loadingText.textContent = "Waiting for chart data...";
        overlay?.classList.remove("hidden");
        return;
      }
      if (!this._pddChart) return;

      const chartData = data.map((c) => ({
        time: c.timestamp,
        open: c.open,
        high: c.high,
        low: c.low,
        close: c.close,
        volume: c.volume || 0,
      }));

      this._pddChart.setData(chartData);
      this._pddChartData = chartData;
      overlay?.classList.add("hidden");

      this._pddLatestCandle = chartData[chartData.length - 1];
      this._updatePddOhlc(this._pddLatestCandle);
      this._updatePositionChartMarkers();
      this._renderPositionChartStats(chartData[chartData.length - 1]?.close);

      if (isInitial) {
        // Anchor the newest candle at a fixed bar width so candle size stays
        // normal for any candle count (no giant stretched bars with few candles).
        this._pddChart.resetUserInteraction();
        this._pddChart.anchorLatest();
      }
    } catch {
      if (loadingText) loadingText.textContent = "Waiting for chart data...";
      overlay?.classList.remove("hidden");
    }
  };

  /** Overlay this position's entries / DCAs / exits + key price lines. */
  proto._updatePositionChartMarkers = function () {
    if (!this._pddChart) return;
    const pos = this.fullDetails?.position || this.positionData || {};
    const entries = this.fullDetails?.entries || [];
    const exits = this.fullDetails?.exits || [];

    this._pddChart.clearPositionMarkers();
    this._pddChart.clearOverlays();

    // lightweight-charts only renders a marker when its `time` matches a bar, so
    // snap each event to the candle that contains it (largest bar time <= event).
    // Events outside the loaded window are skipped (switch timeframe to see them).
    const bars = this._pddChartData || [];
    const firstBar = bars.length ? bars[0].time : null;
    const lastBar = bars.length ? bars[bars.length - 1].time : null;
    const snapToBar = (ts) => {
      if (firstBar === null || ts < firstBar) return null;
      if (ts >= lastBar) return lastBar;
      let chosen = firstBar;
      for (let i = 0; i < bars.length; i++) {
        if (bars[i].time <= ts) chosen = bars[i].time;
        else break;
      }
      return chosen;
    };

    // Build one combined list so markers can be added in strict ascending time
    // order — lightweight-charts requires that, and interleaved DCA/exit events
    // would otherwise be dropped or misplaced.
    const markers = [];
    let dcaIdx = 0;
    entries.forEach((e) => {
      if (!e.price || !e.timestamp) return;
      const isDca = !!e.is_dca;
      if (isDca) dcaIdx += 1;
      const barTime = snapToBar(Number(e.timestamp));
      if (barTime === null) return;
      markers.push({
        type: isDca ? "dca" : "entry",
        price: e.price,
        timestamp: barTime,
        label: isDca ? `DCA ${dcaIdx}` : "Entry",
      });
    });
    exits.forEach((e, i) => {
      if (!e.price || !e.timestamp) return;
      const barTime = snapToBar(Number(e.timestamp));
      if (barTime === null) return;
      markers.push({
        type: "exit",
        price: e.price,
        timestamp: barTime,
        label: exits.length > 1 ? `Exit ${i + 1}` : "Exit",
      });
    });

    markers
      .sort((a, b) => a.timestamp - b.timestamp)
      .forEach((m) => this._pddChart.addPositionMarker(m));

    // Average entry reference line.
    const avgEntry = pos.average_entry_price || pos.entry_price;
    if (avgEntry) {
      this._pddChart.addHorizontalLine({
        price: avgEntry,
        color: "#3b82f6",
        label: "Avg Entry",
        style: 2,
      });
    }
  };

  /**
   * Extra tooltip rows: what this position looked like at the hovered bar.
   * Returns nothing when there is no average entry to compare against — an
   * unanswerable row is worse than no row.
   */
  proto._positionTooltipRows = function (bar) {
    const pos = this.fullDetails?.position || this.positionData || {};
    const avgEntry = pos.average_entry_price || pos.entry_price;
    if (!avgEntry || !bar?.close) return [];

    const pnlPct = ((bar.close - avgEntry) / avgEntry) * 100;
    return [
      { label: "Avg Entry", value: this._formatPrice(avgEntry) },
      {
        label: "P&L @ Bar",
        value: `${pnlPct >= 0 ? "+" : "-"}${Math.abs(pnlPct).toFixed(2)}%`,
        cls: pnlPct >= 0 ? "positive" : "negative",
      },
    ];
  };

  /** Update the O/H/L/C header from one candle (hovered bar, or the latest). */
  proto._updatePddOhlc = function (last) {
    if (!last) return;
    const set = (id, v) => {
      const el = this.dialogEl?.querySelector(id);
      // Subscript notation (0.0₅1311), consistent with the chart axis/tooltip.
      if (el) el.textContent = Utils.formatPriceSubscript(v, { precision: 5 });
    };
    set("#pddO", last.open);
    set("#pddH", last.high);
    set("#pddL", last.low);
    set("#pddC", last.close);

    const chg = this.dialogEl?.querySelector("#pddChg");
    if (chg && last.open) {
      const pct = ((last.close - last.open) / last.open) * 100;
      chg.textContent = `${pct >= 0 ? "+" : ""}${pct.toFixed(2)}%`;
      chg.className = `ohlcv-change ${pct >= 0 ? "positive" : "negative"}`;
    }
  };

  /** Position summary bar under the chart (avg entry, current, PnL, counts). */
  proto._renderPositionChartStats = function (livePrice) {
    const bar = this.dialogEl?.querySelector("#pddPosBar");
    if (!bar) return;
    const pos = this.fullDetails?.position || this.positionData || {};
    const entries = this.fullDetails?.entries || [];
    const exits = this.fullDetails?.exits || [];

    const avgEntry = pos.average_entry_price || pos.entry_price;
    const current = livePrice || pos.current_price;
    const dcaCount = entries.filter((e) => e.is_dca).length;

    let pnlPct = pos.unrealized_pnl_percent ?? pos.pnl_percent;
    if ((pnlPct === null || pnlPct === undefined) && avgEntry && current) {
      pnlPct = ((current - avgEntry) / avgEntry) * 100;
    }
    const pnlClass =
      pnlPct === null || pnlPct === undefined ? "" : pnlPct >= 0 ? "pdd-positive" : "pdd-negative";
    const pnlText =
      pnlPct === null || pnlPct === undefined
        ? "—"
        : `${pnlPct >= 0 ? "+" : ""}${Utils.formatNumber(pnlPct, 2)}%`;

    const cell = (label, value, cls = "") =>
      `<div class="pdd-posbar-cell"><span class="label">${label}</span><span class="value ${cls}">${value}</span></div>`;

    bar.innerHTML = `
      ${cell("Avg Entry", avgEntry ? `${this._formatPrice(avgEntry)} SOL` : "—")}
      ${cell("Current", current ? `${this._formatPrice(current)} SOL` : "—")}
      ${cell("PnL", pnlText, pnlClass)}
      ${cell("Entries", String(entries.length))}
      ${cell("DCAs", String(dcaCount))}
      ${cell("Exits", String(exits.length))}
    `;
  };

  /** Tear down chart, poller and observers. */
  proto._destroyPositionChart = function () {
    if (this._pddChartPoll) {
      clearInterval(this._pddChartPoll);
      this._pddChartPoll = null;
    }
    if (this._pddThemeObserver) {
      this._pddThemeObserver.disconnect();
      this._pddThemeObserver = null;
    }
    if (this._pddChart) {
      try {
        this._pddChart.destroy();
      } catch {
        /* already gone */
      }
      this._pddChart = null;
    }
    this._pddTfHandler = null;
    this._pddCtHandler = null;
    this._pddEmaHandler = null;
  };
}
