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
 *       { id: 'refresh', label: 'Refresh', icon: 'icon-refresh-cw', onClick: () => table.refresh() }
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
 *     fallback: string - Fallback image URL (optional)
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
import { applyColumnManagementMixin } from "./data_table/column_management.js";
import { applyClientPaginationMixin } from "./data_table/client_pagination.js";
import { applyServerPaginationMixin } from "./data_table/server_pagination.js";
import { applyEventHandlersMixin } from "./data_table/event_handlers.js";

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
      floatingColumns: [], // Ordered ids of pinned/floating (sticky-left) columns
      selectedRows: new Set(),
      scrollPosition: 0,
      isLoading: false,
      isLoadingSubtle: false,
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

    // Monotonic id for the active visible (initial) load. Incremented on every
    // reload so a superseded reload's finally can detect it is no longer current
    // and avoid turning OFF the loading indicator the newer reload just turned ON.
    this._loadToken = 0;

    // Interaction tracking: skip re-renders while user is hovering/interacting
    this._isHovering = false;
    this._hoverTimeout = null;

    // PITFALL FIX: _handleResize / _handleResizeEnd are prototype methods that are
    // attached to (and detached from) the document via add/removeEventListener in the
    // event handlers mixin. Passing the bare prototype reference meant `this` was lost
    // when the events fired (resize did nothing), and add/removeEventListener received
    // DIFFERENT function identities on each call so listeners were never actually
    // removed. Binding ONCE here gives a single, stable bound reference that both
    // add and remove use, fixing resize and preventing a stuck `this.resizing` flag
    // (which would otherwise wedge _isUserInteracting() and block sorting/renders).
    // Do not revert to passing the unbound prototype methods.
    this._handleResize = this._handleResize.bind(this);
    this._handleResizeEnd = this._handleResizeEnd.bind(this);

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

    // Viewport-relative prefetch ratio: in scroll mode the next page is fetched
    // once the un-scrolled remainder drops below `prefetchRatio` of the scrollable
    // range (so loading starts ~30% before the end, not AT the end → smooth, no
    // visible stall). The fixed `threshold` above acts as a pixel floor for short
    // lists where 30% would be a tiny distance.
    const prefetchRatio =
      typeof paginationOptions.prefetchRatio === "number" &&
      Number.isFinite(paginationOptions.prefetchRatio)
        ? Math.min(0.9, Math.max(0, paginationOptions.prefetchRatio))
        : 0.3;

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
      prefetchRatio,
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

    // Recompute pinned-column offsets when the viewport changes (column widths
    // can re-fit to the container) so sticky offsets stay accurate.
    this._windowResizeHandler = () => {
      this._updateStickyOffsets();
    };
    window.addEventListener("resize", this._windowResizeHandler);

    // Observe the wrapper for size changes (layout/sidebar toggles, fit-to-container
    // reflows) and refresh pinned offsets. Guarded for environments lacking the API.
    if (typeof ResizeObserver === "function" && this.elements.wrapper) {
      this._wrapperResizeObserver = new ResizeObserver(() => {
        this._updateStickyOffsets();
      });
      this._wrapperResizeObserver.observe(this.elements.wrapper);
    }
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

    const tableClasses = `data-table ${this.options.compact ? "compact" : ""} ${
      this.options.zebra ? "zebra" : ""
    } ${uniformRowsLines ? "uniform-rows" : ""}`.trim();
    const tableStyle = uniformRowsLines ? `style="--dt-row-lines: ${uniformRowsLines};"` : "";

    container.innerHTML = `
      <div class="data-table-wrapper ${fixedHeightClass}" ${
        typeof this.options.fixedHeight === "number"
          ? `style="height: ${this.options.fixedHeight}px;"`
          : ""
      }>
        ${this._renderToolbar()}
        <div class="data-table-header-container">
          <table class="${tableClasses}" ${tableStyle}>
            ${this._renderColgroup()}
            <thead>
              ${this._renderHeader()}
            </thead>
          </table>
        </div>
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
          <table class="${tableClasses}" ${tableStyle}>
            ${this._renderColgroup()}
            <tbody>
              ${this._renderBody()}
            </tbody>
          </table>
        </div>
        <div class="dt-scroll-loader" aria-hidden="true">
          <i class="dt-scroll-loader__icon icon-loader-circle" aria-hidden="true"></i>
          <span class="dt-scroll-loader__text">Loading more…</span>
        </div>
        ${this._renderClientPaginationBar()}
        ${this._renderServerPaginationBar()}
      </div>
    `;

    this.elements.wrapper = container.querySelector(".data-table-wrapper");
    this.elements.toolbar = container.querySelector(".data-table-toolbar");
    this.elements.headerContainer = container.querySelector(".data-table-header-container");
    this.elements.scrollContainer = container.querySelector(".data-table-scroll-container");
    this.elements.headerTable = this.elements.headerContainer?.querySelector(".data-table");
    this.elements.table = this.elements.scrollContainer?.querySelector(".data-table");
    this.elements.thead = container.querySelector("thead");
    this.elements.tbody = container.querySelector("tbody");
    this.elements.scrollLoader = container.querySelector(".dt-scroll-loader");
    this.elements.blockingState = container.querySelector(".data-table-blocking-state");
    this.elements.blockingStateIcon = container.querySelector(".data-table-blocking-state__icon");
    this.elements.blockingStateTitle = container.querySelector(".data-table-blocking-state__title");
    this.elements.blockingStateDescription = container.querySelector(
      ".data-table-blocking-state__description"
    );
    this.elements.clientPaginationBar = container.querySelector(".dt-client-pagination-bar");
    this.elements.serverPaginationBar = container.querySelector(".dt-server-pagination-bar");

    // Cache col elements for fast width updates (from BODY table — header syncs from these)
    this.elements.colgroup = this.elements.table?.querySelector("colgroup");
    this.elements.headerColgroup = this.elements.headerTable?.querySelector("colgroup");
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

    // Apply stored table width to BOTH header and body tables
    if (typeof this.state.tableWidth === "number") {
      if (this.elements.table) this.elements.table.style.width = `${this.state.tableWidth}px`;
      if (this.elements.headerTable) this.elements.headerTable.style.width = `${this.state.tableWidth}px`;
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

    const floatingIds = Array.isArray(this.state.floatingColumns)
      ? this.state.floatingColumns
      : [];

    // Resolve the non-floating columns first using the existing columnOrder logic,
    // so behaviour is byte-for-byte identical when no columns are floating.
    let nonFloating;
    if (this.state.columnOrder.length === 0) {
      nonFloating = sourceColumns;
    } else {
      nonFloating = [];
      const columnMap = new Map(sourceColumns.map((col) => [col.id, col]));

      // Add columns in the order specified by columnOrder
      for (const colId of this.state.columnOrder) {
        if (columnMap.has(colId)) {
          nonFloating.push(columnMap.get(colId));
          columnMap.delete(colId);
        }
      }

      // Add any remaining columns that weren't in columnOrder (new columns)
      nonFloating.push(...columnMap.values());
    }

    // Fast path: nothing pinned → identical to the historical behaviour above.
    if (floatingIds.length === 0) {
      return nonFloating;
    }

    // Floating columns are always leftmost, in floatingColumns order, regardless
    // of columnOrder. Build the group from the (visible-filtered) source columns
    // and strip those ids out of the non-floating list so a column never appears
    // in both groups.
    const sourceById = new Map(sourceColumns.map((col) => [col.id, col]));
    const floatingSet = new Set(floatingIds);
    const floatingGroup = [];
    for (const colId of floatingIds) {
      if (sourceById.has(colId)) {
        floatingGroup.push(sourceById.get(colId));
      }
    }

    return [...floatingGroup, ...nonFloating.filter((col) => !floatingSet.has(col.id))];
  }

  /**
   * Render colgroup for efficient column width management
   * Using <col> elements allows the browser to handle column widths
   * without needing to update every cell individually
   */
  _renderColgroup() {
    const visibleColumns = this._getOrderedColumns();
    const floatingCount = this._getFloatingColumnCount(visibleColumns);

    return `
      <colgroup>
        ${visibleColumns
          .map((col, index) => {
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
            const stickyAttrs = this._stickyCellMarkup(index, floatingCount);
            return `<col data-column-id="${col.id}"${stickyAttrs}${styleAttr}>`;
          })
          .join("")}
      </colgroup>
    `;
  }

  /**
   * Count how many of the leading (leftmost) columns are floating/pinned.
   * Because `_getOrderedColumns()` always places the floating group first, the
   * floating columns are exactly the first N entries, so a simple count of the
   * visible columns whose id is in `floatingColumns` is sufficient.
   * @param {Array} orderedColumns - result of `_getOrderedColumns()`
   * @returns {number} number of leading floating columns
   */
  _getFloatingColumnCount(orderedColumns) {
    const floatingIds = Array.isArray(this.state.floatingColumns)
      ? this.state.floatingColumns
      : [];
    if (floatingIds.length === 0) {
      return 0;
    }
    const floatingSet = new Set(floatingIds);
    let count = 0;
    for (const col of orderedColumns) {
      if (floatingSet.has(col.id)) {
        count += 1;
      }
    }
    return count;
  }

  /**
   * Compute sticky marker pieces for a cell at the given ordered index.
   * Floating columns (the leading `floatingCount` entries) get the
   * `dt-col-sticky` class, `dt-col-sticky-last` on the last one, and a
   * 0-based `data-sticky-index`. Non-floating columns get empty strings, so
   * tables with no floating columns emit no sticky markup at all.
   * @param {number} index - 0-based index within the ordered (visible) columns
   * @param {number} floatingCount - number of leading floating columns
   * @returns {{classes: string, attr: string}} classes to append + extra attrs
   */
  _stickyCellParts(index, floatingCount) {
    if (floatingCount <= 0 || index >= floatingCount) {
      return { classes: "", attr: "" };
    }
    const isLast = index === floatingCount - 1;
    return {
      classes: `dt-col-sticky${isLast ? " dt-col-sticky-last" : ""}`,
      attr: ` data-sticky-index="${index}"`,
    };
  }

  /**
   * Build the full sticky attribute string for a bare `<col>` element (which has
   * no other class attribute). Returns "" for non-floating columns.
   * @param {number} index - 0-based ordered column index
   * @param {number} floatingCount - number of leading floating columns
   * @returns {string} attribute string to splice into a <col> tag
   */
  _stickyCellMarkup(index, floatingCount) {
    const { classes, attr } = this._stickyCellParts(index, floatingCount);
    if (!classes) {
      return "";
    }
    return ` class="${classes}"${attr}`;
  }

  /**
   * Render table header with sortable columns
   */
  _renderHeader() {
    const visibleColumns = this._getOrderedColumns();
    const floatingCount = this._getFloatingColumnCount(visibleColumns);

    return `
      <tr>
        ${visibleColumns
          .map((col, index) => {
            const isSorted = this.state.sortColumn === col.id;
            const sortIcon = isSorted ? (this.state.sortDirection === "asc" ? "▲" : "▼") : "";
            const sticky = this._stickyCellParts(index, floatingCount);

            return `
            <th
              data-column-id="${col.id}"
              class="dt-header-column ${col.sortable ? "sortable" : ""} ${isSorted ? "sorted" : ""} ${sticky.classes}"${sticky.attr}
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
    const emptyTitle = hasFilters ? "No results found" : this.options.emptyTitle || "No data";
    const emptyMessage = hasFilters
      ? "Try adjusting your search or filters"
      : this.options.emptyMessage || "No data to display";

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
        const customClass = this._computeRowClass(row);
        const rowClass = [isSelected ? "selected" : "", customClass]
          .filter(Boolean)
          .join(" ");

        return `
        <tr
          data-row-id="${rowId}"
          class="${rowClass}"${customClass ? ` data-dt-row-class="${customClass}"` : ""}
        >
          ${this._renderRow(row)}
        </tr>
      `;
      })
      .join("");
  }

  /**
   * Compute the optional per-row CSS class from the `rowClass` option.
   * Lets pages tag rows with state styling (e.g. pending/selling) without
   * forking the table. Returns a trimmed, space-separated class string.
   */
  _computeRowClass(row) {
    const fn = this.options.rowClass;
    if (typeof fn !== "function") return "";
    try {
      const cls = fn(row);
      return typeof cls === "string" ? cls.trim() : "";
    } catch (error) {
      this._log("error", "rowClass function failed", error);
      return "";
    }
  }

  /**
   * Apply the computed custom row class to an existing <tr>, diffing against
   * the previously applied set so we never clobber framework classes
   * (e.g. `selected`). Tracks the applied classes on `data-dt-row-class`.
   */
  _syncRowClass(tr, row) {
    const next = this._computeRowClass(row);
    const prev = tr.getAttribute("data-dt-row-class") || "";
    if (prev === next) return;
    prev
      .split(/\s+/)
      .filter(Boolean)
      .forEach((c) => tr.classList.remove(c));
    next
      .split(/\s+/)
      .filter(Boolean)
      .forEach((c) => tr.classList.add(c));
    if (next) {
      tr.setAttribute("data-dt-row-class", next);
    } else {
      tr.removeAttribute("data-dt-row-class");
    }
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
    if (tbody.querySelector(".dt-empty-state")) {
      tbody.innerHTML = "";
    }

    const visibleColumns = this._getOrderedColumns();
    const rowIdField = this.options.rowIdField;

    // Index existing rows
    const existingRows = new Map();
    Array.from(tbody.children).forEach((tr) => {
      const id = tr.getAttribute("data-row-id");
      if (id) existingRows.set(id, tr);
    });

    // Whether the table already had rows before this update. Used to gate the
    // enter animation so an initial full load doesn't animate every row at once.
    const hadRows = existingRows.size > 0;

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
        if (tr.classList.contains("selected") !== isSelected) {
          tr.classList.toggle("selected", isSelected);
        }

        // Keep the optional per-row state class in sync (pending/selling/etc.)
        this._syncRowClass(tr, row);

        // Update cells
        visibleColumns.forEach((col) => {
          const td = tr.querySelector(`td[data-column-id="${col.id}"]`);
          if (td) {
            const cellContent = this._renderCellContent(col, row);

            // Handle wrapping wrapper
            const shouldClamp = this.options.uniformRowHeight && col.wrap !== false;

            let newContent = shouldClamp
              ? `<div class="dt-cell-clamp">${cellContent}</div>`
              : cellContent;

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
        const tr = document.createElement("tr");
        tr.setAttribute("data-row-id", rowIdStr);
        if (this.state.selectedRows.has(rowId)) {
          tr.classList.add("selected");
        }
        this._syncRowClass(tr, row);
        tr.innerHTML = this._renderRow(row);
        // Subtle enter animation only on incremental updates (not initial load),
        // so freshly-injected rows (e.g. a pending buy) slide in gracefully.
        if (hadRows && !this._prefersReducedMotion()) {
          tr.classList.add("dt-row-enter");
          window.setTimeout(() => tr.classList.remove("dt-row-enter"), 360);
        }
        fragment.appendChild(tr);
      }
    });

    // Remove remaining rows — optionally play a leave animation for rows the
    // page flags (e.g. a position that just finished closing) before removal.
    const leaving = Array.from(existingRows.values());
    const animateExit =
      typeof this.options.rowExitAnimation === "function" &&
      leaving.length > 0 &&
      leaving.length <= 8 &&
      !this._prefersReducedMotion();
    leaving.forEach((tr) => {
      let shouldAnimate = false;
      if (animateExit) {
        try {
          shouldAnimate = this.options.rowExitAnimation(tr) === true;
        } catch (error) {
          this._log("error", "rowExitAnimation function failed", error);
        }
      }
      if (shouldAnimate) {
        this._animateRowExit(tr);
      } else {
        tr.remove();
      }
    });

    // Append fragment (reorders existing rows and adds new ones)
    tbody.appendChild(fragment);
  }

  /** Respect the user's reduced-motion preference for table animations. */
  _prefersReducedMotion() {
    try {
      return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    } catch {
      return false;
    }
  }

  /** Add a leaving class and remove the row after the exit animation. */
  _animateRowExit(tr) {
    if (tr.dataset.dtLeaving === "1") return;
    tr.dataset.dtLeaving = "1";
    tr.classList.add("dt-row-leaving");
    const dur = Number(this.options.rowExitDurationMs) || 340;
    window.setTimeout(() => {
      try {
        tr.remove();
      } catch {
        /* row already detached */
      }
    }, dur);
  }

  /**
   * Render individual row cells
   */
  _renderRow(row) {
    const visibleColumns = this._getOrderedColumns();
    const floatingCount = this._getFloatingColumnCount(visibleColumns);

    return visibleColumns
      .map((col, index) => {
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

        // Sticky markers for floating/pinned-left columns
        const sticky = this._stickyCellParts(index, floatingCount);

        // Apply dt-cell-clamp for uniformRowHeight, but NOT for no-wrap columns
        // no-wrap columns handle truncation via CSS (white-space: nowrap + text-overflow: ellipsis)
        const shouldClamp = this.options.uniformRowHeight && col.wrap !== false;
        const content = shouldClamp
          ? `<div class="dt-cell-clamp">${cellContent}</div>`
          : cellContent;

        return `
        <td data-column-id="${col.id}"
            class="${cellClass} ${wrapClass} ${sticky.classes}"${sticky.attr}
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
      } catch {
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
      } catch {
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
    // Remove all stored event handlers. Pass the capture flag through so
    // capture-phase listeners (e.g. the header context menu) are actually
    // removed — removeEventListener must match the phase used at registration.
    this.eventHandlers.forEach(({ element, event, handler, capture }) => {
      element.removeEventListener(event, handler, capture === true);
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

    // Group-aware reorder: the menu presents a single combined list, but floating
    // (pinned-left) columns reorder only among themselves and non-floating only
    // among themselves. Derive each group's new order by filtering the combined
    // list down to its current membership; columns never cross groups here.
    const floatingSet = new Set(
      Array.isArray(this.state.floatingColumns) ? this.state.floatingColumns : []
    );
    const nextFloating = orderedIds.filter((id) => floatingSet.has(id));
    const nextOrder = orderedIds.filter((id) => !floatingSet.has(id));

    const floatingChanged =
      floatingSet.size > 0 && !this._arraysEqual(nextFloating, this.state.floatingColumns);
    const orderChanged = !this._arraysEqual(nextOrder, this.state.columnOrder);

    if (!floatingChanged && !orderChanged) {
      return;
    }

    const existingMenu = this.elements.container
      ? this.elements.container.querySelector(".dt-column-menu")
      : null;
    const shouldReopen = existingMenu?.style.display === "block";

    if (floatingChanged) {
      this.state.floatingColumns = nextFloating;
    }
    this.state.columnOrder = nextOrder;
    this._invalidateRenderedCells();
    this._saveState();
    this._pendingColumnMenuOpen = shouldReopen;
    // `force` is required: we just emptied the tbody, but the column menu is open
    // (and being interacted with), so the interaction guard would otherwise skip
    // the render and leave the body blank until the next unrelated render.
    this._renderTable({ force: true });
    this._updateStickyOffsets();

    this._log("info", "Column order updated", {
      via: "column-menu",
      order: nextOrder,
      floating: nextFloating,
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

    // Settings dialog is appended to document.body (outside container) — detect via body class
    if (document.body.classList.contains("table-settings-open")) {
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

    // Update colgroup in BOTH header and body tables
    const colgroupMarkup = this._renderColgroup().trim();
    for (const tbl of [this.elements.table, this.elements.headerTable]) {
      if (!tbl) continue;
      const existing = tbl.querySelector("colgroup");
      if (existing) {
        existing.outerHTML = colgroupMarkup;
      } else {
        tbl.insertAdjacentHTML("afterbegin", colgroupMarkup);
      }
    }

    // Re-query elements after innerHTML update to ensure fresh references
    if (this.elements.container) {
      this.elements.thead = this.elements.container.querySelector("thead");
      this.elements.tbody = this.elements.container.querySelector("tbody");

      // Re-cache col elements from body table
      this.elements.colgroup = this.elements.table?.querySelector("colgroup");
      this.elements.headerColgroup = this.elements.headerTable?.querySelector("colgroup");
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
      if (this.elements.headerContainer) {
        this.elements.headerContainer.scrollLeft = prevScrollLeft;
      }
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

    // Recompute cumulative left offsets for any pinned/floating columns so the
    // CSS (left: var(--dt-pin-left-N)) stacks them correctly.
    this._updateStickyOffsets();
  }

  /**
   * Measure the rendered width of each floating column and publish cumulative
   * left offsets as CSS custom properties on `.data-table-wrapper`:
   *   --dt-pin-left-0: 0px;
   *   --dt-pin-left-1: <width of col 0>px;
   *   --dt-pin-left-2: <w0 + w1>px; ...
   * The CSS applies `left: var(--dt-pin-left-N)` to `[data-sticky-index="N"]`.
   * Widths are read from the HEADER table's <th> (offsetWidth); the body shares
   * the same colgroup widths, so header and body stay aligned. Stale vars from a
   * previous, larger floating group are cleared. No-op when nothing is pinned or
   * required elements are missing.
   */
  _updateStickyOffsets() {
    const wrapper = this.elements.wrapper;
    if (!wrapper) {
      return;
    }

    const floatingCount = this._getFloatingColumnCount(this._getOrderedColumns());

    // Clear any previously-set pin vars so a shrinking group leaves no leftovers.
    const previous = this._stickyVarCount || 0;
    for (let i = 0; i < Math.max(previous, floatingCount); i++) {
      wrapper.style.removeProperty(`--dt-pin-left-${i}`);
    }
    this._stickyVarCount = floatingCount;

    if (floatingCount <= 0) {
      return;
    }

    const headerThead = this.elements.headerTable?.querySelector("thead");
    if (!headerThead) {
      return;
    }

    // The floating columns are the first `floatingCount` ordered columns. Their
    // widths come from `state.columnWidths` — the single source of truth that
    // `_applyColumnWidth` writes IDENTICALLY to both the header and body <col>
    // elements. Using it (instead of reading a header <th> offsetWidth) keeps the
    // header and body pinned cells in exact lockstep during a live resize: the
    // measured `offsetWidth` of a position:sticky cell mid-drag can lag the width
    // just applied, which left the header pinned columns stuck while the body
    // followed. We fall back to a live header measurement only when a stored width
    // is missing (e.g. before the first sizing pass).
    const orderedColumns = this._getOrderedColumns();
    let cumulative = 0;
    for (let i = 0; i < floatingCount; i++) {
      wrapper.style.setProperty(`--dt-pin-left-${i}`, `${cumulative}px`);
      const colId = orderedColumns[i]?.id;
      let width = colId != null ? this.state.columnWidths[colId] : undefined;
      if (!Number.isFinite(width)) {
        const th = headerThead.querySelector(`th[data-sticky-index="${i}"]`);
        width = th ? th.offsetWidth : 0;
      }
      cumulative += Number.isFinite(width) ? width : 0;
    }
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

    // Claim ownership of the visible loading indicator for this reload. Any older
    // reload still settling will see a newer token and must not toggle loading off.
    const loadToken = ++this._loadToken;

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
      // Use subtle loading (thin bar only) for poll/refresh, full loading for user actions
      const useSubtle = reason === "poll" || reason === "refresh";
      this._setLoadingState(true, {
        subtle: useSubtle,
        sortingColumn: reason === "reload" || reason === "sort" ? this.state.sortColumn : null,
      });
      // When there are no rows to fade (initial load or a just-cleared view switch),
      // render so the loading spinner shows instead of a stale "No data" empty state.
      if (!useSubtle && this.state.filteredData.length === 0) {
        this._renderTable();
      }
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
      // Only the latest reload may clear the shared loading state. A superseded
      // reload (its request was aborted by a newer one) must leave the newer
      // reload's loading indicator intact.
      if (loadToken !== this._loadToken) {
        return;
      }
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
      // Expose the initial/replace load flag too. Silent poll reloads set
      // `loadingInitial` (NOT loadingNext) and do not flip `state.isLoading`, so
      // without this field a caller's in-flight guard can't see a poll fetch in
      // progress — consecutive polls then abort each other (_cancelPaginationRequest)
      // and a slow query (e.g. the 360k-row tokens list) never finishes → table
      // appears frozen / not live.
      loadingInitial: Boolean(this._pagination.loadingInitial),
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
    const subtle = Boolean(options.subtle);

    if (this.state.isLoading === normalized && this.state.isLoadingSubtle === subtle && !options.force) {
      return;
    }
    this.state.isLoading = normalized;
    this.state.isLoadingSubtle = normalized ? subtle : false;

    // Update wrapper loading class for CSS transitions
    if (this.elements.wrapper) {
      if (subtle && normalized) {
        this.elements.wrapper.classList.remove("is-loading");
        this.elements.wrapper.classList.add("is-loading-subtle");
      } else {
        this.elements.wrapper.classList.remove("is-loading-subtle");
        this.elements.wrapper.classList.toggle("is-loading", normalized);
      }
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
        iconEl.textContent = isSorted ? (sortDirection === "asc" ? "▲" : "▼") : "";
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
    // Whether the user already has a persisted floating set (even an empty one):
    // if so we must NOT re-seed defaults, or unpinning every column would silently
    // come back on the next load.
    const hadSavedFloating =
      !!saved && Object.prototype.hasOwnProperty.call(saved, "floatingColumns");
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

    // Seed floating (pinned-left) columns. Only when the user has NO persisted
    // floating set at all — derive the initial set from any columns flagged
    // `floating: true` in their config, preserving column-definition order.
    this._seedFloatingColumns(hadSavedFloating);
  }

  /**
   * Seed `state.floatingColumns` from column configs.
   * Columns whose config has `floating === true` become pinned by default, in
   * column-definition order. Skipped entirely when the user already has a
   * persisted set (`hasSaved`), so an explicit empty set survives reloads.
   * @param {boolean} hasSaved - true if saved state already had floatingColumns
   */
  _seedFloatingColumns(hasSaved = false) {
    if (!Array.isArray(this.state.floatingColumns)) {
      this.state.floatingColumns = [];
    }
    if (hasSaved || this.state.floatingColumns.length > 0) {
      return;
    }
    const seeded = this.options.columns
      .filter((col) => col && col.floating === true)
      .map((col) => col.id);
    if (seeded.length > 0) {
      this.state.floatingColumns = seeded;
    }
  }

  /**
   * Insert a column id into `state.floatingColumns` at the slot that matches its
   * position in the current column-definition order, so a newly-pinned default
   * (e.g. an Actions column defined before Token) lands left of columns that come
   * after it, without disturbing the user's manual ordering of the rest.
   * @param {string} colId
   */
  _insertFloatingInDefinitionOrder(colId) {
    if (!Array.isArray(this.state.floatingColumns)) {
      this.state.floatingColumns = [];
    }
    const defOrder = this.options.columns.map((col) => col.id);
    const targetIndex = defOrder.indexOf(colId);
    let insertAt = 0;
    for (const existingId of this.state.floatingColumns) {
      const existingIndex = defOrder.indexOf(existingId);
      if (existingIndex !== -1 && existingIndex < targetIndex) {
        insertAt += 1;
      }
    }
    this.state.floatingColumns.splice(insertAt, 0, colId);
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
      floatingColumns: this.state.floatingColumns,
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
        currentFloating: this.state.floatingColumns,
        onApply: (settings) => this.applySettings(settings),
        // Pagination toggle options
        showPaginationToggle: !!this.options.clientPagination?.enabled,
        paginationEnabled: this._loadPaginationEnabledPreference(),
        onPaginationToggle: (enabled) => this._handlePaginationToggle(enabled),
      });
    }

    // Update current state before opening. The dialog instance is created once and
    // reused, but the table's COLUMN SET can change between opens (e.g. switching
    // Tokens sub-tabs swaps the columns via setColumns — the Actions column only
    // exists in some views). Re-sync the columns reference too, or the dialog would
    // list/operate on a stale column set and Apply would drop or misorder columns.
    this._settingsDialog.options.columns = this.options.columns;
    this._settingsDialog.options.currentOrder = this.state.columnOrder;
    this._settingsDialog.options.currentVisibility = this.state.visibleColumns;
    this._settingsDialog.options.currentFloating = this.state.floatingColumns;
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

    // Consume floating (pinned-left) column set from the settings dialog when
    // present. Validate ids against known columns, ensure columnOrder never
    // contains a floating id (a column lives in exactly one group), then save +
    // re-render + recompute sticky offsets.
    if (Array.isArray(settings.floatingColumns)) {
      const validColumnIds = new Set(this.options.columns.map((col) => col.id));
      const validFloating = settings.floatingColumns.filter((colId) =>
        validColumnIds.has(colId)
      );
      if (!this._arraysEqual(validFloating, this.state.floatingColumns)) {
        this.state.floatingColumns = validFloating;
        hasChanges = true;
        this.state.hasAutoFitted = false;
      }
    }

    // Reconcile groups: a column lives in exactly ONE group. The dialog reports
    // columnOrder as the full id list (pinned columns included at their natural
    // slot), so always strip the pinned ids out of columnOrder regardless of
    // whether the floating set itself changed — otherwise a column that was
    // already pinned would persist in both arrays.
    if (Array.isArray(this.state.floatingColumns) && this.state.floatingColumns.length > 0) {
      const floatingSet = new Set(this.state.floatingColumns);
      this.state.columnOrder = this.state.columnOrder.filter((colId) => !floatingSet.has(colId));
    }

    if (hasChanges) {
      // The settings dialog can change the column SET (visibility), ORDER, and
      // pinned (floating) membership all at once — every one of these is a
      // structural change. The in-place body diff (`_updateTableBody`) only swaps
      // each reused <td>'s innerHTML; it never inserts cells for newly-visible
      // columns, removes cells for hidden ones, reorders cells, or refreshes the
      // sticky classes. So we MUST drop the rendered rows and rebuild them fully,
      // exactly like the pin/move paths do — otherwise the header + colgroup pick
      // up the new structure while the body keeps its old <td> set in the old
      // order (column "not showing" / "wrong position"). `force` also bypasses the
      // interaction guard so an explicit Apply is never silently skipped while a
      // control still holds focus/hover.
      this._invalidateRenderedCells();
      this.state.hasAutoFitted = false;
      this._saveState();
      this._renderTable({ force: true });
      this._updateStickyOffsets();
      this._log("info", "Table settings applied", {
        columnOrder: this.state.columnOrder,
        visibleColumns: this.state.visibleColumns,
        floatingColumns: this.state.floatingColumns,
      });
    }
  }

  /**
   * Whether a column is currently pinned (floating to the left).
   * @param {string} colId
   * @returns {boolean}
   */
  isColumnFloating(colId) {
    return (
      Array.isArray(this.state.floatingColumns) && this.state.floatingColumns.includes(colId)
    );
  }

  /**
   * Pin or unpin a column to the left.
   * - Pinning appends to the end of the floating group and removes the id from
   *   `columnOrder` (a column never lives in both groups).
   * - Unpinning removes it from the floating group and restores it into
   *   `columnOrder` (so its non-floating position is tracked again).
   * Re-renders, recomputes sticky offsets, saves state, and (when pinning)
   * briefly tags the column's th+td with `dt-col-justpinned` for the CSS move
   * animation.
   * @param {string} colId
   * @param {boolean} isFloating - true to pin, false to unpin
   */
  setColumnFloating(colId, isFloating) {
    if (!colId) {
      return;
    }
    const known = this.options.columns.some((col) => col.id === colId);
    if (!known) {
      return;
    }
    if (!Array.isArray(this.state.floatingColumns)) {
      this.state.floatingColumns = [];
    }

    const currentlyFloating = this.state.floatingColumns.includes(colId);
    const next = Boolean(isFloating);
    if (currentlyFloating === next) {
      return; // No change
    }

    if (next) {
      // Pin: append to floating group, drop from non-floating order.
      this.state.floatingColumns.push(colId);
      this.state.columnOrder = this.state.columnOrder.filter((id) => id !== colId);
    } else {
      // Unpin: remove from floating group, restore into non-floating order so it
      // keeps a tracked position (append to end if not already present).
      this.state.floatingColumns = this.state.floatingColumns.filter((id) => id !== colId);
      if (!this.state.columnOrder.includes(colId)) {
        this.state.columnOrder.push(colId);
      }
    }

    // Floating membership changes the column set per row; rebuild bodies fully so
    // reused <td> elements pick up/lose the sticky classes (the in-place body
    // diff only swaps innerHTML, not cell attributes).
    this._invalidateRenderedCells();
    this.state.hasAutoFitted = false;
    this._renderTable({ force: true });
    this._updateStickyOffsets();
    this._saveState();

    if (next) {
      this._flashJustPinned(colId);
    }

    this._log("info", "Column floating changed", { colId, isFloating: next });
  }

  /**
   * Toggle a column's pinned/floating state.
   * @param {string} colId
   */
  toggleColumnFloating(colId) {
    this.setColumnFloating(colId, !this.isColumnFloating(colId));
  }

  /**
   * Whether a column may be hidden (mirrors the settings dialog's rules so the
   * header menu and the dialog stay consistent).
   * @param {object} col - column config
   * @returns {boolean}
   */
  _canHideColumn(col) {
    if (!col) return false;
    return !(
      col.lockVisibility === true ||
      col.disableVisibilityToggle === true ||
      col.hideable === false
    );
  }

  /**
   * Open a lightweight right-click context menu for a column header at the given
   * viewport coordinates. Offers Pin/Unpin (to the left floating group) and, when
   * allowed, Hide column. The menu is a self-contained element appended to
   * document.body (NOT coupled to the app's token context menu), positioned at the
   * cursor and clamped to the viewport, and dismissed on outside click / Escape /
   * scroll / resize. Only one menu exists at a time.
   * @param {string} colId
   * @param {number} x - clientX
   * @param {number} y - clientY
   */
  _openHeaderColumnMenu(colId, x, y) {
    this._closeHeaderColumnMenu();

    const col = this.options.columns.find((c) => c.id === colId);
    if (!col) {
      return;
    }

    const isFloating = this.isColumnFloating(colId);
    const items = [];

    // Pin / Unpin (unless the column explicitly opts out via floatable: false)
    if (col.floatable !== false) {
      items.push({
        label: isFloating ? "Unpin from left" : "Pin to left",
        icon: isFloating ? "icon-pin-off" : "icon-pin",
        onClick: () => this.toggleColumnFloating(colId),
      });
    }

    // Hide column (when permitted)
    if (this._canHideColumn(col)) {
      items.push({
        label: "Hide column",
        icon: "icon-eye-off",
        onClick: () => {
          this.applySettings({
            visibleColumns: { ...this.state.visibleColumns, [colId]: false },
          });
        },
      });
    }

    if (items.length === 0) {
      return;
    }

    const menu = document.createElement("div");
    menu.className = "dt-col-menu";
    menu.setAttribute("role", "menu");
    menu.innerHTML = items
      .map(
        (item, i) =>
          `<button type="button" class="dt-col-menu-item" role="menuitem" data-index="${i}">
            <i class="dt-col-menu-icon ${item.icon}" aria-hidden="true"></i>
            <span>${item.label}</span>
          </button>`
      )
      .join("");

    document.body.appendChild(menu);

    // Position at the cursor, clamped so the menu stays fully on-screen.
    const rect = menu.getBoundingClientRect();
    const left = Math.min(x, window.innerWidth - rect.width - 8);
    const top = Math.min(y, window.innerHeight - rect.height - 8);
    menu.style.left = `${Math.max(8, left)}px`;
    menu.style.top = `${Math.max(8, top)}px`;

    // Wire item clicks.
    menu.querySelectorAll(".dt-col-menu-item").forEach((btn) => {
      btn.addEventListener("click", () => {
        const idx = Number(btn.dataset.index);
        this._closeHeaderColumnMenu();
        items[idx]?.onClick?.();
      });
    });

    // Dismissal: outside pointer-down, Escape, scroll, resize. Defer attaching the
    // outside-click listener so the originating right-click doesn't instantly close it.
    const onPointerDown = (ev) => {
      if (!menu.contains(ev.target)) {
        this._closeHeaderColumnMenu();
      }
    };
    const onKeyDown = (ev) => {
      if (ev.key === "Escape") {
        this._closeHeaderColumnMenu();
      }
    };
    const onDismiss = () => this._closeHeaderColumnMenu();

    this._headerColMenu = { el: menu, onPointerDown, onKeyDown, onDismiss };

    window.setTimeout(() => {
      document.addEventListener("mousedown", onPointerDown, true);
      document.addEventListener("keydown", onKeyDown, true);
      window.addEventListener("scroll", onDismiss, true);
      window.addEventListener("resize", onDismiss, true);
    }, 0);

    // Focus the first item for keyboard users.
    menu.querySelector(".dt-col-menu-item")?.focus();
  }

  /**
   * Close and clean up the header column context menu, if open.
   */
  _closeHeaderColumnMenu() {
    const ref = this._headerColMenu;
    if (!ref) {
      return;
    }
    this._headerColMenu = null;
    document.removeEventListener("mousedown", ref.onPointerDown, true);
    document.removeEventListener("keydown", ref.onKeyDown, true);
    window.removeEventListener("scroll", ref.onDismiss, true);
    window.removeEventListener("resize", ref.onDismiss, true);
    ref.el?.remove();
  }

  /**
   * Reorder a column WITHIN the floating group only.
   * @param {string} colId - id of a currently-floating column
   * @param {number} toIndex - target index within `floatingColumns`
   */
  moveFloatingColumn(colId, toIndex) {
    if (!Array.isArray(this.state.floatingColumns)) {
      return;
    }
    const from = this.state.floatingColumns.indexOf(colId);
    if (from === -1) {
      return;
    }
    const clamped = Math.max(0, Math.min(this.state.floatingColumns.length - 1, toIndex));
    if (clamped === from) {
      return;
    }
    const [moved] = this.state.floatingColumns.splice(from, 1);
    this.state.floatingColumns.splice(clamped, 0, moved);

    this._invalidateRenderedCells();
    this._renderTable({ force: true });
    this._updateStickyOffsets();
    this._saveState();

    this._log("info", "Floating column moved", { colId, toIndex: clamped });
  }

  /**
   * Force the next render to rebuild row cells from scratch. The in-place body
   * diff (`_updateTableBody`) reuses existing <td> elements and only updates
   * their innerHTML, so structural attribute changes (sticky classes/indices)
   * would not propagate to existing rows. Clearing the tbody makes the diff
   * treat every row as new.
   */
  _invalidateRenderedCells() {
    if (this.elements.tbody) {
      this.elements.tbody.innerHTML = "";
    }
  }

  /**
   * Briefly add `dt-col-justpinned` to a column's header + body cells so the CSS
   * can animate the column sliding into the pinned group, then remove it.
   * @param {string} colId
   */
  _flashJustPinned(colId) {
    const apply = (root) => {
      if (!root) return [];
      return Array.from(root.querySelectorAll(`[data-column-id="${colId}"]`));
    };
    const cells = [...apply(this.elements.thead), ...apply(this.elements.tbody)];
    cells.forEach((cell) => cell.classList.add("dt-col-justpinned"));
    window.setTimeout(() => {
      cells.forEach((cell) => cell.classList.remove("dt-col-justpinned"));
    }, 450);
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

    // Capture the previous column id set BEFORE swapping, so we can tell which
    // columns are genuinely new to this table (e.g. a view-specific Actions column
    // that only exists in some sub-tabs).
    const prevColumnIds = new Set((this.options.columns || []).map((col) => col.id));

    // Update columns
    this.options.columns = newColumns;

    if (resetState) {
      // Reset all column-related state
      this.state.columnWidths = {};
      this.state.visibleColumns = {};
      this.state.columnOrder = [];
      this.state.floatingColumns = [];
      this.state.userResizedColumns = {};
      this.state.hasAutoFitted = false;
      // Re-seed pinned columns from the new column defs' `floating: true` flags.
      this._seedFloatingColumns();
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

      // Remove deleted columns from the floating (pinned-left) group
      if (Array.isArray(this.state.floatingColumns)) {
        this.state.floatingColumns = this.state.floatingColumns.filter((colId) =>
          validColumnIds.has(colId)
        );
      }

      // Pin any genuinely-new column that defaults to `floating: true`. This makes
      // view-specific pinned columns (e.g. the Pool view's Actions column) honor
      // their default even though the table was first seeded in a view that lacked
      // them. Only columns NOT present in the previous set are added, so a column
      // the user explicitly unpinned within the same column set is never re-pinned.
      if (!Array.isArray(this.state.floatingColumns)) {
        this.state.floatingColumns = [];
      }
      const alreadyFloating = new Set(this.state.floatingColumns);
      for (const col of newColumns) {
        if (
          col &&
          col.floating === true &&
          !prevColumnIds.has(col.id) &&
          !alreadyFloating.has(col.id)
        ) {
          this._insertFloatingInDefinitionOrder(col.id);
          alreadyFloating.add(col.id);
        }
      }

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

    // The column SET/ORDER just changed (e.g. a sub-tab switch swapping columns).
    // Drop the rendered rows so the body is rebuilt from scratch against the new
    // structure — the in-place body diff reuses each row's existing <td> set and
    // would otherwise leave any surviving row (same id across views) with the old
    // columns' cells in the old order. Mirrors the reorder/pin paths.
    this._invalidateRenderedCells();

    // Re-render table structure with new columns
    if (preserveData && this.state.data.length > 0) {
      // Re-apply filters with new columns (some filter functions may reference column IDs)
      // Note: _applyFilters() already calls _renderTable() internally
      this._applyFilters();
    } else {
      // Not preserving data: drop the previous column set's rows so they can never
      // linger under the new columns (stale-row bug when switching sub-tabs). The
      // caller is expected to trigger a reload afterwards to populate the new view.
      if (!preserveData) {
        this.state.data = [];
        this.state.filteredData = [];
        this.state.hasAutoFitted = false;
        if (this._pagination?.enabled) {
          this._updatePaginationMeta(
            { cursorNext: null, cursorPrev: null, hasMoreNext: false, hasMorePrev: false },
            { replace: true }
          );
        }
      }
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
    if (this._windowResizeHandler) {
      window.removeEventListener("resize", this._windowResizeHandler);
      this._windowResizeHandler = null;
    }
    if (this._wrapperResizeObserver) {
      this._wrapperResizeObserver.disconnect();
      this._wrapperResizeObserver = null;
    }

    // Close any open header context menu and detach its document listeners
    this._closeHeaderColumnMenu();

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

// Apply mixins to extend the DataTable prototype
applyColumnManagementMixin(DataTable);
applyClientPaginationMixin(DataTable);
applyServerPaginationMixin(DataTable);
applyEventHandlersMixin(DataTable);
