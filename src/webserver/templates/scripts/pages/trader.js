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

  // Hash guard — skip positions summary re-render when data is unchanged
  let _lastPositionsKey = null;

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
  function switchTab(tabId) {
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
    if (tabId === "strategies") {
      traderContent?.classList.add("trader-content--fullbleed");
      activateStrategiesSubtab();
    } else {
      traderContent?.classList.remove("trader-content--fullbleed");
      deactivateStrategiesSubtab();
    }
    traderContent?.classList.toggle("trader-content--split-scroll", tabId === "stats");

    // Start/stop pollers based on tab
    if (tabId === "stats") {
      if (statsPoller && !statsPoller.running) {
        statsPoller.start();
      }
    } else {
      if (statsPoller && statsPoller.running) {
        statsPoller.stop();
      }
    }

    if (tabId === "strategy-control") {
      loadStrategies({ showLoading: true });
      if (strategiesPoller && !strategiesPoller.running) {
        strategiesPoller.start();
      }
    } else {
      if (strategiesPoller && strategiesPoller.running) {
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

      // Update config overview in stats tab
      updateConfigOverview();

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
   * Update config overview section in stats tab
   */
  function updateConfigOverview() {
    if (!state.config) return;

    const trader = state.config.trader || {};
    const positions = state.config.positions || {};

    // Exit Strategies
    updateConfigItem(
      "stop-loss-status",
      trader.stop_loss_enabled,
      `${trader.stop_loss_threshold_pct || 50}%`
    );
    updateConfigItem("roi-status", trader.roi_exit_enabled, `${trader.roi_target_percent || 20}%`);
    updateConfigItem(
      "trailing-status",
      positions.trailing_stop_enabled,
      `${positions.trailing_stop_activation_pct || 10}%→${positions.trailing_stop_distance_pct || 5}%`
    );
    updateConfigItem(
      "time-status",
      trader.time_override_enabled,
      `${trader.time_override_duration || 168}${trader.time_override_unit?.[0] || "h"} @ ${trader.time_override_loss_threshold_percent || -40}%`
    );

    // Position Management
    const maxPositionsEl = $("#config-max-positions");
    if (maxPositionsEl) maxPositionsEl.textContent = trader.max_open_positions || 2;

    const tradeSizeEl = $("#config-trade-size");
    if (tradeSizeEl) tradeSizeEl.textContent = `${trader.trade_size_sol || 0.005} SOL`;

    updateConfigItem(
      "dca-status",
      trader.dca_enabled,
      `${trader.dca_threshold_pct || -10}% (${trader.dca_max_count || 2}x, ${trader.dca_size_percentage || 50}%)`
    );

    // Risk Controls
    const closeCooldownEl = $("#config-close-cooldown");
    if (closeCooldownEl) {
      const seconds = Number.isFinite(trader.close_cooldown_seconds)
        ? trader.close_cooldown_seconds
        : 600;
      const minutes = seconds / 60;
      closeCooldownEl.textContent = minutes < 1 ? "<1m" : `${Math.round(minutes)}m`;
    }

    const entryConcurrencyEl = $("#config-entry-concurrency");
    if (entryConcurrencyEl) entryConcurrencyEl.textContent = trader.entry_monitor_concurrency || 10;
  }

  /**
   * Update individual config item with enable/disable status
   */
  function updateConfigItem(id, enabled, value) {
    const el = $(`#${id}`);
    if (!el) return;

    const icon = enabled
      ? '<i class="icon-circle-check status-icon enabled"></i>'
      : '<i class="icon-circle status-icon disabled"></i>';
    const displayValue = enabled ? value : "Disabled";
    const labelEl = el.querySelector(".label");
    const valueEl = el.querySelector(".value");

    if (labelEl && valueEl) {
      const iconEl = el.querySelector("i");
      if (iconEl) {
        iconEl.outerHTML = icon;
      }
      valueEl.textContent = displayValue;
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
   * Load statistics for Stats tab
   */
  async function loadStats() {
    try {
      const data = await requestManager.fetch("/api/trader/stats", {
        priority: "normal",
      });

      // Update stats period
      const statsPeriod = $("#stats-period");
      if (statsPeriod) {
        statsPeriod.textContent = "Last 30 days";
      }

      // Update performance metrics
      const winRate = $("#win-rate");
      const winRateDetail = $("#win-rate-detail");
      const totalPnl = $("#total-pnl");
      const totalPnlDetail = $("#total-pnl-detail");
      const totalTrades = $("#total-trades");
      const totalTradesDetail = $("#total-trades-detail");
      const avgHoldTime = $("#avg-hold-time");
      const avgHoldTimeDetail = $("#avg-hold-time-detail");
      const bestTrade = $("#best-trade");
      const bestTradeDetail = $("#best-trade-detail");
      const worstTrade = $("#worst-trade");
      const worstTradeDetail = $("#worst-trade-detail");

      // Win Rate
      if (winRate) {
        const rate = data.win_rate_pct.toFixed(1);
        winRate.textContent = `${rate}%`;
        winRate.className = `metric-value ${data.win_rate_pct >= 50 ? "positive" : ""}`;
      }
      if (winRateDetail) {
        const wins = Math.round((data.total_trades * data.win_rate_pct) / 100);
        const losses = data.total_trades - wins;
        winRateDetail.textContent = `${wins} wins, ${losses} losses`;
      }

      // Total P&L (calculated from exit breakdown)
      if (totalPnl && data.exit_breakdown) {
        const totalProfit = data.exit_breakdown.reduce((sum, exit) => {
          return sum + exit.avg_profit_pct * exit.count;
        }, 0);
        const avgProfit = data.total_trades > 0 ? totalProfit / data.total_trades : 0;
        totalPnl.textContent = `${avgProfit >= 0 ? "+" : ""}${avgProfit.toFixed(1)}%`;
        totalPnl.className = `metric-value ${avgProfit >= 0 ? "positive" : "negative"}`;
      }
      if (totalPnlDetail) {
        totalPnlDetail.textContent = "Average profit per trade";
      }

      // Total Trades
      if (totalTrades) {
        totalTrades.textContent = data.total_trades;
      }
      if (totalTradesDetail) {
        totalTradesDetail.textContent =
          data.total_trades === 1 ? "1 position closed" : `${data.total_trades} positions closed`;
      }

      // Avg Hold Time
      if (avgHoldTime) {
        const seconds = data.avg_hold_time_hours * 3600;
        avgHoldTime.textContent = Utils.formatUptime(seconds, { style: "short" });
      }
      if (avgHoldTimeDetail) {
        const seconds = data.avg_hold_time_hours * 3600;
        avgHoldTimeDetail.textContent = Utils.formatUptime(seconds, { style: "detailed" });
      }

      // Best Trade
      if (bestTrade) {
        const pct = data.best_trade_pct;
        bestTrade.textContent = `${pct > 0 ? "+" : ""}${pct.toFixed(1)}%`;
        bestTrade.className = `metric-value ${pct >= 0 ? "positive" : ""}`;
      }
      if (bestTradeDetail) {
        bestTradeDetail.textContent = data.best_trade_token || "No trades yet";
      }

      // Worst Trade (calculate from exit breakdown or set placeholder)
      if (worstTrade) {
        const worstPct = data.worst_trade_pct ?? 0;
        worstTrade.textContent = `${worstPct > 0 ? "+" : ""}${worstPct.toFixed(1)}%`;
        worstTrade.className = `metric-value ${worstPct < 0 ? "negative" : ""}`;
      }
      if (worstTradeDetail) {
        worstTradeDetail.textContent = data.worst_trade_token || "No trades yet";
      }

      // Render the exit strategy breakdown (was previously never rendered)
      renderExitBreakdown(data.exit_breakdown);

      // Update positions summary (if we have active positions)
      await updatePositionsSummary();
    } catch (error) {
      console.error("[Trader] Failed to load stats:", error);
      // Show error state in UI
      const winRate = $("#win-rate");
      const totalTrades = $("#total-trades");
      const avgHoldTime = $("#avg-hold-time");
      if (winRate) winRate.textContent = "—";
      if (totalTrades) totalTrades.textContent = "—";
      if (avgHoldTime) avgHoldTime.textContent = "—";
    }
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
      EXIT_TYPE_LABELS[type] ||
      type.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())
    );
  }

  /**
   * Render the exit strategy breakdown list (how positions were closed).
   */
  function renderExitBreakdown(breakdown) {
    const container = $("#exit-breakdown");
    if (!container) return;

    if (!Array.isArray(breakdown) || breakdown.length === 0) {
      container.innerHTML =
        '<div class="info-state"><i class="icon-inbox"></i><span>No closed trades in the last 30 days</span></div>';
      return;
    }

    const totalCount = breakdown.reduce((sum, e) => sum + (e.count || 0), 0) || 1;

    const rows = breakdown
      .map((e) => {
        const count = e.count || 0;
        const pct = e.avg_profit_pct || 0;
        const share = Math.round((count / totalCount) * 100);
        const profitClass = pct >= 0 ? "positive" : "negative";
        const profitText = `${pct >= 0 ? "+" : ""}${pct.toFixed(1)}%`;
        return `
          <div class="exit-breakdown-row">
            <div class="exit-breakdown-head">
              <span class="exit-breakdown-type">${Utils.escapeHtml(formatExitType(e.exit_type))}</span>
              <span class="exit-breakdown-count">${count} ${count === 1 ? "trade" : "trades"}</span>
            </div>
            <div class="exit-breakdown-bar">
              <div class="exit-breakdown-fill ${profitClass}" style="width: ${share}%"></div>
            </div>
            <div class="exit-breakdown-meta">
              <span class="exit-breakdown-share">${share}% of exits</span>
              <span class="exit-breakdown-profit ${profitClass}">${profitText} avg</span>
            </div>
          </div>`;
      })
      .join("");

    container.innerHTML = rows;
  }

  /**
   * Update positions summary section
   */
  async function updatePositionsSummary() {
    const positionsSummary = $("#positions-summary");
    if (!positionsSummary) return;

    try {
      const data = await requestManager.fetch("/api/positions", {
        priority: "normal",
      });

      const key = JSON.stringify(data.positions?.map(p => ({ id: p.id, roi: p.roi_percent, size: p.size_sol })) ?? null);
      if (key === _lastPositionsKey) return;
      _lastPositionsKey = key;

      if (!data.positions || data.positions.length === 0) {
        positionsSummary.innerHTML = `
          <div class="info-state">
            <i class="icon-inbox"></i>
            <span>No open positions</span>
          </div>
        `;
        return;
      }

      const cardsHtml = data.positions
        .map((pos) => {
          const roi = pos.roi_percent || 0;
          const roiClass = roi >= 0 ? "positive" : "negative";
          const holdTime = pos.opened_at_timestamp
            ? Utils.formatDuration(
                (Date.now() - new Date(pos.opened_at_timestamp).getTime()) / 1000
              )
            : "—";

          return `
          <div class="position-summary-card">
            <div class="position-summary-header">
              <div class="position-summary-token">${Utils.escapeHtml(pos.token_symbol || "Unknown")}</div>
              <div class="position-summary-roi ${roiClass}">${roi >= 0 ? "+" : ""}${roi.toFixed(2)}%</div>
            </div>
            <div class="position-summary-details">
              <div class="position-summary-row">
                <span class="position-summary-label">Size:</span>
                <span class="position-summary-value">${(pos.size_sol || 0).toFixed(4)} SOL</span>
              </div>
              <div class="position-summary-row">
                <span class="position-summary-label">Hold Time:</span>
                <span class="position-summary-value">${holdTime}</span>
              </div>
              <div class="position-summary-row">
                <span class="position-summary-label">Entry:</span>
                <span class="position-summary-value">${Utils.formatPrice(pos.average_entry_price || 0)}</span>
              </div>
            </div>
          </div>
        `;
        })
        .join("");

      positionsSummary.innerHTML = `<div class="positions-grid">${cardsHtml}</div>`;
    } catch (error) {
      console.error("[Trader] Failed to load positions summary:", error);
      positionsSummary.innerHTML = `
        <div class="info-state">
          <i class="icon-circle-alert"></i>
          <span>Failed to load positions</span>
        </div>
      `;
    }
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

      if (state.config) {
        updateConfigOverview();
      }

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
      .map(
        (strategy) => {
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
              <span class="toggle-label">${statusLabel}</span>
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
        }
      )
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
      const data = await requestManager.fetch("/api/positions", {
        priority: "normal",
      });

      const statusList = $("#time-positions-status");
      if (!statusList) return;

      if (!data.positions || data.positions.length === 0) {
        statusList.innerHTML = '<div class="empty-state">No open positions</div>';
        return;
      }

      statusList.innerHTML = data.positions
        .map((position) => {
          const openedDate = position.opened_at_timestamp
            ? new Date(position.opened_at_timestamp)
            : null;
          const holdSeconds = openedDate ? (Date.now() - openedDate.getTime()) / 1000 : 0;
          const holdTime = Utils.formatDuration(holdSeconds);
          const roi = position.roi_percent || 0;

          return `
            <div class="time-rule-item">
              <div class="time-rule-token">
                ${Utils.escapeHtml(position.token_symbol || "Unknown")}
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

    // Config overview "View Details" button
    const expandConfigBtn = $("#expand-config");
    if (expandConfigBtn) {
      addTrackedListener(expandConfigBtn, "click", () => {
        if (tabBar) {
          tabBar.switchTo("general-settings");
        }
      });
    }
  }

  /**
   * Update relative time display for last check
   * NOTE: Removed - config-last-check element no longer exists after System Status column removal
   */
  function updateLastCheckTime() {
    // Deprecated: System Status column removed from Stats tab
    return;
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
        updateConfigOverview();
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

  /**
   * Setup navigation links to other pages
   */
  function setupNavigation() {
    // Link to positions page
    $$(".nav-to-positions").forEach((link) => {
      addTrackedListener(link, "click", (e) => {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent("navigate", { detail: { page: "positions" } }));
      });
    });

    // Link to strategies page
    $$(".nav-to-strategies").forEach((link) => {
      addTrackedListener(link, "click", (e) => {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent("navigate", { detail: { page: "strategies" } }));
      });
    });
  }

  // ============================================================================
  // Lifecycle Methods
  // ============================================================================

  return {
    /**
     * Initialize the page
     */
    async init(ctx) {
      console.log("[Trader] Initializing page");

      // Fetch feature status early (non-blocking, but before tab bar setup)
      const featurePromise = fetchFeatureStatus(requestManager);

      // Per-card Save/Reset controls (injected into each config card header).
      // saveConfig POSTs + hot-reloads + reloads the form, after which
      // loadConfig() calls configCards.snapshot() so the buttons re-hide.
      configCards = createTraderConfigCards({ saveConfig });
      configCards.setup();

      // Wait for features before setting up tabs (important for initial tab selection)
      tradingFeatures = await featurePromise;

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

      // Apply feature status badges/styling to tabs
      applyFeatureStatusToTabs(tradingFeatures, $$);

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
      switchTab(state.currentTab);

      // Setup form handlers
      setupFormHandlers();

      // Setup trading controls event handlers
      controls.setupControlsEventHandlers();

      // Setup preview listeners (Phase 2)
      setupPreviewListeners();

      // Setup navigation links
      setupNavigation();

    },

    /**
     * Activate the page (start pollers)
     */
    async activate(ctx) {
      console.log("[Trader] Activating page");

      // Re-register deactivate cleanup (cleanups are cleared after each deactivate)
      // and force-show tab bar to handle race conditions with TabBarManager
      if (tabBar) {
        ctx.manageTabBar(tabBar);
        tabBar.show({ force: true });
      }

      // Create pollers
      statsPoller = ctx.managePoller(
        new Poller(
          async () => {
            if (state.currentTab === "stats") {
              await loadStats();
              await controls.loadControlsStatus();
            }
          },
          { label: "Trader Stats", intervalMs: 5000 }
        )
      );

      configPoller = ctx.managePoller(
        new Poller(
          async () => {
            await loadConfig({ preserveUnsavedEdits: true });
          },
          { label: "Trader Config", intervalMs: 10000 }
        )
      );

      strategiesPoller = ctx.managePoller(
        new Poller(
          async () => {
            if (state.currentTab === "strategy-control") {
              await loadStrategies();
            }
          },
          { label: "Strategies", intervalMs: 10000 }
        )
      );

      // Poller for updating relative timestamps
      const timestampPoller = ctx.managePoller(
        new Poller(
          () => {
            updateLastCheckTime();
          },
          { label: "Timestamp Updates", intervalMs: 1000 }
        )
      );

      // Start pollers
      if (state.currentTab === "stats") {
        statsPoller.start();
      }
      configPoller.start();
      timestampPoller.start();

      // Initial loads
      await Promise.all([loadConfig(), loadStrategies()]);
      if (state.currentTab === "stats") {
        await loadStats();
        await controls.loadControlsStatus();
      }
      if (state.currentTab === "strategy-control" && strategiesPoller) {
        strategiesPoller.start();
      }

      // Show initial tab
      switchTab(state.currentTab);
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
      state.strategies = [];
      _lastPositionsKey = null;
    },
  };
}

// Register page
registerPage("trader", createLifecycle());
