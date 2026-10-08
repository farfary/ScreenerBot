// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Strategies page - strategy list and condition editor with create, validate and delete flows over the /api/strategies endpoints.

import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { STRATEGY_TYPE_LABELS } from "../ui/strategy_type.js";
import { requestManager } from "../core/request_manager.js";
import { enhanceAllSelects } from "../ui/custom_select.js";
import { createConditionEditor } from "./strategies/condition_editor.js";
import { createConditionCatalog } from "./strategies/condition_catalog.js";

export function createLifecycle() {
  // State
  let currentStrategy = null;
  let strategies = [];
  let conditionSchemas = null;
  let defaultTimeframe = null; // timeframe a new strategy is saved with
  let categoryStates = { data: null }; // Wrapped in object to pass by reference

  // Editor state (vertical cards)
  let conditions = []; // [{ type, enabled, params: {k: v} }]

  // Pollers
  let strategiesPoller = null;

  // Hash guards — skip re-render when polled data is unchanged
  let _lastStrategiesKey = null;

  // Dirty tracking — unsaved editor changes
  let _isDirty = false;
  let _loadingStrategy = false; // suppress dirty during load

  // Event listener cleanup tracking
  const eventCleanups = [];
  const CleanupScope = {
    STATIC: "static",
    STRATEGIES_LIST: "strategies-list",
    CONDITION_CARDS: "condition-cards",
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

  // Editor visibility + dirty state helpers
  function showEditor() {
    const header = $("#editor-header");
    const empty = $("#editor-empty");
    const scrollArea = $("#editor-scroll-area");
    const footer = $("#editor-footer");
    if (header) header.style.display = "flex";
    if (empty) empty.style.display = "none";
    if (scrollArea) scrollArea.style.display = "";
    if (footer) footer.style.display = "";
  }

  function hideEditor() {
    const header = $("#editor-header");
    const empty = $("#editor-empty");
    const scrollArea = $("#editor-scroll-area");
    const footer = $("#editor-footer");
    if (header) header.style.display = "none";
    if (empty) empty.style.display = "";
    if (scrollArea) scrollArea.style.display = "none";
    if (footer) footer.style.display = "none";
  }

  function markDirty() {
    if (_isDirty) return;
    _isDirty = true;
    const header = $("#editor-header");
    if (header) header.classList.add("dirty");
  }

  function clearDirty() {
    _isDirty = false;
    const header = $("#editor-header");
    if (header) header.classList.remove("dirty");
  }

  // Toast with a title and a body; both come from one message and its `.message` attribute.
  function announce(type, title, message) {
    Utils.showToast({ type, title, message });
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
    announce,
    confirm: (config) => ConfirmationDialog.show(config),
    enhanceAllSelects,
    addTrackedListener,
    clearScope,
    CleanupScope,
  });

  const conditionCatalog = createConditionCatalog({
    conditionSchemas: new Proxy({}, {
      get: (_, prop) => conditionSchemas?.[prop],
      ownKeys: () => (conditionSchemas ? Reflect.ownKeys(conditionSchemas) : []),
      getOwnPropertyDescriptor: (_, prop) =>
        conditionSchemas && prop in conditionSchemas
          ? { configurable: true, enumerable: true, writable: false }
          : undefined,
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

  // Wrap conditionEditor.renderConditionsList to mark dirty on user edits
  {
    const _origRender = conditionEditor.renderConditionsList.bind(conditionEditor);
    conditionEditor.renderConditionsList = (...args) => {
      if (currentStrategy && !_loadingStrategy) markDirty();
      return _origRender(...args);
    };
  }

  return {
    async init(_ctx) {
      console.log("[Strategies] Initializing page");

      // Hide editor area until a strategy is selected
      hideEditor();

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
          { label: "Strategies", intervalMs: 10000 } // l10n-ignore: poller label used in logs only
        )
      );

      // Start pollers
      strategiesPoller.start();

      // Initial load
      await loadStrategies();
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
      conditions = [];
      _lastStrategiesKey = null;
      _isDirty = false;
      _loadingStrategy = false;
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

  }

  function showCreateStrategyModal() {
    const modal = $("#create-strategy-modal");
    if (modal) modal.hidden = false;
  }

  function hideCreateStrategyModal() {
    const modal = $("#create-strategy-modal");
    if (modal) modal.hidden = true;
  }

  function setupCreateStrategyModal() {
    const modal = $("#create-strategy-modal");
    const cancelBtn = $("#cancel-create-strategy");
    const closeBtn = $("#create-strategy-close");
    const typeCards = $$(".type-card");

    if (cancelBtn) {
      addTrackedListener(cancelBtn, "click", hideCreateStrategyModal);
    }

    if (closeBtn) {
      addTrackedListener(closeBtn, "click", hideCreateStrategyModal);
    }

    if (modal) {
      addTrackedListener(modal, "click", (event) => {
        if (event.target === modal) hideCreateStrategyModal();
      });
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
    const catalog = $("#condition-catalog-modal");
    const closeCatalog = $("#close-condition-catalog");
    const searchInput = $("#condition-search");

    if (addBtn) {
      addTrackedListener(addBtn, "click", () => openConditionCatalog());
    }
    if (closeCatalog && catalog) {
      addTrackedListener(closeCatalog, "click", () => (catalog.hidden = true));
      addTrackedListener(catalog, "click", (e) => {
        if (e.target === catalog) catalog.hidden = true;
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
    const validateBtn = $("#validate-strategy");
    const enableToggle = $("#strategy-enabled-toggle");
    const nameInput = $("#strategy-name");

    // Sync strategy name in real-time as user types
    if (nameInput) {
      addTrackedListener(nameInput, "input", (e) => {
        if (currentStrategy) {
          currentStrategy.name = e.target.value.trim();
          markDirty();
        }
      });
    }

    if (saveBtn) {
      addTrackedListener(saveBtn, "click", async () => {
        // Validate first before saving
        const isValid = await validateStrategy();
        if (!isValid) {
          Utils.showToast(I18n.t("strategies-toast-fix-validation"), "error");
          return;
        }
        await saveStrategy();
      });
    }

    if (validateBtn) {
      addTrackedListener(validateBtn, "click", async () => {
        await validateStrategy();
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

  /** Create, update and validate request body of the strategy being edited. */
  function strategyRequestBody() {
    return {
      name: currentStrategy.name,
      description: currentStrategy.description || null,
      strategy_type: currentStrategy.type,
      enabled: !!currentStrategy.enabled,
      priority: currentStrategy.priority ?? 10,
      timeframe: currentStrategy.timeframe || undefined,
      rules: currentStrategy.rules || null,
      parameters: currentStrategy.parameters || {},
      author: currentStrategy.author || null,
    };
  }

  async function toggleCurrentStrategyEnabled() {
    if (!currentStrategy?.id) return;

    try {
      const body = strategyRequestBody();

      await requestManager.fetch(`/api/strategies/${currentStrategy.id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
        priority: "high",
      });

      await loadStrategies();
      const args = { name: currentStrategy.name };
      if (currentStrategy.enabled) {
        announce(
          "success",
          I18n.t("strategies-toast-enabled", args),
          I18n.attr("strategies-toast-enabled", "message", args)
        );
      } else {
        announce(
          "success",
          I18n.t("strategies-toast-disabled", args),
          I18n.attr("strategies-toast-disabled", "message", args)
        );
      }
    } catch (error) {
      console.error("Failed to toggle strategy:", error);
      announce(
        "error",
        I18n.t("strategies-toast-toggle-failed"),
        I18n.attr("strategies-toast-toggle-failed", "message")
      );
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
      announce(
        "error",
        I18n.t("strategies-toast-load-failed"),
        I18n.attr("strategies-toast-load-failed", "message")
      );
    }
  }

  async function loadConditionSchemas() {
    try {
      const data = await requestManager.fetch("/api/strategies/conditions/schemas", {
        priority: "normal",
      });
      conditionSchemas = data.schemas || {};
      defaultTimeframe = data.default_timeframe ?? null;
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
          <p>${Utils.escapeHtml(I18n.t("strategies-list-empty-title"))}</p>
          <small>${Utils.escapeHtml(I18n.t("strategies-list-empty-hint"))}</small>
        </div>
      `;
      return;
    }

    listContainer.innerHTML = strategies
      .map((strategy) => {
        const deleteTitle = I18n.t("common-action-delete");
        const toggleTitle = strategy.enabled
          ? I18n.attr("strategies-item-disable", "title")
          : I18n.attr("strategies-item-enable", "title");
        return `
      <div class="strategy-item ${currentStrategy?.id === strategy.id ? "active" : ""}"
           data-strategy-id="${strategy.id}">
        <div class="strategy-item-name">${Utils.escapeHtml(strategy.name)}</div>
        <div class="strategy-item-row">
          <div class="strategy-item-tags">
            <span class="strategy-type-tag ${strategy.type.toLowerCase()}">${Utils.escapeHtml(I18n.label(STRATEGY_TYPE_LABELS, strategy.type))}</span>
            <span class="strategy-state-tag ${strategy.enabled ? "enabled" : "disabled"}">${Utils.escapeHtml(strategy.enabled ? I18n.t("common-state-enabled") : I18n.t("common-state-disabled"))}</span>
          </div>
          <div class="strategy-item-btns">
            <button class="btn-icon btn-icon-sm" data-action="toggle" title="${Utils.escapeHtml(toggleTitle)}" aria-label="${Utils.escapeHtml(toggleTitle)}">
              <i class="${strategy.enabled ? "icon-toggle-right" : "icon-toggle-left"}"></i>
            </button>
            <button class="btn-icon btn-icon-sm" data-action="delete" title="${Utils.escapeHtml(deleteTitle)}" aria-label="${Utils.escapeHtml(deleteTitle)}"><i class="icon-trash-2"></i></button>
          </div>
        </div>
      </div>
    `;
      })
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

  function openConditionCatalog() {
    const modal = $("#condition-catalog-modal");
    if (modal) modal.hidden = false;
  }

  // Strategy Operations
  function createNewStrategy(strategyType = "ENTRY") {
    currentStrategy = {
      id: null,
      name: I18n.t("strategies-new-name"),
      type: strategyType,
      enabled: true,
      priority: 10,
      timeframe: defaultTimeframe,
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

    // Clear editor conditions (suppress dirty during reset)
    _loadingStrategy = true;
    conditions.length = 0;
    conditionEditor.renderConditionsList();
    _loadingStrategy = false;

    showEditor();
    markDirty(); // new strategy is always unsaved
  }

  function updateTypeBadge(type) {
    const badge = $("#strategy-type-badge");
    if (!badge) return;

    badge.className = `strategy-type-badge ${type.toLowerCase()}`;
    const icon = type === "ENTRY" ? "icon-trending-up" : "icon-trending-down";
    badge.innerHTML = `<i class="${icon}"></i> ${Utils.escapeHtml(I18n.label(STRATEGY_TYPE_LABELS, type))}`;
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
        timeframe: data.timeframe,
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

      // Render strategy into vertical editor (suppress dirty during load)
      _loadingStrategy = true;
      conditionEditor.parseRuleTreeToConditions(currentStrategy.rules);
      conditionEditor.renderConditionsList();
      _loadingStrategy = false;

      showEditor();
      clearDirty();

      // Update active state in list
      $$(".strategy-item").forEach((item) => {
        if (item.dataset.strategyId === strategyId) {
          item.classList.add("active");
        } else {
          item.classList.remove("active");
        }
      });
    } catch (error) {
      console.error("Failed to load strategy:", error);
      Utils.showToast(I18n.t("strategies-toast-load-strategy-failed"), "error");
    }
  }

  async function saveStrategy() {
    if (!currentStrategy) {
      announce(
        "error",
        I18n.t("strategies-toast-no-strategy"),
        I18n.attr("strategies-toast-no-strategy", "message")
      );
      return;
    }

    // Validate strategy has conditions
    if (conditions.length === 0) {
      announce(
        "warning",
        I18n.t("strategies-toast-no-conditions-save"),
        I18n.attr("strategies-toast-no-conditions-save", "message")
      );
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
        announce(
        "warning",
        I18n.t("strategies-toast-name-required"),
        I18n.attr("strategies-toast-name-required", "message")
      );
        if (nameInput) nameInput.focus();
        return;
      }

      // Sync rule tree from editor
      conditionEditor.updateRuleTreeFromEditor();

      const body = strategyRequestBody();

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

      clearDirty();
      await loadStrategies();
      const args = { name: currentStrategy.name };
      announce(
        "success",
        I18n.t("strategies-toast-saved", args),
        I18n.attr("strategies-toast-saved", "message", args)
      );
    } catch (error) {
      console.error("Failed to save strategy:", error);
      announce(
        "error",
        I18n.t("strategies-toast-save-failed"),
        I18n.attr("strategies-toast-save-failed", "message")
      );
    }
  }

  async function validateStrategy() {
    if (!currentStrategy) {
      Utils.showToast(I18n.t("strategies-toast-no-strategy-validate"), "warning");
      return false;
    }

    // Make sure rule tree is up to date
    conditionEditor.updateRuleTreeFromEditor();

    // Check if strategy has conditions
    if (!currentStrategy.rules) {
      announce(
        "warning",
        I18n.t("strategies-toast-no-conditions-validate"),
        I18n.attr("strategies-toast-no-conditions-validate", "message")
      );
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
        const body = strategyRequestBody();

        data = await requestManager.fetch("/api/strategies/validate", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(body),
          priority: "high",
        });
      }

      if (data.valid) {
        Utils.showToast(I18n.t("strategies-toast-valid"), "success");
        return true;
      }
      const reasons = (data.errors || []).map((error) => I18n.text(error));
      announce(
        "error",
        I18n.t("strategies-toast-invalid"),
        Utils.formatList(reasons, { type: "unit" })
      );
      return false;
    } catch (error) {
      console.error("Validation failed:", error);
      Utils.showToast(I18n.t("strategies-toast-validation-failed"), "error");
      return false;
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
      Utils.showToast(
        strategy.enabled
          ? I18n.t("strategies-toast-item-disabled")
          : I18n.t("strategies-toast-item-enabled"),
        "success"
      );
    } catch (error) {
      console.error("Failed to toggle strategy:", error);
      Utils.showToast(I18n.t("strategies-toast-item-toggle-failed"), "error");
    }
  }

  async function deleteStrategy(strategyId) {
    const strategy = strategies.find((s) => s.id === strategyId);
    if (!strategy) return;

    const { confirmed } = await ConfirmationDialog.show({
      title: I18n.t("strategies-delete-title"),
      message: I18n.t("strategies-delete-message", { name: strategy.name }),
      confirmLabel: I18n.t("common-action-delete"),
      cancelLabel: I18n.t("common-action-cancel"),
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
      const args = { name: strategy.name };
      announce(
        "success",
        I18n.t("strategies-toast-deleted", args),
        I18n.attr("strategies-toast-deleted", "message", args)
      );
    } catch (error) {
      console.error("Failed to delete strategy:", error);
      announce(
        "error",
        I18n.t("strategies-toast-delete-failed"),
        I18n.attr("strategies-toast-delete-failed", "message")
      );
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

        Utils.showToast(I18n.t("strategies-toast-imported"), "success");
      } catch (error) {
        console.error("Failed to import strategy:", error);
        Utils.showToast(I18n.t("strategies-toast-import-failed"), "error");
      }
    };

    input.click();
  }
}

// Strategies is no longer a standalone router page — it is embedded as the
// third subtab of the Auto Trader page, which imports createLifecycle() and
// drives init/activate/deactivate/dispose itself (see trader.js).
