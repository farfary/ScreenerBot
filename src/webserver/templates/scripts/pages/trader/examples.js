// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Trader example panels - live calculation previews for stop loss, trailing stop, break-even, take-profit and time exit settings.

import {
  formatFixed,
  formatPercentValue,
  formatSol,
  formatTimeSpan,
  formatUptime,
} from "../../core/format.js";
import { levelError, readLevels, runnerShare, setLevelContext } from "./level_list.js";

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
 * @param {() => (Object|null|undefined)} deps.traderConfig - The loaded `trader` config
 *   section, or nothing before the config has loaded
 * @param {() => (boolean|null)} deps.exitStrategiesOn - Whether an exit strategy is enabled,
 *   or `null` before the strategy list has loaded
 * @returns {Object} Example updater functions
 */
export function createExampleUpdaters({
  $,
  Utils: _Utils,
  tradeSize,
  traderConfig,
  exitStrategiesOn,
}) {
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
   * One step of a worked example: its bare glyph, title, figure and detail. `valueId`
   * names the figure for the page's own checks.
   */
  function timelineStep({ tone, icon, title, value, valueId, detail }) {
    const step = document.createElement("div");
    step.className = "timeline-step";
    const glyph = document.createElement("div");
    glyph.className = `step-icon step-icon-${tone}`;
    const glyphIcon = document.createElement("i");
    glyphIcon.className = icon;
    glyph.append(glyphIcon);
    const content = document.createElement("div");
    content.className = "step-content";
    const titleEl = document.createElement("div");
    titleEl.className = "step-title";
    titleEl.textContent = title;
    const valueEl = document.createElement("div");
    valueEl.className = "step-value";
    valueEl.id = valueId;
    valueEl.textContent = value;
    const detailEl = document.createElement("div");
    detailEl.className = "step-detail";
    detailEl.textContent = detail;
    content.append(titleEl, valueEl, detailEl);
    step.append(glyph, content);
    return step;
  }

  function timelineArrow() {
    const arrow = document.createElement("div");
    arrow.className = "timeline-arrow";
    const icon = document.createElement("i");
    icon.className = "icon-chevron-right";
    arrow.append(icon);
    return arrow;
  }

  /**
   * The take-profit ladder the example and the runner line describe: the steps as
   * edited while they are valid, otherwise the loaded config.
   */
  function takeProfitLadder(levels) {
    if (levels && levelError(levels) === null) {
      const edited = readLevels(levels);
      if (edited.levels.length > 0) return { profits: edited.levels, sells: edited.amounts };
    }
    const trader = traderConfig?.();
    if (!trader) return null;
    return { profits: trader.take_profit_levels_pct, sells: trader.take_profit_sell_pct };
  }

  /**
   * Update the take-profit ladder's runner line and its worked example: one buy at
   * the configured trade size, each step's position value and share, and the part
   * no step sells.
   */
  function updateTakeProfitExample() {
    const levels = $("#take-profit-levels");
    const runnerLine = $("#take-profit-runner");
    const runnerWarning = $("#take-profit-runner-warning");
    const timeline = $("#take-profit-example");
    const summary = $("#take-profit-example-summary");
    const ladder = takeProfitLadder(levels);
    const runner = levels && levelError(levels) === null ? runnerShare(levels) : null;

    if (runnerLine) {
      runnerLine.hidden = runner === null;
      if (runner !== null) {
        runnerLine.textContent =
          runner > 0
            ? I18n.t("trader-take-profit-runner", { runner: examplePercent(runner) })
            : I18n.t("trader-take-profit-runner-none");
      }
    }
    if (runnerWarning) {
      // Only a rule that is known to be off leaves the runner to the stop loss and the
      // user: a switch not drawn yet or a strategy list not loaded yet warns of nothing.
      const otherExitsOff =
        $("#trailing-enabled")?.checked === false &&
        $("#break-even-enabled")?.checked === false &&
        $("#time-override-enabled")?.checked === false &&
        $("#time-stagnation-enabled")?.checked === false &&
        exitStrategiesOn?.() === false;
      runnerWarning.hidden = !(runner !== null && runner > 0 && otherExitsOff);
    }
    if (!ladder || !timeline) return;

    const entry = Number(tradeSize?.());
    const steps = [
      timelineStep({
        tone: "entry",
        icon: "icon-arrow-right",
        title: I18n.t("trader-step-entry"),
        value: exampleSol(entry),
        valueId: "take-profit-example-entry",
        detail: I18n.t("trader-take-profit-initial-buy"),
      }),
    ];
    ladder.profits.forEach((profit, index) => {
      steps.push(
        timelineArrow(),
        timelineStep({
          tone: "peak",
          icon: "icon-trending-up",
          title: I18n.t("trader-take-profit-step", { step: index + 1 }),
          value: exampleSol(entry * (1 + profit / 100)),
          valueId: `take-profit-example-step-${index + 1}`,
          detail: I18n.t("trader-take-profit-example-step", {
            profit: examplePercent(profit),
            share: examplePercent(ladder.sells[index]),
          }),
        })
      );
    });
    if (runner !== null && runner > 0) {
      steps.push(
        timelineArrow(),
        timelineStep({
          tone: "activation",
          icon: "icon-flag",
          title: I18n.t("trader-take-profit-example-runner"),
          value: I18n.t("trader-value-percent", { value: examplePercent(runner) }),
          valueId: "take-profit-example-runner",
          detail: I18n.t("trader-take-profit-example-runner-detail"),
        })
      );
    }
    timeline.replaceChildren(...steps);

    if (summary) {
      const sold = ladder.sells.reduce((total, share) => total + share, 0);
      summary.innerHTML = I18n.markup("trader-take-profit-summary", {
        sold: examplePercent(sold),
        profit: examplePercent(ladder.profits[ladder.profits.length - 1]),
      });
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

  /** A signed percent in a computed line: one decimal at most, no trailing zero. */
  const signedPercent = (value) => formatPercentValue(value, { decimals: 1, trim: true });

  /**
   * The trailing stop's locked minimum: the profit its stop keeps on the first price that
   * arms it, `(1 + activation)(1 − distance) − 1`. At or below zero the stop sits at or
   * below the entry when it arms, so the line warns instead and points to the break-even
   * stop.
   */
  function updateTrailingLockedMinimum() {
    const lineEl = $("#trail-locked-minimum");
    if (!lineEl) return;
    const activation = parseFloat($("#trail-activation")?.value);
    const distance = parseFloat($("#trail-distance")?.value);
    const locked = ((1 + activation / 100) * (1 - distance / 100) - 1) * 100;
    if (!Number.isFinite(locked)) {
      lineEl.classList.remove("is-warning");
      lineEl.textContent = "—";
      return;
    }
    const warn = locked <= 0;
    lineEl.classList.toggle("is-warning", warn);
    lineEl.textContent = warn
      ? I18n.t("trader-trailing-locked-minimum-warning")
      : I18n.t("trader-trailing-locked-minimum", { value: signedPercent(locked) });
  }

  /** The break-even card's line: what arms the stop and where it sells. */
  function updateBreakEvenExample() {
    const lineEl = $("#break-even-summary");
    if (!lineEl) return;
    const trigger = parseFloat($("#break-even-trigger")?.value);
    const offset = parseFloat($("#break-even-offset")?.value);
    lineEl.textContent =
      Number.isFinite(trigger) && Number.isFinite(offset)
        ? I18n.t("trader-break-even-summary", {
            trigger: signedPercent(trigger),
            offset: signedPercent(offset),
          })
        : "—";
  }

  /** The flat-position card's line: how long a position must sit inside its band. */
  function updateStagnationExample() {
    const lineEl = $("#time-stagnation-summary");
    if (!lineEl) return;
    const minutes = parseFloat($("#time-stagnation-minutes")?.value);
    const band = parseFloat($("#time-stagnation-band")?.value);
    lineEl.textContent =
      Number.isFinite(minutes) && minutes > 0 && Number.isFinite(band)
        ? I18n.t("trader-time-flat-summary", {
            span: formatUptime(minutes * 60, { style: "trimmed" }),
            band: formatPercentValue(band, { decimals: 1, includeSign: false, trim: true }),
          })
        : "—";
  }

  /**
   * Update trailing stop visual example calculations
   */
  function updateTrailingStopExample() {
    updateTrailingLockedMinimum();
    const activationInput = $("#trail-activation");
    const distanceInput = $("#trail-distance");

    if (!activationInput || !distanceInput) return;

    const typedActivation = parseFloat(activationInput.value);
    const typedDistance = parseFloat(distanceInput.value);
    const activationImpact = $("#activation-impact");
    if (activationImpact) {
      activationImpact.textContent = Number.isFinite(typedActivation)
        ? I18n.t("trader-trailing-activation-impact", { value: examplePercent(typedActivation) })
        : "—";
    }
    const distanceImpact = $("#distance-impact");
    if (distanceImpact) {
      distanceImpact.textContent = Number.isFinite(typedDistance)
        ? I18n.t("trader-trailing-distance-impact", { value: examplePercent(typedDistance) })
        : "—";
    }

    const activation = typedActivation || 15;
    const distance = typedDistance || 5;

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

  /**
   * The DCA ladder's multiples as edited, or null while the edited ladder is invalid or
   * empty. Without a ladder element the loaded config's multiples apply.
   */
  function dcaMultiples(ladder) {
    if (!ladder) return traderConfig?.()?.dca_size_multipliers ?? null;
    if (levelError(ladder) !== null) return null;
    const edited = readLevels(ladder);
    return edited.amounts.length > 0 ? edited.amounts : null;
  }

  /**
   * The DCA tab's computed lines: what each ladder step buys from the configured trade
   * size, the two waits, and the most a position can hold once every step has bought.
   */
  function updateDcaExample() {
    const ladder = $("#dca-levels");
    const base = Number(tradeSize?.());
    const firstBuy = base > 0 ? base : Number.NaN;
    if (ladder) setLevelContext(ladder, { firstBuy });

    const line = (id, text) => {
      const el = $(`#${id}`);
      if (el) el.textContent = text;
    };
    const cooldown = parseInt($("#dca-cooldown")?.value, 10);
    const minHold = parseInt($("#dca-min-hold")?.value, 10);
    line(
      "dca-cooldown-impact",
      Number.isFinite(cooldown)
        ? I18n.t("trader-dca-cooldown-impact", {
            span: formatTimeSpan(cooldown, { unit: "minute" }),
          })
        : "—"
    );
    line(
      "dca-min-hold-impact",
      Number.isFinite(minHold)
        ? I18n.t("trader-dca-min-hold-impact", {
            minutes: minHold,
            span: formatTimeSpan(minHold, { unit: "minute" }),
          })
        : "—"
    );

    const multiples = dcaMultiples(ladder);
    const whole = multiples ? 1 + multiples.reduce((sum, value) => sum + value, 0) : Number.NaN;
    line(
      "dca-capacity",
      multiples
        ? I18n.t("trader-dca-capacity", {
            amount: exampleSol(firstBuy * whole),
            multiple: formatFixed(whole, { decimals: 2, trim: true }),
          })
        : "—"
    );
  }

  // Return public API
  return {
    updateBreakEvenExample,
    updateDcaExample,
    updateStagnationExample,
    updateTakeProfitExample,
    updateStopLossExample,
    updateTrailingStopExample,
    updateTimeConversionHint,
    updateTimeLossExample,
    convertTimeToReadable,
  };
}
