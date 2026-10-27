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
  const total = sells.reduce((sum, value) => sum + value, 0);
  if (total > sell.max + SHARE_SUM_TOLERANCE_PCT) {
    return I18n.t("trader-take-profit-error-total", { max: bound(sell.max) });
  }
  return null;
}

/** The share of the position no step sells, from a valid ladder's shares. */
export function runnerShare(el) {
  const settings = SETTINGS.get(el);
  if (!settings) return null;
  const total = readLevels(el).sells.reduce((sum, value) => sum + value, 0);
  return settings.sell.max - total;
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

/** Shows the ladder's validation message and enables the controls its step count allows. */
function refresh(el) {
  const settings = SETTINGS.get(el);
  const count = rows(el).length;
  const add = el.querySelector(":scope > .level-list-add");
  if (add) add.disabled = !settings || count >= settings.maxItems;
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

/** Draws the ladder `el` with the steps `profits` and `sells`, paired by index. */
export function renderLevels(el, profits, sells) {
  let list = el.querySelector(":scope > .level-list-rows");
  if (!list) {
    list = document.createElement("div");
    list.className = "level-list-rows";

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
      const last = current.profits[current.profits.length - 1];
      const settings = SETTINGS.get(el);
      // A new step starts one metadata step above the last level and sells the
      // smallest share allowed, so it is valid wherever the ladder leaves room.
      current.profits.push(Number.isFinite(last) ? last + settings.profit.step : settings.profit.min);
      current.sells.push(settings.sell.min);
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
