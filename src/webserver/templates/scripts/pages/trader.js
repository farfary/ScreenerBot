// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Trader page - sub-tab shell with the stats dashboard, strategy control list and embedded strategies editor.

import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import { formatFixed } from "../core/format.js";
import * as Utils from "../core/utils.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import { DataTable } from "../ui/data_table.js";
import { renderTokenRowCell } from "../ui/token_identity.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { closeReasonText } from "../ui/trade_reason.js";
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

// Snake-case exit_type ids written by src/trader/stats.rs. Every other stored
// closed_reason resolves through `closeReasonText` (ui/trade_reason.js).
const EXIT_TYPE_LABELS = Object.freeze({
  stop_loss: "trader-exit-type-stop-loss",
  take_profit: "trader-exit-type-take-profit",
  roi: "trader-exit-type-roi",
  roi_exit: "trader-exit-type-roi-exit",
  trailing_stop: "trader-exit-type-trailing-stop",
  time_override: "trader-exit-type-time-override",
  time_rule: "trader-exit-type-time-rule",
  manual: "trader-exit-type-manual",
  manual_close: "trader-exit-type-manual-close",
  dca: "trader-exit-type-dca",
  unknown: "trader-exit-type-unknown",
});

// Sub-tabs configuration. Strategy Control is second and the embedded Strategies
// editor is third (Strategies was formerly its own top-level tab).
function buildSubTabs() {
  const tab = (id, icon, label) => ({ id, icon, label });
  return [
    tab("stats", "icon-chart-bar", I18n.t("trader-tab-stats")),
    tab("strategy-control", "icon-puzzle", I18n.t("trader-tab-strategy-control")),
    tab("strategies", "icon-square-pen", I18n.t("trader-tab-strategies")),
    tab("stop-loss", "icon-shield-off", I18n.t("trader-tab-stop-loss")),
    tab("trailing-stop", "icon-trending-up", I18n.t("trader-tab-trailing-stop")),
    tab("roi", "icon-target", I18n.t("trader-tab-roi")),
    tab("time-rules", "icon-timer", I18n.t("trader-tab-time-rules")),
    tab("dca", "icon-coins", I18n.t("trader-tab-dca")),
    tab("general-settings", "icon-settings", I18n.t("trader-tab-settings")),
  ];
}

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
  let timePositions = null;

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
      state.config = data;
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
        title: I18n.t("trader-toast-load-failed"),
        message: I18n.attr("trader-toast-load-failed", "message"),
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
    if (closeCooldown) closeCooldown.value = trader.position_close_cooldown_minutes ?? 15;
    if (entryConcurrency) entryConcurrency.value = trader.entry_check_concurrency ?? 10;
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
      // Tone of a SOL amount as shown (4 decimals): a value that rounds to zero is untoned.
      const tone = (value) => {
        const shown = Utils.signedTone(value, 4);
        return shown === "neutral" ? null : shown;
      };

      // Net P&L — the booked, fee- and DCA-aware SOL the window actually returned.
      setValue(
        "net-pnl",
        Number.isFinite(data.total_pnl_native)
          ? Utils.formatSignedSol(data.total_pnl_native, { fallback: "—" })
          : "—",
        tone(data.total_pnl_native)
      );
      setDetail(
        "net-pnl-detail",
        data.total_trades > 0
          ? I18n.t("trader-stats-won-lost", {
              won: sol(data.gross_profit_native),
              lost: sol(data.gross_loss_native),
            })
          : I18n.t("trader-stats-empty")
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
        data.total_trades > 0
          ? I18n.t("trader-stats-record", {
              wins: I18n.t("trader-stats-wins", {
                count: data.winners,
                amount: String(data.winners),
              }),
              losses: I18n.t("trader-stats-losses", {
                count: data.losers,
                amount: String(data.losers),
              }),
            })
          : "—"
      );

      setValue(
        "profit-factor",
        formatFixed(data.profit_factor),
        Number.isFinite(data.profit_factor)
          ? data.profit_factor >= 1
            ? "positive"
            : "negative"
          : null
      );
      setDetail(
        "profit-factor-detail",
        Number.isFinite(data.expectancy_native)
          ? I18n.t("trader-stats-expected", { amount: sol(data.expectancy_native) })
          : I18n.t("trader-stats-profit-factor-basis")
      );

      setValue(
        "max-drawdown",
        data.total_trades > 0 ? sol(data.max_drawdown_native) : "—",
        data.max_drawdown_native > 0 ? "negative" : null
      );
      setDetail("max-drawdown-detail", I18n.t("trader-stats-drawdown-basis"));

      setValue("capital-at-work", sol(data.locked_native));
      setDetail(
        "capital-at-work-detail",
        I18n.t("trader-stats-slots", {
          count: data.max_open_positions,
          used: String(data.open_positions_count),
          max: String(data.max_open_positions),
        })
      );

      const hasWin = Number.isFinite(data.avg_win_pct);
      const hasLoss = Number.isFinite(data.avg_loss_pct);
      setValue(
        "avg-win-loss",
        hasWin || hasLoss ? `${pct(data.avg_win_pct)} / ${pct(data.avg_loss_pct)}` : "—"
      );
      setDetail("avg-win-loss-detail", I18n.t("trader-stats-avg-basis"));

      setValue("total-trades", data.total_trades > 0 ? String(data.total_trades) : "—");
      setDetail(
        "total-trades-detail",
        I18n.t("trader-stats-closed", {
          count: data.total_trades,
          amount: String(data.total_trades),
        })
      );

      const holdText = (hours) =>
        Number.isFinite(hours) ? Utils.formatUptime(hours * 3600) : "—";
      setValue("median-hold", holdText(data.median_hold_time_hours));
      setDetail(
        "median-hold-detail",
        Number.isFinite(data.avg_hold_time_hours)
          ? I18n.t("trader-stats-hold-average", { span: holdText(data.avg_hold_time_hours) })
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
            : I18n.t("trader-stats-excluded", { count: n, amount: String(n) });
      }

      renderDailyPnl(data.daily_pnl, data.total_pnl_native);
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
        ? Utils.formatSignedSol(totalPnlSol, { fallback: "—" })
        : "—";
      const totalTone = Utils.signedTone(totalPnlSol, 4);
      totalEl.className = `daily-pnl-total${totalTone === "neutral" ? "" : ` ${totalTone}`}`;
    }

    if (!Array.isArray(days) || days.every((d) => (d.trades || 0) === 0)) {
      container.innerHTML = `<div class="info-state"><i class="icon-inbox"></i><span>${Utils.escapeHtml(I18n.t("trader-stats-empty"))}</span></div>`;
      _lastDailyKey = null;
      return;
    }

    const key = days.map((d) => `${d.date}:${d.net_pnl_native.toFixed(6)}`).join("|");
    if (key === _lastDailyKey) return;
    _lastDailyKey = key;

    const W = 100;
    const H = 40;
    const slot = W / days.length;
    const barW = Math.max(slot * 0.62, 0.35);

    const peak = Math.max(...days.map((d) => Math.abs(d.net_pnl_native)), 1e-9);
    const mid = H / 2;
    const bars = days
      .map((d, i) => {
        const h = (Math.abs(d.net_pnl_native) / peak) * (mid - 1);
        const x = i * slot + (slot - barW) / 2;
        const y = d.net_pnl_native >= 0 ? mid - h : mid;
        const cls = d.net_pnl_native >= 0 ? "positive" : "negative";
        return `<rect class="daily-pnl-bar ${cls}" x="${x.toFixed(2)}" y="${y.toFixed(2)}" width="${barW.toFixed(2)}" height="${Math.max(h, 0.4).toFixed(2)}"></rect>`;
      })
      .join("");

    // Cumulative line on its own scale, so a flat run of small days stays readable.
    let running = 0;
    const cumulative = days.map((d) => (running += d.net_pnl_native));
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
    // Days are UTC calendar dates ("2026-08-31"), shown as a short month and day.
    const axisDate = (date) => Utils.formatDate(`${date}T00:00:00Z`, { includeYear: false, utc: true });
    container.dir = "ltr";
    container.innerHTML = `
      <svg class="daily-pnl-chart" viewBox="0 0 ${W} ${H}" preserveAspectRatio="none" role="img"
           aria-label="${Utils.escapeHtml(I18n.t("trader-daily-chart"))}">
        <line class="daily-pnl-zero" x1="0" y1="${mid}" x2="${W}" y2="${mid}"></line>
        ${bars}
        <polyline class="daily-pnl-line" points="${points}"></polyline>
      </svg>
      <div class="daily-pnl-axis">
        <span>${Utils.escapeHtml(axisDate(first.date))}</span>
        <span>${Utils.escapeHtml(axisDate(last.date))}</span>
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

  function formatExitType(type) {
    const text = String(type || "unknown");
    if (Object.hasOwn(EXIT_TYPE_LABELS, text)) return I18n.label(EXIT_TYPE_LABELS, text);
    return closeReasonText(text);
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
      const message =
        periodDays === 1
          ? I18n.t("trader-exit-empty-day")
          : I18n.t("trader-exit-empty-days", { count: periodDays, amount: String(periodDays) });
      container.innerHTML = `<div class="info-state"><i class="icon-inbox"></i><span>${Utils.escapeHtml(message)}</span></div>`;
      _lastExitKey = null;
      return;
    }

    const key = breakdown
      .map((e) => `${e.exit_type}:${e.count}:${(e.net_pnl_native || 0).toFixed(6)}`)
      .join("|");
    if (key === _lastExitKey) return;
    _lastExitKey = key;

    const totalCount = breakdown.reduce((sum, e) => sum + (e.count || 0), 0) || 1;

    container.innerHTML = breakdown
      .map((e) => {
        const count = e.count || 0;
        const avgPct = e.avg_profit_pct || 0;
        const netSol = e.net_pnl_native || 0;
        const share = Math.round((count / totalCount) * 100);
        const barClass = netSol >= 0 ? "positive" : "negative";
        return `
          <div class="exit-breakdown-row">
            <div class="exit-breakdown-head">
              <span class="exit-breakdown-type">${Utils.escapeHtml(formatExitType(e.exit_type))}</span>
              <span class="exit-breakdown-pnl ${Utils.signedTone(netSol, 4)}">${Utils.formatSignedSol(netSol, { fallback: "—" })}</span>
            </div>
            <div class="exit-breakdown-bar">
              <div class="exit-breakdown-fill ${barClass}" style="width: ${share}%"></div>
            </div>
            <div class="exit-breakdown-meta">
              <span class="exit-breakdown-share">${Utils.escapeHtml(
                I18n.t("trader-exit-share", {
                  count,
                  amount: String(count),
                  share: Utils.formatPercentValue(share, { decimals: 0, includeSign: false }),
                })
              )}</span>
              <span class="exit-breakdown-profit ${avgPct >= 0 ? "positive" : "negative"}">${Utils.escapeHtml(
                I18n.t("trader-exit-average", {
                  value: Utils.formatPercentValue(avgPct, { decimals: 1 }),
                })
              )}</span>
            </div>
          </div>`;
      })
      .join("");
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
          <span>${Utils.escapeHtml(I18n.t("trader-strategy-loading"))}</span>
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
      "#strategy-entry-enabled-label": I18n.t("trader-strategy-active", {
        enabled: String(entryEnabled),
        total: String(entryStrategies.length),
      }),
      "#strategy-exit-enabled-label": I18n.t("trader-strategy-active", {
        enabled: String(exitEnabled),
        total: String(exitStrategies.length),
      }),
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
          <span>${Utils.escapeHtml(I18n.t("trader-strategy-load-failed"))}</span>
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
          <span>${Utils.escapeHtml(I18n.t("trader-strategy-empty"))}</span>
        </div>
      `;
      return;
    }

    container.innerHTML = strategies
      .map((strategy) => {
        // The toggle is the card's one state signal, and the lane title already names
        // the strategy type, so the card carries neither a state word nor a type chip.
        const statusClass = strategy.enabled ? "is-enabled" : "is-disabled";
        const description = Utils.escapeHtml(
          strategy.description || I18n.t("trader-strategy-no-description")
        );
        const priority = Utils.escapeHtml(
          I18n.t("trader-strategy-priority", {
            priority:
              strategy.priority !== null && strategy.priority !== undefined
                ? String(strategy.priority)
                : I18n.t("trader-strategy-priority-auto"),
          })
        );
        const strategyId = Utils.escapeHtml(String(strategy.id));
        const strategyName = Utils.escapeHtml(
          strategy.name ? String(strategy.name) : I18n.t("trader-strategy-unnamed")
        );

        return `
        <div class="strategy-control-item ${statusClass}">
          <div class="strategy-control-item-header">
            <div class="strategy-control-main">
              <h4 class="strategy-control-name">${strategyName}</h4>
              <p class="strategy-control-description">${description}</p>
            </div>
            <label class="toggle">
              <input 
                type="checkbox" 
                data-strategy-id="${strategyId}"
                aria-label="${strategyName}"
                ${strategy.enabled ? "checked" : ""}
              />
              <span class="toggle-track"></span>
            </label>
          </div>
          <div class="strategy-control-meta">
            <span class="strategy-control-chip">
              <i class="icon-list-ordered"></i>
              ${priority}
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
        title: enabled
          ? I18n.t("trader-toast-strategy-enabled")
          : I18n.t("trader-toast-strategy-disabled"),
        message: enabled
          ? I18n.attr("trader-toast-strategy-enabled", "message")
          : I18n.attr("trader-toast-strategy-disabled", "message"),
      });
      await loadStrategies();
    } catch (error) {
      console.error("[Trader] Failed to update strategy status:", error);
      Utils.showToast({
        type: "error",
        title: I18n.t("trader-toast-strategy-failed"),
        message: I18n.attr("trader-toast-strategy-failed", "message"),
      });
      await loadStrategies(); // Reload to reset checkbox
    }
  }

  /** The open positions on the Time Rules tab: one shared DataTable, created once and refreshed in place. */
  function timePositionsTable(container) {
    if (timePositions && timePositions.elements.container !== container) {
      timePositions.destroy();
      timePositions = null;
    }
    if (!timePositions) {
      timePositions = new DataTable({
        container,
        columns: [
          {
            id: "symbol",
            label: I18n.t("trader-time-positions-token"),
            grow: true,
            sortable: true,
            minWidth: 200,
            wrap: false,
            render: (_value, row) =>
              renderTokenRowCell(row.mint, {
                symbol: row.symbol,
                name: row.name,
                logoUrl: row.logo_url || row.image_url,
              }),
          },
          {
            id: "hold_seconds",
            label: I18n.t("trader-time-positions-hold"),
            type: "number",
            sortable: true,
            render: (value) =>
              value == null
                ? "—"
                : Utils.escapeHtml(Utils.formatUptime(value, { style: "trimmed" })),
          },
          {
            id: "unrealized_pnl_percent",
            label: I18n.t("trader-time-positions-roi"),
            type: "percent",
            sortable: true,
            render: (value) =>
              Utils.formatPercent(value, { style: "pnl", decimals: 2, fallback: "—" }),
          },
        ],
        rowIdField: "id",
        stateKey: "trader.time-positions",
        compact: true,
        zebra: true,
        fitToContainer: true,
        sorting: { mode: "client", column: "hold_seconds", direction: "desc" },
        emptyTitle: I18n.t("trader-time-positions-empty"),
        emptyMessage: I18n.t("trader-time-positions-empty-message"),
      });
    }
    return timePositions;
  }

  /** Refresh the Time Rules open-position table with each position's hold time. */
  async function updateTimeRulesStatus() {
    try {
      const open = await requestManager.fetch("/api/positions?status=open&limit=0", {
        priority: "normal",
      });
      const container = $("#time-positions-status");
      if (!container) return;
      const now = Date.now() / 1000;
      const rows = (Array.isArray(open) ? open : []).map((position) => ({
        ...position,
        hold_seconds: position.entry_time ? Math.max(0, now - position.entry_time) : null,
      }));
      timePositionsTable(container).setData(rows);
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
      successTitle = I18n.t("trader-toast-saved"),
      successMessage = I18n.attr("trader-toast-saved", "message"),
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
        title: I18n.t("trader-toast-save-failed"),
        message: I18n.attr("trader-toast-save-failed", "message"),
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
        tabs: buildSubTabs(),
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
          { label: "Trader Stats", intervalMs: 5000 } // l10n-ignore: poller label used in logs only
        );
      }

      if (!configPoller) {
        configPoller = new Poller(
          async () => {
            await loadConfig({ preserveUnsavedEdits: true });
          },
          { label: "Trader Config", intervalMs: 10000 } // l10n-ignore: poller label used in logs only
        );
      }

      if (!strategiesPoller) {
        strategiesPoller = new Poller(
          async () => {
            if (state.currentTab === "strategy-control") {
              await loadStrategies();
            }
          },
          { label: "Strategies", intervalMs: 10000 } // l10n-ignore: poller label used in logs only
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

      timePositions?.destroy();
      timePositions = null;

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
