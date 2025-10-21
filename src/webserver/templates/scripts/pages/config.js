import { registerPage } from "../core/lifecycle.js";
import { $, on, off, create, show, hide } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";

const CONFIG_STATE_KEY = "config.page";
const DEFAULT_SECTION = "trader";

const SECTION_ICONS = {
  rpc: "🛰️",
  trader: "💼",
  positions: "📊",
  filtering: "🎯",
  swaps: "🔁",
  tokens: "🪙",
  sol_price: "🌞",
  events: "📡",
  webserver: "🕸️",
  services: "🔧",
  monitoring: "📈",
  ohlcv: "🕒",
  summary: "🧾",
};

const SECTION_LABEL_OVERRIDES = {
  rpc: "RPC",
  trader: "Trader",
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
};

const SECTION_DISPLAY_ORDER = [
  "rpc",
  "trader",
  "positions",
  "filtering",
  "swaps",
  "tokens",
  "sol_price",
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
    if (metadata.docs || metadata.placeholder || (typeof value === "string" && value.length > 120)) {
      const textarea = create("textarea", {
        id: fieldId,
        value: value ?? "",
        placeholder: metadata.placeholder ?? "",
        disabled,
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
          Utils.showToast(message, "error");
          event.target.dataset.arrayInvalidToastTs = String(now);
        }
        event.target.setAttribute("title", message);
      }
    });
    return textarea;
  },
  object({ fieldId, value, metadata = {}, disabled, onChange }) {
    const textarea = create("textarea", {
      id: fieldId,
      value: JSON.stringify(value ?? {}, null, 2),
      disabled,
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
        Utils.showToast(`Invalid JSON: ${error.message}`, "error");
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
    return Object.fromEntries(
      Object.entries(value).map(([key, val]) => [key, deepClone(val)])
    );
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

function transformMetadata(raw) {
  const sections = {};
  for (const [sectionId, fields] of Object.entries(raw || {})) {
    const normalizedFields = {};
    for (const [fieldKey, fieldMeta] of Object.entries(fields || {})) {
      normalizedFields[fieldKey] = {
        ...fieldMeta,
        type: fieldMeta.type || "string",
        default: deepClone(fieldMeta.default ?? null),
      };
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
  const sectionIds = sortSectionsForDisplay(Object.entries(metadata)).map(([sectionId]) => sectionId);
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
    banner.innerHTML = "<strong>Validation issues detected.</strong> Please review highlighted fields.";
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
    const icon = SECTION_ICONS[sectionId] || "⚙️";
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

    labelEl.innerHTML = `<span class="icon">${icon}</span><span>${Utils.escapeHtml(label)}</span>`;
    const totalFields = summary.total ?? Object.keys(metadata.fields || {}).length;
    const metaParts = [`<span class="config-section-count">${totalFields}</span>`];
    if (sectionPending > 0) {
      metaParts.push(
        `<span class="config-section-pending">+${sectionPending}</span>`
      );
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

  title.innerHTML = `
    <div class="config-section-icon">${SECTION_ICONS[sectionId] || "⚙️"}</div>
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
      summaryItems.push(`<span class="config-summary-badge">${metadata.summary.total} fields</span>`);
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
    summaryItems.push(`<span class="config-summary-badge">No metadata summary</span>`);
  }
  return summaryItems.join("\n");
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
  categories.sort(([a], [b]) => a.localeCompare(b));

  for (const [category, fieldsList] of categories) {
    fieldsList.sort(([keyA], [keyB]) => keyA.localeCompare(keyB));

    const categoryEl = create("div", { className: "config-category" });
    const header = create("button", {
      type: "button",
      className: "config-category-header",
    });
    header.innerHTML = `
      <div class="config-category-label">
        <span class="chevron">▶</span>
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
      const originalValue = originalConfig[fieldKey];
  const defaultValue = deepClone(fieldMeta.default);

      const matchesSearch =
        searchTerm.length > 0 &&
        (fieldKey.toLowerCase().includes(searchTerm) ||
          (fieldMeta.label && fieldMeta.label.toLowerCase().includes(searchTerm)) ||
          (fieldMeta.hint && fieldMeta.hint.toLowerCase().includes(searchTerm)));

      const fieldEl = create("div", { className: "config-field" });
      if (matchesSearch) {
        fieldEl.classList.add("config-field--match");
        categoryHasMatch = true;
      }

      if (!deepEqual(fieldValue, originalValue)) {
        fieldEl.classList.add("config-field--changed");
        pendingCount += 1;
      }

      const labelEl = create("div", { className: "config-field-label" });
      const controlEl = create("div", { className: "config-field-control" });

      const labelHtml = [];
      labelHtml.push(`<div class="config-field-name">${Utils.escapeHtml(fieldMeta.label || fieldKey)}</div>`);
      labelHtml.push(`<div class="config-field-key">${sectionId}.${fieldKey}</div>`);
      if (fieldMeta.hint) {
        labelHtml.push(`<div class="config-field-hint">${Utils.escapeHtml(fieldMeta.hint)}</div>`);
      }

      const metaItems = [];
      if (fieldMeta.unit) {
        metaItems.push(`<span class="config-field-unit">Unit: ${Utils.escapeHtml(fieldMeta.unit)}</span>`);
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

      if (defaultValue !== null && defaultValue !== undefined) {
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
        metadata: fieldMeta,
        disabled: state.saving,
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

    if (categoryHasMatch) {
      categoryEl.classList.add("has-match");
      // Auto-expand matched categories if collapsed by default
      categoryEl.classList.remove("collapsed");
    } else if (searchTerm.length > 0) {
      // When searching, collapse categories without matches to reduce scroll
      categoryEl.classList.add("collapsed");
    }

    categoryEl.appendChild(header);
    categoryEl.appendChild(body);
    container.appendChild(categoryEl);
  }
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
      const response = await fetch(`/api/config/${sectionId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`Failed to save ${sectionId}: ${errorText}`);
      }
    }

    Utils.showToast("Configuration updated", "success");
    await loadConfig();
  } catch (error) {
    console.error("[Config] Save failed", error);
    Utils.showToast(error.message || "Failed to save configuration", "error");
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
    const response = await fetch("/api/config/reload", { method: "POST" });
    if (!response.ok) {
      throw new Error(`Reload failed: HTTP ${response.status}`);
    }
    Utils.showToast("Config reloaded from disk", "success");
    await loadConfig();
  } catch (error) {
    console.error("[Config] Reload failed", error);
    Utils.showToast(error.message || "Reload failed", "error");
  } finally {
    setState({ loading: false });
  }
}

async function handleDiff() {
  try {
    const response = await fetch("/api/config/diff");
    if (!response.ok) {
      throw new Error(`Diff failed: HTTP ${response.status}`);
    }
    const payload = await response.json();
    const message = payload?.message ?? payload?.error?.message;
    Utils.showToast(message || "Diff ready", "info");
    console.info("Config diff:", payload);
  } catch (error) {
    console.error("[Config] Diff failed", error);
    Utils.showToast(error.message || "Diff failed", "error");
  }
}

async function handleResetToDefaults() {
  if (!window.confirm("Reset full config to embedded defaults?")) {
    return;
  }
  try {
    const response = await fetch("/api/config/reset", { method: "POST" });
    if (!response.ok) {
      throw new Error(`Reset failed: HTTP ${response.status}`);
    }
    Utils.showToast("Config reset to defaults", "warning");
    await loadConfig();
  } catch (error) {
    console.error("[Config] Reset failed", error);
    Utils.showToast(error.message || "Reset failed", "error");
  }
}

async function loadMetadata() {
  const response = await fetch("/api/config/metadata");
  if (!response.ok) {
    throw new Error(`Metadata fetch failed: HTTP ${response.status}`);
  }
  const payload = await response.json();
  state.metadata = transformMetadata(payload?.data ?? {});
  ensureActiveSectionValid();
}

async function loadConfig() {
  try {
    setState({ loading: true });
    const response = await fetch("/api/config");
    if (!response.ok) {
      throw new Error(`Config fetch failed: HTTP ${response.status}`);
    }
    const payload = await response.json();

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
    Utils.showToast(error.message || "Failed to load configuration", "error");
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
        const firstMatch = document.querySelector(".config-field.config-field--match input, .config-field.config-field--match textarea, .config-field.config-field--match button, .config-section-item.search-match");
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
      Utils.showToast(error.message || "Failed to load metadata", "error");
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
