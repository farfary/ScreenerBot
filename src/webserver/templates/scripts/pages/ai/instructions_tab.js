import { $, $$ } from "../../core/dom.js";
import * as Utils from "../../core/utils.js";
import { ConfirmationDialog } from "../../ui/confirmation_dialog.js";

export function createInstructionsTab({ state, _eventCleanups }) {
// Instructions Tab
// ============================================================================

/**
 * Load instructions list
 */
async function loadInstructions() {
  try {
    const response = await fetch("/api/ai/instructions");
    if (!response.ok) throw new Error("Failed to load instructions");
    const data = await response.json();
    state.instructions = data.instructions || [];
    renderInstructionsList(state.instructions);
  } catch (error) {
    console.error("[AI] Error loading instructions:", error);
    const container = $("#instructions-list");
    if (container) {
      container.innerHTML = '<div class="empty-state">Failed to load instructions</div>';
    }
  }
}

/**
 * Load templates
 */
async function loadTemplates() {
  try {
    const response = await fetch("/api/ai/templates");
    if (!response.ok) throw new Error("Failed to load templates");
    const data = await response.json();
    state.templates = data.templates || [];
    renderTemplatesList(data.templates || []);
  } catch (error) {
    console.error("[AI] Error loading templates:", error);
  }
}

/**
 * Render instructions list
 */
function renderInstructionsList(instructions) {
  const container = $("#instructions-list");
  if (!container) return;

  if (!instructions || instructions.length === 0) {
    container.innerHTML = `
      <div class="empty-state" id="no-instructions">
        <span class="empty-icon">📝</span>
        <p class="empty-text">No custom instructions yet</p>
        <button class="btn btn-secondary" onclick="window.aiPage.createInstruction()">Add Your First Instruction</button>
      </div>
    `;
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
          <span class="category-tag ${inst.category || "general"}">${inst.category || "general"}</span>
          <span class="priority-text">Priority: ${index + 1}</span>
        </div>
      </div>
      <div class="instruction-actions">
        <label class="toggle toggle-sm instruction-toggle">
          <input type="checkbox" ${inst.enabled ? "checked" : ""} 
                 onchange="window.aiPage.toggleInstruction('${inst.id}', this.checked)">
          <span class="toggle-track"></span>
        </label>
        <button class="instruction-menu-btn" onclick="window.aiPage.showInstructionMenu(event, '${inst.id}')">⋮</button>
      </div>
    </div>
  `
    )
    .join("");

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
  event.stopPropagation();

  // Create a simple context menu
  const existingMenu = $(".instruction-context-menu");
  if (existingMenu) {
    existingMenu.remove();
  }

  const menu = document.createElement("div");
  menu.className = "instruction-context-menu";
  menu.style.position = "fixed";
  menu.style.zIndex = "10000";
  menu.innerHTML = `
    <div class="context-menu-item" onclick="window.aiPage.editInstruction('${id}'); this.parentElement.remove();">
      <i class="icon-edit"></i> Edit
    </div>
    <div class="context-menu-item" onclick="window.aiPage.duplicateInstruction('${id}'); this.parentElement.remove();">
      <i class="icon-copy"></i> Duplicate
    </div>
    <div class="context-menu-item danger" onclick="window.aiPage.deleteInstruction('${id}'); this.parentElement.remove();">
      <i class="icon-trash"></i> Delete
    </div>
  `;

  // Position menu near the button
  const rect = event.target.getBoundingClientRect();
  menu.style.top = `${rect.bottom + 5}px`;
  menu.style.left = `${rect.left - 120}px`;

  document.body.appendChild(menu);

  // Close menu on outside click
  setTimeout(() => {
    const closeMenu = (e) => {
      if (!menu.contains(e.target)) {
        menu.remove();
        document.removeEventListener("click", closeMenu);
      }
    };
    document.addEventListener("click", closeMenu);
  }, 10);
}

/**
 * Get category label with icon
 */
function getCategoryLabel(category) {
  const labels = {
    filtering: '<i class="icon-filter"></i> Filtering',
    trading: '<i class="icon-trending-up"></i> Trading',
    analysis: '<i class="icon-bar-chart"></i> Analysis',
    general: '<i class="icon-info"></i> General',
  };
  return labels[category] || category;
}

/**
 * Toggle instruction expanded state
 */
function toggleInstructionExpanded(id) {
  const card = document.querySelector(`.instruction-card[data-id="${id}"]`);
  if (!card) return;

  const shortContent = card.querySelector(".instruction-content");
  const fullContent = card.querySelector(".instruction-full-content");

  if (fullContent.style.display === "none") {
    shortContent.style.display = "none";
    fullContent.style.display = "block";
    card.classList.add("instruction-expanded");
  } else {
    shortContent.style.display = "block";
    fullContent.style.display = "none";
    card.classList.remove("instruction-expanded");
  }
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
    const response = await fetch("/api/ai/instructions/reorder", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ order }),
    });

    if (!response.ok) throw new Error("Failed to reorder instructions");

    Utils.showToast({
      type: "success",
      title: "Reordered",
      message: "Instructions reordered successfully",
    });

    // Reload to get updated priorities
    await loadInstructions();
  } catch (error) {
    console.error("[AI] Error reordering instructions:", error);
    Utils.showToast({
      type: "error",
      title: "Error",
      message: "Failed to reorder instructions",
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
        <p class="empty-text">No templates available</p>
      </div>
    `;
    return;
  }

  container.innerHTML = templates
    .map(
      (t) => `
    <div class="template-card" data-id="${t.id}" onclick="window.aiPage.customizeTemplate('${t.id}')">
      <div class="template-name">${Utils.escapeHtml(t.name)}</div>
      <div class="template-description">${Utils.escapeHtml(t.description || t.content.substring(0, 100) + "...")}</div>
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
    <div class="modal instruction-modal template-preview-modal">
      <div class="modal-header">
        <h3><i class="icon-eye"></i> Template Preview: ${Utils.escapeHtml(template.name)}</h3>
        <button class="modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
      </div>
      <div class="modal-body">
        <div class="template-preview-info">
          <div class="preview-meta">
            <span class="template-category badge-${template.category}">${getCategoryLabel(template.category)}</span>
            <div class="template-tags">${template.tags.map((tag) => `<span class="tag">${tag}</span>`).join("")}</div>
          </div>
        </div>
        <div class="template-preview-content">
          <h4>Content:</h4>
          <pre class="template-content-display">${Utils.escapeHtml(template.content)}</pre>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Close</button>
        <button class="btn btn-primary" onclick="window.aiPage.customizeTemplate('${template.id}'); this.closest('.modal-overlay').remove();">
          <i class="icon-edit"></i> Customize & Add
        </button>
      </div>
    </div>
  `;
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
    <div class="modal instruction-modal">
      <div class="modal-header">
        <h3><i class="icon-edit"></i> Customize Template</h3>
        <button class="modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label>Name</label>
          <input type="text" id="inst-name" value="${Utils.escapeHtml(template.name)}" placeholder="e.g., Liquidity Guard">
        </div>
        <div class="form-group">
          <label>Category</label>
          <select id="inst-category">
            <option value="filtering" ${template.category === "filtering" ? "selected" : ""}>Filtering</option>
            <option value="trading" ${template.category === "trading" ? "selected" : ""}>Trading</option>
            <option value="analysis" ${template.category === "analysis" ? "selected" : ""}>Analysis</option>
            <option value="general" ${template.category === "general" ? "selected" : ""}>General</option>
          </select>
          <small class="form-hint">${getCategoryHint(template.category)}</small>
        </div>
        <div class="form-group">
          <label>Content</label>
          <textarea id="inst-content" rows="12" class="instruction-editor" placeholder="Enter your instruction...">${Utils.escapeHtml(template.content)}</textarea>
          <div class="char-count">
            <span id="char-counter">${template.content.length}</span> characters
          </div>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Cancel</button>
        <button class="btn btn-primary" onclick="window.aiPage.saveNewInstruction()">
          <i class="icon-plus"></i> Create
        </button>
      </div>
    </div>
  `;
  document.body.appendChild(modal);

  // Add character counter
  const textarea = $("#inst-content");
  const counter = $("#char-counter");
  if (textarea && counter) {
    textarea.addEventListener("input", () => {
      counter.textContent = textarea.value.length;
    });
  }
}

/**
 * Get category hint text
 */
function getCategoryHint(category) {
  const hints = {
    filtering:
      "Instructions for token filtering decisions - helps AI determine which tokens to skip",
    trading: "Instructions for entry/exit analysis - guides AI on trading decisions",
    analysis: "General market analysis guidelines - shapes how AI analyzes market conditions",
    general: "Other instructions - miscellaneous AI behavior customizations",
  };
  return hints[category] || "";
}

/**
 * Create instruction (with modal)
 */
async function createInstruction() {
  // Show modal with form
  const modal = document.createElement("div");
  modal.className = "modal-overlay";
  modal.innerHTML = `
    <div class="modal instruction-modal">
      <div class="modal-header">
        <h3><i class="icon-plus"></i> Create Instruction</h3>
        <button class="modal-close" onclick="this.closest('.modal-overlay').remove()"><i class="icon-x"></i></button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label>Name</label>
          <input type="text" id="inst-name" placeholder="e.g., Liquidity Guard">
        </div>
        <div class="form-group">
          <label>Category</label>
          <select id="inst-category">
            <option value="filtering">Filtering</option>
            <option value="trading">Trading</option>
            <option value="analysis">Analysis</option>
            <option value="general">General</option>
          </select>
          <small class="form-hint" id="category-hint">Instructions for token filtering decisions</small>
        </div>
        <div class="form-group">
          <label>Content</label>
          <textarea id="inst-content" rows="12" class="instruction-editor" placeholder="Enter your instruction..."></textarea>
          <div class="char-count">
            <span id="char-counter">0</span> characters
          </div>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Cancel</button>
        <button class="btn btn-primary" onclick="window.aiPage.saveNewInstruction()">
          <i class="icon-plus"></i> Create
        </button>
      </div>
    </div>
  `;
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
      counter.textContent = textarea.value.length;
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
      title: "Missing Fields",
      message: "Name and content are required",
    });
    return;
  }

  try {
    const response = await fetch("/api/ai/instructions", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, category, content }),
    });

    if (!response.ok) throw new Error("Failed to create instruction");

    document.querySelector(".modal-overlay")?.remove();
    await loadInstructions();
    Utils.showToast({
      type: "success",
      title: "Created",
      message: "Instruction created successfully",
    });
  } catch (error) {
    console.error("[AI] Error creating instruction:", error);
    Utils.showToast({
      type: "error",
      title: "Error",
      message: "Failed to create instruction",
    });
  }
}

/**
 * Toggle instruction enabled state
 */
async function toggleInstruction(id, enabled) {
  try {
    const response = await fetch(`/api/ai/instructions/${id}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ enabled }),
    });
    if (!response.ok) {
      throw new Error("Failed to toggle instruction");
    }
  } catch (error) {
    console.error("[AI] Error toggling instruction:", error);
    // Revert the checkbox on failure
    const checkbox = document.querySelector(`.instruction-item[data-id="${id}"] .toggle input`);
    if (checkbox) checkbox.checked = !enabled;
    Utils.showToast({ type: "error", title: "Error", message: "Failed to toggle instruction" });
  }
}

/**
 * Edit instruction
 */
async function editInstruction(id) {
  try {
    // Fetch instruction data
    const response = await fetch(`/api/ai/instructions/${id}`);
    if (!response.ok) throw new Error("Failed to load instruction");
    const inst = await response.json();

    // Show modal pre-filled with data
    const modal = document.createElement("div");
    modal.className = "modal-overlay";
    modal.innerHTML = `
      <div class="modal instruction-modal">
        <div class="modal-header">
          <h3><i class="icon-edit"></i> Edit Instruction</h3>
          <button class="modal-close" onclick="this.closest('.modal-overlay').remove()">×</button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label>Name</label>
            <input type="text" id="edit-inst-name" value="${Utils.escapeHtml(inst.name)}" placeholder="e.g., Liquidity Guard">
          </div>
          <div class="form-group">
            <label>Category</label>
            <select id="edit-inst-category">
              <option value="filtering" ${inst.category === "filtering" ? "selected" : ""}>Filtering</option>
              <option value="trading" ${inst.category === "trading" ? "selected" : ""}>Trading</option>
              <option value="analysis" ${inst.category === "analysis" ? "selected" : ""}>Analysis</option>
              <option value="general" ${inst.category === "general" ? "selected" : ""}>General</option>
            </select>
            <small class="form-hint" id="edit-category-hint">${getCategoryHint(inst.category)}</small>
          </div>
          <div class="form-group">
            <label>Content</label>
            <textarea id="edit-inst-content" rows="12" class="instruction-editor" placeholder="Enter your instruction...">${Utils.escapeHtml(inst.content)}</textarea>
            <div class="char-count">
              <span id="edit-char-counter">${inst.content.length}</span> characters
            </div>
          </div>
          <div class="instruction-preview-section">
            <h4><i class="icon-eye"></i> Preview</h4>
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
          <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Cancel</button>
          <button class="btn btn-primary" onclick="window.aiPage.saveEditedInstruction(${id})">
            <i class="icon-save"></i> Save Changes
          </button>
        </div>
      </div>
    `;
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
        previewName.textContent = nameInput.value || "Untitled";
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
          counter.textContent = contentTextarea.value.length;
        }
      });
    }
  } catch (error) {
    console.error("[AI] Error loading instruction for edit:", error);
    Utils.showToast({
      type: "error",
      title: "Error",
      message: "Failed to load instruction data",
    });
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
      title: "Missing Fields",
      message: "Name and content are required",
    });
    return;
  }

  try {
    const response = await fetch(`/api/ai/instructions/${id}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, category, content }),
    });

    if (!response.ok) throw new Error("Failed to update instruction");

    document.querySelector(".modal-overlay")?.remove();
    await loadInstructions();
    Utils.showToast({
      type: "success",
      title: "Updated",
      message: "Instruction updated successfully",
    });
  } catch (error) {
    console.error("[AI] Error updating instruction:", error);
    Utils.showToast({
      type: "error",
      title: "Error",
      message: "Failed to update instruction",
    });
  }
}

/**
 * Delete instruction
 */
async function deleteInstruction(id) {
  const confirmed = await ConfirmationDialog.show({
    title: "Delete Instruction",
    message: "Are you sure you want to delete this instruction?",
    confirmText: "Delete",
    cancelText: "Cancel",
    type: "danger",
  });

  if (!confirmed) return;

  try {
    await fetch(`/api/ai/instructions/${id}`, { method: "DELETE" });
    await loadInstructions();
    Utils.showToast({
      type: "success",
      title: "Deleted",
      message: "Instruction deleted successfully",
    });
  } catch (error) {
    console.error("[AI] Error deleting instruction:", error);
    Utils.showToast({
      type: "error",
      title: "Error",
      message: "Failed to delete instruction",
    });
  }
}

/**
 * Duplicate instruction
 */
async function duplicateInstruction(id) {
  try {
    // Fetch the instruction to duplicate
    const response = await fetch(`/api/ai/instructions/${id}`);
    if (!response.ok) throw new Error("Failed to load instruction");
    const inst = await response.json();

    // Create a copy with modified name
    const copyName = `${inst.name} (Copy)`;

    const createResponse = await fetch("/api/ai/instructions", {
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
      title: "Duplicated",
      message: "Instruction duplicated successfully",
    });
  } catch (error) {
    console.error("[AI] Error duplicating instruction:", error);
    Utils.showToast({
      type: "error",
      title: "Error",
      message: "Failed to duplicate instruction",
    });
  }
}

/**
 * Use template to create instruction
 */
async function useTemplate(templateId) {
  const template = state.templates.find((t) => t.id === templateId);
  if (!template) return;

  try {
    await fetch("/api/ai/instructions", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: template.name,
        category: template.category,
        content: template.content,
      }),
    });
    await loadInstructions();
    Utils.showToast({
      type: "success",
      title: "Created",
      message: `Instruction created from template: ${template.name}`,
    });
  } catch (error) {
    console.error("[AI] Error using template:", error);
    Utils.showToast({
      type: "error",
      title: "Error",
      message: "Failed to create instruction from template",
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
