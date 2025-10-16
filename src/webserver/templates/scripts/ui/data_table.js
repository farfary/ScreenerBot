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
 * - wrap: Text wrapping behavior (optional)
 *   - true: Allow multi-line text wrapping (word-break)
 *   - false: Single line with ellipsis truncation (recommended for long text)
 *   - undefined: Default behavior (respects uniformRowHeight if set)
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
import { $ } from "../core/dom.js";
import { TableToolbarView } from "./table_toolbar.js";

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
    this.options.lockColumnWidths = options.lockColumnWidths === true;
    // Uniform row height: number of text lines to display (clamps content)
    // truthy enables, number sets lines; true defaults to 2 lines
    if (options.uniformRowHeight === true) {
      this.options.uniformRowHeight = 2;
    } else if (
      typeof options.uniformRowHeight === "string" &&
      /^\d+$/.test(options.uniformRowHeight.trim())
    ) {
      this.options.uniformRowHeight = parseInt(options.uniformRowHeight.trim(), 10) || undefined;
    } else if (
      typeof options.uniformRowHeight === "number" &&
      Number.isFinite(options.uniformRowHeight) &&
      options.uniformRowHeight > 0
    ) {
      // keep as-is
    } else if (options.uniformRowHeight === undefined) {
      // leave undefined when not provided
    } else {
      // disable on invalid values
      this.options.uniformRowHeight = undefined;
    }

    this.state = {
      data: [],
      filteredData: [],
      sortColumn: this.options.sorting.column,
      sortDirection: this.options.sorting.direction,
      searchQuery: "",
      filters: {},
      customControls: {},
      columnWidths: {},
      visibleColumns: {},
      columnOrder: [], // Store custom column order
      selectedRows: new Set(),
      scrollPosition: 0,
      isLoading: false,
      tableWidth: null,
      hasAutoFitted: false, // Track if columns have been auto-fitted once
      userResizedColumns: {},
      columnWidthsLocked: false,
    };

    this.elements = {};
    this.toolbarView = null;
    this.resizing = null;
    this._pendingRAF = null;
    this.documentClickHandler = null;
    this.scrollThrottle = null;
    this.eventHandlers = new Map(); // Store all event handlers for cleanup
    this._pendingColumnMenuOpen = false;
    this._pagination = this._initializePagination(this.options.pagination);
    this._paginationScrollRAF = null;
    this._pendingRenderOptions = null;

    this._loadState();
    this._init();
  }

  _initializePagination(paginationOptions) {
    if (!paginationOptions || typeof paginationOptions.loadPage !== "function") {
      return null;
    }

    const threshold =
      typeof paginationOptions.threshold === "number" &&
      Number.isFinite(paginationOptions.threshold)
        ? Math.max(0, paginationOptions.threshold)
        : 320;

    const maxRows =
      typeof paginationOptions.maxRows === "number" &&
      Number.isFinite(paginationOptions.maxRows) &&
      paginationOptions.maxRows > 0
        ? Math.floor(paginationOptions.maxRows)
        : null;

    const context =
      paginationOptions.context && typeof paginationOptions.context === "object"
        ? { ...paginationOptions.context }
        : {};

    return {
      enabled: true,
      loadPage: paginationOptions.loadPage,
      threshold,
      maxRows,
      autoLoad: paginationOptions.autoLoad !== false,
      initialCursor: paginationOptions.initialCursor ?? null,
      initialPrevCursor: paginationOptions.initialPrevCursor ?? null,
      initialHasMoreNext: paginationOptions.initialHasMoreNext,
      initialHasMorePrev: paginationOptions.initialHasMorePrev,
      cursorNext: paginationOptions.initialCursor ?? null,
      cursorPrev: paginationOptions.initialPrevCursor ?? null,
      hasMoreNext:
        paginationOptions.initialHasMoreNext !== undefined
          ? Boolean(paginationOptions.initialHasMoreNext)
          : true,
      hasMorePrev:
        paginationOptions.initialHasMorePrev !== undefined
          ? Boolean(paginationOptions.initialHasMorePrev)
          : Boolean(paginationOptions.initialPrevCursor),
      loadingNext: false,
      loadingPrev: false,
      loadingInitial: false,
      pendingRequest: null,
      abortController: null,
      context,
      dedupe: paginationOptions.dedupe !== false,
      dedupeKey:
        typeof paginationOptions.dedupeKey === "function"
          ? paginationOptions.dedupeKey
          : null,
      rowIdField: paginationOptions.rowIdField || this.options.rowIdField,
      preserveScrollOnAppend:
        paginationOptions.preserveScrollOnAppend !== false,
      preserveScrollOnPrepend:
        paginationOptions.preserveScrollOnPrepend !== false,
      onPageLoaded:
        typeof paginationOptions.onPageLoaded === "function"
          ? paginationOptions.onPageLoaded
          : null,
      onStateChange:
        typeof paginationOptions.onStateChange === "function"
          ? paginationOptions.onStateChange
          : null,
      debounceMs:
        typeof paginationOptions.debounceMs === "number" &&
        Number.isFinite(paginationOptions.debounceMs)
          ? Math.max(0, paginationOptions.debounceMs)
          : 120,
      total: null,
      meta: {},
    };
  }

  _getSortingMode() {
    const mode = this.options?.sorting?.mode;
    return mode === "server" ? "server" : "client";
  }

  _getSearchMode() {
    const searchConfig = this.options?.toolbar?.search || {};
    return searchConfig.mode === "server" ? "server" : "client";
  }

  _isServerFilter(filterConfig) {
    if (!filterConfig) {
      return false;
    }
    return filterConfig.mode === "server";
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
    this._setupGlobalCleanup();

    if (this.options.data.length > 0) {
      this.setData(this.options.data);
    }

    this._log("info", "DataTable initialized", {
      columns: this.options.columns.length,
    });

    if (this._pagination?.enabled && this._pagination.autoLoad) {
      this.reload({ reason: "init", silent: true }).catch((error) => {
        this._log("error", "Initial pagination load failed", error);
      });
    }
  }

  /**
   * Setup global cleanup handlers to ensure cursor is never stuck
   */
  _setupGlobalCleanup() {
    // Cleanup on visibility change (tab switch, minimize, etc.)
    this._visibilityHandler = () => {
      if (document.hidden && this.resizing) {
        this._handleResizeEnd();
      }
    };
    document.addEventListener("visibilitychange", this._visibilityHandler);

    // Cleanup on page unload
    this._unloadHandler = () => {
      if (this.resizing) {
        this._handleResizeEnd();
      }
    };
    window.addEventListener("beforeunload", this._unloadHandler);
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

    const uniformRowsLines =
      typeof this.options.uniformRowHeight === "number" && this.options.uniformRowHeight > 0
        ? this.options.uniformRowHeight
        : null;

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
    } ${uniformRowsLines ? "uniform-rows" : ""}" ${
      uniformRowsLines ? `style="--dt-row-lines: ${uniformRowsLines};"` : ""
    }>
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

    this.elements.wrapper = container.querySelector(".data-table-wrapper");
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

    this._updateLoadingClass();
  }

  /**
   * Render toolbar with search, filters, and buttons
   */
  _renderToolbar() {
    const { toolbar } = this.options;
    if (!toolbar || Object.keys(toolbar).length === 0) {
      this.toolbarView = null;
      return "";
    }

    if (!this.toolbarView) {
      this.toolbarView = new TableToolbarView(toolbar);
    } else {
      this.toolbarView.config = toolbar;
    }

    const toolbarState = {
      searchQuery: this.state.searchQuery,
      filters: this.state.filters,
      customControls: this.state.customControls,
    };

    return this.toolbarView.render(toolbarState);
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
          .join("")}
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
    if (this.state.isLoading && this.state.filteredData.length === 0) {
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

        // Apply dt-cell-clamp for uniformRowHeight, but NOT for no-wrap columns
        // no-wrap columns handle truncation via CSS (white-space: nowrap + text-overflow: ellipsis)
        const shouldClamp = this.options.uniformRowHeight && col.wrap !== false;
        const content = shouldClamp
          ? `<div class="dt-cell-clamp">${cellContent}</div>`
          : cellContent;

        return `
        <td data-column-id="${col.id}" 
            class="${cellClass} ${wrapClass}"
            data-row-id="${row[this.options.rowIdField] || ""}">
          ${content}
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
      } catch (_error) {
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
      } catch (_error) {
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

    // Remove resize handlers ONLY if no active resize operation
    // This prevents cursor getting stuck when table re-renders during column resize
    if (!this.resizing) {
      document.removeEventListener("mousemove", this._handleResize);
      document.removeEventListener("mouseup", this._handleResizeEnd);
    }

    if (this._paginationScrollRAF !== null) {
      cancelAnimationFrame(this._paginationScrollRAF);
      this._paginationScrollRAF = null;
    }
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

    const toolbarRoot = this.elements.toolbar;

    // Search input
    const searchInput = toolbarRoot?.querySelector(".dt-search-input");
    const searchClear = toolbarRoot?.querySelector(
      ".table-toolbar-search__clear"
    );
    if (searchInput) {
      const searchConfig = this.options.toolbar?.search || {};
      
      const handler = (e) => {
        this.state.searchQuery = e.target.value;
        if (searchClear) {
          searchClear.hidden = !(e.target.value || "").length;
        }
        
        // Call custom onChange if provided
        if (typeof searchConfig.onChange === "function") {
          searchConfig.onChange(e.target.value, searchInput);
        }
        
        // Only apply filters if not using custom onChange (server-side search)
        if (!searchConfig.onChange) {
          this._applyFilters();
        }
        this._saveState();
      };
      this._addEventListener(searchInput, "input", handler);

      const keyHandler = (e) => {
        if (e.key === "Escape" && searchInput.value) {
          searchInput.value = "";
          this.state.searchQuery = "";
          if (searchClear) {
            searchClear.hidden = true;
          }
          
          // Call custom onChange if provided
          if (typeof searchConfig.onChange === "function") {
            searchConfig.onChange("", searchInput);
          }
          
          // Only apply filters if not using custom onChange
          if (!searchConfig.onChange) {
            this._applyFilters();
          }
          this._saveState();
        } else if (e.key === "Enter") {
          // Call custom onSubmit if provided
          if (typeof searchConfig.onSubmit === "function") {
            searchConfig.onSubmit(searchInput.value, searchInput);
          }
        }
      };
      this._addEventListener(searchInput, "keydown", keyHandler);
    }

    if (searchClear) {
      const searchConfig = this.options.toolbar?.search || {};
      searchClear.hidden = !(this.state.searchQuery || "").length;
      const clearHandler = () => {
        if (searchInput) {
          searchInput.value = "";
          searchInput.focus();
        }
        this.state.searchQuery = "";
        searchClear.hidden = true;
        
        // Call custom onChange if provided
        if (typeof searchConfig.onChange === "function") {
          searchConfig.onChange("", searchInput);
        }
        
        // Only apply filters if not using custom onChange
        if (!searchConfig.onChange) {
          this._applyFilters();
        }
        this._saveState();
      };
      this._addEventListener(searchClear, "click", clearHandler);
    }

    // Filter dropdowns
    const filterSelects = toolbarRoot?.querySelectorAll(".dt-filter") || [];
    filterSelects.forEach((select) => {
      const filterId = select.dataset.filterId;
      if (!filterId) {
        return;
      }

      if (!(filterId in this.state.filters)) {
        this.state.filters[filterId] = select.value;
      }

      const handler = (e) => {
        const value = e.target.value;
        this.state.filters[filterId] = value;
        const filterConfig = this.options.toolbar.filters?.find(
          (filter) => filter.id === filterId
        );

        const autoApply = filterConfig?.autoApply !== false;
        if (autoApply) {
          this._applyFilters();
        }
        this._saveState();

        if (typeof filterConfig?.onChange === "function") {
          filterConfig.onChange(value, e.target);
        }
      };
      this._addEventListener(select, "change", handler);
    });

    // Custom controls (text inputs, etc.)
    const controlInputs =
      toolbarRoot?.querySelectorAll(
        ".table-toolbar-input[data-control-id]"
      ) || [];
    controlInputs.forEach((input) => {
      const controlId = input.dataset.controlId;
      if (!controlId) {
        return;
      }

      const controlConfig = this.options.toolbar.customControls?.find(
        (control) => control.id === controlId
      );

      if (!(controlId in this.state.customControls)) {
        this.state.customControls[controlId] = input.value ?? "";
      } else if (input.value !== this.state.customControls[controlId]) {
        input.value = this.state.customControls[controlId];
      }

      const updateClearButton = () => {
        const wrapper = input.closest("[data-control-wrapper]");
        if (!wrapper) {
          return;
        }
        const clearBtn = wrapper.querySelector(".table-toolbar-input__clear");
        if (clearBtn) {
          clearBtn.hidden = !(input.value || "").length;
        }
      };

      updateClearButton();

      const inputHandler = (e) => {
        const value = e.target.value;
        this.state.customControls[controlId] = value;
        updateClearButton();
        this._saveState();
        if (typeof controlConfig?.onChange === "function") {
          controlConfig.onChange(value, input);
        }
      };
      this._addEventListener(input, "input", inputHandler);

      const keyHandler = (e) => {
        if (e.key === "Enter") {
          if (typeof controlConfig?.onSubmit === "function") {
            controlConfig.onSubmit(input.value, input);
          }
        }
      };
      this._addEventListener(input, "keydown", keyHandler);

      const wrapper = input.closest("[data-control-wrapper]");
      if (wrapper) {
        const clearBtn = wrapper.querySelector(".table-toolbar-input__clear");
        if (clearBtn) {
          const clearHandler = () => {
            input.value = "";
            this.state.customControls[controlId] = "";
            updateClearButton();
            this._saveState();
            if (typeof controlConfig?.onChange === "function") {
              controlConfig.onChange("", input);
            }
            if (typeof controlConfig?.onClear === "function") {
              controlConfig.onClear(input);
            }
            input.focus();
          };
          this._addEventListener(clearBtn, "click", clearHandler);
        }
      }
    });

    // Toolbar buttons
    const buttons =
      toolbarRoot?.querySelectorAll(".table-toolbar-btn[data-btn-id]") || [];
    buttons.forEach((btn) => {
      const handler = () => {
        const btnId = btn.dataset.btnId;
        const btnConfig = this.options.toolbar.buttons?.find(
          (b) => b.id === btnId
        );
        if (typeof btnConfig?.onClick === "function") {
          btnConfig.onClick(btn, this);
        }
      };
      this._addEventListener(btn, "click", handler);
    });

    // Column visibility toggle
    const columnBtn = toolbarRoot?.querySelector(".dt-btn-columns");
    const columnMenu = toolbarRoot?.querySelector(".dt-column-menu");
    if (columnBtn && columnMenu) {
      let draggingMenuItem = null;
      let clearMenuDragHighlights = () => {};

      const buildColumnMenu = () => {
        const menuColumns = this._getOrderedColumns(true);
        columnMenu.innerHTML = menuColumns
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
          .join("");

        clearMenuDragHighlights = () => {
          columnMenu
            .querySelectorAll(".dt-column-menu-item.drag-over")
            .forEach((dragItem) => dragItem.classList.remove("drag-over"));
        };

        const checkboxes = columnMenu.querySelectorAll(
          'input[type="checkbox"]'
        );
        checkboxes.forEach((cb) => {
          const handler = (e) => {
            const columnId = e.target.dataset.columnId;
            this.state.visibleColumns[columnId] = e.target.checked;
            const shouldReopen = columnMenu.style.display === "block";
            this._pendingColumnMenuOpen = shouldReopen;
            this._saveState();
            this._renderTable();
          };
          this._addEventListener(cb, "change", handler);
        });

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
            } else if (item.nextElementSibling !== draggingMenuItem) {
              parent.insertBefore(draggingMenuItem, item.nextElementSibling);
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
      };

      const toggleColumnMenu = (shouldOpen) => {
        const isOpen = columnMenu.style.display === "block";
        const nextState =
          typeof shouldOpen === "boolean" ? shouldOpen : !isOpen;
        if (nextState) {
          buildColumnMenu();
          columnMenu.style.display = "block";
        } else {
          columnMenu.style.display = "none";
        }
      };

      const columnToggleHandler = (e) => {
        e.preventDefault();
        e.stopPropagation();
        toggleColumnMenu();
      };
      this._addEventListener(columnBtn, "click", columnToggleHandler);

      this.documentClickHandler = (event) => {
        if (
          columnMenu.style.display === "block" &&
          !columnMenu.contains(event.target) &&
          !columnBtn.contains(event.target)
        ) {
          toggleColumnMenu(false);
        }
      };
      document.addEventListener("click", this.documentClickHandler);

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
        toggleColumnMenu(true);
        this._pendingColumnMenuOpen = false;
      } else {
        buildColumnMenu();
      }
    }

    // Sortable headers
    const headers = this.elements.thead.querySelectorAll("th.sortable");
    headers.forEach((th) => {
      const handler = (e) => {
        if (e.target.classList.contains("dt-resize-handle")) return;

        const columnId = th.dataset.columnId;
        if (!columnId) {
          return;
        }

        if (this._getSortingMode() === "server") {
          const currentDirection = this.state.sortColumn === columnId ? this.state.sortDirection : null;
          const nextDirection = currentDirection === "asc" ? "desc" : "asc";
          this.state.sortColumn = columnId;
          this.state.sortDirection = nextDirection;
          this._saveState();
          this._renderTable();

          const sortingConfig = this.options.sorting || {};
          if (typeof sortingConfig.onChange === "function") {
            sortingConfig.onChange({
              column: columnId,
              direction: nextDirection,
              table: this,
            });
          }
          return;
        }

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

        // Remove any existing listeners before adding new ones to prevent duplicates
        document.removeEventListener("mousemove", this._handleResize);
        document.removeEventListener("mouseup", this._handleResizeEnd);
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
      if (!this.elements.scrollContainer) {
        return;
      }
      this.state.scrollPosition = this.elements.scrollContainer.scrollTop;
      this._handlePaginationScroll();

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

    if (this.options.lockColumnWidths && this.state.columnWidthsLocked) {
      const needsSizing = visibleColumns.some(
        (col) => typeof this.state.columnWidths[col.id] !== "number"
      );
      if (!needsSizing) {
        return;
      }
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
      let finalWidth = Math.max(minWidth, maxWidth + padding);
      const previous = this.state.columnWidths[columnId];

      if (Number.isFinite(previous)) {
        // Prevent repeated auto-sizing from inflating widths when content width is unchanged
        const growthThreshold = 1;
        const hasContentGrowth = maxWidth > previous + growthThreshold;

        if (!hasContentGrowth) {
          finalWidth = previous;
        } else if (finalWidth < previous) {
          finalWidth = previous;
        }
      }

      if (!Number.isFinite(finalWidth)) {
        return;
      }

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
      // Don't reset hasAutoFitted here - content-based sizing shouldn't trigger container fit
    }

    if (this.options.lockColumnWidths) {
      const allSized = visibleColumns.every(
        (col) => typeof this.state.columnWidths[col.id] === "number"
      );
      if (allSized) {
        this.state.columnWidthsLocked = true;
      }
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
          // Skip user-resized columns - preserve their width
          if (this.state.userResizedColumns?.[col.id]) {
            runningTotal += currentWidth;
            return;
          }

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

    // If under target, expand last non-user-resized column to fill remaining gap for exact fit
    if (totalWidth < targetWidth) {
      // Choose the last visible column that wasn't manually resized to absorb the gap
      let lastCol = null;
      for (let i = visibleColumns.length - 1; i >= 0; i--) {
        if (!this.state.userResizedColumns?.[visibleColumns[i].id]) {
          lastCol = visibleColumns[i];
          break;
        }
      }
      
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
  _applyFilters(options = {}) {
    let data = [...this.state.data];

    // Apply search (client-side only)
    if (this._getSearchMode() !== "server" && this.state.searchQuery) {
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
        if (
          filterValue &&
          filterValue !== "all" &&
          filter.filterFn &&
          !this._isServerFilter(filter)
        ) {
          data = data.filter((row) => filter.filterFn(row, filterValue));
        }
      });
    }

    this.state.filteredData = data;
    this._applySort();
    this._renderTable(options.renderOptions || {});
  }

  /**
   * Apply sorting
   */
  _applySort() {
    if (this._getSortingMode() === "server") {
      return;
    }
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
  _renderTable(renderOptions = {}) {
    const scrollContainer = this.elements.scrollContainer;
    const prevScrollTop =
      typeof renderOptions.prevScrollTop === "number"
        ? renderOptions.prevScrollTop
        : scrollContainer
        ? scrollContainer.scrollTop
        : 0;
    const prevScrollLeft =
      typeof renderOptions.prevScrollLeft === "number"
        ? renderOptions.prevScrollLeft
        : scrollContainer
        ? scrollContainer.scrollLeft
        : 0;
    const prevScrollHeight =
      typeof renderOptions.prevScrollHeight === "number"
        ? renderOptions.prevScrollHeight
        : scrollContainer
        ? scrollContainer.scrollHeight
        : 0;

    if (this.options.lockColumnWidths) {
      const visibleColumns = this._getOrderedColumns();
      const needsSizing = visibleColumns.some(
        (col) => typeof this.state.columnWidths[col.id] !== "number"
      );
      if (needsSizing) {
        this.state.columnWidthsLocked = false;
      }
    }

    this._updateLoadingClass();

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

    if (scrollContainer) {
      const maxScrollTop = Math.max(
        0,
        scrollContainer.scrollHeight - scrollContainer.clientHeight
      );

      let targetTop;
      if (renderOptions.preserveTopDistance && prevScrollHeight > 0) {
        const baseline =
          typeof renderOptions.prevScrollTop === "number"
            ? renderOptions.prevScrollTop
            : prevScrollTop;
        const delta = scrollContainer.scrollHeight - prevScrollHeight;
        targetTop = Math.max(0, baseline + delta);
      } else if (typeof renderOptions.targetScrollTop === "number") {
        targetTop = Math.max(0, renderOptions.targetScrollTop);
      } else if (renderOptions.resetScroll === true) {
        targetTop = 0;
      } else {
        targetTop = Math.min(prevScrollTop, maxScrollTop);
      }

      scrollContainer.scrollTop = targetTop;
      scrollContainer.scrollLeft = prevScrollLeft;
      this.state.scrollPosition = targetTop;

      if (this._pagination?.enabled) {
        this._autoLoadIfViewportShort();
      }
    }
  }

  _handlePaginationScroll() {
    if (!this._pagination?.enabled || !this.elements.scrollContainer) {
      return;
    }

    if (this._paginationScrollRAF !== null) {
      return;
    }

    this._paginationScrollRAF = requestAnimationFrame(() => {
      this._paginationScrollRAF = null;
      this._maybeTriggerPaginationLoad();
    });
  }

  _maybeTriggerPaginationLoad() {
    const pagination = this._pagination;
    const container = this.elements.scrollContainer;
    if (!pagination?.enabled || !container) {
      return;
    }

    const distanceToBottom =
      container.scrollHeight - (container.scrollTop + container.clientHeight);

    if (
      pagination.hasMoreNext !== false &&
      !pagination.loadingNext &&
      distanceToBottom <= pagination.threshold
    ) {
      this.loadNext({ reason: "scroll", silent: true });
    }

    if (
      pagination.hasMorePrev !== false &&
      !pagination.loadingPrev &&
      container.scrollTop <= pagination.threshold
    ) {
      this.loadPrevious({ reason: "scroll", silent: true });
    }
  }

  _autoLoadIfViewportShort() {
    const pagination = this._pagination;
    const container = this.elements.scrollContainer;
    if (!pagination?.enabled || !container) {
      return;
    }

    if (
      pagination.hasMoreNext !== false &&
      !pagination.loadingNext &&
      container.scrollHeight <= container.clientHeight + pagination.threshold
    ) {
      this.loadNext({ reason: "auto-fill", silent: true });
    }
  }

  _loadPage(direction, options = {}) {
    if (!this._pagination?.enabled) {
      return Promise.resolve(null);
    }

    const pagination = this._pagination;
    const normalizedDirection =
      direction === "prev" || direction === "next" ? direction : "initial";
    const reason = options.reason ?? "scroll";
    const silent = options.silent ?? false;
    const force = options.force ?? false;

    if (normalizedDirection === "next") {
      if (pagination.loadingNext && !force) {
        return pagination.pendingRequest ?? Promise.resolve(null);
      }
      if (pagination.hasMoreNext === false && !force) {
        return Promise.resolve(null);
      }
    } else if (normalizedDirection === "prev") {
      if (pagination.loadingPrev && !force) {
        return pagination.pendingRequest ?? Promise.resolve(null);
      }
      if (pagination.hasMorePrev === false && !force) {
        return Promise.resolve(null);
      }
    } else if (pagination.pendingRequest && !force) {
      return pagination.pendingRequest;
    }

    if (force) {
      this._cancelPaginationRequest();
    }

    const controller = this._createAbortController();
    pagination.abortController = controller;

    const cursor =
      options.cursor !== undefined
        ? options.cursor
        : normalizedDirection === "prev"
        ? pagination.cursorPrev ?? null
        : normalizedDirection === "next"
        ? pagination.cursorNext ?? null
        : pagination.cursorNext ?? null;

    const loadArgs = {
      direction: normalizedDirection,
      cursor,
      reason,
      context: pagination.context,
      signal: controller.signal,
      table: this,
    };

    if (normalizedDirection === "next") {
      pagination.loadingNext = true;
    } else if (normalizedDirection === "prev") {
      pagination.loadingPrev = true;
    } else {
      pagination.loadingInitial = true;
    }

    if (!silent && normalizedDirection === "initial") {
      this._setLoadingState(true);
    }

    let loadPromise;
    try {
      loadPromise = Promise.resolve(pagination.loadPage(loadArgs));
    } catch (error) {
      pagination.loadingNext = false;
      pagination.loadingPrev = false;
      pagination.loadingInitial = false;
      this._setLoadingState(false);
      this._log("error", "pagination.loadPage threw", error);
      return Promise.reject(error);
    }

    pagination.pendingRequest = loadPromise
      .then((result) => {
        this._applyPageResult(normalizedDirection, result, options);
        return result;
      })
      .catch((error) => {
        if (error?.name === "AbortError") {
          return null;
        }
        this._log(
          "error",
          `Pagination load failed (${normalizedDirection})`,
          error
        );
        throw error;
      })
      .finally(() => {
        if (pagination.abortController === controller) {
          pagination.abortController = null;
        }
        if (normalizedDirection === "next") {
          pagination.loadingNext = false;
        } else if (normalizedDirection === "prev") {
          pagination.loadingPrev = false;
        } else {
          pagination.loadingInitial = false;
        }
        pagination.pendingRequest = null;
        this._setLoadingState(false);
        this._notifyPaginationStateChange();
      });

    return pagination.pendingRequest;
  }

  _applyPageResult(direction, result, options = {}) {
    const normalized = this._normalizePageResult(result);

    const meta = {
      cursorNext: normalized.cursorNext,
      cursorPrev: normalized.cursorPrev,
      hasMoreNext: normalized.hasMoreNext,
      hasMorePrev: normalized.hasMorePrev,
      total: normalized.total,
      meta: normalized.meta,
      renderOptions: normalized.renderOptions,
      resetScroll: normalized.resetScroll,
      preserveScroll: normalized.preserveScroll,
    };

    const mode = normalized.mode?.toString().toLowerCase?.();
    const effectiveDirection =
      direction === "initial" && mode === "append"
        ? "next"
        : direction === "initial" && mode === "prepend"
        ? "prev"
        : direction;

    if (effectiveDirection === "prev" || mode === "prepend") {
      this._prependData(normalized.rows, {
        ...meta,
        preserveScroll:
          normalized.preserveScroll ?? options.preserveScroll ?? undefined,
      });
    } else if (effectiveDirection === "next" || mode === "append") {
      this._appendData(normalized.rows, meta);
    } else {
      const renderOptions = {
        ...(meta.renderOptions || {}),
      };
      if (normalized.resetScroll ?? options.resetScroll) {
        renderOptions.resetScroll = true;
      }
      this._replaceData(normalized.rows, {
        ...meta,
        renderOptions,
      });
    }

    if (this._pagination) {
      if (normalized.total !== undefined) {
        this._pagination.total = normalized.total;
      }
      if (normalized.meta !== undefined) {
        this._pagination.meta = normalized.meta;
      }
    }

    if (typeof this._pagination?.onPageLoaded === "function") {
      try {
        this._pagination.onPageLoaded({
          direction: effectiveDirection,
          rows: normalized.rows,
          raw: result,
          meta: normalized.meta,
          cursorNext: this._pagination?.cursorNext ?? null,
          cursorPrev: this._pagination?.cursorPrev ?? null,
          total: normalized.total,
          reason: options.reason ?? "scroll",
          table: this,
        });
      } catch (error) {
        this._log("error", "pagination.onPageLoaded failed", error);
      }
    }
  }

  _normalizePageResult(result) {
    if (Array.isArray(result)) {
      return {
        rows: result,
        cursorNext: undefined,
        cursorPrev: undefined,
        hasMoreNext: undefined,
        hasMorePrev: undefined,
        total: undefined,
        meta: undefined,
        mode: undefined,
        renderOptions: undefined,
        resetScroll: undefined,
        preserveScroll: undefined,
      };
    }

    if (!result || typeof result !== "object") {
      return {
        rows: [],
        cursorNext: undefined,
        cursorPrev: undefined,
        hasMoreNext: undefined,
        hasMorePrev: undefined,
        total: undefined,
        meta: undefined,
        mode: undefined,
        renderOptions: undefined,
        resetScroll: undefined,
        preserveScroll: undefined,
      };
    }

    const rows = Array.isArray(result.rows)
      ? result.rows
      : Array.isArray(result.items)
      ? result.items
      : Array.isArray(result.data)
      ? result.data
      : [];

    return {
      rows,
      cursorNext:
        result.cursorNext ??
        result.cursor_next ??
        result.next_cursor ??
        undefined,
      cursorPrev:
        result.cursorPrev ??
        result.cursor_prev ??
        result.prev_cursor ??
        result.previous_cursor ??
        undefined,
      hasMoreNext:
        result.hasMoreNext ??
        result.has_more_next ??
        undefined,
      hasMorePrev:
        result.hasMorePrev ??
        result.has_more_prev ??
        undefined,
      total: result.total ?? result.count ?? undefined,
      meta: result.meta ?? undefined,
      mode: result.mode ?? undefined,
      renderOptions: result.renderOptions ?? undefined,
      resetScroll: result.resetScroll ?? undefined,
      preserveScroll: result.preserveScroll ?? undefined,
    };
  }

  _replaceData(rows, meta = {}) {
    const sanitized = Array.isArray(rows)
      ? rows.filter((row) => row !== null && row !== undefined)
      : [];
    const isInitialLoad = this.state.data.length === 0;
    this.state.data = [...sanitized];
    // Only reset hasAutoFitted on initial load, not on data refreshes
    // This prevents fitToContainer from running on every poll cycle
    if (isInitialLoad) {
      this.state.hasAutoFitted = false;
    }
    this._updatePaginationMeta(meta, { replace: true });
    this._setLoadingState(false);

    const renderOptions = meta.renderOptions ? { ...meta.renderOptions } : {};
    if (meta.resetScroll) {
      renderOptions.resetScroll = true;
    }
    this._applyFilters({ renderOptions });
    this._log("info", "Data replaced", { rows: sanitized.length });
  }

  _appendData(rows, meta = {}) {
    const sanitized = Array.isArray(rows)
      ? rows.filter((row) => row !== null && row !== undefined)
      : [];
    if (sanitized.length === 0) {
      this._updatePaginationMeta(meta);
      this._setLoadingState(false);
      return;
    }

    const deduped = this._dedupeRows(sanitized, "append");
    if (deduped.length === 0) {
      this._updatePaginationMeta(meta);
      this._setLoadingState(false);
      return;
    }

    this.state.data = [...this.state.data, ...deduped];
    this._pruneRows("append");
    // Don't reset hasAutoFitted on append - preserve initial fit state
    this._updatePaginationMeta(meta);
    this._setLoadingState(false);
    this._applyFilters({ renderOptions: meta.renderOptions || {} });
    this._log("info", "Data appended", { rows: deduped.length });
  }

  _prependData(rows, meta = {}) {
    const sanitized = Array.isArray(rows)
      ? rows.filter((row) => row !== null && row !== undefined)
      : [];
    if (sanitized.length === 0) {
      this._updatePaginationMeta(meta);
      this._setLoadingState(false);
      return;
    }

    const deduped = this._dedupeRows(sanitized, "prepend");
    if (deduped.length === 0) {
      this._updatePaginationMeta(meta);
      this._setLoadingState(false);
      return;
    }

    let renderOptions = meta.renderOptions ? { ...meta.renderOptions } : {};
    const preserveScroll =
      meta.preserveScroll !== undefined
        ? meta.preserveScroll
        : this._pagination?.preserveScrollOnPrepend;

    if (preserveScroll && this.elements.scrollContainer) {
      renderOptions = {
        ...renderOptions,
        preserveTopDistance: true,
        prevScrollHeight: this.elements.scrollContainer.scrollHeight,
        prevScrollTop: this.elements.scrollContainer.scrollTop,
      };
    }

    this.state.data = [...deduped, ...this.state.data];
    this._pruneRows("prepend");
    // Don't reset hasAutoFitted on prepend - preserve initial fit state
    this._updatePaginationMeta(meta);
    this._setLoadingState(false);
    this._applyFilters({ renderOptions });
    this._log("info", "Data prepended", { rows: deduped.length });
  }

  _pruneRows(direction) {
    if (!this._pagination?.maxRows) {
      return;
    }

    const maxRows = this._pagination.maxRows;
    if (this.state.data.length <= maxRows) {
      return;
    }

    const excess = this.state.data.length - maxRows;
    if (excess <= 0) {
      return;
    }

    if (direction === "prepend") {
      this.state.data.splice(this.state.data.length - excess, excess);
    } else {
      this.state.data.splice(0, excess);
    }
  }

  _updatePaginationMeta(meta = {}, options = {}) {
    if (!this._pagination?.enabled) {
      return;
    }

    const pagination = this._pagination;

    if (Object.prototype.hasOwnProperty.call(meta, "cursorNext")) {
      pagination.cursorNext = meta.cursorNext ?? null;
      pagination.hasMoreNext =
        meta.hasMoreNext !== undefined
          ? Boolean(meta.hasMoreNext)
          : pagination.cursorNext !== null &&
            pagination.cursorNext !== undefined;
    } else if (Object.prototype.hasOwnProperty.call(meta, "hasMoreNext")) {
      pagination.hasMoreNext = Boolean(meta.hasMoreNext);
    } else if (options.replace) {
      pagination.cursorNext = pagination.cursorNext ?? null;
    }

    if (Object.prototype.hasOwnProperty.call(meta, "cursorPrev")) {
      pagination.cursorPrev = meta.cursorPrev ?? null;
      pagination.hasMorePrev =
        meta.hasMorePrev !== undefined
          ? Boolean(meta.hasMorePrev)
          : pagination.cursorPrev !== null &&
            pagination.cursorPrev !== undefined;
    } else if (Object.prototype.hasOwnProperty.call(meta, "hasMorePrev")) {
      pagination.hasMorePrev = Boolean(meta.hasMorePrev);
    } else if (options.replace) {
      pagination.cursorPrev = pagination.cursorPrev ?? null;
    }

    if (meta.total !== undefined) {
      pagination.total = meta.total;
    }
    if (meta.meta !== undefined) {
      pagination.meta = meta.meta;
    }
  }

  _notifyPaginationStateChange() {
    const pagination = this._pagination;
    if (!pagination?.enabled || typeof pagination.onStateChange !== "function") {
      return;
    }

    try {
      pagination.onStateChange(
        {
          cursorNext: pagination.cursorNext ?? null,
          cursorPrev: pagination.cursorPrev ?? null,
          hasMoreNext: pagination.hasMoreNext !== false,
          hasMorePrev: pagination.hasMorePrev !== false,
          loadingNext: Boolean(pagination.loadingNext),
          loadingPrev: Boolean(pagination.loadingPrev),
          total: pagination.total,
          meta: pagination.meta,
        },
        this
      );
    } catch (error) {
      this._log("error", "pagination.onStateChange failed", error);
    }
  }

  _getRowKey(row) {
    if (!row || typeof row !== "object") {
      return null;
    }

    if (this._pagination?.dedupeKey) {
      try {
        const value = this._pagination.dedupeKey(row);
        if (value === undefined || value === null) {
          return null;
        }
        return String(value);
      } catch (error) {
        this._log("error", "pagination.dedupeKey failed", error);
        return null;
      }
    }

    const field = this._pagination?.rowIdField || this.options.rowIdField;
    const key = row[field];
    if (key === undefined || key === null) {
      return null;
    }
    return String(key);
  }

  _dedupeRows(rows, direction) {
    if (!Array.isArray(rows) || rows.length === 0) {
      return [];
    }

    if (!this._pagination?.enabled || this._pagination.dedupe === false) {
      return rows.slice();
    }

    const seen = new Set(
      this.state.data
        .map((existing) => this._getRowKey(existing))
        .filter((key) => key !== null)
    );

    const result = [];
    for (const row of rows) {
      const key = this._getRowKey(row);
      if (!key) {
        result.push(row);
        continue;
      }

      if (seen.has(key)) {
        if (direction === "prepend") {
          break;
        }
        continue;
      }

      seen.add(key);
      result.push(row);
    }

    return result;
  }

  _createAbortController() {
    if (typeof AbortController !== "undefined") {
      return new AbortController();
    }
    return {
      abort() {},
      signal: undefined,
    };
  }

  _cancelPaginationRequest() {
    if (!this._pagination) {
      return;
    }

    if (this._pagination.abortController) {
      try {
        this._pagination.abortController.abort();
      } catch (error) {
        this._log("warn", "Pagination abort failed", error);
      }
      this._pagination.abortController = null;
    }

    this._pagination.pendingRequest = null;
    this._pagination.loadingNext = false;
    this._pagination.loadingPrev = false;
    this._pagination.loadingInitial = false;
  }

  reload(options = {}) {
    if (!this._pagination?.enabled) {
      return this.refresh(options);
    }

    const pagination = this._pagination;
    const reason = options.reason ?? "reload";
    const silent = options.silent ?? false;
    const preserveScroll = options.preserveScroll ?? false;

    this._cancelPaginationRequest();

    pagination.cursorNext =
      options.cursor ?? pagination.initialCursor ?? null;
    pagination.cursorPrev =
      options.prevCursor ?? pagination.initialPrevCursor ?? null;
    pagination.hasMoreNext =
      options.hasMoreNext !== undefined
        ? Boolean(options.hasMoreNext)
        : pagination.initialHasMoreNext !== undefined
        ? Boolean(pagination.initialHasMoreNext)
        : true;
    pagination.hasMorePrev =
      options.hasMorePrev !== undefined
        ? Boolean(options.hasMorePrev)
        : pagination.initialHasMorePrev !== undefined
        ? Boolean(pagination.initialHasMorePrev)
        : Boolean(pagination.cursorPrev);

    pagination.total = null;
    pagination.meta = {};
    pagination.loadingNext = false;
    pagination.loadingPrev = false;
    pagination.loadingInitial = false;

    if (!silent) {
      this._setLoadingState(true);
    }

    pagination.loadingInitial = true;

    return this._loadPage("initial", {
      reason,
      silent,
      preserveScroll,
      cursor: pagination.cursorNext ?? null,
      replace: true,
      resetScroll: options.resetScroll ?? false,
    }).finally(() => {
      pagination.loadingInitial = false;
      if (!silent) {
        this._setLoadingState(false);
      }
    });
  }

  loadNext(options = {}) {
    return this._loadPage("next", options);
  }

  loadPrevious(options = {}) {
    return this._loadPage("prev", options);
  }

  setPaginationContext(context = {}, options = {}) {
    if (!this._pagination?.enabled) {
      return Promise.resolve();
    }

    this._pagination.context = { ...context };
    this._notifyPaginationStateChange();

    if (options.reload) {
      return this.reload({
        reason: options.reason ?? "context-update",
        silent: options.silent ?? false,
        preserveScroll: options.preserveScroll ?? false,
        resetScroll: options.resetScroll ?? false,
      });
    }

    return Promise.resolve();
  }

  mergePaginationContext(partial = {}, options = {}) {
    if (!this._pagination?.enabled) {
      return Promise.resolve();
    }

    this._pagination.context = {
      ...this._pagination.context,
      ...partial,
    };
    this._notifyPaginationStateChange();

    if (options.reload) {
      return this.reload({
        reason: options.reason ?? "context-update",
        silent: options.silent ?? false,
        preserveScroll: options.preserveScroll ?? false,
        resetScroll: options.resetScroll ?? false,
      });
    }

    return Promise.resolve();
  }

  setSortState(columnId, direction = "asc", options = {}) {
    const nextColumn = columnId ?? null;
    const nextDirection = direction === "desc" ? "desc" : "asc";
    this.state.sortColumn = nextColumn;
    this.state.sortDirection = nextDirection;
    this._saveState();
    if (options.render !== false) {
      this._renderTable(options.renderOptions || {});
    }
  }

  getPaginationState() {
    if (!this._pagination?.enabled) {
      return null;
    }

    return {
      cursorNext: this._pagination.cursorNext ?? null,
      cursorPrev: this._pagination.cursorPrev ?? null,
      hasMoreNext: this._pagination.hasMoreNext !== false,
      hasMorePrev: this._pagination.hasMorePrev !== false,
      loadingNext: Boolean(this._pagination.loadingNext),
      loadingPrev: Boolean(this._pagination.loadingPrev),
      total: this._pagination.total,
      meta: this._pagination.meta,
      context: { ...this._pagination.context },
    };
  }

  cancelPendingLoad() {
    this._cancelPaginationRequest();
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

  _updateLoadingClass() {
    const wrapper = this.elements.wrapper;
    if (!wrapper) {
      return;
    }

    if (this.state.isLoading && this.state.filteredData.length > 0) {
      wrapper.classList.add("is-refreshing");
    } else {
      wrapper.classList.remove("is-refreshing");
    }
  }

  _setLoadingState(value) {
    const normalized = Boolean(value);
    if (this.state.isLoading === normalized) {
      this._updateLoadingClass();
      return;
    }
    this.state.isLoading = normalized;
    this._updateLoadingClass();
  }

  /**
   * Load state from localStorage
   */
  _loadState() {
    const saved = AppState.load(this.options.stateKey);
    if (saved) {
      this.state = { ...this.state, ...saved };
      if (
        saved.customControls &&
        typeof saved.customControls === "object" &&
        !Array.isArray(saved.customControls)
      ) {
        this.state.customControls = { ...saved.customControls };
      } else {
        this.state.customControls = {};
      }
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
      customControls: this.state.customControls,
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
  setData(data, meta = {}) {
    if (Array.isArray(data)) {
      this._replaceData(data, meta);
      return;
    }

    if (data && typeof data === "object") {
      const payload = data;
      const candidateRows = Array.isArray(payload.rows)
        ? payload.rows
        : Array.isArray(payload.items)
        ? payload.items
        : Array.isArray(payload.data)
        ? payload.data
        : [];

      const combinedMeta = {
        cursorNext:
          payload.cursorNext ??
          payload.cursor_next ??
          payload.next_cursor ??
          meta.cursorNext,
        cursorPrev:
          payload.cursorPrev ??
          payload.cursor_prev ??
          payload.prev_cursor ??
          payload.previous_cursor ??
          meta.cursorPrev,
        hasMoreNext:
          payload.hasMoreNext ??
          payload.has_more_next ??
          meta.hasMoreNext,
        hasMorePrev:
          payload.hasMorePrev ??
          payload.has_more_prev ??
          meta.hasMorePrev,
        total: payload.total ?? payload.count ?? meta.total,
        meta: payload.meta ?? meta.meta,
        renderOptions: payload.renderOptions ?? meta.renderOptions,
        resetScroll: payload.resetScroll ?? meta.resetScroll,
        preserveScroll: payload.preserveScroll ?? meta.preserveScroll,
      };

      const modeValue = (payload.mode || meta.mode || "replace").toString();
      const mode = modeValue.toLowerCase();

      if (mode === "noop") {
        this._updatePaginationMeta(combinedMeta);
        this._setLoadingState(false);
        return;
      }

      if (mode === "append") {
        this._appendData(candidateRows, combinedMeta);
      } else if (mode === "prepend") {
        this._prependData(candidateRows, combinedMeta);
      } else {
        this._replaceData(candidateRows, combinedMeta);
      }
      return;
    }

    this._replaceData([], meta);
  }

  /**
   * Clear all table data
   */
  clearData() {
    this.state.data = [];
    this.state.filteredData = [];
    this.state.hasAutoFitted = false;
    this._setLoadingState(false);
    this._updatePaginationMeta(
      { cursorNext: null, cursorPrev: null, hasMoreNext: false, hasMorePrev: false },
      { replace: true }
    );
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
  refresh(request = {}) {
    const options =
      typeof request === "string"
        ? { reason: request }
        : request && typeof request === "object"
        ? { ...request }
        : {};

    if (this._pagination?.enabled) {
      return this.reload({
        reason: options.reason ?? "refresh",
        silent: options.silent ?? false,
        preserveScroll: options.preserveScroll ?? false,
        cursor: options.cursor,
        force: options.force ?? false,
      });
    }

    if (!this.options.onRefresh) {
      this._renderTable();
      return Promise.resolve();
    }

    const hadRows = Array.isArray(this.state.filteredData)
      ? this.state.filteredData.length > 0
      : false;

    this._setLoadingState(true);
    if (!hadRows) {
      this._renderTable();
    }

    const refreshPromise = Promise.resolve(
      this.options.onRefresh(options)
    )
      .then(() => {
        this._setLoadingState(false);
        if (!hadRows && this.state.filteredData.length === 0) {
          this._renderTable();
        }
      })
      .catch((err) => {
        this._setLoadingState(false);
        this._log("error", "Refresh failed", err);
        if (!hadRows) {
          this._renderTable();
        }
        throw err;
      });

    return refreshPromise;
  }

  updateToolbarSummary(summaryItems = []) {
    if (!this.elements.toolbar) {
      return;
    }
    TableToolbarView.updateSummary(this.elements.toolbar, summaryItems);
  }

  updateToolbarMeta(metaItems = []) {
    if (!this.elements.toolbar) {
      return;
    }
    TableToolbarView.updateMeta(this.elements.toolbar, metaItems);
  }

  setToolbarSearchValue(value, options = {}) {
    const nextValue = value ?? "";
    this.state.searchQuery = nextValue;
    if (this.elements.toolbar) {
      TableToolbarView.setSearchValue(this.elements.toolbar, nextValue);
    }
    if (options.apply !== false) {
      this._applyFilters();
    }
    this._saveState();
  }

  setToolbarFilterValue(filterId, value, options = {}) {
    if (!filterId) {
      return;
    }
    this.state.filters[filterId] = value;
    if (this.elements.toolbar) {
      TableToolbarView.setFilterValue(
        this.elements.toolbar,
        filterId,
        value
      );
    }
    if (options.apply !== false) {
      this._applyFilters();
    }
    this._saveState();
  }

  setToolbarCustomControlValue(controlId, value, options = {}) {
    if (!controlId) {
      return;
    }
    const nextValue = value ?? "";
    this.state.customControls[controlId] = nextValue;
    if (this.elements.toolbar) {
      TableToolbarView.setCustomControlValue(
        this.elements.toolbar,
        controlId,
        nextValue
      );
    }
    if (options.apply) {
      this._applyFilters();
    }
    this._saveState();
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

    this._cancelPaginationRequest();

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

    // Clean up global handlers
    if (this._visibilityHandler) {
      document.removeEventListener("visibilitychange", this._visibilityHandler);
      this._visibilityHandler = null;
    }
    if (this._unloadHandler) {
      window.removeEventListener("beforeunload", this._unloadHandler);
      this._unloadHandler = null;
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
