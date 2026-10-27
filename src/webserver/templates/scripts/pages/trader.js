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
import { renderStateView } from "../ui/state_view.js";
import { renderTokenRowCell } from "../ui/token_identity.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { closeReasonText } from "../ui/trade_reason.js";
import { requestManager } from "../core/request_manager.js";
import { LOSS_MAGNITUDE, createTraderConfigCards, displayValue } from "./trader/config_cards.js";
import { playToggleOn, playToggleOff, playError } from "../core/sounds.js";
import { createExampleUpdaters } from "./trader/examples.js";
import { refreshLevels, renderLevels } from "./trader/level_list.js";
import { timeExitStatus } from "./trader/time_status.js";
import { createTraderControls } from "./trader/controls.js";
import {
  fetchFeatureStatus,
  isTabUsable,
  applyFeatureStatusToTabs,
  handleFeatureRestrictedTab,
} from "./trader/features.js";
import { createLifecycle as createStrategiesLifecycle } from "./strategies.js";
import { createStrategyMode, strategyModeRow } from "./trader/strategy_mode.js";
import { createPaperAccount } from "./trader/paper_account.js";
import { gateControl, renderSetupGate, setupRequired } from "../ui/setup_gate.js";
import { RANGE_OPTIONS, renderSegmented } from "../ui/segmented.js";
import { TRADING_MODE_ICONS, tradingModeLabel } from "../ui/trading_mode.js";

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
  const tab = (id, label) => ({ id, label });
  return [
    tab("stats", I18n.t("trader-tab-stats")),
    tab("strategy-control", I18n.t("trader-tab-strategy-control")),
    tab("strategies", I18n.t("trader-tab-strategies")),
    tab("stop-loss", I18n.t("trader-tab-stop-loss")),
    tab("trailing-stop", I18n.t("trader-tab-trailing-stop")),
    tab("take-profit", I18n.t("trader-tab-take-profit")),
    tab("time-rules", I18n.t("trader-tab-time-rules")),
    tab("dca", I18n.t("trader-tab-dca")),
    tab("general-settings", I18n.t("trader-tab-settings")),
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

  // Realized window for the Stats tab (`24h`, `7d`, `30d` or `all`). Owned here and sent
  // to the API — the tab never labels a window it did not ask for.
  let statsWindow = "30d";
  // The book the Stats tab reads: the live book by default, the paper book on request.
  // The two are never summed into one figure.
  let statsBook = "live";

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
    // Whether an exit strategy is enabled; `null` until the strategy list loads.
    exitStrategiesOn: null,
  };

  // Initialize sub-modules
  const examples = createExampleUpdaters({
    $,
    Utils,
    tradeSize: () => state.config?.trader?.trade_size_sol,
    traderConfig: () => state.config?.trader,
    exitStrategiesOn: () => state.exitStrategiesOn,
  });
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

  const strategyMode = createStrategyMode({ requestManager, ConfirmationDialog, Utils });
  const paperAccount = createPaperAccount({
    requestManager,
    ConfirmationDialog,
    Utils,
    onReset: () => void loadStats(),
  });

  // Embedded Strategies subtab — drives the strategies page module's lifecycle.
  // A local ctx adapter owns the strategies pollers so they start when the
  // subtab opens and stop when it is left (instead of running page-wide).
  let strategiesLifecycle = null;
  let strategiesInited = false;
  let strategiesActive = false;
  // What Strategy Control asked the editor to open, handed over once it is active.
  let strategiesRequest = null;
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
      if (strategiesRequest) {
        strategiesLifecycle.open(strategiesRequest);
        strategiesRequest = null;
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
    const changed = state.currentTab !== tabId;
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
      "take-profit": "take-profit-tab",
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
    // A newly chosen sub-tab opens at its top, whatever the previous one was scrolled to.
    if (changed && traderContent) {
      traderContent.scrollTop = 0;
      content?.querySelectorAll(".stats-col").forEach((column) => {
        column.scrollTop = 0;
      });
    }

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

    // The runner warning reads the exit strategies, which change on their own tabs.
    if (tabId === "take-profit") {
      void loadStrategies();
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
      examples.updateTakeProfitExample();
      examples.updateTimeLossExample();
      examples.updateTrailingStopExample();
      examples.updateBreakEvenExample();
      examples.updateStagnationExample();
      examples.updateDcaExample();
    } catch (error) {
      console.error("[Trader] Failed to load config:", error);
      Utils.showToast({
        type: "error",
        title: I18n.t("trader-toast-load-failed"),
        message: I18n.attr("trader-toast-load-failed", "message"),
      });
    }
  }

  /** "Part to Sell" is shown only while the stop loss may sell part of the position first. */
  function syncStopLossPartialField() {
    const group = $("#stop-loss-partial-pct-group");
    const allowPartial = $("#stop-loss-allow-partial");
    if (group && allowPartial) group.hidden = !allowPartial.checked;
  }

  /**
   * Update form fields from config state
   */
  function updateFormFields() {
    if (!state.config) return;

    const trader = state.config.trader || {};

    // Stop Loss
    const stopLossEnabled = $("#stop-loss-enabled");
    const stopLossThreshold = $("#stop-loss-threshold");
    const stopLossAllowPartial = $("#stop-loss-allow-partial");
    const stopLossPartialPct = $("#stop-loss-partial-pct");
    const stopLossMinHold = $("#stop-loss-min-hold");
    if (stopLossEnabled) {
      stopLossEnabled.checked = trader.stop_loss_enabled === true;
    }
    if (stopLossThreshold) {
      stopLossThreshold.value = trader.stop_loss_threshold_pct;
    }
    if (stopLossAllowPartial) {
      stopLossAllowPartial.checked = trader.stop_loss_allow_partial === true;
    }
    if (stopLossPartialPct) {
      stopLossPartialPct.value = trader.stop_loss_partial_pct;
    }
    if (stopLossMinHold) {
      stopLossMinHold.value = trader.stop_loss_min_hold_seconds;
    }
    syncStopLossPartialField();

    // Trailing Stop
    const trailingEnabled = $("#trailing-enabled");
    const trailActivation = $("#trail-activation");
    const trailDistance = $("#trail-distance");
    if (trailingEnabled) {
      trailingEnabled.checked = trader.trailing_stop_enabled === true;
    }
    if (trailActivation) {
      trailActivation.value = trader.trailing_stop_activation_pct;
    }
    if (trailDistance) {
      trailDistance.value = trader.trailing_stop_distance_pct;
    }
    const trailingBands = $("#trailing-bands");
    if (trailingBands) {
      renderLevels(
        trailingBands,
        trader.trailing_stop_band_profit_pct,
        trader.trailing_stop_band_distance_pct
      );
    }

    // Break-even Stop
    const breakEvenEnabled = $("#break-even-enabled");
    const breakEvenTrigger = $("#break-even-trigger");
    const breakEvenOffset = $("#break-even-offset");
    if (breakEvenEnabled) {
      breakEvenEnabled.checked = trader.break_even_enabled === true;
    }
    if (breakEvenTrigger) {
      breakEvenTrigger.value = trader.break_even_trigger_pct;
    }
    if (breakEvenOffset) {
      breakEvenOffset.value = trader.break_even_offset_pct;
    }

    // Take profit
    const takeProfitEnabled = $("#take-profit-enabled");
    const takeProfitLevels = $("#take-profit-levels");
    if (takeProfitEnabled) {
      takeProfitEnabled.checked = trader.take_profit_enabled === true;
    }
    if (takeProfitLevels) {
      renderLevels(takeProfitLevels, trader.take_profit_levels_pct, trader.take_profit_sell_pct);
    }

    // Time Rules
    const timeOverrideEnabled = $("#time-override-enabled");
    const timeMaxHold = $("#time-max-hold");
    const timeUnit = $("#time-unit");
    const timeLossThreshold = $("#time-loss-threshold");

    if (timeOverrideEnabled) {
      timeOverrideEnabled.checked = trader.time_override_enabled === true;
    }
    if (timeMaxHold) {
      timeMaxHold.value = trader.time_override_duration;
    }
    if (timeUnit) {
      timeUnit.value = trader.time_override_unit;
    }
    if (timeLossThreshold) {
      timeLossThreshold.value = displayValue(
        LOSS_MAGNITUDE,
        trader.time_override_loss_threshold_percent
      );
    }

    // Update time conversion hint
    examples.updateTimeConversionHint();

    // Close Flat Positions
    const stagnationEnabled = $("#time-stagnation-enabled");
    const stagnationMinutes = $("#time-stagnation-minutes");
    const stagnationBand = $("#time-stagnation-band");
    if (stagnationEnabled) {
      stagnationEnabled.checked = trader.time_exit_stagnation_enabled === true;
    }
    if (stagnationMinutes) {
      stagnationMinutes.value = trader.time_exit_stagnation_minutes;
    }
    if (stagnationBand) {
      stagnationBand.value = trader.time_exit_stagnation_band_pct;
    }

    // General Settings
    const maxPositions = $("#max-positions");
    const tradeSize = $("#trade-size");
    const entrySizes = $("#entry-sizes");
    const dcaEnabled = $("#dca-enabled");
    const dcaMeasureFrom = $("#dca-measure-from");
    const dcaLevels = $("#dca-levels");
    const dcaCooldown = $("#dca-cooldown");
    const dcaMinHold = $("#dca-min-hold");
    const closeCooldown = $("#close-cooldown");
    const entryConcurrency = $("#entry-concurrency");

    if (maxPositions) maxPositions.value = trader.max_open_positions;
    if (tradeSize) tradeSize.value = trader.trade_size_sol;
    if (entrySizes) entrySizes.value = trader.entry_sizes.join(", ");
    if (dcaEnabled) dcaEnabled.checked = trader.dca_enabled === true;
    if (dcaMeasureFrom) dcaMeasureFrom.value = trader.dca_measure_from;
    if (dcaLevels) renderLevels(dcaLevels, trader.dca_levels_pct, trader.dca_size_multipliers);
    if (dcaCooldown) dcaCooldown.value = trader.dca_cooldown_minutes;
    if (dcaMinHold) dcaMinHold.value = trader.dca_min_hold_minutes;
    if (closeCooldown) closeCooldown.value = trader.position_close_cooldown_minutes;
    if (entryConcurrency) entryConcurrency.value = trader.entry_check_concurrency;
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
      const data = await requestManager.fetch(
        `/api/trader/stats?window=${statsWindow}&book=${statsBook}`,
        { priority: "normal" }
      );
      if (statsBook === "paper") void paperAccount.load($("#stats-paper-account"));

      const pct = (value, decimals = 1) =>
        Utils.formatPercentValue(value, { decimals, fallback: "—" });
      const sol = (value, decimals = 4) => Utils.formatSol(value, { decimals, fallback: "—" });
      const setValue = (id, text, tone) => {
        const el = $(`#${id}`);
        if (!el) return;
        el.textContent = text;
        el.classList.remove("positive", "negative");
        if (tone) el.classList.add(tone);
        if (el.classList.contains("metric-value-fit")) {
          el.style.setProperty("--metric-value-chars", String([...text].length));
        }
      };
      const setDetail = (id, text) => {
        const el = $(`#${id}`);
        if (el) el.textContent = text;
      };
      // Tone of a SOL amount as shown (4 decimals): a value that rounds to zero is untoned.
      // Only signed amounts take a tone; a ratio (win rate, profit factor) stays neutral.
      const tone = (value) => {
        const shown = Utils.signedTone(value, 4);
        return shown === "neutral" ? null : shown;
      };

      // A window without closed trades leaves every ratio card without a basis; its
      // subtitle then says why instead of repeating the value's dash.
      const hasTrades = data.total_trades > 0;

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
        hasTrades
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
        })
      );
      // Counts are facts even in an empty window: 0 wins · 0 losses, never a dash.
      setDetail(
        "win-rate-detail",
        Number.isFinite(data.winners) && Number.isFinite(data.losers)
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
          : I18n.t("trader-stats-empty")
      );

      setValue("profit-factor", formatFixed(data.profit_factor));
      setDetail(
        "profit-factor-detail",
        !hasTrades
          ? I18n.t("trader-stats-empty")
          : Number.isFinite(data.expectancy_native)
            ? I18n.t("trader-stats-expected", { amount: sol(data.expectancy_native) })
            : I18n.t("trader-stats-profit-factor-basis")
      );

      const hasDrawdown =
        hasTrades && Utils.signedTone(data.max_drawdown_native, 4) === "positive";
      setValue(
        "max-drawdown",
        hasTrades ? sol(data.max_drawdown_native) : "—",
        hasDrawdown ? "negative" : null
      );
      setDrawdownAccent(hasDrawdown);
      setDetail(
        "max-drawdown-detail",
        hasTrades ? I18n.t("trader-stats-drawdown-basis") : I18n.t("trader-stats-empty")
      );

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
      setDetail(
        "avg-win-loss-detail",
        hasWin || hasLoss ? I18n.t("trader-stats-avg-basis") : I18n.t("trader-stats-empty")
      );

      setValue(
        "total-trades",
        Number.isFinite(data.total_trades) ? String(data.total_trades) : "—"
      );
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
          : I18n.t("trader-stats-empty")
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
          el.classList.remove("positive", "negative");
          el.style.removeProperty("--metric-value-chars");
        }
      }
      setDrawdownAccent(false);
    }
  }

  /** The danger accent is a state: it marks a drawdown that happened, never an empty card. */
  function setDrawdownAccent(hasDrawdown) {
    $("#max-drawdown")
      ?.closest(".metric-card")
      ?.classList.toggle("metric-highlight-danger", hasDrawdown);
  }

  /**
   * Switch the realized window and reload immediately, so the click is answered by
   * the panel rather than by the next poll tick.
   */
  function setStatsPeriod(range) {
    if (range === statsWindow) return;
    statsWindow = range;
    _lastDailyKey = null;
    _lastExitKey = null;
    void loadStats();
  }

  /** Paint the Stats tab's book and window switches at the current selection. */
  function renderStatsControls() {
    const book = $("#stats-book");
    if (book) {
      book.innerHTML = renderSegmented(
        {
          name: "stats-book",
          options: ["live", "paper"].map((mode) => ({
            value: mode,
            label: tradingModeLabel(mode),
            icon: TRADING_MODE_ICONS[mode],
          })),
          value: statsBook,
          ariaLabel: I18n.attr("trader-stats-book", "aria-label"),
          compact: true,
        },
        Utils.escapeHtml
      );
    }
    const period = $("#stats-period");
    if (period) {
      period.innerHTML = renderSegmented(
        {
          name: "stats-period",
          options: RANGE_OPTIONS,
          value: statsWindow,
          ariaLabel: I18n.attr("trader-stats-window", "aria-label"),
          compact: true,
        },
        Utils.escapeHtml
      );
    }
  }

  /** Switch the book the Stats tab reads; the paper book also shows its run card. */
  function setStatsBook(book) {
    if (book === statsBook) return;
    statsBook = book;
    const card = $("#stats-paper-account");
    if (card) {
      card.hidden = book !== "paper";
      if (book !== "paper") card.replaceChildren();
    }
    _lastDailyKey = null;
    _lastExitKey = null;
    void loadStats();
  }

  /**
   * Name the P&L series by the bucket the server chose: one bar per day, or per ISO week
   * on a history too long for daily bars. Rewritten only when the bucket changes.
   */
  function setPnlSeriesHeading(weekly) {
    const title = $("#daily-pnl-title");
    const subtitle = $("#daily-pnl-subtitle");
    const titleId = weekly ? "trader-weekly-title" : "trader-daily-title";
    const subtitleId = weekly ? "trader-weekly-subtitle" : "trader-daily-subtitle";
    if (title && title.getAttribute("data-l10n-id") !== titleId) {
      title.setAttribute("data-l10n-id", titleId);
      I18n.localizeTree(title);
    }
    if (subtitle && subtitle.getAttribute("data-l10n-id") !== subtitleId) {
      subtitle.setAttribute("data-l10n-id", subtitleId);
      I18n.localizeTree(subtitle);
    }
  }

  /**
   * Render the realized P&L series: one bar per day, or per week on a long history, plus
   * the cumulative line.
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

    const weekly = Array.isArray(days) && days[0]?.bucket === "week";
    setPnlSeriesHeading(weekly);

    if (!Array.isArray(days) || days.every((d) => (d.trades || 0) === 0)) {
      container.innerHTML = renderStateView({
        icon: "icon-inbox",
        message: I18n.t("trader-stats-empty"),
        compact: true,
      });
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
        const cls = Utils.signedTone(d.net_pnl_native, 4);
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
    // Buckets start on UTC calendar dates ("2026-08-31"), shown as a short month and day.
    const axisDate = (date) => Utils.formatDate(`${date}T00:00:00Z`, { includeYear: false, utc: true });
    container.dir = "ltr";
    container.innerHTML = `
      <svg class="daily-pnl-chart" viewBox="0 0 ${W} ${H}" preserveAspectRatio="none" role="img"
           aria-label="${Utils.escapeHtml(I18n.t(weekly ? "trader-weekly-chart" : "trader-daily-chart"))}">
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
        valueEl.className = `stats-extreme-value ${Utils.signedTone(value, 1)}`;
      }
      // A side with no known trade reads as one dash: its value's, with no token beside it.
      if (tokenEl) {
        tokenEl.textContent = token || "";
        tokenEl.hidden = !(Number.isFinite(value) && token);
      }
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
      // A null window is the whole history of the book.
      let message;
      if (periodDays == null) message = I18n.t("trader-exit-empty-all");
      else if (periodDays === 1) message = I18n.t("trader-exit-empty-day");
      else
        message = I18n.t("trader-exit-empty-days", {
          count: periodDays,
          amount: String(periodDays),
        });
      container.innerHTML = renderStateView({ icon: "icon-inbox", message, compact: true });
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
        const barClass = Utils.signedTone(netSol, 4);
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
              <span class="exit-breakdown-profit ${Utils.signedTone(avgPct, 1)}">${Utils.escapeHtml(
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
      state.exitStrategiesOn = exitStrategies.some((strategy) => strategy.enabled === true);
      examples.updateTakeProfitExample();

      updateStrategyLaneCounts(entryStrategies, exitStrategies);

      // Each render replaces the cards, so the listeners of the previous ones go first.
      cleanupStrategyListListeners();
      renderStrategiesList("#entry-strategies", entryStrategies, "ENTRY");
      renderStrategiesList("#exit-strategies", exitStrategies, "EXIT");
      renderPaperSetupNotice();
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
      container.innerHTML = renderStateView({
        kind: "loading",
        message: I18n.t("trader-strategy-loading"),
        compact: true,
      });
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

    const totals = {
      "#strategy-entry-enabled-label": entryStrategies.length,
      "#strategy-exit-enabled-label": exitStrategies.length,
    };
    Object.entries(counts).forEach(([selector, value]) => {
      const el = $(selector);
      if (!el) return;
      el.textContent = String(value);
      // A lane with nothing defined has nothing to signal: its count reads neutral.
      el.classList.toggle("is-empty", totals[selector] === 0);
    });
  }

  function renderStrategiesError() {
    updateStrategyLaneCounts([], []);
    ["#entry-strategies", "#exit-strategies"].forEach((selector) => {
      const container = $(selector);
      if (!container) return;
      container.innerHTML = renderStateView({
        kind: "error",
        icon: "icon-circle-alert",
        message: I18n.t("trader-strategy-load-failed"),
        compact: true,
      });
    });
  }

  /**
   * In Explore Mode no paper or live trade runs: the tab says so above the strategies
   * and offers the setup that enables paper trading.
   */
  function renderPaperSetupNotice() {
    const notice = $("#strategy-paper-setup");
    if (!notice) return;
    const explore = setupRequired();
    notice.hidden = !explore;
    if (explore && !notice.firstChild) {
      renderSetupGate(notice, I18n.t("trader-paper-explore-title"));
    }
  }

  /** Show the embedded editor with `request` (`{ strategyId }` or `{ create }`) acted on. */
  function openStrategiesEditor(request) {
    strategiesRequest = request;
    void tabBar?.setActive("strategies");
  }

  function renderStrategiesList(selector, strategies, type) {
    const container = $(selector);
    if (!container) return;

    if (strategies.length === 0) {
      container.innerHTML = renderStateView({
        icon: "icon-puzzle",
        message: I18n.t("trader-strategy-empty"),
        action: { id: "new-strategy", label: I18n.t("strategies-new"), primary: true },
        compact: true,
      });
      const create = container.querySelector('[data-action="new-strategy"]');
      const handler = () => openStrategiesEditor({ create: type });
      create?.addEventListener("click", handler);
      strategyListCleanups.push(() => create?.removeEventListener("click", handler));
      return;
    }

    container.innerHTML = strategies
      .map((strategy) => {
        // The toggle is the card's one state signal, and the lane title already names
        // the strategy type, so the card carries neither a state word nor a type chip.
        const statusClass = strategy.enabled ? "is-enabled" : "is-disabled";
        const description = String(strategy.description ?? "").trim();
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
              ${description ? `<p class="strategy-control-description">${Utils.escapeHtml(description)}</p>` : ""}
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
            <button
              type="button"
              class="btn btn-sm btn-ghost strategy-control-edit"
              data-edit-strategy="${strategyId}"
            >
              <i class="icon-square-pen" aria-hidden="true"></i>
              <span>${Utils.escapeHtml(I18n.t("common-action-edit"))}</span>
            </button>
          </div>
          ${strategyModeRow(strategy, Utils.escapeHtml)}
        </div>
      `;
      })
      .join("");

    container.querySelectorAll("[data-edit-strategy]").forEach((button) => {
      const strategy = strategies.find((item) => String(item.id) === button.dataset.editStrategy);
      const handler = () => openStrategiesEditor({ strategyId: strategy?.id ?? null });
      button.addEventListener("click", handler);
      strategyListCleanups.push(() => button.removeEventListener("click", handler));
    });

    // The mode actions: arming live is gated behind its dialog, and Explore Mode runs no
    // trades, so there the actions stay disabled and name the setup they need.
    const modeActions = [
      ["[data-arm-strategy]", "armStrategy", strategyMode.armLive],
      ["[data-paper-strategy]", "paperStrategy", strategyMode.returnToPaper],
    ];
    for (const [selector, key, act] of modeActions) {
      container.querySelectorAll(selector).forEach((button) => {
        gateControl(button);
        const strategy = strategies.find((item) => String(item.id) === button.dataset[key]);
        if (!strategy) return;
        const handler = async () => {
          button.disabled = true;
          const changed = await act(strategy);
          if (changed) await loadStrategies();
          else button.disabled = false;
        };
        button.addEventListener("click", handler);
        strategyListCleanups.push(() => button.removeEventListener("click", handler));
      });
    }

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
          {
            id: "time_exit",
            label: I18n.t("trader-time-positions-rule"),
            sortable: false,
            render: (value) => Utils.escapeHtml(value),
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

  /**
   * Refresh the Time Rules open-position table with each position's hold time and the
   * time exit that applies to it.
   */
  async function updateTimeRulesStatus() {
    try {
      // One table: each open position joined to the time exit the route computed for it.
      const [open, watches] = await Promise.all([
        requestManager.fetch("/api/positions?status=open&limit=0", { priority: "normal" }),
        requestManager.fetch("/api/trader/time-exits", { priority: "normal" }),
      ]);
      const container = $("#time-positions-status");
      if (!container) return;
      const now = Date.now() / 1000;
      const watchById = new Map(
        (Array.isArray(watches) ? watches : []).map((entry) => [entry.position_id, entry.time_exit])
      );
      const rows = (Array.isArray(open) ? open : []).map((position) => {
        const holdSeconds = position.entry_time ? Math.max(0, now - position.entry_time) : null;
        return {
          ...position,
          hold_seconds: holdSeconds,
          time_exit: timeExitStatus(watchById.get(position.id), holdSeconds, now),
        };
      });
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

    // "Sell Part First" toggle: `input` also fires when a card Reset restores it.
    const stopLossAllowPartial = $("#stop-loss-allow-partial");
    if (stopLossAllowPartial) {
      addTrackedListener(stopLossAllowPartial, "input", () => {
        syncStopLossPartialField();
        examples.updateStopLossExample();
      });
    }

    const stopLossPartialPct = $("#stop-loss-partial-pct");
    if (stopLossPartialPct) {
      addTrackedListener(stopLossPartialPct, "input", () => {
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

    // Take-profit ladder listener: rows are redrawn on add and remove, so the
    // ladder element itself carries the listener.
    const takeProfitLevels = $("#take-profit-levels");
    if (takeProfitLevels) {
      addTrackedListener(takeProfitLevels, "input", () => {
        examples.updateTakeProfitExample();
      });
    }

    // The other exits that can close a take-profit runner: the runner warning follows them.
    for (const id of [
      "#trailing-enabled",
      "#break-even-enabled",
      "#time-override-enabled",
      "#time-stagnation-enabled",
    ]) {
      addTrackedListener($(id), "change", () => examples.updateTakeProfitExample());
    }

    // Break-even fields: the card's computed line
    for (const id of ["#break-even-trigger", "#break-even-offset"]) {
      addTrackedListener($(id), "input", () => examples.updateBreakEvenExample());
    }

    // Flat-position fields: the card's computed line
    for (const id of ["#time-stagnation-minutes", "#time-stagnation-band"]) {
      addTrackedListener($(id), "input", () => examples.updateStagnationExample());
    }

    // Time loss threshold input listener
    const timeLossThreshold = $("#time-loss-threshold");
    if (timeLossThreshold) {
      addTrackedListener(timeLossThreshold, "input", () => {
        examples.updateTimeLossExample();
      });
    }

    // DCA fields: the ladder's amounts and the tab's computed lines. The ladder's rows
    // are redrawn on add and remove, so the ladder element itself carries the listener.
    for (const id of ["#dca-levels", "#dca-cooldown", "#dca-min-hold"]) {
      addTrackedListener($(id), "input", () => examples.updateDcaExample());
    }
    // The basis decides whether the ladder's steps must deepen.
    addTrackedListener($("#dca-measure-from"), "change", () => {
      refreshLevels($("#dca-levels"));
      examples.updateDcaExample();
    });

    // Trailing bands must sit above the activation: the band list re-checks on its change.
    addTrackedListener($("#trail-activation"), "input", () => refreshLevels($("#trailing-bands")));

    // The Stats tab's book and realized-window switches
    renderStatsControls();
    addTrackedListener($("#stats-book"), "change", (event) => setStatsBook(event.target.value));
    addTrackedListener($("#stats-period"), "change", (event) => setStatsPeriod(event.target.value));
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
        examples.updateTakeProfitExample();
        examples.updateTimeLossExample();
        examples.updateBreakEvenExample();
        examples.updateStagnationExample();
        examples.updateDcaExample();
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
        // Without metadata (its fetch failure is logged above) the cards keep their markup.
        if (metadataResponse?.data) configCards?.applyMetadata(metadataResponse.data);
        examples.updateTakeProfitExample();
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
      state.exitStrategiesOn = null;
      _lastDailyKey = null;
      _lastExitKey = null;
    },
  };
}

// Register page
registerPage("trader", createLifecycle());
