// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
//! The ladder editor of the Auto Trader tabs: a list of steps, each a level and an amount, edited as one config field, configured as the take-profit ladder or the DCA ladder.

import { formatNumber, formatSol } from "../../core/format.js";

/** The field type of a ladder in the Auto Trader card specs. */
export const LEVELS = "levels";

/**
 * Bounds, units and step limit of each ladder element, from the config metadata of
 * its two keys, and for the DCA ladder the minimum trade size. A ladder is only
 * editable once these are known.
 * @type {WeakMap<HTMLElement, {first: object, second: object, maxItems: number, firstUnit?: string, secondUnit?: string, minimum?: number}>}
 */
const SETTINGS = new WeakMap();

/**
 * What the page knows beyond the metadata: the first buy each DCA step is a multiple
 * of, in the native coin.
 * @type {WeakMap<HTMLElement, {firstBuy?: number}>}
 */
const CONTEXT = new WeakMap();

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

/** Whether `value` is missing or outside the metadata `limits`. */
const outside = (value, limits) =>
  !Number.isFinite(value) || value < limits.min || value > limits.max;

/**
 * `value` rounded down to a multiple of the metadata `step`, without the floating-point
 * tail a decimal step leaves (0.1 * 3 is 0.30000000000000004).
 */
function floorToStep(value, step) {
  if (!step) return value;
  return Number((Math.floor(value / step + 1e-9) * step).toPrecision(12));
}

/** The basis the DCA ladder `el` is measured from requires strictly deeper steps. */
function dcaStrictOrder(el) {
  const basis = el.dataset.basis ? document.getElementById(el.dataset.basis) : null;
  return basis?.value === "first_entry";
}

/**
 * Appends a take-profit step to `levels` and `amounts`. The new level is twice the last
 * one, capped at the profit maximum; the add control is disabled while the last level is
 * at that maximum, so the levels keep rising strictly. When the steps already sell the
 * whole position, the new step takes half of the last step's share, rounded down to the
 * share step and never below the smallest share, and the last keeps the rest, so the
 * ladder stays valid. When the last step could not keep the smallest share itself, or a
 * runner is left, the new step sells the smallest share allowed.
 */
function appendTakeProfitStep(levels, amounts, { first, second }) {
  const lastLevel = levels[levels.length - 1];
  levels.push(Number.isFinite(lastLevel) ? Math.min(lastLevel * 2, first.max) : first.min);

  const last = amounts.length - 1;
  const wholePosition = Math.abs(shareTotal(amounts) - second.max) <= SHARE_SUM_TOLERANCE_PCT;
  if (wholePosition && last >= 0) {
    const share = Math.max(floorToStep(amounts[last] / 2, second.step), second.min);
    const kept = Number((amounts[last] - share).toPrecision(12));
    if (kept >= second.min) {
      amounts[last] = kept;
      amounts.push(share);
      return;
    }
  }
  amounts.push(second.min);
}

/**
 * Appends a DCA step to `levels` and `amounts`: twice the last drop, capped at the
 * deepest drop, buying the same multiple as the last step. A first step takes the
 * shallowest drop and the smallest multiple.
 */
function appendDcaStep(levels, amounts, { first, second }) {
  const lastLevel = levels[levels.length - 1];
  levels.push(Number.isFinite(lastLevel) ? Math.max(lastLevel * 2, first.min) : first.max);
  const lastAmount = amounts[amounts.length - 1];
  amounts.push(Number.isFinite(lastAmount) ? lastAmount : second.min);
}

/**
 * The two ladders this editor draws, selected by the element's `data-ladder`: the
 * classes, texts and rules of their two columns. A `magnitude` column stores a
 * negative percent and shows its magnitude after a fixed "−".
 */
const LADDERS = {
  take_profit: {
    first: {
      className: "level-list-profit",
      caption: "trader-take-profit-profit-caption",
      label: "trader-take-profit-profit-label",
    },
    second: {
      className: "level-list-sell",
      caption: "trader-take-profit-sell-caption",
      label: "trader-take-profit-sell-label",
    },
    step: "trader-take-profit-step",
    add: "trader-take-profit-add-step",
    remove: "trader-take-profit-remove-step",
    amountColumn: false,
    append: appendTakeProfitStep,
    // No step fits above a last level at the profit maximum: levels rise strictly.
    full: (el, levels, { first }) => levels[levels.length - 1] >= first.max,
    /**
     * Mirrors the config's own validation: every level and share inside its bounds,
     * levels strictly rising, and the shares together within the whole position.
     */
    error(el, levels, amounts, { first, second }) {
      if (levels.some((value) => outside(value, first))) {
        return I18n.t("trader-take-profit-error-profit-range", {
          min: bound(first.min),
          max: bound(first.max),
        });
      }
      if (amounts.some((value) => outside(value, second))) {
        return I18n.t("trader-take-profit-error-sell-range", {
          min: bound(second.min),
          max: bound(second.max),
        });
      }
      if (levels.some((value, index) => index > 0 && value <= levels[index - 1])) {
        return I18n.t("trader-take-profit-error-order");
      }
      if (shareTotal(amounts) > second.max + SHARE_SUM_TOLERANCE_PCT) {
        return I18n.t("trader-take-profit-error-total", { max: bound(second.max) });
      }
      return null;
    },
  },
  dca: {
    first: {
      className: "level-list-drop",
      caption: "trader-dca-drop-caption",
      label: "trader-dca-drop-label",
      magnitude: true,
    },
    second: {
      className: "level-list-size",
      caption: "trader-dca-size-caption",
      label: "trader-dca-size-label",
    },
    step: "trader-dca-step",
    add: "trader-dca-add-step",
    remove: "trader-dca-remove-step",
    amountColumn: true,
    append: appendDcaStep,
    // With strictly deeper steps, none fits below a last drop at the deepest level.
    full: (el, levels, { first }) =>
      dcaStrictOrder(el) && levels[levels.length - 1] <= first.min,
    /**
     * Mirrors the config's own validation: every drop and multiple inside its bounds,
     * and with the first entry as the basis, every step deeper than the one before.
     */
    error(el, levels, amounts, { first, second }) {
      if (levels.some((value) => outside(value, first))) {
        return I18n.t("trader-dca-error-drop-range", {
          min: bound(-first.max),
          max: bound(-first.min),
        });
      }
      if (amounts.some((value) => outside(value, second))) {
        return I18n.t("trader-dca-error-size-range", {
          min: bound(second.min),
          max: bound(second.max),
        });
      }
      if (
        dcaStrictOrder(el) &&
        levels.some((value, index) => index > 0 && value >= levels[index - 1])
      ) {
        return I18n.t("trader-dca-error-order");
      }
      return null;
    },
  },
};

/** The ladder kind of `el`, the take-profit ladder unless it names another. */
const ladderOf = (el) => LADDERS[el.dataset.ladder] ?? LADDERS.take_profit;

/** Every step row of the ladder `el`, in order. */
function rows(el) {
  return [...el.querySelectorAll(":scope > .level-list-rows > .level-list-row")];
}

/** The stored value of a column input: a magnitude column stores the negated value. */
function storedValue(input, column) {
  const value = parseFloat(input.value);
  return column.magnitude && value !== 0 ? -value : value;
}

/**
 * The steps the ladder `el` holds, as stored in the config: levels and amounts, in
 * step order.
 */
export function readLevels(el) {
  const ladder = ladderOf(el);
  const steps = rows(el).map((row) => ({
    level: storedValue(row.querySelector(`.${ladder.first.className}`), ladder.first),
    amount: storedValue(row.querySelector(`.${ladder.second.className}`), ladder.second),
  }));
  return {
    levels: steps.map((step) => step.level),
    amounts: steps.map((step) => step.amount),
  };
}

/** The ladder's values as one comparable string, for the card's dirty check. */
export function levelSnapshot(el) {
  return JSON.stringify(readLevels(el));
}

/** Restores the ladder to a snapshot taken by `levelSnapshot`. */
export function restoreLevels(el, snapshot) {
  const { levels, amounts } = JSON.parse(snapshot);
  renderLevels(el, levels, amounts);
}

/** The config values a card saves for the ladder field `field`. */
export function levelValues(el, field) {
  const { levels, amounts } = readLevels(el);
  return { [field.key]: levels, [field.pairKey]: amounts };
}

/** Why the ladder `el` cannot be saved, worded for the user, or `null` when it can. */
export function levelError(el) {
  const settings = SETTINGS.get(el);
  if (!settings) return null;
  const { levels, amounts } = readLevels(el);
  return ladderOf(el).error(el, levels, amounts, settings);
}

/** The share of the position no take-profit step sells, from a valid ladder's shares. */
export function runnerShare(el) {
  const settings = SETTINGS.get(el);
  if (!settings) return null;
  return settings.second.max - shareTotal(readLevels(el).amounts);
}

/** Applies a number field's metadata bounds to `input`, as the column shows them. */
function applyBounds(input, limits, column) {
  if (!limits) return;
  const [min, max] = column.magnitude ? [-limits.max, -limits.min] : [limits.min, limits.max];
  if (Number.isFinite(min)) input.min = String(min);
  if (Number.isFinite(max)) input.max = String(max);
  if (limits.step != null) input.step = String(limits.step);
}

/** One number input of a step row, with its sign before it and its unit after it. */
function numberInput(column, value, label, limits, unit) {
  const field = document.createElement("div");
  field.className = "input-group-enhanced level-list-field";
  if (column.magnitude) {
    const prefix = document.createElement("span");
    prefix.className = "input-prefix";
    prefix.setAttribute("aria-hidden", "true");
    prefix.textContent = "−";
    field.append(prefix);
  }
  const input = document.createElement("input");
  input.type = "number";
  input.className = `input-large ${column.className}`;
  const shown = column.magnitude && value !== 0 ? -value : value;
  input.value = Number.isFinite(shown) ? String(shown) : "";
  input.setAttribute("aria-label", label);
  applyBounds(input, limits, column);
  field.append(input);
  if (unit) {
    const suffix = document.createElement("span");
    suffix.className = "input-unit";
    suffix.textContent = unit;
    field.append(suffix);
  }
  return field;
}

/** A step row: its name, its level, its amount, what the DCA step buys, and its remove control. */
function stepRow(el, step, level, amount) {
  const ladder = ladderOf(el);
  const settings = SETTINGS.get(el);
  const row = document.createElement("div");
  row.className = "level-list-row";

  const name = document.createElement("span");
  name.className = "level-list-step";
  name.textContent = I18n.t(ladder.step, { step });

  const remove = document.createElement("button");
  remove.type = "button";
  remove.className = "btn btn-sm btn-ghost level-list-remove";
  remove.setAttribute("aria-label", I18n.t(ladder.remove, { step }));
  const icon = document.createElement("i");
  icon.className = "icon-x";
  remove.append(icon);
  remove.addEventListener("click", () => {
    const { levels, amounts } = readLevels(el);
    levels.splice(step - 1, 1);
    amounts.splice(step - 1, 1);
    renderLevels(el, levels, amounts);
    el.dispatchEvent(new Event("input", { bubbles: true }));
  });

  row.append(
    name,
    numberInput(
      ladder.first,
      level,
      I18n.t(ladder.first.label, { step }),
      settings?.first,
      settings?.firstUnit
    ),
    numberInput(
      ladder.second,
      amount,
      I18n.t(ladder.second.label, { step }),
      settings?.second,
      settings?.secondUnit
    )
  );
  if (ladder.amountColumn) {
    const bought = document.createElement("span");
    bought.className = "level-list-amount";
    row.append(bought);
  }
  row.append(remove);
  return row;
}

/**
 * Writes what each DCA step buys beside it, the step's multiple of the first buy, and
 * flags a step that buys less than the minimum trade.
 */
function refreshAmounts(el) {
  const settings = SETTINGS.get(el);
  const firstBuy = CONTEXT.get(el)?.firstBuy;
  const { amounts } = readLevels(el);
  rows(el).forEach((row, index) => {
    const cell = row.querySelector(".level-list-amount");
    if (!cell) return;
    const bought =
      Number.isFinite(firstBuy) && firstBuy > 0 ? firstBuy * amounts[index] : Number.NaN;
    cell.textContent = I18n.t("trader-dca-step-amount", {
      amount: formatSol(bought, { decimals: 6, fallback: "—", subscript: true }),
    });
    const below =
      Number.isFinite(bought) && Number.isFinite(settings?.minimum) && bought < settings.minimum;
    cell.classList.toggle("is-warning", below);
    if (below) {
      cell.title = I18n.t("trader-dca-step-below-minimum", {
        minimum: formatSol(settings.minimum, { decimals: 6, trim: true }),
      });
    } else {
      cell.removeAttribute("title");
    }
  });
}

/** Shows the ladder's validation message and enables the controls its steps allow. */
function refresh(el) {
  const ladder = ladderOf(el);
  const settings = SETTINGS.get(el);
  const { levels } = readLevels(el);
  const count = levels.length;
  const add = el.querySelector(":scope > .level-list-add");
  if (add) {
    add.disabled = !settings || count >= settings.maxItems || ladder.full(el, levels, settings);
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
  if (ladder.amountColumn) refreshAmounts(el);
}

/** Draws the ladder `el` with the steps `levels` and `amounts`, paired by index. */
export function renderLevels(el, levels, amounts) {
  const ladder = ladderOf(el);
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
      column(I18n.t(ladder.first.caption)),
      column(I18n.t(ladder.second.caption)),
      ...(ladder.amountColumn ? [column("")] : []),
      column("")
    );

    const add = document.createElement("button");
    add.type = "button";
    add.className = "btn btn-sm btn-ghost level-list-add";
    const icon = document.createElement("i");
    icon.className = "icon-plus";
    const label = document.createElement("span");
    label.textContent = I18n.t(ladder.add);
    add.append(icon, label);
    add.addEventListener("click", () => {
      const current = readLevels(el);
      ladder.append(current.levels, current.amounts, SETTINGS.get(el));
      renderLevels(el, current.levels, current.amounts);
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
    ...levels.map((level, index) => stepRow(el, index + 1, level, amounts[index]))
  );
  refresh(el);
}

/**
 * Gives the ladder `el` its metadata: the bounds and unit of a level and of an amount,
 * the most steps a ladder holds, and for the DCA ladder the minimum trade size.
 */
export function configureLevels(el, { first, second, maxItems, firstUnit, secondUnit, minimum }) {
  if (!first || !second || !Number.isInteger(maxItems)) return;
  SETTINGS.set(el, { first, second, maxItems, firstUnit, secondUnit, minimum });
  const { levels, amounts } = readLevels(el);
  renderLevels(el, levels, amounts);
}

/**
 * Gives the ladder `el` what the page knows beyond the metadata (the first buy each DCA
 * step multiplies) and redraws its validation and amounts.
 */
export function setLevelContext(el, context) {
  CONTEXT.set(el, context);
  refresh(el);
}
