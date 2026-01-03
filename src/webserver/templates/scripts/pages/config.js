import { registerPage } from "../core/lifecycle.js";
import { $, on, off, create, show, hide } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { requestManager } from "../core/request_manager.js";
import * as Hints from "../core/hints.js";
import { HintTrigger } from "../ui/hint_popover.js";

const CONFIG_STATE_KEY = "config.page";
const DEFAULT_SECTION = "trader";

const SECTION_ICONS = {
  rpc: "icon-satellite",
  trader: "icon-briefcase",
  positions: "icon-chart-candlestick",
  filtering: "icon-target",
  swaps: "icon-repeat",
  tokens: "icon-coins",
  sol_price: "icon-sun",
  events: "icon-radio",
  webserver: "icon-network",
  services: "icon-wrench",
  monitoring: "icon-trending-up",
  ohlcv: "icon-clock",
  summary: "icon-file-text",
  telegram: "icon-send",
};

const SECTION_LABEL_OVERRIDES = {
  rpc: "RPC",
  trader: "Auto Trader",
  positions: "Positions",
  filtering: "Filtering",
  swaps: "Swaps",
  tokens: "Tokens",
  sol_price: "SOL Price",
  events: "Events",
  webserver: "Webserver",
  services: "Services",
  monitoring: "Monitoring",
  ohlcv: "OHLCV",
  summary: "Summary",
  telegram: "Telegram",
};

const SECTION_DISPLAY_ORDER = [
  "rpc",
  "trader",
  "positions",
  "filtering",
  "swaps",
  "tokens",
  "sol_price",
  "telegram",
  "events",
  "webserver",
  "services",
  "monitoring",
  "ohlcv",
  "summary",
];

function toTitleCase(id) {
  return id
    .split(/[_\s]+/)
    .filter(Boolean)
    .map((chunk) => chunk.charAt(0).toUpperCase() + chunk.slice(1))
    .join(" ");
}

function formatSectionLabel(sectionId) {
  if (SECTION_LABEL_OVERRIDES[sectionId]) {
    return SECTION_LABEL_OVERRIDES[sectionId];
  }
  return toTitleCase(sectionId);
}

function parseArrayInput(rawText, itemType) {
  const lines = rawText
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

  const normalizedType = (itemType || "string").toLowerCase();

  if (normalizedType === "number" || normalizedType === "integer") {
    const values = [];
    const invalid = [];
    lines.forEach((line, index) => {
      const parsed = Number(line);
      const isFiniteNumber = Number.isFinite(parsed);
      const isIntegerValid = normalizedType === "integer" ? Number.isInteger(parsed) : true;
      if (!isFiniteNumber || !isIntegerValid) {
        invalid.push({ index, value: line });
        return;
      }
      values.push(parsed);
    });
    return { values, invalid };
  }

  if (normalizedType === "boolean") {
    const truthy = new Set(["true", "1", "yes", "y", "on"]);
    const falsy = new Set(["false", "0", "no", "n", "off"]);
    const values = [];
    const invalid = [];

    lines.forEach((line, index) => {
      const lowered = line.toLowerCase();
      if (truthy.has(lowered)) {
        values.push(true);
        return;
      }
      if (falsy.has(lowered)) {
        values.push(false);
        return;
      }
      invalid.push({ index, value: line });
    });

    return { values, invalid };
  }

  return { values: lines, invalid: [] };
}

function describeInvalidArrayEntries(invalidEntries, itemType) {
  if (!invalidEntries.length) {
    return "";
  }
  const lines = invalidEntries.map((entry) => entry.index + 1).join(", ");
  const typeLabel = itemType === "integer" ? "integer" : itemType || "value";
  return `Line${invalidEntries.length === 1 ? "" : "s"} ${lines} must be a valid ${typeLabel}.`;
}

function renderObjectWithChildren({
  fieldId,
  value,
  originalValue,
  metadata = {},
  disabled,
  path = [],
  searchTerm = "",
  onChange,
  parentLabel = "",
}) {
  if (!metadata.children) {
    return null;
  }

  const entries = Object.entries(metadata.children);
  if (!entries.length) {
    return null;
  }

  const wrapper = create("div", { className: "config-object-wrapper collapsed" });

  if (parentLabel) {
    const header = create("button", {
      type: "button",
      className: "config-object-header",
    });
    header.innerHTML = `<i class="config-object-chevron icon-chevron-down"></i><span>${Utils.escapeHtml(parentLabel)}</span>`;
    on(header, "click", () => {
      wrapper.classList.toggle("collapsed");
    });
    wrapper.appendChild(header);
  }

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

    if (hasSearch && metadataMatchesSearch(childKey, childMeta, normalizedSearch)) {
      row.classList.add("config-object-field--match");
    }

    const labelHtml = [];
    labelHtml.push(
      `<div class="config-field-name">${Utils.escapeHtml(childMeta.label || childKey)}</div>`
    );
    labelHtml.push(`<div class="config-field-key">${Utils.escapeHtml(childPathLabel)}</div>`);
    if (childMeta.hint) {
      labelHtml.push(`<div class="config-field-hint">${Utils.escapeHtml(childMeta.hint)}</div>`);
    }

    const metaItems = [];
    if (childMeta.unit) {
      metaItems.push(
        `<span class="config-field-unit">Unit: ${Utils.escapeHtml(childMeta.unit)}</span>`
      );
    }
    if (childMeta.impact) {
      metaItems.push(
        `<span class="config-field-impact ${Utils.escapeHtml(childMeta.impact.toLowerCase())}">` +
          `${Utils.escapeHtml(childMeta.impact)}</span>`
      );
    }
    if (childMeta.docs) {
      metaItems.push(`<span>Docs: ${Utils.escapeHtml(childMeta.docs)}</span>`);
    }
    if (metaItems.length > 0) {
      labelHtml.push(`<div class="config-field-meta">${metaItems.join(" ")}</div>`);
    }

    if (childDefault !== null && childDefault !== undefined) {
      const defaultText =
        typeof childDefault === "object"
          ? Utils.escapeHtml(JSON.stringify(childDefault))
          : Utils.escapeHtml(String(childDefault));
      labelHtml.push(`<div class="config-field-default">Default: ${defaultText}</div>`);
    }

    const labelEl = create("div", {
      className: "config-field-label config-object-field-label",
    });
    labelEl.innerHTML = labelHtml.join("\n");

    const controlEl = create("div", {
      className: "config-field-control config-object-field-control",
    });

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
    });

    controlEl.appendChild(childControl);

    if (childDefault !== undefined) {
      const atDefault = deepEqual(childValue, childDefault);
      const resetBtn = create("button", {
        type: "button",
        className: "config-field-reset",
        disabled: atDefault,
      });
      resetBtn.textContent = "Reset to default";
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
      controlEl.appendChild(resetBtn);
    }

    row.appendChild(labelEl);
    row.appendChild(controlEl);
    container.appendChild(row);
  }

  wrapper.appendChild(container);
  return wrapper;
}

const FIELD_RENDERERS = {
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
    if (
      metadata.docs ||
      metadata.placeholder ||
      (typeof value === "string" && value.length > 120)
    ) {
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
      delete event.target.dataset.arrayInvalidToastTs;
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
        const lastToastAt = Number(event.target.dataset.arrayInvalidToastTs || 0);
        const now = Date.now();
        if (Number.isNaN(lastToastAt) || now - lastToastAt > 1500) {
          Utils.showToast({
            type: "error",
            title: "Invalid Array Entry",
            message: message,
          });
          event.target.dataset.arrayInvalidToastTs = String(now);
        }
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
      parentLabel: metadata.label || (path.length > 0 ? path[path.length - 1] : ""),
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
          type: "error",
          title: "Invalid JSON",
          message: error.message,
        });
      }
    });
    return textarea;
  },
};

function renderFieldControl(fieldType, options) {
  const renderer = FIELD_RENDERERS[fieldType];
  if (!renderer) {
    const input = create("input", {
      type: "text",
      value: options.value ?? "",
      disabled: true,
    });
    input.classList.add("config-field-control-unsupported");
    return input;
  }
  return renderer(options);
}

function normalizeFieldValue(fieldType, value) {
  if (value === null || value === undefined) {
    return null;
  }
  if (fieldType === "number" || fieldType === "integer") {
    if (typeof value === "number") {
      return value;
    }
    if (typeof value === "string") {
      const parsed = Number(value);
      return Number.isFinite(parsed) ? parsed : value;
    }
  }
  if (fieldType === "boolean") {
    return Boolean(value);
  }
  if (fieldType === "array") {
    return Array.isArray(value) ? value.slice() : [];
  }
  if (fieldType === "object") {
    return value && typeof value === "object" ? { ...value } : {};
  }
  if (fieldType === "string") {
    return typeof value === "string" ? value : String(value);
  }
  return value;
}

function deepClone(value) {
  if (Array.isArray(value)) {
    return value.map((item) => deepClone(item));
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([key, val]) => [key, deepClone(val)]));
  }
  return value;
}

function deepEqual(a, b) {
  if (a === b) {
    return true;
  }
  if (typeof a !== typeof b) {
    return false;
  }
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) {
      return false;
    }
    for (let i = 0; i < a.length; i += 1) {
      if (!deepEqual(a[i], b[i])) {
        return false;
      }
    }
    return true;
  }
  if (a && typeof a === "object" && b && typeof b === "object") {
    const aKeys = Object.keys(a);
    const bKeys = Object.keys(b);
    if (aKeys.length !== bKeys.length) {
      return false;
    }
    for (const key of aKeys) {
      if (!deepEqual(a[key], b[key])) {
        return false;
      }
    }
    return true;
  }
  return false;
}

function summarizeSectionFields(fields = {}) {
  const entries = Object.values(fields);
  const summary = {
    total: entries.length,
    critical: 0,
    performance: 0,
  };

  for (const field of entries) {
    const impact = (field.impact || "").toLowerCase();
    if (impact === "critical") {
      summary.critical += 1;
    }
    const category = (field.category || "").toLowerCase();
    if (category.includes("performance")) {
      summary.performance += 1;
    }
  }

  return summary;
}

function normalizeFieldMetadata(fieldMeta = {}) {
  const normalized = {
    ...fieldMeta,
    type: typeof fieldMeta.type === "string" ? fieldMeta.type.toLowerCase() : "string",
    default: deepClone(fieldMeta.default ?? null),
  };

  if (typeof fieldMeta.item_type === "string") {
    normalized.item_type = fieldMeta.item_type.toLowerCase();
  }

  if (fieldMeta.children && typeof fieldMeta.children === "object") {
    const normalizedChildren = {};
    for (const [childKey, childMeta] of Object.entries(fieldMeta.children)) {
      normalizedChildren[childKey] = normalizeFieldMetadata(childMeta);
    }
    normalized.children = normalizedChildren;
  }

  return normalized;
}

function metadataMatchesSearch(fieldKey, fieldMeta, term) {
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
      if (matches(childKey) || metadataMatchesSearch(childKey, childMeta, term)) {
        return true;
      }
    }
  }

  return false;
}

function sectionHasMatchingFields(sectionId, term) {
  if (!term || term.length === 0) {
    return true;
  }
  const metadata = state.metadata?.[sectionId];
  if (!metadata) {
    return false;
  }

  const label = metadata.label ?? formatSectionLabel(sectionId);
  if (sectionId.toLowerCase().includes(term) || label.toLowerCase().includes(term)) {
    return true;
  }

  const fields = metadata.fields ?? {};
  for (const [fieldKey, fieldMeta] of Object.entries(fields)) {
    const category = fieldMeta.category ?? "General";
    if (category.toLowerCase().includes(term)) {
      return true;
    }
    if (metadataMatchesSearch(fieldKey, fieldMeta, term)) {
      return true;
    }
  }
  return false;
}

function transformMetadata(raw) {
  const sections = {};
  for (const [sectionId, fields] of Object.entries(raw || {})) {
    const normalizedFields = {};
    for (const [fieldKey, fieldMeta] of Object.entries(fields || {})) {
      normalizedFields[fieldKey] = normalizeFieldMetadata(fieldMeta);
    }

    sections[sectionId] = {
      id: sectionId,
      label: formatSectionLabel(sectionId),
      fields: normalizedFields,
      summary: summarizeSectionFields(normalizedFields),
    };
  }
  return sections;
}

function sortSectionsForDisplay(entries) {
  const orderIndex = (sectionId) => {
    const index = SECTION_DISPLAY_ORDER.indexOf(sectionId);
    return index === -1 ? Number.POSITIVE_INFINITY : index;
  };

  return entries.sort(([idA], [idB]) => {
    const orderA = orderIndex(idA);
    const orderB = orderIndex(idB);
    if (orderA === orderB) {
      return idA.localeCompare(idB);
    }
    return orderA - orderB;
  });
}

function ensureActiveSectionValid() {
  const metadata = state.metadata || {};
  const sectionIds = sortSectionsForDisplay(Object.entries(metadata)).map(
    ([sectionId]) => sectionId
  );
  if (sectionIds.length === 0) {
    if (state.activeSection !== null) {
      state.activeSection = null;
      AppState.save(`${CONFIG_STATE_KEY}.activeSection`, null);
    }
    return;
  }

  if (!state.activeSection || !metadata[state.activeSection]) {
    const preferred = sectionIds.includes(DEFAULT_SECTION) ? DEFAULT_SECTION : sectionIds[0];
    state.activeSection = preferred;
    AppState.save(`${CONFIG_STATE_KEY}.activeSection`, preferred);
  }
}

const state = {
  metadata: null,
  config: null,
  original: null,
  draft: null,
  activeSection: AppState.load(`${CONFIG_STATE_KEY}.activeSection`, DEFAULT_SECTION),
  search: AppState.load(`${CONFIG_STATE_KEY}.search`, ""),
  pendingChanges: new Map(),
  errors: new Map(),
  loading: false,
  saving: false,
};

function setState(partial) {
  Object.assign(state, partial);
  render();
}

function countPendingChanges(sectionId) {
  if (!sectionId) {
    return 0;
  }
  let count = 0;
  for (const key of state.pendingChanges.keys()) {
    if (key.startsWith(`${sectionId}.`)) {
      count += 1;
    }
  }
  return count;
}

function markFieldChanged(sectionId, fieldKey, changed) {
  const key = `${sectionId}.${fieldKey}`;
  if (changed) {
    state.pendingChanges.set(key, true);
  } else {
    state.pendingChanges.delete(key);
  }
}

function hasSectionChanges(sectionId) {
  return countPendingChanges(sectionId) > 0;
}

function getSectionMetadata(sectionId) {
  return state.metadata?.[sectionId] ?? null;
}

function resetFieldToDefault(sectionId, fieldKey) {
  const metadata = getSectionMetadata(sectionId);
  const fieldMeta = metadata?.fields?.[fieldKey];
  if (!fieldMeta) {
    return;
  }
  if (fieldMeta.default === undefined) {
    return;
  }
  const defaultValue = deepClone(fieldMeta.default);
  const normalized = normalizeFieldValue(fieldMeta.type, defaultValue);

  const currentValue = state.draft?.[sectionId]?.[fieldKey];
  const originalSection = state.original?.[sectionId] ?? {};
  if (!deepEqual(currentValue, normalized)) {
    if (!state.draft[sectionId]) {
      state.draft[sectionId] = {};
    }
    state.draft[sectionId][fieldKey] = normalized;
    markFieldChanged(sectionId, fieldKey, !deepEqual(normalized, originalSection[fieldKey]));
    render();
  }
}

function renderStateMessage() {
  const banner = $("#configStateMessage");
  if (!banner) {
    return;
  }

  const { loading, saving, errors } = state;
  banner.innerHTML = "";
  banner.className = "config-state";

  if (loading) {
    banner.innerHTML = "<strong>Loading configuration…</strong><div>Please wait</div>";
    banner.classList.add("loading");
    banner.hidden = false;
    return;
  }

  if (saving) {
    banner.innerHTML = "<strong>Saving changes…</strong><div>Updating configuration</div>";
    banner.classList.add("loading");
    banner.hidden = false;
    return;
  }

  if (errors.size > 0) {
    banner.innerHTML =
      "<strong>Validation issues detected.</strong> Please review highlighted fields.";
    banner.classList.add("error");
    banner.hidden = false;
    return;
  }

  banner.hidden = true;
}

function renderSidebar() {
  const container = $("#configSectionsList");
  if (!container) {
    return;
  }
  container.innerHTML = "";
  const searchTerm = (state.search || "").trim().toLowerCase();

  const sections = sortSectionsForDisplay(Object.entries(state.metadata || {}));

  for (const [sectionId, metadata] of sections) {
    const summary = metadata.summary ?? {};
    const label = metadata.label ?? formatSectionLabel(sectionId);
    const sectionPending = countPendingChanges(sectionId);
    const icon = SECTION_ICONS[sectionId] || "icon-settings";

    // Hide sections that don't have matching fields when searching
    if (searchTerm.length > 0 && !sectionHasMatchingFields(sectionId, searchTerm)) {
      continue;
    }

    const button = create("button", {
      type: "button",
      className: "config-section-item" + (state.activeSection === sectionId ? " active" : ""),
    });

    if (sectionPending > 0) {
      button.classList.add("pending");
    }

    const matchesSearch =
      searchTerm.length > 0 &&
      (sectionId.toLowerCase().includes(searchTerm) || label.toLowerCase().includes(searchTerm));

    if (matchesSearch) {
      button.classList.add("search-match");
    }

    const labelEl = create("div", { className: "config-section-label" });
    const metaEl = create("div", { className: "config-section-meta" });

    labelEl.innerHTML = `<i class="${icon}"></i><span>${Utils.escapeHtml(label)}</span>`;
    const totalFields = summary.total ?? Object.keys(metadata.fields || {}).length;
    const metaParts = [`<span class="config-section-count">${totalFields}</span>`];
    if (sectionPending > 0) {
      metaParts.push(`<span class="config-section-pending">+${sectionPending}</span>`);
    }
    metaEl.innerHTML = metaParts.join("");

    button.appendChild(labelEl);
    button.appendChild(metaEl);

    on(button, "click", () => {
      if (state.activeSection === sectionId) {
        return;
      }
      AppState.save(`${CONFIG_STATE_KEY}.activeSection`, sectionId);
      setState({ activeSection: sectionId });
    });

    container.appendChild(button);
  }
}

function renderToolbar(sectionId) {
  const toolbar = $("#configMainToolbar");
  if (!toolbar) {
    return;
  }
  toolbar.innerHTML = "";

  if (!sectionId) {
    hide(toolbar);
    return;
  }

  const hasChanges = hasSectionChanges(sectionId);
  const sectionPending = countPendingChanges(sectionId);
  const totalPending = state.pendingChanges.size;

  const sectionChip = create("div", { className: "config-info-chip" });
  sectionChip.innerHTML = sectionPending
    ? `<strong>${sectionPending}</strong> change${sectionPending === 1 ? "" : "s"} in section`
    : "No section changes";
  toolbar.appendChild(sectionChip);

  if (totalPending > sectionPending) {
    const globalChip = create("div", { className: "config-info-chip" });
    globalChip.innerHTML = `<strong>${totalPending}</strong> total change${totalPending === 1 ? "" : "s"}`;
    toolbar.appendChild(globalChip);
  }

  if (hasChanges) {
    const revertBtn = create("button", {
      type: "button",
      className: "config-header-action ghost",
    });
    revertBtn.textContent = "Revert section";
    on(revertBtn, "click", () => {
      revertSection(sectionId);
    });
    toolbar.appendChild(revertBtn);
  }

  show(toolbar);
}

function renderHeader(sectionId) {
  const header = $("#configMainHeader");
  if (!header) {
    return;
  }
  header.innerHTML = "";

  if (!sectionId) {
    const empty = create("div", { className: "config-section-title" });
    empty.innerHTML = "Select a configuration section";
    header.appendChild(empty);
    return;
  }

  const metadata = state.metadata?.[sectionId];
  if (!metadata) {
    const missing = create("div", { className: "config-section-title" });
    missing.innerHTML = `No metadata for <code>${Utils.escapeHtml(sectionId)}</code>`;
    header.appendChild(missing);
    return;
  }

  const title = create("div", { className: "config-section-title" });

  const iconClass = SECTION_ICONS[sectionId] || "icon-settings";
  title.innerHTML = `
    <div class="config-section-icon"><i class="${iconClass}"></i></div>
    <div class="config-section-text">
      <h2 class="config-section-name">${Utils.escapeHtml(metadata.label ?? sectionId)}</h2>
      <div class="config-section-summary">${renderSectionSummary(metadata)}</div>
    </div>
  `;

  header.appendChild(title);

  const actions = create("div", { className: "config-header-actions" });

  const saveBtn = create("button", {
    type: "button",
    className: "config-header-action primary",
    disabled: state.saving || state.pendingChanges.size === 0,
  });
  saveBtn.textContent = state.saving ? "Saving…" : "Save Changes";
  on(saveBtn, "click", handleSaveAll);
  actions.appendChild(saveBtn);

  const reloadBtn = create("button", {
    type: "button",
    className: "config-header-action ghost",
    disabled: state.loading,
  });
  reloadBtn.textContent = "Reload from Disk";
  on(reloadBtn, "click", handleReload);
  actions.appendChild(reloadBtn);

  const diffBtn = create("button", {
    type: "button",
    className: "config-header-action ghost",
  });
  diffBtn.textContent = "Compare with Disk";
  on(diffBtn, "click", handleDiff);
  actions.appendChild(diffBtn);

  const resetBtn = create("button", {
    type: "button",
    className: "config-header-action destructive",
    disabled: state.saving,
  });
  resetBtn.textContent = "Reset Section";
  on(resetBtn, "click", () => {
    revertSection(sectionId);
  });
  actions.appendChild(resetBtn);

  header.appendChild(actions);
}

function renderSectionSummary(metadata) {
  const summaryItems = [];
  if (metadata.summary) {
    if (typeof metadata.summary.total === "number") {
      summaryItems.push(
        `<span class="config-summary-badge">${metadata.summary.total} fields</span>`
      );
    }
    if (typeof metadata.summary.critical === "number" && metadata.summary.critical > 0) {
      summaryItems.push(
        `<span class="config-summary-badge warning">${metadata.summary.critical} critical</span>`
      );
    }
    if (typeof metadata.summary.performance === "number" && metadata.summary.performance > 0) {
      summaryItems.push(
        `<span class="config-summary-badge positive">${metadata.summary.performance} performance</span>`
      );
    }
  }
  const pending = countPendingChanges(metadata.id);
  if (pending > 0) {
    summaryItems.push(
      `<span class="config-summary-badge warning">${pending} pending change${pending === 1 ? "" : "s"}</span>`
    );
  }
  if (!summaryItems.length) {
    summaryItems.push('<span class="config-summary-badge">No metadata summary</span>');
  }
  return summaryItems.join("\n");
}

/**
 * Sort categories by visibility level (primary first, then secondary, then technical)
 * Within same visibility, sort alphabetically
 */
function sortCategoriesByVisibility(categories) {
  const visibilityOrder = { primary: 0, secondary: 1, technical: 2 };
  return categories.sort(([catA, fieldsA], [catB, fieldsB]) => {
    const visA = fieldsA[0]?.[1]?.visibility ?? "secondary";
    const visB = fieldsB[0]?.[1]?.visibility ?? "secondary";
    const orderDiff = visibilityOrder[visA] - visibilityOrder[visB];
    if (orderDiff !== 0) return orderDiff;
    return catA.localeCompare(catB);
  });
}

/**
 * Create a visual separator for visibility sections
 */
function createVisibilitySeparator(label) {
  const sep = create("div", { className: "config-visibility-separator" });
  sep.innerHTML = `<span>${Utils.escapeHtml(label)}</span>`;
  return sep;
}

function renderCategories(sectionId) {
  const container = $("#configCategories");
  if (!container) {
    return;
  }
  container.innerHTML = "";

  if (!sectionId || !state.metadata?.[sectionId]) {
    const empty = create("div", { className: "config-state" });
    empty.innerHTML = "Select a configuration section to view details.";
    container.appendChild(empty);
    return;
  }

  const metadata = state.metadata[sectionId];
  const fields = Object.entries(metadata.fields ?? {});

  const grouped = new Map();
  for (const [fieldKey, fieldMeta] of fields) {
    const category = fieldMeta.category ?? "General";
    if (!grouped.has(category)) {
      grouped.set(category, []);
    }
    grouped.get(category).push([fieldKey, fieldMeta]);
  }

  const searchTerm = (state.search || "").trim().toLowerCase();
  const sectionConfig = state.draft?.[sectionId] ?? {};
  const originalConfig = state.original?.[sectionId] ?? {};

  const categories = Array.from(grouped.entries());
  sortCategoriesByVisibility(categories);

  let lastVisibility = null;
  for (const [category, fieldsList] of categories) {
    // Get visibility of this category (from first field)
    const categoryVisibility = fieldsList[0]?.[1]?.visibility ?? "secondary";
    // Sort fields: simple types first, then object types (with sub-configs), alphabetically within each group
    fieldsList.sort(([keyA, metaA], [keyB, metaB]) => {
      const isObjectA = metaA.type === "object";
      const isObjectB = metaB.type === "object";
      if (isObjectA !== isObjectB) {
        return isObjectA ? 1 : -1; // Simple types first
      }
      return keyA.localeCompare(keyB);
    });

    // Add separator before technical categories
    if (categoryVisibility === "technical" && lastVisibility !== "technical") {
      container.appendChild(createVisibilitySeparator("Technical Settings"));
    }
    lastVisibility = categoryVisibility;

    // Primary visibility categories are expanded by default
    const isCollapsedDefault = categoryVisibility !== "primary";
    const categoryEl = create("div", {
      className: isCollapsedDefault ? "config-category collapsed" : "config-category",
    });
    categoryEl.dataset.visibility = categoryVisibility;

    const header = create("button", {
      type: "button",
      className: "config-category-header",
    });
    header.innerHTML = `
      <div class="config-category-label">
        <i class="chevron icon-chevron-down"></i>
        <span>${Utils.escapeHtml(category)}</span>
      </div>
      <div class="config-category-meta">
        <span class="config-category-chip">${fieldsList.length} fields</span>
      </div>
    `;

    const body = create("div", { className: "config-category-body" });

    on(header, "click", () => {
      categoryEl.classList.toggle("collapsed");
    });

    let categoryHasMatch = false;
    let pendingCount = 0;
    for (const [fieldKey, fieldMeta] of fieldsList) {
      const fieldId = `config-${sectionId}-${fieldKey}`;
      const fieldValue = sectionConfig[fieldKey];
      const fieldOriginalValue = originalConfig[fieldKey];
      const defaultValue = deepClone(fieldMeta.default);
      const fieldPath = [sectionId, fieldKey];
      const fieldPathLabel = fieldPath.join(".");

      const matchesSearch = metadataMatchesSearch(fieldKey, fieldMeta, searchTerm);

      // Hide fields that don't match when searching
      if (searchTerm.length > 0 && !matchesSearch) {
        continue;
      }

      const fieldEl = create("div", { className: "config-field" });
      if (matchesSearch) {
        fieldEl.classList.add("config-field--match");
        categoryHasMatch = true;
      }

      if (!deepEqual(fieldValue, fieldOriginalValue)) {
        fieldEl.classList.add("config-field--changed");
        pendingCount += 1;
      }

      const labelEl = create("div", { className: "config-field-label" });
      const controlEl = create("div", { className: "config-field-control" });

      const labelHtml = [];
      labelHtml.push(
        `<div class="config-field-name">${Utils.escapeHtml(fieldMeta.label || fieldKey)}</div>`
      );
      labelHtml.push(`<div class="config-field-key">${Utils.escapeHtml(fieldPathLabel)}</div>`);
      if (fieldMeta.hint) {
        labelHtml.push(`<div class="config-field-hint">${Utils.escapeHtml(fieldMeta.hint)}</div>`);
      }

      const metaItems = [];
      if (fieldMeta.unit) {
        metaItems.push(
          `<span class="config-field-unit">Unit: ${Utils.escapeHtml(fieldMeta.unit)}</span>`
        );
      }
      if (fieldMeta.impact) {
        metaItems.push(
          `<span class="config-field-impact ${Utils.escapeHtml(fieldMeta.impact.toLowerCase())}">` +
            `${Utils.escapeHtml(fieldMeta.impact)}</span>`
        );
      }
      if (fieldMeta.docs) {
        metaItems.push(`<span>Docs: ${Utils.escapeHtml(fieldMeta.docs)}</span>`);
      }
      if (metaItems.length > 0) {
        labelHtml.push(`<div class="config-field-meta">${metaItems.join(" ")}</div>`);
      }

      // Only show default value for non-object types to avoid layout issues
      if (defaultValue !== null && defaultValue !== undefined && fieldMeta.type !== "object") {
        const defaultText =
          typeof defaultValue === "object"
            ? Utils.escapeHtml(JSON.stringify(defaultValue))
            : Utils.escapeHtml(String(defaultValue));
        labelHtml.push(`<div class="config-field-default">Default: ${defaultText}</div>`);
      }

      labelEl.innerHTML = labelHtml.join("\n");

      const isAtDefault = deepEqual(fieldValue, defaultValue);

      const resetBtn = create("button", {
        type: "button",
        className: "config-field-reset",
        disabled: defaultValue === undefined ? true : isAtDefault,
      });
      resetBtn.textContent = "Reset to default";
      on(resetBtn, "click", () => {
        resetFieldToDefault(sectionId, fieldKey);
      });
      if (defaultValue === undefined) {
        resetBtn.hidden = true;
      }

      const control = renderFieldControl(fieldMeta.type, {
        fieldId,
        value: fieldValue,
        originalValue: fieldOriginalValue,
        metadata: fieldMeta,
        disabled: state.saving,
        path: fieldPath,
        searchTerm,
        onChange: (nextValue) => {
          if (!state.draft[sectionId]) {
            state.draft[sectionId] = {};
          }
          state.draft[sectionId][fieldKey] = normalizeFieldValue(fieldMeta.type, nextValue);
          const originalSection = state.original?.[sectionId] ?? {};
          markFieldChanged(
            sectionId,
            fieldKey,
            !deepEqual(state.draft[sectionId][fieldKey], originalSection[fieldKey])
          );
          render();
        },
      });

      controlEl.appendChild(control);
      controlEl.appendChild(resetBtn);

      const errorEl = create("div", { className: "config-field-error" });
      const errorKey = `${sectionId}.${fieldKey}`;
      if (state.errors.has(errorKey)) {
        fieldEl.classList.add("config-field--error");
        errorEl.textContent = state.errors.get(errorKey);
      }

      fieldEl.appendChild(labelEl);
      fieldEl.appendChild(controlEl);
      fieldEl.appendChild(errorEl);

      body.appendChild(fieldEl);
    }

    // Update chip to show pending changes if any
    const chipEl = header.querySelector(".config-category-chip");
    if (chipEl) {
      if (pendingCount > 0) {
        chipEl.classList.add("pending");
        chipEl.textContent = `${fieldsList.length} fields · ${pendingCount} pending`;
      } else {
        chipEl.classList.remove("pending");
        chipEl.textContent = `${fieldsList.length} fields`;
      }
    }

    // Check if category matches search term directly
    const categoryMatchesSearch =
      searchTerm.length > 0 && category.toLowerCase().includes(searchTerm);

    // Hide categories with no visible fields when searching
    const visibleFieldCount = body.querySelectorAll(".config-field").length;
    if (searchTerm.length > 0 && visibleFieldCount === 0 && !categoryMatchesSearch) {
      continue;
    }

    // Update chip to show visible field count when filtering
    if (searchTerm.length > 0 && visibleFieldCount !== fieldsList.length) {
      const chipEl = header.querySelector(".config-category-chip");
      if (chipEl) {
        chipEl.textContent = `${visibleFieldCount} of ${fieldsList.length} fields`;
      }
    }

    if (categoryHasMatch || categoryMatchesSearch) {
      categoryEl.classList.add("has-match");
      // Auto-expand matched categories
      categoryEl.classList.remove("collapsed");
    }

    categoryEl.appendChild(header);
    categoryEl.appendChild(body);
    container.appendChild(categoryEl);
  }

  // Render section-specific actions after categories
  renderSectionActions(sectionId, container);
}

/**
 * Render section-specific action panels (e.g., Telegram test connection)
 */
function renderSectionActions(sectionId, container) {
  if (sectionId === "telegram") {
    renderTelegramActions(container);
  }
}

/**
 * Render Telegram-specific actions (Test Connection, Authentication)
 */
async function renderTelegramActions(container) {
  const overviewHint = Hints.getHint("configTelegram.overview");
  const overviewHintHtml = overviewHint
    ? HintTrigger.render(overviewHint, "configTelegram.overview", { size: "sm" })
    : "";

  const actionsPanel = create("div", { className: "config-section-actions" });
  actionsPanel.innerHTML = `
    <div class="config-actions-header">
      <i class="icon-send"></i>
      <span>Actions</span>
      ${overviewHintHtml}
    </div>
    <div class="config-actions-body">
      <div class="config-action-item">
        <div class="config-action-info">
          <div class="config-action-title">Test Connection</div>
          <div class="config-action-desc">Send a test message to verify your Telegram configuration is working</div>
        </div>
        <button type="button" class="btn primary" id="telegram-test-btn" disabled title="Loading...">
          <i class="icon-loader spin"></i> Loading...
        </button>
      </div>
      <div class="config-action-status" id="telegram-status" role="status" aria-live="polite"></div>
    </div>
  `;

  container.appendChild(actionsPanel);

  // Wire up test button
  const testBtn = actionsPanel.querySelector("#telegram-test-btn");
  const statusEl = actionsPanel.querySelector("#telegram-status");

  // Check if Telegram is configured before enabling test button
  try {
    const response = await fetch("/api/telegram/status");
    const data = await response.json();
    const isConfigured = data.data?.bot_configured;

    if (isConfigured) {
      testBtn.disabled = false;
      testBtn.title = "";
      testBtn.innerHTML = '<i class="icon-send"></i> Send Test Message';
    } else {
      testBtn.disabled = true;
      testBtn.title = "Configure bot token first";
      testBtn.innerHTML = '<i class="icon-send"></i> Send Test Message';
      statusEl.className = "config-action-status info";
      statusEl.innerHTML = '<i class="icon-info"></i> Configure bot token above to enable testing';
    }
  } catch {
    testBtn.disabled = false;
    testBtn.title = "";
    testBtn.innerHTML = '<i class="icon-send"></i> Send Test Message';
  }

  on(testBtn, "click", async () => {
    testBtn.disabled = true;
    testBtn.innerHTML = '<i class="icon-loader spin"></i> Sending...';
    statusEl.className = "config-action-status";
    statusEl.textContent = "";

    try {
      const response = await fetch("/api/telegram/test", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
      });
      const data = await response.json();

      if (response.ok && data.success) {
        statusEl.className = "config-action-status success";
        statusEl.innerHTML =
          '<i class="icon-circle-check"></i> Test message sent successfully! Check your Telegram.';
        Utils.showToast("Telegram test message sent", "success");
      } else {
        throw new Error(data.message || data.error || "Failed to send test message");
      }
    } catch (error) {
      statusEl.className = "config-action-status error";
      statusEl.innerHTML = `<i class="icon-circle-alert"></i> ${Utils.escapeHtml(error.message)}`;
      Utils.showToast(error.message, "error");
    } finally {
      testBtn.disabled = false;
      testBtn.innerHTML = '<i class="icon-send"></i> Send Test Message';
    }
  });

  // Render authentication section
  renderTelegramAuthSection(container);

  // Initialize hint triggers after all sections are added
  HintTrigger.initAll();
}

/**
 * Render Telegram Authentication Section (Password + TOTP 2FA)
 */
async function renderTelegramAuthSection(container) {
  const passwordHint = Hints.getHint("configTelegram.password");
  const passwordHintHtml = passwordHint
    ? HintTrigger.render(passwordHint, "configTelegram.password", { size: "sm" })
    : "";

  const totpHint = Hints.getHint("configTelegram.totp");
  const totpHintHtml = totpHint
    ? HintTrigger.render(totpHint, "configTelegram.totp", { size: "sm" })
    : "";

  const authPanel = create("div", { className: "config-section-actions telegram-auth-section" });
  authPanel.innerHTML = `
    <div class="config-actions-header">
      <i class="icon-shield"></i>
      <span>Bot Authentication</span>
    </div>
    <div class="config-actions-body">
      <!-- Password Section -->
      <div class="telegram-auth-subsection">
        <div class="telegram-auth-header">
          <div class="telegram-auth-title">
            <i class="icon-lock"></i>
            <span>Password</span>
            ${passwordHintHtml}
          </div>
          <div class="telegram-auth-status" id="telegram-password-status" role="status" aria-live="polite">
            <i class="icon-loader spin"></i> Loading...
          </div>
        </div>
        <div class="telegram-auth-content" id="telegram-password-content"></div>
      </div>

      <!-- TOTP 2FA Section -->
      <div class="telegram-auth-subsection" id="telegram-totp-section" style="display: none;">
        <div class="telegram-auth-header">
          <div class="telegram-auth-title">
            <i class="icon-key"></i>
            <span>Two-Factor Authentication</span>
            ${totpHintHtml}
          </div>
          <div class="telegram-auth-status" id="telegram-totp-status" role="status" aria-live="polite">
            <i class="icon-loader spin"></i> Loading...
          </div>
        </div>
        <div class="telegram-auth-content" id="telegram-totp-content"></div>
      </div>
    </div>
  `;

  container.appendChild(authPanel);

  // Load initial auth state
  await loadTelegramAuthState(authPanel);
}

/**
 * Load Telegram authentication state and render UI
 */
async function loadTelegramAuthState(authPanel) {
  const passwordStatusEl = authPanel.querySelector("#telegram-password-status");
  const passwordContentEl = authPanel.querySelector("#telegram-password-content");
  const totpSection = authPanel.querySelector("#telegram-totp-section");
  const totpStatusEl = authPanel.querySelector("#telegram-totp-status");
  const totpContentEl = authPanel.querySelector("#telegram-totp-content");

  try {
    // Fetch current settings to check password_configured
    const settingsResponse = await fetch("/api/telegram/settings");
    const settingsData = await settingsResponse.json();

    if (!settingsResponse.ok) {
      throw new Error(settingsData.error || "Failed to load settings");
    }

    const passwordConfigured = settingsData.data?.password_configured || false;
    const totpEnabled = settingsData.data?.totp_enabled || false;

    // Render password section
    renderPasswordSection(passwordStatusEl, passwordContentEl, passwordConfigured);

    // Show TOTP section only if password is configured
    if (passwordConfigured) {
      totpSection.style.display = "";
      renderTotpSection(totpStatusEl, totpContentEl, totpEnabled);
    } else {
      totpSection.style.display = "none";
    }
  } catch (error) {
    passwordStatusEl.innerHTML = "<span class=\"status-error\"><i class=\"icon-circle-alert\"></i> Error</span>";
    passwordContentEl.innerHTML = `<div class="telegram-auth-error">${Utils.escapeHtml(error.message)}</div>`;
  }
}

/**
 * Render password configuration section
 */
function renderPasswordSection(statusEl, contentEl, isConfigured) {
  if (isConfigured) {
    statusEl.innerHTML = "<span class=\"status-success\"><i class=\"icon-circle-check\"></i> Configured</span>";
    contentEl.innerHTML = `
      <div class="telegram-auth-row">
        <div class="telegram-auth-info">
          <span>Password is set and active for Telegram bot authentication.</span>
        </div>
        <button type="button" class="btn secondary" id="telegram-change-password-btn">
          <i class="icon-pencil"></i> Change Password
        </button>
      </div>
      <div class="telegram-auth-form-container" id="telegram-password-form-container" style="display: none;"></div>
    `;

    const changeBtn = contentEl.querySelector("#telegram-change-password-btn");
    const formContainer = contentEl.querySelector("#telegram-password-form-container");

    on(changeBtn, "click", () => {
      const isVisible = formContainer.style.display !== "none";
      if (isVisible) {
        formContainer.style.display = "none";
        changeBtn.innerHTML = '<i class="icon-pencil"></i> Change Password';
      } else {
        formContainer.style.display = "";
        changeBtn.innerHTML = '<i class="icon-x"></i> Cancel';
        renderPasswordForm(formContainer, true);
      }
    });
  } else {
    statusEl.innerHTML = "<span class=\"status-warning\"><i class=\"icon-circle-alert\"></i> Not Set</span>";
    contentEl.innerHTML = `
      <div class="telegram-auth-row">
        <div class="telegram-auth-info">
          <span>Set a password to enable Telegram bot authentication.</span>
        </div>
        <button type="button" class="btn primary" id="telegram-set-password-btn">
          <i class="icon-lock"></i> Set Password
        </button>
      </div>
      <div class="telegram-auth-form-container" id="telegram-password-form-container" style="display: none;"></div>
    `;

    const setBtn = contentEl.querySelector("#telegram-set-password-btn");
    const formContainer = contentEl.querySelector("#telegram-password-form-container");

    on(setBtn, "click", () => {
      const isVisible = formContainer.style.display !== "none";
      if (isVisible) {
        formContainer.style.display = "none";
        setBtn.innerHTML = '<i class="icon-lock"></i> Set Password';
      } else {
        formContainer.style.display = "";
        setBtn.innerHTML = '<i class="icon-x"></i> Cancel';
        renderPasswordForm(formContainer, false);
      }
    });
  }
}

/**
 * Render password form
 */
function renderPasswordForm(formContainer, hasExistingPassword) {
  formContainer.innerHTML = `
    <form class="telegram-auth-form" id="telegram-password-form">
      ${
        hasExistingPassword
          ? `
        <div class="form-group">
          <label for="telegram-current-password">Current Password <span class="required">*</span></label>
          <input type="password" id="telegram-current-password" placeholder="Enter current password" required autocomplete="current-password" />
        </div>
      `
          : ""
      }
      <div class="form-group">
        <label for="telegram-new-password">New Password <span class="required">*</span></label>
        <input type="password" id="telegram-new-password" placeholder="Minimum 8 characters" minlength="8" required autocomplete="new-password" />
      </div>
      <div class="form-group">
        <label for="telegram-confirm-password">Confirm Password <span class="required">*</span></label>
        <input type="password" id="telegram-confirm-password" placeholder="Confirm your password" required autocomplete="new-password" />
      </div>
      <div class="telegram-auth-form-error" id="telegram-password-error" role="alert" aria-live="polite"></div>
      <div class="telegram-auth-form-actions">
        <button type="submit" class="btn primary" id="telegram-password-submit">
          <i class="icon-check"></i> ${hasExistingPassword ? "Update Password" : "Set Password"}
        </button>
      </div>
    </form>
  `;

  const form = formContainer.querySelector("#telegram-password-form");
  const errorEl = formContainer.querySelector("#telegram-password-error");
  const submitBtn = formContainer.querySelector("#telegram-password-submit");

  // Clear errors on input
  on(form, "input", () => {
    errorEl.textContent = "";
  });

  on(form, "submit", async (e) => {
    e.preventDefault();
    // Race condition guard
    if (form.dataset.submitting === "true") return;
    form.dataset.submitting = "true";
    errorEl.textContent = "";

    const newPassword = form.querySelector("#telegram-new-password").value;
    const confirmPassword = form.querySelector("#telegram-confirm-password").value;
    const currentPasswordEl = form.querySelector("#telegram-current-password");
    const currentPassword = currentPasswordEl ? currentPasswordEl.value : undefined;

    // Validation
    if (newPassword.length < 8) {
      errorEl.textContent = "Password must be at least 8 characters long.";
      return;
    }

    if (newPassword !== confirmPassword) {
      errorEl.textContent = "Passwords do not match.";
      return;
    }

    submitBtn.disabled = true;
    submitBtn.innerHTML = '<i class="icon-loader spin"></i> Saving...';

    try {
      const body = { password: newPassword };
      if (currentPassword) {
        body.current_password = currentPassword;
      }

      const response = await fetch("/api/telegram/password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });

      const data = await response.json();

      if (response.ok && data.success) {
        Utils.showToast("Password updated successfully", "success");
        // Refresh the entire auth section
        const authPanel = formContainer.closest(".telegram-auth-section");
        if (authPanel) {
          await loadTelegramAuthState(authPanel);
        }
      } else {
        throw new Error(data.error || data.message || "Failed to set password");
      }
    } catch (error) {
      errorEl.textContent = error.message;
    } finally {
      form.dataset.submitting = "false";
      submitBtn.disabled = false;
      submitBtn.innerHTML = `<i class="icon-check"></i> ${hasExistingPassword ? "Update Password" : "Set Password"}`;
    }
  });
}

/**
 * Render TOTP 2FA section
 */
function renderTotpSection(statusEl, contentEl, isEnabled) {
  if (isEnabled) {
    statusEl.innerHTML = "<span class=\"status-success\"><i class=\"icon-circle-check\"></i> Enabled</span>";
    contentEl.innerHTML = `
      <div class="telegram-auth-row">
        <div class="telegram-auth-info">
          <span>Two-factor authentication is active. You'll need your authenticator app to use Telegram commands.</span>
        </div>
        <button type="button" class="btn danger" id="telegram-disable-totp-btn">
          <i class="icon-x"></i> Disable 2FA
        </button>
      </div>
      <div class="telegram-auth-form-container" id="telegram-totp-form-container" style="display: none;"></div>
    `;

    const disableBtn = contentEl.querySelector("#telegram-disable-totp-btn");
    const formContainer = contentEl.querySelector("#telegram-totp-form-container");

    on(disableBtn, "click", () => {
      const isVisible = formContainer.style.display !== "none";
      if (isVisible) {
        formContainer.style.display = "none";
        disableBtn.innerHTML = '<i class="icon-x"></i> Disable 2FA';
      } else {
        formContainer.style.display = "";
        disableBtn.innerHTML = '<i class="icon-x"></i> Cancel';
        renderTotpDisableForm(formContainer);
      }
    });
  } else {
    statusEl.innerHTML = "<span class=\"status-warning\"><i class=\"icon-circle-alert\"></i> Not Configured</span>";
    contentEl.innerHTML = `
      <div class="telegram-auth-row">
        <div class="telegram-auth-info">
          <span>Add an extra layer of security by enabling two-factor authentication with Google Authenticator or similar apps.</span>
        </div>
        <button type="button" class="btn primary" id="telegram-enable-totp-btn">
          <i class="icon-shield"></i> Enable 2FA
        </button>
      </div>
      <div class="telegram-auth-form-container" id="telegram-totp-form-container" style="display: none;"></div>
    `;

    const enableBtn = contentEl.querySelector("#telegram-enable-totp-btn");
    const formContainer = contentEl.querySelector("#telegram-totp-form-container");

    on(enableBtn, "click", () => {
      const isVisible = formContainer.style.display !== "none";
      if (isVisible) {
        formContainer.style.display = "none";
        enableBtn.innerHTML = '<i class="icon-shield"></i> Enable 2FA';
      } else {
        formContainer.style.display = "";
        enableBtn.innerHTML = '<i class="icon-x"></i> Cancel';
        renderTotpSetupForm(formContainer);
      }
    });
  }
}

/**
 * Render TOTP setup form (Step 1: Enter password to generate secret)
 */
function renderTotpSetupForm(formContainer) {
  formContainer.innerHTML = `
    <form class="telegram-auth-form" id="telegram-totp-setup-form">
      <div class="telegram-totp-step" role="group" aria-labelledby="totp-step-1-title">
        <div class="telegram-totp-step-header">
          <span class="telegram-totp-step-number" aria-hidden="true">1</span>
          <span id="totp-step-1-title" class="telegram-totp-step-title">Verify your password</span>
        </div>
        <div class="form-group">
          <label for="telegram-totp-password">Password <span class="required">*</span></label>
          <input type="password" id="telegram-totp-password" placeholder="Enter your password" required autocomplete="current-password" />
        </div>
      </div>
      <div class="telegram-auth-form-error" id="telegram-totp-setup-error"></div>
      <div class="telegram-auth-form-actions">
        <button type="submit" class="btn primary" id="telegram-totp-setup-submit">
          <i class="icon-arrow-right"></i> Continue
        </button>
      </div>
    </form>
  `;

  const form = formContainer.querySelector("#telegram-totp-setup-form");
  const errorEl = formContainer.querySelector("#telegram-totp-setup-error");
  const submitBtn = formContainer.querySelector("#telegram-totp-setup-submit");

  // Clear errors on input
  on(form, "input", () => {
    errorEl.textContent = "";
  });

  on(form, "submit", async (e) => {
    e.preventDefault();
    // Race condition guard
    if (form.dataset.submitting === "true") return;
    form.dataset.submitting = "true";
    errorEl.textContent = "";

    const password = form.querySelector("#telegram-totp-password").value;

    submitBtn.disabled = true;
    submitBtn.innerHTML = '<i class="icon-loader spin"></i> Generating...';

    try {
      const response = await fetch("/api/telegram/totp/setup", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ password }),
      });

      const data = await response.json();

      if (response.ok && data.success) {
        // Show QR code and verification step
        renderTotpVerifyForm(formContainer, data.data);
      } else {
        throw new Error(data.error || data.message || "Failed to generate 2FA secret");
      }
    } catch (error) {
      errorEl.textContent = error.message;
      submitBtn.disabled = false;
      submitBtn.innerHTML = '<i class="icon-arrow-right"></i> Continue';
    } finally {
      form.dataset.submitting = "false";
    }
  });
}

/**
 * Render TOTP verification form (Step 2: Scan QR and verify code)
 */
function renderTotpVerifyForm(formContainer, totpData) {
  const { secret, qr_code } = totpData;

  // Validate QR code is a safe data URL (XSS prevention)
  const safeQrCode = qr_code && qr_code.startsWith("data:image/") ? qr_code : "";

  formContainer.innerHTML = `
    <div class="telegram-totp-setup-complete">
      <div class="telegram-totp-step" role="group" aria-labelledby="totp-step-2-title">
        <div class="telegram-totp-step-header">
          <span class="telegram-totp-step-number" aria-hidden="true">2</span>
          <span id="totp-step-2-title" class="telegram-totp-step-title">Scan QR code with your authenticator app</span>
        </div>
        <div class="telegram-totp-qr-container">
          ${safeQrCode ? `<img src="${safeQrCode}" alt="TOTP QR Code" class="telegram-totp-qr-code" />` : '<div class="telegram-totp-qr-error">QR code unavailable</div>'}
        </div>
        <div class="telegram-totp-secret-container">
          <label>Or enter this code manually:</label>
          <div class="telegram-totp-secret">
            <code id="telegram-totp-secret-code">${Utils.escapeHtml(secret)}</code>
            <button type="button" class="btn icon-btn" id="telegram-copy-secret-btn" title="Copy to clipboard">
              <i class="icon-copy"></i>
            </button>
          </div>
        </div>
      </div>

      <form class="telegram-auth-form" id="telegram-totp-verify-form">
        <div class="telegram-totp-step" role="group" aria-labelledby="totp-step-3-title">
          <div class="telegram-totp-step-header">
            <span class="telegram-totp-step-number" aria-hidden="true">3</span>
            <span id="totp-step-3-title" class="telegram-totp-step-title">Enter verification code</span>
          </div>
          <div class="form-group">
            <label for="telegram-totp-code">6-Digit Code <span class="required">*</span></label>
            <input type="text" id="telegram-totp-code" inputmode="numeric" placeholder="000000" maxlength="6" pattern="[0-9]{6}" required autocomplete="one-time-code" />
          </div>
        </div>
        <div class="telegram-auth-form-error" id="telegram-totp-verify-error"></div>
        <div class="telegram-auth-form-actions">
          <button type="submit" class="btn primary" id="telegram-totp-verify-submit">
            <i class="icon-check"></i> Verify & Enable 2FA
          </button>
        </div>
      </form>
    </div>
  `;

  // Copy secret button
  const copyBtn = formContainer.querySelector("#telegram-copy-secret-btn");
  on(copyBtn, "click", async () => {
    try {
      await navigator.clipboard.writeText(secret);
      Utils.showToast("Secret copied to clipboard", "success");
    } catch {
      Utils.showToast("Failed to copy secret", "error");
    }
  });

  // Verify form
  const form = formContainer.querySelector("#telegram-totp-verify-form");
  const errorEl = formContainer.querySelector("#telegram-totp-verify-error");
  const submitBtn = formContainer.querySelector("#telegram-totp-verify-submit");
  const codeInput = formContainer.querySelector("#telegram-totp-code");

  // Auto-focus and format code input
  codeInput.focus();
  on(codeInput, "input", () => {
    codeInput.value = codeInput.value.replace(/\D/g, "").slice(0, 6);
    errorEl.textContent = ""; // Clear errors on input
  });

  on(form, "submit", async (e) => {
    e.preventDefault();
    // Race condition guard
    if (form.dataset.submitting === "true") return;
    form.dataset.submitting = "true";
    errorEl.textContent = "";

    const code = codeInput.value;

    if (!/^\d{6}$/.test(code)) {
      errorEl.textContent = "Please enter a valid 6-digit code.";
      return;
    }

    submitBtn.disabled = true;
    submitBtn.innerHTML = '<i class="icon-loader spin"></i> Verifying...';

    try {
      const response = await fetch("/api/telegram/totp/verify", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ code }),
      });

      const data = await response.json();

      if (response.ok && data.success) {
        Utils.showToast("Two-factor authentication enabled!", "success");
        // Refresh the entire auth section
        const authPanel = formContainer.closest(".telegram-auth-section");
        if (authPanel) {
          await loadTelegramAuthState(authPanel);
        }
      } else {
        throw new Error(data.error || data.message || "Invalid verification code");
      }
    } catch (error) {
      errorEl.textContent = error.message;
      submitBtn.disabled = false;
      submitBtn.innerHTML = '<i class="icon-check"></i> Verify & Enable 2FA';
    } finally {
      form.dataset.submitting = "false";
    }
  });
}

/**
 * Render TOTP disable form
 */
function renderTotpDisableForm(formContainer) {
  formContainer.innerHTML = `
    <form class="telegram-auth-form" id="telegram-totp-disable-form">
      <div class="telegram-totp-warning" role="alert">
        <i class="icon-triangle-alert"></i>
        <span>Disabling 2FA will make your Telegram bot less secure. You can always re-enable it later.</span>
      </div>
      <div class="form-group">
        <label for="telegram-disable-password">Password <span class="required">*</span></label>
        <input type="password" id="telegram-disable-password" placeholder="Enter your password to confirm" required autocomplete="current-password" />
      </div>
      <div class="form-group form-group-checkbox">
        <label class="checkbox-label">
          <input type="checkbox" id="telegram-disable-confirm" required />
          <span>I understand this will reduce the security of my Telegram bot</span>
        </label>
      </div>
      <div class="telegram-auth-form-error" id="telegram-totp-disable-error" role="alert" aria-live="polite"></div>
      <div class="telegram-auth-form-actions">
        <button type="submit" class="btn danger" id="telegram-totp-disable-submit" disabled>
          <i class="icon-x"></i> Disable 2FA
        </button>
      </div>
    </form>
  `;

  const form = formContainer.querySelector("#telegram-totp-disable-form");
  const errorEl = formContainer.querySelector("#telegram-totp-disable-error");
  const submitBtn = formContainer.querySelector("#telegram-totp-disable-submit");
  const confirmCheckbox = formContainer.querySelector("#telegram-disable-confirm");

  // Enable/disable submit based on checkbox
  on(confirmCheckbox, "change", () => {
    submitBtn.disabled = !confirmCheckbox.checked;
  });

  // Clear errors on input
  on(form, "input", () => {
    errorEl.textContent = "";
  });

  on(form, "submit", async (e) => {
    e.preventDefault();
    // Race condition guard
    if (form.dataset.submitting === "true") return;
    form.dataset.submitting = "true";
    errorEl.textContent = "";

    const password = form.querySelector("#telegram-disable-password").value;

    submitBtn.disabled = true;
    submitBtn.innerHTML = '<i class="icon-loader spin"></i> Disabling...';

    try {
      const response = await fetch("/api/telegram/totp/disable", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ password }),
      });

      const data = await response.json();

      if (response.ok && data.success) {
        Utils.showToast("Two-factor authentication disabled", "success");
        // Refresh the entire auth section
        const authPanel = formContainer.closest(".telegram-auth-section");
        if (authPanel) {
          await loadTelegramAuthState(authPanel);
        }
      } else {
        throw new Error(data.error || data.message || "Failed to disable 2FA");
      }
    } catch (error) {
      errorEl.textContent = error.message;
      submitBtn.disabled = false;
      submitBtn.innerHTML = '<i class="icon-x"></i> Disable 2FA';
    } finally {
      form.dataset.submitting = "false";
    }
  });
}

function render() {
  const reloadButton = $("#configReloadButton");
  if (reloadButton) {
    reloadButton.disabled = state.loading || state.saving;
  }
  const resetButton = $("#configResetButton");
  if (resetButton) {
    resetButton.disabled = state.saving;
  }

  renderSidebar();
  renderHeader(state.activeSection);
  renderToolbar(state.activeSection);
  renderStateMessage();
  renderCategories(state.activeSection);
}

function revertSection(sectionId) {
  if (!state.original?.[sectionId]) {
    return;
  }
  const originalSection = deepClone(state.original[sectionId]);
  state.draft[sectionId] = deepClone(originalSection);
  for (const key of Object.keys(originalSection)) {
    markFieldChanged(sectionId, key, false);
  }
  render();
}

async function handleSaveAll() {
  if (state.saving || state.pendingChanges.size === 0) {
    return;
  }
  setState({ saving: true });

  try {
    const updates = {};
    for (const key of state.pendingChanges.keys()) {
      const [sectionId, ...fieldParts] = key.split(".");
      if (!sectionId || fieldParts.length === 0) {
        continue;
      }
      const fieldKey = fieldParts.join(".");
      if (!updates[sectionId]) {
        updates[sectionId] = {};
      }
      updates[sectionId][fieldKey] = state.draft[sectionId][fieldKey];
    }

    for (const [sectionId, payload] of Object.entries(updates)) {
      await requestManager.fetch(`/api/config/${sectionId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
        priority: "high",
      });
    }

    Utils.showToast({
      type: "success",
      title: "Configuration Updated",
      message: "Settings saved successfully",
    });
    await loadConfig();
  } catch (error) {
    console.error("[Config] Save failed", error);
    Utils.showToast({
      type: "error",
      title: "Save Failed",
      message: error.message || "Failed to save configuration",
    });
  } finally {
    setState({ saving: false });
  }
}

async function handleReload() {
  if (state.loading) {
    return;
  }
  setState({ loading: true });
  try {
    await requestManager.fetch("/api/config/reload", {
      method: "POST",
      priority: "high",
    });
    Utils.showToast({
      type: "success",
      title: "Configuration Reloaded",
      message: "Settings reloaded from disk successfully",
    });
    await loadConfig();
  } catch (error) {
    console.error("[Config] Reload failed", error);
    Utils.showToast({
      type: "error",
      title: "Reload Failed",
      message: error.message || "Failed to reload configuration",
    });
  } finally {
    setState({ loading: false });
  }
}

async function handleDiff() {
  try {
    const payload = await requestManager.fetch("/api/config/diff", {
      priority: "normal",
    });
    const message = payload?.message ?? payload?.error?.message;
    Utils.showToast({
      type: "info",
      title: "Configuration Diff",
      message: message || "Diff calculation complete",
    });
    console.info("Config diff:", payload);
  } catch (error) {
    console.error("[Config] Diff failed", error);
    Utils.showToast({
      type: "error",
      title: "Diff Failed",
      message: error.message || "Failed to calculate diff",
    });
  }
}

async function handleResetToDefaults() {
  const { confirmed } = await ConfirmationDialog.show({
    title: "Reset Configuration",
    message:
      "This will reset the entire configuration to embedded default values. All current settings will be lost.\n\nThis action cannot be undone.",
    confirmLabel: "Reset to Defaults",
    cancelLabel: "Cancel",
    variant: "danger",
  });

  if (!confirmed) {
    return;
  }
  try {
    await requestManager.fetch("/api/config/reset", {
      method: "POST",
      priority: "high",
    });
    Utils.showToast({
      type: "warning",
      title: "Configuration Reset",
      message: "All settings restored to default values",
    });
    await loadConfig();
  } catch (error) {
    console.error("[Config] Reset failed", error);
    Utils.showToast({
      type: "error",
      title: "Reset Failed",
      message: error.message || "Failed to reset configuration",
    });
  }
}

async function loadMetadata() {
  const payload = await requestManager.fetch("/api/config/metadata", {
    priority: "normal",
  });
  state.metadata = transformMetadata(payload?.data ?? {});
  ensureActiveSectionValid();
}

async function loadConfig() {
  try {
    setState({ loading: true });
    const payload = await requestManager.fetch("/api/config", {
      priority: "normal",
    });

    const configData = { ...payload };
    delete configData.timestamp;

    state.config = configData;
    state.original = deepClone(configData);
    state.draft = deepClone(configData);
    state.pendingChanges.clear();
    state.errors.clear();

    ensureActiveSectionValid();
    render();
  } catch (error) {
    console.error("[Config] Load failed", error);
    Utils.showToast({
      type: "error",
      title: "Load Failed",
      message: error.message || "Failed to load configuration from server",
    });
  } finally {
    setState({ loading: false });
  }
}

function attachEventHandlers(ctx) {
  const searchInput = $("#configSearchInput");
  if (searchInput) {
    searchInput.value = state.search;
    const handler = (event) => {
      const value = event.target.value;
      AppState.save(`${CONFIG_STATE_KEY}.search`, value);
      state.search = value;
      render();
    };
    on(searchInput, "input", handler);
    // Press Enter to focus the first matched field if any
    const enterHandler = (event) => {
      if (event.key === "Enter") {
        const firstMatch = document.querySelector(
          ".config-field.config-field--match input, .config-field.config-field--match textarea, .config-field.config-field--match button, .config-section-item.search-match"
        );
        if (firstMatch) {
          firstMatch.focus();
        }
      }
    };
    on(searchInput, "keydown", enterHandler);
    ctx.onDispose(() => off(searchInput, "input", handler));
    ctx.onDispose(() => off(searchInput, "keydown", enterHandler));
  }

  const reloadButton = $("#configReloadButton");
  if (reloadButton) {
    const handler = () => {
      if (!state.loading) {
        handleReload();
      }
    };
    on(reloadButton, "click", handler);
    ctx.onDispose(() => off(reloadButton, "click", handler));
  }

  const resetButton = $("#configResetButton");
  if (resetButton) {
    const handler = () => {
      if (!state.saving) {
        handleResetToDefaults();
      }
    };
    on(resetButton, "click", handler);
    ctx.onDispose(() => off(resetButton, "click", handler));
  }
}

function activate() {
  ensureActiveSectionValid();
  render();
}

function deactivate() {}

async function init(ctx) {
  attachEventHandlers(ctx);

  if (!state.metadata) {
    try {
      await loadMetadata();
    } catch (error) {
      console.error("[Config] Metadata load failed", error);
      Utils.showToast({
        type: "error",
        title: "Metadata Load Failed",
        message: error.message || "Failed to load configuration metadata",
      });
      return;
    }
  }

  await loadConfig();
  render();
}

registerPage("config", {
  init,
  activate,
  deactivate,
});
