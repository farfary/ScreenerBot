// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * The one segmented control: a small fixed one-of-many choice drawn as a single track of
 * native radios, each painted on the label beside it, plus the one performance range set
 * every range choice offers. Callers listen for `change` on the group and read the
 * radio's value; the styles live in `styles/components/form_controls.css`.
 *
 * The module imports nothing, so a module tested under node can import it; the caller
 * passes its `escapeHtml`.
 */

/**
 * The performance ranges, shortest first. `hours` is the window a caller turns into a
 * start time; `null` means the whole history. The server alone decides what a window
 * covers, so a caller sends the value, not a computed span, wherever the API accepts it.
 */
export const RANGE_OPTIONS = Object.freeze([
  Object.freeze({
    value: "24h",
    hours: 24,
    get label() {
      return I18n.t("common-range-24h");
    },
  }),
  Object.freeze({
    value: "7d",
    hours: 24 * 7,
    get label() {
      return I18n.t("common-range-7d");
    },
  }),
  Object.freeze({
    value: "30d",
    hours: 24 * 30,
    get label() {
      return I18n.t("common-range-30d");
    },
  }),
  Object.freeze({
    value: "all",
    hours: null,
    get label() {
      return I18n.t("common-range-all");
    },
  }),
]);

const TONES = new Set(["warning", "danger"]);
const ICON_CLASS = /^icon-[a-z0-9-]+$/;

// Radio names carry a per-render number, so two groups with one name never share a set.
let groupCount = 0;

/**
 * Markup of a segmented control.
 * @param {object} control
 * @param {string} control.name - the group's `data-seg` name callers select it by
 * @param {Array<{value: string, label?: string, icon?: string, count?: (number|string),
 *   tooltip?: string, tone?: "warning"|"danger"}>} control.options - `icon` is a Lucide
 *   class (`icon-…`), set bare before the text; `tone` colours the option while checked
 * @param {string} [control.value] - the checked option's value
 * @param {string} [control.ariaLabel] - the group's accessible name
 * @param {boolean} [control.compact] - the shorter track used inside a section header
 * @param {(text: string) => string} escapeHtml - escapes every name, value and text
 * @returns {string}
 */
export function renderSegmented(
  { name, options = [], value, ariaLabel = "", compact = false },
  escapeHtml
) {
  groupCount += 1;
  const group = escapeHtml(String(name));
  const radioName = `seg-${group}-${groupCount}`;
  const current = value === undefined || value === null ? null : String(value);
  const items = options
    .map((option) => {
      const optionValue = escapeHtml(String(option.value));
      const checked = String(option.value) === current ? " checked" : "";
      const tone = TONES.has(option.tone) ? ` data-tone="${option.tone}"` : "";
      const title = option.tooltip ? ` title="${escapeHtml(option.tooltip)}"` : "";
      // An option drawn as a glyph alone is named by its tooltip.
      const named =
        !option.label && option.tooltip ? ` aria-label="${escapeHtml(option.tooltip)}"` : "";
      const icon = ICON_CLASS.test(option.icon || "")
        ? `<i class="segmented-icon ${option.icon}" aria-hidden="true"></i>`
        : "";
      const text = option.label
        ? `<span class="segmented-text">${escapeHtml(option.label)}</span>`
        : "";
      const count =
        option.count === undefined || option.count === null
          ? ""
          : `<span class="segmented-count">${escapeHtml(String(option.count))}</span>`;
      return `<label class="segmented-option"${tone}${title}><input type="radio" name="${radioName}" value="${optionValue}" data-seg-value="${optionValue}"${named}${checked}><span class="segmented-label">${icon}${text}${count}</span></label>`;
    })
    .join("");
  return `<div class="segmented" role="radiogroup" aria-label="${escapeHtml(ariaLabel)}" data-seg="${group}"${
    compact ? " data-compact" : ""
  }>${items}</div>`;
}

/** Check the option of `root` (a `.segmented` group) whose value is `value`, without a change event. */
export function setSegmentedValue(root, value) {
  const current = String(value);
  root?.querySelectorAll("input[data-seg-value]").forEach((input) => {
    input.checked = input.value === current;
  });
}

/**
 * Set the count shown on options of `root` (a `.segmented` group), e.g. `{ open: 12 }`.
 * A `null` count removes the option's count; options left out keep theirs.
 */
export function setSegmentedCounts(root, counts = {}) {
  root?.querySelectorAll("input[data-seg-value]").forEach((input) => {
    if (!Object.hasOwn(counts, input.value)) return;
    const label = input.nextElementSibling;
    if (!label) return;
    const count = counts[input.value];
    let badge = label.querySelector(".segmented-count");
    if (count === null || count === undefined) {
      badge?.remove();
      return;
    }
    if (!badge) {
      badge = document.createElement("span");
      badge.className = "segmented-count";
      label.append(badge);
    }
    badge.textContent = String(count);
  });
}
