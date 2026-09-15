/**
 * Chart section of the Position Details dialog.
 *
 * Uses the same advanced lightweight-charts engine as the Token Details chart
 * (window.createAdvancedChart) and overlays this position's entries, DCAs and exits as
 * markers, so the chart doubles as the trade-analysis view.
 *
 * The defining difference from the token chart: this one is framed on the POSITION, not on
 * "now". It loads the full candle history, opens on a timeframe chosen from the position's
 * duration, frames entry→exit, keeps the average entry inside the price scale, and never drags
 * the view back to the newest candle behind the user's back.
 */
import { Poller } from "../../core/poller.js";
import * as Utils from "../../core/utils.js";
import {
  CHART_TIMEFRAMES,
  barForTimestamp,
  fetchCandles,
  fetchOhlcvStatus,
  renderOhlcvStatus,
  timeframeCoveringSpan,
  timeframeForSpan,
  timeframeSeconds,
  triggerRefresh,
} from "../chart_data.js";

// The chart polls candles on its OWN cadence. It used to ride the dialog's details poll, and a
// closed or archived position has no details poll — so a token opened before its candles were
// collected said "No chart data" forever while the backend filled it within seconds.
const WAITING_POLL_MS = 3000;
// A token that stays empty this many polls is probably not getting data soon; stop asking
// every 3s but keep asking.
const WAITING_POLLS_BEFORE_BACKOFF = 6;
const EMPTY_POLL_MS = 15000;
// Candles move far slower than position figures, and every read pulls the full history.
const OPEN_POLL_MS = 10000;
const SETTLED_POLL_MS = 30000;

const CHART_TYPES = [
  ["candlestick", "Candles"],
  ["line", "Line"],
  ["area", "Area"],
];

export function applyChartMixin(PositionDetailsDialog) {
  const proto = PositionDetailsDialog.prototype;

  /**
   * Build the chart on the first details response; every later one only refreshes the markers
   * (candles have their own poller). Rebuilding per poll flickered and reset the user's view.
   * Checking the bound node, not just the id, matters: a repaint that swapped the container
   * would leave the chart drawing into a detached node.
   */
  proto._renderChart = async function () {
    const section = this.dialogEl?.querySelector("#pddChartSection");
    const mint = this._position()?.mint;
    if (!section || !mint) return;

    const liveContainer = section.querySelector("#pddChart");
    if (this._pddChart && liveContainer && this._pddChart.container === liveContainer) {
      // Markers track the position, which CAN change on any tick (a verified DCA adds an entry).
      this._updatePositionChartMarkers();
      return;
    }

    this._destroyPositionChart();
    // Start on a timeframe that renders THIS position as a readable number of candles (a fixed
    // 5m default put a week-old entry thousands of candles off the left edge); the first
    // status read may move it to one whose stored candles still cover the position.
    if (!this._chartTimeframe) {
      this._chartTimeframe = timeframeForSpan(this._positionSpanSeconds());
      this._pddTfAuto = true;
    }
    this._pddChartType = this._pddChartType || "candlestick";

    const segment = (attrs, label, active) =>
      `<button type="button" class="timeframe-btn${active ? " active" : ""}" ${attrs} aria-pressed="${active}">${label}</button>`;

    section.innerHTML = `
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
          <div class="chart-controls pdd-chart-controls">
            <div class="timeframe-buttons" id="pddChartType" role="group" aria-label="Chart type">
              ${CHART_TYPES.map(([id, label]) => segment(`data-ct="${id}"`, label, id === this._pddChartType)).join("")}
            </div>
            <div class="timeframe-buttons" role="group" aria-label="Chart overlays">
              ${segment('id="pddEmaToggle" title="Exponential moving averages, 9 and 21"', "EMA", Boolean(this._pddEma))}
              <button type="button" class="timeframe-btn" id="pddFrameBtn" title="Frame this position's lifetime">Fit</button>
            </div>
            <div class="timeframe-buttons" id="pddTimeframes" role="group" aria-label="Timeframe">
              ${CHART_TIMEFRAMES.map((tf) => segment(`data-tf="${tf}"`, tf.toUpperCase(), tf === this._chartTimeframe)).join("")}
            </div>
            <div class="timeframe-buttons pdd-pane-group" role="group" aria-label="Chart pane">
              <button type="button" class="timeframe-btn pdd-pane-btn" id="pddChartFocusBtn" aria-controls="pddActivity" aria-pressed="false" title="Expand chart" aria-label="Expand chart"><i class="icon-maximize-2"></i></button>
            </div>
          </div>
        </div>
        <div id="pddChart" class="tradingview-chart pdd-chart-canvas"></div>
        <div class="pdd-chart-legend">
          <span class="pdd-legend-items" id="pddLegendItems"></span>
          <span class="pdd-legend-note" id="pddChartNote"></span>
        </div>
        <div id="pddChartLoading" class="chart-loading-overlay">
          <div class="chart-loading-content">
            <div class="chart-loading-spinner"></div>
            <div class="chart-loading-text">Loading chart data...</div>
          </div>
        </div>
      </div>
    `;

    await this._initPositionChart(mint);
  };

  /** Create the advanced chart and wire its controls. */
  proto._initPositionChart = async function (mint) {
    const container = this.dialogEl?.querySelector("#pddChart");
    if (!container) return;

    if (!window.createAdvancedChart) {
      container.innerHTML =
        '<div class="pdd-chart-empty"><i class="icon-circle-alert"></i><p>Chart engine unavailable</p></div>';
      return;
    }

    const isDark = document.documentElement.getAttribute("data-theme") !== "light";
    const pos = this._position() || {};

    this._pddChart = window.createAdvancedChart(container, {
      theme: isDark ? "dark" : "light",
      chartType: this._pddChartType,
      showVolume: true,
      showGrid: true,
      showCrosshair: true,
      showTooltip: true,
      barSpacing: 10,
      minBarSpacing: 3,
      // This chart is read backwards in time on purpose. Snapping the view back to the newest
      // candle 30s after the user panned to the entry made the position impossible to study.
      interactionDecayMs: null,
      // The hovered bar answers the question that matters here: what the position was worth
      // at that price.
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
    // A click restores an activity-focused layout or finds a marked event (panes.js).
    this._pddChart.onClick = (param) => this._onPositionChartClick(param);
    this._syncPaneControls();

    // Record the frame before the first candles land so the initial paint is already on the
    // position rather than on the newest bars.
    this._framePosition();
    this._renderChartLegend();

    await this._refreshPositionChart(mint);
    this._startPositionChartPoller(mint);

    const setPressed = (group, active) => {
      group.querySelectorAll(".timeframe-btn").forEach((b) => {
        b.classList.toggle("active", b === active);
        b.setAttribute("aria-pressed", String(b === active));
      });
    };

    const tfWrap = this.dialogEl?.querySelector("#pddTimeframes");
    tfWrap?.addEventListener("click", async (e) => {
      const btn = e.target.closest(".timeframe-btn");
      if (!btn || btn.dataset.tf === this._chartTimeframe) return;
      setPressed(tfWrap, btn);
      this._chartTimeframe = btn.dataset.tf;
      this._pddTfAuto = false;
      this._pddEmptyPolls = 0;
      // Kick the backend before reading: a timeframe that has never been collected otherwise
      // sits on "Waiting for chart data" until ordinary monitoring gets around to it.
      triggerRefresh(mint);
      await this._refreshPositionChart(mint);
    });

    const ctWrap = this.dialogEl?.querySelector("#pddChartType");
    ctWrap?.addEventListener("click", (e) => {
      const btn = e.target.closest(".timeframe-btn");
      if (!btn || !this._pddChart) return;
      setPressed(ctWrap, btn);
      this._pddChartType = btn.dataset.ct;
      // setChartType carries markers, reference lines and indicators across to the new series.
      this._pddChart.setChartType(this._pddChartType);
    });

    const emaBtn = this.dialogEl?.querySelector("#pddEmaToggle");
    emaBtn?.addEventListener("click", () => {
      if (!this._pddChart) return;
      this._pddEma = !this._pddEma;
      emaBtn.classList.toggle("active", this._pddEma);
      emaBtn.setAttribute("aria-pressed", String(this._pddEma));
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
    this.dialogEl?.querySelector("#pddFrameBtn")?.addEventListener("click", () => {
      this._framePosition();
    });

    this._pddThemeObserver = new MutationObserver(() => {
      if (!this._pddChart) return;
      const theme =
        document.documentElement.getAttribute("data-theme") === "light" ? "light" : "dark";
      this._pddChart.setTheme(theme);
      // Marker, reference-line and legend colours come from the theme, and setTheme repaints
      // only the series it owns — rebuild the overlay in the new palette.
      this._pddMarkerSignature = null;
      this._updatePositionChartMarkers();
      this._renderChartLegend();
    });
    this._pddThemeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
  };

  /**
   * One chart refresh: read the status for this position's span, settle the timeframe while
   * nothing is drawn yet, then load candles. Shared by the first paint, the poller and a manual
   * timeframe switch, so all three follow the same rules. Overlapping calls are dropped — a
   * slow read must not stack a second one behind it.
   */
  proto._refreshPositionChart = async function (mint) {
    if (this._pddRefreshing) return;
    this._pddRefreshing = true;
    try {
      const status = await this._updatePositionDataIndicator(mint);
      if (!this._pddChart) return;

      // Until the user picks a timeframe and while nothing is drawn, follow what storage holds
      // for THIS position: the span-ideal timeframe of a weeks-old position can hold only
      // candles from after it closed. Once candles are on screen the timeframe stays put, so
      // history arriving later cannot swap the chart under the user.
      if (this._pddTfAuto && !this._pddChartData?.length && status) {
        const { from, to } = this._positionSpan();
        const has = (tf) => status.timeframes?.some((row) => row.timeframe === tf && row.candles > 0);
        const next =
          timeframeCoveringSpan(status, from, to) ||
          (has(this._chartTimeframe) ? null : status.best_timeframe);
        if (next && next !== this._chartTimeframe) {
          this._chartTimeframe = next;
          this._syncPddTimeframeButtons(next);
        }
      }

      const timeframe = this._chartTimeframe;
      await this._loadPositionChartData(mint, timeframe, this._pddRenderedTf !== timeframe, status);
    } finally {
      this._pddRefreshing = false;
      this._syncPositionChartPoller();
    }
  };

  /** Fetch OHLCV and push it into the chart. */
  proto._loadPositionChartData = async function (mint, timeframe, isInitial, status) {
    // Sequence guard: OHLCV reads are slow enough to resolve after the user switched
    // timeframe, which painted the previous timeframe's candles over the current chart.
    const seq = (this._pddLoadSeq = (this._pddLoadSeq || 0) + 1);

    try {
      const chartData = await fetchCandles(mint, timeframe, {
        priority: isInitial ? "high" : "normal",
      });

      if (seq !== this._pddLoadSeq || !this._pddChart) return;
      if (this._chartTimeframe !== timeframe) return;

      if (!chartData.length) {
        this._pddEmptyPolls = (this._pddEmptyPolls || 0) + 1;
        if (this._pddChartData?.length && this._pddRenderedTf === timeframe) return;
        // A settled position's token is usually no longer monitored, so candles never arrive on
        // their own: ask for a collection once per timeframe.
        this._pddRefreshAsked ??= new Set();
        if (!this._pddRefreshAsked.has(timeframe)) {
          this._pddRefreshAsked.add(timeframe);
          triggerRefresh(mint);
        }
        // Say which of the two it is: still being collected, or collected and empty.
        const row = status?.timeframes?.find((tf) => tf.timeframe === timeframe);
        const collecting = !status || (status.monitored && !row?.backfill_complete);
        this._showPositionChartOverlay(
          timeframe,
          collecting ? "Collecting chart data…" : "No chart data for this token yet"
        );
        this._setChartEmpty(true);
        return;
      }

      this._pddEmptyPolls = 0;
      this._pddTfAuto = false;
      this._pddChart.setData(chartData);
      this._pddChartData = chartData;
      this._pddRenderedTf = timeframe;
      this._hidePositionChartOverlay();
      this._setChartEmpty(false);

      this._pddLatestCandle = chartData[chartData.length - 1];
      this._updatePddOhlc(this._pddLatestCandle);
      this._updatePositionChartMarkers();

      if (isInitial) this._framePosition();
    } catch {
      if (seq !== this._pddLoadSeq) return;
      this._showPositionChartOverlay(timeframe, "Waiting for chart data...");
    }
  };

  /** Cadence the chart warrants now: fast while waiting, slow once drawn or long empty. */
  proto._positionChartPollMs = function () {
    if (!this._pddChartData?.length) {
      return (this._pddEmptyPolls || 0) >= WAITING_POLLS_BEFORE_BACKOFF
        ? EMPTY_POLL_MS
        : WAITING_POLL_MS;
    }
    return this._isSettled() ? SETTLED_POLL_MS : OPEN_POLL_MS;
  };

  proto._startPositionChartPoller = function (mint) {
    this._stopPositionChartPoller();
    const intervalMs = this._positionChartPollMs();
    this._pddPoller = new Poller(() => this._refreshPositionChart(mint), {
      label: "PositionChart",
      intervalMs,
    });
    this._pddPoller.start({ silent: true });
  };

  /** `Poller` reads its interval when it starts, so a new cadence needs a restart. */
  proto._syncPositionChartPoller = function () {
    if (!this._pddPoller) return;
    const next = this._positionChartPollMs();
    if (next === this._pddPoller.intervalMs) return;
    this._pddPoller.intervalMs = next;
    this._pddPoller.start({ silent: true });
  };

  proto._stopPositionChartPoller = function () {
    if (!this._pddPoller) return;
    this._pddPoller.stop();
    this._pddPoller.cleanup();
    this._pddPoller = null;
  };

  /**
   * The loading overlay is a full-cover panel, so it may only appear when the canvas
   * underneath is not already showing THIS timeframe. Re-showing it on a single dropped poll
   * hid a good chart behind a spinner; suppressing it unconditionally left the previous
   * timeframe's candles under the newly selected button.
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

  /** A token with no candles gets a short notice instead of a screen-high empty frame. */
  proto._setChartEmpty = function (empty) {
    this.dialogEl?.querySelector("#pddChartSection")?.classList.toggle("is-empty", empty);
  };

  proto._syncPddTimeframeButtons = function (tf) {
    this.dialogEl
      ?.querySelector("#pddTimeframes")
      ?.querySelectorAll(".timeframe-btn")
      .forEach((b) => {
        b.classList.toggle("active", b.dataset.tf === tf);
        b.setAttribute("aria-pressed", String(b.dataset.tf === tf));
      });
  };

  /**
   * Read the status for this position's span and refresh the data-status chip from it. Returns
   * the status (null on failure). Never throws.
   */
  proto._updatePositionDataIndicator = async function (mint) {
    const status = await fetchOhlcvStatus(mint, this._positionSpan());
    const indicator = this.dialogEl?.querySelector("#pddDataIndicator");
    if (status && indicator) {
      renderOhlcvStatus(
        { indicator, tip: this.dialogEl.querySelector("#pddDataTip") },
        status,
        Utils.formatTimeAgo
      );
    }
    return status;
  };

  // ===========================================================================
  // POSITION OVERLAY
  // ===========================================================================

  /**
   * The position's buys and sells for the chart. A position rebuilt from wallet history has no
   * per-fill records, only the entry and exit on the position itself — its chart drew no markers
   * at all while the activity listed both trades. Those recorded fields stand in, never a guess.
   */
  proto._positionEvents = function () {
    const pos = this._position() || {};
    let entries = this.fullDetails?.entries || [];
    let exits = this.fullDetails?.exits || [];
    if (!entries.length && pos.entry_time && pos.entry_price) {
      entries = [{ timestamp: pos.entry_time, price: pos.entry_price, is_dca: false }];
    }
    if (!exits.length && this._isSettled() && pos.exit_time && pos.exit_price) {
      exits = [{ timestamp: pos.exit_time, price: pos.exit_price }];
    }
    return { entries, exits };
  };

  /** First and last moment this position was alive, in unix seconds. */
  proto._positionSpan = function () {
    const pos = this._position() || {};
    const { entries, exits } = this._positionEvents();
    const now = Math.floor(Date.now() / 1000);

    const stamps = [
      ...entries.map((e) => Number(e.timestamp)),
      ...exits.map((e) => Number(e.timestamp)),
      Number(pos.entry_time) || 0,
    ].filter((t) => Number.isFinite(t) && t > 0);

    const from = stamps.length ? Math.min(...stamps) : now - 3600;
    const to = this._isSettled() ? Number(pos.exit_time) || Math.max(...stamps) : now;
    return { from, to: Math.max(to, from + 60) };
  };

  proto._positionSpanSeconds = function () {
    const { from, to } = this._positionSpan();
    return to - from;
  };

  /**
   * Point the view at the position's lifetime and hand the frame back to it. Safe before any
   * candles have landed: the chart records the frame and applies it when data arrives.
   */
  proto._framePosition = function () {
    if (!this._pddChart) return;
    const { from, to } = this._positionSpan();
    this._pddChart.resetUserInteraction();
    this._pddChart.anchorRange(from, to);
  };

  /**
   * Overlay this position's entries / DCAs / exits and the average entry line. Rebuilt only
   * when the events or the loaded bar window changed — recreating every price line on each
   * tick both flickered and cost more than the whole redraw.
   */
  proto._updatePositionChartMarkers = function () {
    if (!this._pddChart) return;
    const pos = this._position() || {};
    const { entries, exits } = this._positionEvents();
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

    // lightweight-charts renders a marker only when its `time` matches a bar, so snap each
    // event to the candle that CONTAINS it. Events with no such candle loaded — outside the
    // window, or in a no-trade gap — are counted, not faked: clamping a sell onto a
    // neighbouring bar claimed it happened in a candle it did not.
    let dropped = 0;
    const snapToBar = (ts) => {
      const bar = this._barForTimestamp(ts);
      if (!bar) dropped += 1;
      return bar ? bar.time : null;
    };

    const markers = [];
    let dcaIdx = 0;
    entries.forEach((e) => {
      if (!e.price || !e.timestamp) return;
      const isDca = Boolean(e.is_dca);
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

    // Candles that carry a marker, with their labels: the tooltip offers them, and a click on
    // one finds the event in the activity list (panes.js).
    this._pddMarkerBars = new Map();
    markers.forEach((marker) => {
      const labels = this._pddMarkerBars.get(marker.timestamp) || [];
      labels.push(marker.label);
      this._pddMarkerBars.set(marker.timestamp, labels);
    });

    // Bar markers only. One dashed price line per DCA and partial exit turned the price scale
    // into an unreadable stack of axis labels; the average entry is the one level worth drawing.
    this._pddChart.setPositionMarkers(markers);
    this._pddChart.setOverlayLines(
      avgEntry
        ? [
            {
              price: avgEntry,
              color: this._pddChart.theme.positionColors.avgEntry,
              label: "Avg Entry",
              style: 2,
              // Price lines do not extend the price scale on their own, so a deeply red or
              // green position drew its entry off-pane.
              autoscale: true,
            },
          ]
        : []
    );
    this._renderChartLegend();

    const note = this.dialogEl?.querySelector("#pddChartNote");
    if (note) {
      note.textContent = dropped
        ? `${dropped} event${dropped > 1 ? "s" : ""} without a candle on this timeframe`
        : "";
    }
  };

  /**
   * A key for what is actually drawn: a legend entry for a marker kind the position does not
   * have is noise. Colours come from the chart theme so the key always matches the marks.
   */
  proto._renderChartLegend = function () {
    const items = this.dialogEl?.querySelector("#pddLegendItems");
    const colors = this._pddChart?.theme?.positionColors;
    if (!items || !colors) return;

    const pos = this._position() || {};
    const { entries, exits } = this._positionEvents();
    const legend = [
      ["Entry", colors.entry, "", entries.some((e) => !e.is_dca)],
      ["DCA", colors.dca, "", entries.some((e) => e.is_dca)],
      ["Exit", colors.exit, "", exits.length > 0],
      ["Avg entry", colors.avgEntry, " is-line", Boolean(pos.average_entry_price || pos.entry_price)],
    ];

    items.innerHTML = legend
      .filter(([, , , shown]) => shown)
      .map(
        ([label, color, cls]) =>
          `<span class="pdd-legend-item${cls}" style="--pdd-legend-color: ${color}">${label}</span>`
      )
      .join("");
  };

  /**
   * The loaded candle that contains a unix-seconds timestamp, or null when it falls outside the
   * loaded window. Shared by the markers and the activity links, so both agree on the candle.
   */
  proto._barForTimestamp = function (ts) {
    return barForTimestamp(this._pddChartData, ts, timeframeSeconds(this._pddRenderedTf));
  };

  /**
   * Extra tooltip rows: what this position looked like at the hovered bar, and on a marked bar
   * that a click finds the event. No P&L rows without an average entry to compare against — an
   * unanswerable row is worse than no row.
   */
  proto._positionTooltipRows = function (bar) {
    const rows = [];
    const pos = this._position() || {};
    const avgEntry = pos.average_entry_price || pos.entry_price;

    if (avgEntry && bar?.close) {
      const pnlPct = ((bar.close - avgEntry) / avgEntry) * 100;
      rows.push(
        { label: "Avg Entry", value: this._formatPrice(avgEntry) },
        {
          label: "P&L @ Bar",
          value: `${pnlPct >= 0 ? "+" : "-"}${Math.abs(pnlPct).toFixed(2)}%`,
          cls: pnlPct >= 0 ? "positive" : "negative",
        }
      );
    }

    const marks = bar ? this._pddMarkerBars?.get(bar.time) : null;
    if (marks) rows.push({ label: marks.join(" · "), value: "Click to locate" });
    return rows;
  };

  /** Update the O/H/L/C header from one candle (hovered bar, or the latest). */
  proto._updatePddOhlc = function (last) {
    if (!last) return;
    const set = (id, v) => {
      const el = this.dialogEl?.querySelector(id);
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

  /** Tear down the chart and its observers. */
  proto._destroyPositionChart = function () {
    this._stopPositionChartPoller();
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
    this._pddMarkerBars = null;
    this._pddEmptyPolls = 0;
    this._pddRefreshAsked = null;
    this._pddRefreshing = false;
  };
}
