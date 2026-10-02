// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Trader example panels - live calculation previews for stop loss, trailing stop and ROI settings.

import { formatFixed, formatPercentValue, formatSol, formatTimeSpan } from "../../core/format.js";

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
export function createExampleUpdaters({ $, Utils: _Utils }) {
  /**
   * A duration in one of the configurable units, worded through the catalog, or
   * `null` for a unit the page does not offer.
   */
  function describeDuration(count, unit) {
    const args = { count, amount: String(count) };
    switch (unit) {
      case "seconds":
        return I18n.t("trader-duration-seconds", args);
      case "minutes":
        return I18n.t("trader-duration-minutes", args);
      case "hours":
        return I18n.t("trader-duration-hours", args);
      case "days":
        return I18n.t("trader-duration-days", args);
      default:
        return null;
    }
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
      return describeDuration(totalSeconds / 86400, "days");
    }
    if (totalSeconds >= 3600 && totalSeconds % 3600 === 0) {
      return describeDuration(totalSeconds / 3600, "hours");
    }
    if (totalSeconds >= 60 && totalSeconds % 60 === 0) {
      return describeDuration(totalSeconds / 60, "minutes");
    }
    return describeDuration(totalSeconds, "seconds");
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
    hintText.textContent = I18n.t("trader-time-conversion", {
      duration: describeDuration(duration, unit) ?? `${duration} ${unit}`,
      readable,
    });

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
      impactText.textContent = I18n.t("trader-roi-impact", { target: String(value) });
    }

    // Update visual example
    if (exampleProfit) {
      exampleProfit.textContent = I18n.t("trader-example-profit", { value: String(value) });
    }
    if (exampleTarget) {
      exampleTarget.textContent = formatSol(0.01 * (1 + value / 100), { decimals: 4 });
    }
    if (exampleSummary) {
      exampleSummary.innerHTML = I18n.markup("trader-roi-summary", { target: String(value) });
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
      impactText.textContent = I18n.t("trader-time-loss-impact", { value: String(absValue) });
    }

    // Update visual example
    if (exampleLoss) {
      exampleLoss.textContent = I18n.t("trader-value-percent", { value: String(value) });
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
      impactText.textContent = I18n.t("trader-stop-loss-impact", { threshold: String(threshold) });
    }

    // Update example values
    const exampleEntry = $("#stop-loss-example-entry");
    const exampleTrigger = $("#stop-loss-example-trigger");
    const exampleExit = $("#stop-loss-example-exit");
    const exampleLoss = $("#stop-loss-example-loss");

    // Example: Entry at 0.01 SOL
    const entryPrice = 0.01;
    const exitPrice = entryPrice * (1 - threshold / 100);

    if (exampleEntry) exampleEntry.textContent = formatSol(entryPrice, { decimals: 6 });
    const lossPercent = I18n.t("trader-value-percent", { value: `-${threshold}` });
    if (exampleTrigger) exampleTrigger.textContent = lossPercent;
    if (exampleExit) exampleExit.textContent = formatSol(exitPrice, { decimals: 6 });
    if (exampleLoss) {
      exampleLoss.innerHTML = I18n.markup("trader-stop-loss-summary", { loss: lossPercent });
    }

    // Update hold time display
    const holdTimeDisplay = $("#stop-loss-hold-time-display");
    if (holdTimeDisplay) {
      if (minHold === 0) {
        holdTimeDisplay.textContent = I18n.t("trader-stop-loss-hold-immediate");
      } else {
        const span =
          minHold < 60
            ? formatTimeSpan(minHold)
            : minHold < 3600
              ? formatTimeSpan(Math.round(minHold / 60), { unit: "minute" })
              : formatTimeSpan(minHold / 3600, { unit: "hour", decimals: 1 });
        holdTimeDisplay.textContent = I18n.t("trader-stop-loss-hold-delay", { span });
      }
    }

    // Update partial exit indicator
    const partialIndicator = $("#stop-loss-partial-indicator");
    if (partialIndicator) {
      partialIndicator.textContent = allowPartial
        ? I18n.t("trader-stop-loss-partial")
        : I18n.t("trader-step-full-exit");
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

    if (stepEntry) stepEntry.textContent = formatSol(entryPrice, { decimals: 4 });
    if (stepActivation) {
      stepActivation.textContent = formatSol(activationPrice, { decimals: 4 });
      const activationDetail = $("#example-activation-pct");
      if (activationDetail) {
        activationDetail.textContent = I18n.t("trader-example-profit", {
          value: String(activation),
        });
      }
    }
    if (stepPeak) {
      stepPeak.textContent = formatSol(peakPrice, { decimals: 4 });
      const peakDetail = $("#example-peak-pct");
      if (peakDetail) {
        const gainFromEntry = ((peakPrice - entryPrice) / entryPrice) * 100;
        peakDetail.textContent = I18n.t("trader-example-profit", {
          value: formatFixed(gainFromEntry, { decimals: 1 }),
        });
      }
    }
    if (stepExit) {
      stepExit.textContent = formatSol(exitPrice, { decimals: 4 });
      const exitDetail = $("#example-exit-pct");
      if (exitDetail) {
        exitDetail.textContent = I18n.t("trader-trailing-final", {
          value: formatFixed(protectedProfit, { decimals: 1 }),
        });
      }
    }

    // Update summary
    const summaryProtected = $("#example-protected");
    const summaryAvoided = $("#example-avoided");
    if (summaryProtected) {
      summaryProtected.innerHTML = I18n.markup("trader-trailing-summary-protected", {
        value: formatPercentValue(protectedProfit, { decimals: 1, plus: "" }),
      });
    }
    if (summaryAvoided) {
      const avoidedLoss = ((peakPrice - exitPrice) / peakPrice) * 100;
      summaryAvoided.innerHTML = I18n.markup("trader-trailing-summary-avoided", {
        value: formatPercentValue(avoidedLoss, { decimals: 1, plus: "" }),
      });
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
