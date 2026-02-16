/**
 * Trader Example Updaters Module
 *
 * Visual example calculation and rendering functions for the trader page.
 * These functions update the example panels in each configuration tab to show
 * users how their settings will affect trading behavior.
 */

/**
 * Create example updater functions
 * @param {Object} deps - Dependencies
 * @param {Function} deps.$ - DOM selector function
 * @param {Object} deps.Utils - Utility functions module
 * @returns {Object} Example updater functions
 */
export function createExampleUpdaters({ $, Utils }) {
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

  // Return public API
  return {
    updateRoiExample,
    updateStopLossExample,
    updateTrailingStopExample,
    updateTimeConversionHint,
    updateTimeLossExample,
    convertTimeToReadable,
  };
}
