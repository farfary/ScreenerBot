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
 * - type: Column type - 'text', 'image', 'badge', etc. (optional, default: 'text')
 * - sortable: Enable sorting (optional, default: false)
 * - width: Column width in px or 'auto' (optional)
 * - resizable: Enable column resizing (optional, default: true)
 * - visible: Initial visibility (optional, default: true)
 * - render: (value, row) => string - Custom cell renderer (optional)
 * - sortFn: (rowA, rowB) => number - Custom sort function (optional)
 * - className: CSS class for cells (optional)
 * - fallback: Default value for null/undefined (optional, default: "—")
 *
 * Image Column Configuration (type: 'image'):
 * - image: {
 *     src: string | (row) => string - Image URL or function to get URL
 *     alt: string | (row) => string - Alt text (optional)
 *     size: number - Image size in px (optional, default: 32)
 *     shape: 'circle' | 'square' | 'rounded' - Image shape (optional, default: 'rounded')
 *     fallback: string - Fallback image URL or emoji (optional, default: '🖼️')
 *     lazyLoad: boolean - Enable lazy loading (optional, default: true)
 *     withText: boolean - Show text alongside image (optional, default: false)
 *     textField: string - Field name for text to display (optional, uses column id)
 *     onClick: (row) => void - Click handler (optional)
 *     title: string | (row) => string - Tooltip text (optional)
 *   }
 *
 * Example - Token Image Column:
 * {
 *   id: 'logo',
 *   label: 'Token',
 *   type: 'image',
 *   width: 150,
 *   image: {
 *     src: (row) => row.logo_url || `https://placeholder.com/32x32`,
 *     alt: (row) => row.symbol || 'Token',
 *     size: 32,
 *     shape: 'circle',
 *     fallback: '🪙',
 *     withText: true,
 *     textField: 'symbol',
 *     title: (row) => `${row.name} (${row.symbol})`
 *   }
 * }
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
      autoSizeColumns: options.autoSizeColumns !== false,
      autoSizeSample:
        Number.isInteger(options.autoSizeSample) && options.autoSizeSample > 0
          ? options.autoSizeSample
          : 25,
      autoSizePadding:
        typeof options.autoSizePadding === "number" &&
        Number.isFinite(options.autoSizePadding)
          ? Math.max(0, options.autoSizePadding)
          : 16,
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
      columnOrder: [], // Store custom column order
      selectedRows: new Set(),
      scrollPosition: 0,
      isLoading: false,
      tableWidth: null,
      hasAutoFitted: false, // Track if columns have been auto-fitted once
      userResizedColumns: {},
    };

    this.elements = {};
    this.resizing = null;
    this._pendingRAF = null;
    this.documentClickHandler = null;
    this.scrollThrottle = null;
    this.eventHandlers = new Map(); // Store all event handlers for cleanup
    this._pendingColumnMenuOpen = false;

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

    // Determine fixed height class
    let fixedHeightClass = "";
    if (this.options.fixedHeight) {
      if (typeof this.options.fixedHeight === "string") {
        // Support 'sm', 'md', 'lg', 'xl' or custom height
        const sizeMap = {
          sm: "fixed-height-sm",
          md: "fixed-height-md",
          lg: "fixed-height-lg",
          xl: "fixed-height-xl",
        };
        fixedHeightClass = sizeMap[this.options.fixedHeight] || "fixed-height";
      } else if (this.options.fixedHeight === true) {
        fixedHeightClass = "fixed-height";
      }
    }

    container.innerHTML = `
      <div class="data-table-wrapper ${fixedHeightClass}" ${
      typeof this.options.fixedHeight === "number"
        ? `style="height: ${this.options.fixedHeight}px;"`
        : ""
    }>
        ${this._renderToolbar()}
        <div class="data-table-scroll-container">
          <table class="data-table ${this.options.compact ? "compact" : ""} ${
      this.options.zebra ? "zebra" : ""
    }">
            ${this._renderColgroup()}
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
    
    // Cache col elements for fast width updates
    this.elements.colgroup = container.querySelector("colgroup");
    this.elements.cols = {};
    if (this.elements.colgroup) {
      const cols = this.elements.colgroup.querySelectorAll("col[data-column-id]");
      cols.forEach(col => {
        const columnId = col.dataset.columnId;
        if (columnId) {
          this.elements.cols[columnId] = col;
        }
      });
    }

    if (this.elements.table && typeof this.state.tableWidth === "number") {
      this.elements.table.style.width = `${this.state.tableWidth}px`;
    }

    // Snapshot natural widths on first paint so later resizes don't cause other columns to stretch
    this._snapshotColumnWidths();

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

    // Column visibility toggle with reordering
    const menuColumns = this._getOrderedColumns(true);
    rightParts.push(`
      <div class="dt-column-toggle">
        <button class="dt-btn dt-btn-columns" title="Show/Hide Columns">
          <span class="dt-btn-icon">⚙️</span>
        </button>
        <div class="dt-column-menu" style="display: none;">
          ${menuColumns
            .map(
              (col) => `
            <label class="dt-column-menu-item" data-column-id="${col.id}">
              <span class="dt-column-drag-handle" draggable="true" title="Drag to reorder columns">☰</span>
              <input 
                type="checkbox" 
                data-column-id="${col.id}"
                ${this._isColumnVisible(col.id) ? "checked" : ""}
              />
              <span class="dt-column-label">${col.label}</span>
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
   * Get columns in the correct order
   */
  _getOrderedColumns(includeHidden = false) {
    const sourceColumns = includeHidden
      ? [...this.options.columns]
      : this.options.columns.filter((col) => this._isColumnVisible(col.id));

    if (this.state.columnOrder.length === 0) {
      return sourceColumns;
    }

    const ordered = [];
    const columnMap = new Map(sourceColumns.map((col) => [col.id, col]));

    for (const colId of this.state.columnOrder) {
      if (columnMap.has(colId)) {
        ordered.push(columnMap.get(colId));
        columnMap.delete(colId);
      }
    }

    ordered.push(...columnMap.values());

    return ordered;
  }

  /**
   * Render colgroup for efficient column width management
   * Using <col> elements allows the browser to handle column widths
   * without needing to update every cell individually
   */
  _renderColgroup() {
    const visibleColumns = this._getOrderedColumns();
    
    return `
      <colgroup>
        ${visibleColumns
          .map((col) => {
            const storedWidth = this.state.columnWidths[col.id];
            const configuredWidth = col.width;

            let widthValue = null;
            if (typeof storedWidth === "number" && !Number.isNaN(storedWidth)) {
              widthValue = `${storedWidth}px`;
            } else if (typeof configuredWidth === "number" && !Number.isNaN(configuredWidth)) {
              widthValue = `${configuredWidth}px`;
            } else if (typeof configuredWidth === "string" && configuredWidth.trim().length > 0) {
              widthValue = configuredWidth;
            } else if (this.options.autoSizeColumns === false) {
              widthValue = "120px";
            }

            const styleAttr = widthValue ? ` style="width: ${widthValue};"` : "";
            return `<col data-column-id="${col.id}"${styleAttr}>`;
          })
          .join('')}
      </colgroup>
    `;
  }

  /**
   * Render table header with sortable columns
   */
  _renderHeader() {
    const visibleColumns = this._getOrderedColumns();

    return `
      <tr>
        ${visibleColumns
          .map((col) => {
            const isSorted = this.state.sortColumn === col.id;
            const sortIcon = isSorted
              ? this.state.sortDirection === "asc"
                ? "▲"
                : "▼"
              : "";

            return `
            <th 
              data-column-id="${col.id}"
              class="dt-header-column ${col.sortable ? "sortable" : ""} ${
              isSorted ? "sorted" : ""
            }"
            >
              <div class="dt-header-content">
                <span class="dt-header-label">
                  ${col.label}
                </span>
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
      return `<tr><td colspan="100" class="dt-loading">${this.options.loadingMessage}</td></tr>`;
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
    const visibleColumns = this._getOrderedColumns();

    return visibleColumns
      .map((col) => {
        let value = row[col.id];
        let cellContent = "";
        let cellClass = col.className || "";

        // Handle different column types
        if (col.type === "actions" && col.actions) {
          cellContent = this._renderActionsCell(col, row);
          cellClass += " dt-actions-cell";
        } else if (col.type === "image" && col.image) {
          cellContent = this._renderImageCell(col, row);
          cellClass += " dt-image-cell";
        } else if (col.render && typeof col.render === "function") {
          // Custom renderer with error handling
          try {
            cellContent = col.render(value, row);
          } catch (error) {
            this._log(
              "error",
              `Render function failed for column ${col.id}`,
              error
            );
            cellContent = `<span class="dt-render-error" title="${error.message}">Error</span>`;
          }
        } else {
          // Default text rendering
          if (value === null || value === undefined) {
            cellContent = col.fallback || "—";
          } else {
            cellContent = value;
          }
        }

        // Add text wrapping class
        const wrapClass = col.wrap
          ? "wrap-text"
          : col.wrap === false
          ? "no-wrap"
          : "";

        return `
        <td data-column-id="${col.id}" 
            class="${cellClass} ${wrapClass}"
            data-row-id="${row[this.options.rowIdField] || ""}">
          ${cellContent}
        </td>
      `;
      })
      .join("");
  }

  /**
   * Render image cell with advanced features
   */
  _renderImageCell(col, row) {
    const config = col.image;

    // Get image source
    let src = "";
    if (typeof config.src === "function") {
      try {
        src = config.src(row);
      } catch (error) {
        this._log(
          "error",
          `Image src function failed for column ${col.id}`,
          error
        );
        src = "";
      }
    } else {
      src = config.src || "";
    }

    // Get alt text
    let alt = "";
    if (typeof config.alt === "function") {
      try {
        alt = config.alt(row);
      } catch (error) {
        alt = "Image";
      }
    } else {
      alt = config.alt || "Image";
    }

    // Get title/tooltip
    let title = "";
    if (typeof config.title === "function") {
      try {
        title = config.title(row);
      } catch (error) {
        title = "";
      }
    } else {
      title = config.title || "";
    }

    // Image configuration
    const size = config.size || 32;
    const shape = config.shape || "rounded"; // 'circle', 'square', 'rounded'
    const fallback = config.fallback || "🖼️";
    const lazyLoad = config.lazyLoad !== false;
    const withText = config.withText || false;
    const textField = config.textField || col.id;

    // Build CSS classes
    const shapeClass =
      {
        circle: "dt-img-circle",
        square: "dt-img-square",
        rounded: "dt-img-rounded",
      }[shape] || "dt-img-rounded";

    // Build image HTML
    let imageHtml = "";
    if (src) {
      imageHtml = `
        <img 
          class="dt-image ${shapeClass}" 
          src="${src}" 
          alt="${alt}"
          ${title ? `title="${title}"` : ""}
          ${lazyLoad ? 'loading="lazy"' : ""}
          style="width: ${size}px; height: ${size}px; object-fit: cover;"
          onerror="this.style.display='none'; this.nextElementSibling.style.display='inline-flex';"
        />
        <span class="dt-image-fallback ${shapeClass}" style="display:none; width: ${size}px; height: ${size}px; font-size: ${
        size * 0.6
      }px;">
          ${fallback}
        </span>
      `;
    } else {
      imageHtml = `
        <span class="dt-image-fallback ${shapeClass}" style="display:inline-flex; width: ${size}px; height: ${size}px; font-size: ${
        size * 0.6
      }px;">
          ${fallback}
        </span>
      `;
    }

    // Add text if configured
    let textHtml = "";
    if (withText) {
      const textValue = row[textField] || "";
      textHtml = `<span class="dt-image-text">${textValue}</span>`;
    }

    // Wrap with click handler if provided
    const hasClickHandler =
      config.onClick && typeof config.onClick === "function";
    const clickClass = hasClickHandler ? "dt-image-clickable" : "";
    const clickAttr = hasClickHandler ? `data-image-click="${col.id}"` : "";

    return `
      <div class="dt-image-container ${clickClass}" ${clickAttr} ${
      title ? `title="${title}"` : ""
    }>
        ${imageHtml}
        ${textHtml}
      </div>
    `;
  }

  /**
   * Render actions cell with buttons or dropdown menu
   *
   * @param {Object} col - Column configuration
   * @param {Object} row - Row data
   * @returns {string} - HTML for actions cell
   *
   * Example configurations:
   *
   * Multiple buttons:
   * {
   *   id: 'actions',
   *   label: 'Actions',
   *   type: 'actions',
   *   actions: {
   *     buttons: [
   *       {
   *         id: 'edit',
   *         label: 'Edit',
   *         icon: '✏️',
   *         variant: 'primary',
   *         onClick: (row) => editRow(row)
   *       },
   *       {
   *         id: 'delete',
   *         label: 'Delete',
   *         icon: '🗑️',
   *         variant: 'danger',
   *         onClick: (row) => deleteRow(row)
   *       }
   *     ]
   *   }
   * }
   *
   * Dropdown menu (three dots):
   * {
   *   id: 'actions',
   *   label: 'Actions',
   *   type: 'actions',
   *   actions: {
   *     dropdown: true,
   *     icon: '⋮', // or '•••' or '⚙️'
   *     menuPosition: 'left', // 'left' or 'right' (default)
   *     items: [
   *       {
   *         id: 'view',
   *         label: 'View Details',
   *         icon: '👁️',
   *         onClick: (row) => viewRow(row)
   *       },
   *       {
   *         id: 'edit',
   *         label: 'Edit',
   *         icon: '✏️',
   *         onClick: (row) => editRow(row)
   *       },
   *       { type: 'divider' },
   *       {
   *         id: 'delete',
   *         label: 'Delete',
   *         icon: '🗑️',
   *         variant: 'danger',
   *         onClick: (row) => deleteRow(row)
   *       }
   *     ]
   *   }
   * }
   */
  _renderActionsCell(col, row) {
    const config = col.actions;

    if (!config) {
      return "";
    }

    // Dropdown menu style
    if (config.dropdown && config.items) {
      const icon = config.icon || "⋮";
      const menuPosition = config.menuPosition || "right";
      const menuClass = menuPosition === "left" ? "menu-left" : "";

      return `
        <div class="dt-actions-container">
          <div class="dt-actions-dropdown" data-row-id="${
            row[this.options.rowIdField] || ""
          }">
            <button class="dt-actions-dropdown-trigger" data-action="dropdown-toggle">
              ${icon}
            </button>
            <div class="dt-actions-dropdown-menu ${menuClass}" style="display: none;">
              ${config.items
                .map((item) => {
                  if (item.type === "divider") {
                    return '<div class="dt-actions-dropdown-divider"></div>';
                  }

                  const itemVariant = item.variant === "danger" ? "danger" : "";
                  const disabled = item.disabled ? "disabled" : "";

                  return `
                  <div class="dt-actions-dropdown-item ${itemVariant} ${disabled}" 
                       data-action-id="${item.id}">
                    ${
                      item.icon
                        ? `<span class="dt-actions-dropdown-item-icon">${item.icon}</span>`
                        : ""
                    }
                    ${item.label}
                  </div>
                `;
                })
                .join("")}
            </div>
          </div>
        </div>
      `;
    }

    // Multiple buttons style
    if (config.buttons && Array.isArray(config.buttons)) {
      return `
        <div class="dt-actions-container">
          ${config.buttons
            .map((btn) => {
              const variant = btn.variant ? `dt-action-btn-${btn.variant}` : "";
              const size = btn.size === "sm" ? "dt-action-btn-sm" : "";
              const iconOnly =
                !btn.label && btn.icon ? "dt-action-btn-icon-only" : "";
              const disabled = btn.disabled ? "disabled" : "";

              return `
              <button class="dt-action-btn ${variant} ${size} ${iconOnly} ${disabled}" 
                      data-action-id="${btn.id}"
                      ${btn.tooltip ? `title="${btn.tooltip}"` : ""}>
                ${
                  btn.icon
                    ? `<span class="dt-action-btn-icon">${btn.icon}</span>`
                    : ""
                }
                ${btn.label ? btn.label : ""}
              </button>
            `;
            })
            .join("")}
        </div>
      `;
    }

    return "";
  }

  /**
   * Remove all attached event listeners
   */
  _removeEventListeners() {
    // Remove all stored event handlers
    this.eventHandlers.forEach(({ element, event, handler }) => {
      element.removeEventListener(event, handler);
    });
    this.eventHandlers.clear();

    // Remove document click handler
    if (this.documentClickHandler) {
      document.removeEventListener("click", this.documentClickHandler);
      this.documentClickHandler = null;
    }

    // Remove resize handlers
    document.removeEventListener("mousemove", this._handleResize);
    document.removeEventListener("mouseup", this._handleResizeEnd);
  }

  /**
   * Helper to add and track event listeners
   */
  _addEventListener(element, event, handler) {
    element.addEventListener(event, handler);
    this.eventHandlers.set(`${event}_${Date.now()}_${Math.random()}`, {
      element,
      event,
      handler,
    });
  }

  /**
   * Attach event listeners
   */
  _attachEvents() {
    // Remove old listeners first to prevent duplicates
    this._removeEventListeners();

    // Search input
    const searchInput =
      this.elements.container.querySelector(".dt-search-input");
    if (searchInput) {
      const handler = (e) => {
        this.state.searchQuery = e.target.value;
        this._applyFilters();
        this._saveState();
      };
      this._addEventListener(searchInput, "input", handler);
    }

    // Filter dropdowns
    const filterSelects =
      this.elements.container.querySelectorAll(".dt-filter");
    filterSelects.forEach((select) => {
      const handler = (e) => {
        const filterId = e.target.dataset.filterId;
        this.state.filters[filterId] = e.target.value;
        this._applyFilters();
        this._saveState();
      };
      this._addEventListener(select, "change", handler);
    });

    // Toolbar buttons
    const buttons = this.elements.container.querySelectorAll(
      ".dt-btn[data-btn-id]"
    );
    buttons.forEach((btn) => {
      const handler = () => {
        const btnId = btn.dataset.btnId;
        const btnConfig = this.options.toolbar.buttons?.find(
          (b) => b.id === btnId
        );
        if (btnConfig?.onClick) {
          btnConfig.onClick();
        }
      };
      this._addEventListener(btn, "click", handler);
    });

    // Column visibility toggle
    const columnBtn = this.elements.container.querySelector(".dt-btn-columns");
    const columnMenu = this.elements.container.querySelector(".dt-column-menu");
    if (columnBtn && columnMenu) {
      const btnHandler = (e) => {
        e.stopPropagation();
        columnMenu.style.display =
          columnMenu.style.display === "none" ? "block" : "none";
      };
      this._addEventListener(columnBtn, "click", btnHandler);

      // Close menu when clicking outside
      this.documentClickHandler = () => {
        columnMenu.style.display = "none";
      };
      document.addEventListener("click", this.documentClickHandler);

      // Column checkboxes
      const checkboxes = columnMenu.querySelectorAll('input[type="checkbox"]');
      checkboxes.forEach((cb) => {
        const handler = (e) => {
          const columnId = e.target.dataset.columnId;
          this.state.visibleColumns[columnId] = e.target.checked;
          this._saveState();
          this._renderTable();
        };
        this._addEventListener(cb, "change", handler);
      });

      // Column reordering within the settings menu
      let draggingMenuItem = null;

      const clearMenuDragHighlights = () => {
        columnMenu
          .querySelectorAll(".dt-column-menu-item.drag-over")
          .forEach((dragItem) => dragItem.classList.remove("drag-over"));
      };

      const menuItems = columnMenu.querySelectorAll(".dt-column-menu-item");
      menuItems.forEach((item) => {
        const handle = item.querySelector(".dt-column-drag-handle");
        if (!handle) {
          return;
        }

        const preventClickHandler = (e) => {
          e.preventDefault();
          e.stopPropagation();
        };
        this._addEventListener(handle, "click", preventClickHandler);

        const dragStartHandler = (e) => {
          draggingMenuItem = item;
          item.classList.add("dragging");
          e.dataTransfer.effectAllowed = "move";
          e.dataTransfer.setData(
            "text/plain",
            item.dataset.columnId || "column"
          );
        };

        const dragEndHandler = () => {
          if (draggingMenuItem) {
            draggingMenuItem.classList.remove("dragging");
            draggingMenuItem = null;
            this._updateColumnOrderFromMenu(columnMenu);
          }
          clearMenuDragHighlights();
        };

        const dragOverHandler = (e) => {
          if (!draggingMenuItem || draggingMenuItem === item) {
            return;
          }

          e.preventDefault();
          const rect = item.getBoundingClientRect();
          const shouldInsertBefore = e.clientY - rect.top < rect.height / 2;

          const parent = item.parentElement;
          if (!parent) {
            return;
          }

          if (shouldInsertBefore) {
            if (item.previousElementSibling !== draggingMenuItem) {
              parent.insertBefore(draggingMenuItem, item);
            }
          } else {
            if (item.nextElementSibling !== draggingMenuItem) {
              parent.insertBefore(draggingMenuItem, item.nextElementSibling);
            }
          }

          clearMenuDragHighlights();
          item.classList.add("drag-over");
        };

        const dragLeaveHandler = () => {
          item.classList.remove("drag-over");
        };

        const dropHandler = (e) => {
          if (!draggingMenuItem || draggingMenuItem === item) {
            return;
          }

          e.preventDefault();
          const rect = item.getBoundingClientRect();
          const shouldInsertBefore = e.clientY - rect.top < rect.height / 2;
          const parent = item.parentElement;
          if (!parent) {
            return;
          }

          if (shouldInsertBefore) {
            parent.insertBefore(draggingMenuItem, item);
          } else {
            parent.insertBefore(draggingMenuItem, item.nextElementSibling);
          }

          this._updateColumnOrderFromMenu(columnMenu);
          if (draggingMenuItem) {
            draggingMenuItem.classList.remove("dragging");
          }
          draggingMenuItem = null;
          clearMenuDragHighlights();
        };

        this._addEventListener(handle, "dragstart", dragStartHandler);
        this._addEventListener(handle, "dragend", dragEndHandler);
        this._addEventListener(item, "dragover", dragOverHandler);
        this._addEventListener(item, "dragleave", dragLeaveHandler);
        this._addEventListener(item, "drop", dropHandler);
      });

      const menuDragOverHandler = (e) => {
        if (!draggingMenuItem || e.target !== columnMenu) {
          return;
        }
        e.preventDefault();
      };

      const menuDropHandler = (e) => {
        if (!draggingMenuItem || e.target !== columnMenu) {
          return;
        }
        e.preventDefault();
        columnMenu.appendChild(draggingMenuItem);
        this._updateColumnOrderFromMenu(columnMenu);
        draggingMenuItem.classList.remove("dragging");
        draggingMenuItem = null;
        clearMenuDragHighlights();
      };

      this._addEventListener(columnMenu, "dragover", menuDragOverHandler);
      this._addEventListener(columnMenu, "drop", menuDropHandler);

      if (this._pendingColumnMenuOpen) {
        columnMenu.style.display = "block";
        this._pendingColumnMenuOpen = false;
      }
    }

    // Sortable headers
    const headers = this.elements.thead.querySelectorAll("th.sortable");
    headers.forEach((th) => {
      const handler = (e) => {
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
      };
      this._addEventListener(th, "click", handler);
    });

    // Column resizing
    const resizeHandles =
      this.elements.thead.querySelectorAll(".dt-resize-handle");
    resizeHandles.forEach((handle) => {
      const handler = (e) => {
        e.preventDefault();
        e.stopPropagation();

        const th = handle.closest("th[data-column-id]");
        if (!th) {
          return;
        }

        const columnId = th.dataset.columnId;
        const minWidth = this._getColumnMinWidth(columnId);

        this.resizing = {
          columnId,
          startX: e.pageX,
          startWidth: th.offsetWidth,
          minWidth,
          leftHeader: th,
          handle,
        };

        th.classList.add("dt-resizing");
        handle.classList.add("active");
        document.body.classList.add("dt-column-resizing");

        document.addEventListener("mousemove", this._handleResize);
        document.addEventListener("mouseup", this._handleResizeEnd);
      };
      this._addEventListener(handle, "mousedown", handler);
    });

    // Image click handlers
    const imageContainers = this.elements.tbody.querySelectorAll(
      ".dt-image-clickable[data-image-click]"
    );
    imageContainers.forEach((container) => {
      const handler = (e) => {
        e.stopPropagation(); // Prevent row click
        const columnId = container.dataset.imageClick;
        const tr = container.closest("tr");
        if (tr && tr.dataset.rowId) {
          const rowId = tr.dataset.rowId;
          const row = this.state.filteredData.find(
            (r) => String(r[this.options.rowIdField]) === String(rowId)
          );
          if (row) {
            const column = this.options.columns.find((c) => c.id === columnId);
            if (column?.image?.onClick) {
              try {
                column.image.onClick(row, e);
              } catch (error) {
                this._log(
                  "error",
                  `Image click handler failed for column ${columnId}`,
                  error
                );
              }
            }
          }
        }
      };
      this._addEventListener(container, "click", handler);
    });

    // Action button click handlers
    const actionButtons = this.elements.tbody.querySelectorAll(
      ".dt-action-btn[data-action-id]"
    );
    actionButtons.forEach((btn) => {
      const handler = (e) => {
        e.stopPropagation(); // Prevent row click
        const actionId = btn.dataset.actionId;
        const td = btn.closest("td");
        if (td && td.dataset.rowId) {
          const rowId = td.dataset.rowId;
          const row = this.state.filteredData.find(
            (r) => String(r[this.options.rowIdField]) === String(rowId)
          );
          if (row) {
            const columnId = td.dataset.columnId;
            const column = this.options.columns.find((c) => c.id === columnId);
            if (column?.actions?.buttons) {
              const action = column.actions.buttons.find(
                (a) => a.id === actionId
              );
              if (action?.onClick) {
                try {
                  action.onClick(row, e);
                } catch (error) {
                  this._log(
                    "error",
                    `Action button handler failed for action ${actionId}`,
                    error
                  );
                }
              }
            }
          }
        }
      };
      this._addEventListener(btn, "click", handler);
    });

    // Dropdown toggle handlers
    const dropdownTriggers = this.elements.tbody.querySelectorAll(
      ".dt-actions-dropdown-trigger[data-action='dropdown-toggle']"
    );
    dropdownTriggers.forEach((trigger) => {
      const handler = (e) => {
        e.stopPropagation(); // Prevent row click
        const dropdown = trigger.closest(".dt-actions-dropdown");
        const menu = dropdown.querySelector(".dt-actions-dropdown-menu");

        // Close all other dropdowns first
        const allMenus = this.elements.tbody.querySelectorAll(
          ".dt-actions-dropdown-menu"
        );
        allMenus.forEach((m) => {
          if (m !== menu) {
            m.style.display = "none";
          }
        });

        // Toggle current menu
        if (menu) {
          const isOpen = menu.style.display === "block";
          menu.style.display = isOpen ? "none" : "block";
          trigger.classList.toggle("active", !isOpen);
        }
      };
      this._addEventListener(trigger, "click", handler);
    });

    // Dropdown item click handlers
    const dropdownItems = this.elements.tbody.querySelectorAll(
      ".dt-actions-dropdown-item[data-action-id]"
    );
    dropdownItems.forEach((item) => {
      const handler = (e) => {
        e.stopPropagation(); // Prevent row click
        const actionId = item.dataset.actionId;
        const dropdown = item.closest(".dt-actions-dropdown");
        const rowId = dropdown?.dataset.rowId;

        if (rowId) {
          const row = this.state.filteredData.find(
            (r) => String(r[this.options.rowIdField]) === String(rowId)
          );
          if (row) {
            const td = dropdown.closest("td");
            const columnId = td?.dataset.columnId;
            const column = this.options.columns.find((c) => c.id === columnId);
            if (column?.actions?.items) {
              const action = column.actions.items.find(
                (a) => a.id === actionId
              );
              if (action?.onClick) {
                try {
                  action.onClick(row, e);

                  // Close dropdown after action
                  const menu = dropdown.querySelector(
                    ".dt-actions-dropdown-menu"
                  );
                  const trigger = dropdown.querySelector(
                    ".dt-actions-dropdown-trigger"
                  );
                  if (menu) menu.style.display = "none";
                  if (trigger) trigger.classList.remove("active");
                } catch (error) {
                  this._log(
                    "error",
                    `Dropdown action handler failed for action ${actionId}`,
                    error
                  );
                }
              }
            }
          }
        }
      };
      this._addEventListener(item, "click", handler);
    });

    // Close dropdowns when clicking outside
    const closeDropdownsHandler = (e) => {
      if (!e.target.closest(".dt-actions-dropdown")) {
        const allMenus = this.elements.tbody.querySelectorAll(
          ".dt-actions-dropdown-menu"
        );
        const allTriggers = this.elements.tbody.querySelectorAll(
          ".dt-actions-dropdown-trigger"
        );
        allMenus.forEach((menu) => (menu.style.display = "none"));
        allTriggers.forEach((trigger) => trigger.classList.remove("active"));
      }
    };
    this._addEventListener(document, "click", closeDropdownsHandler);

    // Row click
    if (this.options.onRowClick) {
      const handler = (e) => {
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
      };
      this._addEventListener(this.elements.tbody, "click", handler);
    }

    // Scroll position tracking (throttled to avoid excessive saves)
    const scrollHandler = () => {
      this.state.scrollPosition = this.elements.scrollContainer.scrollTop;

      // Throttle state saves to once per 500ms
      if (this.scrollThrottle) {
        clearTimeout(this.scrollThrottle);
      }
      this.scrollThrottle = setTimeout(() => {
        this._saveState();
        this.scrollThrottle = null;
      }, 500);
    };
    this._addEventListener(
      this.elements.scrollContainer,
      "scroll",
      scrollHandler
    );
  }

  /**
   * Persist column order changes from the column menu
   * @param {HTMLElement} columnMenu
   */
  _updateColumnOrderFromMenu(columnMenu) {
    if (!columnMenu) {
      return;
    }

    const orderedIds = Array.from(
      columnMenu.querySelectorAll(".dt-column-menu-item")
    )
      .map((item) => item.dataset.columnId)
      .filter(Boolean);

    if (orderedIds.length === 0) {
      return;
    }

    if (this._arraysEqual(orderedIds, this.state.columnOrder)) {
      return;
    }

    const existingMenu = this.elements.container
      ? this.elements.container.querySelector(".dt-column-menu")
      : null;
    const shouldReopen = existingMenu?.style.display === "block";

    this.state.columnOrder = orderedIds;
    this._saveState();
    this._pendingColumnMenuOpen = shouldReopen;
    this._renderTable();

    this._log("info", "Column order updated", {
      via: "column-menu",
      order: orderedIds,
    });
  }

  _arraysEqual(a = [], b = []) {
    if (a.length !== b.length) {
      return false;
    }
    return a.every((value, index) => value === b[index]);
  }

  _getColumnConfig(columnId) {
    return this.options.columns.find((col) => col.id === columnId);
  }

  _getColumnMinWidth(columnId) {
    const column = this._getColumnConfig(columnId);
    if (!column) {
      return 50;
    }
    if (typeof column.minWidth === "number" && column.minWidth >= 0) {
      return column.minWidth;
    }
    return 50;
  }

  _markColumnAsUserResized(columnId) {
    if (!columnId) {
      return;
    }
    if (!this.state.userResizedColumns) {
      this.state.userResizedColumns = {};
    }
    this.state.userResizedColumns[columnId] = true;
  }

  /**
   * Apply column width by updating the <col> element
   * This is much more efficient than updating every <td> element
   * The browser handles the column layout automatically
   */
  _applyColumnWidth(columnId, widthPx) {
    if (!columnId || !Number.isFinite(widthPx)) return;
    
    const w = Math.max(0, Math.round(widthPx));
    
    // Update <col> element (primary width control)
    const col = this.elements.cols?.[columnId];
    if (col) {
      col.style.width = `${w}px`;
      col.style.minWidth = `${w}px`;
      col.style.maxWidth = `${w}px`;
    }
    
    // Update header for proper pointer events hit-box
    const th = this.elements.thead?.querySelector(`th[data-column-id="${columnId}"]`);
    if (th) {
      th.style.width = `${w}px`;
    }
  }

  _applyTableWidth() {
    if (!this.elements.table) {
      return;
    }

    if (typeof this.state.tableWidth === "number") {
      this.elements.table.style.width = `${this.state.tableWidth}px`;
    } else {
      this.elements.table.style.width = "";
    }
  }

  _applyStoredColumnWidths() {
    if (!this.elements.table) {
      return;
    }

    Object.entries(this.state.columnWidths).forEach(([columnId, width]) => {
      if (typeof width === "number" && !Number.isNaN(width)) {
        this._applyColumnWidth(columnId, width);
      }
    });

    this._applyTableWidth();
  }

  _autoSizeColumnsFromContent() {
    if (this.options.autoSizeColumns === false) {
      return;
    }
    if (!this.elements.thead || !this.elements.tbody) {
      return;
    }

    const visibleColumns = this._getOrderedColumns();
    if (!visibleColumns || visibleColumns.length === 0) {
      return;
    }

    const allRows = Array.from(
      this.elements.tbody.querySelectorAll("tr[data-row-id]")
    );
    const sampleSize = Math.min(
      this.options.autoSizeSample,
      allRows.length
    );
    const sampleRows = sampleSize > 0 ? allRows.slice(0, sampleSize) : [];
    const padding = this.options.autoSizePadding;

    let didChange = false;

    visibleColumns.forEach((col) => {
      const columnId = col.id;
      if (!columnId || !this._isColumnVisible(columnId)) {
        return;
      }

      const hasFixedWidth =
        col.autoWidth !== true &&
        col.width !== undefined &&
        col.width !== null &&
        !(
          typeof col.width === "string" &&
          col.width.trim().toLowerCase() === "auto"
        );

      if (hasFixedWidth) {
        if (
          typeof col.width === "number" &&
          !Number.isNaN(col.width) &&
          this.state.columnWidths[columnId] !== col.width
        ) {
          this.state.columnWidths[columnId] = col.width;
          this._applyColumnWidth(columnId, col.width);
          didChange = true;
        }
        return;
      }

      if (this.state.userResizedColumns?.[columnId]) {
        return;
      }

      const headerCell = this.elements.thead.querySelector(
        `th[data-column-id="${columnId}"]`
      );

      let maxWidth = headerCell
        ? Math.ceil(headerCell.scrollWidth)
        : 0;

      sampleRows.forEach((row) => {
        const cell = row.querySelector(`td[data-column-id="${columnId}"]`);
        if (!cell) {
          return;
        }
        const cellWidth = Math.ceil(cell.scrollWidth);
        if (cellWidth > maxWidth) {
          maxWidth = cellWidth;
        }
      });

      if (maxWidth === 0 && headerCell) {
        maxWidth = Math.ceil(headerCell.offsetWidth);
      }

      const minWidth = this._getColumnMinWidth(columnId);
      const finalWidth = Math.max(minWidth, maxWidth + padding);

      if (!Number.isFinite(finalWidth)) {
        return;
      }

      const previous = this.state.columnWidths[columnId];
      if (
        !Number.isFinite(previous) ||
        Math.abs(previous - finalWidth) > 1
      ) {
        this.state.columnWidths[columnId] = finalWidth;
        this._applyColumnWidth(columnId, finalWidth);
        didChange = true;
      }
    });

    if (didChange) {
      const total = this._computeTableWidthFromState();
      if (typeof total === "number") {
        this.state.tableWidth = total;
      }
      this.state.hasAutoFitted = false;
    }
  }

  // Snapshot current natural widths for visible columns into state if missing
  _snapshotColumnWidths() {
    if (!this.elements.thead) return;
    const headers = this.elements.thead.querySelectorAll("th[data-column-id]");
    headers.forEach((th) => {
      const id = th.dataset.columnId;
      if (!id) return;
      if (typeof this.state.columnWidths[id] !== "number") {
        const w = th.offsetWidth;
        if (w && !Number.isNaN(w)) this.state.columnWidths[id] = Math.round(w);
      }
    });
    
    // Compute table width sum
    const sum = this._computeTableWidthFromState();
    if (typeof sum === "number") {
      this.state.tableWidth = sum;
      
      // Auto-fit columns to container if they overflow (only on initial load)
      if (!this.state.hasAutoFitted && this.options.fitToContainer !== false) {
        this._fitColumnsToContainer();
        this.state.hasAutoFitted = true;
      }
      
      this._applyTableWidth();
    }
  }

  _computeTableWidthFromState() {
    const cols = this._getOrderedColumns();
    if (!cols || cols.length === 0) return null;
    let total = 0;
    cols.forEach((c) => {
      if (!this._isColumnVisible(c.id)) return;
      const w = this.state.columnWidths[c.id];
      if (typeof w === "number" && !Number.isNaN(w)) total += w;
    });
    return Math.max(0, Math.round(total));
  }

  /**
   * Fit columns proportionally to container width if they would overflow
   */
  _fitColumnsToContainer() {
    if (!this.elements.scrollContainer) return;

    // Use clientWidth which excludes vertical scrollbar width
    const containerWidth = this.elements.scrollContainer.clientWidth;
    const totalWidth = this._computeTableWidthFromState();
    if (!totalWidth || totalWidth <= 0) return;

    const visibleColumns = this._getOrderedColumns();
    if (!visibleColumns || visibleColumns.length === 0) return;

    // We always attempt to match the container exactly on init-fit
    const targetWidth = Math.max(0, Math.floor(containerWidth));

    // Helper to apply final widths and snap table width to target
    const applyFinal = () => {
      // After adjusting individual columns, recompute and set table width
      const recomputed = this._computeTableWidthFromState();
      // Snap to target to avoid 1px rounding horizontal scrollbars
      this.state.tableWidth = targetWidth;
      this._applyTableWidth();

      this._log("info", "Columns fitted to container", {
        originalWidth: totalWidth,
        containerWidth: targetWidth,
        resultingWidth: recomputed,
      });
    };

    // Proportional scale when overflowing
    if (totalWidth > targetWidth) {
      const scaleFactor = targetWidth / totalWidth;
      let runningTotal = 0;
      const lastIdx = visibleColumns.length - 1;

      visibleColumns.forEach((col, idx) => {
        const currentWidth = this.state.columnWidths[col.id];
        if (typeof currentWidth === "number" && !Number.isNaN(currentWidth)) {
          const minWidth = this._getColumnMinWidth(col.id);
          // Round down to avoid overflow accumulation, we'll fix remainder on last column
          let scaled = Math.max(minWidth, Math.floor(currentWidth * scaleFactor));

          // On last column, absorb remainder so total matches targetWidth exactly (or as close as min allows)
          if (idx === lastIdx) {
            const remainder = targetWidth - runningTotal;
            // If remainder is less than minWidth, respect minWidth but it may still overflow in extreme cases
            scaled = Math.max(minWidth, remainder);
          }

          runningTotal += scaled;
          this.state.columnWidths[col.id] = scaled;
          this._applyColumnWidth(col.id, scaled);
        }
      });

      applyFinal();
      return;
    }

    // If under target, expand last column to fill remaining gap for exact fit
    if (totalWidth < targetWidth) {
      // Choose the last visible column to absorb the gap
      const lastCol = visibleColumns[visibleColumns.length - 1];
      if (lastCol) {
        const currentWidth = this.state.columnWidths[lastCol.id];
        if (typeof currentWidth === "number" && !Number.isNaN(currentWidth)) {
          const gap = targetWidth - totalWidth;
          const newWidth = currentWidth + gap;
          this.state.columnWidths[lastCol.id] = newWidth;
          this._applyColumnWidth(lastCol.id, newWidth);
        }
      }

      applyFinal();
      return;
    }

    // Already exactly matching
    applyFinal();
  }

  /**
   * Handle column resize drag with RAF throttling for smooth performance
   */
  _handleResize = (e) => {
    if (!this.resizing) return;
    e.preventDefault();

    // Throttle updates with requestAnimationFrame
    if (this._pendingRAF) return;
    
    this._pendingRAF = requestAnimationFrame(() => {
      this._pendingRAF = null;
      
      if (!this.resizing) return;

      const { columnId, startX, startWidth, minWidth } = this.resizing;

      const effectiveMin = typeof minWidth === "number" ? minWidth : 50;
      let diff = e.pageX - startX;

      // Prevent shrinking beyond min width
      const maxDecrease = startWidth - effectiveMin;
      if (diff < -maxDecrease) {
        diff = -maxDecrease;
      }

      const newWidth = Math.max(effectiveMin, Math.round(startWidth + diff));
      this._markColumnAsUserResized(columnId);
      this.state.columnWidths[columnId] = newWidth;
      this._applyColumnWidth(columnId, newWidth);

      // Grow table width - don't shrink other columns
      const total = this._computeTableWidthFromState();
      if (typeof total === "number") {
        this.state.tableWidth = total;
        this._applyTableWidth();
      }
    });
  };

  /**
   * Handle resize end
   */
  _handleResizeEnd = () => {
    // Cancel any pending RAF
    if (this._pendingRAF) {
      cancelAnimationFrame(this._pendingRAF);
      this._pendingRAF = null;
    }
    
    if (this.resizing) {
      const { leftHeader, handle } = this.resizing;
      if (leftHeader) {
        leftHeader.classList.remove("dt-resizing");
      }
      if (handle) {
        handle.classList.remove("active");
      }

      this._saveState();
      this.resizing = null;
    }

    document.body.classList.remove("dt-column-resizing");
    document.removeEventListener("mousemove", this._handleResize);
    document.removeEventListener("mouseup", this._handleResizeEnd);
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

    if (this.elements.table) {
      const colgroupMarkup = this._renderColgroup().trim();
      const existingColgroup = this.elements.table.querySelector("colgroup");
      if (existingColgroup) {
        existingColgroup.outerHTML = colgroupMarkup;
      } else {
        this.elements.table.insertAdjacentHTML("afterbegin", colgroupMarkup);
      }
    }

    // Re-query elements after innerHTML update to ensure fresh references
    if (this.elements.container) {
      this.elements.thead = this.elements.container.querySelector("thead");
      this.elements.tbody = this.elements.container.querySelector("tbody");
      
      // Re-cache col elements
      this.elements.colgroup = this.elements.container.querySelector("colgroup");
      this.elements.cols = {};
      if (this.elements.colgroup) {
        const cols = this.elements.colgroup.querySelectorAll("col[data-column-id]");
        cols.forEach(col => {
          const columnId = col.dataset.columnId;
          if (columnId) {
            this.elements.cols[columnId] = col;
          }
        });
      }
    }

    // Make sure widths are captured and applied consistently
    this._autoSizeColumnsFromContent();
    this._snapshotColumnWidths();
    this._applyStoredColumnWidths();

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
      if (
        saved.userResizedColumns &&
        typeof saved.userResizedColumns === "object"
      ) {
        this.state.userResizedColumns = { ...saved.userResizedColumns };
      } else {
        this.state.userResizedColumns = {};
      }
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
      columnOrder: this.state.columnOrder,
      tableWidth: this.state.tableWidth,
      userResizedColumns: this.state.userResizedColumns,
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
    this.state.hasAutoFitted = false;
    this._applyFilters();
    this._log("info", "Data set", { rows: data.length });
  }

  /**
   * Clear all table data
   */
  clearData() {
    this.state.data = [];
    this.state.filteredData = [];
    this.state.hasAutoFitted = false;
    this._renderTable();
    this._log("info", "Data cleared");
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
    if (!this.options.onRefresh) {
      this._renderTable();
      return Promise.resolve();
    }

    this.state.isLoading = true;
    this._renderTable();

    const refreshPromise = Promise.resolve(this.options.onRefresh())
      .then(() => {
        this.state.isLoading = false;
        this._renderTable();
      })
      .catch((err) => {
        this.state.isLoading = false;
        this._log("error", "Refresh failed", err);
        this._renderTable();
        throw err;
      });

    return refreshPromise;
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
    // Remove all event listeners
    this._removeEventListeners();

    // Cancel any pending RAF
    if (this._pendingRAF) {
      cancelAnimationFrame(this._pendingRAF);
      this._pendingRAF = null;
    }

    // Clean up resize listeners
    if (this.resizing) {
      const { leftHeader, handle } = this.resizing;
      if (leftHeader) {
        leftHeader.classList.remove("dt-resizing");
      }
      if (handle) {
        handle.classList.remove("active");
      }
      document.removeEventListener("mousemove", this._handleResize);
      document.removeEventListener("mouseup", this._handleResizeEnd);
      this.resizing = null;
    }

    document.body.classList.remove("dt-column-resizing");

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
