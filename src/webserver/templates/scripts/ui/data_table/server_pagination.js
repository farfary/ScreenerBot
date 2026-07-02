/**
 * Server Pagination Mixin for DataTable
 * Handles server-side pagination with scroll loading and page navigation modes
 */

import * as AppState from "../../core/app_state.js";

export function applyServerPaginationMixin(DataTable) {
  const proto = DataTable.prototype;

  /**
   * Load server pagination mode preference from AppState
   */
  proto._loadServerPaginationMode = function () {
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
  };

  /**
   * Save server pagination mode preference to AppState
   * @param {string} mode - 'scroll' or 'pages'
   */
  proto._saveServerPaginationMode = function (mode) {
    const stateKey = this._pagination?.modeStateKey;
    if (stateKey) {
      AppState.save(stateKey, mode);
    }
  };

  /**
   * Render the pagination mode toggle HTML for the toolbar
   * @returns {string} - HTML for the mode toggle
   */
  proto._renderPaginationModeToggle = function () {
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
  };

  /**
   * Attach event handlers for the pagination mode toggle
   */
  proto._attachModeToggleEvents = function () {
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
  };

  /**
   * Update the mode toggle button UI states
   */
  proto._updateModeToggleUI = function () {
    const toggle = this.elements.toolbar?.querySelector(".dt-pagination-mode-toggle");
    if (!toggle) {
      return;
    }

    const buttons = toggle.querySelectorAll(".dt-pagination-mode-btn");
    buttons.forEach((btn) => {
      const mode = btn.dataset.mode;
      btn.classList.toggle("active", mode === this._serverPaginationMode);
    });
  };

  /**
   * Switch between scroll and pages pagination modes
   * @param {string} newMode - 'scroll' or 'pages'
   */
  proto._switchPaginationMode = async function (newMode) {
    if (newMode === this._serverPaginationMode) {
      return;
    }

    // Cancel any pending pagination requests
    this._cancelPaginationRequest();

    this._serverPaginationMode = newMode;
    this._saveServerPaginationMode(newMode);
    this._scrollLoadingDisabled = newMode === "pages";
    // Hide the floating scroll loader if we just left scroll mode.
    this._updateScrollLoadingIndicator();

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
  };

  /**
   * Load a specific server page (for pages mode)
   * @param {number} pageNumber - Page number to load
   */
  proto._loadServerPage = async function (pageNumber, options = {}) {
    if (!this._pagination?.enabled) {
      return;
    }

    const pageSize = this.state.serverPaginationState.pageSize;
    const loadPage = this._pagination.loadPage;

    if (!loadPage) {
      return;
    }

    if (!options.silent) {
      const useSubtle = options.reason === "poll" || options.reason === "refresh";
      this._setLoadingState(true, { subtle: useSubtle });
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
  };

  /**
   * Change page size for server pagination (pages mode)
   * @param {number} newSize - New page size
   */
  proto._setServerPageSize = async function (newSize) {
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
  };

  /**
   * Navigate to a specific server page
   * @param {number|string} page - Page number or 'first', 'last', 'prev', 'next'
   */
  proto._goToServerPage = async function (page) {
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
  };

  /**
   * Render the server pagination bar HTML (for pages mode)
   * @returns {string} - HTML for pagination bar
   */
  proto._renderServerPaginationBar = function () {
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
  };

  /**
   * Generate smart page buttons with ellipsis for server pagination
   * @param {number} current - Current page
   * @param {number} total - Total pages
   * @returns {string} - HTML for page buttons
   */
  proto._generateServerPaginationButtons = function (current, total) {
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
  };

  /**
   * Attach event handlers for server pagination bar
   */
  proto._attachServerPaginationEvents = function () {
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
  };

  /**
   * Clean up server pagination event handlers before DOM replacement
   */
  proto._cleanupServerPaginationEvents = function () {
    const toRemove = [];
    this.eventHandlers.forEach((entry, key) => {
      if (entry.element?.closest?.(".dt-server-pagination-bar")) {
        entry.element.removeEventListener(entry.event, entry.handler);
        toRemove.push(key);
      }
    });
    toRemove.forEach((key) => this.eventHandlers.delete(key));
  };

  /**
   * Update the server pagination bar (re-render just the bar)
   */
  proto._updateServerPaginationBar = function () {
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
  };

  /**
   * Get current server pagination mode
   * @returns {string} - 'scroll' or 'pages'
   */
  proto.getServerPaginationMode = function () {
    return this._serverPaginationMode;
  };

  /**
   * Set server pagination mode programmatically
   * @param {string} mode - 'scroll' or 'pages'
   */
  proto.setServerPaginationMode = async function (mode) {
    if (mode !== "scroll" && mode !== "pages") {
      this._log("error", "Invalid pagination mode:", mode);
      return;
    }
    await this._switchPaginationMode(mode);
  };

  /**
   * Handle scroll event for pagination loading
   */
  proto._handlePaginationScroll = function () {
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
  };

  /**
   * Check if we should trigger pagination loading based on scroll position
   */
  proto._maybeTriggerPaginationLoad = function () {
    const pagination = this._pagination;
    const container = this.elements.scrollContainer;
    if (!pagination?.enabled || !container) {
      return;
    }

    // Skip scroll-based loading when in pages mode
    if (this._scrollLoadingDisabled) {
      return;
    }

    const scrollTop = container.scrollTop;
    const scrollable = Math.max(0, container.scrollHeight - container.clientHeight);
    const distanceToBottom = container.scrollHeight - (scrollTop + container.clientHeight);

    // Detect scroll direction. Next-page prefetch must only fire while moving DOWN
    // (or on the very first check) — otherwise scrolling UP inside the bottom region
    // keeps re-triggering loads (and flashing the loader) even though the user is
    // moving away from the end. `prev` loading has its own (top-edge) trigger below.
    const prevScrollTop = this._lastPrefetchScrollTop ?? scrollTop;
    const scrollingDown = scrollTop >= prevScrollTop;
    this._lastPrefetchScrollTop = scrollTop;

    // Prefetch margin = ratio of the scrollable range (start loading ~30% before the
    // end so rows are ready before the user arrives — smooth, no stall), but BOUNDED
    // to a few viewports. Without the cap, a large loaded set makes 30% an enormous
    // pixel distance, so the bottom third of everything counts as "near the end" and
    // loads fire continuously. The pixel `threshold` is the floor for short lists.
    const prefetchMargin = Math.max(
      pagination.threshold,
      Math.min(scrollable * (pagination.prefetchRatio ?? 0.3), container.clientHeight * 2)
    );

    if (
      pagination.hasMoreNext !== false &&
      !pagination.loadingNext &&
      scrollingDown &&
      distanceToBottom <= prefetchMargin
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
  };

  /**
   * Auto-load more data if viewport is short and has empty space
   */
  proto._autoLoadIfViewportShort = function () {
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
  };

  /**
   * Load a page of data (scroll mode)
   * @param {string} direction - 'prev', 'next', or 'initial'
   * @param {object} options - Load options
   * @returns {Promise} - Promise that resolves when page is loaded
   */
  proto._loadPage = function (direction, options = {}) {
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
      pagination.loadingNextReason = reason;
      this._updateScrollLoadingIndicator();
    } else if (normalizedDirection === "prev") {
      pagination.loadingPrev = true;
    } else {
      pagination.loadingInitial = true;
    }

    if (!silent && normalizedDirection === "initial") {
      const useSubtle = reason === "poll" || reason === "refresh";
      this._setLoadingState(true, { subtle: useSubtle });
    }

    let loadPromise;
    try {
      loadPromise = Promise.resolve(pagination.loadPage(loadArgs));
    } catch (error) {
      pagination.loadingNext = false;
      pagination.loadingNextReason = null;
      pagination.loadingPrev = false;
      pagination.loadingInitial = false;
      this._setLoadingState(false);
      this._log("error", "pagination.loadPage threw", error);
      return Promise.reject(error);
    }

    pagination.pendingRequest = loadPromise
      .then((result) => {
        // Discard results from a superseded request. If a newer load replaced this
        // one (e.g. the user switched sub-tabs / sort), our controller is no longer
        // the active one. Applying here would overwrite the current view's data with
        // a stale response — this is how the previous sub-tab's rows leak in when a
        // loadPage callback ignores the abort signal.
        if (pagination.abortController !== controller) {
          return result;
        }
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
        // If a newer request superseded this one (via _cancelPaginationRequest or a
        // fresh _loadPage), the abortController will no longer point at ours. In that
        // case this (aborted) request must NOT reset the shared loading / pending
        // state — doing so would turn the loading indicator OFF while the newer
        // request is still loading (intermittent "no loading shown" on sort/refresh).
        const isCurrent = pagination.abortController === controller;
        if (!isCurrent) {
          this._notifyPaginationStateChange();
          return;
        }
        pagination.abortController = null;
        if (normalizedDirection === "next") {
          pagination.loadingNext = false;
          pagination.loadingNextReason = null;
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
  };

  /**
   * Apply page result to table data
   * @param {string} direction - Direction of load
   * @param {object} result - Result from loadPage callback
   * @param {object} options - Additional options
   */
  proto._applyPageResult = function (direction, result, options = {}) {
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
  };

  /**
   * Normalize page result to standard format
   * @param {object|array} result - Result from loadPage callback
   * @returns {object} - Normalized result
   */
  proto._normalizePageResult = function (result) {
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
  };

  /**
   * Check if new data is effectively the same as current data
   * Uses fast JSON comparison for row equality
   * @param {Array} newRows - New data rows
   * @returns {boolean} - True if data is unchanged
   */
  proto._isDataUnchanged = function (newRows) {
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
      for (let i = 0; i < newRows.length; i++) {
        const oldRow = currentData[i];
        const newRow = newRows[i];

        // Check if row identity changed
        if (oldRow?.[rowKey] !== newRow?.[rowKey]) {
          return false;
        }

        // Compare every field, including nested objects/arrays. Render functions
        // frequently read nested data (e.g. row.health.status, row.metrics.*), so
        // skipping objects here would report "unchanged" and freeze the table even
        // though the live values changed. Nested values are compared by value via
        // JSON so a new fetch's fresh object references don't falsely flag change.
        const keys = Object.keys(newRow);
        for (const key of keys) {
          const val = newRow[key];
          if (val !== null && typeof val === "object") {
            const oldVal = oldRow?.[key];
            if (oldVal === val) continue;
            try {
              if (JSON.stringify(oldVal) !== JSON.stringify(val)) return false;
            } catch {
              return false;
            }
            continue;
          }
          if (oldRow?.[key] !== val) {
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
  };

  /**
   * Replace all data in table
   * @param {Array} rows - New rows
   * @param {object} meta - Metadata
   */
  proto._replaceData = function (rows, meta = {}) {
    const sanitized = Array.isArray(rows)
      ? rows.filter((row) => row !== null && row !== undefined)
      : [];

    // Check if data has actually changed to avoid unnecessary re-renders
    // This prevents DOM churn during polling when data is the same
    if (!meta.forceRender && this._isDataUnchanged(sanitized)) {
      // Still re-render periodically to refresh time-relative cells (e.g., "5s ago" text)
      const now = Date.now();
      const timeSinceRender = now - (this._lastFullRenderTime || 0);
      if (timeSinceRender < 5000) {
        this._log("debug", "Data unchanged, skipping re-render", { rows: sanitized.length });
        this._updatePaginationMeta(meta, { replace: true });
        this._setLoadingState(false);
        return;
      }
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
    this._lastFullRenderTime = Date.now();
    this._log("info", "Data replaced", { rows: sanitized.length });
  };

  /**
   * Append rows to table data
   * @param {Array} rows - Rows to append
   * @param {object} meta - Metadata
   */
  proto._appendData = function (rows, meta = {}) {
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
  };

  /**
   * Prepend rows to table data
   * @param {Array} rows - Rows to prepend
   * @param {object} meta - Metadata
   */
  proto._prependData = function (rows, meta = {}) {
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
  };

  /**
   * Prune rows if exceeds maxRows limit
   * @param {string} direction - 'prepend' or 'append'
   */
  proto._pruneRows = function (direction) {
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
  };

  /**
   * Update pagination metadata (cursors, hasMore flags)
   * @param {object} meta - Metadata
   * @param {object} options - Options
   */
  proto._updatePaginationMeta = function (meta = {}, options = {}) {
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
  };

  /**
   * Toggle the floating "Loading more…" indicator at the bottom of the table.
   * Only shown for scroll-mode next-page fetches (which are silent and otherwise
   * give no feedback). Pages mode keeps its own header-row loading bar, so this
   * stays hidden there to avoid double indicators.
   */
  proto._updateScrollLoadingIndicator = function () {
    const loader = this.elements?.scrollLoader;
    if (!loader) {
      return;
    }
    const pagination = this._pagination;
    const visible = Boolean(
      pagination?.enabled &&
        !this._scrollLoadingDisabled &&
        pagination.loadingNext &&
        pagination.loadingNextReason === "scroll"
    );
    loader.classList.toggle("is-visible", visible);
  };

  /**
   * Notify pagination state change callback
   */
  proto._notifyPaginationStateChange = function () {
    this._updateScrollLoadingIndicator();

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
          loadingNextReason: pagination.loadingNextReason ?? null,
          loadingPrev: Boolean(pagination.loadingPrev),
          total: pagination.total,
          meta: pagination.meta,
        },
        this
      );
    } catch (error) {
      this._log("error", "pagination.onStateChange failed", error);
    }
  };

  /**
   * Get row key for deduplication
   * @param {object} row - Row data
   * @returns {string|null} - Row key
   */
  proto._getRowKey = function (row) {
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
  };

  /**
   * Deduplicate rows based on row key
   * @param {Array} rows - Rows to dedupe
   * @param {string} direction - 'prepend' or 'append'
   * @returns {Array} - Deduplicated rows
   */
  proto._dedupeRows = function (rows, direction) {
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
  };

  /**
   * Create AbortController for cancellable requests
   * @returns {AbortController} - AbortController instance
   */
  proto._createAbortController = function () {
    if (typeof AbortController !== "undefined") {
      return new AbortController();
    }
    return {
      abort() {},
      signal: undefined,
    };
  };

  /**
   * Cancel pending pagination request
   */
  proto._cancelPaginationRequest = function () {
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
    this._pagination.loadingNextReason = null;
    this._pagination.loadingPrev = false;
    this._pagination.loadingInitial = false;

    // After cancelling there is no in-flight request, so clear the visible loading
    // indicator here. The aborted request's finally now skips this (it is no longer
    // the current request), so cancel owns turning it off. A reload that cancels as
    // its first step re-enables loading immediately afterwards.
    this._setLoadingState(false);
  };
}
