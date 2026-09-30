import { registerPage } from "../core/lifecycle.js";
import { $, on, off, create, show, hide, setIconLabel } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { requestManager, apiErrorMessage } from "../core/request_manager.js";
import * as Hints from "../core/hints.js";
import { HintTrigger } from "../ui/hint_popover.js";
import { ConfigExportDialog, ConfigImportDialog } from "../ui/config_import_export_dialog.js";
import { createExpandToggle } from "../ui/expand_toggle.js";
import {
  deepClone,
  deepEqual,
  normalizeFieldValue,
  transformMetadata,
  sortSectionsForDisplay,
  sectionHasMatchingFields,
  metadataMatchesSearch,
} from "./config/utils.js";
import {
  SECTION_ICONS,
  renderFieldControl,
  buildFieldLabelHtml,
  createResetButton,
  applyStackedLayout,
  attachGroupToggle,
  serializeOpenState,
  restoreOpenState,
  expandAllObjects,
  collapseAllObjects,
  expandAllCategories,
  collapseAllCategories,
  isCategoryOpen,
  toggleCategory,
} from "./config/field_renderers.js";
import { categoryLabel, fieldLabel, sectionLabel } from "./config/field_text.js";

const CONFIG_STATE_KEY = "config.page";
const DEFAULT_SECTION = "trader";
const OPENED_STATE_KEY = `${CONFIG_STATE_KEY}.openState`;

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

/**
 * Persist the expand/collapse state of both trees to AppState so it survives
 * page reloads. Called on every toggle (cheap — the state is small and
 * AppState uses an in-memory cache).
 */
function persistOpenedState() {
  AppState.save(OPENED_STATE_KEY, serializeOpenState());
}

function loadOpenedState() {
  restoreOpenState(AppState.load(OPENED_STATE_KEY, null));
}

// The live expand/collapse control of the section toolbar. Recreated by every
// renderToolbar() call, so syncExpandToggle() always talks to the current one.
let expandToggleControl = null;

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

function syncSectionFromHash() {
  const sectionId = window.location.hash.slice(1);
  if (!sectionId || !state.metadata?.[sectionId] || sectionId === state.activeSection) return;
  state.activeSection = sectionId;
  AppState.save(`${CONFIG_STATE_KEY}.activeSection`, sectionId);
  render();
}

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
    if (!state.draft) {
      banner.innerHTML = '<div class="loading-spinner" data-l10n-id="system-config-loading"></div>';
      banner.classList.add("initial-loading");
    } else {
      banner.innerHTML =
        '<div class="loading-spinner inline" data-l10n-id="system-config-refreshing"></div>';
    }
    I18n.localizeTree(banner);
    banner.classList.add("loading");
    banner.hidden = false;
    return;
  }

  if (saving) {
    banner.innerHTML =
      '<strong data-l10n-id="system-config-saving-title"></strong><div data-l10n-id="system-config-saving-detail"></div>';
    I18n.localizeTree(banner);
    banner.classList.add("loading");
    banner.hidden = false;
    return;
  }

  if (errors.size > 0) {
    banner.innerHTML = I18n.markup("system-config-validation-issues");
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
    const label = metadata.label ?? sectionLabel(sectionId);
    const sectionPending = countPendingChanges(sectionId);
    const icon = SECTION_ICONS[sectionId] || "icon-settings";

    // Hide sections that don't have matching fields when searching
    if (searchTerm.length > 0 && !sectionHasMatchingFields(sectionId, searchTerm, state.metadata)) {
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
    const metaParts = [
      `<span class="config-section-count">${Utils.escapeHtml(totalFields)}</span>`,
    ];
    if (sectionPending > 0) {
      metaParts.push(
        `<span class="config-section-pending">+${Utils.escapeHtml(sectionPending)}</span>`
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
      window.history.pushState({ page: "config", subtab: sectionId }, "", `#${sectionId}`);
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
  expandToggleControl = null;

  if (!sectionId) {
    hide(toolbar);
    return;
  }

  const sectionPending = countPendingChanges(sectionId);
  const totalPending = state.pendingChanges.size;

  const sectionChip = create("div", { className: "config-info-chip" });
  sectionChip.innerHTML = sectionPending
    ? I18n.markup("system-config-toolbar-section-changes", { count: sectionPending })
    : Utils.escapeHtml(I18n.t("system-config-toolbar-no-changes"));
  toolbar.appendChild(sectionChip);

  if (totalPending > sectionPending) {
    const globalChip = create("div", { className: "config-info-chip" });
    globalChip.innerHTML = I18n.markup("system-config-toolbar-total-changes", {
      count: totalPending,
    });
    toolbar.appendChild(globalChip);
  }

  // One control for both directions — it applies to top-level categories and
  // nested object sub-configs alike (e.g. OHLCV > Data Sources > GeckoTerminal)
  // and is persisted via AppState so the choice survives reloads. Its state is
  // synced from what is actually on screen by syncExpandToggle(), not from the
  // last bulk action, so it never offers "Collapse all" with nothing expanded.
  expandToggleControl = createExpandToggle({
    expandTitle: I18n.t("system-config-expand-title"),
    collapseTitle: I18n.t("system-config-collapse-title"),
    onToggle: (expanded) => {
      if (expanded) {
        expandAllCategories();
        expandAllObjects();
      } else {
        collapseAllCategories();
        collapseAllObjects();
      }
      persistOpenedState();
      render();
    },
  });
  toolbar.appendChild(expandToggleControl.element);

  show(toolbar);
}

/**
 * Point the expand/collapse control at what is actually on screen: it offers
 * "Collapse all" only while every collapsible node of the active section is
 * open, and hides itself when the section has nothing to collapse. Derived
 * from the rendered DOM rather than from the last bulk action, so per-category
 * clicks, search auto-expansion and default-open categories all count.
 */
function syncExpandToggle() {
  if (!expandToggleControl) {
    return;
  }
  const container = $("#configCategories");
  const nodes = container
    ? container.querySelectorAll(".config-category, .config-object-wrapper")
    : [];
  let openCount = 0;
  for (const node of nodes) {
    if (!node.classList.contains("collapsed")) {
      openCount += 1;
    }
  }
  expandToggleControl.element.hidden = nodes.length === 0;
  expandToggleControl.setExpanded(nodes.length > 0 && openCount === nodes.length);
}

function renderHeader(sectionId) {
  const header = $("#configMainHeader");
  if (!header) {
    return;
  }
  header.innerHTML = "";

  if (!sectionId) {
    const empty = create("div", { className: "config-section-title" });
    empty.textContent = I18n.t("system-config-select-section");
    header.appendChild(empty);
    return;
  }

  const metadata = state.metadata?.[sectionId];
  if (!metadata) {
    const missing = create("div", { className: "config-section-title" });
    missing.innerHTML = I18n.markup("system-config-no-metadata", { section: sectionId });
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
  saveBtn.textContent = state.saving
    ? I18n.t("system-config-saving")
    : I18n.t("system-config-save-changes");
  on(saveBtn, "click", handleSaveAll);
  actions.appendChild(saveBtn);

  const reloadBtn = create("button", {
    type: "button",
    className: "config-header-action ghost",
    disabled: state.loading,
  });
  reloadBtn.textContent = I18n.t("system-config-reload");
  on(reloadBtn, "click", handleReload);
  actions.appendChild(reloadBtn);

  const diffBtn = create("button", {
    type: "button",
    className: "config-header-action ghost",
  });
  diffBtn.textContent = I18n.t("system-config-compare");
  on(diffBtn, "click", handleDiff);
  actions.appendChild(diffBtn);

  // Discards this section's unsaved edits. Disabled while there is nothing to
  // discard — a live-looking button that silently does nothing reads as broken.
  const revertBtn = create("button", {
    type: "button",
    className: "config-header-action destructive",
    disabled: state.saving || !hasSectionChanges(sectionId),
  });
  revertBtn.textContent = I18n.t("system-config-revert-section");
  on(revertBtn, "click", () => {
    revertSection(sectionId);
  });
  actions.appendChild(revertBtn);

  header.appendChild(actions);
}

function renderSectionSummary(metadata) {
  const summaryItems = [];
  if (metadata.summary) {
    if (typeof metadata.summary.total === "number") {
      summaryItems.push(
        `<span class="config-summary-badge">${fieldCountLabel(metadata.summary.total)}</span>`
      );
    }
    if (typeof metadata.summary.critical === "number" && metadata.summary.critical > 0) {
      summaryItems.push(
        `<span class="config-summary-badge warning">${Utils.escapeHtml(I18n.t("system-config-summary-critical", { count: metadata.summary.critical }))}</span>`
      );
    }
    if (typeof metadata.summary.performance === "number" && metadata.summary.performance > 0) {
      summaryItems.push(
        `<span class="config-summary-badge positive">${Utils.escapeHtml(I18n.t("system-config-summary-performance", { count: metadata.summary.performance }))}</span>`
      );
    }
  }
  const pending = countPendingChanges(metadata.id);
  if (pending > 0) {
    summaryItems.push(
      `<span class="config-summary-badge warning">${Utils.escapeHtml(I18n.t("system-config-summary-pending", { count: pending }))}</span>`
    );
  }
  if (!summaryItems.length) {
    summaryItems.push(
      `<span class="config-summary-badge">${Utils.escapeHtml(I18n.t("system-config-summary-none"))}</span>`
    );
  }
  return summaryItems.join("\n");
}

/**
 * Sort categories by visibility level (primary first, then secondary, then technical)
 * Within same visibility, sort alphabetically
 */
function sortCategoriesByVisibility(categories) {
  const visibilityOrder = { primary: 0, secondary: 1, technical: 2 };
  // The category holding a section's on/off switch leads it: nothing below it
  // matters until it is on.
  const holdsSwitch = (fields) => fields.some(([key]) => key === "enabled");
  return categories.sort(([catA, fieldsA], [catB, fieldsB]) => {
    const visA = fieldsA[0]?.[1]?.visibility ?? "secondary";
    const visB = fieldsB[0]?.[1]?.visibility ?? "secondary";
    const orderDiff = visibilityOrder[visA] - visibilityOrder[visB];
    if (orderDiff !== 0) return orderDiff;
    const switchDiff = Number(holdsSwitch(fieldsB)) - Number(holdsSwitch(fieldsA));
    if (switchDiff !== 0) return switchDiff;
    return categoryLabel(catA).localeCompare(categoryLabel(catB));
  });
}

function fieldCountLabel(count) {
  return I18n.t("system-config-fields-count", { count });
}

/**
 * Text of a category chip: the field count, with the pending edits or with
 * how many fields the search leaves visible.
 */
function categoryChipText(totalCount, pendingCount, visibleCount) {
  if (typeof visibleCount === "number" && visibleCount !== totalCount) {
    return I18n.t("system-config-chip-visible", {
      visible: visibleCount,
      fields: fieldCountLabel(totalCount),
    });
  }
  if (pendingCount > 0) {
    return I18n.t("system-config-chip-pending", {
      fields: fieldCountLabel(totalCount),
      pending: pendingCount,
    });
  }
  return fieldCountLabel(totalCount);
}

/**
 * Create a visual separator for visibility sections
 */
function createVisibilitySeparator(label) {
  const sep = create("div", { className: "config-visibility-separator" });
  sep.innerHTML = `<span>${Utils.escapeHtml(label)}</span>`;
  return sep;
}

/**
 * Update the "X fields · Y pending" chip on a category header without
 * rebuilding the DOM. Called from the field onChange callback to keep
 * counters in sync after an in-place edit (which no longer triggers a
 * full re-render). When `visibleFieldCount` differs from total, render
 * the search-filter chip instead.
 */
function updateCategoryChip(categoryEl, totalCount, pendingCount, visibleCount) {
  const chipEl = categoryEl?.querySelector(".config-category-chip");
  if (!chipEl) return;
  chipEl.classList.toggle("pending", pendingCount > 0);
  chipEl.textContent = categoryChipText(totalCount, pendingCount, visibleCount);
}

function renderCategories(sectionId) {
  const container = $("#configCategories");
  if (!container) {
    return;
  }
  container.innerHTML = "";

  if (!sectionId || !state.metadata?.[sectionId]) {
    const empty = create("div", { className: "config-state" });
    empty.textContent = I18n.t("system-config-select-section-details");
    container.appendChild(empty);
    return;
  }

  const metadata = state.metadata[sectionId];
  const fields = Object.entries(metadata.fields ?? {});

  const grouped = new Map();
  for (const [fieldKey, fieldMeta] of fields) {
    const category = fieldMeta.category ?? "general";
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
      container.appendChild(createVisibilitySeparator(I18n.t("system-config-technical-settings")));
    }
    lastVisibility = categoryVisibility;

    // Primary visibility categories are expanded by default; a per-category
    // click or a bulk expand/collapse outranks that default.
    const openByDefault = categoryVisibility === "primary";
    const startCollapsed = !isCategoryOpen([sectionId, category], openByDefault);
    const categoryEl = create("div", {
      className: startCollapsed ? "config-category collapsed" : "config-category",
    });
    categoryEl.dataset.visibility = categoryVisibility;
    categoryEl.dataset.categoryPath = `${sectionId}::${category}`;

    const header = create("button", {
      type: "button",
      className: "config-category-header",
    });
    header.innerHTML = `
      <div class="config-category-label">
        <i class="chevron icon-chevron-down"></i>
        <span>${Utils.escapeHtml(categoryLabel(category))}</span>
      </div>
      <div class="config-category-meta">
        <span class="config-category-chip">${fieldCountLabel(fieldsList.length)}</span>
      </div>
    `;

    const body = create("div", { className: "config-category-body" });

    on(header, "click", () => {
      categoryEl.classList.toggle("collapsed");
      // Record the explicit choice so it survives reloads and outranks both
      // the visibility default and the last bulk action.
      toggleCategory([sectionId, category], openByDefault);
      persistOpenedState();
      syncExpandToggle();
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

      let fieldWasChanged = !deepEqual(fieldValue, fieldOriginalValue);
      if (fieldWasChanged) {
        fieldEl.classList.add("config-field--changed");
        pendingCount += 1;
      }

      const labelEl = create("div", { className: "config-field-label" });
      const controlEl = create("div", { className: "config-field-control" });

      labelEl.innerHTML = buildFieldLabelHtml({
        label: fieldLabel(fieldMeta.key),
        pathLabel: fieldPathLabel,
        metadata: fieldMeta,
        defaultValue: fieldMeta.type === "object" ? undefined : defaultValue,
      });

      const isAtDefault = deepEqual(fieldValue, defaultValue);

      const resetBtn = createResetButton(defaultValue, isAtDefault);
      on(resetBtn, "click", () => {
        resetFieldToDefault(sectionId, fieldKey);
      });

      const control = renderFieldControl(fieldMeta.type, {
        fieldId,
        value: fieldValue,
        originalValue: fieldOriginalValue,
        metadata: fieldMeta,
        disabled: state.saving,
        path: fieldPath,
        searchTerm,
        onChange: (nextValue) => {
          // In-place update only — do NOT call render() here. The previous
          // implementation called render() on every keystroke which rebuilt
          // the entire field tree, collapsing object wrappers (the user
          // had to re-expand to keep typing) and stealing focus from the
          // input. Instead we update state + DOM markers locally and let
          // the global render() run on section switch / save / reload.
          if (!state.draft[sectionId]) {
            state.draft[sectionId] = {};
          }
          state.draft[sectionId][fieldKey] = normalizeFieldValue(fieldMeta.type, nextValue);
          const originalSection = state.original?.[sectionId] ?? {};
          const newValue = state.draft[sectionId][fieldKey];
          const isChanged = !deepEqual(newValue, originalSection[fieldKey]);
          markFieldChanged(sectionId, fieldKey, isChanged);

          // Toggle the row's --changed CSS class + reset-button enable state.
          fieldEl.classList.toggle("config-field--changed", isChanged);
          if (resetBtn.hidden === false) {
            const atDefault = deepEqual(newValue, deepClone(fieldMeta.default));
            resetBtn.disabled = atDefault;
          }

          // Recount pending fields in this category + update the chip. Only a
          // real transition counts: every keystroke keeps isChanged true, and
          // adding on each of them inflated the chip past the field count.
          if (isChanged !== fieldWasChanged) {
            pendingCount = isChanged ? pendingCount + 1 : Math.max(0, pendingCount - 1);
            fieldWasChanged = isChanged;
          }
          updateCategoryChip(categoryEl, fieldsList.length, pendingCount);
          renderToolbar(sectionId);
          syncExpandToggle();
          renderHeader(sectionId);
          renderSidebar();
        },
        onCollapseChange: () => {
          persistOpenedState();
          syncExpandToggle();
        },
      });

      controlEl.appendChild(control);
      applyStackedLayout(fieldEl, control);
      attachGroupToggle({
        rowEl: fieldEl,
        labelEl,
        control,
        path: fieldPath,
        onCollapseChange: () => {
          persistOpenedState();
          syncExpandToggle();
        },
      });

      fieldEl.appendChild(labelEl);
      fieldEl.appendChild(controlEl);
      fieldEl.appendChild(resetBtn);

      // The error line only exists while there IS an error — an always-present
      // empty row cost every field a dead line of height.
      const errorKey = `${sectionId}.${fieldKey}`;
      if (state.errors.has(errorKey)) {
        fieldEl.classList.add("config-field--error");
        const errorEl = create("div", { className: "config-field-error" });
        errorEl.textContent = state.errors.get(errorKey);
        fieldEl.appendChild(errorEl);
      }

      body.appendChild(fieldEl);
    }

    // Update chip to show pending changes if any
    const chipEl = header.querySelector(".config-category-chip");
    if (chipEl) {
      chipEl.classList.toggle("pending", pendingCount > 0);
      chipEl.textContent = categoryChipText(fieldsList.length, pendingCount);
    }

    // Check if category matches search term directly
    const categoryMatchesSearch =
      searchTerm.length > 0 && categoryLabel(category).toLowerCase().includes(searchTerm);

    // Hide categories with no visible fields when searching
    const visibleFieldCount = body.querySelectorAll(".config-field").length;
    if (searchTerm.length > 0 && visibleFieldCount === 0 && !categoryMatchesSearch) {
      continue;
    }

    // Update chip to show visible field count when filtering
    if (searchTerm.length > 0 && visibleFieldCount !== fieldsList.length) {
      const chipEl = header.querySelector(".config-category-chip");
      if (chipEl) {
        chipEl.textContent = categoryChipText(fieldsList.length, 0, visibleFieldCount);
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

  syncExpandToggle();
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
      <span data-l10n-id="system-config-telegram-actions"></span>
      ${overviewHintHtml}
    </div>
    <div class="config-actions-body">
      <div class="config-action-item">
        <div class="config-action-info">
          <div class="config-action-title" data-l10n-id="system-config-telegram-test-title"></div>
          <div class="config-action-desc" data-l10n-id="system-config-telegram-test-description"></div>
        </div>
        <button type="button" class="btn primary" id="telegram-test-btn" disabled></button>
      </div>
      <div class="config-action-status" id="telegram-status" role="status" aria-live="polite"></div>
    </div>
  `;

  I18n.localizeTree(actionsPanel);
  container.appendChild(actionsPanel);

  // Wire up test button
  const testBtn = actionsPanel.querySelector("#telegram-test-btn");
  const statusEl = actionsPanel.querySelector("#telegram-status");
  testBtn.title = I18n.t("common-loading");
  setIconLabel(testBtn, "icon-loader spin", I18n.t("common-loading"));
  const showSendLabel = () =>
    setIconLabel(testBtn, "icon-send", I18n.t("system-config-telegram-send-test"));

  // Check if Telegram is configured before enabling test button
  try {
    const response = await fetch("/api/telegram/status");
    const data = await response.json();
    const isConfigured = data.data?.bot_configured;

    if (isConfigured) {
      testBtn.disabled = false;
      testBtn.title = "";
      showSendLabel();
    } else {
      testBtn.disabled = true;
      testBtn.title = I18n.t("system-config-telegram-configure-token-title");
      showSendLabel();
      statusEl.className = "config-action-status info";
      const icon = document.createElement("i");
      icon.className = "icon-info";
      statusEl.replaceChildren(icon, " ", I18n.t("system-config-telegram-configure-token-status"));
    }
  } catch {
    testBtn.disabled = false;
    testBtn.title = "";
    showSendLabel();
  }

  on(testBtn, "click", async () => {
    testBtn.disabled = true;
    setIconLabel(testBtn, "icon-loader spin", I18n.t("system-config-telegram-sending"));
    statusEl.className = "config-action-status";
    statusEl.textContent = "";

    try {
      const response = await fetch("/api/telegram/test", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
      });
      const data = await response.json();

      if (response.ok) {
        statusEl.className = "config-action-status success";
        const icon = document.createElement("i");
        icon.className = "icon-circle-check";
        statusEl.replaceChildren(icon, " ", I18n.t("system-config-telegram-test-sent-status"));
        Utils.showToast(I18n.t("system-config-telegram-test-sent"), "success");
      } else {
        throw new Error(apiErrorMessage(data, I18n.t("system-config-telegram-test-failed")));
      }
    } catch (error) {
      statusEl.className = "config-action-status error";
      statusEl.innerHTML = `<i class="icon-circle-alert"></i> ${Utils.escapeHtml(error.message)}`;
      Utils.showToast(error.message, "error");
    } finally {
      testBtn.disabled = false;
      showSendLabel();
    }
  });

  // Render authentication section
  renderTelegramAuthSection(container);

  // Initialize hint triggers after all sections are added
  HintTrigger.initAll();
}

/**
 * Render Telegram Authentication Section (TOTP status - read only)
 * TOTP is managed in Security settings and shared with dashboard lockscreen
 */
async function renderTelegramAuthSection(container) {
  const authPanel = create("div", { className: "config-section-actions telegram-auth-section" });

  // Fetch TOTP status
  let totpConfigured = false;
  let commandsRequire2fa = false;
  try {
    const response = await fetch("/api/telegram/totp/status");
    const data = await response.json();
    if (response.ok && data.data) {
      totpConfigured = data.data.configured || false;
      commandsRequire2fa = data.data.commands_require_2fa || false;
    }
  } catch {
    // Ignore errors, show as not configured
  }

  const statusIcon = totpConfigured ? "icon-circle-check" : "icon-circle-alert";
  const statusClass = totpConfigured ? "status-success" : "status-warning";
  const statusText = totpConfigured
    ? I18n.t("system-config-telegram-totp-configured")
    : I18n.t("system-config-telegram-totp-not-configured");

  authPanel.innerHTML = `
    <div class="config-actions-header">
      <i class="icon-shield"></i>
      <span data-l10n-id="system-config-telegram-auth-title"></span>
    </div>
    <div class="config-actions-body">
      <div class="telegram-auth-subsection">
        <div class="telegram-auth-header">
          <div class="telegram-auth-title">
            <i class="icon-key"></i>
            <span data-l10n-id="system-config-telegram-totp-title"></span>
          </div>
          <div class="telegram-auth-status">
            <span class="${statusClass}"><i class="${statusIcon}"></i> ${Utils.escapeHtml(statusText)}</span>
          </div>
        </div>
        <div class="telegram-auth-content">
          <div class="telegram-auth-row">
            <div class="telegram-auth-info">
              <span>${Utils.escapeHtml(
                totpConfigured
                  ? I18n.t("system-config-telegram-totp-active")
                  : I18n.t("system-config-telegram-totp-inactive")
              )}</span>
              <p class="telegram-auth-note"><i class="icon-info"></i> <span data-l10n-id="system-config-telegram-totp-note"></span></p>
            </div>
          </div>
          <div class="telegram-auth-row telegram-auth-actions">
            <div class="telegram-auth-toggle">
              <label class="toggle">
                <input type="checkbox" id="telegram-require-2fa-toggle" ${commandsRequire2fa ? "checked" : ""} ${!totpConfigured ? "disabled" : ""}>
                <span class="toggle-track"></span>
              </label>
              <span data-l10n-id="system-config-telegram-require-2fa"></span>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;

  I18n.localizeTree(authPanel);
  container.appendChild(authPanel);

  // Wire up the toggle
  const toggle = authPanel.querySelector("#telegram-require-2fa-toggle");
  if (toggle && !toggle.disabled) {
    on(toggle, "change", async () => {
      try {
        const response = await fetch("/api/telegram/settings", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ commands_require_2fa: toggle.checked }),
        });
        // The toggle itself shows the saved state, so success is silent. A
        // rejected save was previously silent TOO, which left the toggle
        // showing a value the backend had refused.
        if (!response.ok) {
          throw new Error(
            I18n.t("system-config-telegram-save-rejected", { status: response.status })
          );
        }
      } catch (error) {
        toggle.checked = !toggle.checked; // Revert
        Utils.showToast({
          key: "telegram-setting",
          type: "error",
          title: I18n.t("system-config-telegram-save-failed"),
          message: error?.message || null,
        });
      }
    });
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
  if (!sectionId) {
    return;
  }
  const originalSection = deepClone(state.original?.[sectionId] ?? {});
  state.draft[sectionId] = deepClone(originalSection);
  // Clear every pending key of this section, not only the keys the original
  // still has — a field added to the draft alone would otherwise stay pending
  // forever and keep Save Changes enabled with nothing to save.
  for (const key of [...state.pendingChanges.keys()]) {
    if (key.startsWith(`${sectionId}.`)) {
      state.pendingChanges.delete(key);
    }
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

    Utils.showToast({ type: "success", title: I18n.t("system-config-saved") });
    await loadConfig();
  } catch (error) {
    console.error("[Config] Save failed", error);
    Utils.showToast({
      key: "config-save",
      type: "error",
      title: I18n.t("system-config-save-failed"),
      message: error.message || null,
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
    Utils.showToast({ type: "success", title: I18n.t("system-config-reloaded") });
    await loadConfig();
  } catch (error) {
    console.error("[Config] Reload failed", error);
    Utils.showToast({
      key: "config-save",
      type: "error",
      title: I18n.t("system-config-reload-failed"),
      message: error.message || null,
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
    const message = I18n.text(payload?.text) || apiErrorMessage(payload);
    Utils.showToast({
      type: "info",
      title: I18n.t("system-config-diff-title"),
      message: message || I18n.t("system-config-diff-console"),
    });
    console.info("Config diff:", payload);
  } catch (error) {
    console.error("[Config] Diff failed", error);
    Utils.showToast({
      type: "error",
      title: I18n.t("system-config-diff-failed"),
      message: error.message || null,
    });
  }
}

async function handleResetToDefaults() {
  const { confirmed } = await ConfirmationDialog.show({
    title: I18n.t("system-config-reset-title"),
    message: I18n.t("system-config-reset-message"),
    confirmLabel: I18n.t("system-config-reset-defaults"),
    cancelLabel: I18n.t("common-action-cancel"),
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
      title: I18n.t("system-config-reset-done-title"),
      message: I18n.t("system-config-reset-done-message"),
    });
    await loadConfig();
  } catch (error) {
    console.error("[Config] Reset failed", error);
    Utils.showToast({
      type: "error",
      title: I18n.t("system-config-reset-failed"),
      message: error.message || null,
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
      key: "config-load",
      type: "error",
      title: I18n.t("system-config-load-failed"),
      message: error.message || null,
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

  // Export button
  const exportButton = $("#configExportButton");
  if (exportButton) {
    const handler = async () => {
      const result = await ConfigExportDialog.show();
      if (result.exported) {
        // Optionally refresh after export
      }
    };
    on(exportButton, "click", handler);
    ctx.onDispose(() => off(exportButton, "click", handler));
  }

  // Import button
  const importButton = $("#configImportButton");
  if (importButton) {
    const handler = async () => {
      const result = await ConfigImportDialog.show();
      if (result.imported) {
        // Reload config after import
        await loadConfig();
        render();
      }
    };
    on(importButton, "click", handler);
    ctx.onDispose(() => off(importButton, "click", handler));
  }
}

function activate() {
  loadOpenedState();
  ensureActiveSectionValid();
  render();
}

function deactivate() {}

async function loadInitialConfiguration() {
  try {
    if (!state.metadata) {
      await loadMetadata();
    }

    syncSectionFromHash();
    if (state.activeSection && window.location.hash !== `#${state.activeSection}`) {
      window.history.replaceState(
        { page: "config", subtab: state.activeSection },
        "",
        `#${state.activeSection}`
      );
    }

    await loadConfig();
  } catch (error) {
    console.error("[Config] Metadata load failed", error);
    setState({ loading: false });
    Utils.showToast({
      key: "config-load",
      type: "error",
      title: I18n.t("system-config-metadata-failed"),
      message: error.message || null,
    });
  }
}

function init(ctx) {
  attachEventHandlers(ctx);

  const popstateHandler = () => syncSectionFromHash();
  on(window, "popstate", popstateHandler);
  ctx.onDispose(() => off(window, "popstate", popstateHandler));

  // Paint the shell and the shared loader before any remote metadata/config
  // request. The lifecycle hook itself must remain synchronous.
  state.loading = true;
  render();
  void loadInitialConfiguration();
}

registerPage("config", {
  init,
  activate,
  deactivate,
});
