/**
 * TableSettingsDialog - Modal dialog for managing table column settings
 *
 * Features:
 * - Quick visibility controls (show/hide all, invert)
 * - Ordering shortcuts (reset, alphabetical)
 * - Per-column visibility toggles
 * - Reordering via move buttons (top / up / down / bottom)
 * - Apply / Cancel workflow
 * - Reset to defaults
 * - Keyboard accessible
 */

import { on, off } from "../core/dom.js";
import * as Utils from "../core/utils.js";

const ORDER_ACTIONS = ["move-top", "move-up", "move-down", "move-bottom"];

function canToggleVisibility(column) {
  if (!column) {
    return true;
  }
  if (column.lockVisibility === true) {
    return false;
  }
  if (column.disableVisibilityToggle === true) {
    return false;
  }
  if (column.hideable === false) {
    return false;
  }
  return true;
}

function normalizeColumns(columns = [], order = []) {
  const ordered = [];
  const orderSet = new Set(Array.isArray(order) ? order : []);

  if (orderSet.size > 0) {
    order.forEach((colId) => {
      const column = columns.find((col) => col.id === colId);
      if (column) {
        ordered.push(column);
      }
    });
  }

  columns.forEach((col) => {
    if (!orderSet.has(col.id)) {
      ordered.push(col);
    }
  });

  return ordered;
}

export class TableSettingsDialog {
  constructor(options) {
    this.options = {
      columns: options.columns || [],
      currentOrder: options.currentOrder || [],
      currentVisibility: options.currentVisibility || {},
      defaultOrder: options.defaultOrder || [],
      defaultVisibility: options.defaultVisibility || {},
      onApply: typeof options.onApply === "function" ? options.onApply : null,
      // Pagination toggle options
      showPaginationToggle: options.showPaginationToggle || false,
      paginationEnabled: options.paginationEnabled !== false,
      onPaginationToggle:
        typeof options.onPaginationToggle === "function" ? options.onPaginationToggle : null,
    };

    this.root = null;
    this.dialog = null;
    this.columnListEl = null;
    this.searchInput = null;
    this.applyBtn = null;
    this.cancelBtn = null;
    this.resetBtn = null;

    this._isOpen = false;
    this._previousActiveElement = null;
    this._searchTerm = "";

    this._columnListeners = [];
    this._quickActionListeners = [];
    this._paginationToggleListener = null;
    this._searchListener = this._handleSearch.bind(this);

    this._applyListener = this._handleApply.bind(this);
    this._resetListener = this._handleReset.bind(this);

    this._overlayListener = this._handleOverlayClick.bind(this);
    this._closeListener = this._handleCloseClick.bind(this);
    this._keyListener = this._handleKeyDown.bind(this);

    this._workingState = this._createWorkingState();
  }

  _createWorkingState(useDefaults = false) {
    const orderSource = useDefaults ? this.options.defaultOrder : this.options.currentOrder;
    const visibilitySource = useDefaults
      ? this.options.defaultVisibility
      : this.options.currentVisibility;

    const orderedColumns = normalizeColumns(this.options.columns, orderSource);
    const visibility = {};

    orderedColumns.forEach((column) => {
      if (!column || !column.id) {
        return;
      }
      if (Object.prototype.hasOwnProperty.call(visibilitySource, column.id)) {
        visibility[column.id] = Boolean(visibilitySource[column.id]);
      } else {
        visibility[column.id] = column.visible !== false;
      }
    });

    return {
      columns: orderedColumns,
      visibility,
    };
  }

  _ensureElements() {
    if (this.root) {
      return;
    }

    const overlay = document.createElement("div");
    overlay.className = "table-settings-overlay";
    overlay.setAttribute("role", "presentation");
    overlay.setAttribute("aria-hidden", "true");

    overlay.innerHTML = `
      <div class="table-settings-dialog" role="dialog" aria-modal="true" aria-labelledby="table-settings-title" tabindex="-1">
        <header class="table-settings-header">
          <h2 id="table-settings-title" class="table-settings-title">Table Settings</h2>
          <button type="button" class="table-settings-close" data-action="close" aria-label="Close dialog">&times;</button>
        </header>
        <div class="table-settings-body">
          ${this._renderPaginationToggle()}
          <div class="table-settings-controls">
            <section class="table-settings-controls-group" aria-label="Visibility controls">
              <h3 class="table-settings-controls-title">Visibility</h3>
              <div class="table-settings-controls-buttons">
                <button type="button" class="btn tertiary" data-quick-action="show-all">Show All</button>
                <button type="button" class="btn tertiary" data-quick-action="hide-all">Hide All</button>
                <button type="button" class="btn tertiary" data-quick-action="invert-visibility">Invert</button>
              </div>
            </section>
            <section class="table-settings-controls-group" aria-label="Ordering controls">
              <h3 class="table-settings-controls-title">Ordering</h3>
              <div class="table-settings-controls-buttons">
                <button type="button" class="btn tertiary" data-quick-action="reset-order">Reset Order</button>
                <button type="button" class="btn tertiary" data-quick-action="alphabetical">Sort A→Z</button>
              </div>
            </section>
          </div>
          <div class="table-settings-search-container">
            <input type="text" class="table-settings-search" placeholder="Filter columns..." aria-label="Filter columns">
          </div>
          <div class="table-settings-column-list" role="list"></div>
        </div>
        <footer class="table-settings-footer">
          <button type="button" class="btn secondary" data-action="reset">Reset to Defaults</button>
          <div class="table-settings-footer-actions">
            <button type="button" class="btn" data-action="cancel">Cancel</button>
            <button type="button" class="btn primary" data-action="apply">Apply</button>
          </div>
        </footer>
      </div>
    `;

    document.body.appendChild(overlay);

    this.root = overlay;
    this.dialog = overlay.querySelector(".table-settings-dialog");
    this.columnListEl = overlay.querySelector(".table-settings-column-list");
    this.searchInput = overlay.querySelector(".table-settings-search");
    this.applyBtn = overlay.querySelector('[data-action="apply"]');
    this.cancelBtn = overlay.querySelector('[data-action="cancel"]');
    this.resetBtn = overlay.querySelector('[data-action="reset"]');

    const closeBtn = overlay.querySelector('[data-action="close"]');
    const quickActionButtons = Array.from(overlay.querySelectorAll("[data-quick-action]"));

    on(overlay, "click", this._overlayListener);
    if (closeBtn) {
      on(closeBtn, "click", this._closeListener);
    }
    if (this.searchInput) {
      on(this.searchInput, "input", this._searchListener);
    }
    if (this.cancelBtn) {
      on(this.cancelBtn, "click", this._closeListener);
    }
    if (this.applyBtn) {
      on(this.applyBtn, "click", this._applyListener);
    }
    if (this.resetBtn) {
      on(this.resetBtn, "click", this._resetListener);
    }

    quickActionButtons.forEach((button) => {
      const action = button.dataset.quickAction;
      if (!action) {
        return;
      }
      const handler = () => this._handleQuickAction(action);
      on(button, "click", handler);
      this._quickActionListeners.push({ element: button, handler });
    });

    // Setup pagination toggle listener
    this._attachPaginationToggleListener();
  }

  open() {
    // Guard against multiple simultaneous opens
    if (this._isOpen) {
      console.warn("[TableSettingsDialog] Dialog already open, ignoring duplicate request");
      return;
    }

    this._ensureElements();

    this._previousActiveElement =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;

    this._workingState = this._createWorkingState();
    this._render();

    this.root.classList.add("is-visible");
    this.root.setAttribute("aria-hidden", "false");
    document.body.classList.add("table-settings-open");
    this._isOpen = true;

    document.addEventListener("keydown", this._keyListener, true);

    requestAnimationFrame(() => {
      if (!this._isOpen) {
        return;
      }
      this.dialog?.focus();
    });
  }

  close() {
    if (!this._isOpen) {
      return;
    }

    this.root.classList.remove("is-visible");
    this.root.setAttribute("aria-hidden", "true");
    document.body.classList.remove("table-settings-open");
    this._isOpen = false;

    document.removeEventListener("keydown", this._keyListener, true);

    if (this._previousActiveElement) {
      this._previousActiveElement.focus();
      this._previousActiveElement = null;
    }
  }

  destroy() {
    this.close();

    if (!this.root) {
      return;
    }

    this._cleanupColumnListeners();
    this._cleanupQuickActionListeners();
    this._cleanupPaginationToggleListener();

    off(this.root, "click", this._overlayListener);

    if (this.searchInput) {
      off(this.searchInput, "input", this._searchListener);
    }

    const closeBtn = this.root.querySelector('[data-action="close"]');
    if (closeBtn) {
      off(closeBtn, "click", this._closeListener);
    }
    if (this.cancelBtn) {
      off(this.cancelBtn, "click", this._closeListener);
    }
    if (this.applyBtn) {
      off(this.applyBtn, "click", this._applyListener);
    }
    if (this.resetBtn) {
      off(this.resetBtn, "click", this._resetListener);
    }

    this.root.remove();
    this.root = null;
    this.dialog = null;
    this.columnListEl = null;
    this.applyBtn = null;
    this.cancelBtn = null;
    this.resetBtn = null;
  }

  _cleanupQuickActionListeners() {
    this._quickActionListeners.forEach(({ element, handler }) => {
      off(element, "click", handler);
    });
    this._quickActionListeners = [];
  }

  _cleanupColumnListeners() {
    this._columnListeners.forEach(({ element, event, handler }) => {
      off(element, event, handler);
    });
    this._columnListeners = [];
  }

  _handleSearch(event) {
    this._searchTerm = (event.target.value || "").toLowerCase().trim();
    this._render();
  }

  _render() {
    if (!this.columnListEl) {
      return;
    }

    this._cleanupColumnListeners();

    const columns = this._workingState.columns;
    const visibility = this._workingState.visibility;

    if (!columns || columns.length === 0) {
      this.columnListEl.innerHTML = '<div class="table-settings-empty">No columns available.</div>';
      return;
    }

    const filteredColumns = columns
      .map((col, idx) => ({ ...col, originalIndex: idx }))
      .filter((col) => {
        if (!this._searchTerm) return true;
        return (col.label || "").toLowerCase().includes(this._searchTerm);
      });

    if (filteredColumns.length === 0) {
      this.columnListEl.innerHTML =
        '<div class="table-settings-empty">No columns match your search.</div>';
      return;
    }

    this.columnListEl.innerHTML = filteredColumns
      .map((column) => {
        const index = column.originalIndex;
        const isVisible = visibility[column.id] !== false;
        const canToggle = canToggleVisibility(column);
        const disableToggleAttr = canToggle ? "" : " disabled";
        const visibilityLabel = canToggle ? "" : " (locked)";

        const moveTopDisabled = index === 0;
        const moveUpDisabled = index === 0;
        const moveDownDisabled = index === columns.length - 1;
        const moveBottomDisabled = index === columns.length - 1;

        // Disable move buttons if filtered, as index logic gets messy
        const isFiltered = !!this._searchTerm;
        const moveAttr = isFiltered ? "disabled" : "";

        return `
          <div class="table-settings-column-item" draggable="${!isFiltered}" data-column-id="${column.id}" role="listitem">
            <div class="table-settings-drag-handle" aria-hidden="true" title="Drag to reorder">⋮⋮</div>
            <span class="table-settings-column-index" aria-hidden="true">${index + 1}</span>
            <label class="table-settings-column-label${canToggle ? "" : " is-locked"}">
              <input type="checkbox" data-role="visibility-toggle" data-column-id="${column.id}" ${isVisible ? "checked" : ""}${disableToggleAttr} />
              <span class="column-name">${Utils.escapeHtml(column.label)}${visibilityLabel}</span>
            </label>
            <div class="table-settings-column-actions" aria-label="Ordering controls">
              <div class="table-settings-move-group" role="group" aria-label="Move up or down">
                <button type="button" class="table-settings-btn-move" data-action="move-up" data-column-id="${column.id}" ${moveUpDisabled || isFiltered ? "disabled" : ""} title="Move up">
                  <span class="icon" aria-hidden="true">↑</span>
                  <span class="sr-only">Move up</span>
                </button>
                <button type="button" class="table-settings-btn-move" data-action="move-down" data-column-id="${column.id}" ${moveDownDisabled || isFiltered ? "disabled" : ""} title="Move down">
                  <span class="icon" aria-hidden="true">↓</span>
                  <span class="sr-only">Move down</span>
                </button>
              </div>
              <div class="table-settings-move-group" role="group" aria-label="Move to extremes">
                <button type="button" class="table-settings-btn-move" data-action="move-top" data-column-id="${column.id}" ${moveTopDisabled || isFiltered ? "disabled" : ""} title="Move to top">
                  <span class="icon" aria-hidden="true">⇡</span>
                  <span class="sr-only">Move to top</span>
                </button>
                <button type="button" class="table-settings-btn-move" data-action="move-bottom" data-column-id="${column.id}" ${moveBottomDisabled || isFiltered ? "disabled" : ""} title="Move to bottom">
                  <span class="icon" aria-hidden="true">⇣</span>
                  <span class="sr-only">Move to bottom</span>
                </button>
              </div>
            </div>
          </div>
        `;
      })
      .join("");

    this._attachColumnListeners();
  }

  _handleDrop(sourceId, targetId) {
    const columns = this._workingState.columns;
    const sourceIndex = columns.findIndex((col) => col.id === sourceId);
    const targetIndex = columns.findIndex((col) => col.id === targetId);

    if (sourceIndex === -1 || targetIndex === -1 || sourceIndex === targetIndex) {
      return;
    }

    const [column] = columns.splice(sourceIndex, 1);
    columns.splice(targetIndex, 0, column);

    this._render();
  }

  _attachColumnListeners() {
    if (!this.columnListEl) {
      return;
    }

    const checkboxes = this.columnListEl.querySelectorAll('input[data-role="visibility-toggle"]');
    checkboxes.forEach((checkbox) => {
      const handler = (event) => {
        const columnId = event.target.dataset.columnId;
        if (!columnId) {
          return;
        }
        this._workingState.visibility[columnId] = event.target.checked;
      };
      on(checkbox, "change", handler);
      this._columnListeners.push({ element: checkbox, event: "change", handler });
    });

    const buttons = this.columnListEl.querySelectorAll(".table-settings-btn-move");
    buttons.forEach((button) => {
      const action = button.dataset.action;
      const columnId = button.dataset.columnId;
      if (!action || !columnId || !ORDER_ACTIONS.includes(action)) {
        return;
      }
      const handler = () => {
        this._reorderColumn(columnId, action);
      };
      on(button, "click", handler);
      this._columnListeners.push({ element: button, event: "click", handler });
    });

    const items = this.columnListEl.querySelectorAll(
      '.table-settings-column-item[draggable="true"]'
    );
    items.forEach((item) => {
      const dragStartHandler = (e) => {
        e.dataTransfer.effectAllowed = "move";
        e.dataTransfer.setData("text/plain", item.dataset.columnId);
        // Delay adding class so the drag image isn't affected
        requestAnimationFrame(() => item.classList.add("is-dragging"));
      };

      const dragOverHandler = (e) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
        item.classList.add("is-drag-over");
      };

      const dragLeaveHandler = (e) => {
        // Only remove if leaving the element entirely (not entering a child)
        if (item.contains(e.relatedTarget)) {
          return;
        }
        item.classList.remove("is-drag-over");
      };

      const dropHandler = (e) => {
        e.preventDefault();
        item.classList.remove("is-drag-over");
        const sourceId = e.dataTransfer.getData("text/plain");
        const targetId = item.dataset.columnId;

        if (sourceId && targetId && sourceId !== targetId) {
          this._handleDrop(sourceId, targetId);
        }
      };

      const dragEndHandler = (e) => {
        item.classList.remove("is-dragging");
        item.classList.remove("is-drag-over");
        // Clean up any other potential drag-over classes
        if (this.columnListEl) {
          this.columnListEl
            .querySelectorAll(".is-drag-over")
            .forEach((el) => el.classList.remove("is-drag-over"));
        }
      };

      on(item, "dragstart", dragStartHandler);
      on(item, "dragover", dragOverHandler);
      on(item, "dragleave", dragLeaveHandler);
      on(item, "drop", dropHandler);
      on(item, "dragend", dragEndHandler);

      this._columnListeners.push(
        { element: item, event: "dragstart", handler: dragStartHandler },
        { element: item, event: "dragover", handler: dragOverHandler },
        { element: item, event: "dragleave", handler: dragLeaveHandler },
        { element: item, event: "drop", handler: dropHandler },
        { element: item, event: "dragend", handler: dragEndHandler }
      );
    });
  }

  _reorderColumn(columnId, action) {
    const columns = this._workingState.columns;
    const currentIndex = columns.findIndex((col) => col.id === columnId);
    if (currentIndex === -1) {
      return;
    }

    let targetIndex = currentIndex;
    switch (action) {
      case "move-top":
        targetIndex = 0;
        break;
      case "move-up":
        targetIndex = Math.max(0, currentIndex - 1);
        break;
      case "move-down":
        targetIndex = Math.min(columns.length - 1, currentIndex + 1);
        break;
      case "move-bottom":
        targetIndex = columns.length - 1;
        break;
      default:
        return;
    }

    if (targetIndex === currentIndex) {
      return;
    }

    const [column] = columns.splice(currentIndex, 1);
    columns.splice(targetIndex, 0, column);

    this._render();
  }

  _handleQuickAction(action) {
    switch (action) {
      case "show-all":
        this._applyVisibilityToAll(true);
        break;
      case "hide-all":
        this._applyVisibilityToAll(false);
        break;
      case "invert-visibility":
        this._invertVisibility();
        break;
      case "reset-order":
        this._resetOrdering();
        break;
      case "alphabetical":
        this._sortAlphabetically();
        break;
      default:
        break;
    }
  }

  _applyVisibilityToAll(visible) {
    let didChange = false;
    this._workingState.columns.forEach((column) => {
      if (!canToggleVisibility(column)) {
        return;
      }
      if (this._workingState.visibility[column.id] !== visible) {
        this._workingState.visibility[column.id] = visible;
        didChange = true;
      }
    });

    if (didChange) {
      this._render();
    }
  }

  _invertVisibility() {
    let didChange = false;
    this._workingState.columns.forEach((column) => {
      if (!canToggleVisibility(column)) {
        return;
      }
      const current = this._workingState.visibility[column.id] !== false;
      this._workingState.visibility[column.id] = !current;
      didChange = true;
    });

    if (didChange) {
      this._render();
    }
  }

  _resetOrdering() {
    this._workingState.columns = normalizeColumns(
      this.options.columns,
      this.options.columns.map((col) => col.id)
    );
    this._render();
  }

  _sortAlphabetically() {
    this._workingState.columns = [...this._workingState.columns].sort((a, b) => {
      const aLabel = (a.label || "").toString().toLowerCase();
      const bLabel = (b.label || "").toString().toLowerCase();
      return aLabel.localeCompare(bLabel);
    });
    this._render();
  }

  _handleApply(event) {
    if (event) {
      event.preventDefault();
      event.stopPropagation();
    }
    if (!this.options.onApply) {
      this.close();
      return;
    }

    const settings = {
      columnOrder: this._workingState.columns.map((column) => column.id),
      visibleColumns: { ...this._workingState.visibility },
    };

    this.options.onApply(settings);
    this.close();
  }

  _handleReset(event) {
    if (event) {
      event.preventDefault();
      event.stopPropagation();
    }
    this._workingState = this._createWorkingState(true);
    this._render();
  }

  _handleOverlayClick(event) {
    if (event.target === this.root) {
      this.close();
    }
  }

  _handleCloseClick() {
    this.close();
  }

  _handleKeyDown(event) {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      this.close();
    }
  }

  /**
   * Render the pagination toggle section HTML
   * @returns {string} - HTML for pagination toggle or empty string if disabled
   */
  _renderPaginationToggle() {
    if (!this.options.showPaginationToggle) {
      return "";
    }

    const checked = this.options.paginationEnabled ? "checked" : "";

    return `
      <div class="table-settings-pagination-toggle" aria-label="Pagination settings">
        <label class="table-settings-pagination-label">
          <input type="checkbox" 
                 data-role="pagination-toggle" 
                 ${checked} />
          <span class="table-settings-pagination-text">Enable Pagination</span>
        </label>
        <span class="table-settings-pagination-hint">Show data in pages instead of all at once</span>
      </div>
    `;
  }

  /**
   * Attach listener to pagination toggle checkbox
   */
  _attachPaginationToggleListener() {
    if (!this.options.showPaginationToggle || !this.root) {
      return;
    }

    const checkbox = this.root.querySelector('[data-role="pagination-toggle"]');
    if (!checkbox) {
      return;
    }

    this._paginationToggleListener = (event) => {
      const enabled = event.target.checked;
      this.options.paginationEnabled = enabled;
      if (this.options.onPaginationToggle) {
        this.options.onPaginationToggle(enabled);
      }
    };

    on(checkbox, "change", this._paginationToggleListener);
  }

  /**
   * Cleanup pagination toggle listener
   */
  _cleanupPaginationToggleListener() {
    if (!this._paginationToggleListener || !this.root) {
      return;
    }

    const checkbox = this.root.querySelector('[data-role="pagination-toggle"]');
    if (checkbox) {
      off(checkbox, "change", this._paginationToggleListener);
    }
    this._paginationToggleListener = null;
  }

  /**
   * Update pagination toggle state (for external updates)
   * @param {boolean} enabled - Whether pagination is enabled
   */
  updatePaginationState(enabled) {
    this.options.paginationEnabled = enabled;
    if (this.root) {
      const checkbox = this.root.querySelector('[data-role="pagination-toggle"]');
      if (checkbox) {
        checkbox.checked = enabled;
      }
    }
  }
}
