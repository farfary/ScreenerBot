/**
 * Client Pagination Mixin for DataTable
 * Handles client-side pagination with page controls
 */

import * as AppState from "../../core/app_state.js";

export function applyClientPaginationMixin(DataTable) {
  const proto = DataTable.prototype;

  /**
   * Get the slice of data for the current page (client pagination)
   * @returns {Array} - Data for current page
   */
  proto._getClientPaginatedData = function () {
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
  };

  /**
   * Recalculate total pages based on filtered data and page size
   */
  proto._recalculateClientPaginationPages = function () {
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
  };

  /**
   * Navigate to a specific page
   * @param {number|string} page - Page number or 'first', 'last', 'prev', 'next'
   */
  proto._goToClientPage = function (page) {
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
  };

  /**
   * Change items per page
   * @param {number|string} newSize - New page size or 'all'
   */
  proto._setClientPageSize = function (newSize) {
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
  };

  /**
   * Save page size preference
   */
  proto._saveClientPaginationPreference = function () {
    const { stateKey } = this.options.clientPagination;
    if (!stateKey) {
      return;
    }

    AppState.save(stateKey, {
      pageSize: this.state.clientPaginationState.pageSize,
    });
  };

  /**
   * Load saved page size preference
   */
  proto._loadClientPaginationPreference = function () {
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
  };

  /**
   * Load pagination enabled preference from AppState
   * @returns {boolean} - Whether pagination should be active
   */
  proto._loadPaginationEnabledPreference = function () {
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
  };

  /**
   * Save pagination enabled preference to AppState
   * @param {boolean} enabled - Whether pagination is enabled
   */
  proto._savePaginationEnabledPreference = function (enabled) {
    const { stateKey } = this.options.clientPagination;
    if (!stateKey) {
      return;
    }
    AppState.save(stateKey + "_enabled", enabled);
  };

  /**
   * Handle pagination toggle from settings dialog
   * @param {boolean} enabled - Whether pagination should be enabled
   */
  proto._handlePaginationToggle = function (enabled) {
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
  };

  /**
   * Render the client pagination bar HTML
   * @returns {string} - HTML for pagination bar
   */
  proto._renderClientPaginationBar = function () {
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
  };

  /**
   * Generate smart page buttons with ellipsis
   * @param {number} current - Current page
   * @param {number} total - Total pages
   * @returns {string} - HTML for page buttons
   */
  proto._generateClientPaginationButtons = function (current, total) {
    if (total <= 1) {
      return '<button class="dt-client-pagination-btn active" data-page="1">1</button>';
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
      buttons.push('<button class="dt-client-pagination-btn" data-page="1">1</button>');
      if (startPage > 2) {
        buttons.push('<span class="dt-client-pagination-ellipsis">…</span>');
      }
    }

    // Show page range
    for (let i = startPage; i <= endPage; i++) {
      const activeClass = i === current ? "active" : "";
      buttons.push(
        `<button class="dt-client-pagination-btn ${activeClass}" data-page="${i}">${i}</button>`
      );
    }

    // Always show last page
    if (endPage < total) {
      if (endPage < total - 1) {
        buttons.push('<span class="dt-client-pagination-ellipsis">…</span>');
      }
      buttons.push(
        `<button class="dt-client-pagination-btn" data-page="${total}">${total}</button>`
      );
    }

    return buttons.join("");
  };

  /**
   * Attach pagination event handlers
   */
  proto._attachClientPaginationEvents = function () {
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
  };

  /**
   * Clean up pagination event handlers before DOM replacement
   */
  proto._cleanupClientPaginationEvents = function () {
    // Remove handlers for elements within the pagination bar
    const toRemove = [];
    this.eventHandlers.forEach((entry, key) => {
      if (entry.element?.closest?.(".dt-client-pagination-bar")) {
        entry.element.removeEventListener(entry.event, entry.handler);
        toRemove.push(key);
      }
    });
    toRemove.forEach((key) => this.eventHandlers.delete(key));
  };

  /**
   * Update the pagination bar (re-render just the bar)
   */
  proto._updateClientPaginationBar = function () {
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
  };

  /**
   * Check if hybrid pagination mode toggle is enabled
   * @returns {boolean}
   */
  proto._hasHybridPaginationModes = function () {
    const modes = this._pagination?.modes;
    return Array.isArray(modes) && modes.length >= 2;
  };
}
