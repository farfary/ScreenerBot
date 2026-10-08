// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Condition Catalog Module
 * Handles the condition catalog modal and browsing functionality
 */

import {
  categoryLabel,
  conditionDescription,
  conditionName,
} from "./condition_text.js";

export function createConditionCatalog({
  conditionSchemas,
  categoryStates,
  $,
  $$,
  Utils,
  AppState,
  addTrackedListener,
  clearScope,
  CleanupScope,
}) {
  /**
   * Initialize the condition catalog modal with categories and items
   */
  function initializeConditionCatalog(onAddCondition) {
    const container = $("#condition-categories");
    if (!container || !conditionSchemas) return;

    clearScope(CleanupScope.MODAL);

    // Build categories from schema metadata; hide non-strategy origins
    const categories = {};
    Object.entries(conditionSchemas).forEach(([type, schema]) => {
      if (schema.origin && String(schema.origin).toLowerCase() !== "strategy") return;
      const cat = schema.category || "General";
      if (!categories[cat]) categories[cat] = [];
      categories[cat].push({ type, ...schema });
    });

    const savedStates = getCategoryStates();

    // Render
    container.innerHTML = Object.entries(categories)
      .map(([category, list]) => {
        const isCollapsed = savedStates[category] !== false; // Default to collapsed
        return `
          <div class="condition-category">
            <div class="category-header ${isCollapsed ? "collapsed" : ""}" data-category="${Utils.escapeHtml(category)}">
              <div class="category-title">
                <span class="icon"><i class="${getCategoryIcon(category)}"></i></span>
                ${Utils.escapeHtml(categoryLabel(list[0]))}
              </div>
              <i class="category-toggle ${isCollapsed ? "icon-chevron-right" : "icon-chevron-down"}" aria-hidden="true"></i>
            </div>
            <div class="category-items ${isCollapsed ? "collapsed" : ""}">
              ${list.map((c) => renderConditionItem(c)).join("")}
            </div>
          </div>
        `;
      })
      .join("");

    // Toggle with state persistence (with cleanup tracking)
    $$(".category-header").forEach((header) => {
      const category = header.dataset.category;
      const shouldCollapse = savedStates[category] !== false;
      applyCategoryCollapsedState(header, shouldCollapse);

      const handler = () => {
        const nextCollapsed = !header.classList.contains("collapsed");
        applyCategoryCollapsedState(header, nextCollapsed);
        updateCategoryState(category, nextCollapsed);
      };
      addTrackedListener(header, "click", handler, CleanupScope.MODAL);
    });

    setupCategoryBulkControls();

    // Click to add (with cleanup tracking)
    $$(".condition-item").forEach((item) => {
      const handler = () => {
        const type = item.dataset.conditionType;
        onAddCondition(type);
        const catalog = $("#condition-catalog-modal");
        if (catalog) catalog.hidden = true;
      };
      addTrackedListener(item, "click", handler, CleanupScope.MODAL);
    });
  }

  /**
   * Render a single condition item in the catalog
   */
  function renderConditionItem(condition) {
    const iconClass = condition.icon || getConditionIcon(condition.type);
    return `
      <div class="condition-item" draggable="true" data-condition-type="${condition.type}">
        <div class="condition-item-header">
          <i class="${iconClass}"></i>
          <span class="condition-name">${Utils.escapeHtml(conditionName(condition, condition.type))}</span>
        </div>
        <div class="condition-description">
          ${Utils.escapeHtml(conditionDescription(condition) || I18n.t("strategies-catalog-no-description"))}
        </div>
      </div>
    `;
  }

  /** One control folds every category while any is open, and unfolds them all otherwise. */
  function setupCategoryBulkControls() {
    const toggle = $("#toggle-all-categories");
    if (!toggle) return;
    addTrackedListener(
      toggle,
      "click",
      () => setAllCategoriesCollapsed(anyCategoryOpen()),
      CleanupScope.MODAL
    );
    syncBulkToggle();
  }

  function anyCategoryOpen() {
    return $$(".condition-category .category-header").some(
      (header) => !header.classList.contains("collapsed")
    );
  }

  /** Label the bulk control with the action it performs next. */
  function syncBulkToggle() {
    const toggle = $("#toggle-all-categories");
    if (!toggle) return;
    const fold = anyCategoryOpen();
    toggle.querySelector("i").className = fold ? "icon-chevrons-up" : "icon-chevrons-down";
    const label = toggle.querySelector("span");
    label.setAttribute(
      "data-l10n-id",
      fold ? "strategies-catalog-fold-all" : "strategies-catalog-unfold-all"
    );
    I18n.localizeTree(label);
  }

  /**
   * Collapse or expand all categories
   */
  function setAllCategoriesCollapsed(collapsed) {
    const headers = $$(".condition-category .category-header");
    if (!headers.length) return;
    const states = getCategoryStates();
    headers.forEach((header) => {
      const category = header.dataset.category;
      applyCategoryCollapsedState(header, collapsed);
      if (category) {
        states[category] = collapsed;
      }
    });
    updateAllCategoryStates(states);
  }

  /**
   * Update a single category's collapsed state
   */
  function updateCategoryState(category, collapsed) {
    if (!category) return;
    const states = getCategoryStates();
    states[category] = collapsed;
    updateAllCategoryStates(states);
  }

  /**
   * Apply collapsed/expanded visual state to a category header
   */
  function applyCategoryCollapsedState(header, collapsed) {
    if (!header) return;
    const items = header.nextElementSibling;
    const toggle = header.querySelector(".category-toggle");

    header.classList.toggle("collapsed", collapsed);
    if (items) items.classList.toggle("collapsed", collapsed);
    // A closed section points along the reading direction (mirrored in RTL), an open one down.
    if (toggle) toggle.className = `category-toggle ${collapsed ? "icon-chevron-right" : "icon-chevron-down"}`;
    syncBulkToggle();
  }

  /**
   * Get category states (loads from memory or AppState)
   */
  function getCategoryStates() {
    if (!categoryStates.data) {
      categoryStates.data = loadStoredCategoryStates();
    }
    return categoryStates.data;
  }

  /**
   * Load category states from AppState (server-side storage)
   */
  function loadStoredCategoryStates() {
    try {
      const stored = AppState.load("condition-category-states");
      if (stored && typeof stored === "object") {
        return stored;
      }
    } catch (error) {
      console.warn("[Strategies] Failed to load category states:", error);
    }
    return {};
  }

  /**
   * Persist all category states to AppState
   */
  function updateAllCategoryStates(states) {
    categoryStates.data = states;
    persistCategoryStates();
  }

  /**
   * Save category states via AppState (server-side)
   */
  function persistCategoryStates() {
    try {
      AppState.save("condition-category-states", categoryStates.data || {});
    } catch (error) {
      console.warn("[Strategies] Failed to save category states:", error);
    }
  }

  /**
   * Get icon for a category
   */
  function getCategoryIcon(category) {
    const icons = {
      "Price Patterns": "icon-chart-line",
      "Price Analysis": "icon-chart-line",
      "Candle Patterns": "icon-chart-candlestick",
      "Technical Indicators": "icon-sliders-horizontal",
      "Market Context": "icon-globe",
      "Position & Performance": "icon-trophy",
      "Volume Analysis": "icon-chart-bar",
    };
    return icons[category] || "icon-bookmark";
  }

  /**
   * Get icon for a specific condition type
   */
  function getConditionIcon(type) {
    const icons = {
      PriceChangePercent: "icon-percent",
      PriceToMa: "icon-chart-line",
      LiquidityLevel: "icon-droplet",
      PriceBreakout: "icon-rocket",
      PositionHoldingTime: "icon-hourglass",
      CandleSize: "icon-expand",
      ConsecutiveCandles: "icon-chart-candlestick",
      VolumeSpike: "icon-chart-bar",
    };
    return icons[type] || "icon-puzzle";
  }

  return {
    initializeConditionCatalog,
    renderConditionItem,
    getCategoryIcon,
    getConditionIcon,
    getCategoryStates,
    applyCategoryCollapsedState,
    setAllCategoriesCollapsed,
    updateCategoryState,
  };
}
