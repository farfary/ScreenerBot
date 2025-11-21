import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";

// Sub-tabs configuration
const SUB_TABS = [
  { id: "stats", label: '<i class="icon-bar-chart-2"></i> Stats' },
  { id: "trailing-stop", label: '<i class="icon-trending-up"></i> Trailing Stop' },
  { id: "roi", label: '<i class="icon-target"></i> Take Profit' },
  { id: "time-rules", label: '<i class="icon-timer"></i> Time Rules' },
  { id: "strategy-control", label: '<i class="icon-puzzle"></i> Strategy Control' },
  { id: "general-settings", label: '<i class="icon-settings"></i> Settings' },
];

// Constants
const DEFAULT_TAB = "stats";

function createLifecycle() {
  // Component references
  let tabBar = null;
  let statsPoller = null;
  let configPoller = null;
  let strategiesPoller = null;

  // Page state
  const state = {
    currentTab: DEFAULT_TAB,
    config: null,
    stats: null,
    strategies: [],
  };

  // ============================================================================
  // Helper Functions
  // ============================================================================

  /**
   * Convert time duration to human-readable format
   */
  function convertTimeToReadable(duration, unit) {
    const units = {
      seconds: { seconds: 1, minutes: 60, hours: 3600, days: 86400 },
      minutes: { seconds: 1 / 60, minutes: 1, hours: 60, days: 1440 },
      hours: { seconds: 1 / 3600, minutes: 1 / 60, hours: 1, days: 24 },
      days: { seconds: 1 / 86400, minutes: 1 / 1440, hours: 1 / 24, days: 1 },
    };

    if (!units[unit]) return `${duration} ${unit}`;

    const conversions = units[unit];
    const totalSeconds = duration / conversions.seconds;

    // Find the best unit for display
    if (totalSeconds >= 86400 && totalSeconds % 86400 === 0) {
      const days = totalSeconds / 86400;
      return `${days} day${days !== 1 ? "s" : ""}`;
    }
    if (totalSeconds >= 3600 && totalSeconds % 3600 === 0) {
      const hours = totalSeconds / 3600;
      return `${hours} hour${hours !== 1 ? "s" : ""}`;
    }
    if (totalSeconds >= 60 && totalSeconds % 60 === 0) {
      const minutes = totalSeconds / 60;
      return `${minutes} minute${minutes !== 1 ? "s" : ""}`;
    }
    return `${totalSeconds} second${totalSeconds !== 1 ? "s" : ""}`;
  }

  /**
   * Update time conversion hint display
   */
  function updateTimeConversionHint() {
    const durationInput = $("#time-max-hold");
    const unitSelect = $("#time-unit");
    const hintText = $("#time-conversion-hint");
    const exampleDuration = $("#example-duration");

    if (!durationInput || !unitSelect || !hintText) return;

    const duration = parseFloat(durationInput.value) || 168;
    const unit = unitSelect.value || "hours";

    const readable = convertTimeToReadable(duration, unit);
    hintText.textContent = `${duration} ${unit} = ${readable}`;
    
    if (exampleDuration) {
      exampleDuration.textContent = readable;
    }
  }

  /**
   * Update ROI example display
   */
  function updateRoiExample() {
    const roiInput = $("#roi-target");
    const exampleSpan = $("#roi-example");
    
    if (!roiInput || !exampleSpan) return;
    
    const value = parseFloat(roiInput.value) || 20;
    exampleSpan.textContent = value.toFixed(0);
  }

  /**
   * Update time override loss example display
   */
  function updateTimeLossExample() {
    const lossInput = $("#time-loss-threshold");
    const exampleSpan = $("#example-loss");
    
    if (!lossInput || !exampleSpan) return;
    
    const value = parseFloat(lossInput.value) || -40;
    exampleSpan.textContent = Math.abs(value).toFixed(0);
  }

  /**
   * Update trailing stop visual example calculations
   */
  function updateTrailingStopExample() {
    const activationInput = $("#trailing-activation");
    const distanceInput = $("#trailing-distance");
    
    if (!activationInput || !distanceInput) return;
    
    const activation = parseFloat(activationInput.value) || 15;
    const distance = parseFloat(distanceInput.value) || 5;
    
    // Example scenario: Entry at 1.00 SOL
    const entryPrice = 1.00;
    const activationPrice = entryPrice * (1 + activation / 100);
    const peakPrice = activationPrice * 1.20; // +20% from activation
    const exitPrice = peakPrice * (1 - distance / 100);
    const protectedProfit = ((exitPrice - entryPrice) / entryPrice) * 100;
    
    // Update timeline values
    const stepEntry = $("#step-entry-value");
    const stepActivation = $("#step-activation-value");
    const stepPeak = $("#step-peak-value");
    const stepExit = $("#step-exit-value");
    
    if (stepEntry) stepEntry.textContent = `${entryPrice.toFixed(4)} SOL`;
    if (stepActivation) {
      stepActivation.textContent = `${activationPrice.toFixed(4)} SOL`;
      const activationDetail = $("#step-activation-detail");
      if (activationDetail) activationDetail.textContent = `+${activation}%`;
    }
    if (stepPeak) {
      stepPeak.textContent = `${peakPrice.toFixed(4)} SOL`;
      const peakDetail = $("#step-peak-detail");
      if (peakDetail) {
        const gainFromEntry = ((peakPrice - entryPrice) / entryPrice) * 100;
        peakDetail.textContent = `+${gainFromEntry.toFixed(1)}%`;
      }
    }
    if (stepExit) {
      stepExit.textContent = `${exitPrice.toFixed(4)} SOL`;
      const exitDetail = $("#step-exit-detail");
      if (exitDetail) exitDetail.textContent = `-${distance}% from peak`;
    }
    
    // Update summary
    const summaryProfit = $("#summary-protected-profit");
    if (summaryProfit) {
      summaryProfit.textContent = `${protectedProfit.toFixed(1)}%`;
    }
    
    // Update impact indicators
    const activationIndicator = $("#activation-indicator");
    const distanceIndicator = $("#distance-indicator");
    const activationImpact = $("#activation-impact-text");
    const distanceImpact = $("#distance-impact-text");
    
    if (activationIndicator) {
      activationIndicator.innerHTML = activation >= 20 ? '<i class="icon-alert-triangle"></i>' : '<i class="icon-check-circle"></i>';
      activationIndicator.style.background = activation >= 20 ? 'var(--warning-alpha-10)' : 'var(--success-alpha-10)';
      activationIndicator.style.color = activation >= 20 ? 'var(--warning)' : 'var(--success)';
    }
    
    if (activationImpact) {
      if (activation < 10) {
        activationImpact.textContent = 'Activates quickly - good for volatile tokens';
      } else if (activation < 20) {
        activationImpact.textContent = 'Balanced activation - suitable for most scenarios';
      } else {
        activationImpact.textContent = 'Delayed activation - may miss protection window';
      }
    }
    
    if (distanceIndicator) {
      distanceIndicator.innerHTML = distance >= 10 ? '<i class="icon-alert-triangle"></i>' : '<i class="icon-check-circle"></i>';
      distanceIndicator.style.background = distance >= 10 ? 'var(--warning-alpha-10)' : 'var(--success-alpha-10)';
      distanceIndicator.style.color = distance >= 10 ? 'var(--warning)' : 'var(--success)';
    }
    
    if (distanceImpact) {
      if (distance < 5) {
        distanceImpact.textContent = 'Tight protection - may exit on minor dips';
      } else if (distance < 10) {
        distanceImpact.textContent = 'Balanced protection - good for most situations';
      } else {
        distanceImpact.textContent = 'Loose protection - allows larger pullbacks';
      }
    }
  }

  /**
   * Load trailing stop performance stats (placeholder for Phase 2)
   */
  async function loadTrailingStopStats() {
    // This will be implemented in Phase 2 when we add trailing stop tracking
    const statsCards = $$('.quick-stat-card');
    statsCards.forEach(card => {
      const value = card.querySelector('.quick-stat-value');
      if (value) {
        value.textContent = '--';
      }
    });
  }

  /**
   * Load trailing stop preview data (placeholder for Phase 2)
   */
  async function loadTrailingStopPreview() {
    // Update example with current config values
    updateTrailingStopExample();
    
    // Load performance stats (Phase 2)
    await loadTrailingStopStats();
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
      "trailing-stop": "trailing-stop-tab",
      roi: "roi-tab",
      "time-rules": "time-rules-tab",
      "strategy-control": "strategy-control-tab",
      "general-settings": "general-settings-tab",
    };

    const contentId = tabMap[tabId];
    const content = $(`#${contentId}`);
    if (content) {
      content.style.display = "block";
    }

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
      loadStrategies();
      if (strategiesPoller && !strategiesPoller.running) {
        strategiesPoller.start();
      }
    } else {
      if (strategiesPoller && strategiesPoller.running) {
        strategiesPoller.stop();
      }
    }

    // Load preview when switching to trailing stop tab (Phase 2)
    if (tabId === "trailing-stop") {
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
  async function loadConfig() {
    try {
      const response = await fetch("/api/config");
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const data = await response.json();
      state.config = data.config;

      // Update form fields
      updateFormFields();
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
    updateTimeConversionHint();

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
    const dryRun = $("#dry-run");

    if (maxPositions) maxPositions.value = trader.max_open_positions || 2;
    if (tradeSize) tradeSize.value = trader.trade_size_sol || 0.005;
    if (entrySizes) entrySizes.value = (trader.entry_sizes || [0.005, 0.01, 0.02, 0.05]).join(", ");
    if (dcaEnabled) dcaEnabled.checked = trader.dca_enabled || false;
    if (dcaThreshold) dcaThreshold.value = trader.dca_threshold_pct || -10;
    if (dcaMaxCount) dcaMaxCount.value = trader.dca_max_count || 2;
    if (dcaSize) dcaSize.value = trader.dca_size_percentage || 50;
    if (dcaCooldown) dcaCooldown.value = trader.dca_cooldown_minutes || 30;
    if (closeCooldown) closeCooldown.value = trader.close_cooldown_seconds || 10;
    if (entryConcurrency) entryConcurrency.value = trader.entry_monitor_concurrency || 3;
    if (dryRun) dryRun.checked = trader.dry_run || false;

    // Update dry run warning
    updateDryRunWarning();
  }

  /**
   * Load statistics for Stats tab
   */
  async function loadStats() {
    try {
      const response = await fetch("/api/trader/stats");
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const data = await response.json();

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
          return sum + (exit.avg_profit_pct * exit.count);
        }, 0);
        const avgProfit = data.total_trades > 0 ? totalProfit / data.total_trades : 0;
        totalPnl.textContent = `${avgProfit >= 0 ? "+" : ""}${avgProfit.toFixed(1)}%`;
        totalPnl.className = `metric-value ${avgProfit >= 0 ? "positive" : "negative"}`;
      }
      if (totalPnlDetail) {
        totalPnlDetail.textContent = `Average profit per trade`;
      }

      // Total Trades
      if (totalTrades) {
        totalTrades.textContent = data.total_trades;
      }
      if (totalTradesDetail) {
        totalTradesDetail.textContent = data.total_trades === 1 ? "1 position closed" : `${data.total_trades} positions closed`;
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
        const worstPct = data.worst_trade_pct !== undefined ? data.worst_trade_pct : 0;
        worstTrade.textContent = `${worstPct > 0 ? "+" : ""}${worstPct.toFixed(1)}%`;
        worstTrade.className = `metric-value ${worstPct < 0 ? "negative" : ""}`;
      }
      if (worstTradeDetail) {
        worstTradeDetail.textContent = data.worst_trade_token || "No trades yet";
      }

      // Update exit breakdown with visual bars
      const exitBreakdown = $("#exit-breakdown");
      if (exitBreakdown && data.exit_breakdown && data.exit_breakdown.length > 0) {
        const maxCount = Math.max(...data.exit_breakdown.map(e => e.count));
        
        const barsHtml = data.exit_breakdown.map((exit) => {
          const percentage = maxCount > 0 ? (exit.count / maxCount) * 100 : 0;
          const barClass = exit.avg_profit_pct >= 0 ? "success" : "danger";
          const avgClass = exit.avg_profit_pct >= 0 ? "positive" : "negative";
          
          return `
            <div class="exit-bar-item">
              <div class="exit-bar-header">
                <div class="exit-bar-label">
                  <span class="exit-icon">
                    <i class="icon-${getExitIcon(exit.exit_type)}"></i>
                  </span>
                  ${Utils.escapeHtml(exit.exit_type)}
                </div>
                <div class="exit-bar-stats">
                  <span class="exit-bar-count">${exit.count} trade${exit.count !== 1 ? "s" : ""}</span>
                  <span class="exit-bar-avg ${avgClass}">
                    ${exit.avg_profit_pct >= 0 ? "+" : ""}${exit.avg_profit_pct.toFixed(1)}% avg
                  </span>
                </div>
              </div>
              <div class="exit-bar-wrapper">
                <div class="exit-bar ${barClass}" style="width: ${percentage}%"></div>
              </div>
            </div>
          `;
        }).join("");
        
        exitBreakdown.innerHTML = `<div class="exit-bars">${barsHtml}</div>`;
      } else if (exitBreakdown) {
        exitBreakdown.innerHTML = `
          <div class="empty-state">
            <i class="icon-inbox"></i>
            <span>No closed positions yet</span>
          </div>
        `;
      }

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

  /**
   * Get icon name for exit type
   */
  function getExitIcon(exitType) {
    const iconMap = {
      "ROI Target": "target",
      "Trailing Stop": "trending-down",
      "Time Override": "clock",
      "Manual": "hand",
      "Stop Loss": "alert-triangle",
      "Strategy": "zap",
    };
    return iconMap[exitType] || "circle";
  }

  /**
   * Update positions summary section
   */
  async function updatePositionsSummary() {
    const positionsSummary = $("#positions-summary");
    if (!positionsSummary) return;

    try {
      const response = await fetch("/api/positions");
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const data = await response.json();

      if (!data.positions || data.positions.length === 0) {
        positionsSummary.innerHTML = `
          <div class="info-state">
            <i class="icon-inbox"></i>
            <span>No open positions</span>
          </div>
        `;
        return;
      }

      const cardsHtml = data.positions.map((pos) => {
        const roi = pos.roi_percent || 0;
        const roiClass = roi >= 0 ? "positive" : "negative";
        const holdTime = pos.opened_at_timestamp 
          ? Utils.formatDuration((Date.now() - new Date(pos.opened_at_timestamp).getTime()) / 1000)
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
      }).join("");

      positionsSummary.innerHTML = `<div class="positions-grid">${cardsHtml}</div>`;
    } catch (error) {
      console.error("[Trader] Failed to load positions summary:", error);
      positionsSummary.innerHTML = `
        <div class="info-state">
          <i class="icon-alert-circle"></i>
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

      const response = await fetch(`/api/trader/preview-trailing-stop?${params}`);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const data = await response.json();

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
  async function loadStrategies() {
    try {
      const response = await fetch("/api/strategies");
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const data = await response.json();

      state.strategies = data.strategies || [];

      // Separate entry and exit strategies
      const entryStrategies = state.strategies.filter((s) => s.strategy_type === "entry");
      const exitStrategies = state.strategies.filter((s) => s.strategy_type === "exit");

      // Render lists
      renderStrategiesList("#entry-strategies", entryStrategies);
      renderStrategiesList("#exit-strategies", exitStrategies);
    } catch (error) {
      console.error("[Trader] Failed to load strategies:", error);
    }
  }

  /**
   * Render strategies list
   */
  function renderStrategiesList(selector, strategies) {
    const container = $(selector);
    if (!container) return;

    if (strategies.length === 0) {
      container.innerHTML = '<div class="empty-state">No strategies defined</div>';
      return;
    }

    container.innerHTML = strategies
      .map(
        (strategy) => `
        <div class="strategy-item">
          <div class="strategy-header">
            <div class="strategy-name">${Utils.escapeHtml(strategy.name)}</div>
            <label class="switch">
              <input 
                type="checkbox" 
                data-strategy-id="${strategy.id}" 
                ${strategy.enabled ? "checked" : ""}
              />
              <span class="slider"></span>
            </label>
          </div>
          ${
            strategy.description
              ? `<div class="strategy-description">${Utils.escapeHtml(strategy.description)}</div>`
              : ""
          }
          <div class="strategy-meta">
            <span class="strategy-badge">${Utils.escapeHtml(strategy.strategy_type)}</span>
            ${
              strategy.priority !== null && strategy.priority !== undefined
                ? `<span class="strategy-priority">Priority: ${strategy.priority}</span>`
                : ""
            }
          </div>
        </div>
      `
      )
      .join("");

    // Attach event listeners for toggle switches
    container.querySelectorAll('input[type="checkbox"]').forEach((checkbox) => {
      checkbox.addEventListener("change", async (e) => {
        const strategyId = parseInt(e.target.dataset.strategyId, 10);
        const enabled = e.target.checked;
        await updateStrategyStatus(strategyId, enabled);
      });
    });
  }

  /**
   * Update strategy enabled/disabled status
   */
  async function updateStrategyStatus(strategyId, enabled) {
    try {
      const response = await fetch(`/api/strategies/${strategyId}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled }),
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      Utils.showToast({
        type: "success",
        title: enabled ? "Strategy Enabled" : "Strategy Disabled",
        message: enabled ? "Now monitoring for entry signals" : "Entry monitoring stopped",
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
      const response = await fetch("/api/positions");
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const data = await response.json();

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
   */
  function setupFormHandlers() {
    // Trailing Stop form
    const saveTrailing = $("#save-trailing");
    if (saveTrailing) {
      saveTrailing.addEventListener("click", async (e) => {
        e.preventDefault();
        const enabled = $("#trailing-enabled")?.checked || false;
        const activation = parseFloat($("#trail-activation")?.value || "10.0");
        const distance = parseFloat($("#trail-distance")?.value || "5.0");
        await saveConfig({
          positions: {
            trailing_stop_enabled: enabled,
            trailing_stop_activation_pct: activation,
            trailing_stop_distance_pct: distance,
          },
        });
      });
    }

    // Trailing Stop reset button
    const resetTrailing = $("#reset-trailing");
    if (resetTrailing) {
      resetTrailing.addEventListener("click", async (e) => {
        e.preventDefault();
        const { confirmed } = await ConfirmationDialog.show({
          title: "Reset Trailing Stop",
          message:
            "This will reset trailing stop settings to default values:\n• Disabled\n• Activation: 10%\n• Distance: 5%",
          confirmLabel: "Reset",
          cancelLabel: "Cancel",
          variant: "warning",
        });

        if (confirmed) {
          await saveConfig({
            positions: {
              trailing_stop_enabled: false,
              trailing_stop_activation_pct: 10.0,
              trailing_stop_distance_pct: 5.0,
            },
          });
        }
      });
    }

    // ROI save button
    const saveRoi = $("#save-roi");
    if (saveRoi) {
      saveRoi.addEventListener("click", async (e) => {
        e.preventDefault();
        const enabled = $("#roi-enabled")?.checked || false;
        const target = parseFloat($("#roi-target")?.value || "20");
        await saveConfig({
          trader: {
            roi_exit_enabled: enabled,
            roi_target_percent: target,
          },
        });
      });
    }

    // ROI reset button
    const resetRoi = $("#reset-roi");
    if (resetRoi) {
      resetRoi.addEventListener("click", async (e) => {
        e.preventDefault();
        const { confirmed } = await ConfirmationDialog.show({
          title: "Reset ROI Exit",
          message:
            "This will reset ROI exit settings to default values:\n• Enabled\n• Target: 20%",
          confirmLabel: "Reset",
          cancelLabel: "Cancel",
          variant: "warning",
        });

        if (confirmed) {
          await saveConfig({
            trader: {
              roi_exit_enabled: true,
              roi_target_percent: 20,
            },
          });
        }
      });
    }

    // Time Rules save button
    const saveTimeRules = $("#save-time-rules");
    if (saveTimeRules) {
      saveTimeRules.addEventListener("click", async (e) => {
        e.preventDefault();
        const enabled = $("#time-override-enabled")?.checked || false;
        const duration = parseFloat($("#time-max-hold")?.value || "168");
        const unit = $("#time-unit")?.value || "hours";
        const lossThreshold = parseFloat($("#time-loss-threshold")?.value || "-40");
        await saveConfig({
          trader: {
            time_override_enabled: enabled,
            time_override_duration: duration,
            time_override_unit: unit,
            time_override_loss_threshold_percent: lossThreshold,
          },
        });
      });
    }

    // Time Rules reset button
    const resetTimeRules = $("#reset-time-rules");
    if (resetTimeRules) {
      resetTimeRules.addEventListener("click", async (e) => {
        e.preventDefault();
        const { confirmed } = await ConfirmationDialog.show({
          title: "Reset Time Override",
          message:
            "This will reset time override settings to default values:\n• Enabled\n• Duration: 168 hours (7 days)\n• Loss Threshold: -40%",
          confirmLabel: "Reset",
          cancelLabel: "Cancel",
          variant: "warning",
        });

        if (confirmed) {
          await saveConfig({
            trader: {
              time_override_enabled: true,
              time_override_duration: 168,
              time_override_unit: "hours",
              time_override_loss_threshold_percent: -40,
            },
          });
        }
      });
    }

    // General Settings form
    const generalForm = $("#general-settings-form");
    if (generalForm) {
      generalForm.addEventListener("submit", async (e) => {
        e.preventDefault();

        const maxPositions = parseInt($("#max-positions")?.value || "2", 10);
        const tradeSize = parseFloat($("#trade-size")?.value || "0.005");
        const entrySizesRaw = $("#entry-sizes")?.value || "0.005, 0.01, 0.02, 0.05";
        const entrySizes = entrySizesRaw
          .split(",")
          .map((s) => parseFloat(s.trim()))
          .filter((n) => !isNaN(n));
        const dcaEnabled = $("#dca-enabled")?.checked || false;
        const dcaThreshold = parseFloat($("#dca-threshold")?.value || "-10");
        const dcaMaxCount = parseInt($("#dca-max-count")?.value || "2", 10);
        const dcaSize = parseFloat($("#dca-size")?.value || "50");
        const dcaCooldown = parseInt($("#dca-cooldown")?.value || "5", 10);
        const closeCooldown = parseInt($("#close-cooldown")?.value || "10", 10);
        const entryConcurrency = parseInt($("#entry-concurrency")?.value || "3", 10);
        const dryRun = $("#dry-run")?.checked || false;

        await saveConfig({
          trader: {
            max_open_positions: maxPositions,
            trade_size_sol: tradeSize,
            entry_sizes: entrySizes,
            dca_enabled: dcaEnabled,
            dca_threshold_pct: dcaThreshold,
            dca_max_count: dcaMaxCount,
            dca_size_percentage: dcaSize,
            dca_cooldown_minutes: dcaCooldown,
            close_cooldown_seconds: closeCooldown,
            entry_monitor_concurrency: entryConcurrency,
            dry_run: dryRun,
          },
        });
      });
    }

    // Dry run checkbox immediate update
    const dryRunCheckbox = $("#dry-run");
    if (dryRunCheckbox) {
      dryRunCheckbox.addEventListener("change", () => {
        updateDryRunWarning();
      });
    }

    // Export config button
    const exportBtn = $("#export-config-btn");
    if (exportBtn) {
      exportBtn.addEventListener("click", () => {
        exportConfig();
      });
    }

    // Import config button
    const importBtn = $("#import-config-btn");
    if (importBtn) {
      importBtn.addEventListener("click", () => {
        importConfig();
      });
    }

    // Time unit change listener
    const timeUnit = $("#time-unit");
    if (timeUnit) {
      timeUnit.addEventListener("change", () => {
        updateTimeConversionHint();
      });
    }

    // Time duration input listener
    const timeMaxHold = $("#time-max-hold");
    if (timeMaxHold) {
      timeMaxHold.addEventListener("input", () => {
        updateTimeConversionHint();
      });
    }

    // ROI target input listener
    const roiTarget = $("#roi-target");
    if (roiTarget) {
      roiTarget.addEventListener("input", () => {
        updateRoiExample();
      });
    }

    // Time loss threshold input listener
    const timeLossThreshold = $("#time-loss-threshold");
    if (timeLossThreshold) {
      timeLossThreshold.addEventListener("input", () => {
        updateTimeLossExample();
      });
    }
  }

  /**
   * Setup preview event listeners (Phase 2)
   */
  function setupPreviewListeners() {
    // Debounced preview update on config change
    const debouncedTrailingPreview = Utils.debounce(() => {
      updateTrailingStopExample();
    }, 300);

    // Trailing activation input
    const activationInput = $("#trailing-activation");
    if (activationInput) {
      activationInput.addEventListener("input", debouncedTrailingPreview);
    }

    // Trailing distance input
    const distanceInput = $("#trailing-distance");
    if (distanceInput) {
      distanceInput.addEventListener("input", debouncedTrailingPreview);
    }
  }

  /**
   * Save configuration updates
   */
  async function saveConfig(updates) {
    try {
      const response = await fetch("/api/config", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(updates),
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      Utils.showToast({
        type: "success",
        title: "Configuration Saved",
        message: "Trader settings updated successfully",
      });
      await loadConfig(); // Reload to reflect changes
    } catch (error) {
      console.error("[Trader] Failed to save config:", error);
      Utils.showToast({
        type: "error",
        title: "Save Failed",
        message: "Failed to save trader configuration",
      });
    }
  }

  /**
   * Export configuration to JSON file
   */
  function exportConfig() {
    if (!state.config) {
      Utils.showToast({
        type: "error",
        title: "Export Failed",
        message: "No configuration loaded",
      });
      return;
    }

    const dataStr = JSON.stringify(state.config.trader, null, 2);
    const blob = new Blob([dataStr], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `trader-config-${Date.now()}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);

    Utils.showToast({
      type: "success",
      title: "Configuration Exported",
      message: "Trader settings saved to file",
    });
  }

  /**
   * Import configuration from JSON file
   */
  function importConfig() {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "application/json";
    input.onchange = async (e) => {
      const file = e.target.files?.[0];
      if (!file) return;

      try {
        const text = await file.text();
        const imported = JSON.parse(text);

        await saveConfig({ trader: imported });
        Utils.showToast({
          type: "success",
          title: "Configuration Imported",
          message: "Trader settings loaded from file",
        });
      } catch (error) {
        console.error("[Trader] Failed to import config:", error);
        Utils.showToast({
          type: "error",
          title: "Import Failed",
          message: "Failed to import configuration - invalid file format",
        });
      }
    };
    input.click();
  }

  /**
   * Update dry run warning visibility
   */
  function updateDryRunWarning() {
    const dryRunCheckbox = $("#dry-run");
    const warning = $("#dry-run-warning");
    if (!warning) return;

    const isDryRun = dryRunCheckbox?.checked || false;
    warning.style.display = isDryRun ? "block" : "none";
  }

  /**
   * Setup navigation links to other pages
   */
  function setupNavigation() {
    // Link to positions page
    $$(".nav-to-positions").forEach((link) => {
      link.addEventListener("click", (e) => {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent("navigate", { detail: { page: "positions" } }));
      });
    });

    // Link to strategies page
    $$(".nav-to-strategies").forEach((link) => {
      link.addEventListener("click", (e) => {
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

      // Initialize tab bar
      tabBar = new TabBar({
        container: "#subTabsContainer",
        tabs: SUB_TABS,
        defaultTab: DEFAULT_TAB,
        stateKey: "trader.activeTab",
        pageName: "trader",
        onChange: (tabId) => {
          switchTab(tabId);
        },
      });

      // Register with TabBarManager for page-switch coordination
      TabBarManager.register("trader", tabBar);

      // Integrate with lifecycle for auto-cleanup
      ctx.manageTabBar(tabBar);

      // Show the tab bar
      tabBar.show();

      // Sync state with tab bar's restored state (from localStorage or URL hash)
      const activeTab = tabBar.getActiveTab();
      if (activeTab && activeTab !== state.currentTab) {
        state.currentTab = activeTab;
      }

      // Setup form handlers
      setupFormHandlers();

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

      // Create pollers
      statsPoller = ctx.managePoller(
        new Poller(
          async () => {
            if (state.currentTab === "stats") {
              await loadStats();
            }
          },
          { label: "Trader Stats", intervalMs: 5000 }
        )
      );

      configPoller = ctx.managePoller(
        new Poller(
          async () => {
            await loadConfig();
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

      // Start pollers
      if (state.currentTab === "stats") {
        statsPoller.start();
      }
      configPoller.start();

      // Initial loads
      await loadConfig();
      if (state.currentTab === "stats") {
        await loadStats();
      } else if (state.currentTab === "strategy-control") {
        await loadStrategies();
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
      // Pollers stopped automatically by lifecycle context
    },

    /**
     * Dispose the page (cleanup)
     */
    dispose() {
      console.log("[Trader] Disposing page");
      // TabBar cleaned up automatically by manageTabBar
      tabBar = null;
      state.config = null;
      state.stats = null;
      state.strategies = [];
    },
  };
}

// Register page
registerPage("trader", createLifecycle());
