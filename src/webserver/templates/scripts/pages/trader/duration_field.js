// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
//! A duration field of the Auto Trader rule cards: a number beside the unit picker Time Exit uses, shown in any offered unit and converted to the unit its config key stores.

/**
 * A duration input names the unit its config key stores in `data-duration` and its unit
 * picker in `data-duration-unit`. The picker belongs to the view: the number and the picked
 * unit together are the duration, the way Time Exit's duration and unit are, and only the
 * value in the stored unit is saved, so no stored setting changes its unit.
 */

/** Seconds in one of each unit a picker offers. */
export const UNIT_SECONDS = Object.freeze({
  seconds: 1,
  minutes: 60,
  hours: 3600,
  days: 86400,
});

/** The unit picker beside the duration input `input`, or `null`. */
function unitPicker(input) {
  const id = input.dataset.durationUnit;
  return id ? document.getElementById(id) : null;
}

/** The unit `input`'s config key stores. */
function storedUnit(input) {
  return input.dataset.duration;
}

/** The unit `input` is shown in: its picker's, or the stored one without a picker. */
function shownUnit(input) {
  const unit = unitPicker(input)?.value;
  return Object.hasOwn(UNIT_SECONDS, unit) ? unit : storedUnit(input);
}

/** How many stored units one shown unit holds. */
function shownToStored(input) {
  return UNIT_SECONDS[shownUnit(input)] / UNIT_SECONDS[storedUnit(input)];
}

/** The duration `input` holds, in the unit its config stores; `NaN` while the field is blank. */
export function storedDuration(input) {
  return Number.parseFloat(input.value) * shownToStored(input);
}

/**
 * Shows `stored`, a value in the input's stored unit, in the largest unit its picker
 * offers that holds it whole; zero and an unreadable value keep the stored unit.
 */
export function showDuration(input, stored) {
  const picker = unitPicker(input);
  const value = Number(stored);
  const seconds = value * UNIT_SECONDS[storedUnit(input)];
  const offered = picker ? [...picker.options].map((option) => option.value) : [];
  const unit =
    offered
      .filter((name) => Object.hasOwn(UNIT_SECONDS, name))
      .reverse()
      .find((name) => seconds > 0 && seconds % UNIT_SECONDS[name] === 0) ?? storedUnit(input);
  if (picker) picker.value = unit;
  input.value = Number.isFinite(value) ? String(seconds / UNIT_SECONDS[unit]) : "";
  syncBounds(input);
}

/** Keeps `input`'s metadata bounds, in its stored unit, and applies them in the shown unit. */
export function setDurationBounds(input, { min, max, step }) {
  input.dataset.durationMin = min ?? "";
  input.dataset.durationMax = max ?? "";
  input.dataset.durationStep = step ?? "";
  syncBounds(input);
}

/** Writes the kept bounds as the shown unit reads them; a unit other than the stored one takes any step. */
function syncBounds(input) {
  const factor = shownToStored(input);
  for (const [attribute, key] of [
    ["min", "durationMin"],
    ["max", "durationMax"],
  ]) {
    const bound = Number.parseFloat(input.dataset[key] ?? "");
    if (Number.isFinite(bound)) input[attribute] = String(bound / factor);
    else input.removeAttribute(attribute);
  }
  const step = input.dataset.durationStep;
  input.step = factor === 1 && step ? step : "any";
}

/**
 * Wires the unit picker of every duration input under `root`: a picked unit applies the
 * bounds in that unit and reports the input changed, so its card and its computed line
 * read the new duration. Returns the cleanup.
 */
export function bindDurationFields(root = document) {
  const cleanups = [];
  for (const input of root.querySelectorAll("input[data-duration]")) {
    const picker = unitPicker(input);
    if (!picker) continue;
    const onChange = () => {
      syncBounds(input);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    };
    picker.addEventListener("change", onChange);
    cleanups.push(() => picker.removeEventListener("change", onChange));
  }
  return () => cleanups.forEach((cleanup) => cleanup());
}
