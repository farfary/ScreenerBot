/**
 * Chart Tab Mixin for Position Details Dialog
 *
 * Uses the same advanced lightweight-charts engine as the Token Details
 * overview chart (window.createAdvancedChart), and overlays this position's
 * entries, DCAs and exits as markers so the chart doubles as a full
 * trade-analysis view.
 *
 * The defining difference from the token chart: this one is framed on the
 * POSITION, not on "now". It loads the full candle history, opens on a
 * timeframe chosen from the position's duration, frames entry→exit, keeps the
 * average entry inside the price scale, and never drags the view back to the
 * newest candle behind the user's back.
 */
import * as Utils from "../../core/utils.js";
import {
  CHART_TIMEFRAMES,
  fetchCandles,
  fetchOhlcvStatus,
  findTimeframeWithData,
  renderOhlcvStatus,
  timeframeForSpan,
  triggerRefresh,
} from "../chart_data.js";

// Candles are refetched on the dialog's own refresh tick, but they move far slower than
// the position figures do: the finest timeframe is 1m and every read pulls the FULL
// stored history. Throttling here decouples the two, so tightening the dialog's cadence
// (which it does while a swap is confirming) cannot turn into a candle refetch per tick.
const CHART_REFRESH_MS = 10000;

export function applyChartTabMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

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

    // The dialog's refresh poller re-invokes this on every tick. If the chart is
    // already built and still bound to the LIVE container, refresh data, markers
    // and stats in place instead of tearing it down and redrawing — that flicker +
    // view reset was the "keeps refreshing every few seconds" bug. Checking the
    // bound node (not just the id) matters: a re-render swaps the container out and
    // leaves the chart drawing into a detached node.
    const liveContainer = this.dialogEl?.querySelector("#pddChart");
    if (this._pddChart && liveContainer && this._pddChart.container === liveContainer) {
      const now = Date.now();
      if (now - (this._pddDataAt || 0) > CHART_REFRESH_MS) {
        await this._loadPositionChartData(mint, this._chartTimeframe, false);
      } else {
        // Markers still track the position, which CAN change on any tick (a verified
        // DCA adds an entry) even when the candles are not refetched.
        this._updatePositionChartMarkers();
        this._renderPositionChartStats();
      }
      // The status chip changes far slower still; refreshing it on every tick would
      // spend a request per tick to redraw the same chip.
      if (now - (this._pddIndicatorAt || 0) > 30000) {
        this._updatePositionDataIndicator(mint);
      }
      return;
    }

    // First build (or returning to the tab) — start clean.
    this._destroyPositionChart();

    // Open on a timeframe that renders THIS position as a readable number of
    // candles. A fixed 5m default put a week-old entry thousands of candles off
    // the left edge of the view.
    this._chartTimeframe = this._chartTimeframe || timeframeForSpan(this._positionSpanSeconds());
    this._pddChartType = this._pddChartType || "candlestick";

    content.innerHTML = `
      <div class="pdd-chart-wrap">
        <div class="chart-container pdd-chart-container">
          <div class="chart-header pdd-chart-header">
            <div class="chart-header-left">
              <div class="chart-data-indicator" id="pddDataIndicator" tabindex="0" role="status">
                <span class="chart-data-dot"></span>
                <span class="chart-data-label">Data</span>
                <div class="chart-data-tip" id="pddDataTip"></div>
              </div>
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
              <button class="pdd-chart-toggle${this._pddEma ? " active" : ""}" id="pddEmaToggle" title="Toggle EMA 9/21">EMA</button>
              <button class="pdd-chart-toggle" id="pddFrameBtn" title="Frame this position's lifetime">FIT</button>
              <div class="timeframe-buttons" id="pddTimeframes">
                ${CHART_TIMEFRAMES.map(
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
            <span class="pdd-legend-note" id="pddChartNote"></span>
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
      showTooltip: true,
      barSpacing: 10,
      minBarSpacing: 3,
      // This chart is read backwards in time on purpose. Snapping the view back
      // to the newest candle 30s after the user panned to the entry made the
      // position impossible to study.
      interactionDecayMs: null,
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

    // Record the frame before the first candles land so the initial paint is
    // already on the position rather than on the newest bars.
    this._framePosition();

    await this._loadPositionChartData(mint, this._chartTimeframe, true);
    this._updatePositionDataIndicator(mint);

    // Timeframe buttons
    const tfWrap = this.dialogEl.querySelector("#pddTimeframes");
    tfWrap?.addEventListener("click", async (e) => {
      const btn = e.target.closest(".timeframe-btn");
      if (!btn || btn.dataset.tf === this._chartTimeframe) return;
      tfWrap.querySelectorAll(".timeframe-btn").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      this._chartTimeframe = btn.dataset.tf;
      // Kick the backend before reading: a timeframe that has never been
      // collected otherwise sits on "Waiting for chart data" until ordinary
      // monitoring cadence gets around to it.
      triggerRefresh(mint);
      await this._loadPositionChartData(mint, this._chartTimeframe, true);
      this._updatePositionDataIndicator(mint);
    });

    // Chart-type toggle
    const ctWrap = this.dialogEl.querySelector("#pddChartType");
    ctWrap?.addEventListener("click", (e) => {
      const btn = e.target.closest(".pdd-ct-btn");
      if (!btn || !this._pddChart) return;
      ctWrap.querySelectorAll(".pdd-ct-btn").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      this._pddChartType = btn.dataset.ct;
      // setChartType carries markers, reference lines and indicators across to
      // the new series itself.
      this._pddChart.setChartType(this._pddChartType);
    });

    // EMA indicator toggle
    const emaBtn = this.dialogEl.querySelector("#pddEmaToggle");
    emaBtn?.addEventListener("click", () => {
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
    });
    if (this._pddEma) {
      this._pddChart.addIndicator("ema9");
      this._pddChart.addIndicator("ema21");
    }

    // Re-frame the position after the user has panned away.
    this.dialogEl.querySelector("#pddFrameBtn")?.addEventListener("click", () => {
      this._framePosition();
    });

    // Theme sync
    this._pddThemeObserver = new MutationObserver(() => {
      if (!this._pddChart) return;
      const t = document.documentElement.getAttribute("data-theme") === "light" ? "light" : "dark";
      this._pddChart.setTheme(t);
      // Marker and reference-line colours come from the theme, and setTheme only
      // repaints the series it owns — force the overlay to be rebuilt in the new
      // palette rather than leaving dark-theme colours on a light chart.
      this._pddMarkerSignature = null;
      this._updatePositionChartMarkers();
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
    // Sequence guard: OHLCV reads are slow enough to resolve after the user
    // switched timeframe, which painted the previous timeframe's candles over
    // the current chart.
    const seq = (this._pddLoadSeq = (this._pddLoadSeq || 0) + 1);
    // Stamped here, not at the call site, so every entry point (first build, timeframe
    // switch, empty-timeframe fallback) feeds the poll-path throttle.
    this._pddDataAt = Date.now();

    try {
      const chartData = await fetchCandles(mint, timeframe, {
        priority: isInitial ? "high" : "normal",
      });

      if (seq !== this._pddLoadSeq || !this._pddChart) return;
      if (this._chartTimeframe !== timeframe) return;

      if (!chartData.length) {
        // This timeframe has no candles. The token can still have data at
        // another (only daily fetched so far, say) — on the first load, switch
        // to one that does instead of sitting on an empty "Waiting…" chart.
        // Runs once per open and never overrides a manual timeframe choice.
        if (isInitial && !this._pddFallbackTried) {
          this._pddFallbackTried = true;
          const altTf = await findTimeframeWithData(mint, timeframe);
          if (altTf && altTf !== timeframe && this._chartTimeframe === timeframe) {
            this._chartTimeframe = altTf;
            this._syncPddTimeframeButtons(altTf);
            await this._loadPositionChartData(mint, altTf, true);
            return;
          }
        }
        this._showPositionChartOverlay(timeframe, "Waiting for chart data...");
        return;
      }

      this._pddChart.setData(chartData);
      this._pddChartData = chartData;
      this._pddRenderedTf = timeframe;
      this._hidePositionChartOverlay();

      this._pddLatestCandle = chartData[chartData.length - 1];
      this._updatePddOhlc(this._pddLatestCandle);
      this._updatePositionChartMarkers();
      this._renderPositionChartStats();

      if (isInitial) {
        this._framePosition();
      }
    } catch {
      if (seq !== this._pddLoadSeq) return;
      this._showPositionChartOverlay(timeframe, "Waiting for chart data...");
    }
  };

  /**
   * The loading overlay is a full-cover panel, so it may only appear when the
   * canvas underneath is not already showing THIS timeframe. Re-showing it on a
   * single dropped poll response hid a perfectly good chart behind a spinner;
   * suppressing it unconditionally left the previous timeframe's candles under
   * the newly-selected timeframe button.
   */
  proto._showPositionChartOverlay = function (timeframe, text) {
    if (this._pddRenderedTf === timeframe && this._pddChartData?.length) return;
    const overlay = this.dialogEl?.querySelector("#pddChartLoading");
    const loadingText = overlay?.querySelector(".chart-loading-text");
    if (loadingText) loadingText.textContent = text;
    overlay?.classList.remove("hidden");
  };

  proto._hidePositionChartOverlay = function () {
    this.dialogEl?.querySelector("#pddChartLoading")?.classList.add("hidden");
  };

  proto._syncPddTimeframeButtons = function (tf) {
    this.dialogEl
      ?.querySelector("#pddTimeframes")
      ?.querySelectorAll(".timeframe-btn")
      .forEach((b) => b.classList.toggle("active", b.dataset.tf === tf));
  };

  /** Refresh the per-timeframe data-status chip. Never throws. */
  proto._updatePositionDataIndicator = async function (mint) {
    const indicator = this.dialogEl?.querySelector("#pddDataIndicator");
    if (!indicator) return;
    this._pddIndicatorAt = Date.now();
    const status = await fetchOhlcvStatus(mint);
    if (!status || !this.dialogEl?.contains(indicator)) return;
    renderOhlcvStatus(
      { indicator, tip: this.dialogEl.querySelector("#pddDataTip") },
      status,
      Utils.formatTimeAgo
    );
  };

  // ===========================================================================
  // POSITION OVERLAY
  // ===========================================================================

  /** First and last moment this position was alive, in unix seconds. */
  proto._positionSpan = function () {
    const pos = this.fullDetails?.position || this.positionData || {};
    const entries = this.fullDetails?.entries || [];
    const exits = this.fullDetails?.exits || [];
    const now = Math.floor(Date.now() / 1000);

    const stamps = [
      ...entries.map((e) => Number(e.timestamp)),
      ...exits.map((e) => Number(e.timestamp)),
      Number(pos.entry_time) || 0,
    ].filter((t) => Number.isFinite(t) && t > 0);

    const from = stamps.length ? Math.min(...stamps) : now - 3600;
    const to = pos.position_type === "closed" ? Number(pos.exit_time) || Math.max(...stamps) : now;
    return { from, to: Math.max(to, from + 60) };
  };

  proto._positionSpanSeconds = function () {
    const { from, to } = this._positionSpan();
    return to - from;
  };

  /**
   * Point the view at the position's lifetime and hand the frame back to it.
   * Safe to call before any candles have landed: the chart records the frame
   * and applies it when data arrives, so the first paint is already correct
   * instead of flashing the newest candles first.
   */
  proto._framePosition = function () {
    if (!this._pddChart) return;
    const { from, to } = this._positionSpan();
    this._pddChart.resetUserInteraction();
    this._pddChart.anchorRange(from, to);
  };

  /**
   * Overlay this position's entries / DCAs / exits + the average entry line.
   *
   * Rebuilt only when the events or the loaded bar window actually changed —
   * the dialog polls every 5s and recreating every price line on each tick both
   * flickered and cost more than the whole redraw.
   */
  proto._updatePositionChartMarkers = function () {
    if (!this._pddChart) return;
    const pos = this.fullDetails?.position || this.positionData || {};
    const entries = this.fullDetails?.entries || [];
    const exits = this.fullDetails?.exits || [];
    const bars = this._pddChartData || [];
    if (!bars.length) return;

    const avgEntry = pos.average_entry_price || pos.entry_price;
    const signature = JSON.stringify([
      bars[0].time,
      bars[bars.length - 1].time,
      bars.length,
      avgEntry,
      entries.map((e) => [e.timestamp, e.price, e.is_dca]),
      exits.map((e) => [e.timestamp, e.price]),
    ]);
    if (signature === this._pddMarkerSignature) return;
    this._pddMarkerSignature = signature;

    // lightweight-charts only renders a marker when its `time` matches a bar, so
    // snap each event to the candle that CONTAINS it. Events outside the loaded
    // window are counted, not faked: clamping a sell onto the last bar claimed
    // it happened in a candle it did not.
    const firstBar = bars[0].time;
    const lastBar = bars[bars.length - 1].time;
    const barSeconds = bars.length > 1 ? bars[1].time - bars[0].time : 60;
    let dropped = 0;

    const snapToBar = (ts) => {
      if (!Number.isFinite(ts) || ts < firstBar || ts > lastBar + barSeconds) {
        dropped += 1;
        return null;
      }
      if (ts >= lastBar) return lastBar;
      // Bars are ascending: binary-search the last one at or before the event.
      let lo = 0;
      let hi = bars.length - 1;
      while (lo < hi) {
        const mid = Math.ceil((lo + hi) / 2);
        if (bars[mid].time <= ts) lo = mid;
        else hi = mid - 1;
      }
      return bars[lo].time;
    };

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

    // Bar markers only. One dashed price line per DCA and partial exit turned
    // the price scale into an unreadable stack of overlapping axis labels; the
    // average entry is the single level worth drawing.
    this._pddChart.setPositionMarkers(markers);
    this._pddChart.setOverlayLines(
      avgEntry
        ? [
            {
              price: avgEntry,
              color: this._pddChart.theme.positionColors.avgEntry,
              label: "Avg Entry",
              style: 2,
              // Keep the level on screen: price lines do not extend the price
              // scale on their own, so a deeply red or green position drew its
              // entry off-pane and the chart looked position-less.
              autoscale: true,
            },
          ]
        : []
    );

    const note = this.dialogEl?.querySelector("#pddChartNote");
    if (note) {
      note.textContent = dropped
        ? `${dropped} event${dropped > 1 ? "s" : ""} outside this range`
        : "";
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
    if (!chg) return;
    if (!last.open) {
      chg.textContent = "—";
      chg.className = "ohlcv-change";
      return;
    }
    const pct = ((last.close - last.open) / last.open) * 100;
    chg.textContent = `${pct >= 0 ? "+" : ""}${pct.toFixed(2)}%`;
    chg.className = `ohlcv-change ${pct >= 0 ? "positive" : "negative"}`;
  };

  /**
   * Position summary bar under the chart (avg entry, current, PnL, counts).
   *
   * Every price here is the POOL price the trader actually acts on. Feeding it
   * the newest OHLCV candle close instead made this bar disagree with the
   * dialog header and, when the backend had no unrealized P&L yet, derived a
   * P&L number from the wrong price system entirely.
   */
  proto._renderPositionChartStats = function () {
    const bar = this.dialogEl?.querySelector("#pddPosBar");
    if (!bar) return;
    const pos = this.fullDetails?.position || this.positionData || {};
    const entries = this.fullDetails?.entries || [];
    const exits = this.fullDetails?.exits || [];

    const avgEntry = pos.average_entry_price || pos.entry_price;
    const current = pos.current_price;
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

  /** Tear down chart and observers. */
  proto._destroyPositionChart = function () {
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
    this._pddChartData = null;
    this._pddRenderedTf = null;
    this._pddLatestCandle = null;
    this._pddMarkerSignature = null;
    this._pddFallbackTried = false;
    this._pddIndicatorAt = 0;
    this._pddDataAt = 0;
  };
}
