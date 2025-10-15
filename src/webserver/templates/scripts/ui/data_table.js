/**
 * DataTable - Advanced reusable table component
 *
 * Features:
 * - Sortable columns with state persistence
 * - Resizable columns with drag handles
 * - Column visibility toggle (show/hide)
 * - Search and filtering
 * - Toolbar with custom buttons
 * - Scroll position restoration
 * - State persistence via localStorage
 * - Custom cell renderers
 * - Row actions and selection
 * - Optional logging
 *
 * Usage:
 * ```js
 * import { DataTable } from "../ui/data_table.js";
 *
 * const table = new DataTable({
 *   container: '#table-root',
 *   columns: [
 *     {
 *       id: 'name',
 *       label: 'Name',
 *       sortable: true,
 *       width: 200,
 *       resizable: true,
 *       render: (value, row) => `<strong>${value}</strong>`,
 *       sortFn: (a, b) => a.name.localeCompare(b.name)
 *     },
 *     {
 *       id: 'status',
 *       label: 'Status',
 *       sortable: true,
 *       width: 100,
 *       render: (val) => `<span class="badge">${val}</span>`
 *     }
 *   ],
 *   toolbar: {
 *     search: { enabled: true, placeholder: 'Search...' },
 *     filters: [
 *       {
 *         id: 'status',
 *         label: 'Status',
 *         options: [
 *           { value: 'all', label: 'All Statuses' },
 *           { value: 'active', label: 'Active' },
 *           { value: 'inactive', label: 'Inactive' }
 *         ],
 *         filterFn: (row, value) => value === 'all' || row.status === value
 *       }
 *     ],
 *     buttons: [
 *       { id: 'refresh', label: 'Refresh', icon: '🔄', onClick: () => table.refresh() }
 *     ]
 *   },
 *   sorting: { column: 'name', direction: 'asc' },
 *   stateKey: 'my-table',
 *   enableLogging: true,
 *   onRefresh: async () => {
 *     const data = await fetchData();
 *     table.setData(data);
 *   }
 * });
 *
 * table.setData(rowsArray);
 * ```
 *
 * Column Configuration:
 * - id: Unique column identifier (required)
 * - label: Display name (required)
 * - sortable: Enable sorting (optional, default: false)
 * - width: Column width in px or 'auto' (optional)
 * - resizable: Enable column resizing (optional, default: true)
 * - visible: Initial visibility (optional, default: true)
 * - render: (value, row) => string - Custom cell renderer (optional)
 * - sortFn: (rowA, rowB) => number - Custom sort function (optional)
 * - className: CSS class for cells (optional)
 * - fallback: Default value for null/undefined (optional, default: "—")
 *
 * Filter Options Format:
 * - Must be objects with { value: string, label: string }
 * - filterFn: (row, filterValue) => boolean - Custom filter logic (required)
 */

import * as AppState from "../core/app_state.js";
import { $, $$, cls } from "../core/dom.js";

export class DataTable {
  constructor(options) {
    this.options = {
      container: options.container,
      columns: options.columns || [],
      data: options.data || [],
      toolbar: options.toolbar || {},
      sorting: options.sorting || { column: null, direction: "asc" },
      stateKey: options.stateKey || "data-table",
      enableLogging: options.enableLogging || false,
      rowIdField: options.rowIdField || "id",
      emptyMessage: options.emptyMessage || "No data to display",
      loadingMessage: options.loadingMessage || "Loading...",
      onRefresh: options.onRefresh || null,
      onRowClick: options.onRowClick || null,
      onSelectionChange: options.onSelectionChange || null,
      stickyHeader: options.stickyHeader !== false,
      zebra: options.zebra !== false,
      compact: options.compact || false,
      ...options,
    };

    this.state = {
      data: [],
      filteredData: [],
      sortColumn: this.options.sorting.column,
      sortDirection: this.options.sorting.direction,
      searchQuery: "",
      filters: {},
      columnWidths: {},
      visibleColumns: {},
      selectedRows: new Set(),
      scrollPosition: 0,
      isLoading: false,
    };

    this.elements = {};
    this.resizing = null;
    this.documentClickHandler = null;
    this.scrollThrottle = null;

    this._loadState();
    this._init();
  }

  /**
   * Initialize the table
   */
  _init() {
    const container =
      typeof this.options.container === "string"
        ? $(this.options.container)
        : this.options.container;

    if (!container) {
      this._log("error", "Container not found:", this.options.container);
      return;
    }

    this.elements.container = container;
    this._render();
    this._attachEvents();

    if (this.options.data.length > 0) {
      this.setData(this.options.data);
    }

    this._log("info", "DataTable initialized", {
      columns: this.options.columns.length,
    });
  }

  /**
   * Render the complete table structure
   */
  _render() {
    const { container } = this.elements;

    container.innerHTML = `
      <div class="data-table-wrapper">
        ${this._renderToolbar()}
        <div class="data-table-scroll-container">
          <table class="data-table ${this.options.compact ? "compact" : ""} ${
      this.options.zebra ? "zebra" : ""
    }">
            <thead class="${this.options.stickyHeader ? "sticky" : ""}">
              ${this._renderHeader()}
            </thead>
            <tbody>
              ${this._renderBody()}
            </tbody>
          </table>
        </div>
      </div>
    `;

    this.elements.toolbar = container.querySelector(".data-table-toolbar");
    this.elements.scrollContainer = container.querySelector(
      ".data-table-scroll-container"
    );
    this.elements.table = container.querySelector(".data-table");
    this.elements.thead = container.querySelector("thead");
    this.elements.tbody = container.querySelector("tbody");

    // Restore scroll position
    if (this.state.scrollPosition) {
      this.elements.scrollContainer.scrollTop = this.state.scrollPosition;
    }
  }

  /**
   * Render toolbar with search, filters, and buttons
   */
  _renderToolbar() {
    const { toolbar } = this.options;
    if (!toolbar || Object.keys(toolbar).length === 0) return "";

    const parts = [];

    // Left section: Search and filters
    const leftParts = [];

    if (toolbar.search?.enabled) {
      leftParts.push(`
        <div class="dt-search">
          <input 
            type="text" 
            class="dt-search-input" 
            placeholder="${toolbar.search.placeholder || "Search..."}"
            value="${this.state.searchQuery}"
          />
        </div>
      `);
    }

    if (toolbar.filters && toolbar.filters.length > 0) {
      toolbar.filters.forEach((filter) => {
        // Filter options must be objects with { value, label }
        const defaultValue = filter.options[0].value;
        const currentValue = this.state.filters[filter.id] || defaultValue;

        leftParts.push(`
          <select class="dt-filter" data-filter-id="${filter.id}">
            ${filter.options
              .map(
                (opt) => `
              <option value="${opt.value}" ${
                  opt.value === currentValue ? "selected" : ""
                }>
                ${opt.label}
              </option>
            `
              )
              .join("")}
          </select>
        `);
      });
    }

    // Right section: Buttons and column toggle
    const rightParts = [];

    if (toolbar.buttons && toolbar.buttons.length > 0) {
      toolbar.buttons.forEach((btn) => {
        rightParts.push(`
          <button class="dt-btn" data-btn-id="${btn.id}" title="${
          btn.tooltip || btn.label
        }">
            ${btn.icon ? `<span class="dt-btn-icon">${btn.icon}</span>` : ""}
            <span class="dt-btn-label">${btn.label}</span>
          </button>
        `);
      });
    }

    // Column visibility toggle
    rightParts.push(`
      <div class="dt-column-toggle">
        <button class="dt-btn dt-btn-columns" title="Show/Hide Columns">
          <span class="dt-btn-icon">⚙️</span>
        </button>
        <div class="dt-column-menu" style="display: none;">
          ${this.options.columns
            .map(
              (col) => `
            <label class="dt-column-menu-item">
              <input 
                type="checkbox" 
                data-column-id="${col.id}"
                ${this._isColumnVisible(col.id) ? "checked" : ""}
              />
              <span>${col.label}</span>
            </label>
          `
            )
            .join("")}
        </div>
      </div>
    `);

    return `
      <div class="data-table-toolbar">
        <div class="dt-toolbar-left">
          ${leftParts.join("")}
        </div>
        <div class="dt-toolbar-right">
          ${rightParts.join("")}
        </div>
      </div>
    `;
  }

  /**
   * Render table header with sortable columns
   */
  _renderHeader() {
    const visibleColumns = this.options.columns.filter((col) =>
      this._isColumnVisible(col.id)
    );

    return `
      <tr>
        ${visibleColumns
          .map((col) => {
            const width =
              this.state.columnWidths[col.id] || col.width || "auto";
            const isSorted = this.state.sortColumn === col.id;
            const sortIcon = isSorted
              ? this.state.sortDirection === "asc"
                ? "▲"
                : "▼"
              : "";

            return `
            <th 
              data-column-id="${col.id}"
              style="width: ${
                typeof width === "number" ? width + "px" : width
              }; ${col.minWidth ? "min-width: " + col.minWidth + "px;" : ""}"
              class="${col.sortable ? "sortable" : ""} ${
              isSorted ? "sorted" : ""
            }"
            >
              <div class="dt-header-content">
                <span class="dt-header-label">${col.label}</span>
                ${
                  col.sortable
                    ? `<span class="dt-sort-icon">${sortIcon}</span>`
                    : ""
                }
              </div>
              ${
                col.resizable !== false
                  ? '<div class="dt-resize-handle"></div>'
                  : ""
              }
            </th>
          `;
          })
          .join("")}
      </tr>
    `;
  }

  /**
   * Render table body rows
   */
  _renderBody() {
    if (this.state.isLoading) {
      return `<tr><td colspan="100" class="dt-empty">${this.options.loadingMessage}</td></tr>`;
    }

    const data = this.state.filteredData;

    if (data.length === 0) {
      return `<tr><td colspan="100" class="dt-empty">${this.options.emptyMessage}</td></tr>`;
    }

    return data
      .map((row, index) => {
        const rowId = row[this.options.rowIdField] || index;
        const isSelected = this.state.selectedRows.has(rowId);

        return `
        <tr 
          data-row-id="${rowId}" 
          class="${isSelected ? "selected" : ""}"
        >
          ${this._renderRow(row)}
        </tr>
      `;
      })
      .join("");
  }

  /**
   * Render individual row cells
   */
  _renderRow(row) {
    const visibleColumns = this.options.columns.filter((col) =>
      this._isColumnVisible(col.id)
    );

    return visibleColumns
      .map((col) => {
        let value = row[col.id];

        // Custom renderer with error handling
        if (col.render && typeof col.render === "function") {
          try {
            value = col.render(value, row);
          } catch (error) {
            this._log(
              "error",
              `Render function failed for column ${col.id}`,
              error
            );
            value = `<span class="dt-render-error" title="${error.message}">Error</span>`;
          }
        } else if (value === null || value === undefined) {
          value = col.fallback || "—";
        }

        return `
        <td data-column-id="${col.id}" class="${col.className || ""}">
          ${value}
        </td>
      `;
      })
      .join("");
  }

  /**
   * Attach event listeners
   */
  _attachEvents() {
    // Search input
    const searchInput =
      this.elements.container.querySelector(".dt-search-input");
    if (searchInput) {
      searchInput.addEventListener("input", (e) => {
        this.state.searchQuery = e.target.value;
        this._applyFilters();
        this._saveState();
      });
    }

    // Filter dropdowns
    const filterSelects =
      this.elements.container.querySelectorAll(".dt-filter");
    filterSelects.forEach((select) => {
      select.addEventListener("change", (e) => {
        const filterId = e.target.dataset.filterId;
        this.state.filters[filterId] = e.target.value;
        this._applyFilters();
        this._saveState();
      });
    });

    // Toolbar buttons
    const buttons = this.elements.container.querySelectorAll(
      ".dt-btn[data-btn-id]"
    );
    buttons.forEach((btn) => {
      btn.addEventListener("click", () => {
        const btnId = btn.dataset.btnId;
        const btnConfig = this.options.toolbar.buttons?.find(
          (b) => b.id === btnId
        );
        if (btnConfig?.onClick) {
          btnConfig.onClick();
        }
      });
    });

    // Column visibility toggle
    const columnBtn = this.elements.container.querySelector(".dt-btn-columns");
    const columnMenu = this.elements.container.querySelector(".dt-column-menu");
    if (columnBtn && columnMenu) {
      columnBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        columnMenu.style.display =
          columnMenu.style.display === "none" ? "block" : "none";
      });

      // Close menu when clicking outside (cleanup on destroy)
      if (this.documentClickHandler) {
        document.removeEventListener("click", this.documentClickHandler);
      }
      this.documentClickHandler = () => {
        columnMenu.style.display = "none";
      };
      document.addEventListener("click", this.documentClickHandler);

      // Column checkboxes
      const checkboxes = columnMenu.querySelectorAll('input[type="checkbox"]');
      checkboxes.forEach((cb) => {
        cb.addEventListener("change", (e) => {
          const columnId = e.target.dataset.columnId;
          this.state.visibleColumns[columnId] = e.target.checked;
          this._saveState();
          this._renderTable();
        });
      });
    }

    // Sortable headers
    const headers = this.elements.thead.querySelectorAll("th.sortable");
    headers.forEach((th) => {
      th.addEventListener("click", (e) => {
        if (e.target.classList.contains("dt-resize-handle")) return;

        const columnId = th.dataset.columnId;
        if (this.state.sortColumn === columnId) {
          this.state.sortDirection =
            this.state.sortDirection === "asc" ? "desc" : "asc";
        } else {
          this.state.sortColumn = columnId;
          this.state.sortDirection = "asc";
        }

        this._applySort();
        this._saveState();
        this._renderTable();
      });
    });

    // Column resizing
    const resizeHandles =
      this.elements.thead.querySelectorAll(".dt-resize-handle");
    resizeHandles.forEach((handle) => {
      handle.addEventListener("mousedown", (e) => {
        e.preventDefault();
        const th = handle.parentElement;
        const columnId = th.dataset.columnId;

        this.resizing = {
          columnId,
          startX: e.pageX,
          startWidth: th.offsetWidth,
        };

        document.addEventListener("mousemove", this._handleResize);
        document.addEventListener("mouseup", this._handleResizeEnd);
      });
    });

    // Row click
    if (this.options.onRowClick) {
      this.elements.tbody.addEventListener("click", (e) => {
        const tr = e.target.closest("tr");
        if (tr && tr.dataset.rowId) {
          const rowId = tr.dataset.rowId;
          const row = this.state.filteredData.find(
            (r) => String(r[this.options.rowIdField]) === String(rowId)
          );
          if (row) {
            this.options.onRowClick(row, e);
          }
        }
      });
    }

    // Scroll position tracking (throttled to avoid excessive saves)
    this.elements.scrollContainer.addEventListener("scroll", () => {
      this.state.scrollPosition = this.elements.scrollContainer.scrollTop;

      // Throttle state saves to once per 500ms
      if (this.scrollThrottle) {
        clearTimeout(this.scrollThrottle);
      }
      this.scrollThrottle = setTimeout(() => {
        this._saveState();
        this.scrollThrottle = null;
      }, 500);
    });
  }

  /**
   * Handle column resize drag
   */
  _handleResize = (e) => {
    if (!this.resizing) return;

    const diff = e.pageX - this.resizing.startX;
    const newWidth = Math.max(50, this.resizing.startWidth + diff);

    this.state.columnWidths[this.resizing.columnId] = newWidth;

    const th = this.elements.thead.querySelector(
      `th[data-column-id="${this.resizing.columnId}"]`
    );
    if (th) {
      th.style.width = newWidth + "px";
    }
  };

  /**
   * Handle resize end
   */
  _handleResizeEnd = () => {
    if (this.resizing) {
      this._saveState();
      this.resizing = null;
      document.removeEventListener("mousemove", this._handleResize);
      document.removeEventListener("mouseup", this._handleResizeEnd);
    }
  };

  /**
   * Apply filters (search + custom filters)
   */
  _applyFilters() {
    let data = [...this.state.data];

    // Apply search
    if (this.state.searchQuery) {
      const query = this.state.searchQuery.toLowerCase();
      data = data.filter((row) => {
        return this.options.columns.some((col) => {
          const value = String(row[col.id] || "").toLowerCase();
          return value.includes(query);
        });
      });
    }

    // Apply custom filters
    if (this.options.toolbar.filters) {
      this.options.toolbar.filters.forEach((filter) => {
        const filterValue = this.state.filters[filter.id];
        if (filterValue && filterValue !== "all" && filter.filterFn) {
          data = data.filter((row) => filter.filterFn(row, filterValue));
        }
      });
    }

    this.state.filteredData = data;
    this._applySort();
    this._renderTable();
  }

  /**
   * Apply sorting
   */
  _applySort() {
    if (!this.state.sortColumn) return;

    const column = this.options.columns.find(
      (c) => c.id === this.state.sortColumn
    );
    if (!column) return;

    this.state.filteredData.sort((a, b) => {
      // Custom sort function receives full row objects
      if (column.sortFn) {
        const result = column.sortFn(a, b);
        return this.state.sortDirection === "asc" ? result : -result;
      }

      // Default sorting by column values
      let aVal = a[this.state.sortColumn];
      let bVal = b[this.state.sortColumn];

      if (aVal === null || aVal === undefined) aVal = "";
      if (bVal === null || bVal === undefined) bVal = "";

      if (typeof aVal === "string") aVal = aVal.toLowerCase();
      if (typeof bVal === "string") bVal = bVal.toLowerCase();

      const result = aVal < bVal ? -1 : aVal > bVal ? 1 : 0;
      return this.state.sortDirection === "asc" ? result : -result;
    });
  }

  /**
   * Re-render table content only (not full structure)
   */
  _renderTable() {
    if (this.elements.thead) {
      this.elements.thead.innerHTML = this._renderHeader();
    }
    if (this.elements.tbody) {
      this.elements.tbody.innerHTML = this._renderBody();
    }
    this._attachEvents();
  }

  /**
   * Check if column is visible
   */
  _isColumnVisible(columnId) {
    if (columnId in this.state.visibleColumns) {
      return this.state.visibleColumns[columnId];
    }
    const col = this.options.columns.find((c) => c.id === columnId);
    return col ? col.visible !== false : true;
  }

  /**
   * Load state from localStorage
   */
  _loadState() {
    const saved = AppState.load(this.options.stateKey);
    if (saved) {
      this.state = { ...this.state, ...saved };
      this._log("info", "State loaded", saved);
    }
  }

  /**
   * Save state to localStorage
   */
  _saveState() {
    const toSave = {
      sortColumn: this.state.sortColumn,
      sortDirection: this.state.sortDirection,
      searchQuery: this.state.searchQuery,
      filters: this.state.filters,
      columnWidths: this.state.columnWidths,
      visibleColumns: this.state.visibleColumns,
      scrollPosition: this.state.scrollPosition,
    };
    AppState.save(this.options.stateKey, toSave);
    this._log("debug", "State saved", toSave);
  }

  /**
   * Set table data
   */
  setData(data) {
    this.state.data = data;
    this.state.filteredData = [...data];
    this._applyFilters();
    this._log("info", "Data set", { rows: data.length });
  }

  /**
   * Get current filtered data
   */
  getData() {
    return this.state.filteredData;
  }

  /**
   * Refresh table (re-render)
   */
  refresh() {
    if (this.options.onRefresh) {
      this.state.isLoading = true;
      this._renderTable();

      Promise.resolve(this.options.onRefresh())
        .then(() => {
          this.state.isLoading = false;
          this._renderTable();
        })
        .catch((err) => {
          this.state.isLoading = false;
          this._log("error", "Refresh failed", err);
          this._renderTable();
        });
    } else {
      this._renderTable();
    }
  }

  /**
   * Update single row
   */
  updateRow(rowId, newData) {
    const index = this.state.data.findIndex(
      (r) => String(r[this.options.rowIdField]) === String(rowId)
    );
    if (index !== -1) {
      this.state.data[index] = { ...this.state.data[index], ...newData };
      this._applyFilters();
      this._log("info", "Row updated", { rowId });
    }
  }

  /**
   * Clear selection
   */
  clearSelection() {
    this.state.selectedRows.clear();
    this._renderTable();
  }

  /**
   * Destroy table and cleanup
   */
  destroy() {
    // Clean up resize listeners
    if (this.resizing) {
      document.removeEventListener("mousemove", this._handleResize);
      document.removeEventListener("mouseup", this._handleResizeEnd);
    }

    // Clean up document click listener
    if (this.documentClickHandler) {
      document.removeEventListener("click", this.documentClickHandler);
      this.documentClickHandler = null;
    }

    // Clean up scroll throttle
    if (this.scrollThrottle) {
      clearTimeout(this.scrollThrottle);
      this.scrollThrottle = null;
    }

    // Clear container
    if (this.elements.container) {
      this.elements.container.innerHTML = "";
    }

    this._log("info", "DataTable destroyed");
  }

  /**
   * Logging utility
   */
  _log(level, message, data) {
    if (!this.options.enableLogging) return;

    const prefix = `[DataTable:${this.options.stateKey}]`;
    if (data) {
      console[level](prefix, message, data);
    } else {
      console[level](prefix, message);
    }
  }
}
