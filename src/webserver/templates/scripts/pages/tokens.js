import { registerPage } from "../core/lifecycle.js";
import { openMenu, closeMenu } from "../core/menu_manager.js";
import { Poller, getInterval as getGlobalPollInterval } from "../core/poller.js";
import { requestManager } from "../core/request_manager.js";
import * as Utils from "../core/utils.js";
import { DataTable } from "../ui/data_table.js";
import { TabBar, TabBarManager } from "../ui/tab_bar.js";
import { manualTrade } from "../ui/manual_trade.js";
import { TokenDetailsDialog } from "../ui/token_details_dialog.js";
import { showFeaturedRow, hideFeaturedRow } from "../ui/featured_row.js";
import { BOOSTS_CHANGED_EVENT, boostRowClass, ensureBoosts } from "../core/boosts.js";
import { showImageLightbox } from "../ui/image_lightbox.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { InputDialog } from "../ui/input_dialog.js";

// Import sub-modules
import {
  TOKEN_VIEWS,
  DEFAULT_VIEW,
  DEFAULT_SERVER_SORT,
  DEFAULT_FILTERS,
  DEFAULT_SUMMARY,
  PAGE_LIMIT,
  PRICE_HIGHLIGHT_DURATION_MS,
  getDefaultFiltersForView,
  getServerSortKey,
  getTokensTableStateKey,
} from "./tokens/constants.js";
import {
  findFirstDifferenceIndex,
  priceCell,
  usdCell,
  percentCell,
  timeAgoCell,
  getRejectionDisplayLabel,
  tokenCell,
  normalizeBlacklistReasons,
  summarizeBlacklistReasons,
  resolveSortColumn,
  normalizeSortDirection,
  loadPersistedSort,
  attachHintsToTabs,
} from "./tokens/formatters.js";
import { createOhlcvModule } from "./tokens/ohlcv.js";
import { createFavoritesModule } from "./tokens/favorites.js";

function createLifecycle() {
  let table = null;
  let ohlcvTable = null; // Separate table for OHLCV data view
  let poller = null;
  let ohlcvPoller = null; // Separate poller for OHLCV data
  let tabBar = null;
  let tokenDetailsDialog = null;
  let walletBalance = 0;

  // Event cleanup tracking
  const eventCleanups = [];

  // Boosts arrive on their own schedule, so both token tables re-mark their rows
  // when the shared feed changes instead of waiting for the next data poll.
  const onBoostsChanged = () => {
    table?.repaintRows();
    deps.favoritesTable?.repaintRows();
  };

  const priceHistory = new Map();
  let priceBaselineReady = false;

  // OHLCV state
  const ohlcvState = {
    tokens: [],
    stats: null,
    isLoading: false,
  };

  // Favorites state
  let favoritesTable = null;
  const favoritesState = {
    favorites: [],
    isLoading: false,
  };

  const state = {
    view: DEFAULT_VIEW,
    search: "",
    totalCount: null,
    lastUpdate: null,
    sort: { ...DEFAULT_SERVER_SORT },
    filters: getDefaultFiltersForView(DEFAULT_VIEW),
    summary: { ...DEFAULT_SUMMARY },
    availableRejectionReasons: [],
    hasLoadedOnce: false,
  };

  // Create modules dependencies object (mutable refs to share state)
  const deps = {
    state,
    ohlcvState,
    favoritesState,
    ohlcvTable: null,
    favoritesTable: null,
    poller: null,
    ohlcvPoller: null,
    requestManager,
    DataTable,
    Utils,
    Poller,
    ConfirmationDialog,
    InputDialog,
  };

  // Initialize sub-modules
  const ohlcv = createOhlcvModule(deps);
  const favorites = createFavoritesModule(deps);

  const resetPriceTracking = () => {
    priceHistory.clear();
    priceBaselineReady = false;
    if (table?.getData) {
      const currentRows = table.getData();
      if (Array.isArray(currentRows)) {
        currentRows.forEach((row) => {
          if (
            row &&
            typeof row === "object" &&
            Object.prototype.hasOwnProperty.call(row, "price_change_meta")
          ) {
            row.price_change_meta = null;
          }
        });
      }
    }
  };

  const annotatePriceChange = (row) => {
    if (!row || typeof row !== "object" || state.view !== "pool") {
      return row;
    }

    const mint = row.mint;
    if (!mint) {
      row.price_change_meta = null;
      return row;
    }

    const numericPrice = Number(row.price_sol);
    if (!Number.isFinite(numericPrice)) {
      priceHistory.delete(mint);
      row.price_change_meta = null;
      return row;
    }

    const formattedCurrent = Utils.formatPriceSol(numericPrice, { fallback: "", decimals: 12 });
    const now = Date.now();
    const record = priceHistory.get(mint);

    if (
      priceBaselineReady &&
      record &&
      Number.isFinite(record.lastPrice) &&
      numericPrice !== record.lastPrice
    ) {
      const direction = numericPrice > record.lastPrice ? "up" : "down";
      const previousFormatted = record.lastFormatted ?? "";
      const changeStartIndex = findFirstDifferenceIndex(previousFormatted, formattedCurrent);
      if (direction && changeStartIndex !== -1) {
        const meta = {
          direction,
          currentFormatted: formattedCurrent,
          changeStartIndex,
        };
        priceHistory.set(mint, {
          lastPrice: numericPrice,
          lastFormatted: formattedCurrent,
          highlightUntil: now + PRICE_HIGHLIGHT_DURATION_MS,
          meta,
        });
        row.price_change_meta = meta;
        return row;
      }
    }

    if (record && record.highlightUntil > now && record.meta) {
      const refreshedMeta = {
        ...record.meta,
        currentFormatted: formattedCurrent,
      };
      priceHistory.set(mint, {
        lastPrice: numericPrice,
        lastFormatted: formattedCurrent,
        highlightUntil: record.highlightUntil,
        meta: refreshedMeta,
      });
      row.price_change_meta = refreshedMeta;
      return row;
    }

    priceHistory.set(mint, {
      lastPrice: numericPrice,
      lastFormatted: formattedCurrent,
      highlightUntil: 0,
      meta: null,
    });
    row.price_change_meta = null;
    return row;
  };

  const parseToggleValue = (value, fallback = false) => {
    if (typeof value === "boolean") {
      return value;
    }
    if (typeof value === "string") {
      const normalized = value.trim().toLowerCase();
      if (normalized === "true" || normalized === "1" || normalized === "on") {
        return true;
      }
      if (normalized === "false" || normalized === "0" || normalized === "off") {
        return false;
      }
    }
    return fallback;
  };

  // The reject-reason filter only makes sense in the "rejected" view.
  const updateFilterVisibility = () => {
    table?.setToolbarItem("rejection_reason", { hidden: state.view !== "rejected" });
  };

  const updateRejectionReasonOptions = () => {
    if (!table?.elements?.toolbar) {
      return;
    }

    const select = table.elements.toolbar.querySelector(
      '.dt-filter[data-filter-id="rejection_reason"]'
    );
    if (!select) {
      return;
    }

    const reasons = Array.isArray(state.availableRejectionReasons)
      ? Array.from(new Set(state.availableRejectionReasons.filter((item) => item && item.trim())))
      : [];

    // Sort by display label for user-friendly ordering
    reasons.sort((a, b) => {
      const labelA = getRejectionDisplayLabel(a) || a;
      const labelB = getRejectionDisplayLabel(b) || b;
      return labelA.toLowerCase().localeCompare(labelB.toLowerCase());
    });

    const currentValue = state.filters.rejection_reason || "all";

    // Build options array for CustomSelect with human-readable labels
    const newOptions = [
      { value: "all", label: "All" },
      ...reasons.map((reason) => ({
        value: reason,
        label: getRejectionDisplayLabel(reason) || reason,
      })),
    ];

    // Check if the select has a CustomSelect instance attached
    if (
      select._customSelectInstance &&
      typeof select._customSelectInstance.setOptions === "function"
    ) {
      select._customSelectInstance.setOptions(newOptions);
      select._customSelectInstance.setValue(currentValue);
    } else {
      // Fallback: update native select options
      const optionMarkup = [
        '<option value="all">All</option>',
        ...reasons.map((reason) => {
          const escaped = Utils.escapeHtml(reason);
          const label = Utils.escapeHtml(getRejectionDisplayLabel(reason) || reason);
          return `<option value="${escaped}">${label}</option>`;
        }),
      ].join("");

      if (select.innerHTML !== optionMarkup) {
        select.innerHTML = optionMarkup;
      }
      select.value = currentValue;
    }

    const normalizedCurrent = reasons.some((reason) => reason === currentValue)
      ? currentValue
      : "all";

    if (normalizedCurrent !== currentValue) {
      state.filters.rejection_reason = normalizedCurrent;
    }
  };

  const applyViewPreferences = () => {
    if (!table) return;
    const showRejectReason = state.view === "rejected";
    if (table.state && typeof table.state === "object") {
      table.state.visibleColumns = table.state.visibleColumns || {};
      table.state.visibleColumns.reject_reason = showRejectReason;
    }
    const column = Array.isArray(table.options?.columns)
      ? table.options.columns.find((col) => col.id === "reject_reason")
      : null;
    if (column) {
      column.visible = showRejectReason;
    }
    if (typeof table._renderTable === "function") {
      table._renderTable();
    }
    updateFilterVisibility();
    updateRejectionReasonOptions();
  };

  const updateToolbar = () => {
    if (!table) return;
    const rows = table.getData();

    const loaded = rows.length;
    const totalGlobal =
      typeof state.totalCount === "number" && Number.isFinite(state.totalCount)
        ? state.totalCount
        : loaded;

    const summaryPriced =
      typeof state.summary?.priced === "number" && Number.isFinite(state.summary.priced)
        ? state.summary.priced
        : rows.filter((r) => r.has_pool_price).length;
    const summaryPositions =
      typeof state.summary?.positions === "number" && Number.isFinite(state.summary.positions)
        ? state.summary.positions
        : rows.filter((r) => r.has_open_position).length;
    const summaryBlacklisted =
      typeof state.summary?.blacklisted === "number" && Number.isFinite(state.summary.blacklisted)
        ? state.summary.blacklisted
        : rows.filter((r) => r.blacklisted).length;

    table.updateToolbarSummary([
      {
        id: "tokens-total",
        label: "Total",
        value: Utils.formatNumber(totalGlobal, 0),
      },
      {
        id: "tokens-priced",
        label: "With Price",
        value: Utils.formatNumber(summaryPriced, 0),
        variant: "info",
      },
      {
        id: "tokens-positions",
        label: "Positions",
        value: Utils.formatNumber(summaryPositions, 0),
        variant: summaryPositions > 0 ? "success" : "secondary",
      },
      {
        id: "tokens-blacklisted",
        label: "Blacklisted",
        value: Utils.formatNumber(summaryBlacklisted, 0),
        variant: summaryBlacklisted > 0 ? "warning" : "success",
      },
    ]);
  };

  const buildQuery = ({ cursor = null, page = null, pageSize = null } = {}) => {
    const params = new URLSearchParams();
    params.set("view", state.view);
    if (state.search) params.set("search", state.search);
    const sort = state.sort ?? DEFAULT_SERVER_SORT;
    const sortBy = sort?.by ?? DEFAULT_SERVER_SORT.by;
    const sortDir = sort?.direction ?? DEFAULT_SERVER_SORT.direction;
    const sortColumnId = resolveSortColumn(sortBy) ?? sortBy;
    // Use view-aware sort key resolver with column identifier
    const serverSortKey = getServerSortKey(sortColumnId, state.view);
    params.set("sort_by", serverSortKey);
    params.set("sort_dir", sortDir);
    if (state.filters.pool_price) {
      params.set("has_pool_price", "true");
    }
    if (state.filters.positions) {
      params.set("has_open_position", "true");
    }
    if (
      state.view === "rejected" &&
      state.filters.rejection_reason &&
      state.filters.rejection_reason !== "all"
    ) {
      params.set("rejection_reason", state.filters.rejection_reason);
    }
    // Page-based pagination (for 'pages' mode)
    if (page !== null && page !== undefined) {
      params.set("page", String(page));
      params.set("limit", String(pageSize || PAGE_LIMIT));
    } else {
      // Cursor-based pagination (for 'scroll' mode)
      params.set("limit", String(PAGE_LIMIT));
      if (cursor !== null && cursor !== undefined) {
        params.set("cursor", String(cursor));
      }
    }
    return params;
  };

  // Prevent poll reloads from racing with user actions (sort/page/search/etc.)
  // Poll reload uses DataTable.reload(), which cancels in-flight pagination requests.
  // If poll fires during a user-triggered load, it can repeatedly abort and re-queue
  // requests, making even small page sizes feel "stuck".
  let lastUserReloadAt = 0;

  const shouldSkipPollReload = () => {
    if (!table) return false;

    // In server-backed "Pages" mode, reduce poll frequency instead of blocking entirely.
    // Pages mode refetches the current page, but _isDataUnchanged prevents DOM churn.
    if (
      typeof table.getServerPaginationMode === "function" &&
      table.getServerPaginationMode() === "pages"
    ) {
      // Allow poll every 5s in pages mode (vs 1s in scroll mode)
      if (
        !shouldSkipPollReload._lastPagesPoll ||
        Date.now() - shouldSkipPollReload._lastPagesPoll >= 5000
      ) {
        shouldSkipPollReload._lastPagesPoll = Date.now();
        return false;
      }
      return true;
    }

    // If the table is already loading, never start a poll reload.
    // This avoids poll->abort->reload loops that delay UI updates.
    if (table.state?.isLoading) {
      return true;
    }

    // In scroll mode a poll reload re-fetches only the FIRST page (cursor 0) and
    // replaces the whole dataset — fine at the top (newly-updated tokens bubble in),
    // but destructive once the user has scrolled deep: it would collapse the loaded
    // window (e.g. 2000 rows) back to one page and yank the scroll position. The
    // backend caps a single request at 200 rows, so the tail can't be refreshed in
    // one shot anyway. So once scrolled past the first screen, skip the reset and
    // leave the loaded rows stable; live top-of-list refresh resumes at the top.
    const sc = table?.elements?.scrollContainer;
    const loadedCount = table.getData?.().length ?? 0;
    if (sc && loadedCount > PAGE_LIMIT && sc.scrollTop > sc.clientHeight) {
      return true;
    }

    // Skip polls shortly after a user-triggered reload (sort/search/page size/view).
    // Keeps the UI responsive by letting the latest user action complete.
    if (lastUserReloadAt && Date.now() - lastUserReloadAt < 2500) {
      return true;
    }

    const paginationState =
      typeof table.getPaginationState === "function" ? table.getPaginationState() : null;
    if (
      paginationState?.loadingNext ||
      paginationState?.loadingPrev ||
      paginationState?.loadingInitial
    ) {
      return true;
    }

    // Skip if links dropdown menu is open
    const linksDropdown = document.querySelector(".links-dropdown-menu");
    if (linksDropdown) {
      return true;
    }

    // Skip while a control inside the table holds user state a reload would destroy: a focused
    // text input, or an open/focused select (incl. the page-size select).
    //
    // Hovering, and a focused BUTTON, deliberately do NOT skip the poll any more. They used to,
    // and because this guard gates the FETCH (not just the render), the table stopped receiving
    // data at all: resting the pointer on it, or merely having clicked any button in it once
    // (Chromium leaves a clicked button focused), froze every value until the pointer moved
    // away — with no visible cue, since focus rings are suppressed for mouse input. Neither
    // needs the fetch stopped: DataTable applies value-only updates in place while hovering
    // (nothing moves under the cursor), and a focused button carries nothing to lose.
    const container = table?.elements?.container;
    if (container) {
      const focusedElement = document.activeElement;
      if (focusedElement && container.contains(focusedElement)) {
        const tagName = focusedElement.tagName?.toLowerCase();
        if (tagName === "input" || tagName === "select" || tagName === "textarea") {
          return true;
        }
      }
    }

    return false;
  };

  const syncTableSortState = (options = {}) => {
    if (!table) {
      return;
    }
    const columnId = resolveSortColumn(state.sort.by);
    table.setSortState(columnId, state.sort.direction, options);
  };

  const syncToolbarFilters = () => {
    if (!table) {
      return;
    }
    table.setToolbarFilterValue("rejection_reason", state.filters.rejection_reason, {
      apply: false,
    });
    updateFilterVisibility();
    updateRejectionReasonOptions();
  };

  const handleSortChange = ({ column, direction, restored }) => {
    const sortKey = getServerSortKey(column, state.view);
    if (!sortKey) {
      syncTableSortState({ render: true });
      return;
    }

    const nextDirection = normalizeSortDirection(direction);
    state.sort = { by: sortKey, direction: nextDirection };
    state.totalCount = null;
    state.lastUpdate = null;
    updateToolbar();

    // For restored state, we still need to load data, but silently
    requestReload(restored ? "restored" : "sort", {
      silent: false,
      resetScroll: true,
    }).catch(() => {});
  };

  const loadTokensPage = async ({
    direction = "initial",
    cursor,
    page,
    pageSize,
    reason,
    signal,
  }) => {
    // Handle page-based pagination (direction === 'page')
    if (direction === "page" && page !== undefined) {
      return loadTokensPageBased({ page, pageSize, reason, signal });
    }

    if (direction === "prev") {
      const currentTotal = state.totalCount ?? table?.getData?.().length ?? 0;
      return {
        rows: [],
        cursorPrev: null,
        hasMorePrev: false,
        total: currentTotal,
        meta: { timestamp: state.lastUpdate },
        preserveScroll: true,
      };
    }

    if (!state.hasLoadedOnce && reason !== "poll" && table?.showBlockingState) {
      table.showBlockingState({
        variant: "loading",
        title: "Loading tokens snapshot...",
        description: "Large token databases can take a few seconds to warm up during startup.",
      });
    }

    const params = buildQuery({ cursor });
    const url = `/api/tokens/list?${params.toString()}`;

    try {
      const requestPriority = reason === "poll" ? "normal" : "high";
      const data = await requestManager.fetch(url, {
        priority: requestPriority,
        signal,
        skipQueue: reason !== "poll",
      });
      const items = Array.isArray(data?.items) ? data.items : [];

      const rejectionReasons =
        data &&
        typeof data === "object" &&
        data.rejection_reasons &&
        typeof data.rejection_reasons === "object"
          ? data.rejection_reasons
          : null;

      const blacklistSourcesMap =
        data &&
        typeof data === "object" &&
        data.blacklist_reasons &&
        typeof data.blacklist_reasons === "object"
          ? data.blacklist_reasons
          : null;

      const normalizedItems = items.map((row) => {
        if (!row || typeof row !== "object") return row;
        const hasServerReason =
          rejectionReasons &&
          row.mint &&
          Object.prototype.hasOwnProperty.call(rejectionReasons, row.mint);
        const resolvedReason = hasServerReason
          ? rejectionReasons[row.mint]
          : (row.reject_reason ?? null);
        const normalizedSources = normalizeBlacklistReasons(row.mint, blacklistSourcesMap);
        return {
          ...row,
          reject_reason: resolvedReason ?? null,
          blacklist_reasons: normalizedSources,
        };
      });

      let rowsWithPriceMeta = normalizedItems;
      if (state.view === "pool") {
        rowsWithPriceMeta = normalizedItems.map((row) => annotatePriceChange(row));
        if (!priceBaselineReady) {
          priceBaselineReady = true;
        }
      } else if (priceHistory.size > 0) {
        resetPriceTracking();
      }

      if (Array.isArray(data?.available_rejection_reasons)) {
        state.availableRejectionReasons = data.available_rejection_reasons.filter(
          (reason) => typeof reason === "string" && reason.trim().length > 0
        );
      } else {
        state.availableRejectionReasons = [];
      }

      if (state.view === "rejected") {
        updateRejectionReasonOptions();
        if (table) {
          table.setToolbarFilterValue("rejection_reason", state.filters.rejection_reason, {
            apply: false,
          });
        }
      }

      if (typeof data?.total === "number" && Number.isFinite(data.total)) {
        state.totalCount = data.total;
      } else {
        state.totalCount = null;
      }

      state.lastUpdate = data?.timestamp ?? null;
      const pricedTotal =
        typeof data?.priced_total === "number" && Number.isFinite(data.priced_total)
          ? data.priced_total
          : null;
      const positionsTotal =
        typeof data?.positions_total === "number" && Number.isFinite(data.positions_total)
          ? data.positions_total
          : null;
      const blacklistedTotal =
        typeof data?.blacklisted_total === "number" && Number.isFinite(data.blacklisted_total)
          ? data.blacklisted_total
          : null;

      state.summary = {
        priced:
          pricedTotal !== null
            ? pricedTotal
            : rowsWithPriceMeta.filter((row) => row.has_pool_price).length,
        positions:
          positionsTotal !== null
            ? positionsTotal
            : rowsWithPriceMeta.filter((row) => row.has_open_position).length,
        blacklisted:
          blacklistedTotal !== null
            ? blacklistedTotal
            : rowsWithPriceMeta.filter((row) => row.blacklisted).length,
      };

      state.hasLoadedOnce = true;
      if (table?.hideBlockingState) {
        table.hideBlockingState();
      }

      return {
        rows: rowsWithPriceMeta,
        cursorNext: data?.next_cursor ?? null,
        cursorPrev: null,
        hasMoreNext: Boolean(data?.next_cursor),
        hasMorePrev: false,
        total: state.totalCount ?? items.length,
        meta: { timestamp: state.lastUpdate },
        preserveScroll: reason === "poll",
      };
    } catch (error) {
      if (error?.name === "AbortError") {
        throw error;
      }
      console.error("[Tokens] Failed to load tokens list:", error);
      if (!state.hasLoadedOnce) {
        table?.showBlockingState?.({
          variant: "error",
          title: "Still loading tokens...",
          description: "Waiting for the backend to respond. We will retry automatically.",
        });
      } else if (reason !== "poll") {
        Utils.showToast("Failed to load tokens", "warning");
      }
      throw error;
    }
  };

  /**
   * Load tokens using page-based pagination (for 'pages' mode).
   * Returns data in a format DataTable expects for server page navigation.
   */
  const loadTokensPageBased = async ({ page, pageSize, reason, signal }) => {
    if (!state.hasLoadedOnce && reason !== "poll" && table?.showBlockingState) {
      table.showBlockingState({
        variant: "loading",
        title: "Loading tokens page...",
        description: "Fetching page data from server.",
      });
    }

    const params = buildQuery({ page, pageSize });
    const url = `/api/tokens/list?${params.toString()}`;

    try {
      const requestPriority = reason === "poll" ? "normal" : "high";
      const data = await requestManager.fetch(url, {
        priority: requestPriority,
        signal,
      });
      const items = Array.isArray(data?.items) ? data.items : [];

      const rejectionReasons =
        data &&
        typeof data === "object" &&
        data.rejection_reasons &&
        typeof data.rejection_reasons === "object"
          ? data.rejection_reasons
          : null;

      const blacklistSourcesMap =
        data &&
        typeof data === "object" &&
        data.blacklist_reasons &&
        typeof data.blacklist_reasons === "object"
          ? data.blacklist_reasons
          : null;

      const normalizedItems = items.map((row) => {
        if (!row || typeof row !== "object") return row;
        const hasServerReason =
          rejectionReasons &&
          row.mint &&
          Object.prototype.hasOwnProperty.call(rejectionReasons, row.mint);
        const resolvedReason = hasServerReason
          ? rejectionReasons[row.mint]
          : (row.reject_reason ?? null);
        const normalizedSources = normalizeBlacklistReasons(row.mint, blacklistSourcesMap);
        return {
          ...row,
          reject_reason: resolvedReason ?? null,
          blacklist_reasons: normalizedSources,
        };
      });

      let rowsWithPriceMeta = normalizedItems;
      if (state.view === "pool") {
        rowsWithPriceMeta = normalizedItems.map((row) => annotatePriceChange(row));
        if (!priceBaselineReady) {
          priceBaselineReady = true;
        }
      }

      // Update available rejection reasons
      if (Array.isArray(data?.available_rejection_reasons)) {
        state.availableRejectionReasons = data.available_rejection_reasons.filter(
          (r) => typeof r === "string" && r.trim().length > 0
        );
      }

      if (state.view === "rejected") {
        updateRejectionReasonOptions();
        if (table) {
          table.setToolbarFilterValue("rejection_reason", state.filters.rejection_reason, {
            apply: false,
          });
        }
      }

      // Update totals
      if (typeof data?.total === "number" && Number.isFinite(data.total)) {
        state.totalCount = data.total;
      }

      state.lastUpdate = data?.timestamp ?? null;
      const pricedTotal = typeof data?.priced_total === "number" ? data.priced_total : null;
      const positionsTotal =
        typeof data?.positions_total === "number" ? data.positions_total : null;
      const blacklistedTotal =
        typeof data?.blacklisted_total === "number" ? data.blacklisted_total : null;

      state.summary = {
        priced:
          pricedTotal !== null
            ? pricedTotal
            : rowsWithPriceMeta.filter((row) => row.has_pool_price).length,
        positions:
          positionsTotal !== null
            ? positionsTotal
            : rowsWithPriceMeta.filter((row) => row.has_open_position).length,
        blacklisted:
          blacklistedTotal !== null
            ? blacklistedTotal
            : rowsWithPriceMeta.filter((row) => row.blacklisted).length,
      };

      state.hasLoadedOnce = true;
      if (table?.hideBlockingState) {
        table.hideBlockingState();
      }

      // Return page-based response with serverPage info
      return {
        rows: rowsWithPriceMeta,
        total: state.totalCount ?? items.length,
        meta: { timestamp: state.lastUpdate },
        // Server page info for DataTable
        serverPage: {
          page: data?.page ?? page,
          pageSize: data?.page_size ?? pageSize,
          totalPages: data?.total_pages ?? Math.ceil((state.totalCount || items.length) / pageSize),
        },
      };
    } catch (error) {
      if (error?.name === "AbortError") {
        throw error;
      }
      console.error("[Tokens] Failed to load tokens page:", error);
      if (!state.hasLoadedOnce) {
        table?.showBlockingState?.({
          variant: "error",
          title: "Still loading tokens...",
          description: "Waiting for the backend to respond. We will retry automatically.",
        });
      } else if (reason !== "poll") {
        Utils.showToast("Failed to load tokens page", "warning");
      }
      throw error;
    }
  };

  const requestReload = (reason = "manual", options = {}) => {
    if (!table) return Promise.resolve(null);
    if (reason === "poll" && shouldSkipPollReload()) {
      return Promise.resolve(null);
    }

    if (reason !== "poll") {
      lastUserReloadAt = Date.now();
    }

    return table.reload({
      reason,
      silent: options.silent ?? false,
      preserveScroll: options.preserveScroll ?? false,
      resetScroll: options.resetScroll ?? false,
    });
  };

  /**
   * Manual trade (buy / add / sell) for every token-list table AND the favorites
   * table. Delegates to the shared manual-trade flow (ui/manual_trade.js), which owns
   * the dialog, the payload and the toasts; this only supplies the row's context and
   * reloads the view afterwards.
   * @param {{action:string, mint:string, row?:object, btn?:HTMLElement, onReload?:Function}} opts
   */
  const performManualTrade = async ({ action, mint, row = {}, btn = null, onReload }) => {
    if (!action || !mint) return;

    const symbol = row.symbol || "?";
    const context = {};

    if (action === "add") {
      // DCA presets for the add dialog's size buttons.
      let entrySizes = [0.005, 0.01, 0.02, 0.05];
      try {
        const configData = await requestManager.fetch("/api/config/trader", {
          priority: "normal",
        });
        if (Array.isArray(configData?.data?.entry_sizes)) {
          entrySizes = configData.data.entry_sizes;
        }
      } catch (err) {
        console.warn("Failed to fetch entry_sizes config:", err);
      }
      context.entrySize = row.entry_sol || 0.005;
      context.entrySizes = entrySizes;
    }

    const placed = await manualTrade({
      action,
      mint,
      symbol,
      btn,
      context,
      balance: walletBalance,
      holdings: row.token_amount,
      decimals: row.decimals,
    });

    if (placed && typeof onReload === "function") onReload();
  };

  // Expose the shared trade handler to sub-modules (favorites table reuses it).
  deps.performManualTrade = performManualTrade;

  /**
   * Build columns array based on current view
   * Different views show different conditional columns (Actions, reject_reason, blacklist_reason)
   */
  const buildColumns = () => {
    return [
      {
        id: "token",
        label: "Token",
        sortable: true,
        floating: true,
        minWidth: 260,
        wrap: false,
        render: (_v, row) => tokenCell(row),
      },
      {
        id: "price_sol",
        label: "Price (SOL)",
        sortable: true,
        minWidth: 120,
        wrap: false,
        render: (v, row) => priceCell(v, row),
      },
      {
        id: "liquidity_usd",
        label: "Liquidity",
        sortable: true,
        minWidth: 110,
        wrap: false,
        render: (v) => usdCell(v),
      },
      {
        id: "volume_24h",
        label: "24h Vol",
        sortable: true,
        minWidth: 110,
        wrap: false,
        render: (v) => usdCell(v),
      },
      {
        id: "fdv",
        label: "FDV",
        sortable: true,
        minWidth: 110,
        wrap: false,
        render: (v) => usdCell(v),
      },
      {
        id: "market_cap",
        label: "Mkt Cap",
        sortable: true,
        minWidth: 110,
        wrap: false,
        render: (v) => usdCell(v),
      },
      {
        id: "price_change_h1",
        label: "1h",
        sortable: true,
        minWidth: 90,
        wrap: false,
        render: (v) => percentCell(v),
      },
      {
        id: "price_change_h24",
        label: "24h",
        sortable: true,
        minWidth: 90,
        wrap: false,
        render: (v) => percentCell(v),
      },
      {
        id: "txns_5m",
        label: "Txns 5m",
        sortable: true,
        minWidth: 80,
        wrap: false,
        render: (_v, row) => {
          const buys = row.txns_5m_buys || 0;
          const sells = row.txns_5m_sells || 0;
          if (buys === 0 && sells === 0) return "—";
          return `${Utils.formatNumber(buys, 0)}/${Utils.formatNumber(sells, 0)}`;
        },
      },
      {
        id: "txns_1h",
        label: "Txns 1h",
        sortable: true,
        minWidth: 80,
        wrap: false,
        render: (_v, row) => {
          const buys = row.txns_1h_buys || 0;
          const sells = row.txns_1h_sells || 0;
          if (buys === 0 && sells === 0) return "—";
          return `${Utils.formatNumber(buys, 0)}/${Utils.formatNumber(sells, 0)}`;
        },
      },
      {
        id: "txns_6h",
        label: "Txns 6h",
        sortable: true,
        minWidth: 80,
        wrap: false,
        render: (_v, row) => {
          const buys = row.txns_6h_buys || 0;
          const sells = row.txns_6h_sells || 0;
          if (buys === 0 && sells === 0) return "—";
          return `${Utils.formatNumber(buys, 0)}/${Utils.formatNumber(sells, 0)}`;
        },
      },
      {
        id: "txns_24h",
        label: "Txns 24h",
        sortable: true,
        minWidth: 90,
        wrap: false,
        render: (_v, row) => {
          const buys = row.txns_24h_buys || 0;
          const sells = row.txns_24h_sells || 0;
          if (buys === 0 && sells === 0) return "—";
          return `${Utils.formatNumber(buys, 0)}/${Utils.formatNumber(sells, 0)}`;
        },
      },
      {
        id: "risk_score",
        label: "Risk Score",
        sortable: true,
        minWidth: 90,
        wrap: false,
        render: (v) => {
          if (v == null) return "—";
          // Raw rugcheck score: lower = safer, higher = more risky
          const num = Utils.formatNumber(v, 0);
          if (v <= 1000) return `<span style="color: var(--success)">${num}</span>`;
          if (v <= 10000) return `<span style="color: var(--warning)">${num}</span>`;
          return `<span style="color: var(--error)">${num}</span>`;
        },
      },
      // Conditionally add reject_reason column only for rejected view
      ...(state.view === "rejected"
        ? [
            {
              id: "reject_reason",
              label: "Reject Reason",
              sortable: false,
              minWidth: 220,
              wrap: true,
              render: (value) => {
                if (!value) return "—";
                const displayLabel = getRejectionDisplayLabel(value);
                // Clamp long reasons; reveal the full label + raw code on hover.
                const raw = String(value);
                const full = displayLabel && displayLabel !== raw ? `${displayLabel}\n${raw}` : raw;
                return `<div class="dt-longtext" data-tooltip="${Utils.escapeHtml(full)}" data-tooltip-truncated>${Utils.escapeHtml(displayLabel)}</div>`;
              },
            },
          ]
        : []),
      // Conditionally add blacklist_reason column only for blacklisted view
      ...(state.view === "blacklisted"
        ? [
            {
              id: "blacklist_reason",
              label: "Blacklist Reason",
              sortable: false,
              minWidth: 250,
              wrap: true,
              render: (_v, row) => {
                const summary = summarizeBlacklistReasons(row.blacklist_reasons);
                if (!summary) return "—";
                // One reason per line in the hover tooltip for readability.
                const full = summarizeBlacklistReasons(row.blacklist_reasons, "\n");
                return `<div class="dt-longtext" data-tooltip="${Utils.escapeHtml(full)}" data-tooltip-truncated>${Utils.escapeHtml(summary)}</div>`;
              },
            },
          ]
        : []),
      {
        id: "status",
        label: "Status",
        sortable: false,
        minWidth: 140,
        wrap: false,
        render: (_v, row) => {
          const flags = [];
          if (row.has_pool_price) flags.push('<span class="badge info">Price</span>');
          if (row.has_ohlcv) flags.push('<span class="badge">OHLCV</span>');
          if (row.has_open_position) flags.push('<span class="badge success">Position</span>');
          if (row.blacklisted) {
            const summary = summarizeBlacklistReasons(row.blacklist_reasons);
            const tooltip = summary ? `Blacklisted: ${summary}` : "Blacklisted token";
            flags.push(
              `<span class="badge warning" title="${Utils.escapeHtml(tooltip)}">Blacklisted</span>`
            );
          }
          return flags.join(" ") || "—";
        },
      },
      {
        id: "updated_at",
        label: "Updated",
        sortable: true,
        minWidth: 100,
        wrap: false,
        render: (_v, row) => {
          // Select timestamp based on current view
          let timestamp;

          if (state.view === "pool" || state.view === "positions") {
            // Pool/Positions: show pool price calculation time
            timestamp = row.pool_price_last_calculated_at;
          } else if (state.view === "no_market") {
            // No Market: show metadata fetch time
            timestamp = row.metadata_last_fetched_at;
          } else {
            // All others: show market data fetch time
            timestamp = row.market_data_last_fetched_at;
          }

          if (!timestamp) {
            return "—";
          }

          // timestamp is DateTime<Utc> serialized, convert to seconds
          const timestampSeconds =
            typeof timestamp === "string"
              ? Math.floor(new Date(timestamp).getTime() / 1000)
              : timestamp;

          return timeAgoCell(timestampSeconds);
        },
      },
      {
        id: "token_birth_at",
        label: "Birth",
        sortable: true,
        minWidth: 110,
        wrap: false,
        render: (_v, row) => {
          // Blockchain creation time, fallback to bot discovery
          const timestamp = row.blockchain_created_at || row.first_discovered_at;
          if (!timestamp) return "—";

          const timestampSeconds =
            typeof timestamp === "string"
              ? Math.floor(new Date(timestamp).getTime() / 1000)
              : timestamp;

          return timeAgoCell(timestampSeconds);
        },
      },
      {
        id: "first_seen_at",
        label: "First Seen",
        sortable: true,
        minWidth: 110,
        wrap: false,
        render: (_v, row) => {
          const timestamp = row.first_discovered_at;
          if (!timestamp) return "—";

          const timestampSeconds =
            typeof timestamp === "string"
              ? Math.floor(new Date(timestamp).getTime() / 1000)
              : timestamp;

          return timeAgoCell(timestampSeconds);
        },
      },
    ];
  };

  // ═══════════════════════════════════════════════════════════════════════════
  // VIEW SWITCHING
  // ═══════════════════════════════════════════════════════════════════════════

  const switchView = (view) => {
    if (!TOKEN_VIEWS.some((v) => v.id === view)) return;
    const previousView = state.view;
    state.view = view;

    // Handle Favorites view specially - it has its own table
    if (view === "favorites") {
      favorites.initFavoritesTable();
      favorites.showFavoritesView();
      return;
    }

    // Leaving Favorites view - hide it
    if (previousView === "favorites") {
      favorites.hideFavoritesView();
    }

    // Handle OHLCV view specially - it has its own table
    if (view === "ohlcv") {
      ohlcv.initOhlcvTable();
      ohlcv.showOhlcvView();
      return;
    }

    // Leaving OHLCV view - hide it
    if (previousView === "ohlcv") {
      ohlcv.hideOhlcvView();
    }

    state.totalCount = null;
    state.lastUpdate = null;
    state.filters = getDefaultFiltersForView(view);
    state.summary = { ...DEFAULT_SUMMARY };
    state.availableRejectionReasons = [];

    if (view === "pool" && previousView !== "pool") {
      resetPriceTracking();
    } else if (previousView === "pool" && view !== "pool") {
      resetPriceTracking();
    }

    // Update table stateKey for per-tab state persistence
    if (table) {
      const nextStateKey = getTokensTableStateKey(view);
      table.setStateKey(nextStateKey, { render: false });

      // Update columns for the new view (different views have different conditional columns)
      const newColumns = buildColumns();
      table.setColumns(newColumns, {
        preserveData: true,
        preserveScroll: false, // Reset scroll when switching views
        resetState: false, // Keep column widths/visibility preferences within each view
      });

      // Read restored sort state from table and sync to state.sort
      const restoredTableState = table.getServerState();
      if (restoredTableState?.sortColumn) {
        const sortKey = getServerSortKey(restoredTableState.sortColumn, view);
        if (sortKey) {
          state.sort = {
            by: sortKey,
            direction: restoredTableState.sortDirection || "asc",
          };
        } else {
          state.sort = { ...DEFAULT_SERVER_SORT };
        }
      } else {
        const persistedSort = loadPersistedSort(nextStateKey);
        if (persistedSort) {
          const persistedKey = getServerSortKey(persistedSort.column, view);
          if (persistedKey) {
            state.sort = { by: persistedKey, direction: persistedSort.direction };
          } else {
            state.sort = { ...DEFAULT_SERVER_SORT };
          }
        } else {
          state.sort = { ...DEFAULT_SERVER_SORT };
        }
      }
    } else {
      state.sort = { ...DEFAULT_SERVER_SORT };
    }

    applyViewPreferences();
    syncTableSortState({ render: true });
    syncToolbarFilters();
    updateToolbar();
    requestReload("view", { silent: false, resetScroll: true }).catch(() => {});
  };

  const handleLinkAction = (actionId, mint) => {
    const urlMap = {
      dexscreener: `https://dexscreener.com/solana/${mint}`,
      gmgn: `https://gmgn.ai/sol/token/${mint}`,
      solscan: `https://solscan.io/token/${mint}`,
      birdeye: `https://birdeye.so/solana/token/${mint}`,
      rugcheck: `https://rugcheck.xyz/tokens/${mint}`,
      pumpfun: `https://pump.fun/${mint}`,
    };

    if (actionId === "copy") {
      navigator.clipboard
        .writeText(mint)
        .then(() => {
          Utils.showToast("Mint address copied", "success");
        })
        .catch((err) => {
          console.error("Failed to copy mint:", err);
          Utils.showToast("Failed to copy mint", "warning");
        });
    } else if (urlMap[actionId]) {
      Utils.openExternal(urlMap[actionId]);
    }
  };

  const initializeLinkDropdowns = (tbl = table) => {
    // Use event delegation instead of creating Dropdown instances
    // This works better with dynamic table data that re-renders.
    // tbl defaults to the main table but is parameterised so the Favorites
    // table (same columns) can reuse the identical wiring.
    if (!tbl?.elements?.scrollContainer) return;

    // Remove existing listener if any
    const container = tbl.elements.scrollContainer;
    if (container._linksClickHandler) {
      container.removeEventListener("click", container._linksClickHandler);
    }

    // Add delegated click handler
    const clickHandler = (e) => {
      const trigger = e.target.closest(".links-dropdown-trigger");
      if (!trigger) return;

      e.preventDefault();
      e.stopPropagation();

      const mint = trigger.dataset.mint;
      if (!mint) return;

      // Close any existing dropdown
      const existingMenu = document.querySelector(".links-dropdown-menu.open");
      if (existingMenu) {
        existingMenu.remove();
      }

      // Create dropdown menu
      const menu = document.createElement("div");
      menu.className = "links-dropdown-menu dropdown-menu open";
      menu.setAttribute("data-align", "left");
      menu.innerHTML = `
        <button class="dropdown-item" data-action="dexscreener" type="button">
          <i class="icon-chart-bar"></i>
          <span class="label">DexScreener</span>
        </button>
        <button class="dropdown-item" data-action="gmgn" type="button">
          <i class="icon-trending-up"></i>
          <span class="label">GMGN</span>
        </button>
        <button class="dropdown-item" data-action="solscan" type="button">
          <i class="icon-search"></i>
          <span class="label">Solscan</span>
        </button>
        <button class="dropdown-item" data-action="birdeye" type="button">
          <span class="icon"><i class="icon-chart-bar"></i></span>
          <span class="label">Birdeye</span>
        </button>
        <button class="dropdown-item" data-action="rugcheck" type="button">
          <span class="icon"><i class="icon-shield"></i></span>
          <span class="label">RugCheck</span>
        </button>
        <button class="dropdown-item" data-action="pumpfun" type="button">
          <span class="icon"><i class="icon-rocket"></i></span>
          <span class="label">Pump.fun</span>
        </button>
        <div class="dropdown-divider"></div>
        <button class="dropdown-item" data-action="copy" type="button">
          <span class="icon"><i class="icon-copy"></i></span>
          <span class="label">Copy Mint</span>
        </button>
      `;

      // Position menu relative to trigger
      const rect = trigger.getBoundingClientRect();
      const containerRect = container.getBoundingClientRect();
      menu.style.position = "absolute";
      menu.style.top = `${rect.bottom - containerRect.top + container.scrollTop + 6}px`;
      menu.style.left = `${rect.left - containerRect.left + container.scrollLeft}px`;
      menu.style.zIndex = "9999";

      container.appendChild(menu);

      // Register with the global menu coordinator so opening any other menu, or
      // clicking an input/checkbox, or changing page auto-dismisses this one too.
      const menuHandle = {
        owns: (t) => menu.contains(t) || (trigger && trigger.contains(t)),
        close: () => {
          menu.remove();
          document.removeEventListener("click", closeHandler);
          document.removeEventListener("keydown", escapeHandler);
          closeMenu(menuHandle);
        },
      };
      openMenu(menuHandle);

      // Handle menu item clicks
      const menuClickHandler = (e) => {
        const item = e.target.closest(".dropdown-item");
        if (!item) return;

        const action = item.dataset.action;
        if (action) {
          handleLinkAction(action, mint);
        }
        menuHandle.close();
      };
      menu.addEventListener("click", menuClickHandler);

      // Close on outside click (coordinator also covers this; kept as a fallback).
      const closeHandler = (e) => {
        if (!menu.contains(e.target) && e.target !== trigger) {
          menuHandle.close();
        }
      };
      setTimeout(() => document.addEventListener("click", closeHandler), 0);

      // Close on escape
      const escapeHandler = (e) => {
        if (e.key === "Escape") {
          menuHandle.close();
        }
      };
      document.addEventListener("keydown", escapeHandler);
    };

    container._linksClickHandler = clickHandler;
    container.addEventListener("click", clickHandler);
  };

  const initializeImageLightbox = (tbl = table) => {
    // Use event delegation for logo clicks
    if (!tbl?.elements?.scrollContainer) return;

    const container = tbl.elements.scrollContainer;
    if (container._logoClickHandler) {
      container.removeEventListener("click", container._logoClickHandler);
    }

    const clickHandler = (e) => {
      const logo = e.target.closest(".clickable-logo");
      if (!logo) return;

      e.preventDefault();
      e.stopPropagation();

      const imageUrl = logo.dataset.logoUrl;
      const symbol = logo.dataset.tokenSymbol || "";
      const name = logo.dataset.tokenName || "";
      const mint = logo.dataset.tokenMint || "";

      if (!imageUrl) return;

      // Look up age from current table data instead of stale data attribute
      let ageText = "Unknown";
      if (mint && tbl) {
        const tableData = tbl.getData();
        const rowData = tableData.find((row) => row.mint === mint);
        if (rowData) {
          const timestamp = rowData.blockchain_created_at || rowData.first_discovered_at;
          if (timestamp) {
            const ageSeconds =
              typeof timestamp === "string"
                ? Math.floor(new Date(timestamp).getTime() / 1000)
                : timestamp;
            ageText = Utils.formatTimeAgo(ageSeconds, { fallback: "Unknown" });
          }
        }
      }

      showImageLightbox({
        imageUrl,
        symbol,
        name,
        stats: [{ label: "Token Age", value: ageText }],
      });
    };

    container._logoClickHandler = clickHandler;
    container.addEventListener("click", clickHandler);
  };

  const initializeRowClickHandler = (tbl = table) => {
    // Use event delegation for row clicks
    if (!tbl?.elements?.scrollContainer) {
      return;
    }

    const container = tbl.elements.scrollContainer;
    if (container._rowClickHandler) {
      container.removeEventListener("click", container._rowClickHandler);
    }

    const clickHandler = (e) => {
      // Ignore clicks on logos, buttons, and interactive elements
      if (
        e.target.closest(".clickable-logo") ||
        e.target.closest(".row-action") ||
        e.target.closest(".links-dropdown-trigger") ||
        e.target.closest("button") ||
        e.target.closest("a")
      ) {
        return;
      }

      // Find the row element
      const row = e.target.closest("tr[data-row-id]");
      if (!row) return;

      // Get the mint from the row's data-row-id attribute (set by DataTable)
      const mint = row.dataset.rowId;
      if (!mint) return;

      // Find token data in table
      const tableData = tbl.getData();
      const tokenData = tableData.find((token) => token.mint === mint);
      if (!tokenData) return;

      // Show token details dialog
      if (!tokenDetailsDialog) {
        tokenDetailsDialog = new TokenDetailsDialog();
      }
      tokenDetailsDialog.show(tokenData);
    };

    container._rowClickHandler = clickHandler;
    container.addEventListener("click", clickHandler);
  };

  // Delegated Buy/Add/Sell row-action clicks for an arbitrary token table.
  // Generalised from the main table's handler so the Favorites table (same
  // columns) reuses the exact trade flow.
  const attachRowActions = (tbl, onReload) => {
    const container = tbl?.elements?.scrollContainer;
    if (!container) return;
    if (container._rowActionHandler) {
      container.removeEventListener("click", container._rowActionHandler);
    }
    const handler = async (e) => {
      const btn = e.target?.closest?.(".row-action");
      if (!btn) return;
      const action = btn.getAttribute("data-action");
      const mint = btn.getAttribute("data-mint");
      if (!action || !mint) return;
      const row = tbl.getData().find((r) => r.mint === mint);
      if (!row) {
        Utils.showToast("Token data not found", "error");
        return;
      }
      await performManualTrade({ action, mint, row, btn, onReload });
    };
    container._rowActionHandler = handler;
    container.addEventListener("click", handler);
  };

  // One-shot wiring for any token table that uses buildColumns(): row actions,
  // external-links dropdown, logo lightbox, and row-click -> token details.
  const wireTokenTable = (tbl, { onReload } = {}) => {
    attachRowActions(tbl, onReload);
    initializeLinkDropdowns(tbl);
    initializeImageLightbox(tbl);
    initializeRowClickHandler(tbl);
  };

  // Share the column builder and wiring with sub-modules (Favorites) so their
  // table is identical to the all/passed token lists.
  deps.buildColumns = buildColumns;
  deps.wireTokenTable = wireTokenTable;

  return {
    init(ctx) {
      // Initialize token details dialog
      tokenDetailsDialog = new TokenDetailsDialog();

      // Registered ONCE per page instance, not per activate: the boost feed is
      // global and a second listener would repaint every row twice per change.
      window.addEventListener(BOOSTS_CHANGED_EVENT, onBoostsChanged);
      eventCleanups.push(() =>
        window.removeEventListener(BOOSTS_CHANGED_EVENT, onBoostsChanged)
      );

      // Initialize tab bar for tokens page
      tabBar = new TabBar({
        container: "#subTabsContainer",
        tabs: TOKEN_VIEWS,
        defaultTab: DEFAULT_VIEW,
        stateKey: "tokens.activeTab",
        pageName: "tokens",
        onChange: (tabId) => {
          switchView(tabId);
        },
      });

      // Register with TabBarManager for page-switch coordination
      TabBarManager.register("tokens", tabBar);

      // Integrate with lifecycle for auto-cleanup
      ctx.manageTabBar(tabBar);

      // Show the tab bar
      tabBar.show();

      // Attach hint triggers to tabs
      attachHintsToTabs();

      // Get the active tab after state restoration
      const activeTab = tabBar.getActiveTab();
      if (activeTab && activeTab !== state.view) {
        // Sync state with tab bar's restored state (e.g., from URL hash)
        state.view = activeTab;
      }

      const tableStateKey = getTokensTableStateKey(state.view);
      const persistedSort = loadPersistedSort(tableStateKey);
      if (persistedSort) {
        const persistedSortKey = getServerSortKey(persistedSort.column, state.view);
        if (persistedSortKey) {
          state.sort = {
            by: persistedSortKey,
            direction: persistedSort.direction,
          };
        }
      }

      state.filters = getDefaultFiltersForView(state.view);
      state.hasLoadedOnce = false;

      const initialSortColumn = resolveSortColumn(state.sort.by);

      // Build columns based on current view
      const columns = buildColumns();

      table = new DataTable({
        container: "#tokens-root",
        columns,
        rowIdField: "mint",
        stateKey: tableStateKey,
        enableLogging: false,
        // Gold-mark the tokens whose owners paid to boost them. Purely a mark:
        // the row keeps the position the user's own sort gave it.
        rowClass: (row) => boostRowClass(row?.mint),
        sorting: {
          mode: "server",
          column: initialSortColumn,
          direction: state.sort.direction,
          onChange: handleSortChange,
        },
        compact: true,
        stickyHeader: true,
        zebra: true,
        fitToContainer: true,
        autoSizeColumns: false,
        uniformRowHeight: 2,
        pagination: {
          // Hybrid pagination modes: enable both scroll and pages
          modes: ["scroll", "pages"],
          defaultMode: "scroll",
          modeStateKey: `tokens.${state.view}.paginationMode`,
          defaultPageSize: 50,
          pageSizes: [10, 20, 50, 100],
          // Scroll mode settings
          threshold: 160,
          maxRows: 5000,
          loadPage: loadTokensPage,
          dedupeKey: (row) => row?.mint ?? null,
          rowIdField: "mint",
          onPageLoaded: () => {
            updateToolbar();
            initializeLinkDropdowns();
            initializeImageLightbox();
            initializeRowClickHandler();
          },
        },
        toolbar: {
          summary: [
            { id: "tokens-total", label: "Total", value: "0" },
            { id: "tokens-priced", label: "With Price", value: "0", variant: "info" },
            { id: "tokens-positions", label: "Positions", value: "0", variant: "secondary" },
            { id: "tokens-blacklisted", label: "Blacklisted", value: "0", variant: "warning" },
          ],
          search: {
            enabled: true,
            mode: "server",
            placeholder: "Search by symbol or mint...",
            onChange: (value) => {
              state.search = (value || "").trim();
              // onChange just updates state, onSubmit triggers reload
            },
            onSubmit: () => {
              state.totalCount = null;
              state.lastUpdate = null;
              updateToolbar();
              requestReload("search", {
                silent: false,
                resetScroll: true,
              }).catch(() => {});
            },
          },
          filters: [
            {
              id: "rejection_reason",
              label: "Reject Reason",
              mode: "server",
              autoApply: true,
              defaultValue: DEFAULT_FILTERS.rejection_reason,
              options: [{ value: "all", label: "All" }],
              onChange: (value, _el, options) => {
                state.filters.rejection_reason = value || "all";
                if (options?.restored) {
                  return;
                }
                state.totalCount = null;
                state.lastUpdate = null;
                updateToolbar();
                requestReload("filters", {
                  silent: false,
                  resetScroll: true,
                }).catch(() => {});
              },
            },
          ],
        },
      });

      table.showBlockingState?.({
        variant: "loading",
        title: "Loading tokens snapshot...",
        description: "Large token databases can take a few seconds to warm up during startup.",
      });

      // Hide rejection_reason filter immediately if not in rejected tab
      updateFilterVisibility();

      // Row actions: delegate clicks on the table container
      const containerEl = document.querySelector("#tokens-root");
      const handleRowActionClick = async (e) => {
        const btn = e.target?.closest?.(".row-action");
        if (!btn) return;
        const action = btn.getAttribute("data-action");
        const mint = btn.getAttribute("data-mint");
        if (!action || !mint) return;

        // Find row data
        const row = table.getData().find((r) => r.mint === mint);
        if (!row) {
          Utils.showToast("Token data not found", "error");
          return;
        }

        await performManualTrade({
          action,
          mint,
          row,
          btn,
          onReload: () =>
            requestReload("manual", { silent: false, preserveScroll: true }).catch(() => {}),
        });
      };

      if (containerEl) {
        containerEl.addEventListener("click", handleRowActionClick);
      }
      applyViewPreferences();

      // Sync state from DataTable's restored server state
      const restoredState = table.getServerState();
      const serverState = restoredState || { filters: {} };
      let hasSortRestored = false;

      if (serverState.sortColumn) {
        const sortKey = getServerSortKey(serverState.sortColumn, state.view);
        if (sortKey) {
          state.sort = {
            by: sortKey,
            direction: serverState.sortDirection || "asc",
          };
          hasSortRestored = true;
        }
      }
      if (serverState.searchQuery) {
        state.search = serverState.searchQuery;
      }

      const serverFilters = serverState.filters || {};
      if (Object.prototype.hasOwnProperty.call(serverFilters, "pool_price")) {
        state.filters.pool_price = parseToggleValue(serverFilters.pool_price, false);
      } else if (Object.prototype.hasOwnProperty.call(serverFilters, "priced")) {
        const legacy = serverFilters.priced;
        state.filters.pool_price = legacy === "priced" || parseToggleValue(legacy, false);
      }

      if (Object.prototype.hasOwnProperty.call(serverFilters, "positions")) {
        const value = serverFilters.positions;
        if (typeof value === "string") {
          state.filters.positions = value === "open";
        } else {
          state.filters.positions = parseToggleValue(value, false);
        }
      }

      if (Object.prototype.hasOwnProperty.call(serverFilters, "rejection_reason")) {
        const reason = serverFilters.rejection_reason;
        state.filters.rejection_reason =
          typeof reason === "string" && reason.trim().length > 0 ? reason : "all";
      }

      if (state.view === "positions") {
        state.filters.positions = true;
      }
      if (state.view === "no_market") {
        state.filters.pool_price = false;
      }
      if (state.view !== "rejected") {
        state.filters.rejection_reason = "all";
      }

      syncTableSortState({ render: false });
      syncToolbarFilters();
      table.setToolbarSearchValue(state.search, { apply: false });
      updateToolbar();

      // Handle OHLCV view specially - it has its own table
      if (state.view === "ohlcv") {
        ohlcv.initOhlcvTable();
        ohlcv.showOhlcvView();
      } else if (!hasSortRestored) {
        // Trigger initial data load if no sort state was restored
        // (sort restoration triggers reload via handleSortChange)
        requestReload("initial", { silent: false, resetScroll: true }).catch(() => {});
      }
    },

    activate(ctx) {
      // Re-register deactivate cleanup (cleanups are cleared after each deactivate)
      // and force-show tab bar to handle race conditions with TabBarManager
      if (tabBar) {
        ctx.manageTabBar(tabBar);
        tabBar.show({ force: true });
      }

      // Re-attach hints to tabs after TabBar remounts buttons on show()
      // (hints are lost when hide() clears innerHTML and show() rebuilds tabs)
      attachHintsToTabs();

      // Fetch wallet balance for dialog context
      fetch("/api/wallet/balance")
        .then((res) => (res.ok ? res.json() : null))
        .then((data) => {
          if (data?.sol_balance != null) {
            walletBalance = data.sol_balance;
          }
        })
        .catch(() => {
          console.warn("[Tokens] Failed to fetch wallet balance");
        });

      // Handle OHLCV view specially - it has its own poller
      if (state.view === "ohlcv") {
        if (deps.ohlcvPoller) {
          deps.ohlcvPoller.start();
        } else {
          // OHLCV poller not initialized yet, fetch data
          ohlcv.fetchOhlcvData().catch(() => {});
        }
        return;
      }

      // Handle Favorites view specially - refresh favorites data
      if (state.view === "favorites") {
        favorites
          .fetchFavorites()
          .then(() => favorites.updateFavoritesTable())
          .catch(() => {});
        return;
      }

      if (!poller) {
        poller = deps.poller = ctx.managePoller(
          new Poller(() => requestReload("poll", { silent: true, preserveScroll: true }), {
            label: "Tokens",
            // The tokens/list query scans the full token DB (300k+ rows, COALESCE
            // sort) and takes ~2-4s. Polling faster than that just queues reloads
            // that abort each other and pegs a core. Floor the interval at 4s so the
            // table stays live (each poll completes, newest-updated bubbles to top)
            // without hammering. The in-flight guard in shouldSkipPollReload still
            // skips ticks while a fetch is running.
            getInterval: () => Math.max(4000, getGlobalPollInterval() || 4000),
          })
        );
      }
      poller.start();
      if ((table?.getData?.() ?? []).length === 0) {
        requestReload("initial", { silent: false, resetScroll: true }).catch(() => {});
      }

      // The boost feed decides which rows read gold. Kick it off without
      // awaiting -- the table paints from its own data and re-marks on the event.
      ensureBoosts();

      // Show the featured row on the tokens page
      showFeaturedRow();
    },

    deactivate() {
      // Hide featured row when leaving page
      hideFeaturedRow();

      table?.cancelPendingLoad();
      // Lifecycle automatically stops managed pollers
      // Pause OHLCV poller if active
      if (ohlcvPoller) ohlcvPoller.pause();
    },

    dispose() {
      // Clean up all tracked event listeners
      eventCleanups.forEach((cleanup) => cleanup());
      eventCleanups.length = 0;

      // Lifecycle automatically cleans up managed pollers
      // Clean up any open dropdown menus
      const existingMenu = document.querySelector(".links-dropdown-menu");
      if (existingMenu) {
        existingMenu.remove();
      }
      // Clean up any open image lightbox
      const existingLightbox = document.querySelector(".image-lightbox");
      if (existingLightbox) {
        existingLightbox.remove();
      }
      if (tokenDetailsDialog) {
        tokenDetailsDialog.destroy();
        tokenDetailsDialog = null;
      }
      if (table) {
        table.hideBlockingState?.();
        table.destroy();
        table = null;
      }
      // Clean up OHLCV table
      if (ohlcvTable) {
        ohlcvTable.destroy();
        ohlcvTable = null;
      }
      if (ohlcvPoller) {
        ohlcvPoller.stop();
        ohlcvPoller = null;
      }
      // Remove OHLCV container
      const ohlcvContainer = document.querySelector("#ohlcv-table-container");
      if (ohlcvContainer) {
        ohlcvContainer.remove();
      }
      // Clean up Favorites table
      if (favoritesTable) {
        favoritesTable.destroy();
        favoritesTable = null;
      }
      // Remove Favorites container
      const favoritesContainer = document.querySelector("#favorites-table-container");
      if (favoritesContainer) {
        favoritesContainer.remove();
      }
      poller = null;
      tabBar = null; // Cleaned up automatically by manageTabBar
      TabBarManager.unregister("tokens");
      resetPriceTracking();
      state.view = DEFAULT_VIEW;
      state.search = "";
      state.totalCount = null;
      state.lastUpdate = null;
      state.sort = { ...DEFAULT_SERVER_SORT };
      state.filters = getDefaultFiltersForView(DEFAULT_VIEW);
      state.summary = { ...DEFAULT_SUMMARY };
      state.hasLoadedOnce = false;
      walletBalance = 0;
      // Reset OHLCV state
      ohlcvState.tokens = [];
      ohlcvState.stats = null;
      ohlcvState.isLoading = false;
      // Reset Favorites state
      favoritesState.favorites = [];
      favoritesState.isLoading = false;
    },
  };
}

registerPage("tokens", createLifecycle());
