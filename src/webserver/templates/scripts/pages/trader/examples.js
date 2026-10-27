// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Trader example panels - live calculation previews for stop loss, trailing stop and ROI settings.

import {
  formatFixed,
  formatPercentValue,
  formatSol,
  formatTimeSpan,
} from "../../core/format.js";

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
 * @param {() => (number|null|undefined)} deps.tradeSize - The configured trade size in SOL
 *   (`trader.trade_size_sol`), or nothing before the config has loaded
 * @returns {Object} Example updater functions
 */
export function createExampleUpdaters({ $, Utils: _Utils, tradeSize }) {
  /** A SOL amount in an example or a computed line; "—" while the trade size is unknown. */
  const exampleSol = (amount) => formatSol(amount, { decimals: 6, fallback: "—", subscript: true });

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
    const entry = Number(tradeSize?.());
    const exampleEntry = $("#roi-example-entry");
    if (exampleEntry) exampleEntry.textContent = exampleSol(entry);
    if (exampleTarget) exampleTarget.textContent = exampleSol(entry * (1 + value / 100));
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

    // The field holds the loss as its magnitude; the "−" is the field's fixed prefix.
    const loss = Math.abs(parseFloat(lossInput.value) || 0);

    if (impactText) {
      impactText.textContent = I18n.t("trader-time-loss-impact", { value: String(loss) });
    }

    if (exampleLoss) {
      exampleLoss.textContent = I18n.t("trader-value-percent", { value: String(loss ? -loss : 0) });
    }
  }

  /**
   * Update stop loss visual example calculations
   */
  function updateStopLossExample() {
    const thresholdInput = $("#stop-loss-threshold");
    const minHoldInput = $("#stop-loss-min-hold");
    const allowPartialInput = $("#stop-loss-allow-partial");
    const partialPctInput = $("#stop-loss-partial-pct");

    if (!thresholdInput) return;

    const threshold = parseFloat(thresholdInput.value) || 50;
    const minHold = parseInt(minHoldInput?.value || "0", 10);
    const allowPartial = allowPartialInput?.checked === true;
    // The share the first stop-loss sale sells, as loaded from `trader.stop_loss_partial_pct`
    // or as edited; the second sale sells the rest.
    const partialPct = Number(partialPctInput?.value);

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

    // The example position is one buy at the configured trade size.
    const entry = Number(tradeSize?.());
    const exit = entry * (1 - threshold / 100);

    if (exampleEntry) exampleEntry.textContent = exampleSol(entry);
    const lossPercent = I18n.t("trader-value-percent", { value: `-${threshold}` });
    if (exampleTrigger) exampleTrigger.textContent = lossPercent;
    if (exampleExit) {
      exampleExit.textContent = exampleSol(allowPartial ? exit * (partialPct / 100) : exit);
    }
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

    // With "Sell Part First" on, the threshold sells the configured part and a price still
    // below the stop sells the rest; with it off, the threshold sells everything at once.
    const exitTitle = $("#stop-loss-exit-title");
    if (exitTitle) {
      exitTitle.textContent = allowPartial
        ? I18n.t("trader-stop-loss-sell-part", {
            part: formatPercentValue(partialPct, { decimals: 1, includeSign: false, trim: true }),
          })
        : I18n.t("trader-step-auto-exit");
    }
    const exitDetail = $("#stop-loss-exit-detail");
    if (exitDetail) {
      exitDetail.textContent = allowPartial
        ? I18n.t("trader-stop-loss-part-of-position")
        : I18n.t("trader-step-full-exit");
    }
    const restArrow = $("#stop-loss-rest-arrow");
    const restStep = $("#stop-loss-rest-step");
    if (restArrow) restArrow.hidden = !allowPartial;
    if (restStep) restStep.hidden = !allowPartial;
    const exampleRest = $("#stop-loss-example-rest");
    if (exampleRest) exampleRest.textContent = exampleSol(exit * (1 - partialPct / 100));
  }

  /** A percent figure in an example strip: one decimal at most, no trailing zero. */
  const examplePercent = (value) => formatFixed(value, { decimals: 1, trim: true });

  /**
   * Update trailing stop visual example calculations
   */
  function updateTrailingStopExample() {
    const activationInput = $("#trail-activation");
    const distanceInput = $("#trail-distance");

    if (!activationInput || !distanceInput) return;

    const activation = parseFloat(activationInput.value) || 15;
    const distance = parseFloat(distanceInput.value) || 5;

    // The example position is one buy at the configured trade size.
    const entryValue = Number(tradeSize?.());
    // Relative steps: activation, a peak 20% above it, and the trail below the peak.
    const activationRatio = 1 + activation / 100;
    const peakRatio = activationRatio * 1.2;
    const exitRatio = peakRatio * (1 - distance / 100);
    const activationValue = entryValue * activationRatio;
    const peakValue = entryValue * peakRatio;
    const exitValue = entryValue * exitRatio;
    const protectedProfit = (exitRatio - 1) * 100;

    // Update timeline values
    const stepEntry = $("#example-entry");
    const stepActivation = $("#example-activation");
    const stepPeak = $("#example-peak");
    const stepExit = $("#example-exit");

    if (stepEntry) stepEntry.textContent = exampleSol(entryValue);
    if (stepActivation) {
      stepActivation.textContent = exampleSol(activationValue);
      const activationDetail = $("#example-activation-pct");
      if (activationDetail) {
        activationDetail.textContent = I18n.t("trader-example-profit", {
          value: examplePercent(activation),
        });
      }
    }
    if (stepPeak) {
      stepPeak.textContent = exampleSol(peakValue);
      const peakDetail = $("#example-peak-pct");
      if (peakDetail) {
        const gainFromEntry = (peakRatio - 1) * 100;
        peakDetail.textContent = I18n.t("trader-example-profit", {
          value: examplePercent(gainFromEntry),
        });
      }
    }
    if (stepExit) {
      stepExit.textContent = exampleSol(exitValue);
      const exitDetail = $("#example-exit-pct");
      if (exitDetail) {
        exitDetail.textContent = I18n.t("trader-trailing-final", {
          value: examplePercent(protectedProfit),
        });
      }
    }

    // Update summary
    const summaryProtected = $("#example-protected");
    const summaryAvoided = $("#example-avoided");
    if (summaryProtected) {
      summaryProtected.innerHTML = I18n.markup("trader-trailing-summary-protected", {
        value: formatPercentValue(protectedProfit, { decimals: 1, plus: "", trim: true }),
      });
    }
    if (summaryAvoided) {
      // The trail gives back exactly its distance from the peak.
      summaryAvoided.innerHTML = I18n.markup("trader-trailing-summary-avoided", {
        value: formatPercentValue(distance, { decimals: 1, plus: "", trim: true }),
      });
    }
  }

  /** The computed line under each DCA field, and the worked example. */
  function updateDcaExample() {
    const line = (id, text) => {
      const el = $(`#${id}-impact`);
      if (el) el.textContent = text;
    };
    const threshold = parseFloat($("#dca-threshold")?.value);
    const count = parseInt($("#dca-max-count")?.value, 10);
    const size = parseFloat($("#dca-size")?.value);
    const cooldown = parseInt($("#dca-cooldown")?.value, 10);
    const base = Number(tradeSize?.());

    line(
      "dca-threshold",
      I18n.t("trader-dca-threshold-impact", { value: examplePercent(Math.abs(threshold)) })
    );
    line(
      "dca-max-count",
      Number.isFinite(count) ? I18n.t("trader-dca-count-impact", { count }) : "—"
    );
    line(
      "dca-size",
      I18n.t("trader-dca-size-impact", {
        amount: exampleSol(base > 0 && size > 0 ? (base * size) / 100 : Number.NaN),
      })
    );
    line(
      "dca-cooldown",
      I18n.t("trader-dca-cooldown-impact", {
        span: formatTimeSpan(cooldown, { unit: "minute" }),
      })
    );

    const example = $("#dca-example");
    if (example) {
      example.textContent = I18n.t("trader-dca-example", {
        initial: exampleSol(base),
        add: exampleSol(base > 0 && size > 0 ? (base * size) / 100 : Number.NaN),
        threshold: examplePercent(Math.abs(threshold)),
        count: Number.isFinite(count) ? count : 0,
      });
    }
  }

  // Return public API
  return {
    updateDcaExample,
    updateRoiExample,
    updateStopLossExample,
    updateTrailingStopExample,
    updateTimeConversionHint,
    updateTimeLossExample,
    convertTimeToReadable,
  };
}
