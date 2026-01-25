import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { requestManager } from "../core/request_manager.js";
import { ActionBar, ActionBarManager } from "../ui/action_bar.js";
import { playToggleOn, playToggleOff, playError } from "../core/sounds.js";

// Sub-tabs configuration
const SUB_TABS = [
  { id: "stats", label: '<i class="icon-chart-bar"></i> Stats' },
  { id: "stop-loss", label: '<i class="icon-shield-off"></i> Stop Loss' },
  { id: "trailing-stop", label: '<i class="icon-trending-up"></i> Trailing Stop' },
  { id: "roi", label: '<i class="icon-target"></i> Take Profit' },
  { id: "time-rules", label: '<i class="icon-timer"></i> Time Rules' },
  { id: "dca", label: '<i class="icon-dollar-sign"></i> DCA' },
  { id: "strategy-control", label: '<i class="icon-puzzle"></i> Strategy Control' },
  { id: "general-settings", label: '<i class="icon-settings"></i> Settings' },
];

// Constants
const DEFAULT_TAB = "stats";

// =============================================================================
// Feature Status Constants
// =============================================================================

/**
 * Feature status values from the API
 */
const FEATURE_STATUS = {
  AVAILABLE: "available",
  COMING_SOON: "coming_soon",
  BETA: "beta",
  DISABLED: "disabled",
};

/**
 * Maps tab IDs to feature API keys (trading features)
 */
const TAB_TO_FEATURE_MAP = {
  roi: "roi_exit",
  "trailing-stop": "trailing_stop",
  "stop-loss": "stop_loss",
  "time-rules": "time_override",
  dca: "dca",
  "strategy-control": "strategies",
  // These tabs don't have feature flags - always available
  stats: null,
  "general-settings": null,
};

/**
 * Status display configuration for badges
 */
const STATUS_CONFIG = {
  [FEATURE_STATUS.COMING_SOON]: {
    label: "Coming Soon",
    cssClass: "coming-soon",
    tooltip: "This feature is coming soon",
    message: "This feature is coming soon and not yet available.",
  },
  [FEATURE_STATUS.BETA]: {
    label: "Beta",
    cssClass: "beta",
    tooltip: "Beta feature - may have bugs",
    message: null, // Beta features are usable
  },
  [FEATURE_STATUS.DISABLED]: {
    label: "Disabled",
    cssClass: "disabled",
    tooltip: "This feature is currently disabled",
    message: "This feature is currently disabled.",
  },
};

function createLifecycle() {
  // Component references
  let tabBar = null;
  let actionBar = null;
  let statsPoller = null;
  let configPoller = null;
  let strategiesPoller = null;

  // Event cleanup tracking
  const eventCleanups = [];

  // Feature status from API
  let tradingFeatures = {};

  // Page state
  const state = {
    currentTab: DEFAULT_TAB,
    config: null,
    stats: null,
    strategies: [],
  };

  // ============================================================================
  // Feature Status Functions
  // ============================================================================

  /**
   * Fetch trading feature status from the API
   * @returns {Promise<Object>} Feature status by feature key
   */
  async function fetchFeatureStatus() {
    try {
      const response = await fetch("/api/features");
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const data = await response.json();
      return data.trading || {};
    } catch (error) {
      console.warn("[Trader] Failed to fetch feature status, defaulting to available:", error);
      return {};
    }
  }

  /**
   * Get the feature status for a tab
   * @param {string} tabId - The tab ID (e.g., "roi", "stop-loss")
   * @returns {string} The status ("available", "coming_soon", "beta", "disabled")
   */
  function getTabFeatureStatus(tabId) {
    const featureKey = TAB_TO_FEATURE_MAP[tabId];
    // Tabs without feature mapping are always available
    if (!featureKey) {
      return FEATURE_STATUS.AVAILABLE;
    }
    if (!tradingFeatures[featureKey]) {
      return FEATURE_STATUS.AVAILABLE;
    }
    return tradingFeatures[featureKey];
  }

  /**
   * Check if a tab is usable (can be clicked/used)
   * @param {string} tabId - The tab ID
   * @returns {boolean} True if tab is available or beta
   */
  function isTabUsable(tabId) {
    const status = getTabFeatureStatus(tabId);
    return status === FEATURE_STATUS.AVAILABLE || status === FEATURE_STATUS.BETA;
  }

  /**
   * Apply feature status to all tab buttons
   */
  function applyFeatureStatusToTabs() {
    const tabButtons = $$(".sub-tab[data-tab-id]");

    tabButtons.forEach((button) => {
      const tabId = button.getAttribute("data-tab-id");
      const status = getTabFeatureStatus(tabId);

      // Remove any existing status badges
      const existingBadge = button.querySelector(".tab-status-badge");
      if (existingBadge) {
        existingBadge.remove();
      }

      // Remove existing status classes
      button.classList.remove(
        "tab-feature-disabled",
        "tab-feature-beta",
        "tab-feature-coming-soon"
      );
      button.removeAttribute("data-feature-status");

      // If available, no modifications needed
      if (status === FEATURE_STATUS.AVAILABLE) {
        return;
      }

      // Get status configuration
      const config = STATUS_CONFIG[status];
      if (!config) return;

      // Add data attribute for styling
      button.setAttribute("data-feature-status", status);

      // Add appropriate class
      if (status === FEATURE_STATUS.DISABLED) {
        button.classList.add("tab-feature-disabled");
      } else if (status === FEATURE_STATUS.BETA) {
        button.classList.add("tab-feature-beta");
      } else if (status === FEATURE_STATUS.COMING_SOON) {
        button.classList.add("tab-feature-coming-soon");
      }

      // Add status badge for non-available/non-beta statuses
      if (status !== FEATURE_STATUS.BETA) {
        const badge = document.createElement("span");
        badge.className = `tab-status-badge ${config.cssClass}`;
        badge.textContent = config.label;
        button.appendChild(badge);
      } else {
        // Beta gets a small indicator badge
        const badge = document.createElement("span");
        badge.className = `tab-status-badge ${config.cssClass}`;
        badge.textContent = config.label;
        button.appendChild(badge);
      }
    });
  }

  /**
   * Handle click on a feature-restricted tab
   * @param {string} tabId - The tab ID being clicked
   * @returns {boolean} True if tab switch should proceed
   */
  function handleFeatureRestrictedTab(tabId) {
    const status = getTabFeatureStatus(tabId);

    // Available and beta tabs can proceed
    if (status === FEATURE_STATUS.AVAILABLE || status === FEATURE_STATUS.BETA) {
      return true;
    }

    // Show toast for restricted tabs
    const config = STATUS_CONFIG[status];
    if (config && config.message) {
      Utils.showToast({
        type: "warning",
        title: config.label,
        message: config.message,
      });
    }

    return false;
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
    const exampleDuration = $("#time-example-duration");

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
    const impactText = $("#roi-impact");
    const exampleProfit = $("#roi-example-profit");
    const exampleTarget = $("#roi-example-target");
    const exampleSummary = $("#roi-example-summary");

    if (!roiInput) return;

    const value = parseFloat(roiInput.value) || 20;

    // Update impact text
    if (impactText) {
      impactText.textContent = `Exit at +${value}% profit`;
    }

    // Update visual example
    if (exampleProfit) {
      exampleProfit.textContent = `+${value}% profit`;
    }
    if (exampleTarget) {
      const targetValue = (0.01 * (1 + value / 100)).toFixed(4);
      exampleTarget.textContent = `${targetValue} SOL`;
    }
    if (exampleSummary) {
      exampleSummary.textContent = `+${value}%`;
    }
  }

  /**
   * Update time override loss example display
   */
  function updateTimeLossExample() {
    const lossInput = $("#time-loss-threshold");
    const impactText = $("#time-loss-impact");
    const exampleLoss = $("#time-example-loss");

    if (!lossInput) return;

    const value = parseFloat(lossInput.value) || -40;
    const absValue = Math.abs(value);

    // Update impact text
    if (impactText) {
      impactText.textContent = `Exit if down ${absValue}% or more after hold period`;
    }

    // Update visual example
    if (exampleLoss) {
      exampleLoss.textContent = `${value}%`;
    }
  }

  /**
   * Update stop loss visual example calculations
   */
  function updateStopLossExample() {
    const thresholdInput = $("#stop-loss-threshold");
    const minHoldInput = $("#stop-loss-min-hold");
    const allowPartialInput = $("#stop-loss-allow-partial");

    if (!thresholdInput) return;

    const threshold = parseFloat(thresholdInput.value) || 50;
    const minHold = parseInt(minHoldInput?.value || "0", 10);
    const allowPartial = allowPartialInput?.checked || false;

    // Update impact text
    const impactText = $("#stop-loss-impact");
    if (impactText) {
      impactText.textContent = `Exit when down ${threshold}% from entry`;
    }

    // Update example values
    const exampleEntry = $("#stop-loss-example-entry");
    const exampleTrigger = $("#stop-loss-example-trigger");
    const exampleExit = $("#stop-loss-example-exit");
    const exampleLoss = $("#stop-loss-example-loss");

    // Example: Entry at 0.01 SOL
    const entryPrice = 0.01;
    const exitPrice = entryPrice * (1 - threshold / 100);

    if (exampleEntry) exampleEntry.textContent = `${entryPrice.toFixed(6)} SOL`;
    if (exampleTrigger) exampleTrigger.textContent = `-${threshold}%`;
    if (exampleExit) exampleExit.textContent = `${exitPrice.toFixed(6)} SOL`;
    if (exampleLoss) exampleLoss.textContent = `-${threshold}%`;

    // Update hold time display
    const holdTimeDisplay = $("#stop-loss-hold-time-display");
    if (holdTimeDisplay) {
      if (minHold === 0) {
        holdTimeDisplay.textContent = "Immediate";
      } else if (minHold < 60) {
        holdTimeDisplay.textContent = `${minHold}s delay`;
      } else if (minHold < 3600) {
        holdTimeDisplay.textContent = `${Math.round(minHold / 60)}m delay`;
      } else {
        holdTimeDisplay.textContent = `${(minHold / 3600).toFixed(1)}h delay`;
      }
    }

    // Update partial exit indicator
    const partialIndicator = $("#stop-loss-partial-indicator");
    if (partialIndicator) {
      partialIndicator.textContent = allowPartial ? "Partial exits allowed" : "Full position exit";
    }
  }

  /**
   * Update trailing stop visual example calculations
   */
  function updateTrailingStopExample() {
    const activationInput = $("#trail-activation");
    const distanceInput = $("#trail-distance");

    if (!activationInput || !distanceInput) return;

    const activation = parseFloat(activationInput.value) || 15;
    const distance = parseFloat(distanceInput.value) || 5;

    // Example scenario: Entry at 1.00 SOL
    const entryPrice = 1.0;
    const activationPrice = entryPrice * (1 + activation / 100);
    const peakPrice = activationPrice * 1.2; // +20% from activation
    const exitPrice = peakPrice * (1 - distance / 100);
    const protectedProfit = ((exitPrice - entryPrice) / entryPrice) * 100;

    // Update timeline values
    const stepEntry = $("#example-entry");
    const stepActivation = $("#example-activation");
    const stepPeak = $("#example-peak");
    const stepExit = $("#example-exit");

    if (stepEntry) stepEntry.textContent = `${entryPrice.toFixed(4)} SOL`;
    if (stepActivation) {
      stepActivation.textContent = `${activationPrice.toFixed(4)} SOL`;
      const activationDetail = $("#example-activation-pct");
      if (activationDetail) activationDetail.textContent = `+${activation}% profit`;
    }
    if (stepPeak) {
      stepPeak.textContent = `${peakPrice.toFixed(4)} SOL`;
      const peakDetail = $("#example-peak-pct");
      if (peakDetail) {
        const gainFromEntry = ((peakPrice - entryPrice) / entryPrice) * 100;
        peakDetail.textContent = `+${gainFromEntry.toFixed(1)}% profit`;
      }
    }
    if (stepExit) {
      stepExit.textContent = `${exitPrice.toFixed(4)} SOL`;
      const exitDetail = $("#example-exit-pct");
      if (exitDetail) exitDetail.textContent = `+${protectedProfit.toFixed(1)}% final`;
    }

    // Update summary
    const summaryProtected = $("#example-protected");
    const summaryAvoided = $("#example-avoided");
    if (summaryProtected) {
      summaryProtected.textContent = `${protectedProfit.toFixed(1)}%`;
    }
    if (summaryAvoided) {
      const avoidedLoss = ((peakPrice - exitPrice) / peakPrice) * 100;
      summaryAvoided.textContent = `${avoidedLoss.toFixed(1)}%`;
    }

    // Update impact indicators
    const activationIndicator = $("#activation-indicator");
    const distanceIndicator = $("#distance-indicator");
    const activationImpact = $("#activation-impact-text");
    const distanceImpact = $("#distance-impact-text");

    if (activationIndicator) {
      activationIndicator.innerHTML =
        activation >= 20
          ? '<i class="icon-triangle-alert"></i>'
          : '<i class="icon-circle-check"></i>';
      activationIndicator.style.background =
        activation >= 20 ? "var(--warning-alpha-10)" : "var(--success-alpha-10)";
      activationIndicator.style.color = activation >= 20 ? "var(--warning)" : "var(--success)";
    }

    if (activationImpact) {
      if (activation < 10) {
        activationImpact.textContent = "Activates quickly - good for volatile tokens";
      } else if (activation < 20) {
        activationImpact.textContent = "Balanced activation - suitable for most scenarios";
      } else {
        activationImpact.textContent = "Delayed activation - may miss protection window";
      }
    }

    if (distanceIndicator) {
      distanceIndicator.innerHTML =
        distance >= 10
          ? '<i class="icon-triangle-alert"></i>'
          : '<i class="icon-circle-check"></i>';
      distanceIndicator.style.background =
        distance >= 10 ? "var(--warning-alpha-10)" : "var(--success-alpha-10)";
      distanceIndicator.style.color = distance >= 10 ? "var(--warning)" : "var(--success)";
    }

    if (distanceImpact) {
      if (distance < 5) {
        distanceImpact.textContent = "Tight protection - may exit on minor dips";
      } else if (distance < 10) {
        distanceImpact.textContent = "Balanced protection - good for most situations";
      } else {
        distanceImpact.textContent = "Loose protection - allows larger pullbacks";
      }
    }
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
   * Configure ActionBar based on current subtab
   */
  function configureActionBar(tabId) {
    if (!actionBar) return;

    switch (tabId) {
      case "stats":
        // Stats tab is read-only, no actions needed
        actionBar.clear();
        break;

      case "stop-loss":
        actionBar.configure({
          title: "Stop Loss Configuration",
          subtitle: "Automatic exit when loss exceeds threshold",
          icon: "icon-shield-off",
          actions: [
            {
              id: "reset",
              label: "Reset to Defaults",
              icon: "icon-rotate-ccw",
              variant: "secondary",
              onClick: async () => {
                const { confirmed } = await ConfirmationDialog.show({
                  title: "Reset Stop Loss",
                  message:
                    "This will reset stop loss settings to default values:\n• Disabled\n• Threshold: 50%\n• Partial exits: Disabled\n• Min hold time: 0s",
                  confirmLabel: "Reset",
                  cancelLabel: "Cancel",
                  variant: "warning",
                });

                if (confirmed) {
                  await saveConfig({
                    trader: {
                      stop_loss_enabled: false,
                      stop_loss_threshold_pct: 50.0,
                      stop_loss_allow_partial: false,
                      stop_loss_min_hold_seconds: 0,
                    },
                  });
                }
              },
            },
            {
              id: "save",
              label: "Save Configuration",
              icon: "icon-save",
              variant: "primary",
              onClick: async () => {
                const enabled = $("#stop-loss-enabled")?.checked || false;
                const threshold = parseFloat($("#stop-loss-threshold")?.value || "50");
                const allowPartial = $("#stop-loss-allow-partial")?.checked || false;
                const minHoldSeconds = parseInt($("#stop-loss-min-hold")?.value || "0", 10);
                await saveConfig({
                  trader: {
                    stop_loss_enabled: enabled,
                    stop_loss_threshold_pct: threshold,
                    stop_loss_allow_partial: allowPartial,
                    stop_loss_min_hold_seconds: minHoldSeconds,
                  },
                });
              },
            },
          ],
        });
        break;

      case "trailing-stop":
        actionBar.configure({
          title: "Trailing Stop Configuration",
          subtitle: "Automatically exit when price drops from peak",
          icon: "icon-trending-up",
          actions: [
            {
              id: "reset",
              label: "Reset to Defaults",
              icon: "icon-rotate-ccw",
              variant: "secondary",
              onClick: async () => {
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
              },
            },
            {
              id: "save",
              label: "Save Configuration",
              icon: "icon-save",
              variant: "primary",
              onClick: async () => {
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
              },
            },
          ],
        });
        break;

      case "roi":
        actionBar.configure({
          title: "Take Profit Configuration",
          subtitle: "Automatically exit at target profit levels",
          icon: "icon-target",
          actions: [
            {
              id: "reset",
              label: "Reset to Defaults",
              icon: "icon-rotate-ccw",
              variant: "secondary",
              onClick: async () => {
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
              },
            },
            {
              id: "save",
              label: "Save Configuration",
              icon: "icon-save",
              variant: "primary",
              onClick: async () => {
                const enabled = $("#roi-enabled")?.checked || false;
                const target = parseFloat($("#roi-target")?.value || "20");
                await saveConfig({
                  trader: {
                    roi_exit_enabled: enabled,
                    roi_target_percent: target,
                  },
                });
              },
            },
          ],
        });
        break;

      case "time-rules":
        actionBar.configure({
          title: "Time Rules Configuration",
          subtitle: "Exit positions based on holding duration and loss threshold",
          icon: "icon-timer",
          actions: [
            {
              id: "reset",
              label: "Reset to Defaults",
              icon: "icon-rotate-ccw",
              variant: "secondary",
              onClick: async () => {
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
              },
            },
            {
              id: "save",
              label: "Save Configuration",
              icon: "icon-save",
              variant: "primary",
              onClick: async () => {
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
              },
            },
          ],
        });
        break;

      case "strategy-control":
        actionBar.configure({
          title: "Strategy Control",
          subtitle: "Enable or disable automated trading strategies",
          icon: "icon-puzzle",
          actions: [
            {
              id: "refresh",
              label: "Refresh List",
              icon: "icon-refresh-cw",
              variant: "secondary",
              onClick: async () => {
                await loadStrategies();
                Utils.showToast({
                  type: "info",
                  title: "Strategies Refreshed",
                  message: "Strategy list reloaded from server",
                });
              },
            },
            {
              id: "save",
              label: "Save Configuration",
              icon: "icon-save",
              variant: "primary",
              onClick: async () => {
                await loadStrategies();
                Utils.showToast({
                  type: "success",
                  title: "Strategies Saved",
                  message: "Strategy configuration updated successfully",
                });
              },
            },
          ],
        });
        break;

      case "dca":
        actionBar.configure({
          title: "DCA Configuration",
          subtitle: "Dollar Cost Averaging for position management",
          icon: "icon-dollar-sign",
          actions: [
            {
              id: "reset",
              label: "Reset to Defaults",
              icon: "icon-rotate-ccw",
              variant: "secondary",
              onClick: async () => {
                const { confirmed } = await ConfirmationDialog.show({
                  title: "Reset DCA Settings",
                  message:
                    "This will reset all DCA settings to default values.\n\nThis action cannot be undone.",
                  confirmLabel: "Reset",
                  cancelLabel: "Cancel",
                  variant: "warning",
                });

                if (confirmed) {
                  await saveConfig({
                    trader: {
                      dca_enabled: false,
                      dca_threshold_pct: -10,
                      dca_max_count: 2,
                      dca_size_percentage: 50,
                      dca_cooldown_minutes: 30,
                    },
                  });
                }
              },
            },
            {
              id: "save",
              label: "Save DCA Configuration",
              icon: "icon-save",
              variant: "primary",
              onClick: async () => {
                const dcaEnabled = $("#dca-enabled")?.checked || false;
                const dcaThreshold = parseFloat($("#dca-threshold")?.value || "-10");
                const dcaMaxCount = parseInt($("#dca-max-count")?.value || "2", 10);
                const dcaSize = parseFloat($("#dca-size")?.value || "50");
                const dcaCooldown = parseInt($("#dca-cooldown")?.value || "30", 10);

                await saveConfig({
                  trader: {
                    dca_enabled: dcaEnabled,
                    dca_threshold_pct: dcaThreshold,
                    dca_max_count: dcaMaxCount,
                    dca_size_percentage: dcaSize,
                    dca_cooldown_minutes: dcaCooldown,
                  },
                });
              },
            },
          ],
        });
        break;

      case "general-settings":
        actionBar.configure({
          title: "General Settings",
          subtitle: "Position sizing, concurrency, and trading mode",
          icon: "icon-settings",
          actions: [
            {
              id: "export",
              label: "Export Config",
              icon: "icon-download",
              variant: "secondary",
              onClick: () => {
                exportConfig();
              },
            },
            {
              id: "import",
              label: "Import Config",
              icon: "icon-upload",
              variant: "secondary",
              onClick: () => {
                importConfig();
              },
            },
            {
              id: "reset",
              label: "Reset to Defaults",
              icon: "icon-rotate-ccw",
              variant: "secondary",
              onClick: async () => {
                const { confirmed } = await ConfirmationDialog.show({
                  title: "Reset General Settings",
                  message:
                    "This will reset all general settings to default values.\n\nThis action cannot be undone.",
                  confirmLabel: "Reset",
                  cancelLabel: "Cancel",
                  variant: "warning",
                });

                if (confirmed) {
                  await saveConfig({
                    trader: {
                      max_open_positions: 2,
                      trade_size_sol: 0.005,
                      entry_sizes: [0.005, 0.01, 0.02, 0.05],
                      close_cooldown_seconds: 600,
                      entry_monitor_concurrency: 10,
                    },
                  });
                }
              },
            },
            {
              id: "save",
              label: "Save Configuration",
              icon: "icon-save",
              variant: "primary",
              onClick: async () => {
                const maxPositions = parseInt($("#max-positions")?.value || "2", 10);
                const tradeSize = parseFloat($("#trade-size")?.value || "0.005");
                const entrySizesRaw = $("#entry-sizes")?.value || "0.005, 0.01, 0.02, 0.05";
                const entrySizes = entrySizesRaw
                  .split(",")
                  .map((s) => parseFloat(s.trim()))
                  .filter((n) => !isNaN(n));
                const closeCooldownMinutes = parseInt($("#close-cooldown")?.value || "10", 10);
                const closeCooldownSeconds = Number.isNaN(closeCooldownMinutes)
                  ? 600
                  : Math.max(0, closeCooldownMinutes) * 60;
                const entryConcurrency = parseInt($("#entry-concurrency")?.value || "3", 10);

                await saveConfig({
                  trader: {
                    max_open_positions: maxPositions,
                    trade_size_sol: tradeSize,
                    entry_sizes: entrySizes,
                    close_cooldown_seconds: closeCooldownSeconds,
                    entry_monitor_concurrency: entryConcurrency,
                  },
                });
              },
            },
          ],
        });
        break;

      default:
        actionBar.clear();
    }
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

    // Load preview when switching to stop loss tab
    if (tabId === "stop-loss") {
      updateStopLossExample();
    }

    // Load preview when switching to trailing stop tab
    if (tabId === "trailing-stop") {
      updateTrailingStopExample();
      loadTrailingStopStats();
      loadTrailingStopPreview();
    }

    // Update tab-specific data
    if (tabId === "time-rules") {
      updateTimeRulesStatus();
    }

    // Configure ActionBar for the current tab
    configureActionBar(tabId);
  }

  /**
   * Load configuration from server
   */
  async function loadConfig() {
    try {
      const data = await requestManager.fetch("/api/config", {
        priority: "normal",
      });
      state.config = data.config;

      // Update form fields
      updateFormFields();

      // Update config overview in stats tab
      updateConfigOverview();

      // Update visual examples with loaded values
      updateStopLossExample();
      updateRoiExample();
      updateTimeLossExample();
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
   * Update positions summary section
   */
  async function updatePositionsSummary() {
    const positionsSummary = $("#positions-summary");
    if (!positionsSummary) return;

    try {
      const data = await requestManager.fetch("/api/positions", {
        priority: "normal",
      });

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
  async function loadStrategies() {
    try {
      const data = await requestManager.fetch("/api/strategies", {
        priority: "normal",
      });

      state.strategies = data.strategies || [];

      if (state.config) {
        updateConfigOverview();
      }

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
      const handler = async (e) => {
        const strategyId = parseInt(e.target.dataset.strategyId, 10);
        const enabled = e.target.checked;
        await updateStrategyStatus(strategyId, enabled);
      };
      checkbox.addEventListener("change", handler);
      eventCleanups.push(() => checkbox.removeEventListener("change", handler));
    });
  }

  /**
   * Update strategy enabled/disabled status
   */
  async function updateStrategyStatus(strategyId, enabled) {
    try {
      await requestManager.fetch(`/api/strategies/${strategyId}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled }),
        priority: "high",
      });

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

  // ============================================================================
  // Auto Trader Toggle Functions
  // ============================================================================

  /**
   * Update all auto trader status bars on the page
   */
  function updateAutoTraderStatusBars(status) {
    const isRunning = status?.running === true;
    const isAvailable = status !== undefined && status !== null;
    const statusText = isRunning ? "Running" : "Stopped";
    const statusAttr = isRunning ? "running" : "stopped";
    const toggleLabel = isRunning ? "ON" : "OFF";

    // Update stats tab status bar
    const statsBar = $("#trader-status-bar");
    if (statsBar) {
      statsBar.setAttribute("data-status", statusAttr);
      const statsStatusText = $("#trader-status-text");
      if (statsStatusText) statsStatusText.textContent = statusText;
      const statsToggle = $("#stats-trader-toggle");
      if (statsToggle) {
        statsToggle.checked = isRunning;
        statsToggle.disabled = !isAvailable;
      }
      const statsToggleLabel = $("#stats-toggle-label");
      if (statsToggleLabel) statsToggleLabel.textContent = toggleLabel;
    }

    // Update settings tab status bar
    const settingsBar = $("#settings-trader-status-bar");
    if (settingsBar) {
      settingsBar.setAttribute("data-status", statusAttr);
      const settingsStatusText = $("#settings-trader-status-text");
      if (settingsStatusText) settingsStatusText.textContent = statusText;
      const settingsToggle = $("#settings-trader-toggle");
      if (settingsToggle) {
        settingsToggle.checked = isRunning;
        settingsToggle.disabled = !isAvailable;
      }
      const settingsToggleLabel = $("#settings-toggle-label");
      if (settingsToggleLabel) settingsToggleLabel.textContent = toggleLabel;
    }
  }

  /**
   * Toggle auto trader on/off
   */
  async function toggleAutoTrader(shouldStart, _triggerElement) {
    // Disable all toggles while processing
    const allToggles = [$("#stats-trader-toggle"), $("#settings-trader-toggle")];
    allToggles.forEach((toggle) => {
      if (toggle) toggle.disabled = true;
    });

    const endpoint = shouldStart ? "/api/trader/start" : "/api/trader/stop";

    try {
      const response = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
      });

      if (!response.ok) {
        throw new Error(`Failed to ${shouldStart ? "start" : "stop"} trader`);
      }

      // Play sound feedback
      if (shouldStart) {
        playToggleOn();
      } else {
        playToggleOff();
      }

      // Show toast
      Utils.showToast(`Auto Trader ${shouldStart ? "started" : "stopped"}`, "success");

      // Update status bars
      updateAutoTraderStatusBars({ running: shouldStart });
    } catch (error) {
      console.error("Toggle auto trader error:", error);
      Utils.showToast(error.message, "error");
      playError();

      // Revert toggle states
      updateAutoTraderStatusBars({ running: !shouldStart });
    } finally {
      // Re-enable all toggles
      allToggles.forEach((toggle) => {
        if (toggle) toggle.disabled = false;
      });
    }
  }

  /**
   * Setup auto trader toggle event handlers
   */
  function setupAutoTraderToggles() {
    const statsToggle = $("#stats-trader-toggle");
    const settingsToggle = $("#settings-trader-toggle");

    if (statsToggle) {
      addTrackedListener(statsToggle, "change", (e) => {
        toggleAutoTrader(e.target.checked, statsToggle);
      });
    }

    if (settingsToggle) {
      addTrackedListener(settingsToggle, "change", (e) => {
        toggleAutoTrader(e.target.checked, settingsToggle);
      });
    }

    // Initial fetch of trader status
    fetchTraderStatus();
  }

  /**
   * Fetch and update trader status
   */
  async function fetchTraderStatus() {
    try {
      const response = await fetch("/api/trader/status");
      if (response.ok) {
        const data = await response.json();
        updateAutoTraderStatusBars(data);
      }
    } catch (error) {
      console.warn("[Trader] Failed to fetch trader status:", error);
    }
  }

  // ============================================================================
  // Trading Controls (Force Stop, Monitor Toggles, Loss Limit)
  // ============================================================================

  /**
   * Load trading controls status from API
   */
  async function loadControlsStatus() {
    try {
      // Load force stop status
      const forceStopRes = await fetch("/api/trader/force-stop/status");
      if (forceStopRes.ok) {
        const forceStopData = await forceStopRes.json();
        updateForceStopBanner(forceStopData.data);
      }

      // Load monitors status
      const monitorsRes = await fetch("/api/trader/monitors/status");
      if (monitorsRes.ok) {
        const monitorsData = await monitorsRes.json();
        updateMonitorControls(monitorsData.data);
      }

      // Load loss limit status
      const lossLimitRes = await fetch("/api/trader/loss-limit/status");
      if (lossLimitRes.ok) {
        const lossLimitData = await lossLimitRes.json();
        updateLossLimitPanel(lossLimitData.data);
      }
    } catch (err) {
      console.error("[Trader] Failed to load controls status:", err);
    }
  }

  /**
   * Update force stop banner visibility and content
   */
  function updateForceStopBanner(data) {
    const banner = $("#force-stop-banner");
    const btn = $("#force-stop-btn");

    if (!banner || !btn) return;

    if (data && data.is_stopped) {
      banner.style.display = "flex";
      const reasonEl = $("#force-stop-reason");
      if (reasonEl) {
        reasonEl.textContent = data.reason || "Manual force stop";
      }
      btn.style.display = "none";
    } else {
      banner.style.display = "none";
      btn.style.display = "flex";
    }
  }

  /**
   * Update monitor toggle controls
   */
  function updateMonitorControls(data) {
    const entryToggle = $("#entry-monitor-toggle");
    const exitToggle = $("#exit-monitor-toggle");
    const entryStatus = $("#entry-monitor-status");
    const exitStatus = $("#exit-monitor-status");

    if (!data) return;

    if (entryToggle) {
      entryToggle.checked = data.entry_monitor?.enabled ?? false;
      entryToggle.disabled = data.force_stopped ?? false;
    }
    if (exitToggle) {
      exitToggle.checked = data.exit_monitor?.enabled ?? false;
      exitToggle.disabled = data.force_stopped ?? false;
    }

    if (entryStatus) {
      const running = data.entry_monitor?.running ?? false;
      entryStatus.textContent = running ? "Running" : "Stopped";
      entryStatus.className = "control-status " + (running ? "status-running" : "status-stopped");
    }

    if (exitStatus) {
      const running = data.exit_monitor?.running ?? false;
      exitStatus.textContent = running ? "Running" : "Stopped";
      exitStatus.className = "control-status " + (running ? "status-running" : "status-stopped");
    }
  }

  /**
   * Update loss limit panel display
   */
  function updateLossLimitPanel(data) {
    const panel = $("#loss-limit-panel");

    if (!panel) return;

    if (!data || !data.enabled) {
      panel.style.display = "none";
      return;
    }

    panel.style.display = "block";

    const value = $("#loss-limit-value");
    const progress = $("#loss-limit-progress");
    const period = $("#loss-limit-period");
    const status = $("#loss-limit-status");

    if (value) {
      const currentLoss = data.current_loss_sol?.toFixed(4) ?? "0.0000";
      const limitSol = data.limit_sol?.toFixed(4) ?? "0.0000";
      value.textContent = `${currentLoss} / ${limitSol} SOL`;
    }

    if (progress) {
      const percent = Math.min(data.progress_percent ?? 0, 100);
      progress.style.width = `${percent}%`;

      progress.classList.remove("limit-exceeded", "limit-warning");
      if (percent >= 100) {
        progress.classList.add("limit-exceeded");
      } else if (percent >= 75) {
        progress.classList.add("limit-warning");
      }
    }

    if (period) {
      const remainingSecs = data.period_remaining_secs ?? 0;
      const hours = Math.floor(remainingSecs / 3600);
      const mins = Math.floor((remainingSecs % 3600) / 60);
      period.textContent = `Resets in ${hours}h ${mins}m`;
    }

    if (status) {
      if (data.is_limited) {
        status.textContent = "LIMIT REACHED";
        status.className = "loss-limit-status status-limited";
      } else {
        status.textContent = "";
        status.className = "loss-limit-status";
      }
    }
  }

  /**
   * Setup event handlers for trading controls
   */
  function setupControlsEventHandlers() {
    // Force Stop button
    const forceStopBtn = $("#force-stop-btn");
    if (forceStopBtn) {
      addTrackedListener(forceStopBtn, "click", async () => {
        const result = await ConfirmationDialog.show({
          title: "Force Stop Trading",
          message: "This will immediately halt ALL trading operations. Continue?",
          confirmLabel: "Stop Trading",
          variant: "danger",
        });
        if (!result.confirmed) return;

        try {
          const res = await fetch("/api/trader/force-stop", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ reason: "Manual force stop from dashboard" }),
          });
          const data = await res.json();
          if (data.success) {
            Utils.showToast("Force stop activated", "warning");
            playError();
            await loadControlsStatus();
          } else {
            Utils.showToast(data.error || "Failed to activate force stop", "error");
          }
        } catch {
          Utils.showToast("Failed to activate force stop", "error");
        }
      });
    }

    // Resume button
    const resumeBtn = $("#resume-trading-btn");
    if (resumeBtn) {
      addTrackedListener(resumeBtn, "click", async () => {
        try {
          const res = await fetch("/api/trader/resume", { method: "POST" });
          const data = await res.json();
          if (data.success) {
            Utils.showToast("Force stop cleared", "success");
            playToggleOn();
            await loadControlsStatus();
          } else {
            Utils.showToast(data.error || "Failed to resume trading", "error");
          }
        } catch {
          Utils.showToast("Failed to resume trading", "error");
        }
      });
    }

    // Entry monitor toggle
    const entryToggle = $("#entry-monitor-toggle");
    if (entryToggle) {
      addTrackedListener(entryToggle, "change", async (e) => {
        try {
          const res = await fetch("/api/trader/monitors/entry/toggle", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ enabled: e.target.checked }),
          });
          const data = await res.json();
          if (!data.success) {
            e.target.checked = !e.target.checked; // Revert
            Utils.showToast("Failed to toggle entry monitor", "error");
          } else {
            e.target.checked ? playToggleOn() : playToggleOff();
          }
        } catch {
          e.target.checked = !e.target.checked;
          Utils.showToast("Failed to toggle entry monitor", "error");
        }
      });
    }

    // Exit monitor toggle
    const exitToggle = $("#exit-monitor-toggle");
    if (exitToggle) {
      addTrackedListener(exitToggle, "change", async (e) => {
        try {
          const res = await fetch("/api/trader/monitors/exit/toggle", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ enabled: e.target.checked }),
          });
          const data = await res.json();
          if (!data.success) {
            e.target.checked = !e.target.checked; // Revert
            Utils.showToast("Failed to toggle exit monitor", "error");
          } else {
            e.target.checked ? playToggleOn() : playToggleOff();
          }
        } catch {
          e.target.checked = !e.target.checked;
          Utils.showToast("Failed to toggle exit monitor", "error");
        }
      });
    }
  }

  /**
   * Setup form submission handlers
   * Note: Button handlers moved to ActionBar in configureActionBar()
   */
  function setupFormHandlers() {
    // Setup auto trader toggle handlers
    setupAutoTraderToggles();

    // Stop loss threshold input listener
    const stopLossThreshold = $("#stop-loss-threshold");
    if (stopLossThreshold) {
      addTrackedListener(stopLossThreshold, "input", () => {
        updateStopLossExample();
      });
    }

    // Stop loss min hold input listener
    const stopLossMinHold = $("#stop-loss-min-hold");
    if (stopLossMinHold) {
      addTrackedListener(stopLossMinHold, "input", () => {
        updateStopLossExample();
      });
    }

    // Stop loss allow partial toggle listener
    const stopLossAllowPartial = $("#stop-loss-allow-partial");
    if (stopLossAllowPartial) {
      addTrackedListener(stopLossAllowPartial, "change", () => {
        updateStopLossExample();
      });
    }

    // Time unit change listener
    const timeUnit = $("#time-unit");
    if (timeUnit) {
      addTrackedListener(timeUnit, "change", () => {
        updateTimeConversionHint();
      });
    }

    // Time duration input listener
    const timeMaxHold = $("#time-max-hold");
    if (timeMaxHold) {
      addTrackedListener(timeMaxHold, "input", () => {
        updateTimeConversionHint();
      });
    }

    // ROI target input listener
    const roiTarget = $("#roi-target");
    if (roiTarget) {
      addTrackedListener(roiTarget, "input", () => {
        updateRoiExample();
      });
    }

    // Time loss threshold input listener
    const timeLossThreshold = $("#time-loss-threshold");
    if (timeLossThreshold) {
      addTrackedListener(timeLossThreshold, "input", () => {
        updateTimeLossExample();
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
            updateTrailingStopExample();
          }, 300)
        : () => {
            updateTrailingStopExample();
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
   * Save configuration updates
   */
  async function saveConfig(updates) {
    try {
      await requestManager.fetch("/api/config", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(updates),
        priority: "high",
      });

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
      const featurePromise = fetchFeatureStatus();

      // Initialize ActionBar
      actionBar = new ActionBar({
        container: "#toolbarContainer",
      });

      // Register with ActionBarManager for page-switch coordination
      ActionBarManager.register("trader", actionBar);

      // Integrate with lifecycle for auto-cleanup (clears on deactivate, disposes on dispose)
      ctx.manageActionBar(actionBar);

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
          return handleFeatureRestrictedTab(newTabId);
        },
      });

      // Register with TabBarManager for page-switch coordination
      TabBarManager.register("trader", tabBar);

      // Integrate with lifecycle for auto-cleanup
      ctx.manageTabBar(tabBar);

      // Show the tab bar
      tabBar.show();

      // Apply feature status badges/styling to tabs
      applyFeatureStatusToTabs();

      // Sync state with tab bar's restored state (from server or URL hash)
      const activeTab = tabBar.getActiveTab();
      if (activeTab && activeTab !== state.currentTab) {
        // Ensure the restored tab is usable
        if (isTabUsable(activeTab)) {
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
      setupControlsEventHandlers();

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
              await loadControlsStatus();
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
        await loadControlsStatus();
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
      // Pollers stopped automatically by lifecycle context
    },

    /**
     * Dispose the page (cleanup)
     */
    dispose() {
      console.log("[Trader] Disposing page");

      // Unregister ActionBar from manager (lifecycle already disposes it via manageActionBar)
      ActionBarManager.unregister("trader");
      actionBar = null;

      // Clean up all tracked event listeners
      eventCleanups.forEach((cleanup) => cleanup());
      eventCleanups.length = 0;

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
