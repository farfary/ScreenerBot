import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { requestManager } from "../core/request_manager.js";
import { createTraderConfigCards } from "./trader/config_cards.js";
import { playToggleOn, playToggleOff, playError } from "../core/sounds.js";
import { createExampleUpdaters } from "./trader/examples.js";
import { createTraderControls } from "./trader/controls.js";
import {
  fetchFeatureStatus,
  isTabUsable,
  applyFeatureStatusToTabs,
  handleFeatureRestrictedTab,
} from "./trader/features.js";
import { createLifecycle as createStrategiesLifecycle } from "./strategies.js";

// Sub-tabs configuration. Strategy Control is second and the embedded Strategies
// editor is third (Strategies was formerly its own top-level tab).
const SUB_TABS = [
  { id: "stats", label: '<i class="icon-chart-bar"></i> Stats' },
  { id: "strategy-control", label: '<i class="icon-puzzle"></i> Strategy Control' },
  { id: "strategies", label: '<i class="icon-square-pen"></i> Strategies' },
  { id: "stop-loss", label: '<i class="icon-shield-off"></i> Stop Loss' },
  { id: "trailing-stop", label: '<i class="icon-trending-up"></i> Trailing Stop' },
  { id: "roi", label: '<i class="icon-target"></i> Take Profit' },
  { id: "time-rules", label: '<i class="icon-timer"></i> Time Rules' },
  { id: "dca", label: '<i class="icon-dollar-sign"></i> DCA' },
  { id: "general-settings", label: '<i class="icon-settings"></i> Settings' },
];

// Constants
const DEFAULT_TAB = "stats";

function createLifecycle() {
  // Component references
  let tabBar = null;
  let configCards = null;
  let statsPoller = null;
  let configPoller = null;
  let strategiesPoller = null;
  let lifecycleContext = null;

  // Realized window for the Stats tab, in days. Owned here and sent to the API —
  // the tab never labels a window it did not ask for.
  let statsPeriodDays = 30;

  // Hash guards — a 5s poll must not rewrite innerHTML that has not changed, or it
  // destroys the user's text selection on every tick.
  let _lastDailyKey = null;
  let _lastExitKey = null;

  // Event cleanup tracking
  const eventCleanups = [];
  const strategyListCleanups = [];

  // Feature status from API
  let tradingFeatures = {};

  // Page state
  const state = {
    currentTab: DEFAULT_TAB,
    config: null,
    stats: null,
    strategies: [],
  };

  // Initialize sub-modules
  const examples = createExampleUpdaters({ $, Utils });
  const controls = createTraderControls({
    state,
    $,
    Utils,
    requestManager,
    ConfirmationDialog,
    playToggleOn,
    playToggleOff,
    playError,
    eventCleanups,
  });

  // Embedded Strategies subtab — drives the strategies page module's lifecycle.
  // A local ctx adapter owns the strategies pollers so they start when the
  // subtab opens and stop when it is left (instead of running page-wide).
  let strategiesLifecycle = null;
  let strategiesInited = false;
  let strategiesActive = false;
  const strategiesSubtabPollers = [];
  const strategiesCtx = {
    managePoller(poller) {
      strategiesSubtabPollers.push(poller);
      return poller;
    },
  };

  function stopStrategiesSubtabPollers() {
    strategiesSubtabPollers.forEach((p) => {
      try {
        p.stop?.();
        p.cleanup?.();
      } catch {
        /* ignore */
      }
    });
    strategiesSubtabPollers.length = 0;
  }

  async function activateStrategiesSubtab() {
    if (!strategiesLifecycle) strategiesLifecycle = createStrategiesLifecycle();
    try {
      if (!strategiesInited) {
        await strategiesLifecycle.init(strategiesCtx);
        strategiesInited = true;
      }
      if (!strategiesActive) {
        await strategiesLifecycle.activate(strategiesCtx);
        strategiesActive = true;
      }
    } catch (err) {
      console.error("[Trader] Failed to activate Strategies subtab", err);
    }
  }

  function deactivateStrategiesSubtab() {
    if (strategiesLifecycle && strategiesActive) {
      try {
        strategiesLifecycle.deactivate();
      } catch {
        /* ignore */
      }
      strategiesActive = false;
    }
    stopStrategiesSubtabPollers();
  }

  function disposeStrategiesSubtab() {
    deactivateStrategiesSubtab();
    if (strategiesLifecycle) {
      try {
        strategiesLifecycle.dispose();
      } catch {
        /* ignore */
      }
    }
    strategiesLifecycle = null;
    strategiesInited = false;
  }

  // ============================================================================
  // Helper Functions
  // ============================================================================

  /**
   * Add tracked event listener for cleanup
   */
  function addTrackedListener(element, event, handler) {
    if (!element) return;
    element.addEventListener(event, handler);
    eventCleanups.push(() => element.removeEventListener(event, handler));
  }

  /**
   * Load trailing stop performance stats (placeholder for Phase 2)
   */
  async function loadTrailingStopStats() {
    // This will be implemented in Phase 2 when we add trailing stop tracking
    const statsCards = $$(".quick-stat-card");
    statsCards.forEach((card) => {
      const value = card.querySelector(".quick-stat-value");
      if (value) {
        value.textContent = "--";
      }
    });
  }

  /**
   * Switch to a different tab
   */
  function switchTab(tabId, { load = true } = {}) {
    state.currentTab = tabId;

    // Hide all tab contents
    $$(".trader-tab-content").forEach((el) => {
      el.style.display = "none";
    });

    // Show selected tab
    const tabMap = {
      stats: "stats-tab",
      "stop-loss": "stop-loss-tab",
      "trailing-stop": "trailing-stop-tab",
      roi: "roi-tab",
      "time-rules": "time-rules-tab",
      dca: "dca-tab",
      "strategy-control": "strategy-control-tab",
      strategies: "strategies-tab",
      "general-settings": "general-settings-tab",
    };

    const contentId = tabMap[tabId];
    const content = $(`#${contentId}`);
    if (content) {
      content.style.display = "block";
    }

    // Embedded Strategies editor: activate its lifecycle when shown, stop it
    // (and its pollers) when any other subtab is selected. Toggle the
    // edge-to-edge layout class explicitly here (see trader.css) so the
    // padding state is deterministic on every tab switch and re-entry —
    // never left to a :has() inline-style selector that can go stale.
    const traderContent = $("#trader-content");
    const isStrategiesTab = tabId === "strategies";
    traderContent?.classList.toggle("trader-content--fullbleed", isStrategiesTab);
    if (load) {
      if (isStrategiesTab) activateStrategiesSubtab();
      else deactivateStrategiesSubtab();
    }
    traderContent?.classList.toggle("trader-content--split-scroll", tabId === "stats");

    // init() paints only. Remote loaders and embedded lifecycles start once the
    // page is active, after the selected panel is already on screen.
    if (!load) return;

    // Start/stop pollers based on tab
    if (tabId === "stats") {
      if (statsPoller && !statsPoller.active) {
        statsPoller.start();
      }
    } else {
      if (statsPoller?.active) {
        statsPoller.stop();
      }
    }

    if (tabId === "strategy-control") {
      loadStrategies({ showLoading: true });
      if (strategiesPoller && !strategiesPoller.active) {
        strategiesPoller.start();
      }
    } else {
      if (strategiesPoller?.active) {
        strategiesPoller.stop();
      }
    }

    // Load preview when switching to stop loss tab
    if (tabId === "stop-loss") {
      examples.updateStopLossExample();
    }

    // Load preview when switching to trailing stop tab
    if (tabId === "trailing-stop") {
      examples.updateTrailingStopExample();
      loadTrailingStopStats();
      loadTrailingStopPreview();
    }

    // Update tab-specific data
    if (tabId === "time-rules") {
      updateTimeRulesStatus();
    }
  }

  /**
   * Load configuration from server
   */
  async function loadConfig(options = {}) {
    try {
      const data = await requestManager.fetch("/api/config", {
        priority: "normal",
      });
      state.config = data.config;
      const preserveUnsavedEdits =
        options.preserveUnsavedEdits === true && configCards?.hasDirtyCards?.();

      if (!preserveUnsavedEdits) {
        // Update form fields
        updateFormFields();

        // Re-baseline the per-card Save/Reset controls to the freshly loaded
        // values (hides the buttons until the next edit).
        configCards?.snapshot();
      }

      // Update visual examples with loaded values
      examples.updateStopLossExample();
      examples.updateRoiExample();
      examples.updateTimeLossExample();
    } catch (error) {
      console.error("[Trader] Failed to load config:", error);
      Utils.showToast({
        type: "error",
        title: "Load Failed",
        message: "Failed to load trader configuration",
      });
    }
  }

  /**
   * Update form fields from config state
   */
  function updateFormFields() {
    if (!state.config) return;

    const trader = state.config.trader || {};
    const positions = state.config.positions || {};

    // Stop Loss (from trader config)
    const stopLossEnabled = $("#stop-loss-enabled");
    const stopLossThreshold = $("#stop-loss-threshold");
    const stopLossAllowPartial = $("#stop-loss-allow-partial");
    const stopLossMinHold = $("#stop-loss-min-hold");
    if (stopLossEnabled) {
      stopLossEnabled.checked = trader.stop_loss_enabled || false;
    }
    if (stopLossThreshold) {
      stopLossThreshold.value = trader.stop_loss_threshold_pct || 50.0;
    }
    if (stopLossAllowPartial) {
      stopLossAllowPartial.checked = trader.stop_loss_allow_partial || false;
    }
    if (stopLossMinHold) {
      stopLossMinHold.value = trader.stop_loss_min_hold_seconds || 0;
    }

    // Trailing Stop (from positions config)
    const trailingEnabled = $("#trailing-enabled");
    const trailActivation = $("#trail-activation");
    const trailDistance = $("#trail-distance");
    if (trailingEnabled) {
      trailingEnabled.checked = positions.trailing_stop_enabled || false;
    }
    if (trailActivation) {
      trailActivation.value = positions.trailing_stop_activation_pct || 10.0;
    }
    if (trailDistance) {
      trailDistance.value = positions.trailing_stop_distance_pct || 5.0;
    }

    // ROI
    const roiEnabled = $("#roi-enabled");
    const roiTarget = $("#roi-target");
    if (roiEnabled) {
      roiEnabled.checked = trader.roi_exit_enabled || false;
    }
    if (roiTarget) {
      roiTarget.value = trader.roi_target_percent || 20;
    }

    // Time Rules
    const timeOverrideEnabled = $("#time-override-enabled");
    const timeMaxHold = $("#time-max-hold");
    const timeUnit = $("#time-unit");
    const timeLossThreshold = $("#time-loss-threshold");

    if (timeOverrideEnabled) {
      timeOverrideEnabled.checked = trader.time_override_enabled || false;
    }
    if (timeMaxHold) {
      timeMaxHold.value = trader.time_override_duration || 168;
    }
    if (timeUnit) {
      timeUnit.value = trader.time_override_unit || "hours";
    }
    if (timeLossThreshold) {
      timeLossThreshold.value = trader.time_override_loss_threshold_percent || -40;
    }

    // Update time conversion hint
    examples.updateTimeConversionHint();

    // General Settings
    const maxPositions = $("#max-positions");
    const tradeSize = $("#trade-size");
    const entrySizes = $("#entry-sizes");
    const dcaEnabled = $("#dca-enabled");
    const dcaThreshold = $("#dca-threshold");
    const dcaMaxCount = $("#dca-max-count");
    const dcaSize = $("#dca-size");
    const dcaCooldown = $("#dca-cooldown");
    const closeCooldown = $("#close-cooldown");
    const entryConcurrency = $("#entry-concurrency");

    if (maxPositions) maxPositions.value = trader.max_open_positions || 2;
    if (tradeSize) tradeSize.value = trader.trade_size_sol || 0.005;
    if (entrySizes) entrySizes.value = (trader.entry_sizes || [0.005, 0.01, 0.02, 0.05]).join(", ");
    if (dcaEnabled) dcaEnabled.checked = trader.dca_enabled || false;
    if (dcaThreshold) dcaThreshold.value = trader.dca_threshold_pct || -10;
    if (dcaMaxCount) dcaMaxCount.value = trader.dca_max_count || 2;
    if (dcaSize) dcaSize.value = trader.dca_size_percentage || 50;
    if (dcaCooldown) dcaCooldown.value = trader.dca_cooldown_minutes || 30;
    if (closeCooldown) {
      const seconds = Number.isFinite(trader.close_cooldown_seconds)
        ? trader.close_cooldown_seconds
        : 600;
      closeCooldown.value = Math.max(0, Math.round(seconds / 60));
    }
    if (entryConcurrency) entryConcurrency.value = trader.entry_monitor_concurrency || 3;
  }

  /**
   * Load statistics for the Stats tab.
   *
   * The handler is the only place that aggregates: every figure below is read from
   * the response as-is. Nothing is re-derived here, and a `null` means the window
   * holds nothing to derive the figure from, so it renders as an em dash rather
   * than a fabricated zero.
   */
  async function loadStats() {
    try {
      const data = await requestManager.fetch(`/api/trader/stats?days=${statsPeriodDays}`, {
        priority: "normal",
      });

      const pct = (value, decimals = 1) =>
        Utils.formatPercentValue(value, { decimals, fallback: "—" });
      const sol = (value, decimals = 4) => Utils.formatSol(value, { decimals, fallback: "—" });
      const setValue = (id, text, tone) => {
        const el = $(`#${id}`);
        if (!el) return;
        el.textContent = text;
        el.className = tone ? `metric-value ${tone}` : "metric-value";
      };
      const setDetail = (id, text) => {
        const el = $(`#${id}`);
        if (el) el.textContent = text;
      };
      const tone = (value) => {
        if (!Number.isFinite(value) || value === 0) return null;
        return value > 0 ? "positive" : "negative";
      };

      // Net P&L — the booked, fee- and DCA-aware SOL the window actually returned.
      setValue(
        "net-pnl",
        Number.isFinite(data.total_pnl_sol)
          ? `${data.total_pnl_sol > 0 ? "+" : ""}${sol(data.total_pnl_sol)}`
          : "—",
        tone(data.total_pnl_sol)
      );
      setDetail(
        "net-pnl-detail",
        data.total_trades > 0
          ? `${sol(data.gross_profit_sol)} won · ${sol(data.gross_loss_sol)} lost`
          : "No closed trades in this window"
      );

      setValue(
        "win-rate",
        Utils.formatPercentValue(data.win_rate_pct, {
          decimals: 1,
          fallback: "—",
          includeSign: false,
        }),
        Number.isFinite(data.win_rate_pct) && data.win_rate_pct >= 50 ? "positive" : null
      );
      setDetail(
        "win-rate-detail",
        data.total_trades > 0 ? `${data.winners} wins · ${data.losers} losses` : "—"
      );

      setValue(
        "profit-factor",
        Number.isFinite(data.profit_factor) ? data.profit_factor.toFixed(2) : "—",
        Number.isFinite(data.profit_factor)
          ? data.profit_factor >= 1
            ? "positive"
            : "negative"
          : null
      );
      setDetail(
        "profit-factor-detail",
        Number.isFinite(data.expectancy_sol)
          ? `${sol(data.expectancy_sol)} expected per trade`
          : "Gross won ÷ gross lost"
      );

      setValue(
        "max-drawdown",
        data.total_trades > 0 ? sol(data.max_drawdown_sol) : "—",
        data.max_drawdown_sol > 0 ? "negative" : null
      );
      setDetail("max-drawdown-detail", "Deepest realized peak-to-trough");

      setValue("capital-at-work", sol(data.locked_sol));
      setDetail(
        "capital-at-work-detail",
        `${data.open_positions_count} of ${data.max_open_positions} position slots used`
      );

      const hasWin = Number.isFinite(data.avg_win_pct);
      const hasLoss = Number.isFinite(data.avg_loss_pct);
      setValue(
        "avg-win-loss",
        hasWin || hasLoss ? `${pct(data.avg_win_pct)} / ${pct(data.avg_loss_pct)}` : "—"
      );
      setDetail("avg-win-loss-detail", "Average outcome of a winning vs losing trade");

      setValue("total-trades", data.total_trades > 0 ? String(data.total_trades) : "—");
      setDetail(
        "total-trades-detail",
        data.total_trades === 1 ? "1 position closed" : `${data.total_trades} positions closed`
      );

      const holdText = (hours) =>
        Number.isFinite(hours) ? Utils.formatUptime(hours * 3600, { style: "short" }) : "—";
      setValue("median-hold", holdText(data.median_hold_time_hours));
      setDetail(
        "median-hold-detail",
        Number.isFinite(data.avg_hold_time_hours)
          ? `${holdText(data.avg_hold_time_hours)} average`
          : "—"
      );

      // Rounds with an incomplete cost basis carry no honest P&L and are left out of
      // every figure above. Saying so is the difference between a filtered number and
      // a wrong one.
      const excludedEl = $("#stats-excluded");
      if (excludedEl) {
        const n = data.excluded_untrusted || 0;
        excludedEl.hidden = n === 0;
        excludedEl.textContent =
          n === 0
            ? ""
            : `${n} closed ${n === 1 ? "round" : "rounds"} excluded — no complete cost basis, so no honest P&L.`;
      }

      renderDailyPnl(data.daily_pnl, data.total_pnl_sol);
      renderExtremes(data);
      renderExitBreakdown(data.exit_breakdown, data.period_days);
    } catch (error) {
      console.error("[Trader] Failed to load stats:", error);
      for (const id of [
        "net-pnl",
        "win-rate",
        "profit-factor",
        "max-drawdown",
        "capital-at-work",
        "avg-win-loss",
        "total-trades",
        "median-hold",
      ]) {
        const el = $(`#${id}`);
        if (el) {
          el.textContent = "—";
          el.className = "metric-value";
        }
      }
    }
  }

  /**
   * Switch the realized window and reload immediately, so the click is answered by
   * the panel rather than by the next poll tick.
   */
  function setStatsPeriod(days) {
    if (days === statsPeriodDays) return;
    statsPeriodDays = days;
    for (const btn of $$("#stats-period .stats-period-btn")) {
      btn.classList.toggle("active", Number(btn.dataset.days) === days);
    }
    _lastDailyKey = null;
    _lastExitKey = null;
    void loadStats();
  }

  /**
   * Render the daily realized P&L: one bar per day plus the cumulative line.
   *
   * Hand-built SVG on purpose — this is a fixed-size, non-interactive shape, and a
   * charting library would cost more than the whole panel.
   */
  function renderDailyPnl(days, totalPnlSol) {
    const container = $("#daily-pnl");
    if (!container) return;

    const totalEl = $("#daily-pnl-total");
    if (totalEl) {
      const finite = Number.isFinite(totalPnlSol);
      totalEl.textContent = finite
        ? `${totalPnlSol > 0 ? "+" : ""}${Utils.formatSol(totalPnlSol, { fallback: "—" })}`
        : "—";
      totalEl.className = `daily-pnl-total${finite && totalPnlSol !== 0 ? (totalPnlSol > 0 ? " positive" : " negative") : ""}`;
    }

    if (!Array.isArray(days) || days.every((d) => (d.trades || 0) === 0)) {
      container.innerHTML =
        '<div class="info-state"><i class="icon-inbox"></i><span>No closed trades in this window</span></div>';
      _lastDailyKey = null;
      return;
    }

    const key = days.map((d) => `${d.date}:${d.net_pnl_sol.toFixed(6)}`).join("|");
    if (key === _lastDailyKey) return;
    _lastDailyKey = key;

    const W = 100;
    const H = 40;
    const slot = W / days.length;
    const barW = Math.max(slot * 0.62, 0.35);

    const peak = Math.max(...days.map((d) => Math.abs(d.net_pnl_sol)), 1e-9);
    const mid = H / 2;
    const bars = days
      .map((d, i) => {
        const h = (Math.abs(d.net_pnl_sol) / peak) * (mid - 1);
        const x = i * slot + (slot - barW) / 2;
        const y = d.net_pnl_sol >= 0 ? mid - h : mid;
        const cls = d.net_pnl_sol >= 0 ? "positive" : "negative";
        return `<rect class="daily-pnl-bar ${cls}" x="${x.toFixed(2)}" y="${y.toFixed(2)}" width="${barW.toFixed(2)}" height="${Math.max(h, 0.4).toFixed(2)}"></rect>`;
      })
      .join("");

    // Cumulative line on its own scale, so a flat run of small days stays readable.
    let running = 0;
    const cumulative = days.map((d) => (running += d.net_pnl_sol));
    const cMin = Math.min(0, ...cumulative);
    const cMax = Math.max(0, ...cumulative);
    const cSpan = cMax - cMin || 1e-9;
    const points = cumulative
      .map((v, i) => {
        const x = i * slot + slot / 2;
        const y = H - ((v - cMin) / cSpan) * (H - 2) - 1;
        return `${x.toFixed(2)},${y.toFixed(2)}`;
      })
      .join(" ");

    const first = days[0];
    const last = days[days.length - 1];
    container.innerHTML = `
      <svg class="daily-pnl-chart" viewBox="0 0 ${W} ${H}" preserveAspectRatio="none" role="img"
           aria-label="Daily realized profit and loss in SOL">
        <line class="daily-pnl-zero" x1="0" y1="${mid}" x2="${W}" y2="${mid}"></line>
        ${bars}
        <polyline class="daily-pnl-line" points="${points}"></polyline>
      </svg>
      <div class="daily-pnl-axis">
        <span>${Utils.escapeHtml(first.date)}</span>
        <span>${Utils.escapeHtml(last.date)}</span>
      </div>`;
  }

  /**
   * Best and worst closed round in the window.
   */
  function renderExtremes(data) {
    const wrap = $("#stats-extremes");
    if (!wrap) return;

    const hasAny = Number.isFinite(data.best_trade_pct) || Number.isFinite(data.worst_trade_pct);
    wrap.hidden = !hasAny;
    if (!hasAny) return;

    const paint = (valueId, tokenId, value, token) => {
      const valueEl = $(`#${valueId}`);
      const tokenEl = $(`#${tokenId}`);
      if (valueEl) {
        valueEl.textContent = Utils.formatPercentValue(value, { decimals: 1, fallback: "—" });
        valueEl.className = `stats-extreme-value${
          Number.isFinite(value) && value !== 0 ? (value > 0 ? " positive" : " negative") : ""
        }`;
      }
      if (tokenEl) tokenEl.textContent = token || "—";
    };

    paint("best-trade", "best-trade-token", data.best_trade_pct, data.best_trade_token);
    paint("worst-trade", "worst-trade-token", data.worst_trade_pct, data.worst_trade_token);
  }

  // Humanize a closed_reason / exit_type into a readable label.
  const EXIT_TYPE_LABELS = {
    stop_loss: "Stop Loss",
    take_profit: "Take Profit",
    roi: "ROI Target",
    roi_exit: "ROI Target",
    trailing_stop: "Trailing Stop",
    time_override: "Time Override",
    time_rule: "Time Rule",
    manual: "Manual",
    manual_close: "Manual",
    dca: "DCA",
    unknown: "Unknown",
  };

  function formatExitType(type) {
    if (!type) return "Unknown";
    return (
      EXIT_TYPE_LABELS[type] || type.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())
    );
  }

  /**
   * Render the exit strategy breakdown list (how positions were closed).
   *
   * Guarded by a content hash: without it the 5s poll rewrote this innerHTML on
   * every tick and destroyed any text the user had selected in it.
   */
  function renderExitBreakdown(breakdown, periodDays) {
    const container = $("#exit-breakdown");
    if (!container) return;

    if (!Array.isArray(breakdown) || breakdown.length === 0) {
      const label = periodDays === 1 ? "24 hours" : `${periodDays} days`;
      container.innerHTML = `<div class="info-state"><i class="icon-inbox"></i><span>No closed trades in the last ${Utils.escapeHtml(label)}</span></div>`;
      _lastExitKey = null;
      return;
    }

    const key = breakdown
      .map((e) => `${e.exit_type}:${e.count}:${(e.net_pnl_sol || 0).toFixed(6)}`)
      .join("|");
    if (key === _lastExitKey) return;
    _lastExitKey = key;

    const totalCount = breakdown.reduce((sum, e) => sum + (e.count || 0), 0) || 1;

    container.innerHTML = breakdown
      .map((e) => {
        const count = e.count || 0;
        const avgPct = e.avg_profit_pct || 0;
        const netSol = e.net_pnl_sol || 0;
        const share = Math.round((count / totalCount) * 100);
        const barClass = netSol >= 0 ? "positive" : "negative";
        return `
          <div class="exit-breakdown-row">
            <div class="exit-breakdown-head">
              <span class="exit-breakdown-type">${Utils.escapeHtml(formatExitType(e.exit_type))}</span>
              <span class="exit-breakdown-pnl ${barClass}">${netSol > 0 ? "+" : ""}${Utils.formatSol(netSol, { fallback: "—" })}</span>
            </div>
            <div class="exit-breakdown-bar">
              <div class="exit-breakdown-fill ${barClass}" style="width: ${share}%"></div>
            </div>
            <div class="exit-breakdown-meta">
              <span class="exit-breakdown-share">${count} ${count === 1 ? "trade" : "trades"} · ${share}% of exits</span>
              <span class="exit-breakdown-profit ${avgPct >= 0 ? "positive" : "negative"}">${Utils.formatPercentValue(avgPct, { decimals: 1 })} avg</span>
            </div>
          </div>`;
      })
      .join("");
  }

  /**
   * Load trailing stop preview (Phase 2 Feature)
   */
  async function loadTrailingStopPreview(positionId = null) {
    const activation = parseFloat($("#trail-activation")?.value) || 10;
    const distance = parseFloat($("#trail-distance")?.value) || 5;

    try {
      const params = new URLSearchParams();
      if (positionId) params.append("position_id", positionId);
      params.append("activation_pct", activation);
      params.append("distance_pct", distance);

      const data = await requestManager.fetch(`/api/trader/preview-trailing-stop?${params}`, {
        priority: "normal",
      });

      if (data.success) {
        updatePreviewPanel(data.data);
      } else {
        console.error("[Trader] Preview failed:", data.error);
      }
    } catch (error) {
      console.error("[Trader] Failed to load preview:", error);
    }
  }

  /**
   * Update preview panel with data (Phase 2 Feature)
   */
  function updatePreviewPanel(preview) {
    // Update position state
    const symbol = $("#preview-symbol");
    const entryPrice = $("#preview-entry-price");
    const currentPrice = $("#preview-current-price");
    const peakPrice = $("#preview-peak-price");
    const currentProfit = $("#preview-current-profit");

    if (symbol) symbol.textContent = preview.symbol;
    if (entryPrice) entryPrice.textContent = Utils.formatPrice(preview.entry_price);
    if (currentPrice) currentPrice.textContent = Utils.formatPrice(preview.current_price);
    if (peakPrice) peakPrice.textContent = Utils.formatPrice(preview.peak_price);
    if (currentProfit) {
      currentProfit.textContent = Utils.formatPercent(preview.current_profit_pct);
      currentProfit.className = `profit-value ${preview.current_profit_pct >= 0 ? "positive" : "negative"}`;
    }

    // Update trail status
    const trailStatus = $("#preview-trail-status");
    const trailPrice = $("#preview-trail-price");
    const distanceToExit = $("#preview-distance-to-exit");
    const estimatedExit = $("#preview-estimated-exit");
    const estimatedProfit = $("#preview-estimated-profit");

    if (trailStatus) {
      const statusIcon = preview.trail_active
        ? '<i class="icon-check"></i>'
        : '<i class="icon-pause"></i>';
      trailStatus.innerHTML = `${statusIcon} ${preview.trail_active ? "ACTIVE" : "INACTIVE"}`;
      trailStatus.className = preview.trail_active ? "status-active" : "status-inactive";
    }
    if (trailPrice) {
      trailPrice.textContent = preview.trail_stop_price
        ? Utils.formatPrice(preview.trail_stop_price)
        : "—";
    }
    if (distanceToExit) {
      distanceToExit.textContent = preview.distance_to_exit_pct
        ? Utils.formatPercent(preview.distance_to_exit_pct)
        : "—";
    }
    if (estimatedExit) {
      estimatedExit.textContent = Utils.formatPrice(preview.estimated_exit_price);
    }
    if (estimatedProfit) {
      estimatedProfit.textContent = Utils.formatPercent(preview.estimated_exit_profit_pct);
      estimatedProfit.className = `profit-value ${preview.estimated_exit_profit_pct >= 0 ? "positive" : "negative"}`;
    }

    // Update what-if scenarios
    const scenariosContainer = $("#preview-what-if-scenarios");
    if (scenariosContainer && preview.what_if_scenarios) {
      scenariosContainer.innerHTML = "";
      preview.what_if_scenarios.forEach((scenario) => {
        const scenarioDiv = document.createElement("div");
        scenarioDiv.className = "what-if-scenario";
        const statusIcon = scenario.trail_active
          ? '<i class="icon-check"></i>'
          : '<i class="icon-pause"></i>';
        scenarioDiv.innerHTML = `
          <div class="scenario-description">${scenario.description}</div>
          <div class="scenario-result">
            ${statusIcon} Exit: ${Utils.formatPrice(scenario.exit_price)} 
            (${Utils.formatPercent(scenario.exit_profit_pct)} profit)
          </div>
        `;
        scenariosContainer.appendChild(scenarioDiv);
      });
    }
  }

  /**
   * Load strategies list
   */
  async function loadStrategies({ showLoading = false } = {}) {
    try {
      if (showLoading) {
        setStrategiesLoadingState();
      }

      const [entryData, exitData] = await Promise.all([
        requestManager.fetch("/api/strategies?type=ENTRY", {
          priority: "normal",
        }),
        requestManager.fetch("/api/strategies?type=EXIT", {
          priority: "normal",
        }),
      ]);

      const entryStrategies = entryData.items || [];
      const exitStrategies = exitData.items || [];
      state.strategies = [...entryStrategies, ...exitStrategies];

      updateStrategyLaneCounts(entryStrategies, exitStrategies);

      renderStrategiesList("#entry-strategies", entryStrategies);
      renderStrategiesList("#exit-strategies", exitStrategies);
    } catch (error) {
      console.error("[Trader] Failed to load strategies:", error);
      renderStrategiesError();
    }
  }

  function setStrategiesLoadingState() {
    cleanupStrategyListListeners();
    ["#entry-strategies", "#exit-strategies"].forEach((selector) => {
      const container = $(selector);
      if (!container) return;
      container.innerHTML = `
        <div class="strategy-list-state">
          <i class="icon-loader spinning"></i>
          <span>Loading strategies...</span>
        </div>
      `;
    });
  }

  function cleanupStrategyListListeners() {
    while (strategyListCleanups.length > 0) {
      const cleanup = strategyListCleanups.pop();
      try {
        cleanup();
      } catch {
        /* ignore */
      }
    }
  }

  function updateStrategyLaneCounts(entryStrategies, exitStrategies) {
    const entryEnabled = entryStrategies.filter((strategy) => strategy.enabled).length;
    const exitEnabled = exitStrategies.filter((strategy) => strategy.enabled).length;

    const counts = {
      "#strategy-entry-enabled-label": `${entryEnabled}/${entryStrategies.length} active`,
      "#strategy-exit-enabled-label": `${exitEnabled}/${exitStrategies.length} active`,
    };

    Object.entries(counts).forEach(([selector, value]) => {
      const el = $(selector);
      if (el) el.textContent = String(value);
    });
  }

  function renderStrategiesError() {
    updateStrategyLaneCounts([], []);
    ["#entry-strategies", "#exit-strategies"].forEach((selector) => {
      const container = $(selector);
      if (!container) return;
      container.innerHTML = `
        <div class="strategy-list-state is-error">
          <i class="icon-circle-alert"></i>
          <span>Could not load strategies</span>
        </div>
      `;
    });
  }

  /**
   * Render strategies list
   */
  function renderStrategiesList(selector, strategies) {
    const container = $(selector);
    if (!container) return;

    if (strategies.length === 0) {
      container.innerHTML = `
        <div class="strategy-list-state is-empty">
          <i class="icon-circle"></i>
          <span>No strategies defined</span>
        </div>
      `;
      return;
    }

    container.innerHTML = strategies
      .map((strategy) => {
        const strategyType = String(strategy.strategy_type || "").toUpperCase();
        const isEntry = strategyType === "ENTRY";
        const typeClass = isEntry ? "is-entry" : "is-exit";
        const statusClass = strategy.enabled ? "is-enabled" : "is-disabled";
        const statusLabel = strategy.enabled ? "Enabled" : "Disabled";
        const description = strategy.description
          ? Utils.escapeHtml(strategy.description)
          : "No description provided.";
        const priority =
          strategy.priority !== null && strategy.priority !== undefined
            ? Utils.escapeHtml(String(strategy.priority))
            : "Auto";
        const strategyId = Utils.escapeHtml(String(strategy.id));
        const strategyName = strategy.name
          ? Utils.escapeHtml(String(strategy.name))
          : "Unnamed strategy";

        return `
        <div class="strategy-control-item ${statusClass}">
          <div class="strategy-control-item-header">
            <div class="strategy-control-main">
              <div class="strategy-control-name-row">
                <span class="strategy-control-status-dot" aria-hidden="true"></span>
                <h4 class="strategy-control-name">${strategyName}</h4>
              </div>
              <p class="strategy-control-description">${description}</p>
            </div>
            <label class="toggle">
              <input 
                type="checkbox" 
                data-strategy-id="${strategyId}"
                ${strategy.enabled ? "checked" : ""}
              />
              <span class="toggle-track"></span>
              <span class="toggle-state">${statusLabel}</span>
            </label>
          </div>
          <div class="strategy-control-meta">
            <span class="strategy-control-chip ${typeClass}">
              <i class="${isEntry ? "icon-target" : "icon-log-out"}"></i>
              ${Utils.escapeHtml(strategyType || "STRATEGY")}
            </span>
            <span class="strategy-control-chip">
              <i class="icon-list-ordered"></i>
              Priority ${priority}
            </span>
          </div>
        </div>
      `;
      })
      .join("");

    // Attach event listeners for toggle switches
    container.querySelectorAll('input[type="checkbox"]').forEach((checkbox) => {
      const handler = async (e) => {
        const strategyId = e.target.dataset.strategyId;
        const enabled = e.target.checked;
        e.target.disabled = true;
        await updateStrategyStatus(strategyId, enabled);
      };
      checkbox.addEventListener("change", handler);
      strategyListCleanups.push(() => checkbox.removeEventListener("change", handler));
    });
  }

  /**
   * Update strategy enabled/disabled status
   */
  async function updateStrategyStatus(strategyId, enabled) {
    try {
      await requestManager.fetch(`/api/strategies/${encodeURIComponent(strategyId)}/enabled`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled }),
        priority: "high",
      });

      Utils.showToast({
        type: "success",
        title: enabled ? "Strategy Enabled" : "Strategy Disabled",
        message: enabled ? "Strategy is active" : "Strategy is inactive",
      });
      await loadStrategies();
    } catch (error) {
      console.error("[Trader] Failed to update strategy status:", error);
      Utils.showToast({
        type: "error",
        title: "Update Failed",
        message: "Failed to update strategy status",
      });
      await loadStrategies(); // Reload to reset checkbox
    }
  }

  /**
   * Update time rules status display
   */
  async function updateTimeRulesStatus() {
    try {
      const open = await requestManager.fetch("/api/positions?status=open&limit=0", {
        priority: "normal",
      });
      const positions = Array.isArray(open) ? open : [];

      const statusList = $("#time-positions-status");
      if (!statusList) return;

      if (positions.length === 0) {
        statusList.innerHTML = '<div class="empty-state">No open positions</div>';
        return;
      }

      statusList.innerHTML = positions
        .map((position) => {
          const holdSeconds = position.entry_time ? Date.now() / 1000 - position.entry_time : 0;
          const holdTime = Utils.formatDuration(holdSeconds);
          const roi = position.unrealized_pnl_percent ?? 0;

          return `
            <div class="time-rule-item">
              <div class="time-rule-token">
                ${Utils.escapeHtml(position.symbol || "Unknown")}
              </div>
              <div class="time-rule-metrics">
                <div class="time-rule-metric">
                  <span class="time-rule-label">Hold Time:</span>
                  <span class="time-rule-value">${Utils.escapeHtml(holdTime)}</span>
                </div>
                <div class="time-rule-metric">
                  <span class="time-rule-label">ROI:</span>
                  <span class="time-rule-value ${roi >= 0 ? "value-positive" : "value-negative"}">
                    ${roi >= 0 ? "+" : ""}${roi.toFixed(2)}%
                  </span>
                </div>
              </div>
            </div>
          `;
        })
        .join("");
    } catch (error) {
      console.error("[Trader] Failed to update time rules status:", error);
    }
  }

  /**
   * Setup form submission handlers
   * Note: per-card Save/Reset is handled by the config_cards module.
   */
  function setupFormHandlers() {
    // Setup auto trader toggle handlers
    controls.setupAutoTraderToggles();

    // Stop loss threshold input listener
    const stopLossThreshold = $("#stop-loss-threshold");
    if (stopLossThreshold) {
      addTrackedListener(stopLossThreshold, "input", () => {
        examples.updateStopLossExample();
      });
    }

    // Stop loss min hold input listener
    const stopLossMinHold = $("#stop-loss-min-hold");
    if (stopLossMinHold) {
      addTrackedListener(stopLossMinHold, "input", () => {
        examples.updateStopLossExample();
      });
    }

    // Stop loss allow partial toggle listener
    const stopLossAllowPartial = $("#stop-loss-allow-partial");
    if (stopLossAllowPartial) {
      addTrackedListener(stopLossAllowPartial, "change", () => {
        examples.updateStopLossExample();
      });
    }

    // Time unit change listener
    const timeUnit = $("#time-unit");
    if (timeUnit) {
      addTrackedListener(timeUnit, "change", () => {
        examples.updateTimeConversionHint();
      });
    }

    // Time duration input listener
    const timeMaxHold = $("#time-max-hold");
    if (timeMaxHold) {
      addTrackedListener(timeMaxHold, "input", () => {
        examples.updateTimeConversionHint();
      });
    }

    // ROI target input listener
    const roiTarget = $("#roi-target");
    if (roiTarget) {
      addTrackedListener(roiTarget, "input", () => {
        examples.updateRoiExample();
      });
    }

    // Time loss threshold input listener
    const timeLossThreshold = $("#time-loss-threshold");
    if (timeLossThreshold) {
      addTrackedListener(timeLossThreshold, "input", () => {
        examples.updateTimeLossExample();
      });
    }

    // Realized-window segmented control
    for (const btn of $$("#stats-period .stats-period-btn")) {
      addTrackedListener(btn, "click", () => setStatsPeriod(Number(btn.dataset.days)));
    }
  }

  /**
   * Setup preview event listeners (Phase 2)
   */
  function setupPreviewListeners() {
    // Debounced preview update on config change
    const debouncedTrailingPreview =
      typeof Utils.debounce === "function"
        ? Utils.debounce(() => {
            examples.updateTrailingStopExample();
          }, 300)
        : () => {
            examples.updateTrailingStopExample();
          };

    // Trailing activation input
    const activationInput = $("#trail-activation");
    if (activationInput) {
      addTrackedListener(activationInput, "input", debouncedTrailingPreview);
    }

    // Trailing distance input
    const distanceInput = $("#trail-distance");
    if (distanceInput) {
      addTrackedListener(distanceInput, "input", debouncedTrailingPreview);
    }
  }

  /**
   * Save configuration updates and apply them live to core.
   *
   * `updates` is keyed by config section, e.g. { trader: {...}, positions: {...} }.
   * Each section is sent to its PATCH endpoint (`/api/config/<section>`), which
   * merges the flat partial into the live config, validates, persists, and
   * hot-reloads it — the only correct path (the root `/api/config` is GET-only).
   */
  async function saveConfig(updates, options = {}) {
    const {
      reload = true,
      successTitle = "Configuration Saved",
      successMessage = "Trader settings applied successfully",
    } = options;

    try {
      const sections = Object.entries(updates).filter(
        ([, fields]) => fields && Object.keys(fields).length > 0
      );
      for (const [section, fields] of sections) {
        await requestManager.fetch(`/api/config/${section}`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(fields),
          priority: "high",
        });
      }

      Utils.showToast({
        type: "success",
        title: successTitle,
        message: successMessage,
      });
      if (reload) {
        await loadConfig(); // Reload to reflect the applied values
      } else {
        state.config ||= {};
        sections.forEach(([section, fields]) => {
          state.config[section] = {
            ...(state.config[section] || {}),
            ...fields,
          };
        });
        examples.updateStopLossExample();
        examples.updateRoiExample();
        examples.updateTimeLossExample();
      }
    } catch (error) {
      console.error("[Trader] Failed to save config:", error);
      Utils.showToast({
        type: "error",
        title: "Save Failed",
        message: "Failed to save trader configuration",
      });
      throw error;
    }
  }

  function syncTradingFeatureUi() {
    if (!tabBar || tabBar.container?.dataset.page !== "trader") return;
    applyFeatureStatusToTabs(tradingFeatures, $$);
    if (!isTabUsable(tradingFeatures, state.currentTab)) {
      state.currentTab = DEFAULT_TAB;
      void tabBar.setActive(DEFAULT_TAB, { skipValidation: true });
    }
  }

  // ============================================================================
  // Lifecycle Methods
  // ============================================================================

  return {
    /**
     * Initialize the page
     */
    init(ctx) {
      console.log("[Trader] Initializing page");
      lifecycleContext = ctx;

      // Fetch feature status early (non-blocking, but before tab bar setup)
      const featurePromise = fetchFeatureStatus(requestManager);
      const metadataPromise = requestManager
        .fetch("/api/config/metadata", { priority: "normal" })
        .catch((error) => {
          console.warn("[Trader] Configuration metadata unavailable:", error);
          return null;
        });

      // Per-card Save/Reset controls (injected into each config card header).
      // saveConfig POSTs + hot-reloads + reloads the form, after which
      // loadConfig() calls configCards.snapshot() so the buttons re-hide.
      configCards = createTraderConfigCards({ saveConfig });
      configCards.setup();

      // Initialize tab bar with beforeChange hook for feature validation
      tabBar = new TabBar({
        container: "#subTabsContainer",
        tabs: SUB_TABS,
        defaultTab: DEFAULT_TAB,
        stateKey: "trader.activeTab",
        pageName: "trader",
        onChange: (tabId) => {
          switchTab(tabId);
        },
        beforeChange: (newTabId) => {
          // Check if the tab is usable based on feature status
          return handleFeatureRestrictedTab(tradingFeatures, newTabId, Utils);
        },
      });

      // Register with TabBarManager for page-switch coordination
      TabBarManager.register("trader", tabBar);

      // Integrate with lifecycle for auto-cleanup
      ctx.manageTabBar(tabBar);

      // Show the tab bar
      tabBar.show();

      // Sync state with tab bar's restored state (from server or URL hash)
      const activeTab = tabBar.getActiveTab();
      if (activeTab && activeTab !== state.currentTab) {
        // Ensure the restored tab is usable
        if (isTabUsable(tradingFeatures, activeTab)) {
          state.currentTab = activeTab;
        } else {
          // Fallback to default tab if restored tab is not usable
          state.currentTab = DEFAULT_TAB;
          tabBar.setActive(DEFAULT_TAB);
        }
      }

      // Show the active tab content
      switchTab(state.currentTab, { load: false });

      // Setup form handlers
      setupFormHandlers();

      // Setup trading controls event handlers
      controls.setupControlsEventHandlers();

      // Setup preview listeners (Phase 2)
      setupPreviewListeners();

      // Feature/config metadata enhances the already-painted shell. If it
      // resolves while this cached page is detached, activate() reapplies it.
      void Promise.all([featurePromise, metadataPromise]).then(([features, metadataResponse]) => {
        tradingFeatures = features;
        configCards?.applyMetadata(metadataResponse?.data || metadataResponse || {});
        if (lifecycleContext?.isActive()) syncTradingFeatureUi();
      });
    },

    /**
     * Activate the page (start pollers)
     */
    activate(ctx) {
      console.log("[Trader] Activating page");

      // Re-register deactivate cleanup (cleanups are cleared after each deactivate)
      // and force-show tab bar to handle race conditions with TabBarManager
      if (tabBar) {
        ctx.manageTabBar(tabBar);
        tabBar.show({ force: true });
      }
      syncTradingFeatureUi();

      // Create pollers once and re-register them when a cached page is revisited.
      if (!statsPoller) {
        statsPoller = new Poller(
          async () => {
            if (state.currentTab !== "stats") return;
            // Independent reads, so they go out together instead of queueing behind
            // each other. The status bar is polled here because nothing else polls
            // it: fetched once at init, it kept reporting "Running" forever after
            // the trader had stopped itself.
            await Promise.all([
              loadStats(),
              controls.loadControlsStatus(),
              controls.fetchTraderStatus(),
            ]);
          },
          { label: "Trader Stats", intervalMs: 5000 }
        );
      }

      if (!configPoller) {
        configPoller = new Poller(
          async () => {
            await loadConfig({ preserveUnsavedEdits: true });
          },
          { label: "Trader Config", intervalMs: 10000 }
        );
      }

      if (!strategiesPoller) {
        strategiesPoller = new Poller(
          async () => {
            if (state.currentTab === "strategy-control") {
              await loadStrategies();
            }
          },
          { label: "Strategies", intervalMs: 10000 }
        );
      }

      ctx.managePoller(statsPoller);
      ctx.managePoller(configPoller);
      ctx.managePoller(strategiesPoller);

      // Paint/switch first; it starts only the selected tab's poller and loads.
      switchTab(state.currentTab);
      configPoller.start();

      // Independent first loads never hold the router navigation open.
      void loadConfig();
      if (state.currentTab === "stats") {
        void loadStats();
        void controls.loadControlsStatus();
      } else if (state.currentTab !== "strategy-control") {
        void loadStrategies();
      }
    },

    /**
     * Deactivate the page (pollers stopped automatically)
     */
    deactivate() {
      console.log("[Trader] Deactivating page");
      cleanupStrategyListListeners();
      // Pollers stopped automatically by lifecycle context
    },

    /**
     * Dispose the page (cleanup)
     */
    dispose() {
      console.log("[Trader] Disposing page");

      // Dispose the embedded Strategies editor lifecycle + its pollers
      disposeStrategiesSubtab();

      // Remove the per-card Save/Reset controls + their listeners
      configCards?.dispose();
      configCards = null;

      // Clean up all tracked event listeners
      eventCleanups.forEach((cleanup) => cleanup());
      eventCleanups.length = 0;

      // TabBar cleaned up automatically by manageTabBar
      tabBar = null;
      state.config = null;
      state.stats = null;
      statsPoller = null;
      configPoller = null;
      strategiesPoller = null;
      lifecycleContext = null;
      state.strategies = [];
      _lastDailyKey = null;
      _lastExitKey = null;
    },
  };
}

// Register page
registerPage("trader", createLifecycle());
