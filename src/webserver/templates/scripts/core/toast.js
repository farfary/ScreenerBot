/**
 * Toast Manager - Core notification system
 *
 * Singleton service for managing toast notifications with:
 * - Queue management with priority levels
 * - Smart positioning (drawer-aware)
 * - Auto-dismiss with hover pause
 * - Maximum 5 visible toasts
 * - Event emitter for lifecycle hooks
 * - Integration with NotificationManager for persistent toasts
 */

const PRIORITY_ORDER = {
  critical: 0,
  high: 1,
  normal: 2,
  low: 3,
};

const DEFAULT_DURATIONS = {
  success: 4000,
  error: 8000,
  warning: 6000,
  info: 4000,
  loading: 0, // Manual dismiss only
  action: 0, // Manual dismiss only
};

const LOADING_TIMEOUT = 60000; // 60 seconds maximum for loading toasts

const ICONS = {
  success: "✓",
  error: "✕",
  warning: "⚠",
  info: "ℹ",
  loading: "⟳",
  action: '<i class="icon-zap"></i>',
};

class ToastManager {
  constructor() {
    if (ToastManager.instance) {
      return ToastManager.instance;
    }

    this.toasts = new Map(); // id -> toast instance
    this.queue = []; // Pending toasts
    this.visibleToasts = []; // Currently displayed toast IDs
    this.maxVisible = 5;
    this.maxQueue = 20; // Maximum queued toasts
    this.nextId = 1;
    this.container = null;
    this.drawerOpen = false;
    this.subscribers = new Map(); // event -> callbacks[]
    this.groups = new Map(); // groupKey -> toast IDs[]
    this.pausedToasts = new Set(); // Toast IDs with paused timers

    ToastManager.instance = this;
  }

  /**
   * Show a toast notification
   * @param {ToastConfig} config - Toast configuration
   * @returns {Toast} Toast instance with control methods
   */
  show(config) {
    const toastConfig = this._normalizeConfig(config);
    const toastId = `toast-${this.nextId++}`;

    // Handle grouping
    if (toastConfig.groupKey) {
      const grouped = this._handleGrouping(toastConfig);
      if (grouped) {
        return grouped;
      }
    }

    // Create toast instance
    const toast = {
      id: toastId,
      config: toastConfig,
      element: null,
      timer: null,
      timeoutTimer: null, // For loading toast timeout fallback
      createdAt: Date.now(),
      dismissed: false,
    };

    // Add control methods
    toast.dismiss = () => this.dismiss(toastId);
    toast.update = (updates) => this.update(toastId, updates);
    toast.updateProgress = (percent) =>
      this.update(toastId, { progress: Math.min(100, Math.max(0, percent)) });
    toast.complete = (message) => {
      this.update(toastId, {
        type: "success",
        message: message || "Complete",
        progress: 100,
      });
      setTimeout(() => this.dismiss(toastId), 2000);
    };
    toast.error = (message) => {
      this.update(toastId, {
        type: "error",
        message: message || "Failed",
      });
    };

    this.toasts.set(toastId, toast);

    // Add to group if applicable
    if (toastConfig.groupKey) {
      if (!this.groups.has(toastConfig.groupKey)) {
        this.groups.set(toastConfig.groupKey, []);
      }
      this.groups.get(toastConfig.groupKey).push(toastId);
    }

    // Add to queue or show immediately
    if (this.visibleToasts.length >= this.maxVisible) {
      // Check queue limit
      if (this.queue.length >= this.maxQueue) {
        // Remove oldest low-priority toast from queue
        const removed = this._removeLowestPriorityFromQueue();
        if (removed) {
          this._emit("queue-overflow", { removed, new: toastId });
        } else {
          // Queue full with all high priority - reject new toast
          this._emit("queue-full", { rejected: toastId, queueSize: this.queue.length });
          this.toasts.delete(toastId);
          console.warn("Toast queue full - rejecting new toast:", toastConfig.title);
          return toast;
        }
      }
      this.queue.push(toastId);
      this._sortQueue();
    } else {
      this._showToast(toastId);
    }

    this._emit("created", toast);

    return toast;
  }

  /**
   * Dismiss a specific toast
   * @param {string} toastId - Toast ID to dismiss
   */
  dismiss(toastId) {
    const toast = this.toasts.get(toastId);
    if (!toast || toast.dismissed) {
      return;
    }

    toast.dismissed = true;

    // Clear timers
    if (toast.timer) {
      clearTimeout(toast.timer);
      toast.timer = null;
    }
    if (toast.timeoutTimer) {
      clearTimeout(toast.timeoutTimer);
      toast.timeoutTimer = null;
    }

    // Clean up paused state (fix memory leak)
    this.pausedToasts.delete(toastId);

    // Remove from visible list
    const index = this.visibleToasts.indexOf(toastId);
    if (index !== -1) {
      this.visibleToasts.splice(index, 1);
    }

    // Remove from groups
    if (toast.config.groupKey) {
      const group = this.groups.get(toast.config.groupKey);
      if (group) {
        const groupIndex = group.indexOf(toastId);
        if (groupIndex !== -1) {
          group.splice(groupIndex, 1);
        }
        if (group.length === 0) {
          this.groups.delete(toast.config.groupKey);
        }
      }
    }

    // Animate out
    if (toast.element) {
      toast.element.classList.add("toast--exiting");
      setTimeout(() => {
        if (toast.element && toast.element.parentNode) {
          toast.element.remove();
        }
        this.toasts.delete(toastId);
        this._emit("dismissed", toast);

        // Show next queued toast
        this._showNextQueued();
      }, 300); // Match CSS animation duration
    } else {
      this.toasts.delete(toastId);
      this._emit("dismissed", toast);
      this._showNextQueued();
    }
  }

  /**
   * Dismiss all visible toasts
   */
  dismissAll() {
    const toastIds = [...this.visibleToasts];
    toastIds.forEach((id) => this.dismiss(id));
  }

  /**
   * Update a toast's configuration
   * @param {string} toastId - Toast ID
   * @param {Partial<ToastConfig>} updates - Configuration updates
   */
  update(toastId, updates) {
    const toast = this.toasts.get(toastId);
    if (!toast || toast.dismissed) {
      return;
    }

    // Merge updates
    Object.assign(toast.config, updates);

    // Re-render if visible
    if (toast.element) {
      this._updateToastElement(toast);
    }

    this._emit("updated", toast);
  }

  /**
   * Notify toast manager of drawer state change
   * @param {boolean} isOpen - Whether drawer is open
   */
  onDrawerStateChange(isOpen) {
    this.drawerOpen = isOpen;
    this._updateContainerPosition();
  }

  /**
   * Get all persistent toasts (for NotificationManager integration)
   * @returns {Array} Array of persistent toast data
   */
  getPersistentToasts() {
    const persistent = [];
    this.toasts.forEach((toast) => {
      if (toast.config.persistent && !toast.dismissed) {
        persistent.push({
          id: toast.id,
          type: "frontend_toast",
          title: toast.config.title,
          message: toast.config.message,
          timestamp: new Date(toast.createdAt).toISOString(),
          read: false,
          dismissed: false,
          metadata: {
            toastType: toast.config.type,
            icon: toast.config.icon,
          },
        });
      }
    });
    return persistent;
  }

  /**
   * Subscribe to toast events
   * @param {string} event - Event name (created, shown, dismissed, updated)
   * @param {Function} callback - Callback function
   * @returns {Function} Unsubscribe function
   */
  on(event, callback) {
    if (!this.subscribers.has(event)) {
      this.subscribers.set(event, []);
    }
    this.subscribers.get(event).push(callback);

    return () => {
      const callbacks = this.subscribers.get(event);
      if (callbacks) {
        const index = callbacks.indexOf(callback);
        if (index !== -1) {
          callbacks.splice(index, 1);
        }
      }
    };
  }

  /**
   * Get toast container element (create if needed)
   * @returns {HTMLElement} Container element
   */
  _getContainer() {
    if (!this.container) {
      this.container = document.createElement("div");
      this.container.className = "toast-container";
      this.container.setAttribute("role", "region");
      this.container.setAttribute("aria-label", "Notifications");
      document.body.appendChild(this.container);
      this._updateContainerPosition();
    }
    return this.container;
  }

  /**
   * Update container position based on drawer state
   */
  _updateContainerPosition() {
    if (!this.container) {
      return;
    }

    if (this.drawerOpen) {
      this.container.classList.add("drawer-open");
    } else {
      this.container.classList.remove("drawer-open");
    }
  }

  /**
   * Show a toast from the queue
   * @param {string} toastId - Toast ID
   */
  _showToast(toastId) {
    const toast = this.toasts.get(toastId);
    if (!toast || toast.dismissed) {
      return;
    }

    // Import Toast class dynamically to avoid circular dependency
    import("../ui/toast.js").then(({ Toast }) => {
      const toastInstance = new Toast(toast.config, toastId);
      toast.element = toastInstance.render();

      // Add to container
      const container = this._getContainer();
      container.appendChild(toast.element);

      // Add to visible list
      this.visibleToasts.push(toastId);

      // Setup hover pause
      this._setupHoverPause(toast);

      // Setup auto-dismiss timer
      if (toast.config.duration > 0) {
        this._startDismissTimer(toast);
      }

      // Setup loading toast timeout fallback
      if (toast.config.type === "loading") {
        toast.timeoutTimer = setTimeout(() => {
          // Call onTimeout callback if provided
          if (toast.config.onTimeout) {
            try {
              toast.config.onTimeout();
            } catch (error) {
              console.error("Toast onTimeout callback error:", error);
            }
          }
          // Convert to error toast
          this.update(toastId, {
            type: "error",
            title: toast.config.title || "Loading timeout",
            message: "Operation took too long",
            duration: 8000,
          });
        }, LOADING_TIMEOUT);
      }

      // Trigger animation
      requestAnimationFrame(() => {
        toast.element.classList.add("toast--visible");
      });

      this._emit("shown", toast);
    });
  }

  /**
   * Show next queued toast
   */
  _showNextQueued() {
    if (this.queue.length > 0 && this.visibleToasts.length < this.maxVisible) {
      const nextId = this.queue.shift();
      this._showToast(nextId);
    }
  }

  /**
   * Sort queue by priority
   */
  _sortQueue() {
    this.queue.sort((a, b) => {
      const toastA = this.toasts.get(a);
      const toastB = this.toasts.get(b);
      if (!toastA || !toastB) {
        return 0;
      }
      return PRIORITY_ORDER[toastA.config.priority] - PRIORITY_ORDER[toastB.config.priority];
    });
  }

  /**
   * Remove lowest priority toast from queue to make room
   * @returns {string|null} Removed toast ID or null if all are high priority
   */
  _removeLowestPriorityFromQueue() {
    // Sort queue to find lowest priority (highest PRIORITY_ORDER value)
    this._sortQueue();

    // Remove from end (lowest priority)
    for (let i = this.queue.length - 1; i >= 0; i--) {
      const toastId = this.queue[i];
      const toast = this.toasts.get(toastId);

      if (toast && toast.config.priority !== "critical" && toast.config.priority !== "high") {
        // Remove this toast
        this.queue.splice(i, 1);
        this.toasts.delete(toastId);
        return toastId;
      }
    }

    return null; // All toasts are critical/high priority
  }

  /**
   * Start auto-dismiss timer
   * @param {Object} toast - Toast instance
   */
  _startDismissTimer(toast) {
    if (toast.timer) {
      clearTimeout(toast.timer);
    }

    toast.timer = setTimeout(() => {
      this.dismiss(toast.id);
    }, toast.config.duration);
  }

  /**
   * Setup hover pause behavior
   * @param {Object} toast - Toast instance
   */
  _setupHoverPause(toast) {
    if (!toast.element || toast.config.duration === 0) {
      return;
    }

    let remainingTime = toast.config.duration;
    let timerStartTime = Date.now();

    toast.element.addEventListener("mouseenter", () => {
      if (toast.timer && !this.pausedToasts.has(toast.id)) {
        clearTimeout(toast.timer);
        // Calculate actual remaining time based on elapsed time since timer started
        const elapsed = Date.now() - timerStartTime;
        remainingTime = Math.max(0, remainingTime - elapsed);
        this.pausedToasts.add(toast.id);
      }
    });

    toast.element.addEventListener("mouseleave", () => {
      if (this.pausedToasts.has(toast.id)) {
        // Restart timer with remaining time
        timerStartTime = Date.now();
        toast.timer = setTimeout(() => {
          this.dismiss(toast.id);
        }, remainingTime);

        this.pausedToasts.delete(toast.id);
      }
    });
  }

  /**
   * Update toast element content
   * @param {Object} toast - Toast instance
   */
  _updateToastElement(toast) {
    if (!toast.element) {
      return;
    }

    // Update title
    const titleEl = toast.element.querySelector(".toast__title");
    if (titleEl && toast.config.title) {
      titleEl.textContent = toast.config.title;
    }

    // Update description
    const descEl = toast.element.querySelector(".toast__description");
    if (descEl) {
      if (toast.config.description) {
        descEl.textContent = toast.config.description;
        descEl.style.display = "block";
      } else {
        descEl.style.display = "none";
      }
    }

    // Update message
    const messageEl = toast.element.querySelector(".toast__message");
    if (messageEl) {
      if (toast.config.message) {
        messageEl.textContent = toast.config.message;
        messageEl.style.display = "block";
      } else {
        messageEl.style.display = "none";
      }
    }

    // Update progress
    const progressBar = toast.element.querySelector(".toast__progress-bar");
    if (progressBar && toast.config.progress !== undefined) {
      progressBar.style.width = `${toast.config.progress}%`;
      // Update ARIA for screen readers
      const progressContainer = toast.element.querySelector(".toast__progress");
      if (progressContainer) {
        progressContainer.setAttribute("aria-valuenow", toast.config.progress);
        // Announce progress for screen readers (only at certain milestones)
        if (toast.config.progress % 25 === 0 || toast.config.progress === 100) {
          progressContainer.setAttribute("aria-label", `${toast.config.progress}% complete`);
        }
      }
    }

    // Update type (change accent color)
    const typeClasses = [
      "toast--success",
      "toast--error",
      "toast--warning",
      "toast--info",
      "toast--loading",
      "toast--action",
    ];
    typeClasses.forEach((cls) => toast.element.classList.remove(cls));
    toast.element.classList.add(`toast--${toast.config.type}`);

    // Update icon
    const iconEl = toast.element.querySelector(".toast__icon");
    if (iconEl && toast.config.icon) {
      iconEl.textContent = toast.config.icon;
    }

    // Update aria-live for dynamic content changes
    if (toast.config.type === "error") {
      toast.element.setAttribute("aria-live", "assertive");
    } else {
      toast.element.setAttribute("aria-live", "polite");
    }
  }

  /**
   * Handle toast grouping
   * @param {ToastConfig} config - Toast configuration
   * @returns {Toast|null} Existing grouped toast or null
   */
  _handleGrouping(config) {
    if (!config.groupKey) {
      return null;
    }

    const group = this.groups.get(config.groupKey);
    if (!group || group.length === 0) {
      return null;
    }

    // Get the first toast in the group (the "parent")
    const parentId = group[0];
    const parentToast = this.toasts.get(parentId);

    if (!parentToast || parentToast.dismissed) {
      return null;
    }

    // Update parent toast to show count
    const count = group.length + 1;
    this.update(parentId, {
      title: config.title,
      message: `${count} items`,
      description: config.description || `Last: ${config.message || ""}`,
    });

    // Return the parent toast
    return parentToast;
  }

  /**
   * Normalize toast configuration
   * @param {string|ToastConfig} config - Configuration
   * @returns {ToastConfig} Normalized configuration
   */
  _normalizeConfig(config) {
    // Handle legacy string format
    if (typeof config === "string") {
      return {
        type: "info",
        title: config,
        message: null,
        description: null,
        icon: ICONS.info,
        duration: DEFAULT_DURATIONS.info,
        priority: "normal",
        persistent: false,
        progress: undefined,
        actions: [],
        groupKey: null,
        onDismiss: null,
      };
    }

    // Validate and normalize actions array
    const validatedActions = [];
    if (Array.isArray(config.actions)) {
      config.actions.forEach((action, index) => {
        if (!action || typeof action !== "object") {
          console.warn(`Toast: Invalid action at index ${index} - must be an object`);
          return;
        }
        if (!action.label || typeof action.label !== "string") {
          console.warn(`Toast: Invalid action at index ${index} - missing 'label' string`);
          return;
        }
        if (!action.callback || typeof action.callback !== "function") {
          console.warn(`Toast: Invalid action at index ${index} - missing 'callback' function`);
          return;
        }
        validatedActions.push({
          label: action.label,
          callback: action.callback,
          style: action.style || "primary",
          persistent: action.persistent || false,
        });
      });
    }

    // Normalize config object
    return {
      type: config.type || "info",
      title: config.title || "",
      message: config.message || null,
      description: config.description || null,
      icon: config.icon || ICONS[config.type || "info"],
      duration:
        config.duration !== undefined ? config.duration : DEFAULT_DURATIONS[config.type || "info"],
      priority: config.priority || "normal",
      persistent: config.persistent || false,
      progress: config.progress,
      actions: validatedActions,
      groupKey: config.groupKey || null,
      onDismiss: config.onDismiss || null,
      onTimeout: config.onTimeout || null,
    };
  }

  /**
   * Emit event to subscribers
   * @param {string} event - Event name
   * @param {any} data - Event data
   */
  _emit(event, data) {
    const callbacks = this.subscribers.get(event);
    if (callbacks) {
      callbacks.forEach((cb) => {
        try {
          cb(data);
        } catch (error) {
          console.error(`Toast event handler error (${event}):`, error);
        }
      });
    }
  }
}

// Create singleton instance
export const toastManager = new ToastManager();

// Export class for testing
export { ToastManager };
