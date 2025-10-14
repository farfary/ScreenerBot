// State Manager - Browser Storage (globally accessible)
window.AppState = {
  save(key, value) {
    try {
      localStorage.setItem(`screenerbot_${key}`, JSON.stringify(value));
    } catch (e) {
      console.warn("Failed to save state:", key, e);
    }
  },

  load(key, defaultValue = null) {
    try {
      const item = localStorage.getItem(`screenerbot_${key}`);
      return item ? JSON.parse(item) : defaultValue;
    } catch (e) {
      console.warn("Failed to load state:", key, e);
      return defaultValue;
    }
  },

  remove(key) {
    try {
      localStorage.removeItem(`screenerbot_${key}`);
    } catch (e) {
      console.warn("Failed to remove state:", key, e);
    }
  },

  clearAll() {
    try {
      Object.keys(localStorage)
        .filter((key) => key.startsWith("screenerbot_"))
        .forEach((key) => localStorage.removeItem(key));
    } catch (e) {
      console.warn("Failed to clear state:", e);
    }
  },
};

(function () {
  const STORAGE_VERSION = "v1";
  const VISIBILITY_PREFIX = `tableColumns:${STORAGE_VERSION}:`;
  const WIDTH_PREFIX = `tableColumnWidths:${STORAGE_VERSION}:`;

  function resolveElement(ref) {
    if (!ref) return null;
    if (ref instanceof HTMLElement) return ref;
    if (typeof ref === "string") {
      return document.getElementById(ref) || document.querySelector(ref);
    }
    return null;
  }

  function isNonEmptyString(value) {
    return typeof value === "string" && value.trim().length > 0;
  }

  function clampWidth(value, column) {
    const min = Number.isFinite(column.minWidth) ? column.minWidth : 60;
    const max = Number.isFinite(column.maxWidth) ? column.maxWidth : 640;
    let next = Number(value);
    if (!Number.isFinite(next)) {
      next = min;
    }
    next = Math.round(next);
    if (next < min) next = min;
    if (Number.isFinite(max) && next > max) next = max;
    return next;
  }

  function toStorageKey(prefix, key) {
    return `${prefix}${key}`;
  }

  function arraysEqual(a, b) {
    if (a === b) return true;
    if (!Array.isArray(a) || !Array.isArray(b)) return false;
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i += 1) {
      if (a[i] !== b[i]) return false;
    }
    return true;
  }

  function normalizeColumns(columns) {
    if (!Array.isArray(columns) || columns.length === 0) {
      throw new Error(
        "TableColumnManager.create requires a non-empty columns array"
      );
    }

    return columns.map((original) => {
      if (!original || typeof original !== "object") {
        throw new Error("Column definition must be an object");
      }

      if (!isNonEmptyString(original.id)) {
        throw new Error("Column definition is missing a valid id");
      }

      const normalized = {
        ...original,
        id: String(original.id),
        label: isNonEmptyString(original.label)
          ? String(original.label)
          : String(original.id),
        defaultVisible: original.defaultVisible !== false,
        resizable: original.resizable !== false,
        required: original.required === true,
        align: isNonEmptyString(original.align)
          ? original.align
          : original.cellAlign || "left",
        headerAlign: isNonEmptyString(original.headerAlign)
          ? original.headerAlign
          : isNonEmptyString(original.align)
          ? original.align
          : "left",
        minWidth: Number.isFinite(original.minWidth)
          ? Number(original.minWidth)
          : 60,
        defaultWidth: Number.isFinite(original.defaultWidth)
          ? Number(original.defaultWidth)
          : undefined,
        maxWidth: Number.isFinite(original.maxWidth)
          ? Number(original.maxWidth)
          : undefined,
        sortKey: isNonEmptyString(original.sortKey)
          ? String(original.sortKey)
          : undefined,
      };

      if (normalized.minWidth < 40) {
        normalized.minWidth = 40;
      }

      if (
        Number.isFinite(normalized.maxWidth) &&
        normalized.maxWidth < normalized.minWidth
      ) {
        normalized.maxWidth = normalized.minWidth;
      }

      return normalized;
    });
  }

  function sanitizeVisibleIds(columns, requestedIds, defaults) {
    const orderedIds = columns.map((col) => col.id);
    const requiredIds = columns
      .filter((col) => col.required)
      .map((col) => col.id);
    const requested = Array.isArray(requestedIds) ? requestedIds : [];
    const included = new Set();
    const next = [];

    orderedIds.forEach((id) => {
      if (
        requested.includes(id) ||
        (!requested.length && defaults.includes(id))
      ) {
        if (!included.has(id)) {
          included.add(id);
          next.push(id);
        }
      }
    });

    requiredIds.forEach((id) => {
      if (!included.has(id)) {
        included.add(id);
        next.push(id);
      }
    });

    if (!next.length) {
      const fallback = defaults.find((id) => orderedIds.includes(id));
      if (fallback) {
        next.push(fallback);
      } else if (orderedIds.length) {
        next.push(orderedIds[0]);
      }
    }

    return next;
  }

  function loadVisibleIds(storageKey, columns, defaults) {
    try {
      const saved = window.AppState
        ? AppState.load(toStorageKey(VISIBILITY_PREFIX, storageKey), null)
        : null;
      if (!Array.isArray(saved) || !saved.length) {
        return defaults.slice();
      }
      return sanitizeVisibleIds(columns, saved, defaults);
    } catch (err) {
      console.warn(
        "[TableColumnManager] Failed to load column visibility",
        err
      );
      return defaults.slice();
    }
  }

  function persistVisibleIds(storageKey, ids) {
    if (!window.AppState) return;
    try {
      AppState.save(toStorageKey(VISIBILITY_PREFIX, storageKey), ids);
    } catch (err) {
      console.warn(
        "[TableColumnManager] Failed to persist column visibility",
        err
      );
    }
  }

  function loadWidthMap(storageKey, columns) {
    const map = new Map();
    try {
      const saved = window.AppState
        ? AppState.load(toStorageKey(WIDTH_PREFIX, storageKey), null)
        : null;
      if (saved && typeof saved === "object") {
        Object.entries(saved).forEach(([key, value]) => {
          const num = Number(value);
          if (Number.isFinite(num) && num > 0) {
            map.set(key, num);
          }
        });
      }
    } catch (err) {
      console.warn("[TableColumnManager] Failed to load column widths", err);
    }

    columns.forEach((column) => {
      if (!map.has(column.id) && Number.isFinite(column.defaultWidth)) {
        map.set(column.id, clampWidth(column.defaultWidth, column));
      }
    });

    return map;
  }

  function persistWidthMap(storageKey, map) {
    if (!window.AppState) return;
    try {
      const payload = {};
      map.forEach((value, key) => {
        payload[key] = value;
      });
      AppState.save(toStorageKey(WIDTH_PREFIX, storageKey), payload);
    } catch (err) {
      console.warn("[TableColumnManager] Failed to persist column widths", err);
    }
  }

  function TableColumnManager(options = {}) {
    const table = resolveElement(options.table || options.tableId);
    if (!table) {
      throw new Error(
        "TableColumnManager.create requires a valid table element"
      );
    }

    const columns = normalizeColumns(options.columns || []);
    const columnMap = new Map(columns.map((col) => [col.id, col]));

    let colgroup = resolveElement(options.colgroup || options.colGroupId);
    if (!colgroup) {
      colgroup = table.querySelector("colgroup");
    }
    if (!colgroup) {
      colgroup = document.createElement("colgroup");
      const firstChild = table.firstElementChild;
      if (firstChild) {
        table.insertBefore(colgroup, firstChild);
      } else {
        table.appendChild(colgroup);
      }
    }

    let headerRow = resolveElement(options.headerRow || options.headerRowId);
    if (!headerRow) {
      const thead = table.tHead || table.createTHead();
      headerRow = thead.querySelector("tr");
      if (!headerRow) {
        headerRow = document.createElement("tr");
        thead.appendChild(headerRow);
      }
    }

    const storageKey = isNonEmptyString(options.storageKey)
      ? options.storageKey
      : table.id || "table";

    const defaultVisibleIds = columns
      .filter((col) => col.defaultVisible || col.required)
      .map((col) => col.id);

    const state = {
      table,
      headerRow,
      colgroup,
      columns,
      columnMap,
      storageKey,
      defaultVisibleIds,
      visibleIds: [],
      widthMap: new Map(),
      headerCells: new Map(),
      colElements: new Map(),
      sortState: null,
      onLayoutChange:
        typeof options.onLayoutChange === "function"
          ? options.onLayoutChange
          : null,
    };

    state.visibleIds = loadVisibleIds(storageKey, columns, defaultVisibleIds);
    state.widthMap = loadWidthMap(storageKey, columns);

    function getVisibleColumns() {
      return state.visibleIds
        .map((id) => state.columnMap.get(id))
        .filter(Boolean);
    }

    function getColumnWidth(column) {
      if (!column) return 120;
      if (state.widthMap.has(column.id)) {
        return clampWidth(state.widthMap.get(column.id), column);
      }
      if (Number.isFinite(column.defaultWidth)) {
        const width = clampWidth(column.defaultWidth, column);
        state.widthMap.set(column.id, width);
        return width;
      }
      const fallback = Math.max(column.minWidth || 60, 120);
      state.widthMap.set(column.id, fallback);
      return fallback;
    }

    function applyColumnWidth(column, width) {
      if (!column) return;
      const clamped = clampWidth(width, column);
      state.widthMap.set(column.id, clamped);

      const headerCell = state.headerCells.get(column.id);
      if (headerCell) {
        headerCell.style.width = `${clamped}px`;
        headerCell.style.minWidth = `${column.minWidth || clamped}px`;
      }

      const colEl = state.colElements.get(column.id);
      if (colEl) {
        colEl.style.width = `${clamped}px`;
        colEl.style.minWidth = `${column.minWidth || clamped}px`;
      }
    }

    function setFrozenLayout(enabled) {
      if (!state.table) return;
      if (enabled) {
        state.table.classList.add("table--layout-frozen");
      } else {
        state.table.classList.remove("table--layout-frozen");
      }
    }

    function persistWidths() {
      persistWidthMap(storageKey, state.widthMap);
    }

    function buildColgroup() {
      state.colElements.clear();
      while (state.colgroup.firstChild) {
        state.colgroup.removeChild(state.colgroup.firstChild);
      }

      getVisibleColumns().forEach((column) => {
        const colEl = document.createElement("col");
        colEl.setAttribute("data-column-id", column.id);
        state.colgroup.appendChild(colEl);
        state.colElements.set(column.id, colEl);
        applyColumnWidth(column, getColumnWidth(column));
      });
    }

    function buildHeaderRow() {
      state.headerCells.clear();
      while (state.headerRow.firstChild) {
        state.headerRow.removeChild(state.headerRow.firstChild);
      }

      getVisibleColumns().forEach((column) => {
        const th = document.createElement("th");
        th.setAttribute("data-column-id", column.id);
        th.classList.add("tokens-table-header");
        if (column.headerAlign === "right") {
          th.classList.add("align-right");
        } else if (column.headerAlign === "center") {
          th.classList.add("align-center");
        }
        if (column.sortKey) {
          th.dataset.sortKey = column.sortKey;
          th.classList.add("sortable");
        }

        const labelWrapper = document.createElement("span");
        labelWrapper.className = "sort-label";

        const labelSpan = document.createElement("span");
        labelSpan.className = "column-label";
        labelSpan.textContent = column.label;
        labelWrapper.appendChild(labelSpan);

        if (column.sortKey) {
          const indicator = document.createElement("span");
          indicator.className = "sort-indicator";
          indicator.setAttribute("data-sort-key", column.sortKey);
          labelWrapper.appendChild(indicator);
        }

        th.appendChild(labelWrapper);

        if (column.resizable) {
          const handle = document.createElement("div");
          handle.className = "column-resize-handle";
          handle.setAttribute("data-column-id", column.id);
          th.appendChild(handle);
        }

        state.headerRow.appendChild(th);
        state.headerCells.set(column.id, th);
        applyColumnWidth(column, getColumnWidth(column));
      });

      attachResizeHandlers();
      updateSortState();
    }

    function startResize(event, column) {
      if (!column) return;
      event.preventDefault();
      event.stopPropagation();

      const pointerX =
        event.touches && event.touches[0]
          ? event.touches[0].clientX
          : event.clientX;
      const headerCell = state.headerCells.get(column.id);
      if (!headerCell) return;

      const startRect = headerCell.getBoundingClientRect();
      const startWidth = startRect.width;
      let latestWidth = startWidth;
      const handle = event.currentTarget;

      const move = (moveEvent) => {
        const clientX =
          moveEvent.touches && moveEvent.touches[0]
            ? moveEvent.touches[0].clientX
            : moveEvent.clientX;
        const delta = clientX - pointerX;
        latestWidth = clampWidth(startWidth + delta, column);
        applyColumnWidth(column, latestWidth);
        if (moveEvent.cancelable) {
          moveEvent.preventDefault();
        }
      };

      const stop = () => {
        document.removeEventListener("mousemove", move);
        document.removeEventListener("mouseup", stop);
        document.removeEventListener("touchmove", move);
        document.removeEventListener("touchend", stop);
        document.removeEventListener("touchcancel", stop);
        document.body.classList.remove("column-resizing");
        setFrozenLayout(false);
        if (handle) {
          handle.classList.remove("is-active");
        }
        persistWidths();
        if (state.onLayoutChange) {
          state.onLayoutChange({
            type: "resize",
            columnId: column.id,
            width: latestWidth,
          });
        }
      };

      document.addEventListener("mousemove", move);
      document.addEventListener("mouseup", stop);
      document.addEventListener("touchmove", move, { passive: false });
      document.addEventListener("touchend", stop);
      document.addEventListener("touchcancel", stop);

      document.body.classList.add("column-resizing");
      setFrozenLayout(true);
      if (handle) {
        handle.classList.add("is-active");
      }
    }

    function attachResizeHandlers() {
      const handles = state.headerRow.querySelectorAll(".column-resize-handle");
      handles.forEach((handle) => {
        const columnId = handle.getAttribute("data-column-id");
        const column = state.columnMap.get(columnId);
        if (!column) return;
        handle.addEventListener("mousedown", (event) => {
          startResize(event, column);
        });
        handle.addEventListener(
          "touchstart",
          (event) => {
            startResize(event, column);
          },
          { passive: false }
        );
      });
    }

    function updateSortState() {
      const sortKey = state.sortState ? state.sortState.key : null;
      const sortDir = state.sortState ? state.sortState.direction : null;

      state.headerCells.forEach((th) => {
        if (!th) return;
        if (th.dataset.sortKey === sortKey) {
          const ariaValue = sortDir === "desc" ? "descending" : "ascending";
          th.setAttribute("aria-sort", ariaValue);
        } else {
          th.removeAttribute("aria-sort");
        }
      });
    }

    function rebuildLayout() {
      buildColgroup();
      buildHeaderRow();
    }

    rebuildLayout();

    return {
      getAllColumns() {
        return state.columns.slice();
      },
      getVisibleColumns,
      refresh() {
        rebuildLayout();
      },
      setVisibleColumns(ids) {
        const sanitized = sanitizeVisibleIds(
          state.columns,
          Array.isArray(ids) ? ids : [],
          state.defaultVisibleIds
        );
        if (arraysEqual(state.visibleIds, sanitized)) {
          return false;
        }
        state.visibleIds = sanitized;
        persistVisibleIds(storageKey, sanitized);
        rebuildLayout();
        if (state.onLayoutChange) {
          state.onLayoutChange({ type: "visibility" });
        }
        return true;
      },
      resetVisibility() {
        return this.setVisibleColumns(state.defaultVisibleIds);
      },
      getColumnById(columnId) {
        return state.columnMap.get(columnId) || null;
      },
      getColumnWidth(columnId) {
        return getColumnWidth(state.columnMap.get(columnId));
      },
      setSortState(sortKey, sortDir) {
        if (sortKey && typeof sortKey === "object") {
          state.sortState = {
            key: sortKey.key,
            direction: sortKey.direction,
          };
        } else {
          state.sortState = sortKey
            ? { key: sortKey, direction: sortDir }
            : null;
        }
        updateSortState();
      },
    };
  }

  window.TableColumnManager = {
    create(options) {
      return new TableColumnManager(options);
    },
  };
})();

// Global Polling Interval Manager
window.PollingManager = {
  _interval: null,
  _listeners: [],

  init() {
    // Load saved interval or default to 1000ms
    this._interval = AppState.load("pollingInterval", 1000);
    console.log(
      "[PollingManager] Initialized with interval:",
      this._interval,
      "ms"
    );
  },

  getInterval() {
    if (this._interval === null) {
      this.init();
    }
    return this._interval;
  },

  setInterval(ms) {
    const oldInterval = this._interval;
    this._interval = ms;
    AppState.save("pollingInterval", ms);
    console.log(
      "[PollingManager] Interval changed from",
      oldInterval,
      "ms to",
      ms,
      "ms"
    );

    // Notify all listeners
    this._listeners.forEach((callback) => {
      try {
        callback(ms, oldInterval);
      } catch (err) {
        console.error("[PollingManager] Listener callback failed:", err);
      }
    });
  },

  onChange(callback) {
    if (typeof callback === "function") {
      this._listeners.push(callback);
    }
    return callback;
  },

  removeListener(callback) {
    const index = this._listeners.indexOf(callback);
    if (index > -1) {
      this._listeners.splice(index, 1);
    }
  },
};

// Initialize on load
PollingManager.init();

// Shared polling lifecycle helper for page controllers
window.PagePoller = {
  create(options = {}) {
    const { label = "Poller", onPoll, getInterval } = options;
    if (typeof onPoll !== "function") {
      throw new Error(`[PagePoller:${label}] onPoll callback is required`);
    }

    const state = {
      timerId: null,
      listener: null,
      active: false,
    };

    const logPrefix = `[PagePoller:${label}]`;

    const computeInterval = () => {
      if (typeof getInterval === "function") {
        try {
          const value = Number(getInterval());
          if (Number.isFinite(value) && value > 0) {
            return value;
          }
        } catch (err) {
          console.warn(`${logPrefix} getInterval failed, falling back`, err);
        }
      }

      if (
        window.PollingManager &&
        typeof window.PollingManager.getInterval === "function"
      ) {
        try {
          const value = Number(window.PollingManager.getInterval());
          if (Number.isFinite(value) && value > 0) {
            return value;
          }
        } catch (err) {
          console.warn(
            `${logPrefix} PollingManager.getInterval failed, using default`,
            err
          );
        }
      }

      return 1000;
    };

    const schedule = () => {
      const interval = computeInterval();
      state.timerId = setInterval(() => {
        try {
          const result = onPoll();
          if (result && typeof result.then === "function") {
            Promise.resolve(result).catch((error) => {
              console.error(`${logPrefix} Poll callback rejected`, error);
            });
          }
        } catch (error) {
          console.error(`${logPrefix} Poll callback threw`, error);
        }
      }, interval);

      if (window.Router && typeof Router.trackInterval === "function") {
        Router.trackInterval(state.timerId);
      }

      state.active = true;
      return interval;
    };

    const stop = (options = {}) => {
      if (!state.timerId) {
        state.active = false;
        return;
      }

      clearInterval(state.timerId);
      state.timerId = null;
      state.active = false;

      if (!options.silent) {
        console.log(`${logPrefix} Stopped polling`);
      }
    };

    const start = (options = {}) => {
      stop({ silent: true });
      const interval = schedule();
      ensureListener();

      if (!options.silent) {
        console.log(`${logPrefix} Started polling every ${interval} ms`);
      }

      return interval;
    };

    const ensureListener = () => {
      if (
        !window.PollingManager ||
        typeof window.PollingManager.onChange !== "function" ||
        state.listener
      ) {
        return;
      }

      state.listener = window.PollingManager.onChange(() => {
        if (!state.active) {
          return;
        }
        const interval = start({ silent: true });
        console.log(`${logPrefix} Polling interval changed → ${interval} ms`);
      });
    };

    const restart = () => {
      const interval = start({ silent: true });
      console.log(`${logPrefix} Restarted polling (${interval} ms)`);
      return interval;
    };

    const cleanup = () => {
      stop();
      if (
        state.listener &&
        window.PollingManager &&
        typeof window.PollingManager.removeListener === "function"
      ) {
        window.PollingManager.removeListener(state.listener);
      }
      state.listener = null;
    };

    return {
      start,
      stop: () => stop(),
      restart,
      cleanup,
      isActive: () => state.active,
    };
  },
};

// Page lifecycle registry – centralizes init/activate/deactivate/dispose flows per page
const PageLifecycleRegistry = (() => {
  const lifecycles = new Map();
  const contexts = new Map();

  const noop = () => {};

  const defaultLifecycle = {
    init: noop,
    activate: noop,
    deactivate: noop,
    dispose: noop,
  };

  const safeInvokeAll = (callbacks) => {
    callbacks.forEach((callback) => {
      try {
        callback();
      } catch (error) {
        console.error("[PageLifecycle] Cleanup handler failed", error);
      }
    });
  };

  const createContext = (pageName) => {
    const deactivateCleanups = new Set();
    const disposeCleanups = new Set();
    let active = false;

    const register = (store, callback) => {
      if (typeof callback !== "function") {
        return () => {};
      }
      store.add(callback);
      return () => store.delete(callback);
    };

    const context = {
      pageName,
      data: {},
      isActive: () => active,
      onDeactivate(callback) {
        return register(deactivateCleanups, callback);
      },
      onDispose(callback) {
        return register(disposeCleanups, callback);
      },
      managePoller(poller) {
        if (!poller || typeof poller !== "object") {
          return poller;
        }
        this.onDeactivate(() => {
          if (typeof poller.stop === "function") {
            poller.stop({ silent: true });
          }
        });
        this.onDispose(() => {
          if (typeof poller.cleanup === "function") {
            poller.cleanup();
          }
        });
        return poller;
      },
      createAbortController() {
        const controller = new AbortController();
        const abort = () => {
          try {
            controller.abort();
          } catch (error) {
            console.error(
              "[PageLifecycle] Abort controller cleanup failed",
              error
            );
          }
        };
        this.onDeactivate(abort);
        this.onDispose(abort);
        return controller;
      },
      __setActive(value) {
        active = Boolean(value);
      },
      __runDeactivateCleanups() {
        if (!deactivateCleanups.size) {
          return;
        }
        const callbacks = Array.from(deactivateCleanups);
        deactivateCleanups.clear();
        safeInvokeAll(callbacks);
      },
      __runDisposeCleanups() {
        if (!disposeCleanups.size) {
          return;
        }
        const callbacks = Array.from(disposeCleanups);
        disposeCleanups.clear();
        safeInvokeAll(callbacks);
      },
    };

    return context;
  };

  const getLifecycle = (pageName) => {
    const lifecycle = lifecycles.get(pageName);
    if (!lifecycle) {
      return null;
    }
    return {
      init: lifecycle.init || noop,
      activate: lifecycle.activate || noop,
      deactivate: lifecycle.deactivate || noop,
      dispose: lifecycle.dispose || noop,
    };
  };

  const getOrCreateContext = (pageName) => {
    if (!contexts.has(pageName)) {
      contexts.set(pageName, createContext(pageName));
    }
    return contexts.get(pageName);
  };

  return {
    register(pageName, lifecycle = {}) {
      if (typeof pageName !== "string" || !pageName.trim()) {
        throw new Error("PageLifecycleRegistry.register requires a page id");
      }
      const normalized = {
        ...defaultLifecycle,
        ...lifecycle,
      };
      lifecycles.set(pageName, normalized);
    },

    has(pageName) {
      return lifecycles.has(pageName);
    },

    getContext(pageName) {
      return getOrCreateContext(pageName);
    },

    init(pageName) {
      const lifecycle = getLifecycle(pageName);
      if (!lifecycle) {
        return;
      }
      const context = getOrCreateContext(pageName);
      if (context.__initialized) {
        return;
      }
      context.__initialized = true;
      try {
        lifecycle.init(context);
      } catch (error) {
        console.error(`[PageLifecycle] init failed for ${pageName}`, error);
      }
    },

    activate(pageName) {
      const lifecycle = getLifecycle(pageName);
      if (!lifecycle) {
        return;
      }
      const context = getOrCreateContext(pageName);
      this.init(pageName);
      context.__setActive(true);
      try {
        lifecycle.activate(context);
      } catch (error) {
        console.error(`[PageLifecycle] activate failed for ${pageName}`, error);
      }
    },

    deactivate(pageName) {
      const lifecycle = getLifecycle(pageName);
      if (!lifecycle) {
        return;
      }
      const context = getOrCreateContext(pageName);
      if (!context.__initialized) {
        return;
      }
      try {
        lifecycle.deactivate(context);
      } catch (error) {
        console.error(
          `[PageLifecycle] deactivate failed for ${pageName}`,
          error
        );
      } finally {
        context.__setActive(false);
        context.__runDeactivateCleanups();
      }
    },

    dispose(pageName) {
      const lifecycle = getLifecycle(pageName);
      if (!lifecycle) {
        return;
      }
      const context = getOrCreateContext(pageName);
      if (!context.__initialized) {
        lifecycles.delete(pageName);
        contexts.delete(pageName);
        return;
      }
      try {
        lifecycle.dispose(context);
      } catch (error) {
        console.error(`[PageLifecycle] dispose failed for ${pageName}`, error);
      } finally {
        context.__runDeactivateCleanups();
        context.__runDisposeCleanups();
        lifecycles.delete(pageName);
        contexts.delete(pageName);
      }
    },
  };
})();

window.PageRegistry = PageLifecycleRegistry;

if (Array.isArray(window.__pendingPageRegistrations)) {
  const registrations = window.__pendingPageRegistrations.splice(0);
  registrations.forEach((pending) => {
    try {
      pending(window.PageRegistry);
    } catch (error) {
      console.error("[PageLifecycle] Pending registration failed", error);
    }
  });
}

// Client-Side Router - SPA Architecture
window.Router = {
  currentPage: null,
  cleanupHandlers: [],
  _timeoutMs: 10000,
  pageCache: {}, // Cache page containers
  initializedPages: {}, // Track which pages have been initialized

  setActiveTab(pageName) {
    document.querySelectorAll("nav .tab").forEach((tab) => {
      const tabPage = tab.getAttribute("data-page");
      if (tabPage === pageName) {
        tab.classList.add("active");
      } else {
        tab.classList.remove("active");
      }
    });
  },

  deactivatePageLifecycle(pageName) {
    if (!pageName || !window.PageRegistry) {
      return;
    }
    try {
      if (typeof window.PageRegistry.deactivate === "function") {
        window.PageRegistry.deactivate(pageName);
      }
    } catch (error) {
      console.error(
        `[Router] Failed to deactivate lifecycle for ${pageName}`,
        error
      );
    }
  },

  activatePageLifecycle(pageName) {
    if (!pageName || !window.PageRegistry) {
      return;
    }
    try {
      if (typeof window.PageRegistry.activate === "function") {
        window.PageRegistry.activate(pageName);
      }
    } catch (error) {
      console.error(
        `[Router] Failed to activate lifecycle for ${pageName}`,
        error
      );
    }
  },

  registerCleanup(handler) {
    if (typeof handler === "function") {
      this.cleanupHandlers.push(handler);
    }
    return handler;
  },

  runCleanupHandlers() {
    while (this.cleanupHandlers.length) {
      const handler = this.cleanupHandlers.pop();
      try {
        handler();
      } catch (err) {
        console.error("[Router] Cleanup handler failed:", err);
      }
    }
  },

  trackInterval(intervalId) {
    if (intervalId != null) {
      this.registerCleanup(() => clearInterval(intervalId));
    }
    return intervalId;
  },

  trackTimeout(timeoutId) {
    if (timeoutId != null) {
      this.registerCleanup(() => clearTimeout(timeoutId));
    }
    return timeoutId;
  },

  removeCachedPageElements(mainContent) {
    if (!mainContent) {
      return;
    }

    Object.values(this.pageCache).forEach((el) => {
      if (el && el.parentElement === mainContent) {
        mainContent.removeChild(el);
        el.style.display = "none";
      }
    });
  },

  displayPageElement(mainContent, pageEl) {
    if (!mainContent || !pageEl) {
      return;
    }

    this.removeCachedPageElements(mainContent);
    pageEl.style.display = "";
    mainContent.appendChild(pageEl);
  },

  async loadPage(pageName) {
    if (!pageName) {
      return;
    }

    console.log("[Router] Loading page:", pageName);

    const previousPage = this.currentPage;
    if (previousPage) {
      this.deactivatePageLifecycle(previousPage);
    }

    this.runCleanupHandlers();

    this.currentPage = pageName;
    this.setActiveTab(pageName);

    const mainContent = document.querySelector("main");
    if (!mainContent) {
      console.error("[Router] Main content container not found");
      return;
    }

    // Remove any unresolved loading placeholders from prior attempts
    mainContent.querySelectorAll(".page-loading").forEach((el) => el.remove());

    // Cached page path – reuse existing container
    const cachedEl = this.pageCache[pageName];
    if (cachedEl) {
      console.log("[Router] Using cached page:", pageName);

      this.displayPageElement(mainContent, cachedEl);

      cleanupTabContainers();
      this.initPageScripts(pageName);
      this.activatePageLifecycle(pageName);

      const targetUrl = pageName === "home" ? "/" : `/${pageName}`;
      if (window.location.pathname !== targetUrl) {
        window.history.pushState({ page: pageName }, "", targetUrl);
      }

      AppState.save("lastTab", pageName);
      console.log("[Router] Cached page displayed:", pageName);
      return;
    }

    // Page not cached – show loading state and fetch content
    mainContent.setAttribute("data-loading", "true");
    this.removeCachedPageElements(mainContent);

    const loadingEl = document.createElement("div");
    loadingEl.className = "page-loading";
    loadingEl.style.cssText = "padding: 2rem; text-align: center;";
    loadingEl.innerHTML = `
      <div style="font-size: 1.1rem; color: var(--text-secondary);">
        Loading ${pageName}...
      </div>
    `;

    Object.values(this.pageCache).forEach((el) => {
      el.style.display = "none";
    });
    mainContent.appendChild(loadingEl);

    try {
      const html = await this.fetchPageContent(pageName, this._timeoutMs);

      const pageEl = document.createElement("div");
      pageEl.className = "page-container";
      pageEl.id = `page-${pageName}`;
      pageEl.setAttribute("data-page", pageName);
      pageEl.innerHTML = html;

      this.pageCache[pageName] = pageEl;

      loadingEl.remove();
      this.displayPageElement(mainContent, pageEl);
      this.executeEmbeddedScripts(pageEl);

      cleanupTabContainers();
      this.initPageScripts(pageName);
      this.activatePageLifecycle(pageName);

      const targetUrl = pageName === "home" ? "/" : `/${pageName}`;
      if (window.location.pathname !== targetUrl) {
        window.history.pushState({ page: pageName }, "", targetUrl);
      }

      AppState.save("lastTab", pageName);
      console.log("[Router] New page loaded and cached:", pageName);
    } catch (error) {
      console.error("[Router] Failed to load page:", pageName, error);

      loadingEl.innerHTML = `
        <div style="padding: 2rem; text-align: center;">
          <h2 style="color: #ef4444;">⚠️ Failed to Load Page</h2>
          <p style="color: #9ca3af; margin-top: 1rem;">
            ${error.message}
          </p>
          <button onclick="Router.loadPage('${pageName}')"
            style="margin-top: 1rem; padding: 0.5rem 1rem;
                   background: #3b82f6; color: white; border: none;
                   border-radius: 0.5rem; cursor: pointer;">
            Retry
          </button>
        </div>
      `;

      if (previousPage) {
        this.currentPage = previousPage;
        this.setActiveTab(previousPage);
        this.activatePageLifecycle(previousPage);
        AppState.save("lastTab", previousPage);
      }

      cleanupTabContainers();
    } finally {
      mainContent.removeAttribute("data-loading");
    }
  },

  async fetchPageContent(pageName, timeoutMs = 10000) {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
    const url = `/api/pages/${pageName}`;

    try {
      const response = await fetch(url, {
        signal: controller.signal,
        headers: { "X-Requested-With": "fetch" },
      });
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      return await response.text();
    } catch (error) {
      if (error.name === "AbortError") {
        throw new Error(
          `Request timed out after ${Math.round(timeoutMs / 1000)}s`
        );
      }
      throw error;
    } finally {
      clearTimeout(timeoutId);
    }
  },

  executeEmbeddedScripts(container) {
    if (!container) return;

    const scripts = container.querySelectorAll("script");
    scripts.forEach((script) => {
      const newScript = document.createElement("script");
      Array.from(script.attributes).forEach((attr) =>
        newScript.setAttribute(attr.name, attr.value)
      );

      if (script.src) {
        newScript.src = script.src;
      } else {
        newScript.textContent = script.textContent;
      }

      script.parentNode?.replaceChild(newScript, script);
    });
  },

  initPageScripts(pageName) {
    // Only initialize each page once
    if (this.initializedPages[pageName]) {
      return;
    }

    console.log("[Router] Initializing page:", pageName);

    const hasLifecycle =
      Boolean(window.PageRegistry) &&
      typeof window.PageRegistry.has === "function" &&
      window.PageRegistry.has(pageName);

    if (hasLifecycle) {
      this.initializedPages[pageName] = true;
      return;
    }

    // Unmigrated pages: no initialization until PageLifecycleRegistry is implemented
    // Pages without lifecycle will not work until migrated
    console.warn(
      `[Router] Page "${pageName}" has no lifecycle - migration required`
    );
    this.initializedPages[pageName] = true;
  },
};

// Initialize router on page load
document.addEventListener("DOMContentLoaded", () => {
  // Set initial page from URL
  const currentPath = window.location.pathname;
  const initialPage = currentPath === "/" ? "home" : currentPath.substring(1);
  Router.currentPage = initialPage;

  const mainContent = document.querySelector("main");
  if (mainContent) {
    let initialContainer = mainContent.querySelector(".page-container");

    if (!initialContainer) {
      initialContainer = document.createElement("div");
      initialContainer.className = "page-container";
      initialContainer.id = `page-${initialPage}`;
      initialContainer.setAttribute("data-page", initialPage);

      while (mainContent.firstChild) {
        initialContainer.appendChild(mainContent.firstChild);
      }

      mainContent.appendChild(initialContainer);
    }

    Router.pageCache[initialPage] = initialContainer;
    Router.initializedPages[initialPage] = true;

    if (typeof Router.initPageScripts === "function") {
      Router.initPageScripts(initialPage);
    }

    if (typeof Router.activatePageLifecycle === "function") {
      Router.activatePageLifecycle(initialPage);
    }
  }

  if (typeof Router.setActiveTab === "function") {
    Router.setActiveTab(initialPage);
  }

  // Intercept all navigation link clicks
  document.addEventListener("click", (e) => {
    const target = e.target.closest("a[data-page]");
    if (target) {
      e.preventDefault();
      const pageName = target.getAttribute("data-page");
      Router.loadPage(pageName);
    }
  });

  // Handle browser back/forward buttons
  window.addEventListener("popstate", (e) => {
    if (e.state && e.state.page) {
      Router.loadPage(e.state.page);
    } else {
      const path = window.location.pathname;
      const page = path === "/" ? "home" : path.substring(1);
      Router.loadPage(page);
    }
  });

  // Save initial state
  AppState.save("lastTab", initialPage);
  cleanupTabContainers();

  console.log("[Router] Initialized - SPA mode active");
});

// Helper function to hide sub-tabs/toolbar containers
function cleanupTabContainers() {
  const subTabsContainer = document.getElementById("subTabsContainer");
  const toolbarContainer = document.getElementById("toolbarContainer");

  // Only hide if not on a page that explicitly shows them
  // Pages can call initPageSubTabs() to show and populate them
  const activePage =
    (window.Router && Router.currentPage) ||
    (window.location.pathname === "/"
      ? "home"
      : window.location.pathname.substring(1));

  const pagesWithSubTabs = ["tokens", "status", "wallet"]; // Add more as needed
  const pagesWithToolbar = ["tokens"];

  if (!pagesWithSubTabs.includes(activePage)) {
    if (subTabsContainer) {
      subTabsContainer.style.display = "none";
      subTabsContainer.innerHTML = "";
      subTabsContainer.removeAttribute("data-page");
    }
  } else {
    // If switching between pages that both have sub-tabs, clear if owner changed
    if (subTabsContainer) {
      const currentOwner = subTabsContainer.getAttribute("data-page");
      if (currentOwner && currentOwner !== activePage) {
        subTabsContainer.innerHTML = "";
        subTabsContainer.style.display = "none";
        subTabsContainer.removeAttribute("data-page");
      }
    }
  }

  if (!pagesWithToolbar.includes(activePage)) {
    if (toolbarContainer) {
      toolbarContainer.style.display = "none";
      toolbarContainer.innerHTML = "";
    }
  }
}

let statusPollInterval = null;

function setBotBadge(state, message) {
  const badge = document.getElementById("botBadge");
  if (!badge) return;

  switch (state) {
    case "running":
      badge.className = "badge online";
      badge.innerHTML = message || "🤖 Running";
      badge.title = "Bot: Running";
      break;
    case "stopped":
      badge.className = "badge error";
      badge.innerHTML = message || "🤖 Stopped";
      badge.title = "Bot: Stopped";
      break;
    case "error":
      badge.className = "badge error";
      badge.innerHTML = message || "🤖 Error";
      badge.title = "Bot: Error";
      break;
    case "starting":
      badge.className = "badge loading";
      badge.innerHTML = message || "🤖 Starting";
      badge.title = "Bot: Starting...";
      break;
    default:
      badge.className = "badge loading";
      badge.innerHTML = message || "🤖 BOT";
      badge.title = "Bot: Unknown";
      break;
  }
}

function deriveAllReady(payload) {
  if (!payload || typeof payload !== "object") return null;

  if (typeof payload.all_ready === "boolean") {
    return payload.all_ready;
  }

  if (payload.services) {
    if (typeof payload.services.all_ready === "boolean") {
      return payload.services.all_ready;
    }

    const serviceStatuses = Object.values(payload.services);
    if (serviceStatuses.length > 0) {
      return serviceStatuses.every(
        (status) =>
          typeof status === "string" && status.toLowerCase().includes("healthy")
      );
    }
  }

  return null;
}

function renderStatusBadgesFromSnapshot(snapshot) {
  if (!snapshot || typeof snapshot !== "object") {
    setBotBadge("error", "🤖 Error");
    return;
  }

  // Update bot status badge
  const allReady = deriveAllReady(snapshot);
  const tradingEnabled = snapshot.trading_enabled;

  if (allReady === true) {
    if (tradingEnabled === true) {
      setBotBadge("running", "🤖 Running");
    } else if (tradingEnabled === false) {
      setBotBadge("stopped", "🤖 Stopped");
    } else {
      setBotBadge("running", "🤖 Ready");
    }
  } else if (allReady === false) {
    setBotBadge("starting", "🤖 Starting");
  } else {
    setBotBadge("starting", "🤖 Connecting");
  }
}

async function fetchStatusSnapshot() {
  try {
    const res = await fetch("/api/status");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const data = await res.json();
    renderStatusBadgesFromSnapshot(data);
    return data;
  } catch (error) {
    console.warn("Failed to fetch status snapshot:", error);
    setBotBadge("error", "🤖 Error");
    return null;
  }
}

function startStatusPolling(intervalMs = 5000) {
  if (statusPollInterval) return;
  statusPollInterval = setInterval(fetchStatusSnapshot, intervalMs);
}

function stopStatusPolling() {
  if (!statusPollInterval) return;
  clearInterval(statusPollInterval);
  statusPollInterval = null;
}

// Initialize (refresh silently every 1s)
fetchStatusSnapshot();
startStatusPolling();

if (window.Router && typeof Router.registerCleanup === "function") {
  Router.registerCleanup(() => {
    stopStatusPolling();
  });
}

// =============================================================================
// TRADER CONTROL SYSTEM
// =============================================================================

let traderStatusPollInterval = null;
let isReconnecting = false;

// Initialize trader controls on page load
function initializeTraderControls() {
  const traderToggle = document.getElementById("traderToggle");
  const rebootBtn = document.getElementById("rebootBtn");

  if (traderToggle) {
    traderToggle.addEventListener("click", toggleTrader);
  }

  if (rebootBtn) {
    rebootBtn.addEventListener("click", rebootSystem);
  }

  // Start polling trader status
  updateTraderStatus();
  traderStatusPollInterval = setInterval(updateTraderStatus, 2000);

  if (window.Router && typeof Router.registerCleanup === "function") {
    Router.registerCleanup(() => {
      if (traderStatusPollInterval) {
        clearInterval(traderStatusPollInterval);
        traderStatusPollInterval = null;
      }
    });
  }
}

// Update trader status from API
async function updateTraderStatus() {
  if (isReconnecting) return; // Skip during reconnect

  try {
    const res = await fetch("/api/trader/status");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);

    const data = await res.json();
    const status = data.data || data;

    updateTraderUI(status.enabled, status.running);
  } catch (error) {
    console.warn("Failed to fetch trader status:", error);
    // Don't update UI on transient network errors
  }
}

// Update trader UI based on status
function updateTraderUI(enabled, running) {
  const traderToggle = document.getElementById("traderToggle");
  const traderIcon = document.getElementById("traderIcon");
  const traderText = document.getElementById("traderText");

  if (!traderToggle || !traderIcon || !traderText) return;

  // Remove existing state classes
  traderToggle.classList.remove("running", "stopped");

  if (enabled && running) {
    traderToggle.classList.add("running");
    traderIcon.textContent = "▶️";
    traderText.textContent = "Trader Running";
  } else {
    traderToggle.classList.add("stopped");
    traderIcon.textContent = "⏸️";
    traderText.textContent = "Trader Stopped";
  }

  traderToggle.disabled = false;
}

// Toggle trader on/off
async function toggleTrader() {
  const traderToggle = document.getElementById("traderToggle");
  const traderIcon = document.getElementById("traderIcon");
  const traderText = document.getElementById("traderText");

  if (!traderToggle) return;

  // Determine current state from UI
  const isRunning = traderToggle.classList.contains("running");
  const endpoint = isRunning ? "/api/trader/stop" : "/api/trader/start";
  const action = isRunning ? "Stopping" : "Starting";

  // Disable button and show loading state
  traderToggle.disabled = true;
  traderIcon.textContent = "⏳";
  traderText.textContent = `${action}...`;

  try {
    const res = await fetch(endpoint, { method: "POST" });
    const data = await res.json();

    if (!res.ok || !data.success) {
      throw new Error(data.error || data.message || "Request failed");
    }

    // Update UI based on response
    const status = data.status || data.data?.status || {};
    updateTraderUI(status.enabled, status.running);

    const message =
      data.message ||
      data.data?.message ||
      (isRunning
        ? "Trader stopped successfully"
        : "Trader started successfully");
    Utils.showToast(`✅ ${message}`);

    // Immediate status refresh
    setTimeout(updateTraderStatus, 500);
  } catch (error) {
    console.error("Trader toggle error:", error);
    Utils.showToast(
      `❌ Failed to ${isRunning ? "stop" : "start"} trader: ${error.message}`,
      "error"
    );

    // Restore previous state
    updateTraderUI(isRunning, isRunning);
  }
}

// Reboot the entire system
async function rebootSystem() {
  const rebootBtn = document.getElementById("rebootBtn");
  if (!rebootBtn) return;

  // Confirm action
  if (
    !confirm(
      "⚠️ Are you sure you want to reboot ScreenerBot? This will restart the entire process."
    )
  ) {
    return;
  }

  // Disable button and show loading
  rebootBtn.disabled = true;
  const originalHTML = rebootBtn.innerHTML;
  rebootBtn.innerHTML = "<span>⏳</span><span>Rebooting...</span>";

  try {
    const res = await fetch("/api/system/reboot", { method: "POST" });
    const data = await res.json();

    if (!res.ok || !data.success) {
      throw new Error(data.error || "Reboot request failed");
    }

    Utils.showToast("🔄 System reboot initiated. Reconnecting...", "info");

    // Start reconnection attempts
    isReconnecting = true;
    if (traderStatusPollInterval) {
      clearInterval(traderStatusPollInterval);
      traderStatusPollInterval = null;
    }

    attemptReconnect();
  } catch (error) {
    console.error("Reboot error:", error);
    Utils.showToast(`❌ Failed to initiate reboot: ${error.message}`, "error");

    // Restore button
    rebootBtn.disabled = false;
    rebootBtn.innerHTML = originalHTML;
  }
}

// Attempt to reconnect after reboot
async function attemptReconnect() {
  const maxAttempts = 60; // 60 attempts = 2 minutes
  let attempt = 0;

  const checkConnection = async () => {
    attempt++;

    try {
      const res = await fetch("/api/status", {
        cache: "no-cache",
        signal: AbortSignal.timeout(3000),
      });

      if (res.ok) {
        Utils.showToast("✅ System reconnected successfully!");

        // Reload the page to refresh all state
        setTimeout(() => {
          window.location.reload();
        }, 1000);
        return;
      }
    } catch (error) {
      // Connection failed, continue trying
    }

    if (attempt < maxAttempts) {
      Utils.showToast(`🔄 Reconnecting... (${attempt}/${maxAttempts})`, "info");
      setTimeout(checkConnection, 2000);
    } else {
      Utils.showToast(
        "❌ Reconnection timeout. Please refresh the page manually.",
        "error"
      );
      isReconnecting = false;

      // Re-enable reboot button
      const rebootBtn = document.getElementById("rebootBtn");
      if (rebootBtn) {
        rebootBtn.disabled = false;
        rebootBtn.innerHTML = "<span>🔄</span><span>Reboot</span>";
      }
    }
  };

  // Wait 3 seconds before first attempt (give system time to restart)
  setTimeout(checkConnection, 3000);
}

// Show notification toast
function showNotification(message, type = "info") {
  Utils.showToast(message, type);
}

// Initialize refresh interval dropdown
function initializeRefreshInterval() {
  const dropdown = document.getElementById("refreshInterval");
  if (!dropdown) {
    console.warn("[PollingManager] Refresh interval dropdown not found");
    return;
  }

  // Set current value
  const currentInterval = PollingManager.getInterval();
  dropdown.value = currentInterval.toString();

  // Handle changes
  dropdown.addEventListener("change", (e) => {
    const newInterval = parseInt(e.target.value, 10);
    if (!isNaN(newInterval) && newInterval > 0) {
      PollingManager.setInterval(newInterval);
      Utils.showToast(
        `⏱️ Refresh interval set to ${formatInterval(newInterval)}`,
        "success"
      );
    }
  });

  console.log(
    "[PollingManager] Dropdown initialized with:",
    currentInterval,
    "ms"
  );
}

// Format interval for display
function formatInterval(ms) {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${ms / 1000}s`;
  return `${ms / 60000}m`;
}

// Initialize trader controls when DOM is ready
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", () => {
    initializeTraderControls();
    initializeRefreshInterval();
  });
} else {
  initializeTraderControls();
  initializeRefreshInterval();
}
