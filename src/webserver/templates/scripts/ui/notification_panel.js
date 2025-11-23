// Notification drawer UI manager
import { notificationManager } from "../core/notifications.js";
import { toastManager } from "../core/toast.js";
import * as Utils from "../core/utils.js";
import { ConfirmationDialog } from "./confirmation_dialog.js";

let currentTab = "all";
let isInitialized = false;
let isOpen = false;

// Pagination state
let currentPage = 1;
let pageSize = 50;
let totalResults = 0;
let isLoadingHistory = false;

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
  prevPage: null,
  nextPage: null,
  markAllRead: null,
  clearAll: null,
  openHistory: null,
  notificationList: null, // Event delegation handler
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
  setupPagination();
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

  // Backdrop click closes drawer
  if (backdrop) {
    handlers.backdrop = close;
    backdrop.addEventListener("click", handlers.backdrop);
  }

  // Close button
  if (closeBtn) {
    handlers.closeBtn = close;
    closeBtn.addEventListener("click", handlers.closeBtn);
  }

  // ESC key closes drawer
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
      currentPage = 1; // Reset pagination when switching tabs
      setActiveTab(tab);
      renderNotifications();

      // Show filters and pagination only for completed/failed tabs
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
      currentPage = 1; // Reset to page 1
      renderNotifications();
    };
    filterActionType.addEventListener("change", handlers.filterActionType);
  }

  if (filterState) {
    handlers.filterState = () => {
      currentFilters.state = filterState.value;
      currentPage = 1; // Reset to page 1
      renderNotifications();
    };
    filterState.addEventListener("change", handlers.filterState);
  }

  if (clearFiltersBtn) {
    handlers.clearFilters = () => {
      currentFilters = { action_type: "", state: "" };
      if (filterActionType) filterActionType.value = "";
      if (filterState) filterState.value = "";
      currentPage = 1;
      renderNotifications();
    };
    clearFiltersBtn.addEventListener("click", handlers.clearFilters);
  }
}

/**
 * Setup pagination controls
 */
function setupPagination() {
  const prevBtn = document.getElementById("prevPageBtn");
  const nextBtn = document.getElementById("nextPageBtn");

  if (prevBtn) {
    handlers.prevPage = () => {
      if (currentPage > 1) {
        currentPage--;
        renderNotifications();
      }
    };
    prevBtn.addEventListener("click", handlers.prevPage);
  }

  if (nextBtn) {
    handlers.nextPage = () => {
      const totalPages = Math.ceil(totalResults / pageSize);
      if (currentPage < totalPages) {
        currentPage++;
        renderNotifications();
      }
    };
    nextBtn.addEventListener("click", handlers.nextPage);
  }
}

/**
 * Toggle visibility of history controls (filters, pagination)
 */
function toggleHistoryControls(show) {
  const filtersEl = document.getElementById("notificationFilters");
  const paginationEl = document.getElementById("notificationPagination");
  const stateFilterEl = document.getElementById("filterState");

  if (filtersEl) {
    filtersEl.style.display = show ? "block" : "none";
  }
  if (paginationEl) {
    paginationEl.style.display = show ? "flex" : "none";
  }

  // Disable state filter on completed/failed tabs (they have implicit state filter)
  if (stateFilterEl) {
    if (show) {
      stateFilterEl.disabled = true;
      stateFilterEl.value = "";
      stateFilterEl.style.opacity = "0.5";
      stateFilterEl.style.cursor = "not-allowed";
      stateFilterEl.title = "State filter is controlled by tab selection";
    } else {
      stateFilterEl.disabled = false;
      stateFilterEl.style.opacity = "1";
      stateFilterEl.style.cursor = "pointer";
      stateFilterEl.title = "";
    }
  }
}

/**
 * Update pagination UI state
 */
function updatePaginationUI() {
  const prevBtn = document.getElementById("prevPageBtn");
  const nextBtn = document.getElementById("nextPageBtn");
  const paginationInfo = document.getElementById("paginationInfo");

  const totalPages = Math.ceil(totalResults / pageSize);

  if (prevBtn) {
    prevBtn.disabled = currentPage <= 1 || isLoadingHistory;
  }
  if (nextBtn) {
    nextBtn.disabled = currentPage >= totalPages || isLoadingHistory;
  }
  if (paginationInfo) {
    if (isLoadingHistory) {
      paginationInfo.textContent = "Loading...";
    } else {
      paginationInfo.textContent = `Page ${currentPage} of ${totalPages || 1} (${totalResults} total)`;
    }
  }
}

/**
 * Setup panel actions
 */
function setupActions() {
  const markAllReadBtn = document.getElementById("markAllReadBtn");
  const clearAllBtn = document.getElementById("clearAllBtn");
  const openHistoryBtn = document.getElementById("notificationOpenHistory");

  if (markAllReadBtn) {
    handlers.markAllRead = () => {
      notificationManager.markAllAsRead();
      Utils.showToast("✓ All notifications marked as read", "success");
    };
    markAllReadBtn.addEventListener("click", handlers.markAllRead);
  }

  if (clearAllBtn) {
    handlers.clearAll = async () => {
      const { confirmed } = await ConfirmationDialog.show({
        title: "Clear All Notifications",
        message: "This will remove all notifications from the panel. This action cannot be undone.",
        confirmLabel: "Clear All",
        cancelLabel: "Cancel",
        variant: "warning",
      });

      if (confirmed) {
        notificationManager.clearAll();
        Utils.showToast("All notifications cleared", "info");
      }
    };
    clearAllBtn.addEventListener("click", handlers.clearAll);
  }

  if (openHistoryBtn) {
    handlers.openHistory = () => {
      close();
      // TODO: Navigate to dedicated actions/history page when implemented
      Utils.showToast("Full activity history coming soon", "info");
    };
    openHistoryBtn.addEventListener("click", handlers.openHistory);
  }
}

/**
 * Subscribe to notification updates
 */
function subscribeToUpdates() {
  unsubscribe = notificationManager.subscribe((event) => {
    if (event.type === "summary") {
      updateSummaryCards(event.summary);
      updateConnectionStatus(event.summary.connection);
      updateFooterLiveCount(event.summary.active);
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
      renderNotifications();
    }

    if (event.type === "lag") {
      const skipped = event.payload?.skipped || 0;
      Utils.showToast(
        skipped > 0
          ? `Missed ${skipped} action updates — refreshing…`
          : "Action stream fell behind — refreshing…",
        "warning"
      );
    }

    if (event.type === "sync_error") {
      Utils.showToast(
        `Failed to refresh live actions (${event.error || "unknown error"})`,
        "warning"
      );
    }
  });
}

/**
 * Update summary cards in drawer header
 */
function updateSummaryCards(summary) {
  if (!summary) return;

  const activeCountEl = document.getElementById("drawerActiveCount");
  const activeSubEl = document.getElementById("drawerActiveSub");
  const completedCountEl = document.getElementById("drawerCompletedCount");
  const failedCountEl = document.getElementById("drawerFailedCount");

  if (activeCountEl) activeCountEl.textContent = summary.active || 0;
  if (activeSubEl) {
    activeSubEl.textContent = summary.active === 1 ? "In progress" : "In progress";
  }
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

  // Hide connection indicator when disconnected (don't clutter UI)
  if (!connection || connection.status !== "connected") {
    connectionEl.style.display = "none";
    return;
  }

  // Show and update when connected
  connectionEl.style.display = "inline-flex";
  connectionEl.setAttribute("data-state", connection.status);
  textEl.textContent = "Connected";
}

/**
 * Update live count in footer
 */
function updateFooterLiveCount(activeCount) {
  const footerActiveEl = document.getElementById("notificationFooterActive");
  if (footerActiveEl) {
    footerActiveEl.textContent = `${activeCount || 0} live`;
  }
}

/**
 * Handle clicks outside panel
 */
// REMOVED: Drawer uses backdrop dismiss instead

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

  // For completed/failed tabs, fetch from database with pagination
  if (currentTab === "completed" || currentTab === "failed") {
    try {
      isLoadingHistory = true;
      updatePaginationUI();

      // Build query options
      const options = {
        limit: pageSize,
        offset: (currentPage - 1) * pageSize,
      };

      // Add state filter based on tab (completed/failed have implicit state filter)
      if (currentTab === "completed") {
        options.state = "completed";
      } else if (currentTab === "failed") {
        options.state = "failed";
      }

      // Add user filters (only action_type filter is allowed on completed/failed tabs)
      // State filter dropdown should be disabled on these tabs to avoid confusion
      if (currentFilters.action_type) {
        options.action_type = currentFilters.action_type;
      }

      const response = await notificationManager.fetchHistory(options);
      notifications = response.actions || [];
      totalResults = response.total || 0;

      // Merge results into local cache so interactions (dismiss/read) work reliably
      notificationManager.syncFromHistory(notifications, { silent: true });

      isLoadingHistory = false;
      updatePaginationUI();
    } catch (error) {
      console.error("[NotificationPanel] Failed to fetch history:", error);
      isLoadingHistory = false;
      list.innerHTML = `
        <div class="notification-empty">
          <span><i class="icon-alert-triangle"></i></span>
          <p>Failed to load history</p>
          <small>${error.message}</small>
        </div>
      `;
      updatePaginationUI();
      return;
    }
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

  // Apply UI state (read/dismissed/timestamp) from cache when rendering history
  notifications = notifications.map(mergeWithStoredState);

  // Hide dismissed notifications from all tabs
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
        <span><i class="icon-inbox"></i></span>
        <p>No ${currentTab === "all" ? "" : currentTab + " "}notifications</p>
      </div>
    `;
    return;
  }

  list.innerHTML = notifications.map((n) => renderNotification(n)).join("");
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
      ? "✅"
      : isFailed
        ? "❌"
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
    errorHtml = `
      <div class="notification-error">
        ${escapeText(errorMsg)}
      </div>
    `;
  } else if (isCancelled) {
    errorHtml = `
      <div class="notification-error">
        Action cancelled
      </div>
    `;
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
 * Setup event delegation for notification list (fixes memory leak)
 */
function setupNotificationListDelegation() {
  const list = document.getElementById("notificationList");
  if (!list) return;

  handlers.notificationList = (e) => {
    // Handle dismiss button clicks
    const dismissBtn = e.target.closest(".notification-dismiss");
    if (dismissBtn) {
      e.stopPropagation();
      const id = dismissBtn.dataset.id;
      if (id) {
        notificationManager.dismiss(id);
      }
      return;
    }

    // Handle notification item clicks
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

  // Backend sends snake_case format via Serde JSON serialization
  // #[serde(rename_all = "snake_case")] in src/actions/types.rs line 177
  const typeMap = {
    swap_buy: "Buying",
    swap_sell: "Selling",
    position_open: "Opening Position",
    position_close: "Closing Position",
    position_dca: "DCA",
    position_partial_exit: "Partial Exit",
    manual_order: "Manual Order",
  };

  return typeMap[actionType] || actionType;
}

/**
 * Format time for display
 */
function formatTime(timestamp) {
  if (!timestamp) return "";

  const date = new Date(timestamp);

  // Validate date
  if (isNaN(date.getTime())) {
    console.warn("[NotificationPanel] Invalid timestamp:", timestamp);
    return "Invalid date";
  }

  const now = new Date();
  const diffMs = now - date;
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHr = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHr / 24);

  if (diffSec < 60) return "Just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHr < 24) return `${diffHr}h ago`;
  if (diffDay < 7) return `${diffDay}d ago`;

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

  // Remove all event listeners
  const drawer = document.getElementById("notificationDrawer");
  const backdrop = drawer?.querySelector('[data-role="dismiss"]');
  const closeBtn = document.getElementById("notificationDrawerClose");
  const filterActionType = document.getElementById("filterActionType");
  const filterState = document.getElementById("filterState");
  const clearFiltersBtn = document.getElementById("clearFiltersBtn");
  const prevBtn = document.getElementById("prevPageBtn");
  const nextBtn = document.getElementById("nextPageBtn");
  const markAllReadBtn = document.getElementById("markAllReadBtn");
  const clearAllBtn = document.getElementById("clearAllBtn");
  const openHistoryBtn = document.getElementById("notificationOpenHistory");

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
  if (prevBtn && handlers.prevPage) {
    prevBtn.removeEventListener("click", handlers.prevPage);
  }
  if (nextBtn && handlers.nextPage) {
    nextBtn.removeEventListener("click", handlers.nextPage);
  }
  if (markAllReadBtn && handlers.markAllRead) {
    markAllReadBtn.removeEventListener("click", handlers.markAllRead);
  }
  if (clearAllBtn && handlers.clearAll) {
    clearAllBtn.removeEventListener("click", handlers.clearAll);
  }
  if (openHistoryBtn && handlers.openHistory) {
    openHistoryBtn.removeEventListener("click", handlers.openHistory);
  }

  // Remove notification list delegation
  const list = document.getElementById("notificationList");
  if (list && handlers.notificationList) {
    list.removeEventListener("click", handlers.notificationList);
  }

  // Unsubscribe from notifications
  if (unsubscribe) {
    unsubscribe();
    unsubscribe = null;
  }

  // Reset handlers
  handlers = {
    backdrop: null,
    closeBtn: null,
    keydown: null,
    tabs: [],
    filterActionType: null,
    filterState: null,
    clearFilters: null,
    prevPage: null,
    nextPage: null,
    markAllRead: null,
    clearAll: null,
    openHistory: null,
    notificationList: null,
  };

  isInitialized = false;
}
