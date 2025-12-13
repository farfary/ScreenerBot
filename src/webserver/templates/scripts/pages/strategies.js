import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { requestManager } from "../core/request_manager.js";
import { enhanceAllSelects } from "../ui/custom_select.js";

export function createLifecycle() {
  // State
  let currentStrategy = null;
  let strategies = [];
  let templates = [];
  let conditionSchemas = null;
  let categoryStates = null;

  // Editor state (vertical cards)
  let conditions = []; // [{ type, name, enabled, params: {k: v} }]

  // Pollers
  let strategiesPoller = null;
  let templatesPoller = null;

  // Event listener cleanup tracking
  const eventCleanups = [];
  const CleanupScope = {
    STATIC: "static",
    STRATEGIES_LIST: "strategies-list",
    TEMPLATE_LIST: "strategy-templates",
    CONDITION_CARDS: "condition-cards",
    PROPERTY_PANEL: "condition-properties",
    MODAL: "condition-modal",
  };

  // Helper to track event listeners for cleanup
  function addTrackedListener(element, event, handler, scope = CleanupScope.STATIC) {
    if (!element) {
      return;
    }
    element.addEventListener(event, handler);
    eventCleanups.push({
      scope,
      cleanup: () => element.removeEventListener(event, handler),
    });
  }

  function clearScope(scope) {
    if (!scope) {
      return;
    }
    for (let i = eventCleanups.length - 1; i >= 0; i -= 1) {
      const entry = eventCleanups[i];
      if (entry.scope === scope) {
        entry.cleanup();
        eventCleanups.splice(i, 1);
      }
    }
  }

  return {
    async init(_ctx) {
      console.log("[Strategies] Initializing page");

      // Setup strategy type toggle
      setupTypeToggle();

      // Setup sidebar actions
      setupSidebarActions();

      // Setup editor actions
      setupEditorActions();

      // Setup toolbar actions
      setupToolbarActions();

      // Setup search
      setupSearch();

      // Load condition schemas
      await loadConditionSchemas();

      // Initialize condition catalog (modal)
      initializeConditionCatalog();
    },

    async activate(ctx) {
      console.log("[Strategies] Activating page");

      // Create pollers
      strategiesPoller = ctx.managePoller(
        new Poller(
          async () => {
            await loadStrategies();
          },
          { label: "Strategies" }
        )
      );

      templatesPoller = ctx.managePoller(
        new Poller(
          async () => {
            await loadTemplates();
          },
          { label: "Templates" }
        )
      );

      // Start pollers
      strategiesPoller.start();
      templatesPoller.start();

      // Initial load
      await loadStrategies();
      await loadTemplates();
    },

    deactivate() {
      console.log("[Strategies] Deactivating page");
      // Pollers stopped automatically by lifecycle
    },

    dispose() {
      console.log("[Strategies] Disposing page");

      // Clean up all event listeners
      eventCleanups.forEach((entry) => entry.cleanup());
      eventCleanups.length = 0;

      // Cleanup state
      currentStrategy = null;
      strategies = [];
      templates = [];
      conditions = [];
    },
  };

  // Strategy Type Toggle
  function setupTypeToggle() {
    // Setup filter tabs in sidebar
    const filterTabs = document.querySelectorAll(".filter-tab");

    filterTabs.forEach((tab) => {
      tab.addEventListener("click", () => {
        const filter = tab.dataset.filter.toUpperCase();

        // Update active state
        filterTabs.forEach((t) => t.classList.remove("active"));
        tab.classList.add("active");

        // Filter sidebar strategies by type
        filterStrategies(filter);
      });
    });
  }

  // Filters
  function filterStrategies(filter) {
    const items = $$(".strategy-item");
    items.forEach((item) => {
      const strategyId = item.dataset.strategyId;
      const strategy = strategies.find((s) => s.id === strategyId);

      if (!strategy) {
        item.style.display = "none";
        return;
      }

      if (filter === "ALL") {
        item.style.display = "";
      } else {
        item.style.display = strategy.type === filter ? "" : "none";
      }
    });
  }

  // Sidebar Actions
  function setupSidebarActions() {
    // Create new strategy - show type selection modal
    const createBtn = $("#create-strategy");
    if (createBtn) {
      addTrackedListener(createBtn, "click", () => {
        showCreateStrategyModal();
      });
    }

    // Setup create strategy modal
    setupCreateStrategyModal();

    // Import strategy
    const importBtn = $("#import-strategy");
    if (importBtn) {
      addTrackedListener(importBtn, "click", () => {
        importStrategy();
      });
    }

    // Refresh strategies
    const refreshBtn = $("#refresh-strategies");
    if (refreshBtn) {
      addTrackedListener(refreshBtn, "click", async () => {
        await loadStrategies();
        // Removed success toast - silent refresh, only show errors
      });
    }
  }

  function showCreateStrategyModal() {
    const modal = $("#create-strategy-modal");
    if (modal) modal.classList.add("active");
  }

  function hideCreateStrategyModal() {
    const modal = $("#create-strategy-modal");
    if (modal) modal.classList.remove("active");
  }

  function setupCreateStrategyModal() {
    const modal = $("#create-strategy-modal");
    const cancelBtn = $("#cancel-create-strategy");
    const typeCards = $$(".type-card");
    const backdrop = modal?.querySelector(".modal-backdrop");

    if (cancelBtn) {
      addTrackedListener(cancelBtn, "click", hideCreateStrategyModal);
    }

    if (backdrop) {
      addTrackedListener(backdrop, "click", hideCreateStrategyModal);
    }

    typeCards.forEach((card) => {
      addTrackedListener(card, "click", () => {
        const type = card.dataset.type;
        hideCreateStrategyModal();
        createNewStrategy(type);
      });
    });
  }

  // Editor actions (add condition, load template)
  function setupEditorActions() {
    const addBtn = $("#add-condition");
    const loadTemplateBtn = $("#load-template");
    const catalog = $("#condition-catalog-modal");
    const closeCatalog = $("#close-condition-catalog");
    const searchInput = $("#condition-search");

    if (addBtn) {
      addTrackedListener(addBtn, "click", () => openConditionCatalog());
    }
    if (loadTemplateBtn) {
      addTrackedListener(loadTemplateBtn, "click", () => {
        const templatesTab = $(".tab-btn[data-tab='templates']");
        if (templatesTab) templatesTab.click();
      });
    }
    if (closeCatalog && catalog) {
      addTrackedListener(closeCatalog, "click", () => catalog.classList.remove("active"));
      addTrackedListener(catalog, "click", (e) => {
        if (e.target === catalog) catalog.classList.remove("active");
      });
    }

    // Search conditions
    if (searchInput) {
      addTrackedListener(searchInput, "input", (e) => {
        const query = e.target.value.toLowerCase().trim();
        filterConditions(query);
      });
    }
  }

  function filterConditions(query) {
    const categories = $$(".condition-category");

    if (!query) {
      const states = getCategoryStates();
      // Show all, restore saved states
      categories.forEach((cat) => {
        cat.style.display = "block";
        const header = cat.querySelector(".category-header");
        if (!header) return;
        const collapsed = states[header.dataset.category] !== false;
        applyCategoryCollapsedState(header, collapsed);
      });

      $$(".condition-item").forEach((item) => {
        item.style.display = "block";
      });
      return;
    }

    // Filter conditions
    categories.forEach((cat) => {
      const items = cat.querySelectorAll(".condition-item");
      const categoryItems = cat.querySelector(".category-items");
      const header = cat.querySelector(".category-header");
      let hasVisibleItems = false;

      items.forEach((item) => {
        const nameEl = item.querySelector(".condition-name");
        const descEl = item.querySelector(".condition-description");
        if (!nameEl || !descEl) return;

        const name = nameEl.textContent.toLowerCase();
        const desc = descEl.textContent.toLowerCase();
        const matches = name.includes(query) || desc.includes(query);

        if (matches) {
          item.style.display = "block";
          hasVisibleItems = true;
        } else {
          item.style.display = "none";
        }
      });

      // Show/hide category based on matches
      if (hasVisibleItems) {
        cat.style.display = "block";
        categoryItems.classList.remove("collapsed");
        header.classList.remove("collapsed");
      } else {
        cat.style.display = "none";
      }
    });
  }

  // Toolbar Actions
  function setupToolbarActions() {
    const saveBtn = $("#save-strategy");
    const saveAsBtn = $("#save-as-strategy");
    const duplicateBtn = $("#duplicate-strategy");
    const validateBtn = $("#validate-strategy");
    const testBtn = $("#test-strategy");
    const enableToggle = $("#strategy-enabled-toggle");
    const nameInput = $("#strategy-name");

    // Sync strategy name in real-time as user types
    if (nameInput) {
      addTrackedListener(nameInput, "input", (e) => {
        if (currentStrategy) {
          currentStrategy.name = e.target.value.trim();
        }
      });
    }

    if (saveBtn) {
      addTrackedListener(saveBtn, "click", async () => {
        // Validate first before saving
        const isValid = await validateStrategy();
        if (!isValid) {
          window.showToast?.("Please fix validation errors before saving", "error");
          return;
        }
        await saveStrategy();
      });
    }

    if (saveAsBtn) {
      addTrackedListener(saveAsBtn, "click", async () => {
        await saveStrategyAs();
      });
    }

    if (duplicateBtn) {
      addTrackedListener(duplicateBtn, "click", () => {
        duplicateStrategy();
      });
    }

    if (validateBtn) {
      addTrackedListener(validateBtn, "click", async () => {
        await validateStrategy();
      });
    }

    if (testBtn) {
      addTrackedListener(testBtn, "click", async () => {
        await testStrategy();
      });
    }

    // Enable toggle - saves immediately
    if (enableToggle) {
      addTrackedListener(enableToggle, "change", async (e) => {
        if (currentStrategy) {
          currentStrategy.enabled = e.target.checked;
          // If strategy is saved, update via API immediately
          if (currentStrategy.id) {
            await toggleCurrentStrategyEnabled();
          }
        }
      });
    }
  }

  async function toggleCurrentStrategyEnabled() {
    if (!currentStrategy?.id) return;

    try {
      const body = {
        name: currentStrategy.name,
        description: currentStrategy.description || null,
        strategy_type: currentStrategy.type,
        enabled: currentStrategy.enabled,
        priority: currentStrategy.priority ?? 10,
        rules: currentStrategy.rules || null,
        parameters: currentStrategy.parameters || {},
        author: currentStrategy.author || null,
      };

      await requestManager.fetch(`/api/strategies/${currentStrategy.id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
        priority: "high",
      });

      await loadStrategies();
      Utils.showToast({
        type: "success",
        title: currentStrategy.enabled ? "Strategy Enabled" : "Strategy Disabled",
        message: `"${currentStrategy.name}" ${currentStrategy.enabled ? "enabled" : "disabled"}`,
      });
    } catch (error) {
      console.error("Failed to toggle strategy:", error);
      Utils.showToast({
        type: "error",
        title: "Toggle Failed",
        message: "Failed to update strategy status",
      });
      // Revert toggle state
      const toggle = $("#strategy-enabled-toggle");
      if (toggle) toggle.checked = !currentStrategy.enabled;
      currentStrategy.enabled = !currentStrategy.enabled;
    }
  }

  // Search (condition catalog)
  function setupSearch() {
    const searchInput = $("#condition-search");
    const clearBtn = $("#clear-search");

    if (searchInput) {
      addTrackedListener(searchInput, "input", (e) => {
        const query = e.target.value.toLowerCase();
        filterConditions(query);
      });
    }

    if (clearBtn) {
      addTrackedListener(clearBtn, "click", () => {
        if (searchInput) {
          searchInput.value = "";
          filterConditions("");
        }
      });
    }
  }

  // Load Data
  async function loadStrategies() {
    try {
      const data = await requestManager.fetch("/api/strategies", {
        priority: "normal",
      });
      const items = data.items || [];
      strategies = items.map((s) => ({
        id: s.id,
        name: s.name,
        description: s.description || null,
        type: s.strategy_type,
        enabled: !!s.enabled,
        priority: s.priority,
        created_at: s.created_at,
        updated_at: s.updated_at,
        author: s.author || null,
        version: s.version,
      }));

      renderStrategies();
    } catch (error) {
      console.error("Failed to load strategies:", error);
      Utils.showToast({
        type: "error",
        title: "Load Failed",
        message: "Failed to load strategies from server",
      });
    }
  }

  async function loadTemplates() {
    try {
      const data = await requestManager.fetch("/api/strategies/templates", {
        priority: "normal",
      });
      templates = data.items || [];

      renderTemplates();
    } catch (error) {
      console.error("Failed to load templates:", error);
      // Don't show error toast for templates as they're optional
    }
  }

  async function loadConditionSchemas() {
    try {
      const data = await requestManager.fetch("/api/strategies/conditions/schemas", {
        priority: "normal",
      });
      conditionSchemas = data.schemas || {};
    } catch (error) {
      console.error("Failed to load condition schemas:", error);
      conditionSchemas = {};
    }
  }

  // Render Functions
  function renderStrategies() {
    clearScope(CleanupScope.STRATEGIES_LIST);

    const listContainer = $("#strategy-list");
    if (!listContainer) return;

    if (strategies.length === 0) {
      listContainer.innerHTML = `
        <div class="empty-state">
          <span class="icon"><i class="icon-file-text"></i></span>
          <p>No strategies yet</p>
          <small>Create your first strategy</small>
        </div>
      `;
      return;
    }

    listContainer.innerHTML = strategies
      .map(
        (strategy) => `
      <div class="strategy-item ${currentStrategy?.id === strategy.id ? "active" : ""}" 
           data-strategy-id="${strategy.id}">
        <div class="strategy-item-header">
          <span class="strategy-type-icon ${strategy.type.toLowerCase()}">
            <i class="${strategy.type === "ENTRY" ? "icon-trending-up" : "icon-trending-down"}"></i>
          </span>
          <div class="strategy-item-info">
            <div class="strategy-item-title">
              ${Utils.escapeHtml(strategy.name)}
            </div>
            <div class="strategy-item-meta">
              <span class="strategy-badge ${strategy.type.toLowerCase()}">${strategy.type}</span>
              <span class="strategy-badge ${strategy.enabled ? "enabled" : "disabled"}">
                ${strategy.enabled ? "Enabled" : "Disabled"}
              </span>
            </div>
          </div>
          <div class="strategy-item-actions">
            <button class="btn-icon" data-action="toggle" title="${strategy.enabled ? "Disable" : "Enable"}">
              ${strategy.enabled ? '<i class="icon-toggle-right"></i>' : '<i class="icon-toggle-left"></i>'}
            </button>
            <button class="btn-icon" data-action="delete" title="Delete"><i class="icon-trash-2"></i></button>
          </div>
        </div>
      </div>
    `
      )
      .join("");

    // Attach event listeners
    $$(".strategy-item").forEach((item) => {
      const strategyId = item.dataset.strategyId;

      addTrackedListener(
        item,
        "click",
        (e) => {
          if (!e.target.closest(".btn-icon")) {
            loadStrategy(strategyId);
          }
        },
        CleanupScope.STRATEGIES_LIST
      );

      // Action buttons
      const toggleBtn = item.querySelector("[data-action='toggle']");
      const deleteBtn = item.querySelector("[data-action='delete']");

      if (toggleBtn) {
        addTrackedListener(
          toggleBtn,
          "click",
          async (e) => {
            e.stopPropagation();
            await toggleStrategyEnabled(strategyId);
          },
          CleanupScope.STRATEGIES_LIST
        );
      }

      if (deleteBtn) {
        addTrackedListener(
          deleteBtn,
          "click",
          async (e) => {
            e.stopPropagation();
            await deleteStrategy(strategyId);
          },
          CleanupScope.STRATEGIES_LIST
        );
      }
    });

    // Apply current filter from active tab
    const activeFilter = $(".filter-tab.active");
    const filter = activeFilter ? activeFilter.dataset.filter.toUpperCase() : "ALL";
    filterStrategies(filter);
  }

  function renderTemplates() {
    clearScope(CleanupScope.TEMPLATE_LIST);

    const listContainer = $("#template-list");
    if (!listContainer) return;

    if (templates.length === 0) {
      listContainer.innerHTML = `
        <div class="empty-state">
          <span class="icon"><i class="icon-package"></i></span>
          <p>No templates available</p>
        </div>
      `;
      return;
    }

    listContainer.innerHTML = templates
      .map(
        (template) => `
      <div class="template-item" data-template-id="${template.id}">
        <div class="template-item-header">
          <div class="template-item-title">${Utils.escapeHtml(template.name)}</div>
          <span class="risk-badge ${template.risk_level || "medium"}">
            ${(template.risk_level || "medium").toUpperCase()}
          </span>
        </div>
        <div class="template-item-description">
          ${Utils.escapeHtml(template.description || "No description")}
        </div>
        <div class="template-item-footer">
          <span class="strategy-badge">${template.category || "General"}</span>
          <button class="btn" data-action="use">Use Template</button>
        </div>
      </div>
    `
      )
      .join("");

    // Attach event listeners
    $$(".template-item").forEach((item) => {
      const useBtn = item.querySelector("[data-action='use']");
      if (useBtn) {
        addTrackedListener(
          useBtn,
          "click",
          () => {
            const templateId = item.dataset.templateId;
            useTemplate(templateId);
          },
          CleanupScope.TEMPLATE_LIST
        );
      }
    });
  }

  function initializeConditionCatalog() {
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
        addCondition(type);
        const catalog = $("#condition-catalog-modal");
        if (catalog) catalog.classList.remove("active");
      };
      addTrackedListener(item, "click", handler, CleanupScope.MODAL);
    });
  }

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
    categoryStates = states;
    persistCategoryStates();
  }

  function updateCategoryState(category, collapsed) {
    if (!category) return;
    const states = getCategoryStates();
    states[category] = collapsed;
    categoryStates = states;
    persistCategoryStates();
  }

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

  function getCategoryStates() {
    if (!categoryStates) {
      categoryStates = loadStoredCategoryStates();
    }
    return categoryStates;
  }

  function loadStoredCategoryStates() {
    // Category states are loaded via AppState (server-side)
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

  function persistCategoryStates() {
    // Save via AppState (server-side)
    try {
      AppState.save("condition-category-states", categoryStates || {});
    } catch (error) {
      console.warn("[Strategies] Failed to save category states:", error);
    }
  }

  // Helper Functions
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

  function filterTemplates() {
    const categorySelect = $("#template-category");
    const riskSelect = $("#template-risk");

    if (!categorySelect || !riskSelect) return;

    const category = categorySelect.value;
    const risk = riskSelect.value;

    const items = $$(".template-item");
    items.forEach((item) => {
      const templateId = item.dataset.templateId;
      const template = templates.find((t) => t.id === templateId);

      if (!template) {
        item.style.display = "none";
        return;
      }

      const matchesCategory =
        category === "all" || (template.category || "").toLowerCase() === category;
      const matchesRisk = risk === "all" || (template.risk_level || "").toLowerCase() === risk;

      item.style.display = matchesCategory && matchesRisk ? "" : "none";
    });
  }

  function openConditionCatalog() {
    const modal = $("#condition-catalog-modal");
    if (modal) modal.classList.add("active");
  }

  // Condition Management
  function addCondition(conditionType) {
    const schema = conditionSchemas?.[conditionType];
    if (!schema)
      return Utils.showToast({
        type: "error",
        title: "Unknown Condition",
        message: "Condition type not found",
      });

    // Auto-create strategy if none exists (first condition added)
    // Show modal to select type first
    if (!currentStrategy) {
      Utils.showToast({
        type: "warning",
        title: "Create Strategy First",
        message: "Click 'New Strategy' to create a strategy before adding conditions",
      });
      return;
    }

    const params = {};
    Object.entries(schema.parameters || {}).forEach(([k, p]) => {
      params[k] = p.default ?? null;
    });
    conditions.push({
      type: conditionType,
      name: schema.name || conditionType,
      enabled: true,
      params,
    });
    renderConditionsList();
    updateRuleTreeFromEditor();
    Utils.showToast({
      type: "success",
      title: "Condition Added",
      message: `${schema.name || conditionType} added to strategy`,
    });
  }

  // Removed createDefaultParameters (unused)

  function renderConditionsList() {
    const list = $("#conditions-list");
    if (!list) return;
    if (!conditions.length) {
      list.innerHTML =
        '<div class="empty-state"><i class="icon-puzzle"></i><p>No conditions yet</p><small>Use "Add Condition" to start building</small></div>';
      return;
    }

    clearScope(CleanupScope.CONDITION_CARDS);

    list.innerHTML = conditions.map((c, idx) => renderConditionCard(c, idx)).join("");

    // Enhance native selects with custom styling
    enhanceAllSelects(list);

    // Wire card header click to expand/collapse (except when clicking on interactive elements)
    $$(".condition-card .card-header").forEach((header) => {
      const card = header.closest(".condition-card");
      const index = parseInt(card.dataset.index, 10);
      const handler = (e) => {
        // Don't toggle if clicking on checkbox, button, or action button
        if (
          e.target.closest("input") ||
          e.target.closest("button") ||
          e.target.closest(".condition-actions")
        ) {
          return;
        }
        toggleCardExpand(index);
      };
      addTrackedListener(header, "click", handler, CleanupScope.CONDITION_CARDS);
    });

    // Wire actions with cleanup tracking
    $$(".condition-card [data-action]").forEach((btn) => {
      const action = btn.dataset.action;
      const index = parseInt(btn.closest(".condition-card").dataset.index, 10);
      let handler;
      if (action === "toggle-expand") {
        handler = () => toggleCardExpand(index);
      } else if (action === "delete") {
        handler = () => deleteCondition(index);
      } else if (action === "duplicate") {
        handler = () => duplicateCondition(index);
      } else if (action === "move-up") {
        handler = () => moveCondition(index, -1);
      } else if (action === "move-down") {
        handler = () => moveCondition(index, 1);
      }
      if (handler) {
        addTrackedListener(btn, "click", handler, CleanupScope.CONDITION_CARDS);
      }
    });

    // Toggles and param inputs with cleanup tracking
    $$(".condition-card .toggle-enabled").forEach((el) => {
      const handler = (e) => {
        const idx = parseInt(el.closest(".condition-card").dataset.index, 10);
        const card = el.closest(".condition-card");
        conditions[idx].enabled = e.target.checked;

        // Update card status class
        if (e.target.checked) {
          card.classList.remove("status-disabled");
          card.classList.add("status-enabled");
        } else {
          card.classList.remove("status-enabled");
          card.classList.add("status-disabled");
        }

        updateRuleTreeFromEditor();
      };
      addTrackedListener(el, "change", handler, CleanupScope.CONDITION_CARDS);
    });

    // Param inputs with cleanup tracking
    $$(".condition-card .param-field input, .condition-card .param-field select").forEach(
      (input) => {
        const handler = () => {
          const card = input.closest(".condition-card");
          const idx = parseInt(card.dataset.index, 10);
          const key = input.dataset.key;
          const schema = conditionSchemas[conditions[idx].type];
          const spec = schema.parameters?.[key] || {};
          let value = input.value;
          if (spec.type === "number" || spec.type === "percent" || spec.type === "sol")
            value = parseFloat(value);
          if (spec.type === "boolean") value = input.checked;
          conditions[idx].params[key] = value;
          updateRuleTreeFromEditor();
          // Update summary text
          const summaryContent = card.querySelector(".summary-content");
          if (summaryContent) summaryContent.textContent = buildConditionSummary(conditions[idx]);
        };
        addTrackedListener(input, "change", handler, CleanupScope.CONDITION_CARDS);
      }
    );
  }

  function renderConditionCard(c, idx) {
    const schema = conditionSchemas?.[c.type] || {};
    const iconClass = schema.icon || getConditionIcon(c.type);
    const category = schema.category || "General";
    const description = schema.description || "";
    const summary = buildConditionSummary(c);
    const body = renderParamEditor(c, schema, idx);
    const statusClass = c.enabled ? "status-enabled" : "status-disabled";
    const categorySlug = category.toLowerCase().replace(/[^a-z0-9]+/g, "-");

    return `
      <div class="condition-card ${statusClass}" data-index="${idx}" data-category="${categorySlug}">
        <div class="card-header">
          <div class="card-header-left">
            <div class="condition-icon">
              <i class="${iconClass}"></i>
            </div>
            <div class="condition-info">
              <div class="condition-name">
                ${Utils.escapeHtml(c.name || c.type)}
                <span class="condition-category-badge category-${categorySlug}">${Utils.escapeHtml(category)}</span>
              </div>
              <div class="condition-description">${Utils.escapeHtml(description)}</div>
            </div>
          </div>
          <div class="card-header-right">
            <div class="condition-status">
              <label class="status-toggle" title="${c.enabled ? "Enabled" : "Disabled"}">
                <input type="checkbox" class="toggle-enabled" ${c.enabled ? "checked" : ""}/>
                <span class="status-indicator"></span>
              </label>
            </div>
            <div class="condition-actions">
              <button class="btn-icon" data-action="move-up" title="Move up"><i class="icon-chevron-up"></i></button>
              <button class="btn-icon" data-action="move-down" title="Move down"><i class="icon-chevron-down"></i></button>
              <button class="btn-icon" data-action="duplicate" title="Duplicate"><i class="icon-copy"></i></button>
              <button class="btn-icon" data-action="delete" title="Delete"><i class="icon-trash-2"></i></button>
            </div>
            <span class="expand-indicator"><i class="icon-chevron-down"></i></span>
          </div>
        </div>
        <div class="card-summary">
          <div class="summary-content">${Utils.escapeHtml(summary)}</div>
        </div>
        <div class="card-body">${body}</div>
      </div>
    `;
  }

  function toggleCardExpand(index) {
    const card = document.querySelector(`.condition-card[data-index="${index}"]`);
    if (card) card.classList.toggle("expanded");
  }

  function duplicateCondition(index) {
    const copy = JSON.parse(JSON.stringify(conditions[index]));
    conditions.splice(index + 1, 0, copy);
    renderConditionsList();
    updateRuleTreeFromEditor();
  }

  function deleteCondition(index) {
    conditions.splice(index, 1);
    renderConditionsList();
    updateRuleTreeFromEditor();
  }

  function moveCondition(index, delta) {
    const newIndex = index + delta;
    if (newIndex < 0 || newIndex >= conditions.length) return;
    const [item] = conditions.splice(index, 1);
    conditions.splice(newIndex, 0, item);
    renderConditionsList();
    updateRuleTreeFromEditor();
  }

  function buildConditionSummary(c) {
    const schema = conditionSchemas?.[c.type] || {};
    const params = schema.parameters || {};
    const parts = [];

    // Special handling for conditions with time period (time_value + time_unit)
    if (c.params.time_value !== undefined && c.params.time_unit !== undefined) {
      const timeValue = c.params.time_value;
      const timeUnit = c.params.time_unit;
      const unitLabel = timeUnit === "SECONDS" ? "sec" : timeUnit === "MINUTES" ? "min" : "hrs";
      const timePart = `${timeValue} ${unitLabel}`;

      // Add other parameters (skip time_value and time_unit as they're combined)
      Object.entries(c.params).forEach(([key, value]) => {
        if (key === "time_value" || key === "time_unit") return;
        const spec = params[key];
        if (!spec) return;

        const label = spec.name || key;
        const formattedValue = formatParamValueWithUnit(value, spec);
        parts.push(`${label}: ${formattedValue}`);
      });

      // Add time period last
      parts.push(`Period: ${timePart}`);
    } else {
      // Build human-readable summary based on condition type
      Object.entries(c.params).forEach(([key, value]) => {
        const spec = params[key];
        if (!spec) return;

        const label = spec.name || key;
        const formattedValue = formatParamValueWithUnit(value, spec);
        parts.push(`${label}: ${formattedValue}`);
      });
    }

    return parts.slice(0, 3).join(", ") || "No parameters";
  }

  function formatParamValue(v) {
    if (v === undefined || v === null) return "";
    if (typeof v === "number") return String(v);
    if (typeof v === "boolean") return v ? "true" : "false";
    return String(v);
  }

  function formatParamValueWithUnit(value, spec) {
    if (value === undefined || value === null) return "—";

    // Handle enum types - show label instead of value
    if (spec.type === "enum" && spec.options) {
      const option = spec.options.find((opt) => {
        const optValue = typeof opt === "object" ? opt.value : opt;
        return optValue === value;
      });
      if (option) {
        return typeof option === "object" ? option.label : option;
      }
      return String(value);
    }

    // Handle boolean
    if (spec.type === "boolean") {
      return value
        ? '<i class="icon-check" style="color: var(--success);"></i> Yes'
        : '<i class="icon-x" style="color: var(--error);"></i> No';
    }

    // Handle numbers with units
    if (typeof value === "number") {
      // Percent type
      if (spec.type === "percent") {
        return `${value}%`;
      }
      // SOL type
      if (spec.type === "sol") {
        return `${value} SOL`;
      }
      // Check name for hints about unit
      const name = (spec.name || "").toLowerCase();
      if (name.includes("hour")) {
        return value === 1 ? `${value} hour` : `${value} hours`;
      }
      if (name.includes("minute")) {
        return value === 1 ? `${value} minute` : `${value} minutes`;
      }
      if (name.includes("candle") || name.includes("period") || name.includes("lookback")) {
        return value === 1 ? `${value} candle` : `${value} candles`;
      }
      if (name.includes("multiplier") || name.includes("ratio")) {
        return `${value}×`;
      }
      // Default number formatting
      return value % 1 === 0 ? String(value) : value.toFixed(2);
    }

    return String(value);
  }

  function renderParamEditor(c, schema, idx) {
    const entries = Object.entries(schema.parameters || {});
    if (!entries.length) return '<div class="param-row">No parameters</div>';
    // Basic approach: show all params; could gate last N as advanced in future
    const fields = entries.map(([key, spec]) => {
      const label = spec.name || key;
      const val = c.params[key] ?? spec.default ?? "";
      return `
        <div class="param-field">
          <label>${Utils.escapeHtml(label)}</label>
          ${renderParamInput(idx, key, spec, val)}
          ${spec.description ? `<div class="property-description">${Utils.escapeHtml(spec.description)}</div>` : ""}
        </div>
      `;
    });
    return `<div class="param-row">${fields.join("")}</div>`;
  }

  function renderParamInput(idx, key, spec, value) {
    const id = `param-${idx}-${key}`;
    const data = `data-key="${key}"`;
    const min = spec.min !== undefined ? `min="${spec.min}"` : "";
    const max = spec.max !== undefined ? `max="${spec.max}"` : "";
    const step = spec.step !== undefined ? `step="${spec.step}"` : "";

    switch (spec.type) {
      case "percent": {
        return `<div class="input-with-unit">
          <input id="${id}" ${data} type="number" value="${value}" ${min} ${max} ${step} placeholder="0">
          <span class="input-unit">%</span>
        </div>`;
      }
      case "sol": {
        return `<div class="input-with-unit">
          <input id="${id}" ${data} type="number" value="${value}" ${min} ${max} ${step} placeholder="0">
          <span class="input-unit">SOL</span>
        </div>`;
      }
      case "number": {
        // Check if we should add a unit based on the name
        const name = (spec.name || "").toLowerCase();
        let unit = "";
        if (name.includes("hour")) unit = "hrs";
        else if (name.includes("minute")) unit = "min";
        else if (name.includes("multiplier")) unit = "×";

        if (unit) {
          return `<div class="input-with-unit">
            <input id="${id}" ${data} type="number" value="${value}" ${min} ${max} ${step} placeholder="0">
            <span class="input-unit">${unit}</span>
          </div>`;
        }
        return `<input id="${id}" ${data} type="number" value="${value}" ${min} ${max} ${step} placeholder="0">`;
      }
      case "boolean":
        return `<label class="toggle-switch">
          <input id="${id}" ${data} type="checkbox" ${value ? "checked" : ""}>
          <span class="toggle-slider"></span>
        </label>`;
      case "enum": {
        const options = spec.options || spec.values || [];
        const optionsHtml = options
          .map((opt) => {
            const optValue = typeof opt === "object" ? opt.value : opt;
            const optLabel = typeof opt === "object" ? opt.label : opt;
            const selected = optValue === value ? "selected" : "";
            return `<option value="${Utils.escapeHtml(String(optValue))}" ${selected}>${Utils.escapeHtml(String(optLabel))}</option>`;
          })
          .join("");
        return `<select id="${id}" ${data} class="select-field" data-custom-select>${optionsHtml}</select>`;
      }
      default:
        return `<input id="${id}" ${data} type="text" value="${Utils.escapeHtml(String(value))}">`;
    }
  }

  function updateRuleTreeFromEditor() {
    if (!currentStrategy) return;
    if (conditions.length === 0) {
      currentStrategy.rules = null;
      return;
    }
    const condNodes = conditions
      .filter((c) => c.enabled)
      .map((c) => {
        const schema = conditionSchemas?.[c.type] || { parameters: {} };
        const params = {};
        Object.keys(schema.parameters || {}).forEach((k) => {
          const v = c.params[k];
          const defv = schema.parameters[k]?.default;
          params[k] = { value: v, default: defv };
        });
        return { condition: { type: c.type, parameters: params } };
      });
    if (condNodes.length === 1) currentStrategy.rules = condNodes[0];
    else currentStrategy.rules = { operator: "AND", conditions: condNodes };
  }

  function renderPropertiesPanel(node) {
    clearScope(CleanupScope.PROPERTY_PANEL);
    const editor = $("#property-editor");
    if (!editor) return;

    if (!node) {
      editor.innerHTML = `
        <div class="empty-state">
          <span class="icon"><i class="icon-target"></i></span>
          <p>No selection</p>
          <small>Select a node to edit properties</small>
        </div>
      `;
      return;
    }

    const schema = conditionSchemas[node.conditionType];
    if (!schema) {
      editor.innerHTML = `
        <div class="empty-state">
          <i class="icon-triangle-alert"></i>
          <p>Unknown condition type</p>
        </div>
      `;
      return;
    }

    let html = `
      <div class="property-group">
        <div class="property-group-title">Condition Details</div>
        <div class="property-field">
          <label class="property-label">Name</label>
          <input type="text" class="property-input" id="node-name" value="${Utils.escapeHtml(node.name)}">
        </div>
        <div class="property-field">
          <label class="property-label">Type</label>
          <input type="text" class="property-input" value="${Utils.escapeHtml(node.conditionType)}" disabled>
        </div>
      </div>
    `;

    if (schema.parameters && Object.keys(schema.parameters).length > 0) {
      html += `
        <div class="property-group">
          <div class="property-group-title">Parameters</div>
      `;

      Object.entries(schema.parameters).forEach(([key, paramSchema]) => {
        const value = node.parameters[key] ?? paramSchema.default ?? "";
        html += renderParameterField(key, paramSchema, value);
      });

      html += "</div>";
    }

    editor.innerHTML = html;

    // Attach event listeners with cleanup tracking
    const nameInput = $("#node-name");
    if (nameInput) {
      const handler = (e) => {
        node.name = e.target.value;
        renderConditionsList();
      };
      addTrackedListener(nameInput, "input", handler, CleanupScope.PROPERTY_PANEL);
    }

    // Parameter inputs with cleanup tracking
    if (schema.parameters) {
      Object.keys(schema.parameters).forEach((key) => {
        const input = $(`#param-${key}`);
        if (input) {
          const handler = (e) => {
            const paramSchema = schema.parameters[key];
            let value = e.target.value;

            // Convert to appropriate type
            if (paramSchema.type === "number") {
              value = parseFloat(value) || 0;
            } else if (paramSchema.type === "boolean") {
              value = e.target.checked;
            }

            const existing = node.parameters[key];
            const defaultVal = paramSchema.default !== undefined ? paramSchema.default : null;
            if (existing && typeof existing === "object" && "value" in existing) {
              node.parameters[key] = { ...existing, value };
            } else {
              node.parameters[key] = { value, default: defaultVal };
            }
            updateRuleTreeFromEditor();
          };
          addTrackedListener(input, "input", handler, CleanupScope.PROPERTY_PANEL);
          addTrackedListener(input, "change", handler, CleanupScope.PROPERTY_PANEL);
        }
      });
    }
  }

  function renderParameterField(key, schema, value) {
    const type = schema.type || "string";
    const effectiveValue =
      value && typeof value === "object" && "value" in value ? value.value : value;
    const description = schema.description
      ? `<div class="property-description">${Utils.escapeHtml(schema.description)}</div>`
      : "";

    let inputHtml = "";

    switch (type) {
      case "number":
        inputHtml = `<input type="number" class="property-input" id="param-${key}" value="${effectiveValue}" 
          ${schema.min !== undefined ? `min="${schema.min}"` : ""} 
          ${schema.max !== undefined ? `max="${schema.max}"` : ""} 
          ${schema.step !== undefined ? `step="${schema.step}"` : ""}>`;
        break;

      case "boolean":
        inputHtml = `<input type="checkbox" id="param-${key}" ${effectiveValue ? "checked" : ""}>`;
        break;

      case "enum":
        {
          const opts =
            schema.values && Array.isArray(schema.values) ? schema.values : schema.options || [];
          if (opts && Array.isArray(opts) && opts.length > 0) {
            inputHtml = `<select class="property-input" id="param-${key}" data-custom-select>
              ${opts.map((v) => `<option value="${v}" ${v === effectiveValue ? "selected" : ""}>${v}</option>`).join("")}
            </select>`;
          } else {
            inputHtml = `<input type="text" class="property-input" id="param-${key}" value="${Utils.escapeHtml(String(effectiveValue))}">`;
          }
        }
        break;

      default:
        if (schema.options && Array.isArray(schema.options) && schema.options.length > 0) {
          inputHtml = `<select class="property-input" id="param-${key}" data-custom-select>
            ${schema.options.map((v) => `<option value="${v}" ${v === effectiveValue ? "selected" : ""}>${v}</option>`).join("")}
          </select>`;
        } else {
          inputHtml = `<input type="text" class="property-input" id="param-${key}" value="${Utils.escapeHtml(String(effectiveValue))}">`;
        }
    }

    return `
      <div class="property-field">
        <label class="property-label">${schema.name || key}</label>
        ${inputHtml}
        ${description}
      </div>
    `;
  }

  // Parameter Editor Modal Functions (kept for potential future use, not auto-opened)
  function openParameterEditor(node) {
    const modal = $("#parameter-editor-modal");
    const body = $("#parameter-editor-body");
    if (!modal || !body) return;

    const schema = conditionSchemas[node.conditionType];
    if (!schema) {
      Utils.showToast("Unknown condition type", "error");
      return;
    }

    let html = `
      <div class="property-group">
        <div class="property-group-title">Condition Details</div>
        <div class="property-field">
          <label class="property-label">Name</label>
          <input type="text" class="property-input" id="modal-node-name" value="${Utils.escapeHtml(node.name)}">
        </div>
        <div class="property-field">
          <label class="property-label">Type</label>
          <input type="text" class="property-input" value="${Utils.escapeHtml(node.conditionType)}" disabled>
        </div>
      </div>
    `;

    if (schema.parameters && Object.keys(schema.parameters).length > 0) {
      html += `
        <div class="property-group">
          <div class="property-group-title">Parameters</div>
      `;

      Object.entries(schema.parameters).forEach(([key, paramSchema]) => {
        const value = node.parameters[key] ?? paramSchema.default ?? "";
        html += renderParameterField(key, paramSchema, value);
      });

      html += "</div>";
    }

    body.innerHTML = html;

    // Enhance native selects with custom styling
    enhanceAllSelects(body);

    // Store reference to the current node being edited
    modal.dataset.editingNodeId = node.id;

    // Show modal
    modal.classList.add("active");

    // Setup event listeners
    setupParameterEditorListeners(node, schema);
  }

  function setupParameterEditorListeners(node, schema) {
    const modal = $("#parameter-editor-modal");
    const closeBtn = $("#close-parameter-editor");
    const cancelBtn = $("#cancel-parameter-edit");
    const applyBtn = $("#apply-parameter-edit");

    clearScope(CleanupScope.MODAL);

    // Close handlers
    const closeModal = () => {
      modal.classList.remove("active");
      clearScope(CleanupScope.MODAL);
    };

    addTrackedListener(closeBtn, "click", closeModal, CleanupScope.MODAL);
    addTrackedListener(cancelBtn, "click", closeModal, CleanupScope.MODAL);

    // Click outside to close
    const outsideClickHandler = (e) => {
      if (e.target === modal) {
        closeModal();
      }
    };
    addTrackedListener(modal, "click", outsideClickHandler, CleanupScope.MODAL);

    // Apply changes
    const applyHandler = () => {
      // Update node name
      const nameInput = $("#modal-node-name");
      if (nameInput) {
        node.name = nameInput.value;
      }

      // Update parameters
      if (schema.parameters) {
        Object.keys(schema.parameters).forEach((key) => {
          const input = $(`#param-${key}`);
          if (input) {
            const paramSchema = schema.parameters[key];
            let value = input.value;

            // Convert to appropriate type
            if (paramSchema.type === "number") {
              value = parseFloat(value) || 0;
            } else if (paramSchema.type === "boolean") {
              value = input.checked;
            }

            const existing = node.parameters[key];
            const defaultVal = paramSchema.default !== undefined ? paramSchema.default : null;
            if (existing && typeof existing === "object" && "value" in existing) {
              node.parameters[key] = { ...existing, value };
            } else {
              node.parameters[key] = { value, default: defaultVal };
            }
          }
        });
      }

      updateRuleTreeFromEditor();
      renderConditionsList();
      closeModal();
      Utils.showToast("Parameters updated", "success");
    };
    addTrackedListener(applyBtn, "click", applyHandler, CleanupScope.MODAL);

    // ESC key to close (with cleanup tracking)
    const escHandler = (e) => {
      if (e.key === "Escape") {
        closeModal();
        document.removeEventListener("keydown", escHandler);
      }
    };
    addTrackedListener(document, "keydown", escHandler, CleanupScope.MODAL);
  }

  // Strategy Operations
  function createNewStrategy(strategyType = "ENTRY") {
    currentStrategy = {
      id: null,
      name: "New Strategy",
      type: strategyType,
      enabled: true,
      priority: 10,
      rules: null,
      parameters: {},
    };

    // Update UI
    const nameInput = $("#strategy-name");
    if (nameInput) nameInput.value = currentStrategy.name;

    // Update type badge
    updateTypeBadge(strategyType);

    // Update enable toggle
    const enableToggle = $("#strategy-enabled-toggle");
    if (enableToggle) enableToggle.checked = true;

    // Clear editor conditions
    conditions = [];
    renderConditionsList();
    renderPropertiesPanel(null);

    Utils.showToast({
      type: "success",
      title: "New Strategy",
      message: `Created new ${strategyType.toLowerCase()} strategy`,
    });
  }

  function updateTypeBadge(type) {
    const badge = $("#strategy-type-badge");
    if (!badge) return;

    badge.className = `strategy-type-badge ${type.toLowerCase()}`;
    const icon = type === "ENTRY" ? "icon-trending-up" : "icon-trending-down";
    badge.innerHTML = `<i class="${icon}"></i> ${type}`;
  }

  async function loadStrategy(strategyId) {
    try {
      const data = await requestManager.fetch(`/api/strategies/${strategyId}`, {
        priority: "normal",
      });
      currentStrategy = {
        id: data.id,
        name: data.name,
        description: data.description || null,
        type: data.strategy_type,
        enabled: !!data.enabled,
        priority: data.priority,
        rules: data.rules || null,
        parameters: data.parameters || {},
        created_at: data.created_at,
        updated_at: data.updated_at,
        author: data.author || null,
        version: data.version,
      };

      // Update UI
      const nameInput = $("#strategy-name");
      if (nameInput) nameInput.value = currentStrategy.name;

      // Update type badge
      updateTypeBadge(currentStrategy.type);

      // Update enable toggle
      const enableToggle = $("#strategy-enabled-toggle");
      if (enableToggle) enableToggle.checked = currentStrategy.enabled;

      // Render strategy into vertical editor
      parseRuleTreeToConditions(currentStrategy.rules);
      renderConditionsList();

      // Update active state in list
      $$(".strategy-item").forEach((item) => {
        if (item.dataset.strategyId === strategyId) {
          item.classList.add("active");
        } else {
          item.classList.remove("active");
        }
      });

      Utils.showToast(`Loaded strategy: ${currentStrategy.name}`, "success");
    } catch (error) {
      console.error("Failed to load strategy:", error);
      Utils.showToast("Failed to load strategy", "error");
    }
  }
  function parseRuleTreeToConditions(rules) {
    conditions = [];
    if (!rules) return;
    const leafs = [];
    function walk(node) {
      if (!node) return;
      if (node.condition) {
        leafs.push(node.condition);
        return;
      }
      (node.conditions || []).forEach((c) => walk(c));
    }
    walk(rules);
    leafs.forEach((cond) => {
      const schema = conditionSchemas?.[cond.type] || { parameters: {} };
      const params = {};
      Object.keys(schema.parameters || {}).forEach((k) => {
        const p = cond.parameters?.[k];
        params[k] =
          p && typeof p === "object" && "value" in p
            ? p.value
            : (schema.parameters[k]?.default ?? null);
      });
      conditions.push({
        type: cond.type,
        name: schema.name || cond.type,
        enabled: true,
        params,
      });
    });
  }

  async function saveStrategy() {
    if (!currentStrategy) {
      Utils.showToast({
        type: "error",
        title: "No Strategy Created",
        message: "Add at least one condition or click 'New Strategy' to create a strategy first",
      });
      return;
    }

    // Validate strategy has conditions
    if (conditions.length === 0) {
      Utils.showToast({
        type: "warning",
        title: "No Conditions",
        message: "Add at least one condition to the strategy before saving",
      });
      return;
    }

    try {
      // Get current values from UI
      const nameInput = $("#strategy-name");
      const typeSelect = $("#strategy-type");

      if (nameInput) currentStrategy.name = nameInput.value.trim();
      if (typeSelect) currentStrategy.type = typeSelect.value;

      // Validate name
      if (!currentStrategy.name) {
        Utils.showToast({
          type: "warning",
          title: "Name Required",
          message: "Enter a strategy name before saving",
        });
        if (nameInput) nameInput.focus();
        return;
      }

      // Sync rule tree from editor
      updateRuleTreeFromEditor();

      const body = {
        name: currentStrategy.name,
        description: currentStrategy.description || null,
        strategy_type: currentStrategy.type,
        enabled: !!currentStrategy.enabled,
        priority: currentStrategy.priority ?? 10,
        rules: currentStrategy.rules || null,
        parameters: currentStrategy.parameters || {},
        author: currentStrategy.author || null,
      };

      const method = currentStrategy.id ? "PUT" : "POST";
      const url = currentStrategy.id ? `/api/strategies/${currentStrategy.id}` : "/api/strategies";

      const data = await requestManager.fetch(url, {
        method,
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
        priority: "high",
      });
      if (!currentStrategy.id && data.id) {
        currentStrategy.id = data.id;
      }

      await loadStrategies();
      Utils.showToast({
        type: "success",
        title: "Strategy Saved",
        message: `"${currentStrategy.name}" saved successfully`,
      });
    } catch (error) {
      console.error("Failed to save strategy:", error);
      Utils.showToast({
        type: "error",
        title: "Save Failed",
        message: "Failed to save strategy to database",
      });
    }
  }

  async function saveStrategyAs() {
    if (!currentStrategy) {
      Utils.showToast("No strategy to save", "warning");
      return;
    }

    // eslint-disable-next-line no-undef
    const newName = prompt("Enter new strategy name:", `${currentStrategy.name} (Copy)`);
    if (!newName) return;
    const newStrategy = { ...currentStrategy, id: null, name: newName };
    currentStrategy = newStrategy;

    await saveStrategy();
  }

  function duplicateStrategy() {
    if (!currentStrategy) {
      Utils.showToast("No strategy to duplicate", "warning");
      return;
    }

    currentStrategy = {
      ...currentStrategy,
      id: null,
      name: `${currentStrategy.name} (Copy)`,
    };

    const nameInput = $("#strategy-name");
    if (nameInput) nameInput.value = currentStrategy.name;

    Utils.showToast("Strategy duplicated", "success");
  }

  async function validateStrategy() {
    if (!currentStrategy) {
      Utils.showToast("No strategy to validate", "warning");
      return false;
    }

    try {
      const data = await requestManager.fetch(`/api/strategies/${currentStrategy.id}/validate`, {
        method: "POST",
        priority: "high",
      });

      if (data.valid) {
        updateValidationStatus(true, "Strategy is valid");
        Utils.showToast("Strategy is valid", "success");
        return true;
      } else {
        updateValidationStatus(false, data.errors?.join(", ") || "Invalid strategy");
        Utils.showToast("Strategy has errors", "error");
        return false;
      }
    } catch (error) {
      console.error("Validation failed:", error);
      updateValidationStatus(false, error.message);
      Utils.showToast("Validation failed", "error");
      return false;
    }
  }

  async function testStrategy() {
    if (!currentStrategy?.id) {
      Utils.showToast("Please save the strategy first", "warning");
      return;
    }

    try {
      const data = await requestManager.fetch(`/api/strategies/${currentStrategy.id}/test`, {
        method: "POST",
        priority: "high",
      });
      Utils.showToast(
        `Test result: ${data.result ? "Passed" : "Failed"}`,
        data.result ? "success" : "error"
      );
    } catch (error) {
      console.error("Test failed:", error);
      Utils.showToast("Test failed", "error");
    }
  }

  async function toggleStrategyEnabled(strategyId) {
    try {
      const strategy = strategies.find((s) => s.id === strategyId);
      if (!strategy) return;
      // Fetch full detail to avoid missing fields
      const detail = await requestManager.fetch(`/api/strategies/${strategyId}`, {
        priority: "normal",
      });

      const body = {
        name: detail.name,
        description: detail.description || null,
        strategy_type: detail.strategy_type,
        enabled: !strategy.enabled,
        priority: detail.priority,
        rules: detail.rules || null,
        parameters: detail.parameters || {},
        author: detail.author || null,
      };

      await requestManager.fetch(`/api/strategies/${strategyId}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
        priority: "high",
      });

      await loadStrategies();
      Utils.showToast(`Strategy ${strategy.enabled ? "disabled" : "enabled"}`, "success");
    } catch (error) {
      console.error("Failed to toggle strategy:", error);
      Utils.showToast("Failed to toggle strategy", "error");
    }
  }

  async function deleteStrategy(strategyId) {
    const strategy = strategies.find((s) => s.id === strategyId);
    if (!strategy) return;

    const { confirmed } = await ConfirmationDialog.show({
      title: "Delete Strategy",
      message: `Delete strategy "${strategy.name}"? This action cannot be undone.`,
      confirmLabel: "Delete",
      cancelLabel: "Cancel",
      variant: "danger",
    });

    if (!confirmed) return;

    try {
      await requestManager.fetch(`/api/strategies/${strategyId}`, {
        method: "DELETE",
        priority: "high",
      });

      if (currentStrategy?.id === strategyId) {
        currentStrategy = null;
        createNewStrategy();
      }

      await loadStrategies();
      Utils.showToast({
        type: "success",
        title: "Strategy Deleted",
        message: `"${strategy.name}" removed successfully`,
      });
    } catch (error) {
      console.error("Failed to delete strategy:", error);
      Utils.showToast({
        type: "error",
        title: "Delete Failed",
        message: "Failed to delete strategy from database",
      });
    }
  }

  function importStrategy() {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";

    input.onchange = async (e) => {
      const file = e.target.files?.[0];
      if (!file) return;

      try {
        const text = await file.text();
        const strategy = JSON.parse(text);

        currentStrategy = { ...strategy, id: null };

        const nameInput = $("#strategy-name");
        const typeSelect = $("#strategy-type");

        if (nameInput) nameInput.value = currentStrategy.name;
        if (typeSelect) typeSelect.value = currentStrategy.type;

        Utils.showToast("Strategy imported", "success");
      } catch (error) {
        console.error("Failed to import strategy:", error);
        Utils.showToast("Failed to import strategy", "error");
      }
    };

    input.click();
  }

  function useTemplate(templateId) {
    const template = templates.find((t) => t.id === templateId);
    if (!template) return;

    currentStrategy = {
      id: null,
      name: template.name,
      type: "ENTRY",
      enabled: false,
      priority: 10,
      rules: template.rules,
      parameters: template.parameters || {},
    };

    // Update UI
    const nameInput = $("#strategy-name");
    const typeSelect = $("#strategy-type");

    if (nameInput) nameInput.value = currentStrategy.name;
    if (typeSelect) typeSelect.value = currentStrategy.type;

    // Render into vertical editor
    parseRuleTreeToConditions(currentStrategy.rules);
    renderConditionsList();

    // Switch to strategies tab
    const strategiesTab = $(".tab-btn[data-tab='strategies']");
    if (strategiesTab) strategiesTab.click();

    Utils.showToast(`Loaded template: ${template.name}`, "success");
  }

  function updateValidationStatus(valid, message) {
    const status = $("#validation-status");
    if (!status) return;

    const icon = status.querySelector(".status-icon");
    const text = status.querySelector(".status-text");

    if (valid) {
      status.classList.remove("invalid");
      status.classList.add("valid");
      if (icon) icon.innerHTML = '<i class="icon-check"></i>';
    } else {
      status.classList.remove("valid");
      status.classList.add("invalid");
      if (icon) icon.innerHTML = '<i class="icon-x"></i>';
    }

    if (text) text.textContent = message;
  }
}

// Register page so router can init/activate it
registerPage("strategies", createLifecycle());
