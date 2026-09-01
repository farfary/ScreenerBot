/**
 * Field rendering logic for config page.
 * Maps field types to render functions and provides field control rendering.
 */

import { create, on } from "../../core/dom.js";
import * as Utils from "../../core/utils.js";
import {
  parseArrayInput,
  describeInvalidArrayEntries,
  deepClone,
  deepEqual,
  normalizeFieldValue,
} from "./utils.js";

/**
 * Open/closed state of one collapsible tree (categories or nested objects).
 *
 * Three layers, most specific first:
 *   1. `overrides` — per-node choice, keyed by a stable ID
 *      (`${sectionId}::${category}` / `${sectionId}::${fieldKey}::${childKey}`).
 *   2. `mode` — the last bulk action: `"all"` or `"none"`, `null` when none ran.
 *   3. the node's own default (categories: `primary` visibility is open).
 *
 * A bulk action MUST be able to close a node that is open by default, so
 * "collapse all" cannot be modelled as an empty opened-set — that only drops
 * back to the defaults and leaves every primary category expanded. Hence the
 * explicit `"none"` mode, which outranks the default but still yields to a
 * later per-node click. Persistence is handled by the caller via AppState.
 */
function createTreeState() {
  return { mode: null, overrides: new Map() };
}

let objectState = createTreeState();
let categoryState = createTreeState();

function isOpen(tree, id, defaultOpen) {
  if (tree.overrides.has(id)) {
    return tree.overrides.get(id);
  }
  if (tree.mode === "all") return true;
  if (tree.mode === "none") return false;
  return defaultOpen;
}

function setBulk(tree, mode) {
  tree.mode = mode;
  tree.overrides.clear();
}

function serializeTree(tree) {
  return { mode: tree.mode, overrides: Array.from(tree.overrides.entries()) };
}

function restoreTree(data) {
  const tree = createTreeState();
  if (!data || typeof data !== "object") {
    return tree;
  }
  tree.mode = data.mode === "all" || data.mode === "none" ? data.mode : null;
  if (Array.isArray(data.overrides)) {
    for (const entry of data.overrides) {
      if (Array.isArray(entry) && typeof entry[0] === "string") {
        tree.overrides.set(entry[0], Boolean(entry[1]));
      }
    }
  }
  return tree;
}

export function serializeOpenState() {
  return { objects: serializeTree(objectState), categories: serializeTree(categoryState) };
}

export function restoreOpenState(data) {
  objectState = restoreTree(data?.objects);
  categoryState = restoreTree(data?.categories);
}

export function expandAllObjects() {
  setBulk(objectState, "all");
}

export function collapseAllObjects() {
  setBulk(objectState, "none");
}

export function expandAllCategories() {
  setBulk(categoryState, "all");
}

export function collapseAllCategories() {
  setBulk(categoryState, "none");
}

export const SECTION_ICONS = {
  rpc: "icon-satellite",
  trader: "icon-briefcase",
  positions: "icon-chart-candlestick",
  filtering: "icon-target",
  swaps: "icon-repeat",
  tokens: "icon-coins",
  pools: "icon-database",
  wallet: "icon-wallet",
  sol_price: "icon-sun",
  events: "icon-radio",
  webserver: "icon-network",
  services: "icon-wrench",
  monitoring: "icon-trending-up",
  maintenance: "icon-settings",
  updates: "icon-arrow-down-to-line",
  ohlcv: "icon-clock",
  summary: "icon-file-text",
  telegram: "icon-send",
  llm: "icon-brain-circuit",
  llm_analysis: "icon-scan-search",
  assistant: "icon-bot-message-square",
  agent_control: "icon-shield-check",
  strategies: "icon-brain",
  holder_watch: "icon-eye",
  performance: "icon-gauge",
  network: "icon-globe",
  // Verified against src/webserver/assets/lucide-font/lucide.css — a name that
  // is not in the font renders a silent blank box. Same mark as the website
  // admin's referral surfaces, so the two read as one feature.
  referral: "icon-hand-coins",
  // Verified against assets/lucide-font/lucide.css — a name the font does not
  // carry renders as a silent blank box.
  account: "icon-circle-user",
};

/**
 * Build a stable ID for an object wrapper at this path. The path is the
 * array of config keys from the section root down to (and including) this
 * object's key. The path is joined with `::` to avoid collisions with
 * config keys that contain dots.
 */
function objectPathId(path) {
  return path.join("::");
}

// Nested sub-configs are collapsed by default; categories pass their own
// default (primary categories open, everything else closed).
export function isObjectOpen(path) {
  return isOpen(objectState, objectPathId(path), false);
}

export function isCategoryOpen(path, defaultOpen = false) {
  return isOpen(categoryState, path.join("::"), defaultOpen);
}

export function toggleObject(path) {
  const id = objectPathId(path);
  objectState.overrides.set(id, !isObjectOpen(path));
}

export function toggleCategory(path, defaultOpen = false) {
  const id = path.join("::");
  categoryState.overrides.set(id, !isCategoryOpen(path, defaultOpen));
}

/**
 * The label block every config row shares — top-level fields and nested
 * sub-config rows alike, so the two can never drift apart:
 *
 *   name (+ impact chip)
 *   one description line
 *   one reference strip: config key · unit · default
 *
 * `docs` is the field's Rust doc comment and near-duplicates `hint`, so it is
 * only the fallback description — never a second line of its own.
 */
export function buildFieldLabelHtml({ label, pathLabel, metadata = {}, defaultValue }) {
  const impact = metadata.impact
    ? `<span class="config-field-impact ${Utils.escapeHtml(metadata.impact.toLowerCase())}">` +
      `${Utils.escapeHtml(metadata.impact)}</span>`
    : "";
  // A sub-config's own title IS its collapse control — a separate header bar
  // below it only repeated the name.
  const grouped = isGroupField(metadata);
  const titleTag = grouped ? "button" : "div";
  const titleAttrs = grouped
    ? ' type="button" class="config-field-title config-field-title--toggle"'
    : ' class="config-field-title"';
  const chevron = grouped
    ? '<i class="config-object-chevron icon-chevron-down" aria-hidden="true"></i>'
    : "";
  const parts = [
    `<${titleTag}${titleAttrs}>${chevron}` +
      `<span class="config-field-name">${Utils.escapeHtml(label)}</span>${impact}</${titleTag}>`,
  ];

  const description = metadata.hint || metadata.docs;
  if (description) {
    parts.push(`<div class="config-field-hint">${Utils.escapeHtml(description)}</div>`);
  }

  const meta = [`<span class="config-field-key">${Utils.escapeHtml(pathLabel)}</span>`];
  if (metadata.unit) {
    meta.push(`<span>Unit: ${Utils.escapeHtml(metadata.unit)}</span>`);
  }
  if (defaultValue !== null && defaultValue !== undefined) {
    const defaultText = Utils.escapeHtml(
      typeof defaultValue === "object" ? JSON.stringify(defaultValue) : String(defaultValue)
    );
    meta.push(
      `<span class="config-field-default" title="Default: ${defaultText}">` +
        `Default: ${defaultText}</span>`
    );
  }
  parts.push(`<div class="config-field-meta">${meta.join("")}</div>`);

  return parts.join("\n");
}

/**
 * The reset control is a bare glyph in a slot the row always reserves, so a
 * value returning to its default cannot reflow anything. Fields with no
 * declared default hide it outright.
 */
export function createResetButton(defaultValue, isAtDefault) {
  const button = create("button", {
    type: "button",
    className: "config-field-reset",
    disabled: defaultValue === undefined ? true : isAtDefault,
    title: "Reset to default",
  });
  button.innerHTML = '<i class="icon-rotate-ccw" aria-hidden="true"></i>';
  button.setAttribute("aria-label", "Reset to default");
  if (defaultValue === undefined) {
    button.hidden = true;
  }
  return button;
}

/**
 * A field that expands into a tree of child rows rather than taking a value of
 * its own. An object with no declared children falls back to a JSON textarea,
 * so it is NOT a group.
 */
export function isGroupField(metadata = {}) {
  return (
    metadata.type === "object" &&
    Boolean(metadata.children) &&
    Object.keys(metadata.children).length > 0
  );
}

/**
 * Wide controls (textareas, sub-config trees) cannot live in the narrow control
 * track: the row keeps label + reset on its first line and gives the control
 * the full width underneath.
 */
export function applyStackedLayout(rowEl, control) {
  if (control.tagName === "TEXTAREA" || control.classList.contains("config-object-wrapper")) {
    rowEl.classList.add("config-field--stacked");
  }
}

/**
 * Wire a group row's title to its sub-config tree. The open/closed class lives
 * on BOTH the wrapper (what the bulk expand/collapse control counts) and the
 * row (which owns the chevron), and the row is seeded from the wrapper so a
 * restored collapsed state paints correctly on first render.
 */
export function attachGroupToggle({ rowEl, labelEl, control, path, onCollapseChange }) {
  const toggle = labelEl.querySelector(".config-field-title--toggle");
  if (!toggle || !control.classList.contains("config-object-wrapper")) {
    return;
  }
  rowEl.classList.toggle("collapsed", control.classList.contains("collapsed"));
  on(toggle, "click", () => {
    toggleObject(path);
    const collapsed = control.classList.toggle("collapsed");
    rowEl.classList.toggle("collapsed", collapsed);
    if (typeof onCollapseChange === "function") {
      onCollapseChange();
    }
  });
}

/**
 * Render an object field with children recursively. The tree carries no header
 * of its own — the owning row's title is the collapse control
 * (`attachGroupToggle`), so the sub-config name is printed exactly once.
 * @param {Object} options - Rendering options
 * @returns {HTMLElement|null} Rendered element or null
 */
export function renderObjectWithChildren({
  fieldId,
  value,
  originalValue,
  metadata = {},
  disabled,
  path = [],
  searchTerm = "",
  onChange,
  onCollapseChange,
}) {
  if (!metadata.children) {
    return null;
  }

  const entries = Object.entries(metadata.children);
  if (!entries.length) {
    return null;
  }

  const startCollapsed = !isObjectOpen(path);
  const wrapper = create("div", {
    className: startCollapsed ? "config-object-wrapper collapsed" : "config-object-wrapper",
  });

  const container = create("div", {
    className: "config-object-group",
  });

  const safeValue = value && typeof value === "object" ? value : {};
  const safeOriginal = originalValue && typeof originalValue === "object" ? originalValue : {};
  const normalizedSearch = typeof searchTerm === "string" ? searchTerm.trim().toLowerCase() : "";
  const hasSearch = normalizedSearch.length > 0;

  entries.sort(([keyA, metaA], [keyB, metaB]) => {
    const labelA = metaA.label || keyA;
    const labelB = metaB.label || keyB;
    return labelA.localeCompare(labelB);
  });

  for (const [childKey, childMeta] of entries) {
    const childId = `${fieldId}-${childKey}`;
    const childPath = [...path, childKey];
    const childPathLabel = childPath.join(".");
    const childValue = safeValue[childKey];
    const childOriginal = safeOriginal[childKey];
    const childDefault = deepClone(childMeta.default);

    const row = create("div", { className: "config-object-field" });

    if (!deepEqual(childValue, childOriginal)) {
      row.classList.add("config-object-field--changed");
    }

    if (hasSearch && metadataMatchesSearchLocal(childKey, childMeta, normalizedSearch)) {
      row.classList.add("config-object-field--match");
    }

    const labelEl = create("div", { className: "config-field-label" });
    labelEl.innerHTML = buildFieldLabelHtml({
      label: childMeta.label || childKey,
      pathLabel: childPathLabel,
      metadata: childMeta,
      defaultValue: childMeta.type === "object" ? undefined : childDefault,
    });

    const controlEl = create("div", { className: "config-field-control" });

    const childControl = renderFieldControl(childMeta.type, {
      fieldId: childId,
      value: childValue,
      originalValue: childOriginal,
      metadata: childMeta,
      disabled,
      path: childPath,
      searchTerm: normalizedSearch,
      onChange: (nextValue) => {
        const normalizedChild = normalizeFieldValue(childMeta.type, nextValue);
        const nextObject = deepClone(safeValue);
        nextObject[childKey] = normalizedChild;
        onChange(nextObject);
      },
      onCollapseChange,
    });

    controlEl.appendChild(childControl);
    applyStackedLayout(row, childControl);
    attachGroupToggle({
      rowEl: row,
      labelEl,
      control: childControl,
      path: childPath,
      onCollapseChange,
    });

    const resetBtn = createResetButton(childDefault, deepEqual(childValue, childDefault));
    on(resetBtn, "click", () => {
      const nextObject = deepClone(safeValue);
      if (childDefault === null) {
        nextObject[childKey] = null;
      } else if (typeof childDefault === "object") {
        nextObject[childKey] = deepClone(childDefault);
      } else {
        nextObject[childKey] = childDefault;
      }
      onChange(nextObject);
    });

    row.appendChild(labelEl);
    row.appendChild(controlEl);
    row.appendChild(resetBtn);
    container.appendChild(row);
  }

  wrapper.appendChild(container);
  return wrapper;
}

/**
 * Local copy of metadataMatchesSearch for field_renderers.js
 * (avoids circular dependency with utils.js)
 */
function metadataMatchesSearchLocal(fieldKey, fieldMeta, term) {
  if (!term || term.length === 0) {
    return false;
  }
  const matches = (value) => typeof value === "string" && value.toLowerCase().includes(term);

  if (matches(fieldKey)) {
    return true;
  }
  if (
    matches(fieldMeta.label) ||
    matches(fieldMeta.hint) ||
    matches(fieldMeta.docs) ||
    matches(fieldMeta.unit)
  ) {
    return true;
  }

  if (fieldMeta.children) {
    for (const [childKey, childMeta] of Object.entries(fieldMeta.children)) {
      if (matches(childKey) || metadataMatchesSearchLocal(childKey, childMeta, term)) {
        return true;
      }
    }
  }

  return false;
}

/**
 * Field renderer functions for different field types.
 */
export const FIELD_RENDERERS = {
  boolean({ fieldId, value, disabled, onChange }) {
    const input = create("input", {
      type: "checkbox",
      id: fieldId,
      checked: Boolean(value),
      disabled,
    });
    on(input, "change", (event) => {
      onChange(event.target.checked);
    });
    return input;
  },
  number({ fieldId, value, metadata = {}, disabled, onChange }) {
    const input = create("input", {
      type: "number",
      id: fieldId,
      value: value ?? "",
      disabled,
      step: metadata.step ?? "any",
      min: metadata.min ?? undefined,
      max: metadata.max ?? undefined,
      autocomplete: "off",
    });
    on(input, "input", (event) => {
      const raw = event.target.value;
      const num = raw === "" ? null : Number(raw);
      onChange(Number.isFinite(num) ? num : raw === "" ? null : raw);
    });
    on(input, "keydown", (event) => {
      if (event.key === "ArrowUp" || event.key === "ArrowDown") {
        event.stopPropagation();
      }
    });
    return input;
  },
  integer(options) {
    const component = FIELD_RENDERERS.number({
      ...options,
      metadata: {
        ...options.metadata,
        step: options.metadata?.step ?? 1,
      },
    });
    return component;
  },
  string({ fieldId, value, metadata = {}, disabled, onChange }) {
    // Only genuinely long or multi-line values earn a textarea. `docs` is the
    // field's Rust doc comment, which nearly every field carries, so treating
    // it as a "long text" marker turned every string into a 100px box.
    if (typeof value === "string" && (value.length > 120 || value.includes("\n"))) {
      const textarea = create("textarea", {
        id: fieldId,
        value: value ?? "",
        placeholder: metadata.placeholder ?? "",
        disabled,
        autocomplete: "off",
        spellcheck: false,
      });
      on(textarea, "input", (event) => {
        onChange(event.target.value);
      });
      return textarea;
    }

    const input = create("input", {
      type: "text",
      id: fieldId,
      value: value ?? "",
      placeholder: metadata.placeholder ?? "",
      disabled,
      autocomplete: "off",
      spellcheck: false,
    });
    on(input, "input", (event) => {
      onChange(event.target.value);
    });
    return input;
  },
  array({ fieldId, value, metadata = {}, disabled, onChange }) {
    const textarea = create("textarea", {
      id: fieldId,
      value: Array.isArray(value) ? value.join("\n") : "",
      placeholder: metadata.placeholder ?? "Enter one value per line",
      disabled,
      autocomplete: "off",
      spellcheck: false,
    });
    const itemType = metadata.item_type || "string";

    const handleInput = (event) => {
      const { values, invalid } = parseArrayInput(event.target.value, itemType);
      if (invalid.length > 0) {
        const message = describeInvalidArrayEntries(invalid, itemType);
        event.target.classList.add("config-field-error-input");
        if (message) {
          event.target.setAttribute("title", message);
        }
        event.target.dataset.arrayInvalid = "true";
        return;
      }

      event.target.classList.remove("config-field-error-input");
      event.target.removeAttribute("title");
      delete event.target.dataset.arrayInvalid;
      onChange(values);
    };

    on(textarea, "input", handleInput);
    on(textarea, "blur", (event) => {
      if (!event.target.dataset.arrayInvalid) {
        return;
      }
      const { invalid } = parseArrayInput(event.target.value, itemType);
      if (invalid.length === 0) {
        handleInput(event);
        return;
      }
      const message = describeInvalidArrayEntries(invalid, itemType);
      if (message) {
        // Keyed, so typing through several invalid entries updates ONE notice.
        // This used to be a hand-rolled 1.5s throttle stamped onto the input's
        // dataset, which existed only because toasts could not replace one
        // another.
        Utils.showToast({
          key: "config-invalid-array",
          type: "error",
          title: "Invalid array entry",
          message,
        });
        event.target.setAttribute("title", message);
      }
    });
    return textarea;
  },
  object({
    fieldId,
    value,
    originalValue,
    metadata = {},
    disabled,
    path = [],
    searchTerm = "",
    onChange,
    onCollapseChange,
  }) {
    const nested = renderObjectWithChildren({
      fieldId,
      value,
      originalValue,
      metadata,
      disabled,
      path,
      searchTerm,
      onChange,
      onCollapseChange,
    });
    if (nested) {
      return nested;
    }

    const textarea = create("textarea", {
      id: fieldId,
      value: JSON.stringify(value ?? {}, null, 2),
      disabled,
      autocomplete: "off",
      spellcheck: false,
    });
    on(textarea, "blur", (event) => {
      const raw = event.target.value.trim();
      if (raw === "") {
        onChange({});
        return;
      }
      try {
        const parsed = JSON.parse(raw);
        onChange(parsed);
        textarea.classList.remove("config-field-error-input");
      } catch (error) {
        textarea.classList.add("config-field-error-input");
        Utils.showToast({
          key: "config-invalid-json",
          type: "error",
          title: "Invalid JSON",
          message: error.message,
        });
      }
    });
    return textarea;
  },
};

/**
 * Render a field control based on field type.
 * @param {string} fieldType - Type of field
 * @param {Object} options - Rendering options
 * @returns {HTMLElement} Rendered field control
 */
export function renderFieldControl(fieldType, options) {
  const renderer = FIELD_RENDERERS[fieldType];
  if (!renderer) {
    return create("input", {
      type: "text",
      value: options.value ?? "",
      disabled: true,
    });
  }
  return renderer(options);
}
