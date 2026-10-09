// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// The four exit rules as the editor and the Rules tab present them: field specs,
// the policy a task runs under after its overrides, presets, and validation.
import { duration, finite, settingPct, settingSignedPct } from "./format.js";

const status = {
  key: "enabled",
  bool: true,
  get label() {
    return I18n.t("copy-rule-status");
  },
  text: (on) => (on ? I18n.t("copy-rule-on") : I18n.t("copy-rule-off")),
};

export const RULES = [
  {
    group: "stop_loss",
    get title() {
      return I18n.t("copy-exit-stop-loss");
    },
    fields: [
      status,
      {
        key: "threshold_pct",
        get label() {
          return I18n.t("copy-rule-stop-loss-threshold");
        },
        unit: "%",
        text: (v) => settingSignedPct(-Number(v)),
      },
      {
        key: "min_hold_seconds",
        get label() {
          return I18n.t("copy-rule-stop-loss-min-hold");
        },
        get unit() {
          return I18n.t("copy-rule-unit-seconds");
        },
        integer: true,
        text: (v) => (Number(v) > 0 ? duration(v) : I18n.t("copy-rule-no-minimum")),
      },
      {
        key: "allow_partial",
        get label() {
          return I18n.t("copy-rule-partial-exits");
        },
        bool: true,
        text: (on) =>
          on ? I18n.t("copy-rule-partial-allowed") : I18n.t("copy-rule-partial-full-only"),
      },
      {
        key: "partial_exit_default_pct",
        get label() {
          return I18n.t("copy-rule-partial-size");
        },
        unit: "%",
        text: (v) => settingPct(v),
      },
    ],
  },
  {
    group: "trailing",
    get title() {
      return I18n.t("copy-exit-trailing-stop");
    },
    fields: [
      status,
      {
        key: "activation_pct",
        get label() {
          return I18n.t("copy-rule-trailing-activation");
        },
        unit: "%",
        text: (v) => settingSignedPct(v),
      },
      {
        key: "distance_pct",
        get label() {
          return I18n.t("copy-rule-trailing-distance");
        },
        unit: "%",
        text: (v) => settingPct(v),
      },
    ],
  },
  {
    group: "roi",
    get title() {
      return I18n.t("copy-exit-take-profit");
    },
    fields: [
      status,
      {
        key: "target_profit_pct",
        get label() {
          return I18n.t("copy-rule-take-profit-target");
        },
        unit: "%",
        text: (v) => settingSignedPct(v),
      },
    ],
  },
  {
    group: "time",
    get title() {
      return I18n.t("copy-exit-time-override");
    },
    fields: [
      status,
      {
        key: "duration_seconds",
        get label() {
          return I18n.t("copy-rule-time-duration");
        },
        get unit() {
          return I18n.t("copy-rule-unit-minutes");
        },
        scale: 60,
        text: (v) => duration(v),
      },
      {
        key: "loss_threshold_pct",
        get label() {
          return I18n.t("copy-rule-time-threshold");
        },
        unit: "%",
        text: (v) => settingSignedPct(v),
      },
    ],
  },
];

export function blankOverrides() {
  return Object.fromEntries(
    RULES.map((rule) => [
      rule.group,
      Object.fromEntries(rule.fields.map((field) => [field.key, null])),
    ])
  );
}

/** A task's overrides with every known field present (null = inherit). */
export function normalizeOverrides(overrides) {
  const result = blankOverrides();
  RULES.forEach((rule) =>
    rule.fields.forEach((field) => {
      const value = overrides?.[rule.group]?.[field.key];
      if (value !== undefined && value !== null) result[rule.group][field.key] = value;
    })
  );
  return result;
}

export function effectivePolicy(traderDefaults, overrides) {
  return Object.fromEntries(
    RULES.map((rule) => [
      rule.group,
      Object.fromEntries(
        rule.fields.map((field) => [
          field.key,
          overrides?.[rule.group]?.[field.key] ?? traderDefaults?.[rule.group]?.[field.key] ?? null,
        ])
      ),
    ])
  );
}

export function isOverridden(overrides, group, key) {
  const value = overrides?.[group]?.[key];
  return value !== null && value !== undefined;
}

export function fieldText(field, value) {
  return value === null || value === undefined ? "—" : field.text(value);
}

/** One line per rule: "Off", or its values joined. */
export function ruleSummary(rule, policy) {
  const values = policy?.[rule.group];
  if (!values?.enabled) return I18n.t("copy-rule-off");
  return rule.fields
    .filter((field) => field.key !== "enabled")
    .filter(
      (field) =>
        rule.group !== "stop_loss" ||
        field.key !== "partial_exit_default_pct" ||
        values.allow_partial
    )
    .map((field) => fieldText(field, values[field.key]))
    .join(" · ");
}

/**
 * The note on a rule that follows the Trader's switch. The backend merges overrides per field,
 * so a stored value under an inherited switch still applies: the note reads the same resolved
 * policy the exit warnings read, and says when the values are the task's own.
 */
export function inheritNote(rule, traderDefaults, overrides) {
  if (!traderDefaults) return I18n.t("copy-editor-rule-follows-plain");
  const summary = ruleSummary(rule, effectivePolicy(traderDefaults, overrides));
  const own = rule.fields.some(
    (field) => field.key !== "enabled" && isOverridden(overrides, rule.group, field.key)
  );
  return own
    ? I18n.t("copy-editor-rule-follows-own", { summary })
    : I18n.t("copy-editor-rule-follows", { summary });
}

function preset(groups) {
  const overrides = blankOverrides();
  Object.entries(groups).forEach(([group, values]) => Object.assign(overrides[group], values));
  return overrides;
}

export const PRESETS = [
  {
    id: "inherit",
    get label() {
      return I18n.t("copy-preset-inherit");
    },
    overrides: blankOverrides(),
  },
  {
    id: "conservative",
    get label() {
      return I18n.t("copy-preset-conservative");
    },
    overrides: preset({
      stop_loss: { enabled: true, threshold_pct: 15 },
      trailing: { enabled: true, activation_pct: 15, distance_pct: 8 },
      roi: { enabled: true, target_profit_pct: 40 },
      time: { enabled: true, duration_seconds: 1800, loss_threshold_pct: -5 },
    }),
  },
  {
    id: "balanced",
    get label() {
      return I18n.t("copy-preset-balanced");
    },
    overrides: preset({
      stop_loss: { enabled: true, threshold_pct: 25 },
      trailing: { enabled: true, activation_pct: 30, distance_pct: 12 },
      roi: { enabled: true, target_profit_pct: 100 },
      time: { enabled: true, duration_seconds: 3600, loss_threshold_pct: -10 },
    }),
  },
  {
    id: "aggressive",
    get label() {
      return I18n.t("copy-preset-aggressive");
    },
    overrides: preset({
      stop_loss: { enabled: true, threshold_pct: 40 },
      trailing: { enabled: true, activation_pct: 60, distance_pct: 20 },
      roi: { enabled: false },
      time: { enabled: false },
    }),
  },
];

export function matchPreset(overrides) {
  const current = JSON.stringify(normalizeOverrides(overrides));
  return PRESETS.find((item) => JSON.stringify(item.overrides) === current)?.id || "custom";
}

const inRange = (value, max, exclusive) =>
  value === null ||
  (Number.isFinite(value) && value > 0 && (exclusive ? value < max : value <= max));

/** The server's own override limits, checked before a save. */
/**
 * The first invalid exit override as `{ message, field }`, where `field` is the
 * `data-rule-field` name of the input that holds it, or null when every override is valid.
 */
export function validateOverrides(overrides) {
  const { stop_loss: stop, trailing, roi, time } = overrides;
  const problem = (message, field) => ({ message, field });
  if (!inRange(stop.threshold_pct, 100))
    return problem(I18n.t("copy-validate-stop-loss"), "stop_loss.threshold_pct");
  if (!inRange(stop.partial_exit_default_pct, 100, true))
    return problem(I18n.t("copy-validate-partial-size"), "stop_loss.partial_exit_default_pct");
  if (
    stop.min_hold_seconds !== null &&
    !(Number.isInteger(stop.min_hold_seconds) && stop.min_hold_seconds >= 0)
  ) {
    return problem(I18n.t("copy-validate-min-hold"), "stop_loss.min_hold_seconds");
  }
  if (!inRange(trailing.activation_pct, 100))
    return problem(I18n.t("copy-validate-trailing-activation"), "trailing.activation_pct");
  if (!inRange(trailing.distance_pct, 100))
    return problem(I18n.t("copy-validate-trailing-distance"), "trailing.distance_pct");
  if (roi.target_profit_pct !== null && !(roi.target_profit_pct > 0))
    return problem(I18n.t("copy-validate-take-profit"), "roi.target_profit_pct");
  if (time.duration_seconds !== null && !(time.duration_seconds > 0))
    return problem(I18n.t("copy-validate-time-duration"), "time.duration_seconds");
  if (
    time.loss_threshold_pct !== null &&
    !(Number.isFinite(time.loss_threshold_pct) && time.loss_threshold_pct <= 0)
  ) {
    return problem(I18n.t("copy-validate-time-threshold"), "time.loss_threshold_pct");
  }
  return null;
}

/**
 * What a reader must know about the rules in effect. `slippagePct` and `feePct`
 * are the task's own selling costs: an exit rule measures from the entry price,
 * which already carries the buy's costs, so a sell still pays both again.
 */
export function exitWarnings(policy, exitMode, { slippagePct = null, feePct = null } = {}) {
  const warnings = [];
  if (exitMode === "mirror") {
    warnings.push(I18n.t("copy-warning-mirror"));
    return warnings;
  }
  const anyRule = ["stop_loss", "trailing", "roi", "time"].some(
    (group) => policy?.[group]?.enabled
  );
  if (exitMode === "buy_only" && !anyRule) {
    warnings.push(I18n.t("copy-warning-no-rules"));
  } else if (!policy?.stop_loss?.enabled) {
    warnings.push(I18n.t("copy-warning-no-stop-loss"));
  }
  const stop = policy?.stop_loss;
  if (stop?.enabled && Number(stop.min_hold_seconds) > 0) {
    warnings.push(
      I18n.t("copy-warning-stop-delay", {
        hold: duration(stop.min_hold_seconds),
        threshold: settingSignedPct(-Number(stop.threshold_pct)),
      })
    );
  }
  const roi = policy?.roi;
  const slippage = finite(slippagePct);
  const fee = finite(feePct);
  if (roi?.enabled && slippage !== null && fee !== null) {
    const sellCost = slippage + fee;
    if (Number(roi.target_profit_pct) <= sellCost) {
      warnings.push(
        I18n.t("copy-warning-take-profit-cost", {
          target: settingSignedPct(roi.target_profit_pct),
          slippage: settingPct(slippage),
          fee: settingPct(fee),
        })
      );
    }
  }
  const trailing = policy?.trailing;
  if (trailing?.enabled && Number(trailing.distance_pct) >= Number(trailing.activation_pct)) {
    warnings.push(I18n.t("copy-warning-trailing-distance"));
  }
  return warnings;
}
