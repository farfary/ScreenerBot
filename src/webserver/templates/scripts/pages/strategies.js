import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { requestManager } from "../core/request_manager.js";

export function createLifecycle() {
  // State
  let currentStrategy = null;
  let strategies = [];
  let templates = [];
  let conditionSchemas = null;

  // Editor state (vertical cards)
  let conditions = []; // [{ type, name, enabled, required, params: {k: v} }]

  // Pollers
  let strategiesPoller = null;
  let templatesPoller = null;

  // Event listener cleanup tracking
  const eventCleanups = [];
  let dynamicCleanupStart = 0; // Track where dynamic listeners start

  // Helper to track event listeners for cleanup
  function addTrackedListener(element, event, handler) {
    element.addEventListener(event, handler);
    eventCleanups.push(() => element.removeEventListener(event, handler));
  }

  // Helper to clear only dynamic listeners (added during render)
  function clearDynamicListeners() {
    // Remove dynamic listeners from the end of the array
    while (eventCleanups.length > dynamicCleanupStart) {
      const cleanup = eventCleanups.pop();
      cleanup();
    }
  }

  return {
    async init(_ctx) {
      console.log("[Strategies] Initializing page");

      // Setup tab switching
      setupTabs();

      // Setup filters
      setupFilters();

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

      // Mark where static listeners end (everything before this is in init)
      dynamicCleanupStart = eventCleanups.length;

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
      eventCleanups.forEach((cleanup) => cleanup());
      eventCleanups.length = 0;

      // Cleanup state
      currentStrategy = null;
      strategies = [];
      templates = [];
      conditions = [];
    },
  };

  // Tab System
  function setupTabs() {
    const tabButtons = $$(".sidebar-tabs .tab-btn");
    const tabContents = $$(".tab-content");

    tabButtons.forEach((button) => {
      addTrackedListener(button, "click", () => {
        const targetTab = button.dataset.tab;

        // Update buttons
        tabButtons.forEach((btn) => btn.classList.remove("active"));
        button.classList.add("active");

        // Update content
        tabContents.forEach((content) => {
          if (content.dataset.tab === targetTab) {
            content.classList.add("active");
          } else {
            content.classList.remove("active");
          }
        });
      });
    });
  }

  // Filters
  function setupFilters() {
    // Strategy type filters
    const filterButtons = $$(".strategy-filters .filter-btn");
    filterButtons.forEach((button) => {
      addTrackedListener(button, "click", () => {
        filterButtons.forEach((btn) => btn.classList.remove("active"));
        button.classList.add("active");

        const filter = button.dataset.filter;
        filterStrategies(filter);
      });
    });

    // Template filters
    const categorySelect = $("#template-category");
    const riskSelect = $("#template-risk");

    if (categorySelect) {
      addTrackedListener(categorySelect, "change", () => {
        filterTemplates();
      });
    }

    if (riskSelect) {
      addTrackedListener(riskSelect, "change", () => {
        filterTemplates();
      });
    }
  }

  // Sidebar Actions
  function setupSidebarActions() {
    // Create new strategy
    const createBtn = $("#create-strategy");
    if (createBtn) {
      addTrackedListener(createBtn, "click", () => {
        createNewStrategy();
      });
    }

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
      // Show all, restore saved states
      categories.forEach((cat) => {
        cat.style.display = "block";
        const header = cat.querySelector(".category-header");
        if (!header) return;
        const categoryName = header.dataset.category;
        let savedStates = {};
        try {
          savedStates = JSON.parse(localStorage.getItem("condition-category-states") || "{}");
        } catch (e) {
          console.warn("[Strategies] Invalid category states, resetting:", e);
        }
        const items = cat.querySelector(".category-items");
        const isCollapsed = savedStates[categoryName] !== false;

        if (isCollapsed) {
          items.classList.add("collapsed");
          header.classList.add("collapsed");
        } else {
          items.classList.remove("collapsed");
          header.classList.remove("collapsed");
        }
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
    const deployBtn = $("#deploy-strategy");

    if (saveBtn) {
      addTrackedListener(saveBtn, "click", async () => {
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

    if (deployBtn) {
      addTrackedListener(deployBtn, "click", async () => {
        await deployStrategy();
      });
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
    // Clear old dynamic listeners before re-render
    clearDynamicListeners();

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
          <div class="strategy-item-title">
            <i class="${strategy.type === "ENTRY" ? "icon-trending-up" : "icon-trending-down"}"></i>
            ${Utils.escapeHtml(strategy.name)}
          </div>
          <div class="strategy-item-actions">
            <button class="btn-icon" data-action="edit" title="Edit"><i class="icon-edit"></i></button>
            <button class="btn-icon" data-action="toggle" title="${strategy.enabled ? "Disable" : "Enable"}">
              ${strategy.enabled ? "✓" : "○"}
            </button>
            <button class="btn-icon" data-action="delete" title="Delete"><i class="icon-trash-2"></i></button>
          </div>
        </div>
        <div class="strategy-item-meta">
          <span class="strategy-badge ${strategy.type.toLowerCase()}">${strategy.type}</span>
          <span class="strategy-badge ${strategy.enabled ? "enabled" : "disabled"}">
            ${strategy.enabled ? "Enabled" : "Disabled"}
          </span>
          ${strategy.priority ? `<span class="strategy-badge">Priority: ${strategy.priority}</span>` : ""}
        </div>
      </div>
    `
      )
      .join("");

    // Attach event listeners
    $$(".strategy-item").forEach((item) => {
      const strategyId = item.dataset.strategyId;

      addTrackedListener(item, "click", (e) => {
        if (!e.target.closest(".btn-icon")) {
          loadStrategy(strategyId);
        }
      });

      // Action buttons
      const editBtn = item.querySelector("[data-action='edit']");
      const toggleBtn = item.querySelector("[data-action='toggle']");
      const deleteBtn = item.querySelector("[data-action='delete']");

      if (editBtn) {
        addTrackedListener(editBtn, "click", (e) => {
          e.stopPropagation();
          loadStrategy(strategyId);
        });
      }

      if (toggleBtn) {
        addTrackedListener(toggleBtn, "click", async (e) => {
          e.stopPropagation();
          await toggleStrategyEnabled(strategyId);
        });
      }

      if (deleteBtn) {
        addTrackedListener(deleteBtn, "click", async (e) => {
          e.stopPropagation();
          await deleteStrategy(strategyId);
        });
      }
    });
  }

  function renderTemplates() {
    // Clear old dynamic listeners before re-render
    clearDynamicListeners();

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
        addTrackedListener(useBtn, "click", () => {
          const templateId = item.dataset.templateId;
          useTemplate(templateId);
        });
      }
    });
  }

  function initializeConditionCatalog() {
    const container = $("#condition-categories");
    if (!container || !conditionSchemas) return;

    // Build categories from schema metadata; hide non-strategy origins
    const categories = {};
    Object.entries(conditionSchemas).forEach(([type, schema]) => {
      if (schema.origin && String(schema.origin).toLowerCase() !== "strategy") return;
      const cat = schema.category || "General";
      if (!categories[cat]) categories[cat] = [];
      categories[cat].push({ type, ...schema });
    });

    // Load saved category states (default: all collapsed)
    let savedStates = {};
    try {
      savedStates = JSON.parse(localStorage.getItem("condition-category-states") || "{}");
    } catch (e) {
      console.warn("[Strategies] Invalid category states, resetting:", e);
    }

    // Render
    container.innerHTML = Object.entries(categories)
      .map(([category, list]) => {
        const isCollapsed = savedStates[category] !== false; // Default to collapsed
        return `
          <div class="condition-category">
            <div class="category-header ${isCollapsed ? "collapsed" : ""}" data-category="${category}">
              <div class="category-title">
                <span class="icon">${getCategoryIcon(category)}</span>
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
      const handler = () => {
        const category = header.dataset.category;
        const items = header.nextElementSibling;
        const toggle = header.querySelector(".category-toggle");
        const isCollapsed = items.classList.contains("collapsed");

        if (isCollapsed) {
          items.classList.remove("collapsed");
          header.classList.remove("collapsed");
          toggle.textContent = "▼";
        } else {
          items.classList.add("collapsed");
          header.classList.add("collapsed");
          toggle.textContent = "▶";
        }

        // Save state
        let states = {};
        try {
          states = JSON.parse(localStorage.getItem("condition-category-states") || "{}");
        } catch (e) {
          console.warn("[Strategies] Invalid category states, resetting:", e);
        }
        states[category] = !isCollapsed;
        try {
          localStorage.setItem("condition-category-states", JSON.stringify(states));
        } catch (e) {
          console.warn("[Strategies] Failed to save category states:", e);
        }
      };
      header.addEventListener("click", handler);
      eventCleanups.push(() => header.removeEventListener("click", handler));
    });

    // Click to add (with cleanup tracking)
    $$(".condition-item").forEach((item) => {
      const handler = () => {
        const type = item.dataset.conditionType;
        addCondition(type);
        const catalog = $("#condition-catalog-modal");
        if (catalog) catalog.classList.remove("active");
      };
      item.addEventListener("click", handler);
      eventCleanups.push(() => item.removeEventListener("click", handler));
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

  // Helper Functions
  function getCategoryIcon(category) {
    const icons = {
      "Price Patterns": "icon-dollar-sign",
      "Technical Indicators": "icon-bar-chart-2",
      "Market Context": "icon-globe",
      "Position & Performance": "icon-trending-up",
    };
    return icons[category] || "icon-bookmark";
  }

  function getConditionIcon(type) {
    const icons = {
      PriceThreshold: "icon-target",
      PriceMovement: "icon-trending-up",
      RelativeToMA: "icon-trending-down",
      LiquidityDepth: "icon-droplet",
      PositionAge: "icon-timer",
    };
    return icons[type] || "icon-circle";
  }

  function filterStrategies(filter) {
    const items = $$(".strategy-item");
    items.forEach((item) => {
      const strategyId = item.dataset.strategyId;
      const strategy = strategies.find((s) => s.id === strategyId);

      if (!strategy) {
        item.style.display = "none";
        return;
      }

      if (filter === "all") {
        item.style.display = "";
      } else if (filter === "entry") {
        item.style.display = strategy.type === "ENTRY" ? "" : "none";
      } else if (filter === "exit") {
        item.style.display = strategy.type === "EXIT" ? "" : "none";
      }
    });
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
    const params = {};
    Object.entries(schema.parameters || {}).forEach(([k, p]) => {
      params[k] = p.default ?? null;
    });
    conditions.push({
      type: conditionType,
      name: schema.name || conditionType,
      enabled: true,
      required: true,
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
      list.innerHTML = `<div class="empty-state"><i class="icon-puzzle"></i><p>No conditions yet</p><small>Use "Add Condition" to start building</small></div>`;
      return;
    }

    // Clear old dynamic listeners before re-rendering
    clearDynamicListeners();

    list.innerHTML = conditions.map((c, idx) => renderConditionCard(c, idx)).join("");

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
        btn.addEventListener("click", handler);
        eventCleanups.push(() => btn.removeEventListener("click", handler));
      }
    });

    // Toggles and param inputs with cleanup tracking
    $$(".condition-card .toggle-enabled").forEach((el) => {
      const handler = (e) => {
        const idx = parseInt(el.closest(".condition-card").dataset.index, 10);
        conditions[idx].enabled = e.target.checked;
        updateRuleTreeFromEditor();
      };
      el.addEventListener("change", handler);
      eventCleanups.push(() => el.removeEventListener("change", handler));
    });
    $$(".condition-card .toggle-required").forEach((el) => {
      const handler = (e) => {
        const idx = parseInt(el.closest(".condition-card").dataset.index, 10);
        conditions[idx].required = e.target.checked;
        // For now, required flag is cosmetic; combinator remains global AND
      };
      el.addEventListener("change", handler);
      eventCleanups.push(() => el.removeEventListener("change", handler));
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
          const summary = card.querySelector(".condition-summary");
          if (summary) summary.textContent = buildConditionSummary(conditions[idx]);
        };
        input.addEventListener("change", handler);
        eventCleanups.push(() => input.removeEventListener("change", handler));
      }
    );
  }

  function renderConditionCard(c, idx) {
    const schema = conditionSchemas?.[c.type] || {};
    const iconClass = schema.icon || getConditionIcon(c.type);
    const badges = [schema.category || "General"]
      .map((b) => `<span class="condition-badge">${Utils.escapeHtml(b)}</span>`)
      .join("");
    const summary = buildConditionSummary(c);
    const body = renderParamEditor(c, schema, idx);
    return `
      <div class="condition-card" data-index="${idx}">
        <div class="card-header">
          <div class="card-title"><i class="${iconClass}"></i>${Utils.escapeHtml(c.name || c.type)}</div>
          <div class="card-meta">
            ${badges}
            <label><input type="checkbox" class="toggle-enabled" ${c.enabled ? "checked" : ""}/> Enabled</label>
            <label><input type="checkbox" class="toggle-required" ${c.required ? "checked" : ""}/> Required</label>
            <div class="condition-actions">
              <button class="btn-icon" data-action="move-up" title="Move up">▲</button>
              <button class="btn-icon" data-action="move-down" title="Move down">▼</button>
              <button class="btn-icon" data-action="duplicate" title="Duplicate"><i class="icon-copy"></i></button>
              <button class="btn-icon" data-action="delete" title="Delete"><i class="icon-trash-2"></i></button>
              <button class="btn-icon" data-action="toggle-expand" title="More">⋯</button>
            </div>
          </div>
        </div>
        <div class="card-header" style="padding-top:0;">
          <div class="condition-summary">${Utils.escapeHtml(summary)}</div>
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
    const keys = Object.keys(schema.parameters || {}).slice(0, 3);
    const parts = keys.map((k) => `${k}=${formatParamValue(c.params[k])}`);
    return parts.join(", ") || "No parameters";
  }

  function formatParamValue(v) {
    if (v === undefined || v === null) return "";
    if (typeof v === "number") return String(v);
    if (typeof v === "boolean") return v ? "true" : "false";
    return String(v);
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
    switch (spec.type) {
      case "number":
      case "percent":
      case "sol":
        return `<input id="${id}" ${data} type="number" value="${value}" ${spec.min !== undefined ? `min="${spec.min}"` : ""} ${spec.max !== undefined ? `max="${spec.max}"` : ""} ${spec.step !== undefined ? `step="${spec.step}"` : ""}>`;
      case "boolean":
        return `<input id="${id}" ${data} type="checkbox" ${value ? "checked" : ""}>`;
      case "enum": {
        // Handle both old format (string array) and new format (object array with value/label)
        const options = spec.options || spec.values || [];
        const optionsHtml = options
          .map((opt) => {
            // Check if option is an object with value/label or a simple string
            const optValue = typeof opt === "object" ? opt.value : opt;
            const optLabel = typeof opt === "object" ? opt.label : opt;
            const selected = optValue === value ? "selected" : "";
            return `<option value="${Utils.escapeHtml(String(optValue))}" ${selected}>${Utils.escapeHtml(String(optLabel))}</option>`;
          })
          .join("");
        return `<select id="${id}" ${data}>${optionsHtml}</select>`;
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
          <i class="icon-alert-triangle"></i>
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
      nameInput.addEventListener("input", handler);
      eventCleanups.push(() => nameInput.removeEventListener("input", handler));
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
          input.addEventListener("input", handler);
          input.addEventListener("change", handler);
          eventCleanups.push(() => {
            input.removeEventListener("input", handler);
            input.removeEventListener("change", handler);
          });
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
            inputHtml = `<select class="property-input" id="param-${key}">
              ${opts.map((v) => `<option value="${v}" ${v === effectiveValue ? "selected" : ""}>${v}</option>`).join("")}
            </select>`;
          } else {
            inputHtml = `<input type="text" class="property-input" id="param-${key}" value="${Utils.escapeHtml(String(effectiveValue))}">`;
          }
        }
        break;

      default:
        if (schema.options && Array.isArray(schema.options) && schema.options.length > 0) {
          inputHtml = `<select class="property-input" id="param-${key}">
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

    // Close handlers
    const closeModal = () => {
      modal.classList.remove("active");
    };

    closeBtn.onclick = closeModal;
    cancelBtn.onclick = closeModal;

    // Click outside to close
    modal.onclick = (e) => {
      if (e.target === modal) {
        closeModal();
      }
    };

    // Apply changes
    applyBtn.onclick = () => {
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

    // ESC key to close (with cleanup tracking)
    const escHandler = (e) => {
      if (e.key === "Escape") {
        closeModal();
        document.removeEventListener("keydown", escHandler);
      }
    };
    document.addEventListener("keydown", escHandler);
    eventCleanups.push(() => document.removeEventListener("keydown", escHandler));
  }

  // Strategy Operations
  function createNewStrategy() {
    currentStrategy = {
      id: null,
      name: "New Strategy",
      type: "ENTRY",
      enabled: true,
      priority: 10,
      rules: null,
      parameters: {},
    };

    // Update UI
    const nameInput = $("#strategy-name");
    const typeSelect = $("#strategy-type");

    if (nameInput) nameInput.value = currentStrategy.name;
    if (typeSelect) typeSelect.value = currentStrategy.type;

    // Clear editor conditions
    conditions = [];
    renderConditionsList();
    renderPropertiesPanel(null);

    Utils.showToast("New strategy created", "success");
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
      const typeSelect = $("#strategy-type");

      if (nameInput) nameInput.value = currentStrategy.name;
      if (typeSelect) typeSelect.value = currentStrategy.type;

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
        required: true,
        params,
      });
    });
  }

  async function saveStrategy() {
    if (!currentStrategy) {
      Utils.showToast("No strategy to save", "warning");
      return;
    }

    try {
      // Get current values from UI
      const nameInput = $("#strategy-name");
      const typeSelect = $("#strategy-type");

      if (nameInput) currentStrategy.name = nameInput.value;
      if (typeSelect) currentStrategy.type = typeSelect.value;

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
      return;
    }

    try {
      const data = await requestManager.fetch(`/api/strategies/${currentStrategy.id}/validate`, {
        method: "POST",
        priority: "high",
      });

      if (data.valid) {
        updateValidationStatus(true, "Strategy is valid");
        Utils.showToast("Strategy is valid", "success");
      } else {
        updateValidationStatus(false, data.errors?.join(", ") || "Invalid strategy");
        Utils.showToast("Strategy has errors", "error");
      }
    } catch (error) {
      console.error("Validation failed:", error);
      updateValidationStatus(false, error.message);
      Utils.showToast("Validation failed", "error");
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

  async function deployStrategy() {
    if (!currentStrategy?.id) {
      Utils.showToast("Please save the strategy first", "warning");
      return;
    }

    const { confirmed } = await ConfirmationDialog.show({
      title: "Deploy Strategy",
      message: `Deploy strategy "${currentStrategy.name}"? This will enable it for live trading.`,
      confirmLabel: "Deploy",
      cancelLabel: "Cancel",
      variant: "warning",
    });

    if (!confirmed) return;

    try {
      await requestManager.fetch(`/api/strategies/${currentStrategy.id}/deploy`, {
        method: "POST",
        priority: "high",
      });

      await loadStrategies();
      Utils.showToast("Strategy deployed successfully", "success");
    } catch (error) {
      console.error("Deploy failed:", error);
      Utils.showToast("Deploy failed", "error");
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
      if (icon) icon.textContent = "✓";
    } else {
      status.classList.remove("valid");
      status.classList.add("invalid");
      if (icon) icon.textContent = "✗";
    }

    if (text) text.textContent = message;
  }
}

// Register page so router can init/activate it
registerPage("strategies", createLifecycle());
