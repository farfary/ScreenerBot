// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Strategies sub-tab - the strategy roster and the condition editor a roster row opens, with create, import, validate, save and delete flows over the /api/strategies endpoints.

import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { renderStateView } from "../ui/state_view.js";
import { STRATEGY_TYPE_LABELS } from "../ui/strategy_type.js";
import { gateControl } from "../ui/setup_gate.js";
import { requestManager, errorMessageOf } from "../core/request_manager.js";
import { downloadJson } from "../core/download.js";
import { enhanceAllSelects } from "../ui/custom_select.js";
import { createRuleEditor } from "./strategies/rule_editor.js";
import { createConditionCatalog } from "./strategies/condition_catalog.js";
import { createRoster } from "./strategies/roster.js";
import { setStrategyEnabled } from "./strategies/enable.js";
import { createStrategyMode } from "./strategies/strategy_mode.js";
import { conditionName, optionLabel, optionValue } from "./strategies/condition_text.js";
import {
  renderNodeVerdict,
  renderTestRunning,
  renderTestSummary,
  verdictsByNode,
} from "./strategies/test_result.js";
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

/**
 * A live test first waits up to 10 s on the server for the candles of a token the app holds
 * none for, past the request manager's 10 s default, then evaluates the strategy.
 */
const TEST_TIMEOUT_MS = 30000;

export function createLifecycle() {
  // State
  let currentStrategy = null;
  const conditionSchemas = {}; // filled in place once the schemas are loaded
  let defaultTimeframe = null; // timeframe a new strategy is saved with

  // Dirty tracking — unsaved editor changes
  let _isDirty = false;

  // Incremented per strategy test; a response for an older test is dropped.
  let testSeq = 0;

  // Event listener cleanup tracking
  const eventCleanups = [];

  // Helper to track event listeners for cleanup
  function addTrackedListener(element, event, handler) {
    if (!element) {
      return;
    }
    element.addEventListener(event, handler);
    eventCleanups.push(() => element.removeEventListener(event, handler));
  }

  /** Show the editor in place of the roster. */
  function showEditor() {
    const roster = $("#strategies-roster");
    const editor = $("#strategy-editor");
    if (roster) roster.hidden = true;
    if (editor) editor.hidden = false;
  }

  /** Show the roster in place of the editor, at the view, sort and scroll it was left at. */
  function showRoster() {
    const roster = $("#strategies-roster");
    const editor = $("#strategy-editor");
    if (editor) editor.hidden = true;
    if (roster) roster.hidden = false;
  }

  function markDirty() {
    if (_isDirty) return;
    _isDirty = true;
    // A test result describes the saved rules, which an edit leaves behind.
    hideTestResult();
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
    renderVerdict: renderNodeVerdict,
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

  const strategyMode = createStrategyMode({ requestManager, ConfirmationDialog, Utils });
  let roster = null;

  return {
    async init(_ctx) {
      console.log("[Strategies] Initializing page");

      showRoster();
      roster = createRoster({
        container: $("#strategies-roster"),
        onOpen: (id) => void loadStrategy(id),
        onCreate: (type) => showCreateStrategyModal(type),
        onImport: importStrategy,
        onDelete: (row) => void deleteStrategy(row),
        strategyMode,
      });
      roster.mount();

      setupCreateStrategyModal();
      setupHeaderActions();
      setupSettingsActions();

      // Load condition schemas
      await loadConditionSchemas();

      ruleEditor.bind();
      conditionCatalog.bind();
    },

    async activate(ctx) {
      console.log("[Strategies] Activating page");

      // The roster keeps polling while the editor is open, so the editor reads the
      // strategy's current mode when it saves new rules.
      ctx
        .managePoller(
          new Poller(() => roster.load(), {
            label: "Strategies", // l10n-ignore: poller label used in logs only
            intervalMs: 10000,
          })
        )
        .start();

      await roster.load();
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

      roster?.destroy();
      roster = null;

      // Cleanup state
      currentStrategy = null;
      _isDirty = false;
    },
  };

  /** Open the new-strategy dialog; `type` ("ENTRY" or "EXIT") preselects that kind. */
  function showCreateStrategyModal(type = null) {
    const modal = $("#create-strategy-modal");
    if (!modal) return;
    modal.hidden = false;
    let preselected = null;
    $$(".type-card").forEach((card) => {
      const selected = card.dataset.type === type;
      card.setAttribute("aria-pressed", String(selected));
      if (selected) preselected = card;
    });
    preselected?.focus();
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

  /** A field handler that edits the open strategy and marks it unsaved. */
  function editField(apply) {
    return (event) => {
      if (!currentStrategy) return;
      apply(event.target);
      markDirty();
    };
  }

  /**
   * The editor's On switch. A saved strategy switches through the one enable path, as the
   * roster switch does, and returns when the server refuses; an unsaved one keeps the
   * state for its first save.
   */
  async function onEnabledChange(event) {
    const input = event.currentTarget;
    if (!currentStrategy) return;
    const enabled = input.checked;
    if (!currentStrategy.id) {
      currentStrategy.enabled = enabled;
      markDirty();
      return;
    }
    const strategy = currentStrategy;
    input.disabled = true;
    const changed = await setStrategyEnabled(strategy.id, enabled);
    input.disabled = false;
    if (!changed) {
      input.checked = !enabled;
      return;
    }
    strategy.enabled = enabled;
    void roster?.reload();
  }

  /** Editor toolbar: the way back, the name, the On switch and the strategy actions. */
  function setupHeaderActions() {
    addTrackedListener($("#strategy-back"), "click", () => void backToRoster());
    addTrackedListener(
      $("#strategy-name"),
      "input",
      editField((input) => {
        currentStrategy.name = input.value;
      })
    );
    addTrackedListener($("#strategy-enabled-toggle"), "change", onEnabledChange);

    // A narrow editor shows these actions as their glyph alone; the label stays their
    // accessible name and becomes their tooltip.
    $$("#editor-header .editor-action").forEach((button) => {
      button.title = button.textContent.trim();
    });

    addTrackedListener($("#save-strategy"), "click", saveStrategy);
    addTrackedListener($("#test-strategy"), "click", pickTestToken);
    addTrackedListener($("#strategy-test-close"), "click", hideTestResult);
    addTrackedListener($("#validate-strategy"), "click", validateStrategy);
    addTrackedListener($("#export-strategy"), "click", exportStrategy);
  }

  /** Strategy settings above the rules: timeframe, priority, description and binding. */
  function setupSettingsActions() {
    addTrackedListener(
      $("#strategy-timeframe"),
      "change",
      editField((select) => {
        currentStrategy.timeframe = select.value;
        // Conditions without a timeframe of their own show the strategy's.
        ruleEditor.render();
      })
    );
    addTrackedListener(
      $("#strategy-priority"),
      "input",
      editField((input) => {
        currentStrategy.priority = input.value === "" ? null : Number(input.value);
        setPriorityInvalid(!isValidPriority(currentStrategy.priority));
      })
    );
    addTrackedListener(
      $("#strategy-description"),
      "input",
      editField((input) => {
        currentStrategy.description = input.value;
      })
    );
    addTrackedListener($("#toggle-description"), "click", () => {
      setDescriptionOpen($("#strategy-description-panel")?.hidden ?? false);
    });
    addTrackedListener(
      $("#strategy-entry-binding"),
      "change",
      editField((input) => {
        currentStrategy.entry_strategy_ids = [
          ...input.closest("#strategy-entry-binding").querySelectorAll("input:checked"),
        ].map((box) => box.value);
        updateEntryBindingSummary();
      })
    );
    addTrackedListener($("#toggle-entry-binding"), "click", () => {
      setEntryBindingOpen($("#strategy-entry-binding-panel")?.hidden ?? false);
    });
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

  function setEntryBindingOpen(open) {
    const panel = $("#strategy-entry-binding-panel");
    const toggle = $("#toggle-entry-binding");
    if (panel) panel.hidden = !open;
    if (toggle) toggle.setAttribute("aria-expanded", String(open));
  }

  /**
   * The entry strategies an exit strategy can be bound to: every entry strategy, then a
   * bound id whose strategy no longer exists, so it can be unchecked.
   */
  function entryBindingChoices() {
    const entries = (roster?.entries() ?? []).map((strategy) => ({
      id: strategy.id,
      label: strategy.name,
    }));
    const known = new Set(entries.map((entry) => entry.id));
    const deleted = currentStrategy.entry_strategy_ids
      .filter((id) => !known.has(id))
      .map((id) => ({ id, label: I18n.t("strategies-editor-entry-binding-deleted", { id }) }));
    return [...entries, ...deleted];
  }

  /** The binding button's text after its label: every entry strategy, or the bound ones' names. */
  function updateEntryBindingSummary() {
    const summary = $("#entry-binding-summary");
    if (!summary || !currentStrategy) return;
    const bound = new Set(currentStrategy.entry_strategy_ids);
    const names = entryBindingChoices()
      .filter((choice) => bound.has(choice.id))
      .map((choice) => choice.label);
    summary.textContent =
      names.length === 0 ? I18n.t("strategies-editor-entry-binding-all") : Utils.formatList(names);
  }

  /**
   * The exit strategy's binding: a checkbox per entry strategy, none checked for every
   * position. An entry strategy manages no positions and shows no binding.
   */
  function renderEntryBinding() {
    const field = $("#entry-binding-field");
    const list = $("#strategy-entry-binding");
    if (!field || !list) return;
    const isExit = currentStrategy?.type === "EXIT";
    field.hidden = !isExit;
    if (!isExit) {
      setEntryBindingOpen(false);
      list.replaceChildren();
      return;
    }
    const bound = new Set(currentStrategy.entry_strategy_ids);
    const choices = entryBindingChoices();
    if (choices.length === 0) {
      const empty = document.createElement("p");
      empty.className = "editor-entry-binding-empty";
      empty.textContent = I18n.t("strategies-editor-entry-binding-empty");
      list.replaceChildren(empty);
    } else {
      list.replaceChildren(
        ...choices.map((choice) => {
          const label = document.createElement("label");
          label.className = "editor-entry-binding-choice";
          const box = document.createElement("input");
          box.type = "checkbox";
          box.value = choice.id;
          box.checked = bound.has(choice.id);
          const name = document.createElement("span");
          name.textContent = choice.label;
          label.append(box, name);
          return label;
        })
      );
    }
    updateEntryBindingSummary();
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

  /** Show a strategy in the toolbar, the settings line and the rule editor. */
  function fillEditor(strategy) {
    const nameInput = $("#strategy-name");
    if (nameInput) nameInput.value = strategy.name;
    updateTypeBadge(strategy.type);
    const enableToggle = $("#strategy-enabled-toggle");
    if (enableToggle) {
      enableToggle.checked = strategy.enabled;
      enableToggle.disabled = false;
      enableToggle.removeAttribute("title");
      // In Explore Mode a saved strategy cannot be switched; a new one keeps its draft state.
      if (strategy.id) gateControl(enableToggle);
    }
    populateTimeframe(strategy.timeframe);
    const priority = $("#strategy-priority");
    if (priority) priority.value = strategy.priority ?? DEFAULT_PRIORITY;
    setPriorityInvalid(false);
    const description = $("#strategy-description");
    if (description) description.value = strategy.description ?? "";
    setDescriptionOpen(false);
    setEntryBindingOpen(false);
    renderEntryBinding();
    hideTestResult();
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

  /** Put a strategy in the editor and show the editor in place of the roster. */
  function openInEditor(strategy, { dirty }) {
    currentStrategy = strategy;
    fillEditor(strategy);
    showEditor();
    clearDirty();
    if (dirty) markDirty();
  }

  /** Clear the editor and return to the roster. */
  function closeEditor() {
    hideTestResult();
    currentStrategy = null;
    clearDirty();
    showRoster();
  }

  /** The back control: unsaved changes are confirmed first, then the roster returns. */
  async function backToRoster() {
    if (!(await confirmDiscard())) return;
    closeEditor();
    void roster?.reload();
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
        entry_strategy_ids: [],
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
    if (!(await confirmDiscard())) return;
    try {
      const data = await requestManager.fetch(`/api/strategies/${encodeURIComponent(strategyId)}`, {
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

  /**
   * Before a stored strategy is saved, ask the server whether the save changes the rules
   * it trades on; the editor never compares rules itself. New rules on a live strategy
   * trade real funds from the next check and restart its paper evidence, so they are
   * confirmed first. Resolves `{ confirmedLive }` to go on, or null to save nothing.
   */
  async function checkRulesChange(saved, body) {
    let data;
    try {
      data = await requestManager.fetch(
        `/api/strategies/validate?id=${encodeURIComponent(saved.id)}`,
        { method: "POST", headers: { "Content-Type": "application/json" }, body, priority: "high" }
      );
    } catch (error) {
      console.error("Validation before save failed:", error);
      announce(
        "error",
        I18n.t("strategies-toast-save-failed"),
        await errorMessageOf(error, I18n.attr("strategies-toast-save-failed", "message"))
      );
      return null;
    }
    if (!data.valid) {
      const reasons = (data.errors || []).map((error) => I18n.text(error));
      announce("error", I18n.t("strategies-toast-invalid"), Utils.formatList(reasons, { type: "unit" }));
      return null;
    }
    const mode = roster?.row(saved.id)?.mode ?? saved.mode;
    if (!data.rules_changed || mode !== "live") return { confirmedLive: false };
    const args = { name: saved.name };
    const { confirmed } = await ConfirmationDialog.show({
      title: I18n.t("strategies-live-rules-confirm", args),
      message: I18n.attr("strategies-live-rules-confirm", "message", args),
      confirmLabel: I18n.t("strategies-live-rules-save"),
      cancelLabel: I18n.t("common-action-cancel"),
      variant: "danger",
    });
    return confirmed ? { confirmedLive: true } : null;
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

    const saved = editedStrategy();
    const body = JSON.stringify(requestBodyOf(saved));
    let confirmedLive = false;
    if (saved.id) {
      const check = await checkRulesChange(saved, body);
      if (!check) return;
      confirmedLive = check.confirmedLive;
    }

    try {
      const method = saved.id ? "PUT" : "POST";
      const url = saved.id ? `/api/strategies/${encodeURIComponent(saved.id)}` : "/api/strategies";

      const data = await requestManager.fetch(url, {
        method,
        headers: { "Content-Type": "application/json" },
        body,
        priority: "high",
      });
      currentStrategy.name = saved.name;
      if (!currentStrategy.id && data.id) {
        currentStrategy.id = data.id;
      }

      clearDirty();
      await roster?.reload();
      const args = { name: currentStrategy.name };
      // New rules restart the paper evidence; a live save the dialog confirmed said so
      // already, any other save that changed the rules states it now.
      const notice =
        data.rules_changed === true && !confirmedLive
          ? "strategies-toast-evidence-restarted"
          : "strategies-toast-saved";
      announce("success", I18n.t(notice, args), I18n.attr(notice, "message", args));
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

  /** Close the test result and take its marks off the rules; a test still running is dropped. */
  function hideTestResult() {
    testSeq += 1;
    const line = $("#strategy-test-line");
    if (line) line.hidden = true;
    const body = $("#strategy-test-result");
    if (body) body.replaceChildren();
    ruleEditor.clearVerdicts();
  }

  /**
   * Test the stored strategy against a token picked in the search dialog. The test reads
   * the saved rules, so a strategy with unsaved changes or never saved is saved first.
   */
  function pickTestToken() {
    if (!currentStrategy) {
      Utils.showToast(I18n.t("strategies-toast-no-strategy-test"), "warning");
      return;
    }
    if (_isDirty || !currentStrategy.id) {
      Utils.showToast(I18n.t("strategies-toast-test-save-first"), "warning");
      return;
    }
    // An exit strategy reads a position, so its picker opens on the held tokens.
    window.openSearchDialog?.({
      onPick: testAgainstToken,
      tab: currentStrategy.type === "EXIT" ? "positions" : null,
    });
  }

  async function testAgainstToken(token) {
    const strategyId = currentStrategy?.id;
    const line = $("#strategy-test-line");
    const body = $("#strategy-test-result");
    if (!strategyId || !line || !body) return;
    ruleEditor.clearVerdicts();
    testSeq += 1;
    const seq = testSeq;
    const label = token.symbol || token.mint;
    body.innerHTML = renderTestRunning({ token: label });
    line.hidden = false;

    try {
      const result = await requestManager.fetch(
        `/api/strategies/${encodeURIComponent(strategyId)}/test`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ token_mint: token.mint }),
          priority: "high",
          timeout: TEST_TIMEOUT_MS,
        }
      );
      if (seq !== testSeq) return;
      body.innerHTML = renderTestSummary(result.rules, { token: label });
      ruleEditor.setVerdicts(verdictsByNode(ruleEditor.model(), result.rules));
    } catch (error) {
      if (seq !== testSeq) return;
      line.hidden = true;
      announce(
        "error",
        I18n.t("strategies-toast-test-failed"),
        await errorMessageOf(error, I18n.attr("strategies-toast-test-failed", "message"))
      );
    }
  }

  /** Delete a roster row's strategy after confirmation. */
  async function deleteStrategy(strategy) {
    const strategyId = strategy.id;
    const { confirmed } = await ConfirmationDialog.show({
      title: I18n.t("strategies-delete-title"),
      message: I18n.t("strategies-delete-message", { name: strategy.name }),
      confirmLabel: I18n.t("common-action-delete"),
      cancelLabel: I18n.t("common-action-cancel"),
      variant: "danger",
    });

    if (!confirmed) return;

    try {
      await requestManager.fetch(`/api/strategies/${encodeURIComponent(strategyId)}`, {
        method: "DELETE",
        priority: "high",
      });

      await roster?.reload();
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
      case "priority":
        return I18n.t("strategies-import-error-priority", args);
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

// Strategies is embedded as a sub-tab of the Auto Trader page, which imports
// createLifecycle() and drives init/activate/deactivate/dispose itself (see trader.js).
