// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
//! The take-profit ladder editor: a list of steps, each a profit level and the share of the position it sells, edited as one config field.

import { formatNumber } from "../../core/format.js";

/** The field type of a ladder in the Auto Trader card specs. */
export const LEVELS = "levels";

/**
 * Bounds, units and step limit of each ladder element, from the config metadata of
 * its two keys. A ladder is only editable once these are known.
 * @type {WeakMap<HTMLElement, {profit: object, sell: object, maxItems: number, profitUnit?: string, sellUnit?: string}>}
 */
const SETTINGS = new WeakMap();

/** Every step row of the ladder `el`, in order. */
function rows(el) {
  return [...el.querySelectorAll(":scope > .level-list-rows > .level-list-row")];
}

/** The steps the ladder `el` holds, as typed: profit levels and shares, in step order. */
export function readLevels(el) {
  const steps = rows(el).map((row) => ({
    profit: parseFloat(row.querySelector(".level-list-profit").value),
    sell: parseFloat(row.querySelector(".level-list-sell").value),
  }));
  return {
    profits: steps.map((step) => step.profit),
    sells: steps.map((step) => step.sell),
  };
}

/** The ladder's values as one comparable string, for the card's dirty check. */
export function levelSnapshot(el) {
  return JSON.stringify(readLevels(el));
}

/** Restores the ladder to a snapshot taken by `levelSnapshot`. */
export function restoreLevels(el, snapshot) {
  const { profits, sells } = JSON.parse(snapshot);
  renderLevels(el, profits, sells);
}

/** The config values a card saves for the ladder field `field`. */
export function levelValues(el, field) {
  const { profits, sells } = readLevels(el);
  return { [field.key]: profits, [field.pairKey]: sells };
}

/** A metadata bound written into a message, in the locale's digits. */
const bound = (value) => formatNumber(value, { decimals: 0, maxDecimals: 2 });

/**
 * Decimal shares that add up to exactly the whole position can sum past it in floating
 * point (5.2 + 78.9 + 15.9 is 100.00000000000001), so their total is checked against the
 * whole position within this tolerance, the one the config validation applies.
 */
const SHARE_SUM_TOLERANCE_PCT = 1e-9;

/** The total share the shares `sells` sell together. */
const shareTotal = (sells) => sells.reduce((sum, value) => sum + value, 0);

/**
 * Why the ladder `el` cannot be saved, worded for the user, or `null` when it can.
 * Mirrors the config's own validation: every level and share inside its bounds,
 * levels strictly rising, and the shares together within the whole position.
 */
export function levelError(el) {
  const settings = SETTINGS.get(el);
  if (!settings) return null;
  const { profits, sells } = readLevels(el);
  const { profit, sell } = settings;
  const outside = (value, limits) =>
    !Number.isFinite(value) || value < limits.min || value > limits.max;
  if (profits.some((value) => outside(value, profit))) {
    return I18n.t("trader-take-profit-error-profit-range", {
      min: bound(profit.min),
      max: bound(profit.max),
    });
  }
  if (sells.some((value) => outside(value, sell))) {
    return I18n.t("trader-take-profit-error-sell-range", {
      min: bound(sell.min),
      max: bound(sell.max),
    });
  }
  if (profits.some((value, index) => index > 0 && value <= profits[index - 1])) {
    return I18n.t("trader-take-profit-error-order");
  }
  const total = shareTotal(sells);
  if (total > sell.max + SHARE_SUM_TOLERANCE_PCT) {
    return I18n.t("trader-take-profit-error-total", { max: bound(sell.max) });
  }
  return null;
}

/** The share of the position no step sells, from a valid ladder's shares. */
export function runnerShare(el) {
  const settings = SETTINGS.get(el);
  if (!settings) return null;
  return settings.sell.max - shareTotal(readLevels(el).sells);
}

/**
 * `value` rounded down to a multiple of the metadata `step`, without the floating-point
 * tail a decimal step leaves (0.1 * 3 is 0.30000000000000004).
 */
function floorToStep(value, step) {
  if (!step) return value;
  return Number((Math.floor(value / step + 1e-9) * step).toPrecision(12));
}

/** Applies a number field's metadata bounds to `input`. */
function applyBounds(input, limits) {
  if (!limits) return;
  if (limits.min != null) input.min = String(limits.min);
  if (limits.max != null) input.max = String(limits.max);
  if (limits.step != null) input.step = String(limits.step);
}

/** One number input of a step row, with its unit after it. */
function numberInput(className, value, label, limits, unit) {
  const field = document.createElement("div");
  field.className = "input-group-enhanced level-list-field";
  const input = document.createElement("input");
  input.type = "number";
  input.className = `input-large ${className}`;
  input.value = Number.isFinite(value) ? String(value) : "";
  input.setAttribute("aria-label", label);
  applyBounds(input, limits);
  field.append(input);
  if (unit) {
    const suffix = document.createElement("span");
    suffix.className = "input-unit";
    suffix.textContent = unit;
    field.append(suffix);
  }
  return field;
}

/** A step row: its name, its profit level, its share and its remove control. */
function stepRow(el, step, profit, sell) {
  const settings = SETTINGS.get(el);
  const row = document.createElement("div");
  row.className = "level-list-row";

  const name = document.createElement("span");
  name.className = "level-list-step";
  name.textContent = I18n.t("trader-take-profit-step", { step });

  const remove = document.createElement("button");
  remove.type = "button";
  remove.className = "btn btn-sm btn-ghost level-list-remove";
  remove.setAttribute("aria-label", I18n.t("trader-take-profit-remove-step", { step }));
  const icon = document.createElement("i");
  icon.className = "icon-x";
  remove.append(icon);
  remove.addEventListener("click", () => {
    const { profits, sells } = readLevels(el);
    profits.splice(step - 1, 1);
    sells.splice(step - 1, 1);
    renderLevels(el, profits, sells);
    el.dispatchEvent(new Event("input", { bubbles: true }));
  });

  row.append(
    name,
    numberInput(
      "level-list-profit",
      profit,
      I18n.t("trader-take-profit-profit-label", { step }),
      settings?.profit,
      settings?.profitUnit
    ),
    numberInput(
      "level-list-sell",
      sell,
      I18n.t("trader-take-profit-sell-label", { step }),
      settings?.sell,
      settings?.sellUnit
    ),
    remove
  );
  return row;
}

/** Shows the ladder's validation message and enables the controls its steps allow. */
function refresh(el) {
  const settings = SETTINGS.get(el);
  const { profits } = readLevels(el);
  const count = profits.length;
  const add = el.querySelector(":scope > .level-list-add");
  if (add) {
    // No step fits above a last level at the profit maximum: levels rise strictly.
    add.disabled =
      !settings || count >= settings.maxItems || profits[count - 1] >= settings.profit.max;
  }
  el.querySelectorAll(".level-list-remove").forEach((button) => {
    button.disabled = count <= 1;
  });
  const message = el.querySelector(":scope > .level-list-error");
  if (message) {
    const error = levelError(el);
    message.textContent = error ?? "";
    message.hidden = error === null;
  }
}

/**
 * Appends a step to `profits` and `sells` (as read from a ladder with the metadata
 * `settings`). The new level is twice the last one, capped at the profit maximum; the
 * add control is disabled while the last level is at that maximum, so the levels keep
 * rising strictly. When the steps already sell the whole position, the new step takes
 * half of the last step's share, rounded down to the share step and never below the
 * smallest share, and the last keeps the rest, so the ladder stays valid. When the last
 * step could not keep the smallest share itself, or a runner is left, the new step sells
 * the smallest share allowed.
 */
function appendStep(profits, sells, { profit, sell }) {
  const lastLevel = profits[profits.length - 1];
  profits.push(Number.isFinite(lastLevel) ? Math.min(lastLevel * 2, profit.max) : profit.min);

  const last = sells.length - 1;
  const wholePosition = Math.abs(shareTotal(sells) - sell.max) <= SHARE_SUM_TOLERANCE_PCT;
  if (wholePosition && last >= 0) {
    const share = Math.max(floorToStep(sells[last] / 2, sell.step), sell.min);
    const kept = Number((sells[last] - share).toPrecision(12));
    if (kept >= sell.min) {
      sells[last] = kept;
      sells.push(share);
      return;
    }
  }
  sells.push(sell.min);
}

/** Draws the ladder `el` with the steps `profits` and `sells`, paired by index. */
export function renderLevels(el, profits, sells) {
  let list = el.querySelector(":scope > .level-list-rows");
  let caption = list?.querySelector(":scope > .level-list-caption");
  if (!list) {
    list = document.createElement("div");
    list.className = "level-list-rows";

    // Column captions over the two inputs of every row. Each input carries its own
    // accessible name, so the captions are hidden from assistive technology.
    caption = document.createElement("div");
    caption.className = "level-list-caption";
    caption.setAttribute("aria-hidden", "true");
    const column = (text) => {
      const cell = document.createElement("span");
      cell.textContent = text;
      return cell;
    };
    caption.append(
      column(""),
      column(I18n.t("trader-take-profit-profit-caption")),
      column(I18n.t("trader-take-profit-sell-caption")),
      column("")
    );

    const add = document.createElement("button");
    add.type = "button";
    add.className = "btn btn-sm btn-ghost level-list-add";
    const icon = document.createElement("i");
    icon.className = "icon-plus";
    const label = document.createElement("span");
    label.textContent = I18n.t("trader-take-profit-add-step");
    add.append(icon, label);
    add.addEventListener("click", () => {
      const current = readLevels(el);
      // The new step's level and share follow `appendStep`: twice the last level, and
      // half of the last share when the steps already sell the whole position.
      appendStep(current.profits, current.sells, SETTINGS.get(el));
      renderLevels(el, current.profits, current.sells);
      el.dispatchEvent(new Event("input", { bubbles: true }));
    });

    const message = document.createElement("p");
    message.className = "level-list-error";
    message.setAttribute("role", "alert");
    message.hidden = true;

    el.replaceChildren(list, add, message);
    el.addEventListener("input", () => refresh(el));
  }
  list.replaceChildren(
    caption,
    ...profits.map((profit, index) => stepRow(el, index + 1, profit, sells[index]))
  );
  refresh(el);
}

/**
 * Gives the ladder `el` its metadata: the bounds and unit of a profit level and of a
 * share, and the most steps a ladder holds.
 */
export function configureLevels(el, { profit, sell, maxItems, profitUnit, sellUnit }) {
  if (!profit || !sell || !Number.isInteger(maxItems)) return;
  SETTINGS.set(el, { profit, sell, maxItems, profitUnit, sellUnit });
  const { profits, sells } = readLevels(el);
  renderLevels(el, profits, sells);
}
