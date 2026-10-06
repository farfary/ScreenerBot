// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Display text of strategy conditions, read from the localization catalog.
 *
 * `/api/strategies/conditions/schemas` carries structure only. Each condition,
 * category, parameter and enum option holds the id of its message in `key` (or
 * `category_key`); the description is the message's `.description` attribute.
 *
 * Every id read here lives in the `strategies-condition-` namespace, which the
 * Rust test `strategies_catalog_covers_conditions` keeps complete.
 */

import { formatBooleanFlag, formatFixed, withSolUnit } from "../../core/format.js";

// Ids are the `unit` values of a numeric parameter schema; summaries write them as counted amounts.

// The subset of units that is also written beside the input.
const PARAM_UNIT_INPUT_LABELS = Object.freeze({
  hours: "strategies-unit-hours",
  multiplier: "strategies-unit-multiplier",
});

/** Name of a condition, or its type id when the schema is unknown. */
export function conditionName(schema, type) {
  return schema?.key ? I18n.t(schema.key) : String(type); // l10n-dynamic: strategies-condition-
}

/** Description of a condition, or an empty string. */
export function conditionDescription(schema) {
  return schema?.key ? (I18n.attr(schema.key, "description") ?? "") : ""; // l10n-dynamic: strategies-condition-
}

/** Display name of a condition's category, or an empty string. */
export function categoryLabel(schema) {
  return schema?.category_key ? I18n.t(schema.category_key) : ""; // l10n-dynamic: strategies-condition-
}

/** Name of a parameter, or its id when the schema carries none. */
export function paramLabel(spec, key) {
  return spec?.key ? I18n.t(spec.key) : String(key); // l10n-dynamic: strategies-condition-
}

/** Description of a parameter, or an empty string. */
export function paramDescription(spec) {
  return spec?.key ? (I18n.attr(spec.key, "description") ?? "") : ""; // l10n-dynamic: strategies-condition-
}

/** Display label of an enum option (`{ value, key }`, or a bare value). */
export function optionLabel(option) {
  if (option && typeof option === "object") {
    return option.key ? I18n.t(option.key) : String(option.value); // l10n-dynamic: strategies-condition-
  }
  return String(option);
}

/** Value of an enum option. */
export function optionValue(option) {
  return option && typeof option === "object" ? option.value : option;
}

/** Text written beside a numeric input, or `null` when the parameter has none. */
export function inputUnitText(spec) {
  if (spec.type === "percent") return I18n.t("strategies-unit-percent");
  if (spec.type === "sol") return I18n.t("strategies-unit-native");
  if (Object.hasOwn(PARAM_UNIT_INPUT_LABELS, spec.unit)) {
    return I18n.label(PARAM_UNIT_INPUT_LABELS, spec.unit);
  }
  return null;
}

function trimmed(value) {
  return formatFixed(value, { decimals: 4, trim: true });
}

/** A parameter value as the card summary shows it: enum label, unit and count included. */
export function formatParamValue(value, spec) {
  if (value === undefined || value === null) return "—";

  if (spec.type === "enum" && spec.options) {
    const option = spec.options.find((opt) => optionValue(opt) === value);
    return option ? optionLabel(option) : String(value);
  }
  if (spec.type === "boolean") return formatBooleanFlag(value);

  if (typeof value === "number") {
    if (spec.type === "percent") return I18n.t("strategies-value-percent", { amount: trimmed(value) });
    if (spec.type === "sol") return withSolUnit(trimmed(value));
    const counted = { count: value, amount: trimmed(value) };
    switch (spec.unit) {
      case "hours":
        return I18n.t("strategies-value-hours", counted);
      case "candles":
        return I18n.t("strategies-value-candles", counted);
      case "multiplier":
        return I18n.t("strategies-value-multiplier", counted);
      default:
        break;
    }
    return formatFixed(value, { decimals: Number.isInteger(value) ? 0 : 2 });
  }
  return String(value);
}
