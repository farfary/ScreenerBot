// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Actions drawer - the tabs, the type filter, the day-grouped action rows and their detail.
import { notificationManager } from "../core/notifications.js";
import * as Utils from "../core/utils.js";
import { pushEscapeHandler } from "../core/escape_stack.js";
import { ConfirmationDialog } from "./confirmation_dialog.js";
import { enhanceAllSelects } from "./custom_select.js";
import { playTabSwitch } from "../core/sounds.js";
import { renderStateView } from "./state_view.js";
import { resolveTokenCells } from "./token_identity.js";
import { elapsedLabel, renderActionList, timeLabel } from "./action_row.js";

// Tab ids of the drawer (`data-tab` in base.html).
const EMPTY_TAB_LABELS = Object.freeze({
  all: "notifications-empty-all",
  active: "notifications-empty-active",
  completed: "notifications-empty-completed",
  failed: "notifications-empty-failed",
});

/** Tabs listed from the persisted history; "active" lists the live in-memory actions. */
const HISTORY_TABS = new Set(["all", "completed", "failed"]);

/** How often running rows refresh their elapsed time and recent rows their relative time. */
const CLOCK_INTERVAL_MS = 1000;

const PAGE_SIZE = 30;

let currentTab = "all";
let isInitialized = false;
let isOpen = false;
let releaseEscape = null;
let clockTimer = null;

// Infinite scroll state
let currentOffset = 0;
let totalResults = 0;
let isLoadingMore = false;
let hasMoreData = true;
let loadedNotifications = [];

/** The type filter value: a trade side the history route understands, or "" for all. */
let typeFilter = "";

/** Ids of the rows whose detail is open; kept across re-renders. */
const expandedRows = new Set();

/** The rows on screen, by id, for the detail actions. */
let renderedById = new Map();

// Event listener cleanup tracking
let handlers = {
  backdrop: null,
  closeBtn: null,
  tabs: [],
  filterActionType: null,
  clearFilters: null,
  clearAll: null,
  notificationList: null,
  scroll: null,
  backToTop: null,
};

// Subscription cleanup
let unsubscribe = null;

/**
 * Initialize notification drawer
 */
export function init() {
  if (isInitialized) {
    console.warn("[NotificationPanel] Already initialized, skipping");
    return;
  }

  setupHeaderTooltips();
  setupTabs();
  setupActions();
  setupDrawerControls();
  setupFilters();
  setupInfiniteScroll();
  setupBackToTop();
  setupNotificationListDelegation();
  subscribeToUpdates();
  renderNotifications();
  toggleTabControls(currentTab);

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
  releaseEscape = pushEscapeHandler(close);
  startClock();

  // Refresh with fresh data each time the drawer opens (DB-backed tabs refetch).
  resetScrollState();
  renderNotifications();

  // Opening the drawer reads every action in it.
  setTimeout(() => {
    notificationManager.markAllAsRead().catch((error) => {
      console.error("[NotificationPanel] Failed to mark notifications read on open", error);
    });
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
  if (releaseEscape) {
    releaseEscape();
    releaseEscape = null;
  }
  stopClock();
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
 * The header's icon buttons name themselves in the shared hover tooltip, which shows at
 * once, instead of the native title, which shows late and not at all on touch.
 */
function setupHeaderTooltips() {
  const buttons = document.querySelectorAll(
    "#notificationDrawer .notification-panel-actions button[title]"
  );
  buttons.forEach((button) => {
    button.dataset.tooltip = button.getAttribute("title");
    button.removeAttribute("title");
  });
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
      toggleTabControls(currentTab);
    };
    handlers.tabs.push({ element: tab, handler });
    tab.addEventListener("click", handler);
  });
}

/** Clearing is offered only while the filter narrows the list. */
function syncClearFilters() {
  const clearFiltersBtn = document.getElementById("clearFiltersBtn");
  if (clearFiltersBtn) clearFiltersBtn.hidden = !typeFilter;
}

/**
 * Setup filter controls
 */
function setupFilters() {
  const filterActionType = document.getElementById("filterActionType");
  const clearFiltersBtn = document.getElementById("clearFiltersBtn");
  syncClearFilters();

  if (filterActionType) {
    handlers.filterActionType = () => {
      typeFilter = filterActionType.value;
      syncClearFilters();
      resetScrollState();
      renderNotifications();
    };
    filterActionType.addEventListener("change", handlers.filterActionType);
  }

  if (clearFiltersBtn) {
    handlers.clearFilters = () => {
      typeFilter = "";
      if (filterActionType) filterActionType.value = "";
      syncClearFilters();
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
    if (!HISTORY_TABS.has(currentTab)) return;

    const scrollTop = list.scrollTop;
    const scrollHeight = list.scrollHeight;
    const clientHeight = list.clientHeight;

    // Load more when within 100px of bottom
    if (!isLoadingMore && hasMoreData && scrollTop + clientHeight >= scrollHeight - 100) {
      loadMoreNotifications();
    }

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
  backToTopBtn.style.display = scrollTop > 200 ? "flex" : "none";
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
 * Show the controls that act on the given tab. The type filter narrows the history tabs;
 * Clear all dismisses actions from the live Active list, the only list it changes.
 */
function toggleTabControls(tab) {
  const filtersEl = document.getElementById("notificationFilters");
  if (filtersEl) filtersEl.hidden = !HISTORY_TABS.has(tab);

  const clearAllBtn = document.getElementById("clearAllBtn");
  if (clearAllBtn) clearAllBtn.hidden = tab !== "active";
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
  const clearAllBtn = document.getElementById("clearAllBtn");

  if (clearAllBtn) {
    handlers.clearAll = async () => {
      const { confirmed } = await ConfirmationDialog.show({
        title: I18n.t("notifications-clear-title"),
        message: I18n.t("notifications-clear-message"),
        confirmLabel: I18n.t("common-action-clear"),
        cancelLabel: I18n.t("common-action-cancel"),
        variant: "warning",
      });

      if (confirmed) {
        try {
          // No success toast: the list the user is looking at empties.
          await notificationManager.clearAll();
        } catch (error) {
          console.error("[NotificationPanel] Failed to clear notifications:", error);
          Utils.showToast(I18n.t("notifications-clear-failed"), "error");
        }
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
      Utils.showToast({
        key: "actions-stream",
        type: "warning",
        title: I18n.t("notifications-stream-lag-title"),
        message:
          skipped > 0
            ? I18n.t("notifications-stream-lag-missed", { count: skipped })
            : I18n.t("notifications-stream-lag-refreshing"),
      });
    }

    if (event.type === "sync_error") {
      // Keyed: a backend that is down fails every sync, and this must stay one
      // notice rather than one per retry.
      Utils.showToast({
        key: "actions-stream",
        type: "warning",
        title: I18n.t("notifications-sync-failed"),
        message: event.error || null,
      });
    }
  });
}

/**
 * Set active tab
 */
function setActiveTab(activeTab) {
  const tabs = document.querySelectorAll(".notification-tab");
  tabs.forEach((tab) => {
    const active = tab === activeTab;
    tab.classList.toggle("active", active);
    tab.setAttribute("aria-selected", active ? "true" : "false");
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
    el.hidden = !(count > 0);
  }
}

/** The history request for the current tab, filter and page. */
function historyOptions(offset) {
  const options = { limit: PAGE_SIZE, offset };
  if (currentTab === "completed" || currentTab === "failed") options.state = currentTab;
  if (typeFilter) options.action_type = typeFilter;
  return options;
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
  if (HISTORY_TABS.has(currentTab)) {
    if (currentOffset === 0) {
      try {
        isLoadingMore = true;
        showLoading(true);

        const response = await notificationManager.fetchHistory(historyOptions(0));
        loadedNotifications = response.actions || [];
        totalResults = response.total || 0;
        currentOffset = loadedNotifications.length;
        hasMoreData = currentOffset < totalResults;

        notificationManager.syncFromHistory(loadedNotifications, { silent: true });
      } catch (error) {
        console.error("[NotificationPanel] Failed to fetch history:", error);
        list.innerHTML = renderStateView({
          kind: "error",
          icon: "icon-triangle-alert",
          message: I18n.t("notifications-load-failed"),
          compact: true,
        });
        return;
      } finally {
        isLoadingMore = false;
        showLoading(false);
      }
    }

    notifications = loadedNotifications.map(mergeWithStoredState);
  } else {
    // Active is the inbox, so a dismissed action leaves it. All, Completed and Failed are
    // history views: they list every action their tab count includes, dismissed or not
    // (completed and failed actions are auto-dismissed shortly after they finish).
    notifications = notificationManager
      .getActive()
      .map(mergeWithStoredState)
      .filter((n) => !n.dismissed);
  }

  // Newest first
  notifications.sort(
    (a, b) => new Date(resolveTimestamp(b)).getTime() - new Date(resolveTimestamp(a)).getTime()
  );

  paintList(list, notifications);
}

/**
 * Write the rows into the list, keeping its scroll position. A first paint of the open
 * drawer resolves the tokens' identities and paints once more with them; that repaint
 * resolves nothing, so a failed lookup cannot loop.
 */
function paintList(list, notifications, { resolveTokens = true } = {}) {
  renderedById = new Map(notifications.map((n) => [n.id, n]));

  if (notifications.length === 0) {
    list.innerHTML = renderStateView({
      icon: "icon-inbox",
      message: I18n.label(EMPTY_TAB_LABELS, currentTab),
      compact: true,
    });
    return;
  }

  const scrollTop = list.scrollTop;
  list.innerHTML = renderActionList(notifications, { expanded: expandedRows });
  list.scrollTop = scrollTop;

  // Identities are looked up only for a list the user can see: the drawer renders while
  // closed on every page load, and that must not cost a request.
  if (!resolveTokens || !isOpen) return;
  resolveTokenCells(
    notifications.map((n) => n.entity_id),
    () => paintList(list, [...renderedById.values()], { resolveTokens: false })
  );
}

/**
 * Load more notifications for infinite scroll
 */
async function loadMoreNotifications() {
  if (isLoadingMore || !hasMoreData) return;
  if (!HISTORY_TABS.has(currentTab)) return;

  try {
    isLoadingMore = true;
    showLoading(true);

    const response = await notificationManager.fetchHistory(historyOptions(currentOffset));
    const newNotifications = response.actions || [];
    totalResults = response.total || 0;

    if (newNotifications.length > 0) {
      loadedNotifications = [...loadedNotifications, ...newNotifications];
      currentOffset = loadedNotifications.length;
      hasMoreData = currentOffset < totalResults;

      notificationManager.syncFromHistory(newNotifications, { silent: true });

      // Re-render the whole list: a new page can continue the last day group.
      const list = document.getElementById("notificationList");
      if (list) paintList(list, loadedNotifications.map(mergeWithStoredState));
    } else {
      hasMoreData = false;
    }
  } catch (error) {
    console.error("[NotificationPanel] Failed to load more:", error);
  } finally {
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

/** Open or close a row's detail in place. */
function toggleRow(summary) {
  const row = summary.closest(".action-row");
  const id = row?.dataset.id;
  if (!id) return;
  const detail = row.querySelector(".action-detail");
  const expanded = !expandedRows.has(id);
  if (expanded) {
    expandedRows.add(id);
  } else {
    expandedRows.delete(id);
  }
  summary.setAttribute("aria-expanded", expanded ? "true" : "false");
  row.classList.toggle("is-open", expanded);
  if (detail) detail.hidden = !expanded;

  notificationManager.markAsRead(id).catch((error) => {
    console.error("[NotificationPanel] Failed to mark notification read:", error);
  });
}

function openToken(mint) {
  const action = [...renderedById.values()].find((n) => n.entity_id === mint);
  window.dispatchEvent(
    new CustomEvent("screenerbot:open-token-details", {
      detail: { mint, symbol: action?.metadata?.symbol },
    })
  );
}

async function openPosition(id, row) {
  const action = renderedById.get(row?.dataset.id);
  // The dialog module installs the listener this event reaches; it loads on first use.
  await import("./position_details_dialog.js");
  window.dispatchEvent(
    new CustomEvent("screenerbot:open-position-details", {
      detail: { id: Number(id), mint: action?.entity_id, symbol: action?.metadata?.symbol },
    })
  );
}

let transactionDialog = null;

async function openTransaction(signature) {
  const { TransactionDetailsDialog } = await import("./transaction_details_dialog.js");
  if (!transactionDialog) transactionDialog = new TransactionDetailsDialog();
  transactionDialog.show({ signature });
}

/**
 * Setup event delegation for notification list
 */
function setupNotificationListDelegation() {
  const list = document.getElementById("notificationList");
  if (!list) return;

  handlers.notificationList = (e) => {
    const token = e.target.closest("[data-open-token]");
    if (token) {
      openToken(token.dataset.openToken);
      return;
    }
    const position = e.target.closest("[data-open-position]");
    if (position) {
      openPosition(position.dataset.openPosition, position.closest(".action-row")).catch((error) =>
        console.error("[NotificationPanel] Failed to open position:", error)
      );
      return;
    }
    const transaction = e.target.closest("[data-open-transaction]");
    if (transaction) {
      openTransaction(transaction.dataset.openTransaction).catch((error) =>
        console.error("[NotificationPanel] Failed to open transaction:", error)
      );
      return;
    }
    const summary = e.target.closest(".action-row__summary");
    if (summary) toggleRow(summary);
  };

  list.addEventListener("click", handlers.notificationList);
}

/** Keep running rows' elapsed time and recent rows' relative time current while open. */
function startClock() {
  stopClock();
  clockTimer = window.setInterval(() => {
    const list = document.getElementById("notificationList");
    if (!list) return;
    const now = new Date();
    list.querySelectorAll("[data-elapsed-from]").forEach((el) => {
      el.textContent = elapsedLabel(el.dataset.elapsedFrom, now);
    });
    list.querySelectorAll("[data-relative-time]").forEach((el) => {
      el.textContent = timeLabel(el.dataset.relativeTime, now);
    });
  }, CLOCK_INTERVAL_MS);
}

function stopClock() {
  if (clockTimer !== null) {
    window.clearInterval(clockTimer);
    clockTimer = null;
  }
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
  const clearFiltersBtn = document.getElementById("clearFiltersBtn");
  const clearAllBtn = document.getElementById("clearAllBtn");
  const list = document.getElementById("notificationList");
  const backToTopBtn = document.getElementById("backToTopBtn");

  if (backdrop && handlers.backdrop) {
    backdrop.removeEventListener("click", handlers.backdrop);
  }
  if (closeBtn && handlers.closeBtn) {
    closeBtn.removeEventListener("click", handlers.closeBtn);
  }

  handlers.tabs.forEach(({ element, handler }) => {
    element.removeEventListener("click", handler);
  });

  if (filterActionType && handlers.filterActionType) {
    filterActionType.removeEventListener("change", handlers.filterActionType);
  }
  if (clearFiltersBtn && handlers.clearFilters) {
    clearFiltersBtn.removeEventListener("click", handlers.clearFilters);
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
    tabs: [],
    filterActionType: null,
    clearFilters: null,
    clearAll: null,
    notificationList: null,
    scroll: null,
    backToTop: null,
  };

  expandedRows.clear();
  isInitialized = false;
}
