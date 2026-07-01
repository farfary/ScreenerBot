// Real-time notification manager for action progress tracking
/* global EventSource */

import { toastManager } from "./toast.js";
import { waitForReady } from "./bootstrap.js";

const AUTO_DISMISS_COMPLETED_MS = 10000; // 10 seconds
const AUTO_DISMISS_FAILED_MS = 30000; // 30 seconds
const RECONNECT_DELAY_MS = 3000;

// Action types that represent an in-flight trade for a token. Used by the
// positions list and the position-details dialog to disable Add/Sell/Close
// while a buy or sell is processing in the background.
const TRADE_BUY_ACTION_TYPES = new Set(["swap_buy", "position_open"]);
const TRADE_SELL_ACTION_TYPES = new Set([
  "swap_sell",
  "position_close",
  "position_partial_exit",
]);
const actionEntityMint = (n) => n?.entity_id || n?.metadata?.mint || "";

class NotificationManager {
  constructor() {
    this.eventSource = null;
    this.notifications = new Map();
    this.subscribers = new Set();
    this.isConnected = false;
    this.lastConnectionChange = null;
    this.reconnectTimer = null;
    this.autoDismissTimers = new Map();
    this.hadInitialConnect = false;
    this.activeSyncPromise = null;
  }

  /**
   * Initialize the notification manager
   */
  init() {
    this.connect();
    this.syncActiveActions({ reason: "initial" }).catch((error) => {
      console.error("[NotificationManager] Initial sync failed", error);
    });
  }

  /**
   * Connect to the SSE stream
   */
  connect() {
    if (this.eventSource) {
      this.eventSource.close();
    }

    this.eventSource = new EventSource("/api/actions/stream");

    this.eventSource.onopen = () => {
      this.isConnected = true;
      this.lastConnectionChange = Date.now();
      const isReconnect = this.hadInitialConnect;
      this.hadInitialConnect = true;
      this.notifySubscribers({
        type: "connection",
        status: "connected",
        changedAt: this.lastConnectionChange,
        isReconnect,
      });
      if (this.reconnectTimer) {
        clearTimeout(this.reconnectTimer);
        this.reconnectTimer = null;
      }
      this.syncActiveActions({ reason: isReconnect ? "reconnect" : "initial_connect" }).catch(
        (error) => {
          console.error("[NotificationManager] Active sync on connect failed", error);
        }
      );
    };

    this.eventSource.onmessage = (event) => {
      try {
        const update = JSON.parse(event.data);
        this.handleUpdate(update);
      } catch (error) {
        console.error("Failed to parse notification update:", error);
      }
    };

    this.eventSource.addEventListener("lag", (event) => {
      this.handleLagEvent(event);
    });

    this.eventSource.onerror = () => {
      this.isConnected = false;
      this.lastConnectionChange = Date.now();
      this.notifySubscribers({
        type: "connection",
        status: "disconnected",
        changedAt: this.lastConnectionChange,
      });
      this.eventSource.close();
      this.scheduleReconnect();
    };
  }

  /**
   * Schedule reconnection attempt
   */
  scheduleReconnect() {
    if (this.reconnectTimer) return;

    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, RECONNECT_DELAY_MS);
  }

  handleLagEvent(event) {
    let payload = null;
    try {
      payload = event?.data ? JSON.parse(event.data) : null;
    } catch (error) {
      console.error("[NotificationManager] Failed to parse lag event", error);
    }

    this.notifySubscribers({
      type: "lag",
      payload,
    });

    this.syncActiveActions({ reason: "lag" }).catch((error) => {
      console.error("[NotificationManager] Failed to resync after lag", error);
    });
  }

  /**
   * Handle incoming action update
   */
  handleUpdate(update) {
    const { action_id, update_type } = update;
    const action = update.action ?? null;

    if (!update_type) {
      console.warn("[NotificationManager] Ignoring update without type", update);
      return;
    }

    switch (update_type) {
      case "action_started":
        if (!action) {
          console.warn("[NotificationManager] Missing action payload for action_started", update);
          break;
        }
        this.addNotification(action);
        break;

      case "step_progress":
      case "step_completed":
      case "step_failed":
        if (!action) {
          console.warn("[NotificationManager] Missing action payload for step update", update);
          break;
        }
        this.updateNotification(action_id, action);
        break;

      case "action_completed":
        if (!action) {
          console.warn("[NotificationManager] Missing action payload for action_completed", update);
          break;
        }
        this.completeNotification(action_id, action);
        this.scheduleAutoDismiss(action_id, AUTO_DISMISS_COMPLETED_MS);
        break;

      case "action_failed":
        if (!action) {
          console.warn("[NotificationManager] Missing action payload for action_failed", update);
          break;
        }
        this.failNotification(action_id, action);
        this.scheduleAutoDismiss(action_id, AUTO_DISMISS_FAILED_MS);
        break;

      case "action_cancelled":
        if (!action) {
          console.warn("[NotificationManager] Missing action payload for action_cancelled", update);
          break;
        }
        this.updateNotification(action_id, action);
        this.scheduleAutoDismiss(action_id, AUTO_DISMISS_COMPLETED_MS);
        break;

      default:
        console.warn("[NotificationManager] Unknown update type:", update_type, update);
    }

    this.emitSummary();
  }

  /**
   * Add new notification
   */
  addNotification(action) {
    const notification = this.buildNotificationRecord(action, null, {
      resetRead: true,
      resetDismissed: true,
    });

    this.notifications.set(action.id, notification);
    this.notifySubscribers({
      type: "added",
      notification,
    });
  }

  /**
   * Update existing notification
   */
  updateNotification(actionId, action) {
    const existing = this.notifications.get(actionId);
    if (!existing) {
      this.addNotification(action);
      return;
    }

    const updated = this.buildNotificationRecord(action, existing);

    this.notifications.set(actionId, updated);
    this.notifySubscribers({
      type: "updated",
      notification: updated,
    });
  }

  /**
   * Mark notification as completed
   */
  completeNotification(actionId, action) {
    this.updateNotification(actionId, action);
  }

  /**
   * Mark notification as failed
   */
  failNotification(actionId, action) {
    this.updateNotification(actionId, action);
  }

  /**
   * Schedule auto-dismiss for completed/failed notifications
   */
  scheduleAutoDismiss(actionId, delayMs) {
    // Clear existing timer if any
    if (this.autoDismissTimers.has(actionId)) {
      clearTimeout(this.autoDismissTimers.get(actionId));
    }

    const timer = setTimeout(() => {
      this.dismiss(actionId).catch((error) => {
        console.error("[NotificationManager] Auto-dismiss failed", error);
      });
      this.autoDismissTimers.delete(actionId);
    }, delayMs);

    this.autoDismissTimers.set(actionId, timer);
  }

  /**
   * Mark notification as read
   */
  async markAsRead(actionId) {
    const notification = this.notifications.get(actionId);
    if (!notification) return;

    await this.postActionMutation(`/api/actions/${encodeURIComponent(actionId)}/read`);
    notification.read = true;
    this.notifications.set(actionId, notification);
    this.notifySubscribers({
      type: "marked_read",
      notification,
    });
    this.emitSummary();
  }

  /**
   * Mark all notifications as read
   */
  async markAllAsRead() {
    const result = await this.postActionMutation("/api/actions/read-all");
    this.serverUnread = Number(result?.unread) || 0;

    for (const [id, notification] of this.notifications) {
      if (!notification.read) {
        notification.read = true;
        this.notifications.set(id, notification);
      }
    }
    this.notifySubscribers({
      type: "all_marked_read",
    });
    this.emitSummary();
  }

  /**
   * Dismiss notification
   */
  async dismiss(actionId) {
    const notification = this.notifications.get(actionId);
    if (!notification) return;

    const result = await this.postActionMutation(`/api/actions/${encodeURIComponent(actionId)}/dismiss`);
    this.serverUnread = Number(result?.unread) || 0;
    notification.dismissed = true;
    notification.read = true;
    this.notifications.set(actionId, notification);

    // Clear auto-dismiss timer if exists
    if (this.autoDismissTimers.has(actionId)) {
      clearTimeout(this.autoDismissTimers.get(actionId));
      this.autoDismissTimers.delete(actionId);
    }

    this.notifySubscribers({
      type: "dismissed",
      notification,
    });
    this.emitSummary();

    // Note: Don't delete from map - dismissed notifications should persist
    // in memory so they remain visible in Completed/Failed tabs.
    // They'll be removed when user explicitly clears or storage limit is hit.
  }

  /**
   * Clear all notifications
   */
  async clearAll() {
    const result = await this.postActionMutation("/api/actions/dismiss-all");
    this.serverUnread = Number(result?.unread) || 0;

    // Clear all auto-dismiss timers
    for (const timer of this.autoDismissTimers.values()) {
      clearTimeout(timer);
    }
    this.autoDismissTimers.clear();

    for (const [id, notification] of this.notifications) {
      notification.dismissed = true;
      notification.read = true;
      this.notifications.set(id, notification);
    }
    this.notifySubscribers({
      type: "cleared",
    });
    this.emitSummary();
  }

  /**
   * Get all notifications
   */
  getAll() {
    return Array.from(this.notifications.values()).filter((n) => !n.dismissed);
  }

  /**
   * Get active (in-progress) notifications
   */
  getActive() {
    return this.getAll().filter((n) => this.getStatus(n) === "in_progress");
  }

  /**
   * Return the kind of trade currently in-flight for a mint, or null.
   * @returns {"buying"|"selling"|null}
   */
  getInFlightTradeForMint(mint) {
    if (!mint) return null;
    for (const n of this.getActive()) {
      if (actionEntityMint(n) !== mint) continue;
      if (TRADE_SELL_ACTION_TYPES.has(n?.action_type)) return "selling";
      if (TRADE_BUY_ACTION_TYPES.has(n?.action_type)) return "buying";
    }
    return null;
  }

  /**
   * Get completed notifications (includes dismissed for history)
   */
  getCompleted({ includeDismissed = true } = {}) {
    return Array.from(this.notifications.values()).filter((n) => {
      if (this.getStatus(n) !== "completed") {
        return false;
      }
      return includeDismissed || !n.dismissed;
    });
  }

  /**
   * Get failed notifications (includes dismissed for history)
   */
  getFailed({ includeDismissed = true } = {}) {
    return Array.from(this.notifications.values()).filter((n) => {
      if (this.getStatus(n) !== "failed") {
        return false;
      }
      return includeDismissed || !n.dismissed;
    });
  }

  getNotification(actionId) {
    return this.notifications.get(actionId) || null;
  }

  /**
   * Get unread count
   */
  getUnreadCount() {
    if (Number.isFinite(this.serverUnread)) {
      return this.serverUnread;
    }
    return this.getAll().filter((n) => !n.read).length;
  }

  /**
   * Subscribe to notification updates
   */
  subscribe(callback) {
    this.subscribers.add(callback);
    try {
      callback({
        type: "summary",
        summary: this.getSummary(),
        recent: this.getRecent(8),
      });
    } catch (error) {
      console.error("Subscriber callback error during registration:", error);
    }
    return () => this.subscribers.delete(callback);
  }

  /**
   * Notify all subscribers
   */
  notifySubscribers(event) {
    for (const callback of this.subscribers) {
      try {
        callback(event);
      } catch (error) {
        console.error("Subscriber callback error:", error);
      }
    }
  }

  async syncActiveActions({ reason = "manual" } = {}) {
    if (this.activeSyncPromise) {
      return this.activeSyncPromise;
    }

    this.activeSyncPromise = this.performActiveSync(reason);

    try {
      await this.activeSyncPromise;
    } finally {
      this.activeSyncPromise = null;
    }
  }

  async performActiveSync(reason) {
    try {
      const response = await fetch("/api/actions/active", {
        headers: {
          "Cache-Control": "no-store",
          "X-Requested-With": "fetch",
        },
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const payload = await response.json();
      const actions = Array.isArray(payload?.actions) ? payload.actions : [];

      // Persisted totals across the whole DB (not just the in-memory cache) so
      // the center's tab badges reflect full history.
      this.serverCounts = {
        in_progress: Number(payload?.in_progress) || 0,
        completed: Number(payload?.completed) || 0,
        failed: Number(payload?.failed) || 0,
      };
      this.serverUnread = Number(payload?.unread) || 0;

      const changed = this.upsertActions(actions);

      if (changed) {
        this.notifySubscribers({ type: "bulk_update", reason });
      }

      this.emitSummary();
    } catch (error) {
      console.error("[NotificationManager] Failed to sync active actions", error);
      this.notifySubscribers({
        type: "sync_error",
        reason,
        error: error.message,
      });
    }
  }

  syncFromHistory(actions, { silent = false } = {}) {
    const changed = this.upsertActions(actions);

    if (changed && !silent) {
      this.notifySubscribers({ type: "history_synced" });
    }

    if (changed) {
      this.emitSummary();
    }
  }

  upsertActions(actions, options = {}) {
    if (!Array.isArray(actions) || actions.length === 0) {
      return false;
    }

    let changed = false;

    for (const action of actions) {
      if (!action || !action.id) continue;
      const existing = this.notifications.get(action.id) || null;
      const record = this.buildNotificationRecord(action, existing, options);

      if (!existing || !this.areRecordsEqual(existing, record)) {
        this.notifications.set(action.id, record);
        changed = true;
      }
    }

    return changed;
  }

  buildNotificationRecord(action, existing = null, options = {}) {
    const { resetRead = false, resetDismissed = false } = options;

    const read = resetRead ? false : Boolean(action?.read || existing?.read || false);
    const dismissed = resetDismissed
      ? false
      : Boolean(action?.dismissed || existing?.dismissed || false);

    const timestamp =
      existing?.timestamp ||
      action?.timestamp ||
      action?.completed_at ||
      action?.started_at ||
      new Date().toISOString();

    const merged = {
      ...(existing || {}),
      ...action,
    };

    merged.read = read;
    merged.dismissed = dismissed;
    merged.timestamp = timestamp;

    return merged;
  }

  async postActionMutation(url) {
    const response = await fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Requested-With": "fetch",
      },
      cache: "no-store",
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const payload = await response.json();
    if (!payload?.success) {
      throw new Error("Action notification update failed");
    }
    return payload;
  }

  areRecordsEqual(a, b) {
    if (a === b) {
      return true;
    }

    try {
      return JSON.stringify(a) === JSON.stringify(b);
    } catch {
      return false;
    }
  }

  /**
   * Get normalized status from notification state
   */
  getStatus(notification) {
    if (!notification || typeof notification !== "object") {
      return "";
    }

    const state = notification.state;
    if (!state || typeof state !== "object") {
      return "";
    }

    return state.status || "";
  }

  /**
   * Aggregate summary metrics for dashboards and UI badges
   */
  getSummary() {
    const all = Array.from(this.notifications.values()); // Include dismissed for metrics
    const now = Date.now();
    const dayAgo = now - 24 * 60 * 60 * 1000;

    let active = 0;
    let completed24h = 0;
    let failed24h = 0;

    for (const item of all) {
      const status = this.getStatus(item);
      const timestampMs = new Date(this.getTimestamp(item)).getTime();
      const withinDay = Number.isFinite(timestampMs) && timestampMs >= dayAgo;

      // Only count non-dismissed as active
      if (status === "in_progress" && !item.dismissed) {
        active += 1;
      } else if (status === "completed" && withinDay) {
        // Count all completed in last 24h (including dismissed)
        completed24h += 1;
      } else if (status === "failed" && withinDay) {
        // Count all failed in last 24h (including dismissed)
        failed24h += 1;
      }
    }

    // Prefer DB-accurate totals from the server when available (full history);
    // fall back to the in-memory tally before the first sync.
    const sc = this.serverCounts || null;
    const totals = sc
      ? {
          in_progress: sc.in_progress,
          completed: sc.completed,
          failed: sc.failed,
          all: sc.in_progress + sc.completed + sc.failed,
        }
      : null;

    return {
      total: all.filter((n) => !n.dismissed).length, // Total visible (non-dismissed)
      active,
      completed24h,
      failed24h,
      totals,
      unread: this.getUnreadCount(),
      connection: {
        status: this.isConnected ? "connected" : "disconnected",
        changedAt: this.lastConnectionChange,
      },
    };
  }

  /**
   * Get latest notifications sorted by most recent timestamp
   */
  getRecent(limit = 5) {
    return this.getAll()
      .sort((a, b) => {
        const aTime = new Date(this.getTimestamp(a)).getTime();
        const bTime = new Date(this.getTimestamp(b)).getTime();
        return (Number.isFinite(bTime) ? bTime : 0) - (Number.isFinite(aTime) ? aTime : 0);
      })
      .slice(0, limit);
  }

  emitSummary() {
    this.notifySubscribers({
      type: "summary",
      summary: this.getSummary(),
      recent: this.getRecent(8),
    });
  }

  getTimestamp(notification) {
    if (!notification) {
      return "";
    }

    return notification.completed_at || notification.timestamp || notification.started_at || "";
  }

  /**
   * Disconnect and cleanup
   */
  disconnect() {
    if (this.eventSource) {
      this.eventSource.close();
      this.eventSource = null;
    }

    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    // Clear all auto-dismiss timers
    for (const timer of this.autoDismissTimers.values()) {
      clearTimeout(timer);
    }
    this.autoDismissTimers.clear();

    this.isConnected = false;
    this.subscribers.clear();
  }

  /**
   * Fetch action history from database with pagination and filters
   * @param {Object} options - Query options
   * @param {number} options.limit - Number of results per page (default: 50)
   * @param {number} options.offset - Offset for pagination (default: 0)
   * @param {string} options.action_type - Filter by action type (swapbuy, swapsell, positionopen, etc.)
   * @param {string} options.entity_id - Filter by entity ID (mint address, position ID)
   * @param {string} options.state - Filter by state (in_progress, completed, failed, cancelled)
   * @param {string} options.started_after - Filter by start time (RFC3339 format)
   * @param {string} options.started_before - Filter by start time (RFC3339 format)
   * @returns {Promise<Object>} Response with {actions: Action[], total: number, limit: number, offset: number}
   */
  async fetchHistory(options = {}) {
    try {
      const params = new URLSearchParams();

      if (options.limit !== undefined) params.append("limit", options.limit.toString());
      if (options.offset !== undefined) params.append("offset", options.offset.toString());
      if (options.action_type) params.append("action_type", options.action_type);
      if (options.entity_id) params.append("entity_id", options.entity_id);
      if (options.state) params.append("state", options.state);
      if (options.started_after) params.append("started_after", options.started_after);
      if (options.started_before) params.append("started_before", options.started_before);

      const url = `/api/actions/history?${params.toString()}`;
      const response = await fetch(url, {
        headers: {
          "X-Requested-With": "fetch",
        },
        cache: "no-store",
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();
      return data;
    } catch (error) {
      console.error("[NotificationManager] Failed to fetch action history:", error);
      throw error;
    }
  }

  /**
   * Show a toast notification (bridge to toast manager)
   * @param {Object|string} config - Toast configuration or message string
   * @returns {Object} Toast instance with control methods
   */
  showToast(config) {
    const toast = toastManager.show(config);

    // If persistent, also add to notification panel as frontend-only notification
    if (typeof config === "object" && config.persistent) {
      this.addFrontendNotification({
        id: toast.id,
        type: "frontend_toast",
        title: config.title,
        message: config.message || null,
        timestamp: new Date().toISOString(),
        read: false,
        dismissed: false,
        metadata: {
          toastType: config.type,
          icon: config.icon,
        },
      });
    }

    return toast;
  }

  /**
   * Add a frontend-only notification to the panel
   * @param {Object} notification - Notification data
   */
  addFrontendNotification(notification) {
    this.notifications.set(notification.id, notification);
    this.notifySubscribers({
      type: "added",
      notification,
    });
    this.emitSummary();
  }

  /**
   * Get persistent toasts from toast manager (for panel integration)
   * @returns {Array} Array of persistent toast data
   */
  getPersistentToasts() {
    return toastManager.getPersistentToasts();
  }
}

// Global singleton instance
const notificationManager = new NotificationManager();

// Auto-initialize once backend is ready
waitForReady()
  .then(() => notificationManager.init())
  .catch((error) => {
    console.error("[NotificationManager] Failed to start after bootstrap", error);
  });

export { notificationManager, NotificationManager };
