import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { InputDialog } from "../ui/input_dialog.js";
import { requestManager } from "../core/request_manager.js";
import { enhanceAllSelects } from "../ui/custom_select.js";
import { createConditionEditor } from "./strategies/condition_editor.js";
import { createConditionCatalog } from "./strategies/condition_catalog.js";

export function createLifecycle() {
  // State
  let currentStrategy = null;
  let strategies = [];
  let templates = [];
  let conditionSchemas = null;
  let categoryStates = { data: null }; // Wrapped in object to pass by reference

  // Editor state (vertical cards)
  let conditions = []; // [{ type, name, enabled, params: {k: v} }]

  // Pollers
  let strategiesPoller = null;
  let templatesPoller = null;

  // Hash guards — skip re-render when polled data is unchanged
  let _lastStrategiesKey = null;
  let _lastTemplatesKey = null;

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

  // Initialize sub-modules
  const conditionEditor = createConditionEditor({
    state: { get currentStrategy() { return currentStrategy; }, set currentStrategy(val) { currentStrategy = val; } },
    conditions,
    conditionSchemas: new Proxy({}, {
      get: (_, prop) => conditionSchemas?.[prop]
    }),
    $,
    $$,
    Utils,
    enhanceAllSelects,
    addTrackedListener,
    clearScope,
    CleanupScope,
  });

  const conditionCatalog = createConditionCatalog({
    conditionSchemas: new Proxy({}, {
      get: (_, prop) => conditionSchemas?.[prop]
    }),
    categoryStates,
    $,
    $$,
    Utils,
    AppState,
    addTrackedListener,
    clearScope,
    CleanupScope,
  });

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
      conditionCatalog.initializeConditionCatalog(conditionEditor.addCondition);
    },

    async activate(ctx) {
      console.log("[Strategies] Activating page");

      // Create pollers
      strategiesPoller = ctx.managePoller(
        new Poller(
          async () => {
            await loadStrategies();
          },
          { label: "Strategies", intervalMs: 10000 }
        )
      );

      templatesPoller = ctx.managePoller(
        new Poller(
          async () => {
            await loadTemplates();
          },
          { label: "Templates", intervalMs: 30000 }
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
      _lastStrategiesKey = null;
      _lastTemplatesKey = null;
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
      const states = conditionCatalog.getCategoryStates();
      // Show all, restore saved states
      categories.forEach((cat) => {
        cat.style.display = "block";
        const header = cat.querySelector(".category-header");
        if (!header) return;
        const collapsed = states[header.dataset.category] !== false;
        conditionCatalog.applyCategoryCollapsedState(header, collapsed);
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
    const key = JSON.stringify(strategies.map(s => ({ ...s, active: currentStrategy?.id === s.id })));
    if (key === _lastStrategiesKey) return;
    _lastStrategiesKey = key;

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
    const key = JSON.stringify(templates);
    if (key === _lastTemplatesKey) return;
    _lastTemplatesKey = key;

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
        conditionEditor.renderConditionsList();
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
            conditionEditor.updateRuleTreeFromEditor();
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

      conditionEditor.updateRuleTreeFromEditor();
      conditionEditor.renderConditionsList();
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
    conditions.length = 0; // Clear array (mutable reference)
    conditionEditor.renderConditionsList();
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
      conditionEditor.parseRuleTreeToConditions(currentStrategy.rules);
      conditionEditor.renderConditionsList();

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
      conditionEditor.updateRuleTreeFromEditor();

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

    const result = await InputDialog.show({
      title: "Duplicate Strategy",
      message: "Enter name for the copied strategy",
      placeholder: "Strategy name...",
      defaultValue: `${currentStrategy.name} (Copy)`,
      confirmLabel: "Save",
      validate: (value) => {
        if (!value || !value.trim()) return "Strategy name is required";
        return null;
      },
      formatValue: (value) => value.trim(),
    });
    if (!result) return;
    const newName = result.value;
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

    // Make sure rule tree is up to date
    conditionEditor.updateRuleTreeFromEditor();

    // Check if strategy has conditions
    if (!currentStrategy.rules) {
      Utils.showToast({
        type: "warning",
        title: "No Conditions",
        message: "Add at least one condition before validating",
      });
      updateValidationStatus(false, "No conditions defined");
      return false;
    }

    try {
      let data;

      if (currentStrategy.id) {
        // Strategy exists on server - use ID-based validation
        data = await requestManager.fetch(`/api/strategies/${currentStrategy.id}/validate`, {
          method: "POST",
          priority: "high",
        });
      } else {
        // Unsaved strategy - use inline validation with JSON body
        const body = {
          name: currentStrategy.name || "Untitled",
          description: currentStrategy.description || null,
          strategy_type: currentStrategy.type,
          enabled: !!currentStrategy.enabled,
          priority: currentStrategy.priority ?? 10,
          rules: currentStrategy.rules,
          parameters: currentStrategy.parameters || {},
          author: currentStrategy.author || null,
        };

        data = await requestManager.fetch("/api/strategies/validate", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(body),
          priority: "high",
        });
      }

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
    conditionEditor.parseRuleTreeToConditions(currentStrategy.rules);
    conditionEditor.renderConditionsList();

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
