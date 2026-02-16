/**
 * Condition Catalog Module
 * Handles the condition catalog modal and browsing functionality
 */

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
            <div class="category-header ${isCollapsed ? "collapsed" : ""}" data-category="${category}">
              <div class="category-title">
                <span class="icon"><i class="${getCategoryIcon(category)}"></i></span>
                ${category}
              </div>
              <span class="category-toggle">▶</span>
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
        if (catalog) catalog.classList.remove("active");
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
          <span class="condition-name">${Utils.escapeHtml(condition.name || condition.type)}</span>
        </div>
        <div class="condition-description">
          ${condition.description || "No description available"}
        </div>
      </div>
    `;
  }

  /**
   * Setup bulk collapse/expand controls for categories
   */
  function setupCategoryBulkControls() {
    const collapseBtn = $("#collapse-all-categories");
    const expandBtn = $("#expand-all-categories");

    if (collapseBtn) {
      addTrackedListener(
        collapseBtn,
        "click",
        () => setAllCategoriesCollapsed(true),
        CleanupScope.MODAL
      );
    }

    if (expandBtn) {
      addTrackedListener(
        expandBtn,
        "click",
        () => setAllCategoriesCollapsed(false),
        CleanupScope.MODAL
      );
    }
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

    if (collapsed) {
      header.classList.add("collapsed");
      if (items) items.classList.add("collapsed");
      if (toggle) toggle.textContent = "▶";
    } else {
      header.classList.remove("collapsed");
      if (items) items.classList.remove("collapsed");
      if (toggle) toggle.textContent = "▼";
    }
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

  /**
   * Get human-readable label for a category
   */
  function getCategoryLabel(category) {
    // Category names are already human-readable
    return category;
  }

  return {
    initializeConditionCatalog,
    renderConditionItem,
    getCategoryIcon,
    getConditionIcon,
    getCategoryLabel,
    getCategoryStates,
    applyCategoryCollapsedState,
    setAllCategoriesCollapsed,
    updateCategoryState,
  };
}
