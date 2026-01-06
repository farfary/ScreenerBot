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
 * - State persistence via server-side storage
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
 * - maxWidth: Maximum width in px (optional, clamps auto + resize)
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

/* global queueMicrotask */

import * as AppState from "../core/app_state.js";
import { $ } from "../core/dom.js";
import { enhanceAllSelects } from "./custom_select.js";
import { TableToolbarView } from "./table_toolbar.js";
import { TableSettingsDialog } from "./table_settings_dialog.js";

const BLOCKING_STATE_VARIANTS = ["loading", "info", "warning", "error"];
const BLOCKING_STATE_DEFAULT_ICONS = {
  loading: "icon-loader",
  info: "icon-info",
  warning: "icon-triangle-alert",
  error: "icon-triangle-alert",
};

export class DataTable {
  constructor(options) {
    this.options = {
      container: options.container,
      columns: options.columns || [],
      data: options.data || [],
      toolbar: options.toolbar || {},
      sorting: options.sorting || { column: null, direction: "asc" },
      stateKey: options.stateKey || "data-table",
      serverStateKey: options.serverStateKey || null, // NEW: Separate key for server-side state
      restoreServerState: options.restoreServerState !== false, // NEW: Auto-restore server state
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
        typeof options.autoSizePadding === "number" && Number.isFinite(options.autoSizePadding)
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
      // Client pagination state
      clientPaginationState: {
        currentPage: 1,
        pageSize: options.clientPagination?.defaultPageSize ?? 50,
        totalPages: 1,
      },
      // Server pagination mode state (for hybrid pagination)
      serverPaginationState: {
        currentPage: 1,
        pageSize: options.pagination?.defaultPageSize ?? 50,
        totalPages: 1,
        totalItems: 0,
      },
    };

    // Client pagination configuration
    this.options.clientPagination = {
      enabled: options.clientPagination?.enabled ?? false,
      pageSizes: options.clientPagination?.pageSizes ?? [10, 20, 50, 100, "all"],
      defaultPageSize: options.clientPagination?.defaultPageSize ?? 50,
      stateKey: options.clientPagination?.stateKey ?? null,
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
    this._serverStateRestored = false; // Track if server state has been restored
    this._settingsDialog = null; // NEW: Table settings dialog instance
    this._blockingState = null; // Overlay for full-table loading/error states
    // Client pagination active state (can be toggled by user)
    this._clientPaginationActive = this.options.clientPagination?.enabled ?? false;

    // Hybrid pagination mode: 'scroll' (infinite scroll) or 'pages' (server-side page navigation)
    this._serverPaginationMode = "scroll";
    this._scrollLoadingDisabled = false;
    
    // Interaction tracking: skip re-renders while user is hovering/interacting
    this._isHovering = false;
    this._hoverTimeout = null;

    this._loadState();
    this._restoreServerState(); // NEW: Restore server-side state after loading
    this._loadServerPaginationMode(); // Load hybrid pagination mode preference
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

    // Hybrid pagination mode configuration
    const modes = Array.isArray(paginationOptions.modes) ? paginationOptions.modes : null;
    const defaultMode = paginationOptions.defaultMode || "scroll";
    const modeStateKey = paginationOptions.modeStateKey || null;
    const defaultPageSize = paginationOptions.defaultPageSize || 50;
    const pageSizes = paginationOptions.pageSizes || [10, 20, 50, 100];

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
        typeof paginationOptions.dedupeKey === "function" ? paginationOptions.dedupeKey : null,
      rowIdField: paginationOptions.rowIdField || this.options.rowIdField,
      preserveScrollOnAppend: paginationOptions.preserveScrollOnAppend !== false,
      preserveScrollOnPrepend: paginationOptions.preserveScrollOnPrepend !== false,
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
      // Hybrid mode config
      modes,
      defaultMode,
      modeStateKey,
      defaultPageSize,
      pageSizes,
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
          <div class="data-table-blocking-state" aria-live="polite" aria-hidden="true">
            <div class="data-table-blocking-state__inner">
              <i class="data-table-blocking-state__icon icon-loader" aria-hidden="true"></i>
              <div class="data-table-blocking-state__text">
                <div class="data-table-blocking-state__title"></div>
                <div class="data-table-blocking-state__description"></div>
              </div>
            </div>
          </div>
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
        ${this._renderClientPaginationBar()}
        ${this._renderServerPaginationBar()}
      </div>
    `;

    this.elements.wrapper = container.querySelector(".data-table-wrapper");
    this.elements.toolbar = container.querySelector(".data-table-toolbar");
    this.elements.scrollContainer = container.querySelector(".data-table-scroll-container");
    this.elements.table = container.querySelector(".data-table");
    this.elements.thead = container.querySelector("thead");
    this.elements.tbody = container.querySelector("tbody");
    this.elements.blockingState = container.querySelector(".data-table-blocking-state");
    this.elements.blockingStateIcon = container.querySelector(".data-table-blocking-state__icon");
    this.elements.blockingStateTitle = container.querySelector(".data-table-blocking-state__title");
    this.elements.blockingStateDescription = container.querySelector(
      ".data-table-blocking-state__description"
    );
    this.elements.clientPaginationBar = container.querySelector(".dt-client-pagination-bar");
    this.elements.serverPaginationBar = container.querySelector(".dt-server-pagination-bar");

    // Cache col elements for fast width updates
    this.elements.colgroup = container.querySelector("colgroup");
    this.elements.cols = {};
    if (this.elements.colgroup) {
      const cols = this.elements.colgroup.querySelectorAll("col[data-column-id]");
      cols.forEach((col) => {
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

    this._syncBlockingState();

    // Enhance all selects in the table (toolbar filters, pagination size)
    enhanceAllSelects(container);
  }

  _normalizeBlockingState(config = {}) {
    const variant = BLOCKING_STATE_VARIANTS.includes(config.variant) ? config.variant : "info";
    const title = typeof config.title === "string" ? config.title : "";
    const description = typeof config.description === "string" ? config.description : "";
    const iconClass =
      typeof config.icon === "string" && config.icon.trim().length > 0
        ? config.icon.trim()
        : BLOCKING_STATE_DEFAULT_ICONS[variant];
    return {
      variant,
      title,
      description,
      icon: iconClass,
    };
  }

  _syncBlockingState() {
    const container = this.elements.blockingState;
    if (!container) {
      return;
    }

    const iconEl = this.elements.blockingStateIcon;
    const titleEl = this.elements.blockingStateTitle;
    const descEl = this.elements.blockingStateDescription;

    const active = Boolean(this._blockingState);
    container.classList.toggle("is-visible", active);
    container.setAttribute("aria-hidden", active ? "false" : "true");

    container.classList.remove(
      "data-table-blocking-state--loading",
      "data-table-blocking-state--info",
      "data-table-blocking-state--warning",
      "data-table-blocking-state--error"
    );

    if (!active) {
      if (titleEl) titleEl.textContent = "";
      if (descEl) descEl.textContent = "";
      if (iconEl) iconEl.className = "data-table-blocking-state__icon";
      return;
    }

    const state = this._blockingState;
    container.classList.add(`data-table-blocking-state--${state.variant}`);
    if (iconEl) {
      iconEl.className = `data-table-blocking-state__icon ${state.icon}`.trim();
    }
    if (titleEl) {
      titleEl.textContent = state.title;
    }
    if (descEl) {
      descEl.textContent = state.description;
    }
  }

  showBlockingState(config = {}) {
    this._blockingState = this._normalizeBlockingState(config);
    this._syncBlockingState();
  }

  updateBlockingState(config = {}) {
    if (!this._blockingState) {
      this.showBlockingState(config);
      return;
    }
    const merged = {
      ...this._blockingState,
      ...config,
    };
    this._blockingState = this._normalizeBlockingState(merged);
    this._syncBlockingState();
  }

  hideBlockingState() {
    if (!this._blockingState) {
      return;
    }
    this._blockingState = null;
    this._syncBlockingState();
  }

  isBlockingStateVisible() {
    return Boolean(this._blockingState);
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

    let toolbarHtml = this.toolbarView.render(toolbarState);

    // Inject pagination mode toggle into toolbar if hybrid modes are enabled
    if (this._hasHybridPaginationModes()) {
      const modeToggleHtml = this._renderPaginationModeToggle();
      // Insert before the settings button (.dt-column-toggle) or at end of .table-toolbar-actions
      toolbarHtml = toolbarHtml.replace(/(<div class="dt-column-toggle)/, `${modeToggleHtml}$1`);
      // Fallback if no settings button: insert before closing of .table-toolbar-actions
      if (!toolbarHtml.includes("dt-pagination-mode-toggle")) {
        toolbarHtml = toolbarHtml.replace(
          /(<\/div>\s*<\/div>\s*<\/div>\s*$)/,
          `${modeToggleHtml}$1`
        );
      }
    }

    return toolbarHtml;
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

    // Add columns in the order specified by columnOrder
    for (const colId of this.state.columnOrder) {
      if (columnMap.has(colId)) {
        ordered.push(columnMap.get(colId));
        columnMap.delete(colId);
      }
    }

    // Add any remaining columns that weren't in columnOrder (new columns)
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
            const sortIcon = isSorted ? (this.state.sortDirection === "asc" ? "▲" : "▼") : "";

            return `
            <th 
              data-column-id="${col.id}"
              class="dt-header-column ${col.sortable ? "sortable" : ""} ${isSorted ? "sorted" : ""}"
            >
              <div class="dt-header-content">
                <span class="dt-header-label">
                  ${col.label}
                </span>
                ${col.sortable ? `<span class="dt-sort-icon">${sortIcon}</span>` : ""}
              </div>
              ${col.resizable !== false ? '<div class="dt-resize-handle"></div>' : ""}
            </th>
          `;
          })
          .join("")}
      </tr>
    `;
  }

  /**
   * Render empty state
   */
  _renderEmptyState() {
      const hasFilters = this.state.searchQuery || Object.keys(this.state.filters || {}).length > 0;
      const emptyIcon = hasFilters ? "icon-search" : "icon-inbox";
      const emptyTitle = hasFilters ? "No results found" : (this.options.emptyTitle || "No data");
      const emptyMessage = hasFilters
        ? "Try adjusting your search or filters"
        : (this.options.emptyMessage || "No data to display");

      return `
        <tr>
          <td colspan="100" class="dt-state-cell">
            <div class="dt-empty-state">
              <i class="dt-empty-icon ${emptyIcon}"></i>
              <div class="dt-empty-title">${emptyTitle}</div>
              <div class="dt-empty-message">${emptyMessage}</div>
            </div>
          </td>
        </tr>`;
  }

  /**
   * Render table body rows
   */
  _renderBody() {
    if (this.state.isLoading && this.state.filteredData.length === 0) {
      return `
        <tr>
          <td colspan="100" class="dt-state-cell">
            <div class="dt-loading-state">
              <div class="dt-loading-spinner"></div>
              <div class="dt-loading-text">${this.options.loadingMessage}</div>
            </div>
          </td>
        </tr>`;
    }

    // Use paginated data if client pagination is enabled
    const data = this._getClientPaginatedData();

    if (data.length === 0) {
      return this._renderEmptyState();
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
   * Render cell content (helper for _renderRow and _updateTableBody)
   */
  _renderCellContent(col, row) {
    let value = row[col.id];
    let cellContent = "";

    if (col.type === "actions" && col.actions) {
      cellContent = this._renderActionsCell(col, row);
    } else if (col.type === "image" && col.image) {
      cellContent = this._renderImageCell(col, row);
    } else if (col.render && typeof col.render === "function") {
      try {
        cellContent = col.render(value, row);
      } catch (error) {
        this._log("error", `Render function failed for column ${col.id}`, error);
        cellContent = `<span class="dt-render-error" title="${error.message}">Error</span>`;
      }
    } else {
      if (value === null || value === undefined) {
        cellContent = col.fallback || "—";
      } else {
        cellContent = value;
      }
    }
    return cellContent;
  }

  /**
   * Update table body in-place (diffing) to avoid full reload
   */
  _updateTableBody(newData) {
    const tbody = this.elements.tbody;
    
    if (newData.length === 0) {
       tbody.innerHTML = this._renderEmptyState();
       return;
    }
    
    // Check if currently showing empty state
    if (tbody.querySelector('.dt-empty-state')) {
       tbody.innerHTML = '';
    }

    const visibleColumns = this._getOrderedColumns();
    const rowIdField = this.options.rowIdField;
    
    // Index existing rows
    const existingRows = new Map();
    Array.from(tbody.children).forEach(tr => {
      const id = tr.getAttribute('data-row-id');
      if (id) existingRows.set(id, tr);
    });

    const fragment = document.createDocumentFragment();
    
    newData.forEach((row, index) => {
      const rowId = row[rowIdField] || index;
      const rowIdStr = String(rowId);
      let tr = existingRows.get(rowIdStr);
      
      if (tr) {
        // Update existing row
        existingRows.delete(rowIdStr);
        
        // Update selection state
        const isSelected = this.state.selectedRows.has(rowId);
        if (tr.classList.contains('selected') !== isSelected) {
           tr.classList.toggle('selected', isSelected);
        }

        // Update cells
        visibleColumns.forEach(col => {
           const td = tr.querySelector(`td[data-column-id="${col.id}"]`);
           if (td) {
             const cellContent = this._renderCellContent(col, row);
             
             // Handle wrapping wrapper
             const shouldClamp = this.options.uniformRowHeight && col.wrap !== false;
             
             let newContent = shouldClamp ? `<div class="dt-cell-clamp">${cellContent}</div>` : cellContent;
             
             if (td.innerHTML !== newContent) {
               td.innerHTML = newContent;
             }
             
             // Update data-row-id on TD just in case
             if (td.dataset.rowId !== rowIdStr) {
                td.dataset.rowId = rowIdStr;
             }
           }
        });
        
        fragment.appendChild(tr);
      } else {
        // Create new row
        const tr = document.createElement('tr');
        tr.setAttribute('data-row-id', rowIdStr);
        if (this.state.selectedRows.has(rowId)) {
          tr.classList.add('selected');
        }
        tr.innerHTML = this._renderRow(row);
        fragment.appendChild(tr);
      }
    });
    
    // Remove remaining rows
    existingRows.forEach(tr => tr.remove());
    
    // Append fragment (reorders existing rows and adds new ones)
    tbody.appendChild(fragment);
  }

  /**
   * Render individual row cells
   */
  _renderRow(row) {
    const visibleColumns = this._getOrderedColumns();

    return visibleColumns
      .map((col) => {
        const cellContent = this._renderCellContent(col, row);
        let cellClass = col.className || "";

        // Handle different column types
        if (col.type === "actions" && col.actions) {
          cellClass += " dt-actions-cell";
        } else if (col.type === "image" && col.image) {
          cellClass += " dt-image-cell";
        }

        // Add text wrapping class
        const wrapClass = col.wrap ? "wrap-text" : col.wrap === false ? "no-wrap" : "";

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
        this._log("error", `Image src function failed for column ${col.id}`, error);
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
    const fallback = config.fallback || '<i class="icon-image"></i>';
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
    const hasClickHandler = config.onClick && typeof config.onClick === "function";
    const clickClass = hasClickHandler ? "dt-image-clickable" : "";
    const clickAttr = hasClickHandler ? `data-image-click="${col.id}"` : "";

    return `
      <div class="dt-image-container ${clickClass}" ${clickAttr} ${title ? `title="${title}"` : ""}>
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
      const icon = config.icon || '<i class="icon-ellipsis-vertical"></i>';
      const menuPosition = config.menuPosition || "right";
      const menuClass = menuPosition === "left" ? "menu-left" : "";

      return `
        <div class="dt-actions-container">
          <div class="dt-actions-dropdown" data-row-id="${row[this.options.rowIdField] || ""}">
            <button class="dt-actions-dropdown-trigger" data-action="dropdown-toggle" aria-haspopup="menu" aria-expanded="false">
              ${icon}
            </button>
            <div class="dt-actions-dropdown-menu ${menuClass}" role="menu" tabindex="-1" aria-hidden="true" style="display: none;">
              ${config.items
                .map((item) => {
                  if (item.type === "divider") {
                    return '<div class="dt-actions-dropdown-divider"></div>';
                  }

                  const itemVariant = item.variant === "danger" ? "danger" : "";
                  const disabled = item.disabled ? "disabled" : "";

                  return `
                  <div class="dt-actions-dropdown-item ${itemVariant} ${disabled}" 
                       data-action-id="${item.id}" role="menuitem" tabindex="-1" data-item-index>
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
              const iconOnly = !btn.label && btn.icon ? "dt-action-btn-icon-only" : "";
              const disabled = btn.disabled ? "disabled" : "";

              return `
              <button class="dt-action-btn ${variant} ${size} ${iconOnly} ${disabled}" 
                      data-action-id="${btn.id}"
                      ${btn.tooltip ? `title="${btn.tooltip}"` : ""}>
                ${btn.icon ? `<span class="dt-action-btn-icon">${btn.icon}</span>` : ""}
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

    // Always remove resize handlers to prevent accumulation
    // The handlers will be re-added in the resize start handler if needed
    document.removeEventListener("mousemove", this._handleResize);
    document.removeEventListener("mouseup", this._handleResizeEnd);

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
    const searchClear = toolbarRoot?.querySelector(".table-toolbar-search__clear");
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

      const isCheckbox = select.type === "checkbox";

      if (!(filterId in this.state.filters)) {
        this.state.filters[filterId] = isCheckbox ? select.checked : select.value;
      }

      const handler = (e) => {
        const input = e.target;
        const isSwitch = input.type === "checkbox";
        const value = isSwitch ? input.checked : input.value;
        this.state.filters[filterId] = value;
        if (isSwitch) {
          const status = input
            .closest(".table-toolbar-switch")
            ?.querySelector(".table-toolbar-switch__status");
          if (status) {
            const onLabel = status.dataset.onLabel || "On";
            const offLabel = status.dataset.offLabel || "All";
            status.textContent = value ? onLabel : offLabel;
          }
        }
        const filterConfig = this.options.toolbar.filters?.find((filter) => filter.id === filterId);

        const autoApply = filterConfig?.autoApply !== false;
        if (autoApply) {
          this._applyFilters();
        }
        this._saveState();

        if (typeof filterConfig?.onChange === "function") {
          filterConfig.onChange(value, input);
        }
      };
      this._addEventListener(select, "change", handler);
    });

    // Custom controls (text inputs, etc.)
    const controlInputs =
      toolbarRoot?.querySelectorAll(".table-toolbar-input[data-control-id]") || [];
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
    const buttons = toolbarRoot?.querySelectorAll(".table-toolbar-btn[data-btn-id]") || [];
    buttons.forEach((btn) => {
      const handler = () => {
        const btnId = btn.dataset.btnId;
        const btnConfig = this.options.toolbar.buttons?.find((b) => b.id === btnId);
        if (typeof btnConfig?.onClick === "function") {
          btnConfig.onClick(btn, this);
        }
      };
      this._addEventListener(btn, "click", handler);
    });

    // Column visibility toggle - NEW: Using settings dialog instead of dropdown
    const columnBtn = toolbarRoot?.querySelector(".dt-btn-columns");
    if (columnBtn) {
      const settingsHandler = () => {
        this._openSettingsDialog();
      };
      this._addEventListener(columnBtn, "click", settingsHandler);
    }

    // Sortable headers - use event delegation on thead to survive innerHTML updates
    // This is critical because _renderTable() replaces thead.innerHTML, destroying direct th handlers
    const sortHandler = (e) => {
      // Ignore clicks on resize handles
      if (e.target.classList.contains("dt-resize-handle")) {
        return;
      }

      // Find the closest sortable th element
      const th = e.target.closest("th.sortable[data-column-id]");
      if (!th) return;

      const columnId = th.dataset.columnId;
      if (!columnId) return;

      if (this._getSortingMode() === "server") {
        const currentDirection =
          this.state.sortColumn === columnId ? this.state.sortDirection : null;
        const nextDirection = currentDirection === "asc" ? "desc" : "asc";
        this.state.sortColumn = columnId;
        this.state.sortDirection = nextDirection;

        this._saveState();
        
        // For server-side sorting, only update header sort icons - don't re-render body
        // The body will be re-rendered when new data arrives from the server
        this._updateHeaderSortIndicators();

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
        this.state.sortDirection = this.state.sortDirection === "asc" ? "desc" : "asc";
      } else {
        this.state.sortColumn = columnId;
        this.state.sortDirection = "asc";
      }

      this._applySort();
      this._saveState();
      // Force render since this is a user-initiated action (even if hovering)
      this._renderTable({ force: true });
    };
    this._addEventListener(this.elements.thead, "click", sortHandler);

    // Column resizing - use event delegation on thead to survive innerHTML updates
    const resizeHandler = (e) => {
      // Only handle mousedown on resize handles
      if (!e.target.classList.contains("dt-resize-handle")) return;

      e.preventDefault();
      e.stopPropagation();

      const handle = e.target;
      const th = handle.closest("th[data-column-id]");
      if (!th) return;

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
    this._addEventListener(this.elements.thead, "mousedown", resizeHandler);

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
                this._log("error", `Image click handler failed for column ${columnId}`, error);
              }
            }
          }
        }
      };
      this._addEventListener(container, "click", handler);
    });

    // Action button click handlers
    const actionButtons = this.elements.tbody.querySelectorAll(".dt-action-btn[data-action-id]");
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
              const action = column.actions.buttons.find((a) => a.id === actionId);
              if (action?.onClick) {
                try {
                  action.onClick(row, e);
                } catch (error) {
                  this._log("error", `Action button handler failed for action ${actionId}`, error);
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
        const allMenus = this.elements.tbody.querySelectorAll(".dt-actions-dropdown-menu");
        allMenus.forEach((m) => {
          if (m !== menu) {
            m.style.display = "none";
            m.classList.remove("open");
            m.setAttribute("aria-hidden", "true");
            const t = m
              .closest(".dt-actions-dropdown")
              ?.querySelector(".dt-actions-dropdown-trigger");
            if (t) t.setAttribute("aria-expanded", "false");
          }
        });

        // Toggle current menu
        if (menu) {
          const isOpen = menu.classList.contains("open") || menu.style.display === "block";
          if (isOpen) {
            menu.style.display = "none";
            menu.classList.remove("open");
            menu.setAttribute("aria-hidden", "true");
            trigger.classList.remove("active");
            trigger.setAttribute("aria-expanded", "false");
          } else {
            menu.style.display = "block";
            menu.classList.add("open");
            menu.setAttribute("aria-hidden", "false");
            trigger.classList.add("active");
            trigger.setAttribute("aria-expanded", "true");

            // Focus first interactive item for keyboard users
            const items = Array.from(
              menu.querySelectorAll(".dt-actions-dropdown-item:not(.disabled)")
            );
            if (items.length) {
              items.forEach((it) => it.setAttribute("tabindex", "-1"));
              const first = items[0];
              first.setAttribute("tabindex", "0");
              first.focus();
            }
          }
        }
      };
      this._addEventListener(trigger, "click", handler);

      // Keyboard toggle and navigation from trigger
      const keyHandler = (e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          trigger.click();
          return;
        }
        if (e.key === "ArrowDown") {
          e.preventDefault();
          // open and focus first
          trigger.click();
          return;
        }
        if (e.key === "Escape") {
          const dropdown = trigger.closest(".dt-actions-dropdown");
          const menu = dropdown.querySelector(".dt-actions-dropdown-menu");
          if (menu && menu.classList.contains("open")) {
            menu.style.display = "none";
            menu.classList.remove("open");
            menu.setAttribute("aria-hidden", "true");
            trigger.classList.remove("active");
            trigger.setAttribute("aria-expanded", "false");
            trigger.focus();
          }
        }
      };
      this._addEventListener(trigger, "keydown", keyHandler);
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
              const action = column.actions.items.find((a) => a.id === actionId);
              if (action?.onClick) {
                try {
                  action.onClick(row, e);

                  // Close dropdown after action
                  const menu = dropdown.querySelector(".dt-actions-dropdown-menu");
                  const trigger = dropdown.querySelector(".dt-actions-dropdown-trigger");
                  if (menu) {
                    menu.style.display = "none";
                    menu.classList.remove("open");
                    menu.setAttribute("aria-hidden", "true");
                  }
                  if (trigger) {
                    trigger.classList.remove("active");
                    trigger.setAttribute("aria-expanded", "false");
                    trigger.focus();
                  }
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

      // Keyboard support for menu items
      const itemKeyHandler = (e) => {
        const menu = item.closest(".dt-actions-dropdown-menu");
        const items = Array.from(menu.querySelectorAll(".dt-actions-dropdown-item:not(.disabled)"));
        const idx = items.indexOf(item);
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          item.click();
          return;
        }
        if (e.key === "ArrowDown") {
          e.preventDefault();
          const next = items[(idx + 1) % items.length];
          if (next) {
            items.forEach((it) => it.setAttribute("tabindex", "-1"));
            next.setAttribute("tabindex", "0");
            next.focus();
          }
          return;
        }
        if (e.key === "ArrowUp") {
          e.preventDefault();
          const prev = items[(idx - 1 + items.length) % items.length];
          if (prev) {
            items.forEach((it) => it.setAttribute("tabindex", "-1"));
            prev.setAttribute("tabindex", "0");
            prev.focus();
          }
          return;
        }
        if (e.key === "Home") {
          e.preventDefault();
          const first = items[0];
          if (first) {
            items.forEach((it) => it.setAttribute("tabindex", "-1"));
            first.setAttribute("tabindex", "0");
            first.focus();
          }
          return;
        }
        if (e.key === "End") {
          e.preventDefault();
          const last = items[items.length - 1];
          if (last) {
            items.forEach((it) => it.setAttribute("tabindex", "-1"));
            last.setAttribute("tabindex", "0");
            last.focus();
          }
          return;
        }
        if (e.key === "Escape") {
          e.preventDefault();
          const dropdown = item.closest(".dt-actions-dropdown");
          const trigger = dropdown.querySelector(".dt-actions-dropdown-trigger");
          const menu = dropdown.querySelector(".dt-actions-dropdown-menu");
          if (menu) {
            menu.style.display = "none";
            menu.classList.remove("open");
            menu.setAttribute("aria-hidden", "true");
          }
          if (trigger) {
            trigger.classList.remove("active");
            trigger.setAttribute("aria-expanded", "false");
            trigger.focus();
          }
        }
      };
      this._addEventListener(item, "keydown", itemKeyHandler);
    });

    // Close dropdowns when clicking outside
    const closeDropdownsHandler = (e) => {
      if (!e.target.closest(".dt-actions-dropdown")) {
        const allMenus = this.elements.tbody.querySelectorAll(".dt-actions-dropdown-menu");
        const allTriggers = this.elements.tbody.querySelectorAll(".dt-actions-dropdown-trigger");
        allMenus.forEach((menu) => {
          menu.style.display = "none";
          menu.classList.remove("open");
          menu.setAttribute("aria-hidden", "true");
        });
        allTriggers.forEach((trigger) => {
          trigger.classList.remove("active");
          trigger.setAttribute("aria-expanded", "false");
        });
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
    
    // Hover tracking for render skipping during user interaction
    // This prevents table re-renders while user is hovering rows
    const mouseEnterHandler = () => {
      this._isHovering = true;
      if (this._hoverTimeout) {
        clearTimeout(this._hoverTimeout);
        this._hoverTimeout = null;
      }
    };
    const mouseLeaveHandler = () => {
      // Small delay before allowing re-renders to prevent flicker on quick mouse movements
      if (this._hoverTimeout) {
        clearTimeout(this._hoverTimeout);
      }
      this._hoverTimeout = setTimeout(() => {
        this._isHovering = false;
        this._hoverTimeout = null;
      }, 150);
    };
    this._addEventListener(this.elements.scrollContainer, "mouseenter", mouseEnterHandler);
    this._addEventListener(this.elements.scrollContainer, "mouseleave", mouseLeaveHandler);

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
    this._addEventListener(this.elements.scrollContainer, "scroll", scrollHandler);

    // Attach client pagination events
    this._attachClientPaginationEvents();

    // Attach hybrid pagination mode toggle and server pagination events
    this._attachModeToggleEvents();
    this._attachServerPaginationEvents();
  }

  /**
   * Persist column order changes from the column menu
   * @param {HTMLElement} columnMenu
   */
  _updateColumnOrderFromMenu(columnMenu) {
    if (!columnMenu) {
      return;
    }

    const orderedIds = Array.from(columnMenu.querySelectorAll(".dt-column-menu-item"))
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

  _arraysEqual(a, b) {
    const arrA = Array.isArray(a) ? a : [];
    const arrB = Array.isArray(b) ? b : [];
    if (arrA.length !== arrB.length) {
      return false;
    }
    return arrA.every((value, index) => value === arrB[index]);
  }

  _getColumnConfig(columnId) {
    return this.options.columns.find((col) => col.id === columnId);
  }

  _getColumnMinWidth(columnId) {
    const column = this._getColumnConfig(columnId);
    if (!column) {
      return 80;
    }
    if (typeof column.minWidth === "number" && column.minWidth >= 0) {
      return column.minWidth;
    }
    return 80;
  }

  _getColumnMaxWidth(columnId) {
    const column = this._getColumnConfig(columnId);
    if (!column) {
      return Number.POSITIVE_INFINITY;
    }
    if (typeof column.maxWidth === "number" && column.maxWidth > 0) {
      return column.maxWidth;
    }
    return Number.POSITIVE_INFINITY;
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

    const minWidth = this._getColumnMinWidth(columnId);
    const maxWidth = this._getColumnMaxWidth(columnId);
    const w = Math.min(maxWidth, Math.max(minWidth, Math.round(widthPx)));

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

    // Skip if columns are locked and all have sizes
    if (this.options.lockColumnWidths && this.state.columnWidthsLocked) {
      const needsSizing = visibleColumns.some(
        (col) => typeof this.state.columnWidths[col.id] !== "number"
      );
      if (!needsSizing) {
        return;
      }
    }

    // Skip content-based sizing if we've already auto-fitted and have stable widths
    // This prevents oscillation during rapid data updates
    if (this.state.hasAutoFitted && this._allColumnsHaveWidths()) {
      return;
    }

    const allRows = Array.from(this.elements.tbody.querySelectorAll("tr[data-row-id]"));
    const sampleSize = Math.min(this.options.autoSizeSample, allRows.length);
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
        !(typeof col.width === "string" && col.width.trim().toLowerCase() === "auto");

      if (hasFixedWidth) {
        const minWidth = this._getColumnMinWidth(columnId);
        const maxWidth = this._getColumnMaxWidth(columnId);
        const fixed = Math.min(maxWidth, Math.max(minWidth, Number(col.width)));
        if (
          typeof fixed === "number" &&
          !Number.isNaN(fixed) &&
          this.state.columnWidths[columnId] !== fixed
        ) {
          this.state.columnWidths[columnId] = fixed;
          this._applyColumnWidth(columnId, fixed);
          didChange = true;
        }
        return;
      }

      if (this.state.userResizedColumns?.[columnId]) {
        return;
      }

      const headerCell = this.elements.thead.querySelector(`th[data-column-id="${columnId}"]`);

      let maxWidth = headerCell ? Math.ceil(headerCell.scrollWidth) : 0;

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
      const maxWidthLimit = this._getColumnMaxWidth(columnId);
      let finalWidth = Math.max(minWidth, maxWidth + padding);
      const previous = this.state.columnWidths[columnId];

      if (Number.isFinite(previous)) {
        // Prevent oscillation: only allow width changes if content significantly changed
        // Use a higher threshold to prevent micro-adjustments from causing visual jitter
        const growthThreshold = 4;
        const shrinkThreshold = 8;

        // Calculate the difference between new measured width and stored width
        const widthDiff = finalWidth - previous;

        if (widthDiff > growthThreshold) {
          // Content grew significantly, allow increase
          // finalWidth stays as calculated
        } else if (widthDiff < -shrinkThreshold) {
          // Content shrank significantly, but only shrink if user hasn't interacted
          // Keep previous width to prevent shrinking on data updates
          finalWidth = previous;
        } else {
          // Within threshold, keep stable
          finalWidth = previous;
        }
      }

      if (!Number.isFinite(finalWidth)) {
        return;
      }

      finalWidth = Math.min(maxWidthLimit, finalWidth);

      if (!Number.isFinite(previous) || Math.abs(previous - finalWidth) > 1) {
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
    // Note: fitToContainer is now called separately in _renderTable to avoid double-fitting
  }

  // Snapshot current natural widths for visible columns into state if missing
  // This function ONLY captures widths - fitting is done separately
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

    // Update table width sum (fitting and DOM application done by caller)
    const sum = this._computeTableWidthFromState();
    if (typeof sum === "number") {
      this.state.tableWidth = sum;
    }
  }

  // Check if all visible columns have stored widths
  _allColumnsHaveWidths() {
    const visibleColumns = this._getOrderedColumns();
    if (!visibleColumns || visibleColumns.length === 0) return false;
    return visibleColumns.every((col) => typeof this.state.columnWidths[col.id] === "number");
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
          const maxWidth = this._getColumnMaxWidth(col.id);
          // Round down to avoid overflow accumulation, we'll fix remainder on last column
          let scaled = Math.max(minWidth, Math.floor(currentWidth * scaleFactor));

          if (Number.isFinite(maxWidth)) {
            scaled = Math.min(maxWidth, scaled);
          }

          // On last column, absorb remainder so total matches targetWidth exactly (or as close as min allows)
          if (idx === lastIdx) {
            const remainder = targetWidth - runningTotal;
            // If remainder is less than minWidth, respect minWidth but it may still overflow in extreme cases
            const capped = Number.isFinite(maxWidth) ? Math.min(maxWidth, remainder) : remainder;
            scaled = Math.max(minWidth, capped);
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
          const minWidth = this._getColumnMinWidth(lastCol.id);
          const maxWidth = this._getColumnMaxWidth(lastCol.id);
          const unclamped = currentWidth + gap;
          const newWidth = Math.min(maxWidth, Math.max(minWidth, unclamped));
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

  // ==================== Client Pagination Methods ====================

  /**
   * Get the slice of data for the current page (client pagination)
   * @returns {Array} - Data for current page
   */
  _getClientPaginatedData() {
    const { enabled } = this.options.clientPagination;
    // Return all data if pagination not enabled or user disabled it
    if (!enabled || !this._clientPaginationActive) {
      return this.state.filteredData;
    }

    const { pageSize, currentPage } = this.state.clientPaginationState;

    // "all" means show all data
    if (pageSize === "all") {
      return this.state.filteredData;
    }

    const numericPageSize = parseInt(pageSize, 10);
    if (!Number.isFinite(numericPageSize) || numericPageSize <= 0) {
      return this.state.filteredData;
    }

    const startIndex = (currentPage - 1) * numericPageSize;
    const endIndex = startIndex + numericPageSize;

    return this.state.filteredData.slice(startIndex, endIndex);
  }

  /**
   * Recalculate total pages based on filtered data and page size
   */
  _recalculateClientPaginationPages() {
    if (!this.options.clientPagination?.enabled) {
      return;
    }

    const { pageSize } = this.state.clientPaginationState;
    const totalItems = this.state.filteredData.length;

    if (pageSize === "all" || totalItems === 0) {
      this.state.clientPaginationState.totalPages = 1;
      this.state.clientPaginationState.currentPage = 1;
      return;
    }

    const numericPageSize = parseInt(pageSize, 10);
    if (!Number.isFinite(numericPageSize) || numericPageSize <= 0) {
      this.state.clientPaginationState.totalPages = 1;
      this.state.clientPaginationState.currentPage = 1;
      return;
    }

    const totalPages = Math.ceil(totalItems / numericPageSize);
    this.state.clientPaginationState.totalPages = Math.max(1, totalPages);

    // Ensure current page is within bounds
    if (
      this.state.clientPaginationState.currentPage > this.state.clientPaginationState.totalPages
    ) {
      this.state.clientPaginationState.currentPage = this.state.clientPaginationState.totalPages;
    }
    if (this.state.clientPaginationState.currentPage < 1) {
      this.state.clientPaginationState.currentPage = 1;
    }
  }

  /**
   * Navigate to a specific page
   * @param {number|string} page - Page number or 'first', 'last', 'prev', 'next'
   */
  _goToClientPage(page) {
    if (!this.options.clientPagination?.enabled) {
      return;
    }

    const { totalPages, currentPage } = this.state.clientPaginationState;
    let targetPage = currentPage;

    if (page === "first") {
      targetPage = 1;
    } else if (page === "last") {
      targetPage = totalPages;
    } else if (page === "prev") {
      targetPage = Math.max(1, currentPage - 1);
    } else if (page === "next") {
      targetPage = Math.min(totalPages, currentPage + 1);
    } else {
      const numericPage = parseInt(page, 10);
      if (Number.isFinite(numericPage)) {
        targetPage = Math.max(1, Math.min(totalPages, numericPage));
      }
    }

    if (targetPage === currentPage) {
      return;
    }

    // Cancel pending page change to prevent rapid-click race condition
    if (this._pendingPageChange) {
      cancelAnimationFrame(this._pendingPageChange);
    }

    this.state.clientPaginationState.currentPage = targetPage;

    // Use requestAnimationFrame to debounce rapid clicks
    this._pendingPageChange = requestAnimationFrame(() => {
      this._pendingPageChange = null;
      this._renderTable({ resetScroll: true });
      this._updateClientPaginationBar();
    });

    this._log("debug", "Client pagination: navigated to page", { page: targetPage, totalPages });
  }

  /**
   * Change items per page
   * @param {number|string} newSize - New page size or 'all'
   */
  _setClientPageSize(newSize) {
    if (!this.options.clientPagination?.enabled) {
      return;
    }

    const currentSize = this.state.clientPaginationState.pageSize;
    if (newSize === currentSize) {
      return;
    }

    this.state.clientPaginationState.pageSize = newSize;
    this.state.clientPaginationState.currentPage = 1;
    this._recalculateClientPaginationPages();
    this._renderTable({ resetScroll: true });
    this._updateClientPaginationBar();
    this._saveClientPaginationPreference();
    this._log("debug", "Client pagination: page size changed", { newSize });
  }

  /**
   * Save page size preference
   */
  _saveClientPaginationPreference() {
    const { stateKey } = this.options.clientPagination;
    if (!stateKey) {
      return;
    }

    AppState.save(stateKey, {
      pageSize: this.state.clientPaginationState.pageSize,
    });
  }

  /**
   * Load saved page size preference
   */
  _loadClientPaginationPreference() {
    const { stateKey, defaultPageSize } = this.options.clientPagination;
    if (!stateKey) {
      return;
    }

    const saved = AppState.load(stateKey);
    if (saved && saved.pageSize !== undefined) {
      this.state.clientPaginationState.pageSize = saved.pageSize;
    } else {
      this.state.clientPaginationState.pageSize = defaultPageSize;
    }

    // Also load the enabled preference
    this._clientPaginationActive = this._loadPaginationEnabledPreference();
  }

  /**
   * Load pagination enabled preference from AppState
   * @returns {boolean} - Whether pagination should be active
   */
  _loadPaginationEnabledPreference() {
    if (!this.options.clientPagination?.enabled) {
      return false;
    }

    const { stateKey } = this.options.clientPagination;
    if (!stateKey) {
      return true; // Default to enabled if no state key
    }

    const saved = AppState.load(stateKey + "_enabled");
    if (saved !== null && saved !== undefined) {
      return Boolean(saved);
    }
    return true; // Default to enabled
  }

  /**
   * Save pagination enabled preference to AppState
   * @param {boolean} enabled - Whether pagination is enabled
   */
  _savePaginationEnabledPreference(enabled) {
    const { stateKey } = this.options.clientPagination;
    if (!stateKey) {
      return;
    }
    AppState.save(stateKey + "_enabled", enabled);
  }

  /**
   * Handle pagination toggle from settings dialog
   * @param {boolean} enabled - Whether pagination should be enabled
   */
  _handlePaginationToggle(enabled) {
    if (this._clientPaginationActive === enabled) {
      return;
    }

    this._clientPaginationActive = enabled;
    this._savePaginationEnabledPreference(enabled);

    // Re-render table with/without pagination
    this._recalculateClientPaginationPages();
    this._renderTable({ resetScroll: true });
    this._updateClientPaginationBar();

    this._log("info", "Client pagination toggled", { enabled });
  }

  /**
   * Render the client pagination bar HTML
   * @returns {string} - HTML for pagination bar
   */
  _renderClientPaginationBar() {
    // Don't show pagination bar if not enabled or not active (user disabled)
    if (!this.options.clientPagination?.enabled || !this._clientPaginationActive) {
      return "";
    }

    const { currentPage, pageSize, totalPages } = this.state.clientPaginationState;
    const { pageSizes } = this.options.clientPagination;
    const totalItems = this.state.filteredData.length;

    // Ensure current pageSize is in the list
    const allPageSizes = [...pageSizes];
    const isAll = pageSize === "all";
    const numericSize = isAll ? null : parseInt(pageSize, 10);

    const hasSize = allPageSizes.some((s) => {
      if (s === "all") return isAll;
      return parseInt(s, 10) === numericSize;
    });

    if (!hasSize) {
      if (isAll) {
        allPageSizes.push("all");
      } else if (Number.isFinite(numericSize)) {
        allPageSizes.push(numericSize);
        // Sort numbers, keep "all" at end
        allPageSizes.sort((a, b) => {
          if (a === "all") return 1;
          if (b === "all") return -1;
          return a - b;
        });
      }
    }

    // Calculate range display
    let startItem = 0;
    let endItem = 0;

    if (totalItems > 0) {
      if (pageSize === "all") {
        startItem = 1;
        endItem = totalItems;
      } else {
        const numericPageSize = parseInt(pageSize, 10);
        startItem = (currentPage - 1) * numericPageSize + 1;
        endItem = Math.min(currentPage * numericPageSize, totalItems);
      }
    }

    // Generate page size options
    const pageSizeOptions = allPageSizes
      .map((size) => {
        const value = size === "all" ? "all" : size;
        const label = size === "all" ? "All" : size;
        const selected = String(pageSize) === String(value) ? "selected" : "";
        return `<option value="${value}" ${selected}>${label}</option>`;
      })
      .join("");

    // Generate page buttons
    const pageButtons = this._generateClientPaginationButtons(currentPage, totalPages);

    return `
      <div class="dt-client-pagination-bar">
        <div class="dt-client-pagination-info">
          <span class="dt-client-pagination-range">
            Showing <strong>${startItem}</strong>–<strong>${endItem}</strong> of <strong>${totalItems}</strong>
          </span>
        </div>
        
        <div class="dt-client-pagination-controls">
          <button class="dt-client-pagination-btn dt-client-pagination-first" 
                  data-page="first" 
                  ${currentPage <= 1 ? "disabled" : ""} 
                  title="First page">«</button>
          <button class="dt-client-pagination-btn dt-client-pagination-prev" 
                  data-page="prev" 
                  ${currentPage <= 1 ? "disabled" : ""} 
                  title="Previous page">‹</button>
          
          <div class="dt-client-pagination-pages">
            ${pageButtons}
          </div>
          
          <button class="dt-client-pagination-btn dt-client-pagination-next" 
                  data-page="next" 
                  ${currentPage >= totalPages ? "disabled" : ""} 
                  title="Next page">›</button>
          <button class="dt-client-pagination-btn dt-client-pagination-last" 
                  data-page="last" 
                  ${currentPage >= totalPages ? "disabled" : ""} 
                  title="Last page">»</button>
        </div>
        
        <div class="dt-client-pagination-size">
          <label class="dt-client-pagination-size__label">Per page:</label>
          <select class="dt-client-pagination-size__select" data-pagination-size data-custom-select>
            ${pageSizeOptions}
          </select>
        </div>
      </div>
    `;
  }

  /**
   * Generate smart page buttons with ellipsis
   * @param {number} current - Current page
   * @param {number} total - Total pages
   * @returns {string} - HTML for page buttons
   */
  _generateClientPaginationButtons(current, total) {
    if (total <= 1) {
      return '<button class="dt-client-pagination-btn dt-client-pagination-page active" data-page="1">1</button>';
    }

    const buttons = [];
    const maxVisible = 5;

    // Calculate which pages to show
    let startPage = Math.max(1, current - Math.floor(maxVisible / 2));
    let endPage = Math.min(total, startPage + maxVisible - 1);

    // Adjust if we're near the end
    if (endPage - startPage < maxVisible - 1) {
      startPage = Math.max(1, endPage - maxVisible + 1);
    }

    // Always show first page
    if (startPage > 1) {
      buttons.push(
        '<button class="dt-client-pagination-btn dt-client-pagination-page" data-page="1">1</button>'
      );
      if (startPage > 2) {
        buttons.push('<span class="dt-client-pagination-ellipsis">…</span>');
      }
    }

    // Show page range
    for (let i = startPage; i <= endPage; i++) {
      const activeClass = i === current ? "active" : "";
      buttons.push(
        `<button class="dt-client-pagination-btn dt-client-pagination-page ${activeClass}" data-page="${i}">${i}</button>`
      );
    }

    // Always show last page
    if (endPage < total) {
      if (endPage < total - 1) {
        buttons.push('<span class="dt-client-pagination-ellipsis">…</span>');
      }
      buttons.push(
        `<button class="dt-client-pagination-btn dt-client-pagination-page" data-page="${total}">${total}</button>`
      );
    }

    return buttons.join("");
  }

  /**
   * Attach pagination event handlers
   */
  _attachClientPaginationEvents() {
    if (!this.options.clientPagination?.enabled || !this.elements.clientPaginationBar) {
      return;
    }

    const bar = this.elements.clientPaginationBar;

    // Page navigation buttons
    const pageButtons = bar.querySelectorAll(".dt-client-pagination-btn[data-page]");
    pageButtons.forEach((btn) => {
      const handler = () => {
        if (btn.disabled) return;
        const page = btn.dataset.page;
        this._goToClientPage(page);
      };
      this._addEventListener(btn, "click", handler);
    });

    // Page size select
    const pageSizeSelect = bar.querySelector("[data-pagination-size]");
    if (pageSizeSelect) {
      const handler = (e) => {
        const value = e.target.value;
        this._setClientPageSize(value === "all" ? "all" : parseInt(value, 10));
      };
      this._addEventListener(pageSizeSelect, "change", handler);
    }
  }

  /**
   * Clean up pagination event handlers before DOM replacement
   */
  _cleanupClientPaginationEvents() {
    // Remove handlers for elements within the pagination bar
    const toRemove = [];
    this.eventHandlers.forEach((entry, key) => {
      if (entry.element?.closest?.(".dt-client-pagination-bar")) {
        entry.element.removeEventListener(entry.event, entry.handler);
        toRemove.push(key);
      }
    });
    toRemove.forEach((key) => this.eventHandlers.delete(key));
  }

  /**
   * Update the pagination bar (re-render just the bar)
   */
  _updateClientPaginationBar() {
    if (!this.options.clientPagination?.enabled || !this.elements.wrapper) {
      return;
    }

    // Clean up old event handlers before replacing DOM
    this._cleanupClientPaginationEvents();

    const existingBar = this.elements.clientPaginationBar;
    const newBarHtml = this._renderClientPaginationBar();

    if (existingBar) {
      existingBar.outerHTML = newBarHtml;
    } else {
      // Insert after scroll container
      const scrollContainer = this.elements.scrollContainer;
      if (scrollContainer) {
        scrollContainer.insertAdjacentHTML("afterend", newBarHtml);
      }
    }

    // Re-cache and re-attach events
    this.elements.clientPaginationBar = this.elements.wrapper.querySelector(
      ".dt-client-pagination-bar"
    );
    this._attachClientPaginationEvents();
  }

  // ==================== Hybrid Server Pagination Mode Methods ====================

  /**
   * Check if hybrid pagination mode toggle is enabled
   * @returns {boolean}
   */
  _hasHybridPaginationModes() {
    const modes = this._pagination?.modes;
    return Array.isArray(modes) && modes.length >= 2;
  }

  /**
   * Load server pagination mode preference from AppState
   */
  _loadServerPaginationMode() {
    if (!this._hasHybridPaginationModes()) {
      return;
    }

    const stateKey = this._pagination?.modeStateKey;
    if (stateKey) {
      const saved = AppState.load(stateKey);
      if (saved === "scroll" || saved === "pages") {
        this._serverPaginationMode = saved;
        this._scrollLoadingDisabled = saved === "pages";
        return;
      }
    }

    // Use default mode
    this._serverPaginationMode = this._pagination?.defaultMode || "scroll";
    this._scrollLoadingDisabled = this._serverPaginationMode === "pages";
  }

  /**
   * Save server pagination mode preference to AppState
   * @param {string} mode - 'scroll' or 'pages'
   */
  _saveServerPaginationMode(mode) {
    const stateKey = this._pagination?.modeStateKey;
    if (stateKey) {
      AppState.save(stateKey, mode);
    }
  }

  /**
   * Render the pagination mode toggle HTML for the toolbar
   * @returns {string} - HTML for the mode toggle
   */
  _renderPaginationModeToggle() {
    if (!this._hasHybridPaginationModes()) {
      return "";
    }

    const modes = this._pagination.modes;
    const scrollActive = this._serverPaginationMode === "scroll" ? "active" : "";
    const pagesActive = this._serverPaginationMode === "pages" ? "active" : "";

    const buttons = [];

    if (modes.includes("scroll")) {
      buttons.push(`
        <button type="button" class="dt-pagination-mode-btn ${scrollActive}" 
                data-mode="scroll" title="Infinite scroll mode">
          <i class="icon-arrow-down" aria-hidden="true"></i>
          <span>Scroll</span>
        </button>
      `);
    }

    if (modes.includes("pages")) {
      buttons.push(`
        <button type="button" class="dt-pagination-mode-btn ${pagesActive}" 
                data-mode="pages" title="Page navigation mode">
          <i class="icon-layout-grid" aria-hidden="true"></i>
          <span>Pages</span>
        </button>
      `);
    }

    return `
      <div class="dt-pagination-mode-toggle" role="group" aria-label="Pagination mode">
        ${buttons.join("")}
      </div>
    `;
  }

  /**
   * Attach event handlers for the pagination mode toggle
   */
  _attachModeToggleEvents() {
    const toggle = this.elements.toolbar?.querySelector(".dt-pagination-mode-toggle");
    if (!toggle) {
      return;
    }

    const buttons = toggle.querySelectorAll(".dt-pagination-mode-btn");
    buttons.forEach((btn) => {
      const handler = (e) => {
        const newMode = e.currentTarget.dataset.mode;
        if (newMode && newMode !== this._serverPaginationMode) {
          this._switchPaginationMode(newMode);
        }
      };
      this._addEventListener(btn, "click", handler);
    });
  }

  /**
   * Update the mode toggle button UI states
   */
  _updateModeToggleUI() {
    const toggle = this.elements.toolbar?.querySelector(".dt-pagination-mode-toggle");
    if (!toggle) {
      return;
    }

    const buttons = toggle.querySelectorAll(".dt-pagination-mode-btn");
    buttons.forEach((btn) => {
      const mode = btn.dataset.mode;
      btn.classList.toggle("active", mode === this._serverPaginationMode);
    });
  }

  /**
   * Switch between scroll and pages pagination modes
   * @param {string} newMode - 'scroll' or 'pages'
   */
  async _switchPaginationMode(newMode) {
    if (newMode === this._serverPaginationMode) {
      return;
    }

    // Cancel any pending pagination requests
    this._cancelPaginationRequest();

    this._serverPaginationMode = newMode;
    this._saveServerPaginationMode(newMode);

    // Update toggle button states
    this._updateModeToggleUI();

    // Immediately update pagination bar to reflect new mode (before async reload)
    // This ensures UI updates instantly rather than waiting for data fetch
    this._updateServerPaginationBar();

    if (newMode === "pages") {
      // Disable scroll loading, load first page
      this._scrollLoadingDisabled = true;
      await this._loadServerPage(1);
    } else {
      // Enable scroll loading, reload with cursor
      this._scrollLoadingDisabled = false;
      await this.reload({ reason: "mode-switch", resetScroll: true });
    }

    // Re-render footer/pagination bar after data loads (ensures correct state after reload)
    this._updateServerPaginationBar();

    this._log("info", "Pagination mode switched", { mode: newMode });
  }

  /**
   * Load a specific server page (for pages mode)
   * @param {number} pageNumber - Page number to load
   */
  async _loadServerPage(pageNumber, options = {}) {
    if (!this._pagination?.enabled) {
      return;
    }

    const pageSize = this.state.serverPaginationState.pageSize;
    const loadPage = this._pagination.loadPage;

    if (!loadPage) {
      return;
    }

    if (!options.silent) {
      this._setLoadingState(true);
    }

    try {
      const controller = this._createAbortController();
      this._pagination.abortController = controller;

      const result = await loadPage({
        direction: "page",
        page: pageNumber,
        pageSize: pageSize,
        reason: options.reason || "page-navigation",
        context: this._pagination.context,
        signal: controller.signal,
        table: this,
      });

      if (result) {
        const normalized = this._normalizePageResult(result);

        // In page mode, always replace data
        this.state.data = normalized.rows;
        this.state.filteredData = [...normalized.rows];

        // Update server page state from response
        // Always use the requested pageSize from state to prevent race conditions
        // where a stale response could overwrite user's page size choice
        if (normalized.serverPage) {
          this.state.serverPaginationState = {
            currentPage: normalized.serverPage.page || pageNumber,
            pageSize: pageSize, // Use requested pageSize, not from response
            totalPages: normalized.serverPage.totalPages || 1,
            totalItems: normalized.total || normalized.rows.length,
          };
        } else if (normalized.total !== undefined) {
          // Calculate from total
          const totalItems = normalized.total;
          this.state.serverPaginationState = {
            currentPage: pageNumber,
            pageSize: pageSize,
            totalPages: Math.ceil(totalItems / pageSize) || 1,
            totalItems: totalItems,
          };
        } else {
          // Fallback - use page number directly
          this.state.serverPaginationState = {
            ...this.state.serverPaginationState,
            currentPage: pageNumber,
          };
        }

        // Reset hasAutoFitted if this is initial load
        if (pageNumber === 1) {
          this.state.hasAutoFitted = false;
        }

        // Re-render table - force render since this is user-initiated (sort, page nav, etc.)
        this._renderTable({ resetScroll: true, force: true });
        this._updateServerPaginationBar();

        // Call onPageLoaded callback if defined
        if (typeof this._pagination?.onPageLoaded === "function") {
          try {
            this._pagination.onPageLoaded({
              direction: "page",
              rows: normalized.rows,
              raw: result,
              meta: normalized.meta,
              total: normalized.total ?? this.state.serverPaginationState.totalItems,
              reason: "page-navigation",
              page: pageNumber,
              pageSize: pageSize,
              table: this,
            });
          } catch (error) {
            this._log("error", "pagination.onPageLoaded failed in server page mode", error);
          }
        }
      }
    } catch (err) {
      if (err?.name !== "AbortError") {
        this._log("error", "Failed to load server page:", err);
      }
    } finally {
      this._setLoadingState(false);
      if (this._pagination) {
        this._pagination.abortController = null;
      }
    }
  }

  /**
   * Change page size for server pagination (pages mode)
   * @param {number} newSize - New page size
   */
  async _setServerPageSize(newSize) {
    const currentSize = this.state.serverPaginationState.pageSize;
    if (newSize === currentSize) {
      return;
    }

    // Cancel any pending requests to prevent race conditions with poller
    this._cancelPaginationRequest();

    this.state.serverPaginationState.pageSize = newSize;
    this.state.serverPaginationState.currentPage = 1;

    // Save state immediately
    this._saveState();

    // Optimistic update: update bar immediately to reflect selection
    this._updateServerPaginationBar();

    // Reload first page with new size
    await this._loadServerPage(1);
  }

  /**
   * Navigate to a specific server page
   * @param {number|string} page - Page number or 'first', 'last', 'prev', 'next'
   */
  async _goToServerPage(page) {
    const { totalPages, currentPage } = this.state.serverPaginationState;
    let targetPage = currentPage;

    if (page === "first" || page === 1) {
      targetPage = 1;
    } else if (page === "last") {
      targetPage = totalPages;
    } else if (page === "prev") {
      targetPage = Math.max(1, currentPage - 1);
    } else if (page === "next") {
      targetPage = Math.min(totalPages, currentPage + 1);
    } else {
      const numericPage = parseInt(page, 10);
      if (Number.isFinite(numericPage)) {
        targetPage = Math.max(1, Math.min(totalPages, numericPage));
      }
    }

    if (targetPage === currentPage) {
      return;
    }

    await this._loadServerPage(targetPage);
  }

  /**
   * Render the server pagination bar HTML (for pages mode)
   * @returns {string} - HTML for pagination bar
   */
  _renderServerPaginationBar() {
    if (!this._pagination?.enabled || this._serverPaginationMode !== "pages") {
      return "";
    }

    const { currentPage, pageSize, totalPages, totalItems } = this.state.serverPaginationState;
    const pageSizes = this._pagination.pageSizes || [10, 20, 50, 100];

    // Ensure current pageSize is in the list to avoid UI mismatch
    const allPageSizes = [...pageSizes];
    const numericSize = parseInt(pageSize, 10);

    const hasSize = allPageSizes.some((s) => parseInt(s, 10) === numericSize);

    if (!hasSize && Number.isFinite(numericSize)) {
      allPageSizes.push(numericSize);
      allPageSizes.sort((a, b) => a - b);
    }

    // Calculate range display
    let startItem = 0;
    let endItem = 0;

    if (totalItems > 0) {
      startItem = (currentPage - 1) * pageSize + 1;
      endItem = Math.min(currentPage * pageSize, totalItems);
    }

    // Generate page size options
    const pageSizeOptions = allPageSizes
      .map((size) => {
        const selected = parseInt(size, 10) === numericSize ? "selected" : "";
        return `<option value="${size}" ${selected}>${size}</option>`;
      })
      .join("");

    // Generate page buttons
    const pageButtons = this._generateServerPaginationButtons(currentPage, totalPages);

    return `
      <div class="dt-server-pagination-bar">
        <div class="dt-server-pagination-info">
          <span class="dt-server-pagination-range">
            Showing <strong>${startItem}</strong>–<strong>${endItem}</strong> of <strong>${totalItems}</strong>
          </span>
        </div>
        
        <div class="dt-server-pagination-controls">
          <button class="dt-server-pagination-btn dt-server-pagination-first" 
                  data-page="first" 
                  ${currentPage <= 1 ? "disabled" : ""} 
                  title="First page">«</button>
          <button class="dt-server-pagination-btn dt-server-pagination-prev" 
                  data-page="prev" 
                  ${currentPage <= 1 ? "disabled" : ""} 
                  title="Previous page">‹</button>
          
          <div class="dt-server-pagination-pages">
            ${pageButtons}
          </div>
          
          <button class="dt-server-pagination-btn dt-server-pagination-next" 
                  data-page="next" 
                  ${currentPage >= totalPages ? "disabled" : ""} 
                  title="Next page">›</button>
          <button class="dt-server-pagination-btn dt-server-pagination-last" 
                  data-page="last" 
                  ${currentPage >= totalPages ? "disabled" : ""} 
                  title="Last page">»</button>
        </div>
        
        <div class="dt-server-pagination-size">
          <label class="dt-server-pagination-size__label">Per page:</label>
          <select class="dt-server-pagination-size__select" data-server-pagination-size data-custom-select>
            ${pageSizeOptions}
          </select>
        </div>
      </div>
    `;
  }

  /**
   * Generate smart page buttons with ellipsis for server pagination
   * @param {number} current - Current page
   * @param {number} total - Total pages
   * @returns {string} - HTML for page buttons
   */
  _generateServerPaginationButtons(current, total) {
    if (total <= 1) {
      return '<button class="dt-server-pagination-btn dt-server-pagination-page active" data-page="1">1</button>';
    }

    const buttons = [];
    const maxVisible = 5;

    // Calculate which pages to show
    let startPage = Math.max(1, current - Math.floor(maxVisible / 2));
    let endPage = Math.min(total, startPage + maxVisible - 1);

    // Adjust if we're near the end
    if (endPage - startPage < maxVisible - 1) {
      startPage = Math.max(1, endPage - maxVisible + 1);
    }

    // Always show first page
    if (startPage > 1) {
      buttons.push(
        '<button class="dt-server-pagination-btn dt-server-pagination-page" data-page="1">1</button>'
      );
      if (startPage > 2) {
        buttons.push('<span class="dt-server-pagination-ellipsis">…</span>');
      }
    }

    // Show page range
    for (let i = startPage; i <= endPage; i++) {
      const activeClass = i === current ? "active" : "";
      buttons.push(
        `<button class="dt-server-pagination-btn dt-server-pagination-page ${activeClass}" data-page="${i}">${i}</button>`
      );
    }

    // Always show last page
    if (endPage < total) {
      if (endPage < total - 1) {
        buttons.push('<span class="dt-server-pagination-ellipsis">…</span>');
      }
      buttons.push(
        `<button class="dt-server-pagination-btn dt-server-pagination-page" data-page="${total}">${total}</button>`
      );
    }

    return buttons.join("");
  }

  /**
   * Attach event handlers for server pagination bar
   */
  _attachServerPaginationEvents() {
    if (!this._pagination?.enabled || !this.elements.serverPaginationBar) {
      return;
    }

    const bar = this.elements.serverPaginationBar;

    // Page navigation buttons
    const pageButtons = bar.querySelectorAll(".dt-server-pagination-btn[data-page]");
    pageButtons.forEach((btn) => {
      const handler = () => {
        if (btn.disabled) return;
        const page = btn.dataset.page;
        this._goToServerPage(page);
      };
      this._addEventListener(btn, "click", handler);
    });

    // Page size select
    const pageSizeSelect = bar.querySelector("[data-server-pagination-size]");
    if (pageSizeSelect) {
      const handler = (e) => {
        const value = parseInt(e.target.value, 10);
        if (Number.isFinite(value)) {
          this._setServerPageSize(value);
        }
      };
      this._addEventListener(pageSizeSelect, "change", handler);
    }
  }

  /**
   * Clean up server pagination event handlers before DOM replacement
   */
  _cleanupServerPaginationEvents() {
    const toRemove = [];
    this.eventHandlers.forEach((entry, key) => {
      if (entry.element?.closest?.(".dt-server-pagination-bar")) {
        entry.element.removeEventListener(entry.event, entry.handler);
        toRemove.push(key);
      }
    });
    toRemove.forEach((key) => this.eventHandlers.delete(key));
  }

  /**
   * Update the server pagination bar (re-render just the bar)
   */
  _updateServerPaginationBar() {
    if (!this._pagination?.enabled || !this.elements.wrapper) {
      return;
    }

    // Clean up old event handlers before replacing DOM
    this._cleanupServerPaginationEvents();

    // Always re-query DOM for the element (cache may be stale after table re-renders)
    const existingBar = this.elements.wrapper.querySelector(".dt-server-pagination-bar");
    const newBarHtml = this._renderServerPaginationBar();

    if (existingBar) {
      if (newBarHtml) {
        existingBar.outerHTML = newBarHtml;
      } else {
        existingBar.remove();
      }
    } else if (newBarHtml) {
      // Insert after scroll container (or after client pagination bar if it exists)
      const insertAfter = this.elements.clientPaginationBar || this.elements.scrollContainer;
      if (insertAfter) {
        insertAfter.insertAdjacentHTML("afterend", newBarHtml);
      }
    }

    // Re-cache and re-attach events
    this.elements.serverPaginationBar = this.elements.wrapper.querySelector(
      ".dt-server-pagination-bar"
    );
    this._attachServerPaginationEvents();
  }

  /**
   * Get current server pagination mode
   * @returns {string} - 'scroll' or 'pages'
   */
  getServerPaginationMode() {
    return this._serverPaginationMode;
  }

  /**
   * Set server pagination mode programmatically
   * @param {string} mode - 'scroll' or 'pages'
   */
  async setServerPaginationMode(mode) {
    if (mode !== "scroll" && mode !== "pages") {
      this._log("error", "Invalid pagination mode:", mode);
      return;
    }
    await this._switchPaginationMode(mode);
  }

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

    // Recalculate client pagination after filtering
    if (this.options.clientPagination?.enabled) {
      this.state.clientPaginationState.currentPage = 1; // Reset to page 1 on filter
      this._recalculateClientPaginationPages();
    }

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

    const column = this.options.columns.find((c) => c.id === this.state.sortColumn);
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
   * Check if user is currently interacting with table controls
   * Skip render during interaction to preserve UI state (dropdowns, focus, hover)
   */
  _isUserInteracting() {
    // Check explicit hover tracking (set by mouseenter/mouseleave events)
    if (this._isHovering) {
      return true;
    }
    
    const container = this.elements?.container;
    if (!container) return false;

    // Check if any select element inside table is focused or open
    const activeEl = document.activeElement;
    if (activeEl && container.contains(activeEl)) {
      const tag = activeEl.tagName?.toLowerCase();
      if (tag === "select" || tag === "input" || tag === "button") {
        return true;
      }
    }

    // Check for open dropdown menus (may be outside container, in document body)
    const openDropdown = document.querySelector(
      ".links-dropdown-menu, .dropdown-menu.open, [data-dropdown-open]"
    );
    if (openDropdown) {
      return true;
    }

    return false;
  }

  /**
   * Re-render table content only (not full structure)
   */
  _renderTable(renderOptions = {}) {
    // Skip render during user interaction (unless forced)
    if (!renderOptions.force && this._isUserInteracting()) {
      this._log("debug", "Skipping render during user interaction");
      return;
    }

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

    if (this.elements.thead) {
      this.elements.thead.innerHTML = this._renderHeader();
    }
    if (this.elements.tbody) {
      const visibleRows = this._getClientPaginatedData();
      this._updateTableBody(visibleRows);
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
        cols.forEach((col) => {
          const columnId = col.dataset.columnId;
          if (columnId) {
            this.elements.cols[columnId] = col;
          }
        });
      }
    }

    // Column width handling - coordinated to prevent oscillation
    // 1. Auto-size from content (only on initial load or when needed)
    this._autoSizeColumnsFromContent();
    // 2. Snapshot any missing widths from DOM
    this._snapshotColumnWidths();
    // 3. Apply stored widths to DOM
    this._applyStoredColumnWidths();
    // 4. Fit to container ONCE if not already done (prevents double-fitting)
    if (this.options.fitToContainer !== false && !this.state.hasAutoFitted) {
      this._fitColumnsToContainer();
      this.state.hasAutoFitted = true;
    }

    // NOTE: Do NOT call _attachEvents() here - event handlers use event delegation
    // on parent elements (thead, tbody, scrollContainer) which are NOT replaced,
    // only their innerHTML is. Re-attaching on every render would cause issues
    // with hover tracking (mouseenter fires spuriously when handlers are re-added
    // while mouse is already over the element).

    if (scrollContainer) {
      const maxScrollTop = Math.max(0, scrollContainer.scrollHeight - scrollContainer.clientHeight);

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

    // Update client pagination bar after table render
    if (this.options.clientPagination?.enabled) {
      this._updateClientPaginationBar();
    }

    // Update server pagination bar after table render (for pages mode)
    if (this._pagination?.enabled && this._hasHybridPaginationModes()) {
      this._updateServerPaginationBar();
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

    // Skip scroll-based loading when in pages mode
    if (this._scrollLoadingDisabled) {
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

    // Skip auto-loading when in pages mode
    if (this._scrollLoadingDisabled) {
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
          ? (pagination.cursorPrev ?? null)
          : normalizedDirection === "next"
            ? (pagination.cursorNext ?? null)
            : (pagination.cursorNext ?? null);

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
        this._log("error", `Pagination load failed (${normalizedDirection})`, error);
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

    // Always force-render pagination updates so new rows appear even while the
    // user is hovering/scrolling (interaction guard would otherwise skip renders)
    const metaWithForce = {
      ...meta,
      renderOptions: {
        ...(meta.renderOptions || {}),
        force: true,
      },
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
        ...metaWithForce,
        preserveScroll: normalized.preserveScroll ?? options.preserveScroll ?? undefined,
      });
    } else if (effectiveDirection === "next" || mode === "append") {
      this._appendData(normalized.rows, metaWithForce);
    } else {
      const renderOptions = {
        ...(metaWithForce.renderOptions || {}),
      };
      if (normalized.resetScroll ?? options.resetScroll) {
        renderOptions.resetScroll = true;
      }
      this._replaceData(normalized.rows, {
        ...metaWithForce,
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
        serverPage: undefined,
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
        serverPage: undefined,
      };
    }

    const rows = Array.isArray(result.rows)
      ? result.rows
      : Array.isArray(result.items)
        ? result.items
        : Array.isArray(result.data)
          ? result.data
          : [];

    // Handle serverPage for pages mode
    const serverPage = result.serverPage ?? result.server_page ?? result.pagination ?? undefined;

    return {
      rows,
      cursorNext: result.cursorNext ?? result.cursor_next ?? result.next_cursor ?? undefined,
      cursorPrev:
        result.cursorPrev ??
        result.cursor_prev ??
        result.prev_cursor ??
        result.previous_cursor ??
        undefined,
      hasMoreNext: result.hasMoreNext ?? result.has_more_next ?? undefined,
      hasMorePrev: result.hasMorePrev ?? result.has_more_prev ?? undefined,
      total: result.total ?? result.count ?? undefined,
      meta: result.meta ?? undefined,
      mode: result.mode ?? undefined,
      renderOptions: result.renderOptions ?? undefined,
      resetScroll: result.resetScroll ?? undefined,
      preserveScroll: result.preserveScroll ?? undefined,
      serverPage,
    };
  }

  /**
   * Check if new data is effectively the same as current data
   * Uses fast JSON comparison for row equality
   * @param {Array} newRows - New data rows
   * @returns {boolean} - True if data is unchanged
   */
  _isDataUnchanged(newRows) {
    const currentData = this.state.data;
    
    // Different lengths means definitely changed
    if (currentData.length !== newRows.length) {
      return false;
    }
    
    // Empty arrays are equal
    if (currentData.length === 0) {
      return true;
    }
    
    // Use rowKey if available for faster comparison
    const rowKey = this.options.rowKey;
    if (rowKey) {
      // Compare by row keys and a subset of values for performance
      for (let i = 0; i < newRows.length; i++) {
        const oldRow = currentData[i];
        const newRow = newRows[i];
        
        // Check if key changed
        if (oldRow?.[rowKey] !== newRow?.[rowKey]) {
          return false;
        }
        
        // Quick shallow comparison of visible columns
        for (const col of this.options.columns) {
          if (oldRow?.[col.id] !== newRow?.[col.id]) {
            return false;
          }
        }
      }
      return true;
    }
    
    // Fallback: JSON stringify comparison (slower but thorough)
    try {
      return JSON.stringify(currentData) === JSON.stringify(newRows);
    } catch {
      // If JSON stringify fails, assume data changed
      return false;
    }
  }

  _replaceData(rows, meta = {}) {
    const sanitized = Array.isArray(rows)
      ? rows.filter((row) => row !== null && row !== undefined)
      : [];
    
    // Check if data has actually changed to avoid unnecessary re-renders
    // This prevents DOM churn during polling when data is the same
    if (!meta.forceRender && this._isDataUnchanged(sanitized)) {
      this._log("debug", "Data unchanged, skipping re-render", { rows: sanitized.length });
      this._updatePaginationMeta(meta, { replace: true });
      this._setLoadingState(false);
      return;
    }
    
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
          : pagination.cursorNext !== null && pagination.cursorNext !== undefined;
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
          : pagination.cursorPrev !== null && pagination.cursorPrev !== undefined;
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
      this.state.data.map((existing) => this._getRowKey(existing)).filter((key) => key !== null)
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

    // If in pages mode (hybrid pagination), use server page loading
    // Preserve current page during refresh unless resetPage is true
    if (this._serverPaginationMode === "pages" && this._hasHybridPaginationModes()) {
      const preservePage = options.resetPage !== true;
      const page = preservePage ? this.state.serverPaginationState?.currentPage || 1 : 1;
      return this._loadServerPage(page, {
        silent: options.silent,
        reason: options.reason,
      });
    }

    const pagination = this._pagination;
    const reason = options.reason ?? "reload";
    const silent = options.silent ?? false;
    const preserveScroll = options.preserveScroll ?? false;

    this._cancelPaginationRequest();

    pagination.cursorNext = options.cursor ?? pagination.initialCursor ?? null;
    pagination.cursorPrev = options.prevCursor ?? pagination.initialPrevCursor ?? null;
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
      // Pass sorting column for visual feedback
      this._setLoadingState(true, {
        sortingColumn: reason === "reload" || reason === "sort" ? this.state.sortColumn : null,
      });
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

  /**
   * Update the stateKey and reload state
   * Useful for pages with tabs where each tab should have separate table state
   * @param {string} newStateKey - The new state key to use
   * @param {Object} options - Options for state reload
   * @param {boolean} options.saveCurrentState - Save current state before switching (default: true)
   * @param {boolean} options.render - Re-render table after state reload (default: true)
   */
  setStateKey(newStateKey, options = {}) {
    if (!newStateKey || typeof newStateKey !== "string" || newStateKey.trim().length === 0) {
      this._log("error", "Invalid stateKey provided", { newStateKey });
      return;
    }

    const saveCurrentState = options.saveCurrentState !== false;
    const shouldRender = options.render !== false;

    // Save current state before switching (optional)
    if (saveCurrentState) {
      this._saveState();
      this._log("debug", "Saved state before switching", {
        oldStateKey: this.options.stateKey,
        newStateKey,
      });
    }

    // Update stateKey
    this.options.stateKey = newStateKey;

    // Load state for new stateKey
    this._loadState();
    this._log("info", "StateKey updated and state reloaded", {
      stateKey: newStateKey,
      loadedState: {
        sortColumn: this.state.sortColumn,
        sortDirection: this.state.sortDirection,
      },
    });

    // Re-render if requested
    if (shouldRender) {
      this._renderTable();
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

  _setLoadingState(value, options = {}) {
    const normalized = Boolean(value);
    if (this.state.isLoading === normalized && !options.force) {
      return;
    }
    this.state.isLoading = normalized;

    // Update wrapper loading class for CSS transitions
    if (this.elements.wrapper) {
      this.elements.wrapper.classList.toggle("is-loading", normalized);
    }

    // Track which column is being sorted for loading indicator
    if (options.sortingColumn) {
      this._loadingSortColumn = options.sortingColumn;
    }
    if (!normalized) {
      this._loadingSortColumn = null;
    }

    // Update sorted column loading state
    this._syncSortColumnLoadingState();
  }

  _syncSortColumnLoadingState() {
    if (!this.elements.thead) return;

    // Remove loading class from all columns
    this.elements.thead.querySelectorAll("th.is-sorting").forEach((th) => {
      th.classList.remove("is-sorting");
    });

    // Add loading class to the column being sorted
    if (this._loadingSortColumn && this.state.isLoading) {
      const th = this.elements.thead.querySelector(
        `th[data-column-id="${this._loadingSortColumn}"]`
      );
      if (th) {
        th.classList.add("is-sorting");
      }
    }
  }

  /**
   * Update header sort indicators without re-rendering the entire table
   * Used for server-side sorting to avoid unnecessary body re-render
   */
  _updateHeaderSortIndicators() {
    if (!this.elements.thead) return;

    const { sortColumn, sortDirection } = this.state;

    // Update all sortable header cells
    this.elements.thead.querySelectorAll("th.sortable[data-column-id]").forEach((th) => {
      const columnId = th.dataset.columnId;
      const isSorted = columnId === sortColumn;

      // Update sorted class (matches _renderHeader contract)
      th.classList.toggle("sorted", isSorted);

      // Update sort icon text (matches _renderHeader contract)
      const iconEl = th.querySelector(".dt-sort-icon");
      if (iconEl) {
        iconEl.textContent =
          isSorted ? (sortDirection === "asc" ? "▲" : "▼") : "";
      }

      // Update aria-sort attribute
      if (isSorted) {
        th.setAttribute("aria-sort", sortDirection === "asc" ? "ascending" : "descending");
      } else {
        th.removeAttribute("aria-sort");
      }
    });

    // Sync loading state on the column being sorted
    this._syncSortColumnLoadingState();
  }

  /**
   * Load state from server
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
      if (saved.userResizedColumns && typeof saved.userResizedColumns === "object") {
        this.state.userResizedColumns = { ...saved.userResizedColumns };
      } else {
        this.state.userResizedColumns = {};
      }

      // Restore server page size if saved
      if (saved.serverPageSize && Number.isFinite(saved.serverPageSize)) {
        this.state.serverPaginationState.pageSize = saved.serverPageSize;
      }

      this._log("info", "State loaded", saved);
    }

    // Load client pagination preference (separate state key)
    if (this.options.clientPagination?.enabled) {
      this._loadClientPaginationPreference();
    }
  }

  /**
   * Restore server-side state and trigger callbacks
   * This enables automatic persistence for server-side sorting, filtering, and search
   */
  _restoreServerState() {
    if (!this.options.restoreServerState || this._serverStateRestored) {
      return;
    }

    this._serverStateRestored = true;

    // For server-side tables, fire onChange callbacks with restored state
    // This allows pages to reload data with the restored parameters

    // Restore server-side sort
    if (this._getSortingMode() === "server" && this.state.sortColumn) {
      const sortingConfig = this.options.sorting || {};
      if (typeof sortingConfig.onChange === "function") {
        // Defer callback to avoid firing before table is fully initialized
        queueMicrotask(() => {
          sortingConfig.onChange({
            column: this.state.sortColumn,
            direction: this.state.sortDirection,
            table: this,
            restored: true, // Flag to indicate this is state restoration
          });
        });
        this._log("info", "Server-side sort state restored", {
          column: this.state.sortColumn,
          direction: this.state.sortDirection,
        });
      }
    }

    // Restore server-side search
    if (this._getSearchMode() === "server" && this.state.searchQuery) {
      const searchConfig = this.options.toolbar?.search || {};
      if (typeof searchConfig.onChange === "function") {
        queueMicrotask(() => {
          searchConfig.onChange(this.state.searchQuery, null, { restored: true });
        });
        this._log("info", "Server-side search state restored", {
          query: this.state.searchQuery,
        });
      }
    }

    // Restore server-side filters
    if (this.options.toolbar?.filters) {
      Object.entries(this.state.filters).forEach(([filterId, value]) => {
        const filterConfig = this.options.toolbar.filters.find((f) => f.id === filterId);
        if (filterConfig && this._isServerFilter(filterConfig)) {
          if (typeof filterConfig.onChange === "function") {
            queueMicrotask(() => {
              filterConfig.onChange(value, null, { restored: true });
            });
            this._log("info", "Server-side filter state restored", {
              filterId,
              value,
            });
          }
        }
      });
    }
  }

  /**
   * Get server-side state (for pages that need to read it)
   */
  getServerState() {
    return {
      sortColumn: this.state.sortColumn,
      sortDirection: this.state.sortDirection,
      searchQuery: this.state.searchQuery,
      filters: { ...this.state.filters },
    };
  }

  /**
   * Save state to server
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
      serverPageSize: this.state.serverPaginationState.pageSize,
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
          payload.cursorNext ?? payload.cursor_next ?? payload.next_cursor ?? meta.cursorNext,
        cursorPrev:
          payload.cursorPrev ??
          payload.cursor_prev ??
          payload.prev_cursor ??
          payload.previous_cursor ??
          meta.cursorPrev,
        hasMoreNext: payload.hasMoreNext ?? payload.has_more_next ?? meta.hasMoreNext,
        hasMorePrev: payload.hasMorePrev ?? payload.has_more_prev ?? meta.hasMorePrev,
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
   * Open the table settings dialog
   */
  _openSettingsDialog() {
    if (!this._settingsDialog) {
      this._settingsDialog = new TableSettingsDialog({
        columns: this.options.columns,
        currentOrder: this.state.columnOrder,
        currentVisibility: this.state.visibleColumns,
        onApply: (settings) => this.applySettings(settings),
        // Pagination toggle options
        showPaginationToggle: !!this.options.clientPagination?.enabled,
        paginationEnabled: this._loadPaginationEnabledPreference(),
        onPaginationToggle: (enabled) => this._handlePaginationToggle(enabled),
      });
    }

    // Update current state before opening
    this._settingsDialog.options.currentOrder = this.state.columnOrder;
    this._settingsDialog.options.currentVisibility = this.state.visibleColumns;
    // Update pagination state
    if (this.options.clientPagination?.enabled) {
      this._settingsDialog.updatePaginationState(this._clientPaginationActive);
    }

    this._settingsDialog.open();
  }

  /**
   * Apply column settings from the settings dialog
   * @param {Object} settings - {columnOrder: string[], visibleColumns: {[id]: boolean}}
   */
  applySettings(settings) {
    if (!settings) {
      console.warn("[DataTable] applySettings called with no settings");
      return;
    }

    let hasChanges = false;

    // Validate and apply column order
    if (Array.isArray(settings.columnOrder) && settings.columnOrder.length > 0) {
      // Validate that all column IDs exist in table configuration
      const validColumnIds = new Set(this.options.columns.map((col) => col.id));
      const validOrder = settings.columnOrder.filter((colId) => validColumnIds.has(colId));

      if (validOrder.length > 0 && !this._arraysEqual(validOrder, this.state.columnOrder)) {
        this.state.columnOrder = validOrder;
        hasChanges = true;
      }
    }

    // Apply column visibility
    if (settings.visibleColumns && typeof settings.visibleColumns === "object") {
      const validColumnIds = new Set(this.options.columns.map((col) => col.id));

      Object.keys(settings.visibleColumns).forEach((colId) => {
        // Only apply visibility for valid column IDs
        if (validColumnIds.has(colId)) {
          const newVisibility = settings.visibleColumns[colId];
          if (this.state.visibleColumns[colId] !== newVisibility) {
            this.state.visibleColumns[colId] = newVisibility;
            hasChanges = true;
            // Reset hasAutoFitted when visibility changes so columns re-fit
            this.state.hasAutoFitted = false;
          }
        }
      });
    }

    if (hasChanges) {
      this._saveState();
      this._renderTable();
      this._log("info", "Table settings applied", {
        columnOrder: this.state.columnOrder,
        visibleColumns: this.state.visibleColumns,
      });
    }
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
   * Get client pagination state
   * @returns {Object|null} - Pagination state or null if not enabled
   */
  getClientPaginationState() {
    if (!this.options.clientPagination?.enabled) {
      return null;
    }

    return {
      currentPage: this.state.clientPaginationState.currentPage,
      pageSize: this.state.clientPaginationState.pageSize,
      totalPages: this.state.clientPaginationState.totalPages,
      totalItems: this.state.filteredData.length,
    };
  }

  /**
   * Set client pagination page programmatically
   * @param {number|string} page - Page number or 'first', 'last', 'prev', 'next'
   */
  setClientPage(page) {
    this._goToClientPage(page);
  }

  /**
   * Set client pagination page size programmatically
   * @param {number|string} size - Page size or 'all'
   */
  setClientPageSize(size) {
    this._setClientPageSize(size);
  }

  /**
   * Update table columns dynamically
   * Useful for views with conditional columns (e.g., tokens page sub-tabs)
   * @param {Array} newColumns - Array of column configurations
   * @param {Object} options - Options for column update
   * @param {boolean} options.preserveData - Keep current data (default: true)
   * @param {boolean} options.preserveScroll - Keep scroll position (default: true)
   * @param {boolean} options.resetState - Reset column widths/visibility/order (default: false)
   */
  setColumns(newColumns, options = {}) {
    if (!Array.isArray(newColumns) || newColumns.length === 0) {
      this._log("error", "setColumns: newColumns must be a non-empty array");
      return;
    }

    const preserveData = options.preserveData !== false;
    const preserveScroll = options.preserveScroll !== false;
    const resetState = options.resetState === true;

    // Save current scroll position
    const scrollPosition = preserveScroll ? this.elements.scrollContainer?.scrollTop || 0 : 0;

    // Update columns
    this.options.columns = newColumns;

    if (resetState) {
      // Reset all column-related state
      this.state.columnWidths = {};
      this.state.visibleColumns = {};
      this.state.columnOrder = [];
      this.state.userResizedColumns = {};
      this.state.hasAutoFitted = false;
      this._log("info", "Column state reset");
    } else {
      // Clean up state for columns that no longer exist
      const validColumnIds = new Set(newColumns.map((col) => col.id));

      // Remove widths for deleted columns
      Object.keys(this.state.columnWidths).forEach((colId) => {
        if (!validColumnIds.has(colId)) {
          delete this.state.columnWidths[colId];
        }
      });

      // Remove visibility for deleted columns
      Object.keys(this.state.visibleColumns).forEach((colId) => {
        if (!validColumnIds.has(colId)) {
          delete this.state.visibleColumns[colId];
        }
      });

      // Remove deleted columns from order
      this.state.columnOrder = this.state.columnOrder.filter((colId) => validColumnIds.has(colId));

      // Remove user resize flags for deleted columns
      Object.keys(this.state.userResizedColumns).forEach((colId) => {
        if (!validColumnIds.has(colId)) {
          delete this.state.userResizedColumns[colId];
        }
      });

      // Check if new columns were added (columns without widths)
      // If so, reset hasAutoFitted to allow re-fitting with new columns
      const hasNewColumns = newColumns.some(
        (col) => typeof this.state.columnWidths[col.id] !== "number"
      );
      if (hasNewColumns) {
        this.state.hasAutoFitted = false;
      }
    }

    // Save updated state
    this._saveState();

    // Re-render table structure with new columns
    if (preserveData && this.state.data.length > 0) {
      // Re-apply filters with new columns (some filter functions may reference column IDs)
      // Note: _applyFilters() already calls _renderTable() internally
      this._applyFilters();
    } else {
      // Just re-render empty table
      this._renderTable();
    }

    // Restore scroll position
    if (preserveScroll && this.elements.scrollContainer) {
      this.elements.scrollContainer.scrollTop = scrollPosition;
    }

    this._log("info", "Columns updated", {
      columnCount: newColumns.length,
      preserveData,
      preserveScroll,
      resetState,
    });
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

    if (!options.silent) {
      this._setLoadingState(true);
    }
    if (!hadRows) {
      this._renderTable();
    }

    const refreshPromise = Promise.resolve(this.options.onRefresh(options))
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
    let normalizedValue = value;
    if (this.elements.toolbar) {
      const input = this.elements.toolbar.querySelector(`.dt-filter[data-filter-id="${filterId}"]`);
      if (input?.type === "checkbox") {
        normalizedValue = Boolean(value);
      }
      TableToolbarView.setFilterValue(this.elements.toolbar, filterId, normalizedValue);
    }
    this.state.filters[filterId] = normalizedValue;
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
      TableToolbarView.setCustomControlValue(this.elements.toolbar, controlId, nextValue);
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
    
    // Cancel hover timeout
    if (this._hoverTimeout) {
      clearTimeout(this._hoverTimeout);
      this._hoverTimeout = null;
    }

    // Clean up settings dialog
    if (this._settingsDialog) {
      this._settingsDialog.destroy();
      this._settingsDialog = null;
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

    // Clear data arrays to release memory
    this.state.data = [];
    this.state.filteredData = [];
    this.state.selectedRows.clear();

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
