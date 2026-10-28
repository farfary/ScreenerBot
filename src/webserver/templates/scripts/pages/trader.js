// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Trader page - sub-tab shell with the stats dashboard, the embedded strategies roster and editor, and the rule-exit and settings tabs.

import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import { DataTable } from "../ui/data_table.js";
import { renderTokenRowCell } from "../ui/token_identity.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
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
import { createStatsView } from "./trader/stats.js";

// Sub-tabs configuration. The embedded Strategies roster and editor is second.
function buildSubTabs() {
  const tab = (id, label) => ({ id, label });
  return [
    tab("stats", I18n.t("trader-tab-stats")),
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
  let lifecycleContext = null;
  let timePositions = null;

  // Event cleanup tracking
  const eventCleanups = [];

  // Feature status from API
  let tradingFeatures = {};

  // Page state
  const state = {
    currentTab: DEFAULT_TAB,
    config: null,
    stats: null,
    // Whether an exit strategy is enabled; `null` until the strategies overview loads.
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
  // The Stats view owns the shown book; the controls read it for the entries chip.
  let statsView = null;
  const controls = createTraderControls({
    $,
    Utils,
    requestManager,
    ConfirmationDialog,
    playToggleOn,
    playToggleOff,
    playError,
    eventCleanups,
    getBook: () => statsView?.getBook() ?? "live",
    onStatus: (status) => statsView?.setStatus(status),
  });
  statsView = createStatsView({
    requestManager,
    Utils,
    DataTable,
    ConfirmationDialog,
    controls,
    switchTab: (tabId) => void tabBar?.setActive(tabId),
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
   * Read the Stats tab's figures and its controls. Independent reads, so they go out
   * together instead of queueing behind each other. The trader status is polled here
   * because nothing else reads it while the tab is open.
   */
  async function refreshStats() {
    if (state.currentTab !== "stats") return;
    await Promise.all([statsView.refresh(), controls.refresh()]);
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
    // A newly chosen sub-tab opens at its top, whatever the previous one was scrolled to.
    if (changed && traderContent) traderContent.scrollTop = 0;

    // init() paints only. Remote loaders and embedded lifecycles start once the
    // page is active, after the selected panel is already on screen.
    if (!load) return;

    // Start/stop pollers based on tab
    if (tabId === "stats") {
      // The poller waits a full interval before its first run, so load now.
      void refreshStats();
      if (statsPoller && !statsPoller.active) {
        statsPoller.start();
      }
    } else {
      if (statsPoller?.active) {
        statsPoller.stop();
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

    // The runner warning reads the exit strategies, which change on their own tab.
    if (tabId === "take-profit") {
      void loadExitStrategiesOn();
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
   * Whether an exit strategy is switched on, for the Take Profit runner warning. Read from
   * the strategies overview, which owns the count; a failed read keeps the last answer.
   */
  async function loadExitStrategiesOn() {
    try {
      const data = await requestManager.fetch("/api/strategies/overview", { priority: "low" });
      const enabled = data?.counts?.exit?.enabled;
      state.exitStrategiesOn = typeof enabled === "number" ? enabled > 0 : null;
      examples.updateTakeProfitExample();
    } catch (error) {
      console.error("[Trader] Failed to load the strategies overview:", error);
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

      // Setup trading controls event handlers and the Stats view's own switches
      controls.setupControlsEventHandlers();
      statsView.mount();

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
          refreshStats,
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

      ctx.managePoller(statsPoller);
      ctx.managePoller(configPoller);

      // Paint/switch first; it starts only the selected tab's poller and loads.
      switchTab(state.currentTab);
      configPoller.start();

      // Independent first loads never hold the router navigation open.
      void loadConfig();
    },

    /**
     * Deactivate the page (pollers stopped automatically)
     */
    deactivate() {
      console.log("[Trader] Deactivating page");
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
      statsView.destroy();

      // Clean up all tracked event listeners
      eventCleanups.forEach((cleanup) => cleanup());
      eventCleanups.length = 0;

      // TabBar cleaned up automatically by manageTabBar
      tabBar = null;
      state.config = null;
      state.stats = null;
      statsPoller = null;
      configPoller = null;
      lifecycleContext = null;
      state.exitStrategiesOn = null;
    },
  };
}

// Register page
registerPage("trader", createLifecycle());
