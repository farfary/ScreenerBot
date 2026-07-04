/**
 * Token Details Dialog - Chart Tab Mixin
 * Extracted from token_details_dialog.js to reduce file size
 * Handles price chart rendering with lightweight-charts
 */
import * as Utils from "../../core/utils.js";
import { requestManager } from "../../core/request_manager.js";
import * as AppState from "../../core/app_state.js";

// Persisted across dialog opens so the user's last chart timeframe is restored.
const TIMEFRAME_STATE_KEY = "tokenChartTimeframe";

// Candle count requested for the chart. 0 = the FULL stored history for the
// selected timeframe (no cap) — the chart shows every candle we have, like a
// normal price chart, and lightweight-charts virtualizes rendering so large
// sets stay smooth. This MUST be identical on the initial load and on every
// poll refresh: a mismatch (the initial load used to omit the param → backend
// default 100, while the poll asked for 200) made the chart visibly jump to
// "different data" a few seconds after opening. Requesting the full set also
// avoids the cache-vs-DB flip, where a capped read returned the newest N from
// the warm cache but the OLDEST N from a cold-cache SQL read. Single source of
// truth for both fetch sites.
export const CHART_CANDLE_LIMIT = 0;

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

    // The overview tab is (re)rendered more than once in quick succession — it
    // renders with partial row data on open, then again the moment the immediate
    // detail fetch resolves. Each render does `content.innerHTML = …`, which
    // REPLACES the chart container node. If a chart already exists AND it is
    // still bound to the live container, just refresh its data (avoids leaks +
    // duplicate pollers). But if the container was swapped out by a re-render,
    // the existing chart is now drawing into a DETACHED node — the visible chart
    // would stay blank forever even as OHLCV data arrives (only a full page
    // reload fixed it). In that case tear the orphan down and recreate it in the
    // live container.
    if (this.advancedChart) {
      if (this.advancedChart.container === chartContainer && chartContainer.isConnected) {
        await this._loadChartData(mint, this.currentTimeframe, false);
        return;
      }
      // Orphaned by a container swap — dispose before recreating.
      this._stopChartPolling();
      if (this._themeObserver) {
        this._themeObserver.disconnect();
        this._themeObserver = null;
      }
      this.advancedChart.destroy();
      this.advancedChart = null;
      this.chart = null;
      this.chartDataLoaded = false;
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

    // Restore the user's last-used timeframe (falling back to the active button,
    // then 5m). Only honor a saved value that maps to a real button.
    const buttons = timeframeButtons ? [...timeframeButtons.querySelectorAll(".timeframe-btn")] : [];
    const saved = AppState.load(TIMEFRAME_STATE_KEY, null);
    const activeBtn = timeframeButtons?.querySelector(".timeframe-btn.active");
    const savedValid = saved && buttons.some((b) => b.dataset.tf === saved);
    this.currentTimeframe = savedValid ? saved : activeBtn?.dataset.tf || "5m";

    // Sync the active button highlight to the restored timeframe.
    buttons.forEach((b) => b.classList.toggle("active", b.dataset.tf === this.currentTimeframe));

    // Allow ONE automatic timeframe fallback on this token's first load (see
    // _loadChartData): if the chosen timeframe has no candles yet but the token
    // has data at another (e.g. only daily so far), switch to it instead of
    // sitting on an empty chart. Manual timeframe clicks never auto-switch.
    this._chartInitialLoadPending = true;
    await this._loadChartData(mint, this.currentTimeframe, true); // Initial load - set view

    // Populate the data-status indicator (fire-and-forget; never blocks render).
    this._updateDataIndicator(mint);

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
        // Remember the choice for the next time any token dialog is opened.
        AppState.save(TIMEFRAME_STATE_KEY, this.currentTimeframe);
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
      // Use requestManager with high priority for initial chart data load.
      // limit MUST match the poll refresh (CHART_CANDLE_LIMIT) so the dataset
      // doesn't change the moment the first poll lands.
      const data = await requestManager.fetch(
        `/api/tokens/${mint}/ohlcv?timeframe=${timeframe}&limit=${CHART_CANDLE_LIMIT}`,
        { priority: isInitialLoad ? "high" : "normal" }
      );

      if (!Array.isArray(data) || data.length === 0) {
        // The selected timeframe has no candles. The token can still have OHLCV
        // at a different timeframe (e.g. only daily candles fetched so far) — the
        // has_ohlcv badge is true while this specific timeframe is empty. On the
        // token's first load, probe for a timeframe that DOES have data and
        // switch to it instead of sitting on an empty "Waiting…" chart. Runs once
        // per open and never overrides a manual timeframe selection.
        if (isInitialLoad && this._chartInitialLoadPending) {
          this._chartInitialLoadPending = false;
          const altTf = await this._findTimeframeWithData(mint, timeframe);
          if (altTf && altTf !== timeframe) {
            this.currentTimeframe = altTf;
            this._syncTimeframeButtons(altTf);
            await this._loadChartData(mint, altTf, true);
            return;
          }
        }
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

      this._chartInitialLoadPending = false;

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
        // Reset interaction flag and anchor the newest candle at a fixed bar
        // width (fills the pane with normal-size candles for any candle count —
        // no giant stretched bars when only a few candles exist).
        this.advancedChart.resetUserInteraction();
        this.advancedChart.anchorLatest();
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
   * Find the finest timeframe that currently has candles for this token, probing
   * cheaply (limit=1). Returns null if none have data. Order is finest→coarsest
   * so the chart lands on the most granular timeframe available.
   * @private
   * @param {string} mint
   * @param {string} exclude - timeframe already known to be empty (skipped)
   * @returns {Promise<string|null>}
   */
  proto._findTimeframeWithData = async function (mint, exclude) {
    const order = ["1m", "5m", "15m", "1h", "4h", "12h", "1d"];
    const checks = await Promise.all(
      order.map(async (tf) => {
        if (tf === exclude) return null;
        try {
          const d = await requestManager.fetch(
            `/api/tokens/${mint}/ohlcv?timeframe=${tf}&limit=1`,
            { priority: "low" }
          );
          return Array.isArray(d) && d.length > 0 ? tf : null;
        } catch {
          return null;
        }
      })
    );
    return checks.find((tf) => tf) || null;
  };

  /**
   * Highlight the given timeframe button (used when an automatic fallback
   * switches the chart to a timeframe the user didn't click).
   * @private
   * @param {string} tf
   */
  proto._syncTimeframeButtons = function (tf) {
    const timeframeButtons = this.dialogEl?.querySelector("#timeframeButtons");
    if (!timeframeButtons) return;
    timeframeButtons
      .querySelectorAll(".timeframe-btn")
      .forEach((b) => b.classList.toggle("active", b.dataset.tf === tf));
  };

  /**
   * Fetch and render the chart data-status indicator (the small "Data" chip that
   * replaced the "Price Chart" label). Shows an at-a-glance state — ready /
   * collecting / no data — with a hover tooltip breaking down candle counts and
   * backfill completion per timeframe. Cheap single endpoint; safe to call on
   * each chart poll. Never throws.
   * @private
   * @param {string} mint
   */
  proto._updateDataIndicator = async function (mint) {
    const indicator = this.dialogEl?.querySelector("#chartDataIndicator");
    const tip = this.dialogEl?.querySelector("#chartDataTip");
    if (!indicator) return;

    let status;
    try {
      status = await requestManager.fetch(`/api/tokens/${mint}/ohlcv/status`, {
        priority: "low",
      });
    } catch {
      return; // transient — keep the last rendered state
    }
    if (!status || typeof status !== "object") return;

    // Overall state: ready (any data + backfill done) / collecting (monitored or
    // partial data) / none (nothing yet).
    let state = "none";
    let summary = "No chart data yet";
    if (status.has_data && status.backfill_complete) {
      state = "ready";
      summary = "Data ready";
    } else if (status.has_data) {
      state = "partial";
      summary = "Collecting history…";
    } else if (status.monitored) {
      state = "collecting";
      summary = "Fetching data…";
    }
    indicator.dataset.state = state;
    indicator.setAttribute("aria-label", `Chart data: ${summary}`);

    if (tip) {
      const ago = (ts) => (ts ? Utils.formatTimeAgo(ts) : "—");
      const rows = (status.timeframes || [])
        .map((tf) => {
          const has = tf.candles > 0;
          const dot = has ? (tf.backfill_complete ? "ready" : "partial") : "none";
          const count = has ? Number(tf.candles).toLocaleString() : "—";
          const fresh = has ? ago(tf.last_new_data_at) : "—";
          return `
            <div class="chart-data-tip-row">
              <span class="chart-data-tip-dot" data-state="${dot}"></span>
              <span class="chart-data-tip-tf">${tf.timeframe.toUpperCase()}</span>
              <span class="chart-data-tip-count">${count}</span>
              <span class="chart-data-tip-fresh" title="Last new candle">${fresh}</span>
            </div>`;
        })
        .join("");
      const checked = status.last_checked_at
        ? `checked ${ago(status.last_checked_at)}`
        : status.monitored
          ? "checking…"
          : "not checked";
      const updated = status.last_new_data_at
        ? `updated ${ago(status.last_new_data_at)}`
        : "no candles yet";
      tip.innerHTML = `
        <div class="chart-data-tip-head">
          <span class="chart-data-tip-title">${summary}</span>
          <span class="chart-data-tip-checked">${checked}</span>
        </div>
        <div class="chart-data-tip-cols">
          <span></span>
          <span>TF</span>
          <span class="chart-data-tip-count">Candles</span>
          <span class="chart-data-tip-fresh">New</span>
        </div>
        <div class="chart-data-tip-list">${rows}</div>
        <div class="chart-data-tip-foot">
          <span>${Number(status.total_candles || 0).toLocaleString()} candles · ${
            status.monitored ? "monitoring" : "idle"
          }</span>
          <span class="chart-data-tip-foot-time">${updated}</span>
        </div>`;
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
