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
      toggleHistoryControls(currentTab === "completed" || currentTab === "failed");
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
    if (currentTab !== "completed" && currentTab !== "failed") return;
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
 * Toggle visibility of history controls (filters)
 */
function toggleHistoryControls(show) {
  const filtersEl = document.getElementById("notificationFilters");
  const stateFilterEl = document.getElementById("filterState");

  if (filtersEl) {
    filtersEl.style.display = show ? "block" : "none";
  }

  if (stateFilterEl) {
    if (show) {
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
        title: "Clear All",
        message: "Remove all notifications? This cannot be undone.",
        confirmLabel: "Clear",
        cancelLabel: "Cancel",
        variant: "warning",
      });

      if (confirmed) {
        notificationManager.clearAll();
        Utils.showToast("All cleared", "info");
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
      updateSummaryStats(event.summary);
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
      // Only re-render for active/all tabs or on clear
      if (currentTab === "active" || currentTab === "all" || event.type === "cleared") {
        renderNotifications();
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
 * Update summary stats
 */
function updateSummaryStats(summary) {
  if (!summary) return;

  const activeCountEl = document.getElementById("drawerActiveCount");
  const completedCountEl = document.getElementById("drawerCompletedCount");
  const failedCountEl = document.getElementById("drawerFailedCount");

  if (activeCountEl) activeCountEl.textContent = summary.active || 0;
  if (completedCountEl) completedCountEl.textContent = summary.completed24h || 0;
  if (failedCountEl) failedCountEl.textContent = summary.failed24h || 0;
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
  const allCount = notificationManager.getAll().length;
  const activeCount = notificationManager.getActive().length;
  const completedCount = notificationManager.getCompleted().length;
  const failedCount = notificationManager.getFailed().length;

  updateCount("allCount", allCount);
  updateCount("activeCount", activeCount);
  updateCount("completedCount", completedCount);
  updateCount("failedCount", failedCount);
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

  // For completed/failed tabs, fetch from database with infinite scroll
  if (currentTab === "completed" || currentTab === "failed") {
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
    // For active/all tabs, use in-memory data
    switch (currentTab) {
      case "active":
        notifications = notificationManager.getActive().map((n) => ({ ...n }));
        break;
      default:
        notifications = notificationManager.getAll().map((n) => ({ ...n }));
    }
  }

  // Apply UI state from cache
  notifications = notifications.map(mergeWithStoredState);

  // Hide dismissed notifications from all/active tabs
  const hideDismissed = currentTab === "all" || currentTab === "active";
  if (hideDismissed) {
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
  if (currentTab !== "completed" && currentTab !== "failed") return;

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

  let descriptionHtml = "";
  if (metadata && typeof metadata === "object" && metadata !== null) {
    const { input_amount, router } = metadata;
    const inputLamports = Number(input_amount);

    if (Number.isFinite(inputLamports)) {
      const amountSol = (inputLamports / 1_000_000_000).toFixed(4);
      descriptionHtml = `<div class="notification-description">${escapeText(amountSol)} SOL</div>`;
    }

    if (isFailed && router) {
      descriptionHtml += `<div class="notification-meta">via ${escapeText(router)}</div>`;
    }
  }

  const timeLabel = escapeText(formatTime(completed_at || notification.timestamp || started_at));

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
    errorHtml = "<div class=\"notification-error\">Cancelled</div>";
  }

  const safeId = escapeText(id);
  const timeHtml = `<div class="notification-time">${timeLabel}</div>`;

  return `
    <div class="notification-item ${statusClass} ${read ? "read" : "unread"}" data-id="${safeId}">
      <div class="notification-header">
        <span class="notification-icon">${statusIcon}</span>
        <div class="notification-title">
          <strong>${actionTypeLabel}</strong>
          ${symbol ? `<span class="notification-symbol">${symbol}</span>` : ""}
        </div>
        <button class="notification-dismiss" data-id="${safeId}" title="Dismiss">×</button>
      </div>
      ${descriptionHtml}
      ${progressHtml}
      ${errorHtml}
      ${timeHtml}
    </div>
  `;
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
 * Format time for display
 */
function formatTime(timestamp) {
  if (!timestamp) return "";

  const date = new Date(timestamp);
  if (isNaN(date.getTime())) {
    return "";
  }

  const now = new Date();
  const diffMs = now - date;
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHr = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHr / 24);

  if (diffSec < 60) return "now";
  if (diffMin < 60) return `${diffMin}m`;
  if (diffHr < 24) return `${diffHr}h`;
  if (diffDay < 7) return `${diffDay}d`;

  return date.toLocaleDateString();
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
