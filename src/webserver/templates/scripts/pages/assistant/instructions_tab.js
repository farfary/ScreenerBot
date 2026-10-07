// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Instructions tab of the assistant page that loads, creates, edits, toggles, duplicates and deletes instructions through the /api/llm-analysis/instructions routes, reorders them by drag-and-drop, and previews or customizes the built-in templates.

import { $, $$ } from "../../core/dom.js";
import { closeMenu, openMenu, trackAnchoredMenu } from "../../core/menu_manager.js";
import * as Utils from "../../core/utils.js";
import { formatNumber } from "../../core/format.js";
import { ConfirmationDialog } from "../../ui/confirmation_dialog.js";

// Message key of each instruction category; the static filter in assistant.html
// uses the same messages.
const INSTRUCTION_CATEGORY_LABELS = Object.freeze({
  filtering: "assistant-instructions-category-filtering",
  trading: "assistant-instructions-category-trading",
  analysis: "assistant-instructions-category-analysis",
  general: "assistant-instructions-category-general",
});

// Message key of the one-line guidance shown under the category picker.
const INSTRUCTION_CATEGORY_HINT_LABELS = Object.freeze({
  filtering: "assistant-instructions-hint-filtering",
  trading: "assistant-instructions-hint-trading",
  analysis: "assistant-instructions-hint-analysis",
  general: "assistant-instructions-hint-general",
});

const INSTRUCTION_CATEGORY_ICONS = Object.freeze({
  filtering: "funnel",
  trading: "trending-up",
  analysis: "chart-bar",
  general: "info",
});

// Message key of each built-in template tag; ids are the `tags` of
// `get_builtin_templates` in src/llm_analysis/database.rs.
const INSTRUCTION_TAG_LABELS = Object.freeze({
  activity: "assistant-template-tag-activity",
  age: "assistant-template-tag-age",
  authority: "assistant-template-tag-authority",
  caution: "assistant-template-tag-caution",
  distribution: "assistant-template-tag-distribution",
  holders: "assistant-template-tag-holders",
  honeypot: "assistant-template-tag-honeypot",
  "large-holders": "assistant-template-tag-large-holders",
  liquidity: "assistant-template-tag-liquidity",
  momentum: "assistant-template-tag-momentum",
  "new-tokens": "assistant-template-tag-new-tokens",
  "price-action": "assistant-template-tag-price-action",
  "risk-management": "assistant-template-tag-risk-management",
  "rug-risk": "assistant-template-tag-rug-risk",
  safety: "assistant-template-tag-safety",
  security: "assistant-template-tag-security",
  volume: "assistant-template-tag-volume",
  whales: "assistant-template-tag-whales",
});

const esc = Utils.escapeHtml;

export function createInstructionsTab({ state, _eventCleanups }) {
  let activeInstructionMenu = null;
  // Instructions Tab
  // ============================================================================

  /**
   * Load instructions list
   */
  async function loadInstructions() {
    try {
      const response = await fetch("/api/llm-analysis/instructions");
      if (!response.ok) throw new Error("Failed to load instructions");
      const data = await response.json();
      state.instructions = data.instructions || [];
      renderInstructionsList(state.instructions);
    } catch (error) {
      console.error("[Assistant] Error loading instructions:", error);
      const container = $("#instructions-list");
      if (container) {
        container.innerHTML = '<div class="empty-state" data-l10n-id="assistant-instructions-load-failed"></div>';
        I18n.localizeTree(container);
      }
    }
  }

  /**
   * Load templates
   */
  async function loadTemplates() {
    try {
      const response = await fetch("/api/llm-analysis/templates");
      if (!response.ok) throw new Error("Failed to load templates");
      const data = await response.json();
      state.templates = data.templates || [];
      renderTemplatesList(data.templates || []);
    } catch (error) {
      console.error("[Assistant] Error loading templates:", error);
    }
  }

  /**
   * Render instructions list
   */
  function renderInstructionsList(instructions) {
    activeInstructionMenu?.close("superseded");
    const container = $("#instructions-list");
    if (!container) return;

    if (!instructions || instructions.length === 0) {
      container.innerHTML = `
      <div class="empty-state" id="no-instructions">
        <span class="empty-icon">📝</span>
        <p class="empty-text" data-l10n-id="assistant-instructions-empty"></p>
        <button class="btn btn-secondary" data-l10n-id="assistant-instructions-empty-add" onclick="window.assistantPage.createInstruction()"></button>
      </div>
    `;
      I18n.localizeTree(container);
      return;
    }

    container.innerHTML = instructions
      .map(
        (inst, index) => `
    <div class="instruction-item"
         data-id="${inst.id}"
         data-category="${inst.category || "general"}"
         data-active="${inst.enabled}"
         draggable="true">
      <span class="instruction-drag-handle">≡</span>
      <div class="instruction-info">
        <div class="instruction-name">${Utils.escapeHtml(inst.name)}</div>
        <div class="instruction-meta">
          <span class="category-tag ${inst.category || "general"}">${esc(I18n.label(INSTRUCTION_CATEGORY_LABELS, inst.category || "general"))}</span>
          <span class="priority-text">${esc(I18n.t("assistant-instructions-priority", { position: String(index + 1) }))}</span>
        </div>
      </div>
      <div class="instruction-actions">
        <label class="toggle toggle-sm instruction-toggle">
          <input type="checkbox" ${inst.enabled ? "checked" : ""}
                 onchange="window.assistantPage.toggleInstruction('${inst.id}', this.checked)">
          <span class="toggle-track"></span>
        </label>
        <button class="instruction-menu-btn" type="button" data-l10n-id="assistant-instructions-actions" aria-haspopup="menu" aria-expanded="false" onclick="window.assistantPage.showInstructionMenu(event, '${inst.id}')"><span aria-hidden="true">⋮</span></button>
      </div>
    </div>
  `
      )
      .join("");
    I18n.localizeTree(container);

    // Setup drag and drop
    setupDragAndDrop();

    // Setup filters
    setupInstructionFilters();
  }

  /**
   * Setup instruction filters
   */
  function setupInstructionFilters() {
    const searchInput = $("#instructions-search");
    const categoryFilter = $("#instructions-category-filter");
    const statusFilter = $("#instructions-status-filter");

    // Remove old listeners by replacing elements (simple approach)
    if (searchInput && !searchInput.dataset.filtered) {
      searchInput.dataset.filtered = "true";
      searchInput.addEventListener("input", Utils.debounce(filterInstructions, 300));
    }
    if (categoryFilter && !categoryFilter.dataset.filtered) {
      categoryFilter.dataset.filtered = "true";
      categoryFilter.addEventListener("change", filterInstructions);
    }
    if (statusFilter && !statusFilter.dataset.filtered) {
      statusFilter.dataset.filtered = "true";
      statusFilter.addEventListener("change", filterInstructions);
    }
  }

  /**
   * Filter instructions based on search and filters
   */
  function filterInstructions() {
    const search = ($("#instructions-search")?.value || "").toLowerCase();
    const category = $("#instructions-category-filter")?.value || "all";
    const status = $("#instructions-status-filter")?.value || "all";

    $$(".instruction-item").forEach((item) => {
      const name = (item.querySelector(".instruction-name")?.textContent || "").toLowerCase();
      const itemCategory = item.dataset.category || "";
      const isActive = item.dataset.active === "true";

      let visible = true;

      if (search && !name.includes(search)) visible = false;
      if (category !== "all" && itemCategory !== category) visible = false;
      if (status === "active" && !isActive) visible = false;
      if (status === "inactive" && isActive) visible = false;

      item.style.display = visible ? "" : "none";
    });
  }

  /**
   * Show instruction menu (edit, duplicate, delete)
   */
  function showInstructionMenu(event, id) {
    event.preventDefault();
    event.stopPropagation();

    const trigger = event.currentTarget;
    if (activeInstructionMenu?.trigger === trigger) {
      activeInstructionMenu.close();
      return;
    }
    activeInstructionMenu?.close("superseded");

    const menu = document.createElement("div");
    menu.className = "instruction-context-menu";
    menu.setAttribute("role", "menu");
    menu.innerHTML = `
    <button class="context-menu-item" type="button" role="menuitem" data-action="edit">
      <i class="icon-square-pen"></i> <span data-l10n-id="common-action-edit"></span>
    </button>
    <button class="context-menu-item" type="button" role="menuitem" data-action="duplicate">
      <i class="icon-copy"></i> <span data-l10n-id="common-action-duplicate"></span>
    </button>
    <button class="context-menu-item danger" type="button" role="menuitem" data-action="delete">
      <i class="icon-trash"></i> <span data-l10n-id="common-action-delete"></span>
    </button>
  `;
    I18n.localizeTree(menu);
    document.body.appendChild(menu);

    let stopPositionTracking = null;
    let closeTimer = null;
    const handle = {
      trigger,
      owns: (target) => menu.contains(target) || trigger.contains(target),
      close: (reason) => {
        if (activeInstructionMenu === handle) activeInstructionMenu = null;
        stopPositionTracking?.();
        stopPositionTracking = null;
        trigger.setAttribute("aria-expanded", "false");
        menu.removeEventListener("click", onClick);
        menu.removeEventListener("keydown", onKeyDown);
        closeMenu(handle);
        menu.classList.remove("open");
        if (reason === "escape") trigger.focus({ preventScroll: true });

        const finish = () => {
          if (closeTimer !== null) clearTimeout(closeTimer);
          closeTimer = null;
          if (activeInstructionMenu?.trigger !== trigger) {
            trigger.classList.remove("active", "menu-above");
          }
          menu.remove();
        };
        if (
          [
            "superseded",
            "outside-pointer",
            "focus-left",
            "document-hidden",
            "navigation",
            "dialog-open",
          ].includes(reason)
        ) {
          finish();
        } else {
          closeTimer = setTimeout(finish, 220);
        }
      },
    };

    const onClick = (clickEvent) => {
      const action = clickEvent.target.closest("[data-action]")?.dataset.action;
      if (!action) return;
      handle.close();
      if (action === "edit") editInstruction(id);
      else if (action === "duplicate") duplicateInstruction(id);
      else if (action === "delete") deleteInstruction(id);
    };
    const onKeyDown = (keyEvent) => {
      const items = Array.from(menu.querySelectorAll("[role='menuitem']"));
      const index = items.indexOf(document.activeElement);
      if (keyEvent.key === "ArrowDown" || keyEvent.key === "ArrowUp") {
        keyEvent.preventDefault();
        const direction = keyEvent.key === "ArrowDown" ? 1 : -1;
        const nextIndex =
          index < 0
            ? direction > 0
              ? 0
              : items.length - 1
            : (index + direction + items.length) % items.length;
        items[nextIndex]?.focus();
      } else if (keyEvent.key === "Home" || keyEvent.key === "End") {
        keyEvent.preventDefault();
        items[keyEvent.key === "Home" ? 0 : items.length - 1]?.focus();
      }
    };

    activeInstructionMenu = handle;
    openMenu(handle);
    trigger.classList.add("active");
    trigger.setAttribute("aria-haspopup", "menu");
    trigger.setAttribute("aria-expanded", "true");
    menu.addEventListener("click", onClick);
    menu.addEventListener("keydown", onKeyDown);
    stopPositionTracking = trackAnchoredMenu({
      trigger,
      menu,
      align: "end",
      onDetach: () => handle.close(),
    });
    requestAnimationFrame(() => {
      if (activeInstructionMenu === handle) {
        menu.classList.add("open");
        menu.querySelector("[role='menuitem']")?.focus({ preventScroll: true });
      }
    });
  }

  /**
   * Get category label with icon
   */
  function getCategoryLabel(category) {
    const label = esc(I18n.label(INSTRUCTION_CATEGORY_LABELS, category));
    return Object.hasOwn(INSTRUCTION_CATEGORY_ICONS, category)
      ? `<i class="icon-${INSTRUCTION_CATEGORY_ICONS[category]}"></i> ${label}`
      : label;
  }

  /**
   * Category picker options with `selected` applied to one category.
   */
  function categoryOptions(selected) {
    return Object.keys(INSTRUCTION_CATEGORY_LABELS)
      .map(
        (category) =>
          `<option value="${category}" ${category === selected ? "selected" : ""}>${esc(I18n.label(INSTRUCTION_CATEGORY_LABELS, category))}</option>`
      )
      .join("");
  }

  /**
   * Character counter readout for an instruction body.
   */
  function characterCount(length) {
    return I18n.t("assistant-instructions-char-count", {
      count: length,
      amount: formatNumber(length, 0),
    });
  }

  /**
   * Setup drag and drop for instructions
   */
  function setupDragAndDrop() {
    const items = $$(".instruction-item");

    items.forEach((item) => {
      // Drag start
      item.addEventListener("dragstart", (e) => {
        state.draggedItem = item;
        item.classList.add("dragging");
        e.dataTransfer.effectAllowed = "move";
      });

      // Drag end
      item.addEventListener("dragend", () => {
        item.classList.remove("dragging");
        state.draggedItem = null;
        // Remove all drag-over classes
        items.forEach((i) => i.classList.remove("drag-over"));
      });

      // Drag over
      item.addEventListener("dragover", (e) => {
        e.preventDefault();
        if (state.draggedItem === item) return;
        item.classList.add("drag-over");
      });

      // Drag leave
      item.addEventListener("dragleave", () => {
        item.classList.remove("drag-over");
      });

      // Drop
      item.addEventListener("drop", async (e) => {
        e.preventDefault();
        item.classList.remove("drag-over");

        if (!state.draggedItem || state.draggedItem === item) return;

        // Get all items in current order
        const container = $("#instructions-list");
        const allItems = Array.from(container.querySelectorAll(".instruction-item"));
        const draggedIndex = allItems.indexOf(state.draggedItem);
        const targetIndex = allItems.indexOf(item);

        // Reorder in DOM
        if (draggedIndex < targetIndex) {
          item.after(state.draggedItem);
        } else {
          item.before(state.draggedItem);
        }

        // Get new order
        const newOrder = Array.from(container.querySelectorAll(".instruction-item")).map((i) =>
          parseInt(i.dataset.id)
        );

        // Save new order to backend
        await reorderInstructions(newOrder);
      });
    });
  }

  /**
   * Save instruction order to backend
   */
  async function reorderInstructions(order) {
    try {
      const response = await fetch("/api/llm-analysis/instructions/reorder", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ order }),
      });

      if (!response.ok) throw new Error("Failed to reorder instructions");

      Utils.showToast({
        type: "success",
        title: I18n.t("assistant-instructions-reordered-title"),
        message: I18n.t("assistant-instructions-reordered-message"),
      });

      // Reload to get updated priorities
      await loadInstructions();
    } catch (error) {
      console.error("[Assistant] Error reordering instructions:", error);
      Utils.showToast({
        type: "error",
        title: I18n.t("assistant-instructions-reorder-failed"),
      });
      // Reload to restore original order
      await loadInstructions();
    }
  }

  /**
   * Render templates
   */
  function renderTemplatesList(templates) {
    const container = $("#templates-list");
    if (!container) return;

    if (!templates || templates.length === 0) {
      container.innerHTML = `
      <div class="empty-state">
        <p class="empty-text" data-l10n-id="assistant-templates-empty"></p>
      </div>
    `;
      I18n.localizeTree(container);
      return;
    }

    container.innerHTML = templates
      .map(
        (t) => `
    <div class="template-card" data-id="${t.id}" onclick="window.assistantPage.customizeTemplate('${t.id}')">
      <div class="template-name">${esc(I18n.text(t.name))}</div>
      <div class="template-description">${esc(I18n.text(t.description))}</div>
    </div>
  `
      )
      .join("");
  }

  /**
   * Preview template content
   */
  function previewTemplate(templateId) {
    const template = state.templates.find((t) => t.id === templateId);
    if (!template) return;

    const modal = document.createElement("div");
    modal.className = "modal-overlay";
    modal.innerHTML = `
    <div class="modal-dialog instruction-modal template-preview-modal">
      <div class="modal-header">
        <h3><i class="icon-eye"></i> <span>${esc(I18n.t("assistant-templates-preview-title", { name: I18n.text(template.name) }))}</span></h3>
        <button class="modal-close" data-l10n-id="assistant-modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
      </div>
      <div class="modal-body">
        <div class="template-preview-info">
          <div class="preview-meta">
            <span class="template-category badge-${template.category}">${getCategoryLabel(template.category)}</span>
            <div class="template-tags">${template.tags.map((tag) => `<span class="tag">${esc(I18n.label(INSTRUCTION_TAG_LABELS, tag))}</span>`).join("")}</div>
          </div>
        </div>
        <div class="template-preview-content">
          <h4 data-l10n-id="assistant-templates-preview-content"></h4>
          <pre class="template-content-display">${Utils.escapeHtml(template.content)}</pre>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" data-l10n-id="common-action-close" onclick="this.closest('.modal-overlay').remove()"></button>
        <button class="btn btn-primary" onclick="window.assistantPage.customizeTemplate('${template.id}'); this.closest('.modal-overlay').remove();">
          <i class="icon-square-pen"></i> <span data-l10n-id="assistant-templates-customize-add"></span>
        </button>
      </div>
    </div>
  `;
    I18n.localizeTree(modal);
    document.body.appendChild(modal);
  }

  /**
   * Customize template before adding
   */
  function customizeTemplate(templateId) {
    const template = state.templates.find((t) => t.id === templateId);
    if (!template) return;

    // Show modal pre-filled with template data
    const modal = document.createElement("div");
    modal.className = "modal-overlay";
    modal.innerHTML = `
    <div class="modal-dialog instruction-modal">
      <div class="modal-header">
        <h3><i class="icon-square-pen"></i> <span data-l10n-id="assistant-templates-customize-title"></span></h3>
        <button class="modal-close" data-l10n-id="assistant-modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label data-l10n-id="assistant-instructions-field-name"></label>
          <input type="text" id="inst-name" value="${esc(I18n.text(template.name))}" data-l10n-id="assistant-instructions-name-input">
        </div>
        <div class="form-group">
          <label data-l10n-id="assistant-instructions-field-category"></label>
          <select id="inst-category" data-custom-select>${categoryOptions(template.category)}</select>
          <small class="form-hint">${esc(getCategoryHint(template.category))}</small>
        </div>
        <div class="form-group">
          <label data-l10n-id="assistant-instructions-field-content"></label>
          <textarea id="inst-content" rows="12" class="instruction-editor" data-l10n-id="assistant-instructions-content-input">${esc(template.content)}</textarea>
          <div class="char-count" id="char-counter">${esc(characterCount(template.content.length))}</div>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" data-l10n-id="common-action-cancel" onclick="this.closest('.modal-overlay').remove()"></button>
        <button class="btn btn-primary" onclick="window.assistantPage.saveNewInstruction()">
          <i class="icon-plus"></i> <span data-l10n-id="assistant-instructions-create"></span>
        </button>
      </div>
    </div>
  `;
    I18n.localizeTree(modal);
    document.body.appendChild(modal);

    // Add character counter
    const textarea = $("#inst-content");
    const counter = $("#char-counter");
    if (textarea && counter) {
      textarea.addEventListener("input", () => {
        counter.textContent = characterCount(textarea.value.length);
      });
    }
  }

  /**
   * Get category hint text
   */
  function getCategoryHint(category) {
    return Object.hasOwn(INSTRUCTION_CATEGORY_HINT_LABELS, category)
      ? I18n.label(INSTRUCTION_CATEGORY_HINT_LABELS, category)
      : "";
  }

  /**
   * Create instruction (with modal)
   */
  async function createInstruction() {
    // Show modal with form
    const modal = document.createElement("div");
    modal.className = "modal-overlay";
    modal.innerHTML = `
    <div class="modal-dialog instruction-modal">
      <div class="modal-header">
        <h3><i class="icon-plus"></i> <span data-l10n-id="assistant-instructions-create-title"></span></h3>
        <button class="modal-close" data-l10n-id="assistant-modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label data-l10n-id="assistant-instructions-field-name"></label>
          <input type="text" id="inst-name" data-l10n-id="assistant-instructions-name-input">
        </div>
        <div class="form-group">
          <label data-l10n-id="assistant-instructions-field-category"></label>
          <select id="inst-category" data-custom-select>${categoryOptions("filtering")}</select>
          <small class="form-hint" id="category-hint">${esc(getCategoryHint("filtering"))}</small>
        </div>
        <div class="form-group">
          <label data-l10n-id="assistant-instructions-field-content"></label>
          <textarea id="inst-content" rows="12" class="instruction-editor" data-l10n-id="assistant-instructions-content-input"></textarea>
          <div class="char-count" id="char-counter">${esc(characterCount(0))}</div>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" data-l10n-id="common-action-cancel" onclick="this.closest('.modal-overlay').remove()"></button>
        <button class="btn btn-primary" onclick="window.assistantPage.saveNewInstruction()">
          <i class="icon-plus"></i> <span data-l10n-id="assistant-instructions-create"></span>
        </button>
      </div>
    </div>
  `;
    I18n.localizeTree(modal);
    document.body.appendChild(modal);

    // Setup category hint updater
    const categorySelect = $("#inst-category");
    const hintEl = $("#category-hint");
    if (categorySelect && hintEl) {
      categorySelect.addEventListener("change", () => {
        hintEl.textContent = getCategoryHint(categorySelect.value);
      });
    }

    // Setup character counter
    const textarea = $("#inst-content");
    const counter = $("#char-counter");
    if (textarea && counter) {
      textarea.addEventListener("input", () => {
        counter.textContent = characterCount(textarea.value.length);
      });
    }
  }

  /**
   * Save new instruction
   */
  async function saveNewInstruction() {
    const name = $("#inst-name")?.value;
    const category = $("#inst-category")?.value || "general";
    const content = $("#inst-content")?.value;

    if (!name || !content) {
      Utils.showToast({
        type: "warning",
        title: I18n.t("assistant-instructions-missing-title"),
        message: I18n.t("assistant-instructions-missing-message"),
      });
      return;
    }

    try {
      const response = await fetch("/api/llm-analysis/instructions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name, category, content }),
      });

      if (!response.ok) throw new Error("Failed to create instruction");

      document.querySelector(".modal-overlay")?.remove();
      await loadInstructions();
      Utils.showToast({
        type: "success",
        title: I18n.t("assistant-instructions-created-title"),
        message: I18n.t("assistant-instructions-created-message"),
      });
    } catch (error) {
      console.error("[Assistant] Error creating instruction:", error);
      Utils.showToast({ type: "error", title: I18n.t("assistant-instructions-create-failed") });
    }
  }

  /**
   * Toggle instruction enabled state
   */
  async function toggleInstruction(id, enabled) {
    try {
      const response = await fetch(`/api/llm-analysis/instructions/${id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled }),
      });
      if (!response.ok) {
        throw new Error("Failed to toggle instruction");
      }
    } catch (error) {
      console.error("[Assistant] Error toggling instruction:", error);
      // Revert the checkbox on failure
      const checkbox = document.querySelector(`.instruction-item[data-id="${id}"] .toggle input`);
      if (checkbox) checkbox.checked = !enabled;
      Utils.showToast({ type: "error", title: I18n.t("assistant-instructions-toggle-failed") });
    }
  }

  /**
   * Edit instruction
   */
  async function editInstruction(id) {
    try {
      // Fetch instruction data
      const response = await fetch(`/api/llm-analysis/instructions/${id}`);
      if (!response.ok) throw new Error("Failed to load instruction");
      const inst = await response.json();

      // Show modal pre-filled with data
      const modal = document.createElement("div");
      modal.className = "modal-overlay";
      modal.innerHTML = `
      <div class="modal-dialog instruction-modal">
        <div class="modal-header">
          <h3><i class="icon-square-pen"></i> <span data-l10n-id="assistant-instructions-edit-title"></span></h3>
          <button class="modal-close" data-l10n-id="assistant-modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label data-l10n-id="assistant-instructions-field-name"></label>
            <input type="text" id="edit-inst-name" value="${esc(inst.name)}" data-l10n-id="assistant-instructions-name-input">
          </div>
          <div class="form-group">
            <label data-l10n-id="assistant-instructions-field-category"></label>
            <select id="edit-inst-category" data-custom-select>${categoryOptions(inst.category)}</select>
            <small class="form-hint" id="edit-category-hint">${esc(getCategoryHint(inst.category))}</small>
          </div>
          <div class="form-group">
            <label data-l10n-id="assistant-instructions-field-content"></label>
            <textarea id="edit-inst-content" rows="12" class="instruction-editor" data-l10n-id="assistant-instructions-content-input">${esc(inst.content)}</textarea>
            <div class="char-count" id="edit-char-counter">${esc(characterCount(inst.content.length))}</div>
          </div>
          <div class="instruction-preview-section">
            <h4><i class="icon-eye"></i> <span data-l10n-id="assistant-instructions-preview"></span></h4>
            <div class="instruction-preview">
              <div class="preview-header">
                <span class="preview-name">${Utils.escapeHtml(inst.name)}</span>
                <span class="preview-category badge-${inst.category}">${getCategoryLabel(inst.category)}</span>
              </div>
              <div class="preview-content">${Utils.escapeHtml(inst.content)}</div>
            </div>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" data-l10n-id="common-action-cancel" onclick="this.closest('.modal-overlay').remove()"></button>
          <button class="btn btn-primary" onclick="window.assistantPage.saveEditedInstruction(${id})">
            <i class="icon-save"></i> <span data-l10n-id="assistant-instructions-save-changes"></span>
          </button>
        </div>
      </div>
    `;
      I18n.localizeTree(modal);
      document.body.appendChild(modal);

      // Setup live preview updater
      const nameInput = $("#edit-inst-name");
      const categorySelect = $("#edit-inst-category");
      const contentTextarea = $("#edit-inst-content");
      const previewName = modal.querySelector(".preview-name");
      const previewCategory = modal.querySelector(".preview-category");
      const previewContent = modal.querySelector(".preview-content");
      const hintEl = $("#edit-category-hint");
      const counter = $("#edit-char-counter");

      function updatePreview() {
        if (nameInput && previewName) {
          previewName.textContent = nameInput.value || I18n.t("assistant-instructions-untitled");
        }
        if (categorySelect && previewCategory) {
          const cat = categorySelect.value;
          previewCategory.className = `preview-category badge-${cat}`;
          previewCategory.innerHTML = getCategoryLabel(cat);
        }
        if (contentTextarea && previewContent) {
          previewContent.textContent = contentTextarea.value;
        }
      }

      if (nameInput) {
        nameInput.addEventListener("input", updatePreview);
      }
      if (categorySelect) {
        categorySelect.addEventListener("change", () => {
          updatePreview();
          if (hintEl) {
            hintEl.textContent = getCategoryHint(categorySelect.value);
          }
        });
      }
      if (contentTextarea) {
        contentTextarea.addEventListener("input", () => {
          updatePreview();
          if (counter) {
            counter.textContent = characterCount(contentTextarea.value.length);
          }
        });
      }
    } catch (error) {
      console.error("[Assistant] Error loading instruction for edit:", error);
      Utils.showToast({ type: "error", title: I18n.t("assistant-instructions-load-item-failed") });
    }
  }

  /**
   * Save edited instruction
   */
  async function saveEditedInstruction(id) {
    const name = $("#edit-inst-name")?.value;
    const category = $("#edit-inst-category")?.value || "general";
    const content = $("#edit-inst-content")?.value;

    if (!name || !content) {
      Utils.showToast({
        type: "warning",
        title: I18n.t("assistant-instructions-missing-title"),
        message: I18n.t("assistant-instructions-missing-message"),
      });
      return;
    }

    try {
      const response = await fetch(`/api/llm-analysis/instructions/${id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name, category, content }),
      });

      if (!response.ok) throw new Error("Failed to update instruction");

      document.querySelector(".modal-overlay")?.remove();
      await loadInstructions();
      Utils.showToast({
        type: "success",
        title: I18n.t("assistant-instructions-updated-title"),
        message: I18n.t("assistant-instructions-updated-message"),
      });
    } catch (error) {
      console.error("[Assistant] Error updating instruction:", error);
      Utils.showToast({ type: "error", title: I18n.t("assistant-instructions-update-failed") });
    }
  }

  /**
   * Delete instruction
   */
  async function deleteInstruction(id) {
    const confirmed = await ConfirmationDialog.show({
      title: I18n.t("assistant-instructions-delete-title"),
      message: I18n.t("assistant-instructions-delete-message"),
      confirmText: I18n.t("common-action-delete"),
      cancelText: I18n.t("common-action-cancel"),
      type: "danger",
    });

    if (!confirmed) return;

    try {
      await fetch(`/api/llm-analysis/instructions/${id}`, { method: "DELETE" });
      await loadInstructions();
      Utils.showToast({
        type: "success",
        title: I18n.t("assistant-instructions-deleted-title"),
        message: I18n.t("assistant-instructions-deleted-message"),
      });
    } catch (error) {
      console.error("[Assistant] Error deleting instruction:", error);
      Utils.showToast({ type: "error", title: I18n.t("assistant-instructions-delete-failed") });
    }
  }

  /**
   * Duplicate instruction
   */
  async function duplicateInstruction(id) {
    try {
      // Fetch the instruction to duplicate
      const response = await fetch(`/api/llm-analysis/instructions/${id}`);
      if (!response.ok) throw new Error("Failed to load instruction");
      const inst = await response.json();

      // Create a copy with modified name
      const copyName = I18n.t("assistant-instructions-copy-name", { name: inst.name });

      const createResponse = await fetch("/api/llm-analysis/instructions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: copyName,
          category: inst.category,
          content: inst.content,
        }),
      });

      if (!createResponse.ok) throw new Error("Failed to duplicate instruction");

      await loadInstructions();
      Utils.showToast({
        type: "success",
        title: I18n.t("assistant-instructions-duplicated-title"),
        message: I18n.t("assistant-instructions-duplicated-message"),
      });
    } catch (error) {
      console.error("[Assistant] Error duplicating instruction:", error);
      Utils.showToast({ type: "error", title: I18n.t("assistant-instructions-duplicate-failed") });
    }
  }

  /**
   * Use template to create instruction
   */
  async function useTemplate(templateId) {
    const template = state.templates.find((t) => t.id === templateId);
    if (!template) return;

    try {
      await fetch("/api/llm-analysis/instructions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: I18n.text(template.name),
          category: template.category,
          content: template.content,
        }),
      });
      await loadInstructions();
      Utils.showToast({
        type: "success",
        title: I18n.t("assistant-instructions-created-title"),
        message: I18n.t("assistant-instructions-created-from-template", {
          name: I18n.text(template.name),
        }),
      });
    } catch (error) {
      console.error("[Assistant] Error using template:", error);
      Utils.showToast({
        type: "error",
        title: I18n.t("assistant-instructions-create-from-template-failed"),
      });
    }
  }

  // ============================================================================

  // Return public API
  return {
    loadInstructions,
    loadTemplates,
    renderInstructionsList,
    renderTemplatesList,
    previewTemplate,
    customizeTemplate,
    createInstruction,
    saveNewInstruction,
    toggleInstruction,
    editInstruction,
    saveEditedInstruction,
    deleteInstruction,
    duplicateInstruction,
    useTemplate,
    showInstructionMenu,
  };
}
