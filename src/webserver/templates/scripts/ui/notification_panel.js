// Notification drawer UI manager
import { notificationManager } from "../core/notifications.js";
import { toastManager } from "../core/toast.js";
import * as Utils from "../core/utils.js";
import { ConfirmationDialog } from "./confirmation_dialog.js";
import { enhanceAllSelects } from "./custom_select.js";
import { playPanelOpen, playPanelClose, playTabSwitch } from "../core/sounds.js";

let currentTab = "all";
let isInitialized = false;
let isOpen = false;

// Infinite scroll state
let pageSize = 30;
let currentOffset = 0;
let totalResults = 0;
let isLoadingMore = false;
let hasMoreData = true;
let loadedNotifications = [];

// Filter state
let currentFilters = {
  action_type: "",
  state: "",
};

// Event listener cleanup tracking
let handlers = {
  backdrop: null,
  closeBtn: null,
  keydown: null,
  tabs: [],
  filterActionType: null,
  filterState: null,
  clearFilters: null,
  markAllRead: null,
  clearAll: null,
  notificationList: null,
  scroll: null,
  backToTop: null,
};

// Subscription cleanup
let unsubscribe = null;

function escapeText(value) {
  return Utils.escapeHtml(value === undefined || value === null ? "" : String(value));
}

/**
 * Initialize notification drawer
 */
export function init() {
  if (isInitialized) {
    console.warn("[NotificationPanel] Already initialized, skipping");
    return;
  }

  setupTabs();
  setupActions();
  setupDrawerControls();
  setupFilters();
  setupInfiniteScroll();
  setupBackToTop();
  setupNotificationListDelegation();
  subscribeToUpdates();
  renderNotifications();
  toggleHistoryControls(currentTab);

  isInitialized = true;
}

/**
 * Open the notification drawer
 */
export function open() {
  const drawer = document.getElementById("notificationDrawer");
  if (!drawer) return;

  isOpen = true;
  drawer.setAttribute("data-state", "open");
  drawer.setAttribute("aria-hidden", "false");
  document.body.classList.add("notification-drawer-open");

  // Refresh with fresh data each time the drawer opens (DB-backed tabs refetch).
  resetScrollState();
  renderNotifications();

  // Play panel open sound
  playPanelOpen();

  // Notify toast manager about drawer state
  toastManager.onDrawerStateChange(true);

  // Mark all as read after short delay
  setTimeout(() => {
    notificationManager.markAllAsRead();
  }, 500);
}

/**
 * Close the notification drawer
 */
export function close() {
  const drawer = document.getElementById("notificationDrawer");
  if (!drawer) return;

  isOpen = false;
  drawer.setAttribute("data-state", "closed");
  drawer.setAttribute("aria-hidden", "true");
  document.body.classList.remove("notification-drawer-open");

  // Play panel close sound
  playPanelClose();

  // Notify toast manager about drawer state
  toastManager.onDrawerStateChange(false);
}

/**
 * Toggle drawer open/close
 */
export function toggle() {
  if (isOpen) {
    close();
  } else {
    open();
  }
}

/**
 * Setup drawer control handlers
 */
function setupDrawerControls() {
  const drawer = document.getElementById("notificationDrawer");
  const backdrop = drawer?.querySelector('[data-role="dismiss"]');
  const closeBtn = document.getElementById("notificationDrawerClose");

  if (backdrop) {
    handlers.backdrop = close;
    backdrop.addEventListener("click", handlers.backdrop);
  }

  if (closeBtn) {
    handlers.closeBtn = close;
    closeBtn.addEventListener("click", handlers.closeBtn);
  }

  handlers.keydown = (e) => {
    if (e.key === "Escape" && isOpen) {
      close();
    }
  };
  document.addEventListener("keydown", handlers.keydown);
}

/**
 * Setup tab switching
 */
function setupTabs() {
  const tabs = document.querySelectorAll(".notification-tab");
  tabs.forEach((tab) => {
    const handler = () => {
      currentTab = tab.dataset.tab;
      resetScrollState();
      setActiveTab(tab);
      playTabSwitch(); // Sound feedback for tab switch
      renderNotifications();
      toggleHistoryControls(currentTab);
    };
    handlers.tabs.push({ element: tab, handler });
    tab.addEventListener("click", handler);
  });
}

/**
 * Setup filter controls
 */
function setupFilters() {
  const filterActionType = document.getElementById("filterActionType");
  const filterState = document.getElementById("filterState");
  const clearFiltersBtn = document.getElementById("clearFiltersBtn");

  if (filterActionType) {
    handlers.filterActionType = () => {
      currentFilters.action_type = filterActionType.value;
      resetScrollState();
      renderNotifications();
    };
    filterActionType.addEventListener("change", handlers.filterActionType);
  }

  if (filterState) {
    handlers.filterState = () => {
      currentFilters.state = filterState.value;
      resetScrollState();
      renderNotifications();
    };
    filterState.addEventListener("change", handlers.filterState);
  }

  if (clearFiltersBtn) {
    handlers.clearFilters = () => {
      currentFilters = { action_type: "", state: "" };
      if (filterActionType) filterActionType.value = "";
      if (filterState) filterState.value = "";
      resetScrollState();
      renderNotifications();
    };
    clearFiltersBtn.addEventListener("click", handlers.clearFilters);
  }

  // Enhance native selects with custom styled dropdowns
  const filtersContainer = document.getElementById("notificationFilters");
  if (filtersContainer) {
    enhanceAllSelects(filtersContainer);
  }
}

/**
 * Setup infinite scroll
 */
function setupInfiniteScroll() {
  const list = document.getElementById("notificationList");
  if (!list) return;

  handlers.scroll = Utils.throttle(() => {
    if (!isOpen) return;
    if (currentTab !== "all" && currentTab !== "completed" && currentTab !== "failed") return;
    if (isLoadingMore || !hasMoreData) return;

    const scrollTop = list.scrollTop;
    const scrollHeight = list.scrollHeight;
    const clientHeight = list.clientHeight;

    // Load more when within 100px of bottom
    if (scrollTop + clientHeight >= scrollHeight - 100) {
      loadMoreNotifications();
    }

    // Show/hide back to top button
    updateBackToTopVisibility(scrollTop);
  }, 100);

  list.addEventListener("scroll", handlers.scroll);
}

/**
 * Setup back to top button
 */
function setupBackToTop() {
  const backToTopBtn = document.getElementById("backToTopBtn");
  if (!backToTopBtn) return;

  handlers.backToTop = () => {
    const list = document.getElementById("notificationList");
    if (list) {
      list.scrollTo({ top: 0, behavior: "smooth" });
    }
  };
  backToTopBtn.addEventListener("click", handlers.backToTop);
}

/**
 * Update back to top button visibility
 */
function updateBackToTopVisibility(scrollTop) {
  const backToTopBtn = document.getElementById("backToTopBtn");
  if (!backToTopBtn) return;

  if (scrollTop > 200) {
    backToTopBtn.style.display = "flex";
  } else {
    backToTopBtn.style.display = "none";
  }
}

/**
 * Reset scroll state for new queries
 */
function resetScrollState() {
  currentOffset = 0;
  totalResults = 0;
  hasMoreData = true;
  loadedNotifications = [];
}

/**
 * Toggle visibility of history controls (filters) for the given tab.
 *
 * Filters show on the DB-backed tabs (all/completed/failed). The state filter is
 * usable only on "all" — on completed/failed the state is fixed by the tab.
 */
function toggleHistoryControls(tab) {
  const showFilters = tab === "all" || tab === "completed" || tab === "failed";
  const stateLocked = tab === "completed" || tab === "failed";

  const filtersEl = document.getElementById("notificationFilters");
  const stateFilterEl = document.getElementById("filterState");

  if (filtersEl) {
    filtersEl.style.display = showFilters ? "block" : "none";
  }

  if (stateFilterEl) {
    if (stateLocked) {
      stateFilterEl.disabled = true;
      stateFilterEl.value = "";
      stateFilterEl.style.opacity = "0.5";
      stateFilterEl.style.cursor = "not-allowed";
      stateFilterEl.title = "State is controlled by tab";
    } else {
      stateFilterEl.disabled = false;
      stateFilterEl.style.opacity = "1";
      stateFilterEl.style.cursor = "pointer";
      stateFilterEl.title = "";
    }
  }
}

/**
 * Show/hide loading indicator
 */
function showLoading(show) {
  const loadingEl = document.getElementById("notificationLoading");
  if (loadingEl) {
    loadingEl.style.display = show ? "flex" : "none";
  }
}

/**
 * Setup panel actions
 */
function setupActions() {
  const markAllReadBtn = document.getElementById("markAllReadBtn");
  const clearAllBtn = document.getElementById("clearAllBtn");

  if (markAllReadBtn) {
    handlers.markAllRead = () => {
      notificationManager.markAllAsRead();
      Utils.showToast({ type: "success", title: "All marked as read" });
    };
    markAllReadBtn.addEventListener("click", handlers.markAllRead);
  }

  if (clearAllBtn) {
    handlers.clearAll = async () => {
      const { confirmed } = await ConfirmationDialog.show({
        title: "Clear notifications",
        message:
          "Dismiss all notifications from this list? They remain in the Completed/Failed history.",
        confirmLabel: "Clear",
        cancelLabel: "Cancel",
        variant: "warning",
      });

      if (confirmed) {
        notificationManager.clearAll();
        Utils.showToast("Notifications cleared", "info");
      }
    };
    clearAllBtn.addEventListener("click", handlers.clearAll);
  }
}

/**
 * Subscribe to notification updates
 */
function subscribeToUpdates() {
  unsubscribe = notificationManager.subscribe((event) => {
    if (event.type === "summary") {
      updateConnectionStatus(event.summary.connection);
    }

    updateTabCounts();

    if (
      event.type === "added" ||
      event.type === "updated" ||
      event.type === "dismissed" ||
      event.type === "cleared" ||
      event.type === "marked_read" ||
      event.type === "all_marked_read" ||
      event.type === "bulk_update" ||
      event.type === "history_synced"
    ) {
      // The live "active" tab always re-renders. DB-backed tabs (all/completed/
      // failed) are only refreshed when the list is scrolled to the top, so a
      // live update never yanks the scroll position out from under the user.
      if (currentTab === "active" || event.type === "cleared") {
        renderNotifications();
      } else {
        const list = document.getElementById("notificationList");
        if (list && list.scrollTop < 40 && (event.type === "added" || event.type === "updated")) {
          resetScrollState();
          renderNotifications();
        }
      }
    }

    if (event.type === "lag") {
      const skipped = event.payload?.skipped || 0;
      Utils.showToast(
        skipped > 0
          ? `Missed ${skipped} updates — refreshing…`
          : "Stream fell behind — refreshing…",
        "warning"
      );
    }

    if (event.type === "sync_error") {
      Utils.showToast(`Failed to refresh (${event.error || "unknown"})`, "warning");
    }
  });
}

/**
 * Update connection status indicator
 */
function updateConnectionStatus(connection) {
  const connectionEl = document.getElementById("notificationConnection");
  const textEl = document.getElementById("notificationConnectionText");

  if (!connectionEl || !textEl) return;

  if (!connection || connection.status !== "connected") {
    connectionEl.setAttribute("data-state", "disconnected");
    textEl.textContent = "Offline";
    return;
  }

  connectionEl.setAttribute("data-state", "connected");
  textEl.textContent = "Live";
}

/**
 * Set active tab
 */
function setActiveTab(activeTab) {
  const tabs = document.querySelectorAll(".notification-tab");
  tabs.forEach((tab) => {
    tab.classList.toggle("active", tab === activeTab);
  });
}

/**
 * Update tab counts
 */
function updateTabCounts() {
  // Prefer DB-accurate totals (full persisted history) from the server summary;
  // fall back to the in-memory tally before the first sync completes.
  const totals = notificationManager.getSummary().totals;

  if (totals) {
    updateCount("allCount", totals.all);
    updateCount("activeCount", totals.in_progress);
    updateCount("completedCount", totals.completed);
    updateCount("failedCount", totals.failed);
    return;
  }

  updateCount("allCount", notificationManager.getAll().length);
  updateCount("activeCount", notificationManager.getActive().length);
  updateCount("completedCount", notificationManager.getCompleted().length);
  updateCount("failedCount", notificationManager.getFailed().length);
}

/**
 * Update individual count badge
 */
function updateCount(elementId, count) {
  const el = document.getElementById(elementId);
  if (el) {
    el.textContent = count;
    el.style.display = count > 0 ? "inline" : "none";
  }
}

/**
 * Render notifications based on current tab
 */
async function renderNotifications() {
  updateTabCounts();

  const list = document.getElementById("notificationList");
  if (!list) return;

  let notifications = [];

  // All/Completed/Failed are DB-backed (full persisted history) with infinite
  // scroll; only the live "active" tab uses the in-memory cache. "all" passes no
  // state filter so every recorded action shows, newest first.
  if (currentTab === "all" || currentTab === "completed" || currentTab === "failed") {
    if (currentOffset === 0) {
      // Initial load
      try {
        isLoadingMore = true;
        showLoading(true);

        const options = {
          limit: pageSize,
          offset: 0,
        };

        if (currentTab === "completed") {
          options.state = "completed";
        } else if (currentTab === "failed") {
          options.state = "failed";
        } else if (currentTab === "all" && currentFilters.state) {
          // "All" honors the optional state dropdown filter.
          options.state = currentFilters.state;
        }

        if (currentFilters.action_type) {
          options.action_type = currentFilters.action_type;
        }

        const response = await notificationManager.fetchHistory(options);
        loadedNotifications = response.actions || [];
        totalResults = response.total || 0;
        currentOffset = loadedNotifications.length;
        hasMoreData = currentOffset < totalResults;

        notificationManager.syncFromHistory(loadedNotifications, { silent: true });

        isLoadingMore = false;
        showLoading(false);
      } catch (error) {
        console.error("[NotificationPanel] Failed to fetch history:", error);
        isLoadingMore = false;
        showLoading(false);
        list.innerHTML = `
          <div class="notification-empty">
            <i class="icon-triangle-alert"></i>
            <p>Failed to load</p>
          </div>
        `;
        return;
      }
    }

    notifications = loadedNotifications.map(mergeWithStoredState);
  } else {
    // Only the live "active" tab uses the in-memory cache.
    notifications = notificationManager.getActive().map((n) => ({ ...n }));
  }

  // Apply UI state from cache
  notifications = notifications.map(mergeWithStoredState);

  // The live "active" tab hides client-dismissed items; the DB-backed history
  // tabs always show the full persisted record.
  if (currentTab === "active") {
    notifications = notifications.filter((n) => !n.dismissed);
  }

  // Sort by timestamp (newest first)
  notifications.sort((a, b) => {
    const timeA = new Date(resolveTimestamp(a)).getTime();
    const timeB = new Date(resolveTimestamp(b)).getTime();
    return timeB - timeA;
  });

  if (notifications.length === 0) {
    list.innerHTML = `
      <div class="notification-empty">
        <i class="icon-inbox"></i>
        <p>No ${currentTab === "all" ? "" : currentTab + " "}actions</p>
      </div>
    `;
    return;
  }

  list.innerHTML = notifications.map((n) => renderNotification(n)).join("");
}

/**
 * Load more notifications for infinite scroll
 */
async function loadMoreNotifications() {
  if (isLoadingMore || !hasMoreData) return;
  if (currentTab !== "all" && currentTab !== "completed" && currentTab !== "failed") return;

  try {
    isLoadingMore = true;
    showLoading(true);

    const options = {
      limit: pageSize,
      offset: currentOffset,
    };

    if (currentTab === "completed") {
      options.state = "completed";
    } else if (currentTab === "failed") {
      options.state = "failed";
    } else if (currentTab === "all" && currentFilters.state) {
      options.state = currentFilters.state;
    }

    if (currentFilters.action_type) {
      options.action_type = currentFilters.action_type;
    }

    const response = await notificationManager.fetchHistory(options);
    const newNotifications = response.actions || [];
    totalResults = response.total || 0;

    if (newNotifications.length > 0) {
      loadedNotifications = [...loadedNotifications, ...newNotifications];
      currentOffset = loadedNotifications.length;
      hasMoreData = currentOffset < totalResults;

      notificationManager.syncFromHistory(newNotifications, { silent: true });

      // Append new items to DOM
      const list = document.getElementById("notificationList");
      if (list) {
        const newHtml = newNotifications
          .map(mergeWithStoredState)
          .map((n) => renderNotification(n))
          .join("");
        list.insertAdjacentHTML("beforeend", newHtml);
      }
    } else {
      hasMoreData = false;
    }

    isLoadingMore = false;
    showLoading(false);
  } catch (error) {
    console.error("[NotificationPanel] Failed to load more:", error);
    isLoadingMore = false;
    showLoading(false);
  }
}

function mergeWithStoredState(notification) {
  if (!notification || !notification.id) {
    return notification;
  }

  const stored = notificationManager.getNotification(notification.id);
  if (!stored) {
    return {
      ...notification,
      read: notification.read ?? false,
      dismissed: notification.dismissed ?? false,
      timestamp:
        notification.completed_at ||
        notification.timestamp ||
        notification.started_at ||
        new Date().toISOString(),
    };
  }

  const merged = {
    ...stored,
    ...notification,
  };

  merged.read = stored.read;
  merged.dismissed = stored.dismissed;
  merged.timestamp =
    stored.timestamp ||
    notification.completed_at ||
    notification.timestamp ||
    notification.started_at ||
    new Date().toISOString();

  return merged;
}

/**
 * Render single notification
 */
function renderNotification(notification) {
  const { id, action_type, state, steps, metadata, completed_at, started_at, read } = notification;

  const status = notificationManager.getStatus(notification);
  const isInProgress = status === "in_progress";
  const isCompleted = status === "completed";
  const isFailed = status === "failed";
  const isCancelled = status === "cancelled";

  const statusClass = isInProgress
    ? "in-progress"
    : isCompleted
      ? "completed"
      : isFailed
        ? "failed"
        : isCancelled
          ? "cancelled"
          : "";

  const statusIcon = isInProgress
    ? '<i class="icon-loader"></i>'
    : isCompleted
      ? '<i class="icon-circle-check"></i>'
      : isFailed
        ? '<i class="icon-circle-x"></i>'
        : isCancelled
          ? '<i class="icon-ban"></i>'
          : "";

  const actionTypeLabel = escapeText(formatActionType(action_type));
  const rawSymbol =
    metadata && typeof metadata === "object" && metadata !== null ? metadata.symbol : "";
  const symbol = rawSymbol ? escapeText(rawSymbol) : "";

  // Build a compact set of detail chips from whatever metadata the action
  // carries (amount, sell %, strategy reason, strategy id, auto/manual source,
  // router). Each chip is "<label> value"; they render on one wrapped row.
  const md = metadata && typeof metadata === "object" ? metadata : {};
  const details = [];

  const sizeSol = Number(md.size_sol);
  const inputLamports = Number(md.input_amount);
  if (Number.isFinite(sizeSol) && sizeSol > 0) {
    details.push(`${formatSol(sizeSol)} SOL`);
  } else if (Number.isFinite(inputLamports) && inputLamports > 0) {
    details.push(`${formatSol(inputLamports / 1_000_000_000)} SOL`);
  }

  const pct = Number(md.percentage);
  if (Number.isFinite(pct) && pct > 0) {
    details.push(`${pct % 1 === 0 ? pct : pct.toFixed(1)}%`);
  }

  if (md.reason) details.push(humanizeToken(md.reason));
  if (md.strategy_id) details.push(escapeText(md.strategy_id));

  const source = sourceFromOperation(md.operation);
  const detailChips = details.length
    ? `<div class="notification-details">${details
        .map((d) => `<span class="notification-chip">${escapeText(d)}</span>`)
        .join("")}</div>`
    : "";

  const ts = completed_at || notification.timestamp || started_at;
  const timeLabel = escapeText(formatTime(ts));
  const timeTitle = escapeText(formatAbsoluteTime(ts));
  const durationMs = Number(notification.duration_ms);
  const durationLabel =
    (isCompleted || isFailed) && Number.isFinite(durationMs) && durationMs > 0
      ? escapeText(formatDuration(durationMs))
      : "";

  const progressInfo = isInProgress ? state : null;
  const totalSteps = progressInfo?.total_steps ?? steps?.length ?? 0;
  const currentIndex = progressInfo?.current_step_index ?? notification.current_step_index ?? 0;
  const progressPctRaw = progressInfo?.progress_pct ?? 0;
  const boundedProgressPct = Math.max(0, Math.min(100, Number(progressPctRaw) || 0));
  const currentStepName = progressInfo?.current_step || steps?.[currentIndex]?.name || "Processing";
  const safeStepName = escapeText(currentStepName);
  const stepPosition =
    totalSteps > 0 ? `${Math.min(currentIndex + 1, totalSteps)}/${totalSteps}` : "";
  const safeStepPosition = stepPosition ? escapeText(stepPosition) : "";

  let progressHtml = "";
  if (isInProgress) {
    progressHtml = `
      <div class="notification-progress">
        <div class="progress-bar-container">
          <div class="progress-bar-fill" style="width: ${boundedProgressPct}%"></div>
        </div>
        <div class="progress-text">
          ${safeStepName}${safeStepPosition ? ` (${safeStepPosition})` : ""}
        </div>
      </div>
    `;
  }

  let errorHtml = "";
  if (isFailed) {
    const failedStep = steps?.find((step) => step.status === "failed");
    const errorMsg = state?.error || failedStep?.error || notification.error || "Unknown error";
    errorHtml = `<div class="notification-error">${escapeText(errorMsg)}</div>`;
  } else if (isCancelled) {
    errorHtml = '<div class="notification-error">Cancelled</div>';
  }

  const safeId = escapeText(id);
  const sourceBadge = source
    ? `<span class="notification-source notification-source--${source}">${source}</span>`
    : "";
  const footerHtml = `
    <div class="notification-footer">
      <span class="notification-time" title="${timeTitle}">${timeLabel}</span>
      ${durationLabel ? `<span class="notification-duration">${durationLabel}</span>` : ""}
    </div>`;

  return `
    <div class="notification-item ${statusClass} ${read ? "read" : "unread"}" data-id="${safeId}">
      <div class="notification-header">
        <span class="notification-icon">${statusIcon}</span>
        <div class="notification-title">
          <strong>${actionTypeLabel}</strong>
          ${symbol ? `<span class="notification-symbol">${symbol}</span>` : ""}
          ${sourceBadge}
        </div>
        <button class="notification-dismiss" data-id="${safeId}" title="Dismiss">×</button>
      </div>
      ${detailChips}
      ${progressHtml}
      ${errorHtml}
      ${footerHtml}
    </div>
  `;
}

/** Format a SOL amount compactly (trim trailing zeros, max 4 dp). */
function formatSol(value) {
  const n = Number(value);
  if (!Number.isFinite(n)) return "0";
  return parseFloat(n.toFixed(4)).toString();
}

/** Humanize a CamelCase / snake_case token, e.g. "TakeProfit" -> "Take Profit". */
function humanizeToken(value) {
  return String(value)
    .replace(/_/g, " ")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/\b\w/g, (c) => c.toUpperCase())
    .trim();
}

/** Derive an "auto" | "manual" source tag from the operation field. */
function sourceFromOperation(operation) {
  if (!operation) return "";
  const op = String(operation).toLowerCase();
  if (op.startsWith("auto")) return "auto";
  if (op.startsWith("manual")) return "manual";
  return "";
}

/** Compact duration: "820ms", "3.4s", "1m 12s". */
function formatDuration(ms) {
  if (ms < 1000) return `${Math.round(ms)}ms`;
  const sec = ms / 1000;
  if (sec < 60) return `${parseFloat(sec.toFixed(1))}s`;
  const m = Math.floor(sec / 60);
  const s = Math.round(sec % 60);
  return `${m}m ${s}s`;
}

/**
 * Setup event delegation for notification list
 */
function setupNotificationListDelegation() {
  const list = document.getElementById("notificationList");
  if (!list) return;

  handlers.notificationList = (e) => {
    const dismissBtn = e.target.closest(".notification-dismiss");
    if (dismissBtn) {
      e.stopPropagation();
      const id = dismissBtn.dataset.id;
      if (id) {
        notificationManager.dismiss(id);
      }
      return;
    }

    const item = e.target.closest(".notification-item");
    if (item) {
      const id = item.dataset.id;
      if (id) {
        notificationManager.markAsRead(id);
      }
    }
  };

  list.addEventListener("click", handlers.notificationList);
}

/**
 * Format action type for display
 */
function formatActionType(actionType) {
  if (!actionType) return "Action";

  const typeMap = {
    swap_buy: "Buy",
    swap_sell: "Sell",
    position_open: "Open",
    position_close: "Close",
    position_dca: "DCA",
    position_partial_exit: "Partial Exit",
    manual_order: "Manual",
  };

  return typeMap[actionType] || actionType;
}

/**
 * Modern relative time: "just now", "5 min ago", "2 hours ago", "3 days ago",
 * then an absolute "Jun 25" / "Jun 25, 2025" for anything older than a week.
 */
function formatTime(timestamp) {
  if (!timestamp) return "";

  const date = new Date(timestamp);
  if (isNaN(date.getTime())) {
    return "";
  }

  const now = new Date();
  const diffSec = Math.max(0, Math.floor((now - date) / 1000));
  const diffMin = Math.floor(diffSec / 60);
  const diffHr = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHr / 24);

  if (diffSec < 45) return "just now";
  if (diffMin < 60) return `${diffMin} min ago`;
  if (diffHr < 24) return `${diffHr} ${diffHr === 1 ? "hour" : "hours"} ago`;
  if (diffDay < 7) return `${diffDay} ${diffDay === 1 ? "day" : "days"} ago`;

  const sameYear = date.getFullYear() === now.getFullYear();
  return date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    ...(sameYear ? {} : { year: "numeric" }),
  });
}

/**
 * Full, absolute timestamp for the item's hover title (tooltip).
 */
function formatAbsoluteTime(timestamp) {
  if (!timestamp) return "";
  const date = new Date(timestamp);
  if (isNaN(date.getTime())) return "";
  return date.toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function resolveTimestamp(notification) {
  return notification?.completed_at || notification?.timestamp || notification?.started_at || "";
}

/**
 * Cleanup
 */
export function dispose() {
  close();

  const drawer = document.getElementById("notificationDrawer");
  const backdrop = drawer?.querySelector('[data-role="dismiss"]');
  const closeBtn = document.getElementById("notificationDrawerClose");
  const filterActionType = document.getElementById("filterActionType");
  const filterState = document.getElementById("filterState");
  const clearFiltersBtn = document.getElementById("clearFiltersBtn");
  const markAllReadBtn = document.getElementById("markAllReadBtn");
  const clearAllBtn = document.getElementById("clearAllBtn");
  const list = document.getElementById("notificationList");
  const backToTopBtn = document.getElementById("backToTopBtn");

  if (backdrop && handlers.backdrop) {
    backdrop.removeEventListener("click", handlers.backdrop);
  }
  if (closeBtn && handlers.closeBtn) {
    closeBtn.removeEventListener("click", handlers.closeBtn);
  }
  if (handlers.keydown) {
    document.removeEventListener("keydown", handlers.keydown);
  }

  handlers.tabs.forEach(({ element, handler }) => {
    element.removeEventListener("click", handler);
  });

  if (filterActionType && handlers.filterActionType) {
    filterActionType.removeEventListener("change", handlers.filterActionType);
  }
  if (filterState && handlers.filterState) {
    filterState.removeEventListener("change", handlers.filterState);
  }
  if (clearFiltersBtn && handlers.clearFilters) {
    clearFiltersBtn.removeEventListener("click", handlers.clearFilters);
  }
  if (markAllReadBtn && handlers.markAllRead) {
    markAllReadBtn.removeEventListener("click", handlers.markAllRead);
  }
  if (clearAllBtn && handlers.clearAll) {
    clearAllBtn.removeEventListener("click", handlers.clearAll);
  }
  if (list && handlers.notificationList) {
    list.removeEventListener("click", handlers.notificationList);
  }
  if (list && handlers.scroll) {
    list.removeEventListener("scroll", handlers.scroll);
  }
  if (backToTopBtn && handlers.backToTop) {
    backToTopBtn.removeEventListener("click", handlers.backToTop);
  }

  if (unsubscribe) {
    unsubscribe();
    unsubscribe = null;
  }

  handlers = {
    backdrop: null,
    closeBtn: null,
    keydown: null,
    tabs: [],
    filterActionType: null,
    filterState: null,
    clearFilters: null,
    markAllRead: null,
    clearAll: null,
    notificationList: null,
    scroll: null,
    backToTop: null,
  };

  isInitialized = false;
}
