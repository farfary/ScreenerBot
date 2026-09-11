// The four exit rules as the editor and the Rules tab present them: field specs,
// the policy a task runs under after its overrides, presets, and validation.
import { duration, fixed, signedPct } from "./format.js";

const status = { key: "enabled", label: "Status", bool: true, text: (on) => (on ? "On" : "Off") };

export const RULES = [
  {
    group: "stop_loss",
    title: "Stop loss",
    fields: [
      status,
      {
        key: "threshold_pct",
        label: "Sells at a loss of",
        unit: "%",
        text: (v) => `−${fixed(v, 1)}%`,
      },
      {
        key: "min_hold_seconds",
        label: "Not before holding",
        unit: "seconds",
        integer: true,
        text: (v) => (Number(v) > 0 ? duration(v) : "No minimum"),
      },
      {
        key: "allow_partial",
        label: "Partial exits",
        bool: true,
        text: (on) => (on ? "Allowed" : "Full exit only"),
      },
      {
        key: "partial_exit_default_pct",
        label: "Partial exit size",
        unit: "%",
        text: (v) => `${fixed(v, 0)}%`,
      },
    ],
  },
  {
    group: "trailing",
    title: "Trailing stop",
    fields: [
      status,
      {
        key: "activation_pct",
        label: "Arms at a gain of",
        unit: "%",
        text: (v) => `+${fixed(v, 1)}%`,
      },
      {
        key: "distance_pct",
        label: "Sells below the peak by",
        unit: "%",
        text: (v) => `${fixed(v, 1)}%`,
      },
    ],
  },
  {
    group: "roi",
    title: "Take profit",
    fields: [
      status,
      {
        key: "target_profit_pct",
        label: "Sells at a gain of",
        unit: "%",
        text: (v) => `+${fixed(v, 1)}%`,
      },
    ],
  },
  {
    group: "time",
    title: "Time rule",
    fields: [
      status,
      {
        key: "duration_seconds",
        label: "Checks after holding",
        unit: "minutes",
        scale: 60,
        text: (v) => duration(v),
      },
      {
        key: "loss_threshold_pct",
        label: "Sells while P&L is at or below",
        unit: "%",
        text: (v) => signedPct(v, 1),
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
  if (!values?.enabled) return "Off";
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

function preset(groups) {
  const overrides = blankOverrides();
  Object.entries(groups).forEach(([group, values]) => Object.assign(overrides[group], values));
  return overrides;
}

export const PRESETS = [
  { id: "inherit", label: "Trader defaults", overrides: blankOverrides() },
  {
    id: "conservative",
    label: "Conservative",
    overrides: preset({
      stop_loss: { enabled: true, threshold_pct: 15 },
      trailing: { enabled: true, activation_pct: 15, distance_pct: 8 },
      roi: { enabled: true, target_profit_pct: 40 },
      time: { enabled: true, duration_seconds: 1800, loss_threshold_pct: -5 },
    }),
  },
  {
    id: "balanced",
    label: "Balanced",
    overrides: preset({
      stop_loss: { enabled: true, threshold_pct: 25 },
      trailing: { enabled: true, activation_pct: 30, distance_pct: 12 },
      roi: { enabled: true, target_profit_pct: 100 },
      time: { enabled: true, duration_seconds: 3600, loss_threshold_pct: -10 },
    }),
  },
  {
    id: "aggressive",
    label: "Aggressive",
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
export function validateOverrides(overrides) {
  const { stop_loss: stop, trailing, roi, time } = overrides;
  if (!inRange(stop.threshold_pct, 100)) return "Stop loss must be above 0% and at most 100%.";
  if (!inRange(stop.partial_exit_default_pct, 100, true))
    return "Partial exit size must be between 0% and 100%.";
  if (
    stop.min_hold_seconds !== null &&
    !(Number.isInteger(stop.min_hold_seconds) && stop.min_hold_seconds >= 0)
  ) {
    return "Minimum hold must be a whole number of seconds.";
  }
  if (!inRange(trailing.activation_pct, 100))
    return "Trailing activation must be above 0% and at most 100%.";
  if (!inRange(trailing.distance_pct, 100))
    return "Trailing distance must be above 0% and at most 100%.";
  if (roi.target_profit_pct !== null && !(roi.target_profit_pct > 0))
    return "Take profit must be above 0%.";
  if (time.duration_seconds !== null && !(time.duration_seconds > 0))
    return "The time rule needs a duration above zero.";
  if (
    time.loss_threshold_pct !== null &&
    !(Number.isFinite(time.loss_threshold_pct) && time.loss_threshold_pct <= 0)
  ) {
    return "The time rule threshold is a loss: use 0% or a negative number.";
  }
  return null;
}

/** What a reader must know about the rules in effect. */
export function exitWarnings(policy, exitMode) {
  const warnings = [];
  if (exitMode === "mirror") return warnings;
  const anyRule = ["stop_loss", "trailing", "roi", "time"].some(
    (group) => policy?.[group]?.enabled
  );
  if (exitMode === "buy_only" && !anyRule) {
    warnings.push("No exit rule is on and wallet sells are ignored: holdings are never sold.");
  } else if (!policy?.stop_loss?.enabled) {
    warnings.push(
      "No stop loss applies: a falling token is held until another rule or the wallet sells."
    );
  }
  const trailing = policy?.trailing;
  if (trailing?.enabled && Number(trailing.distance_pct) >= Number(trailing.activation_pct)) {
    warnings.push(
      "The trailing distance is at least its activation gain, so an armed trail can sell below entry."
    );
  }
  return warnings;
}
