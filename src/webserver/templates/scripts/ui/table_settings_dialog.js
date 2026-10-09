// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

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

function canFloatColumn(column) {
  if (!column) {
    return false;
  }
  // A column is pinnable when it explicitly opts in via `floating` capability,
  // is currently floating, or doesn't forbid it. Columns can mark
  // `floatable: false` to opt out entirely.
  if (column.floatable === false) {
    return false;
  }
  return true;
}

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
      currentFloating: Array.isArray(options.currentFloating) ? options.currentFloating : [],
      defaultFloating: Array.isArray(options.defaultFloating) ? options.defaultFloating : [],
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
    const floatingSource = useDefaults
      ? this.options.defaultFloating
      : this.options.currentFloating;

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

    // Ordered list of pinned (floating) column ids, restricted to columns that
    // both exist and may float. Preserves the order given by the source.
    const validIds = new Set(orderedColumns.filter((col) => canFloatColumn(col)).map((c) => c.id));
    const floating = (Array.isArray(floatingSource) ? floatingSource : []).filter((id) =>
      validIds.has(id)
    );

    return {
      columns: orderedColumns,
      visibility,
      floating,
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
          <h2 id="table-settings-title" class="table-settings-title" data-l10n-id="table-settings-title"></h2>
          <button type="button" class="modal-close" data-action="close" data-l10n-id="table-settings-close"><i class="icon-x" aria-hidden="true"></i></button>
        </header>
        <div class="table-settings-body">
          ${this._renderPaginationToggle()}
          <div class="table-settings-controls">
            <section class="table-settings-controls-group" data-l10n-id="table-settings-visibility-group">
              <h3 class="table-settings-controls-title" data-l10n-id="table-settings-visibility"></h3>
              <div class="table-settings-controls-buttons">
                <button type="button" class="btn btn-outline" data-quick-action="show-all" data-l10n-id="table-settings-show-all"></button>
                <button type="button" class="btn btn-outline" data-quick-action="hide-all" data-l10n-id="table-settings-hide-all"></button>
                <button type="button" class="btn btn-outline" data-quick-action="invert-visibility" data-l10n-id="table-settings-invert"></button>
              </div>
            </section>
            <section class="table-settings-controls-group" data-l10n-id="table-settings-ordering-group">
              <h3 class="table-settings-controls-title" data-l10n-id="table-settings-ordering"></h3>
              <div class="table-settings-controls-buttons">
                <button type="button" class="btn btn-outline" data-quick-action="reset-order" data-l10n-id="table-settings-reset-order"></button>
                <button type="button" class="btn btn-outline" data-quick-action="alphabetical" data-l10n-id="table-settings-sort-alphabetical"></button>
              </div>
            </section>
          </div>
          <div class="table-settings-search-container">
            <input type="text" class="table-settings-search" data-l10n-id="table-settings-filter">
          </div>
          <div class="table-settings-column-list" role="list"></div>
        </div>
        <footer class="table-settings-footer">
          <button type="button" class="btn btn-secondary" data-action="reset" data-l10n-id="table-settings-reset-defaults"></button>
          <div class="table-settings-footer-actions">
            <button type="button" class="btn btn-ghost" data-action="cancel" data-l10n-id="common-action-cancel"></button>
            <button type="button" class="btn btn-primary" data-action="apply" data-l10n-id="common-action-apply"></button>
          </div>
        </footer>
      </div>
    `;

    I18n.localizeTree(overlay);
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
      this.columnListEl.innerHTML = `<div class="table-settings-empty">${Utils.escapeHtml(I18n.t("table-settings-empty"))}</div>`;
      return;
    }

    const floatingSet = new Set(this._workingState.floating);
    const isFiltered = !!this._searchTerm;

    const matchesSearch = (col) => {
      if (!this._searchTerm) return true;
      return (col.label || "").toLowerCase().includes(this._searchTerm);
    };

    // Pinned columns render first, in the working floating order. The rest keep
    // their natural column order. Reordering is group-local, so each item's
    // move-button disabled state is computed from its position WITHIN its group.
    const pinnedColumns = this._workingState.floating
      .map((id) => columns.find((col) => col.id === id))
      .filter((col) => col && matchesSearch(col));

    const unpinnedColumns = columns.filter((col) => !floatingSet.has(col.id) && matchesSearch(col));

    // Group position lookup (id -> { index, length }) used for move-button state.
    const groupPosition = new Map();
    this._workingState.floating.forEach((id, idx, list) => {
      groupPosition.set(id, { index: idx, length: list.length });
    });
    const unpinnedIds = columns.filter((col) => !floatingSet.has(col.id)).map((col) => col.id);
    unpinnedIds.forEach((id, idx) => {
      groupPosition.set(id, { index: idx, length: unpinnedIds.length });
    });
    const displayIndexById = new Map();
    [...pinnedColumns, ...unpinnedColumns].forEach((col, idx) => {
      displayIndexById.set(col.id, idx);
    });

    if (pinnedColumns.length === 0 && unpinnedColumns.length === 0) {
      this.columnListEl.innerHTML = `<div class="table-settings-empty">${Utils.escapeHtml(I18n.t("table-settings-no-match"))}</div>`;
      return;
    }

    const renderItem = (column) => {
      // Visible 1-based position shown in the list (purely informational).
      const index = displayIndexById.get(column.id) ?? 0;
      const isVisible = visibility[column.id] !== false;
      const canToggle = canToggleVisibility(column);
      const disableToggleAttr = canToggle ? "" : " disabled";
      const visibilityLabel = canToggle ? "" : ` ${Utils.escapeHtml(I18n.t("table-settings-column-locked"))}`;
      const columnId = Utils.escapeHtml(column.id);

      const canFloat = canFloatColumn(column);
      const isPinned = floatingSet.has(column.id);
      const pinDisabledAttr = canFloat ? "" : " disabled";

      // Move buttons enable/disable based on position within the column's GROUP.
      const pos = groupPosition.get(column.id) || { index: 0, length: 1 };
      const moveTopDisabled = pos.index === 0;
      const moveUpDisabled = pos.index === 0;
      const moveDownDisabled = pos.index === pos.length - 1;
      const moveBottomDisabled = pos.index === pos.length - 1;

      return `
        <div class="table-settings-column-item${isPinned ? " is-pinned" : ""}" draggable="${!isFiltered}" data-column-id="${columnId}" role="listitem">
          <div class="table-settings-drag-handle" aria-hidden="true" title="${Utils.escapeHtml(I18n.t("table-settings-drag-handle"))}">⋮⋮</div>
          <span class="table-settings-column-index" aria-hidden="true">${index + 1}</span>
          <label class="table-settings-column-label${canToggle ? "" : " is-locked"}">
            <input type="checkbox" data-role="visibility-toggle" data-column-id="${columnId}" ${isVisible ? "checked" : ""}${disableToggleAttr} />
            <span class="column-name">${Utils.escapeHtml(column.label)}${visibilityLabel}</span>
          </label>
          <div class="table-settings-column-actions" aria-label="${Utils.escapeHtml(I18n.t("table-settings-column-controls"))}">
            <button type="button" class="table-settings-btn-pin${isPinned ? " is-active" : ""}" data-role="floating-toggle" data-column-id="${columnId}"${pinDisabledAttr} aria-pressed="${isPinned ? "true" : "false"}" title="${isPinned ? Utils.escapeHtml(I18n.t("table-column-unpin")) : Utils.escapeHtml(I18n.t("table-column-pin"))}">
              <i class="icon ${isPinned ? "icon-pin-off" : "icon-pin"}" aria-hidden="true"></i>
              <span class="sr-only">${isPinned ? Utils.escapeHtml(I18n.t("table-settings-unpin-column")) : Utils.escapeHtml(I18n.t("table-settings-pin-column"))}</span>
            </button>
            <div class="table-settings-move-group" role="group" aria-label="${Utils.escapeHtml(I18n.t("table-settings-move-vertical-group"))}">
              <button type="button" class="table-settings-btn-move" data-action="move-up" data-column-id="${columnId}" ${moveUpDisabled || isFiltered ? "disabled" : ""} title="${Utils.escapeHtml(I18n.t("table-settings-move-up"))}">
                <span class="icon" aria-hidden="true">↑</span>
                <span class="sr-only">${Utils.escapeHtml(I18n.t("table-settings-move-up"))}</span>
              </button>
              <button type="button" class="table-settings-btn-move" data-action="move-down" data-column-id="${columnId}" ${moveDownDisabled || isFiltered ? "disabled" : ""} title="${Utils.escapeHtml(I18n.t("table-settings-move-down"))}">
                <span class="icon" aria-hidden="true">↓</span>
                <span class="sr-only">${Utils.escapeHtml(I18n.t("table-settings-move-down"))}</span>
              </button>
            </div>
            <div class="table-settings-move-group" role="group" aria-label="${Utils.escapeHtml(I18n.t("table-settings-move-extremes-group"))}">
              <button type="button" class="table-settings-btn-move" data-action="move-top" data-column-id="${columnId}" ${moveTopDisabled || isFiltered ? "disabled" : ""} title="${Utils.escapeHtml(I18n.t("table-settings-move-top"))}">
                <span class="icon" aria-hidden="true">⇡</span>
                <span class="sr-only">${Utils.escapeHtml(I18n.t("table-settings-move-top"))}</span>
              </button>
              <button type="button" class="table-settings-btn-move" data-action="move-bottom" data-column-id="${columnId}" ${moveBottomDisabled || isFiltered ? "disabled" : ""} title="${Utils.escapeHtml(I18n.t("table-settings-move-bottom"))}">
                <span class="icon" aria-hidden="true">⇣</span>
                <span class="sr-only">${Utils.escapeHtml(I18n.t("table-settings-move-bottom"))}</span>
              </button>
            </div>
          </div>
        </div>
      `;
    };

    let html = "";

    if (pinnedColumns.length > 0) {
      html += `<div class="table-settings-group-heading">${Utils.escapeHtml(I18n.t("table-settings-group-pinned"))}</div>`;
      html += pinnedColumns.map(renderItem).join("");
      html += `<div class="table-settings-group-heading">${Utils.escapeHtml(I18n.t("table-settings-group-other"))}</div>`;
    }

    html += unpinnedColumns.map(renderItem).join("");

    this.columnListEl.innerHTML = html;

    this._attachColumnListeners();
  }

  _toggleFloating(columnId) {
    const column = this._workingState.columns.find((col) => col.id === columnId);
    if (!column || !canFloatColumn(column)) {
      return;
    }

    const idx = this._workingState.floating.indexOf(columnId);
    if (idx === -1) {
      // Newly pinned columns append to the end of the pinned group.
      this._workingState.floating.push(columnId);
    } else {
      this._workingState.floating.splice(idx, 1);
    }

    this._render();
  }

  /**
   * Reorder happens WITHIN a group: pinned (floating) columns reorder among the
   * pinned ids in `_workingState.floating`; everything else reorders within
   * `_workingState.columns`. This keeps the two groups separate (a column never
   * crosses groups via drag/move — only the Pin button changes membership).
   * Returns the array a column belongs to for reordering purposes.
   * @param {string} columnId
   * @returns {Array} the working array (floating ids OR column objects)
   */
  _reorderTargetFor(columnId) {
    return this._workingState.floating.includes(columnId)
      ? this._workingState.floating
      : this._workingState.columns;
  }

  _handleDrop(sourceId, targetId) {
    // Only reorder when source and target are in the SAME group.
    const sourcePinned = this._workingState.floating.includes(sourceId);
    const targetPinned = this._workingState.floating.includes(targetId);
    if (sourcePinned !== targetPinned) {
      return;
    }

    const arr = this._reorderTargetFor(sourceId);
    const idOf = (entry) => (typeof entry === "string" ? entry : entry.id);
    const sourceIndex = arr.findIndex((entry) => idOf(entry) === sourceId);
    const targetIndex = arr.findIndex((entry) => idOf(entry) === targetId);

    if (sourceIndex === -1 || targetIndex === -1 || sourceIndex === targetIndex) {
      return;
    }

    const [moved] = arr.splice(sourceIndex, 1);
    arr.splice(targetIndex, 0, moved);

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

    const pinButtons = this.columnListEl.querySelectorAll(
      '.table-settings-btn-pin[data-role="floating-toggle"]'
    );
    pinButtons.forEach((button) => {
      const columnId = button.dataset.columnId;
      if (!columnId) {
        return;
      }
      const handler = () => {
        this._toggleFloating(columnId);
      };
      on(button, "click", handler);
      this._columnListeners.push({ element: button, event: "click", handler });
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

      const dragEndHandler = (_e) => {
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
    // Reorder within the column's own group (pinned ids or non-pinned columns).
    const arr = this._reorderTargetFor(columnId);
    const idOf = (entry) => (typeof entry === "string" ? entry : entry.id);
    const currentIndex = arr.findIndex((entry) => idOf(entry) === columnId);
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
        targetIndex = Math.min(arr.length - 1, currentIndex + 1);
        break;
      case "move-bottom":
        targetIndex = arr.length - 1;
        break;
      default:
        return;
    }

    if (targetIndex === currentIndex) {
      return;
    }

    const [moved] = arr.splice(currentIndex, 1);
    arr.splice(targetIndex, 0, moved);

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
      floatingColumns: [...this._workingState.floating],
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
      <div class="table-settings-pagination-toggle" data-l10n-id="table-settings-pagination-group">
        <label class="table-settings-pagination-label">
          <input type="checkbox" 
                 data-role="pagination-toggle" 
                 ${checked} />
          <span class="table-settings-pagination-text" data-l10n-id="table-settings-pagination-enable"></span>
        </label>
        <span class="table-settings-pagination-hint" data-l10n-id="table-settings-pagination-hint"></span>
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
