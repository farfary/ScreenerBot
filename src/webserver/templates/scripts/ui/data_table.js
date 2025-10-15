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
    };

    this.elements = {};
    this.resizing = null;
    this.draggingColumn = null; // Track column being dragged
    this.documentClickHandler = null;
    this.scrollThrottle = null;
    this.eventHandlers = new Map(); // Store all event handlers for cleanup

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
   * Get columns in the correct order
   */
  _getOrderedColumns() {
    const visibleColumns = this.options.columns.filter((col) =>
      this._isColumnVisible(col.id)
    );

    // If no custom order, return original order
    if (this.state.columnOrder.length === 0) {
      return visibleColumns;
    }

    // Sort columns by custom order
    const ordered = [];
    const columnMap = new Map(visibleColumns.map((col) => [col.id, col]));

    // Add columns in saved order
    for (const colId of this.state.columnOrder) {
      if (columnMap.has(colId)) {
        ordered.push(columnMap.get(colId));
        columnMap.delete(colId);
      }
    }

    // Add any remaining columns (newly added)
    ordered.push(...columnMap.values());

    return ordered;
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
              draggable="true"
              style="width: ${
                typeof width === "number" ? width + "px" : width
              }; ${col.minWidth ? "min-width: " + col.minWidth + "px;" : ""}"
              class="${col.sortable ? "sortable" : ""} ${
              isSorted ? "sorted" : ""
            } dt-draggable-column"
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
    const visibleColumns = this._getOrderedColumns();

    return visibleColumns
      .map((col) => {
        let value = row[col.id];
        let cellContent = "";

        // Handle different column types
        if (col.type === "image" && col.image) {
          cellContent = this._renderImageCell(col, row);
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

        return `
        <td data-column-id="${col.id}" class="${col.className || ""} ${
          col.type === "image" ? "dt-image-cell" : ""
        }">
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

    // Column drag and drop for reordering
    const draggableHeaders = this.elements.thead.querySelectorAll(
      ".dt-draggable-column"
    );
    draggableHeaders.forEach((th) => {
      const dragStartHandler = (e) => {
        // Don't start drag if clicking on resize handle
        if (e.target.classList.contains("dt-resize-handle")) {
          e.preventDefault();
          return;
        }

        this.draggingColumn = th.dataset.columnId;
        e.dataTransfer.effectAllowed = "move";
        e.dataTransfer.setData("text/html", th.innerHTML);
        th.classList.add("dt-dragging");
      };

      const dragOverHandler = (e) => {
        if (e.preventDefault) {
          e.preventDefault();
        }
        e.dataTransfer.dropEffect = "move";

        const targetColumnId = e.currentTarget.dataset.columnId;
        if (this.draggingColumn && this.draggingColumn !== targetColumnId) {
          e.currentTarget.classList.add("dt-drag-over");
        }
        return false;
      };

      const dragLeaveHandler = (e) => {
        e.currentTarget.classList.remove("dt-drag-over");
      };

      const dropHandler = (e) => {
        if (e.stopPropagation) {
          e.stopPropagation();
        }
        e.preventDefault();

        const targetColumnId = e.currentTarget.dataset.columnId;

        if (this.draggingColumn && this.draggingColumn !== targetColumnId) {
          this._reorderColumn(this.draggingColumn, targetColumnId);
        }

        e.currentTarget.classList.remove("dt-drag-over");
        return false;
      };

      const dragEndHandler = (e) => {
        e.currentTarget.classList.remove("dt-dragging");
        draggableHeaders.forEach((header) => {
          header.classList.remove("dt-drag-over");
        });
        this.draggingColumn = null;
      };

      this._addEventListener(th, "dragstart", dragStartHandler);
      this._addEventListener(th, "dragover", dragOverHandler);
      this._addEventListener(th, "dragleave", dragLeaveHandler);
      this._addEventListener(th, "drop", dropHandler);
      this._addEventListener(th, "dragend", dragEndHandler);
    });

    // Column resizing
    const resizeHandles =
      this.elements.thead.querySelectorAll(".dt-resize-handle");
    resizeHandles.forEach((handle) => {
      const handler = (e) => {
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
   * Reorder columns by moving sourceId before/after targetId
   */
  _reorderColumn(sourceId, targetId) {
    // Get current order or create from visible columns
    let order = this.state.columnOrder;
    if (order.length === 0) {
      order = this._getOrderedColumns().map((col) => col.id);
    }

    // Find indices
    const sourceIndex = order.indexOf(sourceId);
    const targetIndex = order.indexOf(targetId);

    if (sourceIndex === -1 || targetIndex === -1) return;

    // Remove source
    order.splice(sourceIndex, 1);

    // Insert at new position (adjust if source was before target)
    const newTargetIndex =
      sourceIndex < targetIndex ? targetIndex : targetIndex + 1;
    order.splice(newTargetIndex, 0, sourceId);

    this.state.columnOrder = order;
    this._saveState();
    this._renderTable();

    this._log("info", "Column reordered", { sourceId, targetId, order });
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

    // Re-query elements after innerHTML update to ensure fresh references
    if (this.elements.container) {
      this.elements.thead = this.elements.container.querySelector("thead");
      this.elements.tbody = this.elements.container.querySelector("tbody");
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
    // Remove all event listeners
    this._removeEventListeners();

    // Clean up resize listeners
    if (this.resizing) {
      document.removeEventListener("mousemove", this._handleResize);
      document.removeEventListener("mouseup", this._handleResizeEnd);
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
