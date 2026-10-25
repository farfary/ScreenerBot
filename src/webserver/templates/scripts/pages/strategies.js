// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Strategies page - strategy list and condition editor with create, validate and delete flows over the /api/strategies endpoints.

import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { renderStateView } from "../ui/state_view.js";
import { STRATEGY_TYPE_LABELS } from "../ui/strategy_type.js";
import { requestManager, errorMessageOf } from "../core/request_manager.js";
import { downloadJson } from "../core/download.js";
import { enhanceAllSelects } from "../ui/custom_select.js";
import { createRuleEditor } from "./strategies/rule_editor.js";
import { createConditionCatalog } from "./strategies/condition_catalog.js";
import { conditionName, optionLabel, optionValue } from "./strategies/condition_text.js";
import {
  DEFAULT_PRIORITY,
  exportEnvelope,
  exportFileName,
  isValidPriority,
  parseImport,
  requestBodyOf,
  strategyFromDetail,
  timeframeOptions,
} from "./strategies/strategy_io.js";

export function createLifecycle() {
  // State
  let currentStrategy = null;
  let strategies = [];
  const conditionSchemas = {}; // filled in place once the schemas are loaded
  let defaultTimeframe = null; // timeframe a new strategy is saved with

  // Pollers
  let strategiesPoller = null;

  // Hash guards — skip re-render when polled data is unchanged
  let _lastStrategiesKey = null;

  // Dirty tracking — unsaved editor changes
  let _isDirty = false;

  // Event listener cleanup tracking
  const eventCleanups = [];
  const CleanupScope = {
    STATIC: "static",
    STRATEGIES_LIST: "strategies-list",
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
  const ruleEditor = createRuleEditor({
    state: {
      get currentStrategy() {
        return currentStrategy;
      },
    },
    conditionSchemas,
    $,
    Utils,
    renderStateView,
    announce,
    confirm: (config) => ConfirmationDialog.show(config),
    enhanceAllSelects,
    addTrackedListener,
    onChange: markDirty,
    openCatalog: openConditionCatalog,
  });

  const conditionCatalog = createConditionCatalog({
    conditionSchemas,
    $,
    Utils,
    addTrackedListener,
  });

  return {
    async init(_ctx) {
      console.log("[Strategies] Initializing page");

      // Hide editor area until a strategy is selected
      hideEditor();

      setupFilterTabs();
      setupSidebarActions();
      setupHeaderActions();

      // Load condition schemas
      await loadConditionSchemas();

      ruleEditor.bind();
      conditionCatalog.bind();
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
      _lastStrategiesKey = null;
      _isDirty = false;
    },
  };

  // Strategy list filter tabs
  function setupFilterTabs() {
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
        startNewStrategy(type);
      });
    });
  }

  /** Editor header: every field edits the open strategy and marks it unsaved. */
  function setupHeaderActions() {
    const edit = (apply) => (event) => {
      if (!currentStrategy) return;
      apply(event.target);
      markDirty();
    };

    addTrackedListener(
      $("#strategy-name"),
      "input",
      edit((input) => {
        currentStrategy.name = input.value;
      })
    );
    addTrackedListener(
      $("#strategy-enabled-toggle"),
      "change",
      edit((input) => {
        currentStrategy.enabled = input.checked;
      })
    );
    addTrackedListener(
      $("#strategy-timeframe"),
      "change",
      edit((select) => {
        currentStrategy.timeframe = select.value;
        // Conditions without a timeframe of their own show the strategy's.
        ruleEditor.render();
      })
    );
    addTrackedListener(
      $("#strategy-priority"),
      "input",
      edit((input) => {
        currentStrategy.priority = input.value === "" ? null : Number(input.value);
        setPriorityInvalid(!isValidPriority(currentStrategy.priority));
      })
    );
    addTrackedListener(
      $("#strategy-description"),
      "input",
      edit((input) => {
        currentStrategy.description = input.value;
      })
    );
    addTrackedListener($("#toggle-description"), "click", () => {
      setDescriptionOpen($("#strategy-description-panel")?.hidden ?? false);
    });

    addTrackedListener($("#save-strategy"), "click", saveStrategy);
    addTrackedListener($("#validate-strategy"), "click", validateStrategy);
    addTrackedListener($("#export-strategy"), "click", exportStrategy);
  }

  function setPriorityInvalid(invalid) {
    const input = $("#strategy-priority");
    if (!input) return;
    if (invalid) input.setAttribute("aria-invalid", "true");
    else input.removeAttribute("aria-invalid");
  }

  function setDescriptionOpen(open) {
    const panel = $("#strategy-description-panel");
    const toggle = $("#toggle-description");
    if (panel) panel.hidden = !open;
    if (toggle) toggle.setAttribute("aria-expanded", String(open));
  }

  /** The timeframe select: the options candle conditions offer, plus a saved value outside them. */
  function populateTimeframe(value) {
    const select = $("#strategy-timeframe");
    if (!select) return;
    const options = timeframeOptions(conditionSchemas).map(
      (option) =>
        `<option value="${Utils.escapeHtml(String(optionValue(option)))}">${Utils.escapeHtml(optionLabel(option))}</option>`
    );
    if (value && !timeframeOptions(conditionSchemas).some((option) => optionValue(option) === value)) {
      const text = Utils.escapeHtml(String(value));
      options.push(`<option value="${text}">${text}</option>`);
    }
    select.innerHTML = options.join("");
    select.value = value ?? "";
  }

  /** Show a strategy in the header and the rule editor. */
  function fillEditor(strategy) {
    const nameInput = $("#strategy-name");
    if (nameInput) nameInput.value = strategy.name;
    updateTypeBadge(strategy.type);
    const enableToggle = $("#strategy-enabled-toggle");
    if (enableToggle) enableToggle.checked = strategy.enabled;
    populateTimeframe(strategy.timeframe);
    const priority = $("#strategy-priority");
    if (priority) priority.value = strategy.priority ?? DEFAULT_PRIORITY;
    setPriorityInvalid(false);
    const description = $("#strategy-description");
    if (description) description.value = strategy.description ?? "";
    setDescriptionOpen(Boolean(strategy.description));
    ruleEditor.load(strategy.rules);
  }

  /** The strategy in the editor with the name trimmed and the rules the editor holds. */
  function editedStrategy() {
    return {
      ...currentStrategy,
      name: String(currentStrategy.name ?? "").trim(),
      rules: ruleEditor.serialize(),
    };
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
      Object.assign(conditionSchemas, data.schemas || {});
      defaultTimeframe = data.default_timeframe ?? null;
    } catch (error) {
      console.error("Failed to load condition schemas:", error);
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
      listContainer.innerHTML = renderStateView({
        icon: "icon-file-text",
        title: I18n.t("strategies-list-empty-title"),
        message: I18n.t("strategies-list-empty-hint"),
        compact: true,
      });
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

  function openConditionCatalog(groupId) {
    if (!currentStrategy) {
      return announce(
        "warning",
        I18n.t("strategies-toast-create-first"),
        I18n.attr("strategies-toast-create-first", "message")
      );
    }
    return conditionCatalog.open({
      strategyType: currentStrategy.type,
      handler: (type) => ruleEditor.addConditionTo(groupId, type),
    });
  }

  /** Ask before an unsaved strategy is replaced; true when the editor may be overwritten. */
  async function confirmDiscard() {
    if (!_isDirty || !currentStrategy) return true;
    const { confirmed } = await ConfirmationDialog.show({
      title: I18n.t("strategies-unsaved-confirm"),
      message: I18n.attr("strategies-unsaved-confirm", "message", { name: currentStrategy.name }),
      confirmLabel: I18n.t("strategies-unsaved-discard"),
      cancelLabel: I18n.t("common-action-cancel"),
      variant: "warning",
    });
    return confirmed;
  }

  /** Put a strategy in the editor; the list highlights it when it is saved. */
  function openInEditor(strategy, { dirty }) {
    currentStrategy = strategy;
    fillEditor(strategy);
    showEditor();
    clearDirty();
    if (dirty) markDirty();
    $$(".strategy-item").forEach((item) => {
      item.classList.toggle("active", Boolean(strategy.id) && item.dataset.strategyId === strategy.id);
    });
  }

  /** Clear the editor back to its empty state, as when nothing is selected. */
  function closeEditor() {
    currentStrategy = null;
    clearDirty();
    hideEditor();
  }

  // Strategy Operations
  function createNewStrategy(strategyType = "ENTRY") {
    openInEditor(
      {
        id: null,
        name: I18n.t("strategies-new-name"),
        description: null,
        type: strategyType,
        enabled: true,
        priority: DEFAULT_PRIORITY,
        timeframe: defaultTimeframe,
        rules: null,
        parameters: {},
      },
      { dirty: true }
    );
  }

  async function startNewStrategy(strategyType) {
    if (await confirmDiscard()) createNewStrategy(strategyType);
  }

  function updateTypeBadge(type) {
    const badge = $("#strategy-type-badge");
    if (!badge) return;

    badge.className = `strategy-type-badge ${type.toLowerCase()}`;
    const icon = type === "ENTRY" ? "icon-trending-up" : "icon-trending-down";
    badge.innerHTML = `<i class="${icon}"></i> ${Utils.escapeHtml(I18n.label(STRATEGY_TYPE_LABELS, type))}`;
  }

  async function loadStrategy(strategyId) {
    if (currentStrategy?.id === strategyId) return;
    if (!(await confirmDiscard())) return;
    try {
      const data = await requestManager.fetch(`/api/strategies/${strategyId}`, {
        priority: "normal",
      });
      openInEditor(strategyFromDetail(data), { dirty: false });
    } catch (error) {
      console.error("Failed to load strategy:", error);
      Utils.showToast(I18n.t("strategies-toast-load-strategy-failed"), "error");
    }
  }

  /**
   * True when the strategy is fit to send: it has a name, a priority in range and a rule
   * tree without problems. Otherwise the first thing to fix is brought into view.
   */
  function checkBeforeSend() {
    const nameInput = $("#strategy-name");
    if (!currentStrategy.name.trim()) {
      announce(
        "warning",
        I18n.t("strategies-toast-name-required"),
        I18n.attr("strategies-toast-name-required", "message")
      );
      if (nameInput) nameInput.focus();
      return false;
    }
    if (!isValidPriority(currentStrategy.priority)) {
      setPriorityInvalid(true);
      announce(
        "warning",
        I18n.t("strategies-toast-priority-invalid"),
        I18n.attr("strategies-toast-priority-invalid", "message")
      );
      $("#strategy-priority")?.focus();
      return false;
    }
    if (!ruleEditor.checkBeforeSave()) {
      Utils.showToast(I18n.t("strategies-toast-fix-validation"), "error");
      return false;
    }
    return true;
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
    if (!checkBeforeSend()) return;

    try {
      const saved = editedStrategy();
      const method = saved.id ? "PUT" : "POST";
      const url = saved.id ? `/api/strategies/${saved.id}` : "/api/strategies";

      const data = await requestManager.fetch(url, {
        method,
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(requestBodyOf(saved)),
        priority: "high",
      });
      currentStrategy.name = saved.name;
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
        await errorMessageOf(error, I18n.attr("strategies-toast-save-failed", "message"))
      );
    }
  }

  async function validateStrategy() {
    if (!currentStrategy) {
      Utils.showToast(I18n.t("strategies-toast-no-strategy-validate"), "warning");
      return;
    }
    if (!checkBeforeSend()) return;

    try {
      const data = await requestManager.fetch("/api/strategies/validate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(requestBodyOf(editedStrategy())),
        priority: "high",
      });

      if (data.valid) {
        Utils.showToast(I18n.t("strategies-toast-valid"), "success");
        return;
      }
      const reasons = (data.errors || []).map((error) => I18n.text(error));
      announce("error", I18n.t("strategies-toast-invalid"), Utils.formatList(reasons, { type: "unit" }));
    } catch (error) {
      console.error("Validation failed:", error);
      announce(
        "error",
        I18n.t("strategies-toast-validation-failed"),
        await errorMessageOf(error, null)
      );
    }
  }

  /** Switch a listed strategy on or off; the rest of the stored strategy is untouched. */
  async function toggleStrategyEnabled(strategyId) {
    try {
      const strategy = strategies.find((s) => s.id === strategyId);
      if (!strategy) return;
      const enabled = !strategy.enabled;

      await requestManager.fetch(`/api/strategies/${encodeURIComponent(strategyId)}/enabled`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled }),
        priority: "high",
      });

      if (currentStrategy?.id === strategyId) {
        currentStrategy.enabled = enabled;
        const toggle = $("#strategy-enabled-toggle");
        if (toggle) toggle.checked = enabled;
      }
      await loadStrategies();
      Utils.showToast(
        enabled ? I18n.t("strategies-toast-item-enabled") : I18n.t("strategies-toast-item-disabled"),
        "success"
      );
    } catch (error) {
      console.error("Failed to toggle strategy:", error);
      announce(
        "error",
        I18n.t("strategies-toast-item-toggle-failed"),
        await errorMessageOf(error, null)
      );
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

      if (currentStrategy?.id === strategyId) closeEditor();

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
        await errorMessageOf(error, I18n.attr("strategies-toast-delete-failed", "message"))
      );
    }
  }

  /** Download the strategy in the editor as an export file. */
  function exportStrategy() {
    if (!currentStrategy) return;
    const strategy = editedStrategy();
    downloadJson(exportFileName(strategy.name), exportEnvelope(strategy));
    Utils.showToast(I18n.t("strategies-toast-exported"), "success");
  }

  /** Reason an import file was refused. */
  function importRefusal({ code, args }) {
    switch (code) {
      case "json":
        return I18n.t("strategies-import-error-json");
      case "unknown-condition":
        return I18n.t("strategies-import-error-unknown-condition", { type: args.type });
      case "not-applicable":
        return I18n.t("strategies-import-error-not-applicable", {
          name: conditionName(conditionSchemas[args.type], args.type),
        });
      default:
        return I18n.t("strategies-import-error-shape");
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
        const { strategy, error } = parseImport(await file.text(), conditionSchemas, defaultTimeframe);
        if (error) {
          announce("error", I18n.t("strategies-toast-import-failed"), importRefusal(error));
          return;
        }
        if (!(await confirmDiscard())) return;
        openInEditor(strategy, { dirty: true });
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
