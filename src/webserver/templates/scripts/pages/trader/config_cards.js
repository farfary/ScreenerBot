// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
//! Per-card Save / Reset controls for the Auto Trader configuration tabs.
//!
//! Each editable config card gets a Save and a Reset button injected into its
//! header (to the left of the feature on/off toggle). The buttons stay hidden
//! until the card has unsaved changes — i.e. a field differs from the value last
//! loaded from / saved to the backend. Save POSTs the card's fields to
//! /api/config (applied + hot-reloaded by core via the caller's saveConfig).
//! Reset discards the unsaved edits, restoring the last-saved values.

import { fieldHint, fieldLabel, fieldUnit } from "../config/field_text.js";
import {
  LEVELS,
  configureLevels,
  levelError,
  levelSnapshot,
  levelValues,
  restoreLevels,
} from "./level_list.js";

/**
 * A loss threshold the config stores as a negative percent (-100..0) and this page
 * shows as its magnitude after a fixed "−" (0..100). Stop Loss already stores a
 * positive magnitude and keeps the "float" type; the Configuration page shows every
 * value as stored.
 */
export const LOSS_MAGNITUDE = "loss-magnitude";

/** The value a field shows for a stored config value. */
export function displayValue(type, stored) {
  if (type === LOSS_MAGNITUDE) return stored === 0 ? 0 : -stored;
  return stored;
}

// Declarative spec: which DOM fields belong to which card, their config section
// and key, and how to coerce the input value into the saved payload. Keys/types
// mirror the backend config schema (trader section). `hint` gives this
// page's own hint for a field shown differently from the Configuration page. A
// `LEVELS` field edits two paired list keys, `key` and `pairKey`, as one ladder.
const CARD_SPECS = [
  {
    section: "trader",
    fields: [
      { id: "stop-loss-enabled", key: "stop_loss_enabled", type: "bool", autoSave: true },
      { id: "stop-loss-threshold", key: "stop_loss_threshold_pct", type: "float" },
      { id: "stop-loss-allow-partial", key: "stop_loss_allow_partial", type: "bool" },
      { id: "stop-loss-partial-pct", key: "stop_loss_partial_pct", type: "float" },
      { id: "stop-loss-min-hold", key: "stop_loss_min_hold_seconds", type: "int" },
    ],
  },
  {
    section: "trader",
    fields: [
      { id: "trailing-enabled", key: "trailing_stop_enabled", type: "bool", autoSave: true },
      { id: "trail-activation", key: "trailing_stop_activation_pct", type: "float" },
      { id: "trail-distance", key: "trailing_stop_distance_pct", type: "float" },
    ],
  },
  {
    section: "trader",
    fields: [
      { id: "break-even-enabled", key: "break_even_enabled", type: "bool", autoSave: true },
      { id: "break-even-trigger", key: "break_even_trigger_pct", type: "float" },
      { id: "break-even-offset", key: "break_even_offset_pct", type: "float" },
    ],
  },
  {
    section: "trader",
    fields: [
      { id: "take-profit-enabled", key: "take_profit_enabled", type: "bool", autoSave: true },
      {
        id: "take-profit-levels",
        key: "take_profit_levels_pct",
        pairKey: "take_profit_sell_pct",
        type: LEVELS,
      },
    ],
  },
  {
    section: "trader",
    fields: [
      { id: "time-override-enabled", key: "time_override_enabled", type: "bool", autoSave: true },
      { id: "time-max-hold", key: "time_override_duration", type: "float" },
      { id: "time-unit", key: "time_override_unit", type: "string" },
      {
        id: "time-loss-threshold",
        key: "time_override_loss_threshold_percent",
        type: LOSS_MAGNITUDE,
        hint: () => I18n.t("trader-time-loss-hint"),
      },
    ],
  },
  {
    section: "trader",
    fields: [
      {
        id: "time-stagnation-enabled",
        key: "time_exit_stagnation_enabled",
        type: "bool",
        autoSave: true,
      },
      { id: "time-stagnation-minutes", key: "time_exit_stagnation_minutes", type: "float" },
      { id: "time-stagnation-band", key: "time_exit_stagnation_band_pct", type: "float" },
    ],
  },
  {
    section: "trader",
    fields: [
      { id: "dca-enabled", key: "dca_enabled", type: "bool", autoSave: true },
      {
        id: "dca-threshold",
        key: "dca_threshold_pct",
        type: LOSS_MAGNITUDE,
        hint: () => I18n.t("trader-dca-threshold-hint"),
      },
      { id: "dca-max-count", key: "dca_max_count", type: "int" },
      { id: "dca-size", key: "dca_size_percentage", type: "float" },
      { id: "dca-cooldown", key: "dca_cooldown_minutes", type: "int" },
    ],
  },
  {
    section: "trader",
    fields: [
      { id: "max-positions", key: "max_open_positions", type: "int" },
      { id: "trade-size", key: "trade_size_sol", type: "float" },
      { id: "entry-sizes", key: "entry_sizes", type: "csv-floats" },
    ],
  },
  {
    section: "trader",
    fields: [
      { id: "close-cooldown", key: "position_close_cooldown_minutes", type: "int" },
      { id: "entry-concurrency", key: "entry_check_concurrency", type: "int" },
    ],
  },
];

function buttonIcon(iconClass) {
  const icon = document.createElement("i");
  icon.className = iconClass;
  return icon;
}

function buttonLabel(text) {
  const label = document.createElement("span");
  label.textContent = text;
  return label;
}

/** Snapshot string used for dirty comparison (independent of value coercion). */
function snapValue(el, type) {
  if (type === LEVELS) return levelSnapshot(el);
  return el.type === "checkbox" ? String(el.checked) : el.value;
}

/** The config values a card saves for `field`, keyed by config key. */
function fieldValues(el, field) {
  if (field.type === LEVELS) return levelValues(el, field);
  return { [field.key]: readField(el, field.type) };
}

/** Coerce an input into the value sent to the backend for a given field type. */
function readField(el, type) {
  switch (type) {
    case "bool":
      return el.checked;
    case "int": {
      const n = parseInt(el.value, 10);
      return Number.isNaN(n) ? 0 : n;
    }
    case "float": {
      const n = parseFloat(el.value);
      return Number.isNaN(n) ? 0 : n;
    }
    case LOSS_MAGNITUDE: {
      const n = parseFloat(el.value);
      return Number.isNaN(n) || n === 0 ? 0 : -n;
    }
    case "csv-floats":
      return el.value
        .split(",")
        .map((s) => parseFloat(s.trim()))
        .filter((n) => !Number.isNaN(n));
    case "minutes-to-seconds": {
      const m = parseInt(el.value, 10);
      return Math.max(0, Number.isNaN(m) ? 0 : m) * 60;
    }
    default:
      return el.value;
  }
}

/**
 * @param {{ saveConfig: (updates: object) => Promise<void> }} deps
 *   saveConfig POSTs the nested config update, reloads, and (via the caller's
 *   loadConfig) calls snapshot() again so the buttons re-hide on success.
 */
export function createTraderConfigCards({ saveConfig }) {
  /** @type {Array<{spec:object, el:HTMLElement, saved:object, saveBtn:HTMLButtonElement, resetBtn:HTMLButtonElement, cleanups:Array<Function>}>} */
  const cards = [];

  function isDirty(card) {
    return card.spec.fields.some((f) => {
      const el = document.getElementById(f.id);
      return el && snapValue(el, f.type) !== card.saved[f.id];
    });
  }

  /** Whether a field of the card holds a value its config refuses. */
  function isInvalid(card) {
    return card.spec.fields.some((f) => {
      const el = document.getElementById(f.id);
      return f.type === LEVELS && el !== null && levelError(el) !== null;
    });
  }

  function evaluate(card) {
    const dirty = isDirty(card);
    card.saveBtn.hidden = !dirty;
    card.resetBtn.hidden = !dirty;
    card.saveBtn.disabled = isInvalid(card);
  }

  function hasDirtyCards() {
    return cards.some((card) => isDirty(card));
  }

  async function save(card) {
    const payload = { [card.spec.section]: {} };
    card.spec.fields.forEach((f) => {
      const el = document.getElementById(f.id);
      if (el) Object.assign(payload[card.spec.section], fieldValues(el, f));
    });
    card.saveBtn.disabled = true;
    card.resetBtn.disabled = true;
    try {
      await saveConfig(payload);
    } catch {
      // saveConfig already surfaced an error toast; keep the buttons visible so
      // the user can retry.
    } finally {
      card.resetBtn.disabled = false;
      evaluate(card);
    }
  }

  async function saveField(card, field, el) {
    const nextSaved = snapValue(el, field.type);
    const previousSaved = card.saved[field.id];
    const payload = { [card.spec.section]: { [field.key]: readField(el, field.type) } };

    el.disabled = true;
    try {
      await saveConfig(payload, {
        reload: false,
        successTitle: el.checked
          ? I18n.t("trader-toast-feature-enabled")
          : I18n.t("trader-toast-feature-disabled"),
        successMessage: I18n.t("trader-toast-feature-applied"),
      });
      card.saved[field.id] = nextSaved;
      evaluate(card);
    } catch {
      if (el.type === "checkbox") {
        el.checked = previousSaved === "true";
      } else {
        el.value = previousSaved;
      }
      evaluate(card);
    } finally {
      el.disabled = false;
    }
  }

  function reset(card) {
    card.spec.fields.forEach((f) => {
      const el = document.getElementById(f.id);
      if (!el) return;
      const saved = card.saved[f.id];
      if (f.type === LEVELS) {
        restoreLevels(el, saved);
      } else if (el.type === "checkbox") {
        el.checked = saved === "true";
      } else {
        el.value = saved;
      }
      el.dispatchEvent(new Event("input", { bubbles: true }));
    });
    evaluate(card);
  }

  function buildButtons(card) {
    const header = card.el.querySelector(".card-header");
    if (!header) return false;

    const actions = document.createElement("div");
    actions.className = "card-header-actions";

    const resetBtn = document.createElement("button");
    resetBtn.type = "button";
    resetBtn.className = "btn btn-sm btn-ghost config-card-reset";
    resetBtn.append(buttonIcon("icon-rotate-ccw"), buttonLabel(I18n.t("common-action-reset")));
    resetBtn.hidden = true;
    resetBtn.addEventListener("click", () => reset(card));

    const saveBtn = document.createElement("button");
    saveBtn.type = "button";
    saveBtn.className = "btn btn-sm btn-primary config-card-save";
    saveBtn.append(buttonIcon("icon-save"), buttonLabel(I18n.t("common-action-save")));
    saveBtn.hidden = true;
    saveBtn.addEventListener("click", () => save(card));

    const toggle = header.querySelector(":scope > .toggle");
    actions.append(resetBtn, saveBtn);
    if (toggle) {
      header.insertBefore(actions, toggle);
    } else {
      header.appendChild(actions);
    }

    card.saveBtn = saveBtn;
    card.resetBtn = resetBtn;
    return true;
  }

  function setup() {
    CARD_SPECS.forEach((spec) => {
      const first = document.getElementById(spec.fields[0].id);
      const el = first?.closest(".config-card");
      if (!el) return;

      const card = { spec, el, saved: {}, saveBtn: null, resetBtn: null, cleanups: [] };
      if (!buildButtons(card)) return;

      spec.fields.forEach((f) => {
        const fe = document.getElementById(f.id);
        if (!fe) return;
        const evt = fe.tagName === "SELECT" || fe.type === "checkbox" ? "change" : "input";
        const handler = () => {
          if (f.autoSave) {
            saveField(card, f, fe);
            return;
          }
          evaluate(card);
        };
        fe.addEventListener(evt, handler);
        card.cleanups.push(() => fe.removeEventListener(evt, handler));
      });

      cards.push(card);
    });
    snapshot();
  }

  /**
   * Apply the same schema metadata used by the main Configuration tab to this
   * curated Auto Trader layout. The layout remains task-focused, while field
   * names, help text, units and numeric constraints retain one backend owner.
   */
  function applyMetadata(allMetadata = {}) {
    CARD_SPECS.forEach((spec) => {
      const sectionMetadata = allMetadata[spec.section] || {};
      spec.fields.forEach((field) => {
        const metadata = sectionMetadata[field.key];
        const input = document.getElementById(field.id);
        if (!metadata || !input) return;

        if (field.type === LEVELS) {
          const pair = sectionMetadata[field.pairKey];
          configureLevels(input, {
            profit: metadata,
            sell: pair,
            maxItems: metadata.max_items,
            profitUnit: fieldUnit(metadata.key),
            sellUnit: pair ? fieldUnit(pair.key) : undefined,
          });
        }

        // A group's hint and unit belong to the field its label names: the label `for` a
        // control, or the one a group control (the ladder) is labelled by. A companion
        // control in the same group (the duration's unit select) only names itself.
        const group = input.closest(".config-group");
        const labelledBy = input.getAttribute("aria-labelledby");
        const ownLabel =
          group?.querySelector(`label[for="${field.id}"]`) ??
          (labelledBy ? group?.querySelector(`label[id="${labelledBy}"]`) : null);
        const ownsGroup = Boolean(ownLabel);
        const label = ownLabel?.querySelector("span");
        const hint = ownsGroup ? group.querySelector(".config-hint") : null;
        // A ladder's units sit on its own rows, set by `configureLevels`.
        const unit = ownsGroup && field.type !== LEVELS ? group.querySelector(".input-unit") : null;

        const labelText = fieldLabel(metadata.key);
        const hintText = field.hint ? field.hint() : fieldHint(metadata.key);
        const unitText = fieldUnit(metadata.key);

        if (label) label.textContent = labelText;
        if (hint && hintText) hint.textContent = hintText;
        if (unit && unitText) unit.textContent = unitText;

        if (input.type === "number" && field.type !== "minutes-to-seconds") {
          // A magnitude field's bounds are the stored bounds negated and swapped.
          const [min, max] =
            field.type === LOSS_MAGNITUDE
              ? [metadata.max, metadata.min].map((bound) => (bound == null ? null : displayValue(field.type, bound)))
              : [metadata.min, metadata.max];
          if (min != null) input.min = String(min);
          if (max != null) input.max = String(max);
          if (metadata.step != null) input.step = String(metadata.step);
        }

        // A control labelled by its group's label takes its name from that label.
        if (!labelledBy) input.setAttribute("aria-label", labelText);
        if (hintText) input.setAttribute("title", hintText);
      });
    });
  }

  // Record the current field values as the saved baseline and re-hide buttons.
  // Called after every config load/save so "dirty" means "differs from saved".
  function snapshot() {
    cards.forEach((card) => {
      card.spec.fields.forEach((f) => {
        const el = document.getElementById(f.id);
        if (el) card.saved[f.id] = snapValue(el, f.type);
      });
      evaluate(card);
    });
  }

  function dispose() {
    cards.forEach((card) => {
      card.cleanups.forEach((c) => c());
      const actions = card.el.querySelector(".card-header-actions");
      if (!actions) return;
      actions.remove();
    });
    cards.length = 0;
  }

  return { setup, applyMetadata, snapshot, dispose, hasDirtyCards };
}
