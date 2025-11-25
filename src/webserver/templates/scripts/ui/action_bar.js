/**
 * ActionBar - Centralized action bar component for pages with tabs/subtabs
 *
 * Renders into #toolbarContainer and provides consistent UI for:
 * - Title/subtitle (optional)
 * - Window/time selector (optional)
 * - Action buttons (primary/secondary)
 *
 * Usage:
 * ```js
 * import { ActionBar } from "../ui/action_bar.js";
 *
 * const actionBar = new ActionBar({
 *   container: "#toolbarContainer"
 * });
 *
 * // Configure for a subtab
 * actionBar.configure({
 *   title: "Balance Overview",
 *   subtitle: "Track your wallet performance",
 *   windowSelector: {
 *     options: [
 *       { id: "24", label: "24h", value: 24 },
 *       { id: "168", label: "7d", value: 168 }
 *     ],
 *     active: 24,
 *     onChange: (value) => { ... }
 *   },
 *   actions: [
 *     { id: "export", label: "Export CSV", icon: "icon-download", variant: "secondary", onClick: () => {} },
 *     { id: "refresh", label: "Refresh", icon: "icon-refresh-cw", variant: "primary", onClick: () => {} }
 *   ]
 * });
 *
 * // Clear when leaving
 * actionBar.clear();
 *
 * // Dispose when done
 * actionBar.dispose();
 * ```
 */

/* global queueMicrotask */

function escapeHtml(str) {
  if (str == null) return "";
  return String(str)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

export class ActionBar {
  constructor(options = {}) {
    this.container =
      typeof options.container === "string"
        ? document.querySelector(options.container)
        : options.container;

    if (!this.container) {
      console.warn("[ActionBar] Container not found:", options.container);
      return;
    }

    this.config = null;
    this.mounted = false;
    this.eventCleanups = [];
  }

  /**
   * Configure the action bar with new settings
   * @param {Object} config - Configuration object
   * @param {string} [config.title] - Main title text
   * @param {string} [config.subtitle] - Subtitle/description text
   * @param {string} [config.icon] - Icon class for title (e.g., "icon-wallet")
   * @param {Object} [config.windowSelector] - Window/time selector config
   * @param {Array} [config.windowSelector.options] - Array of {id, label, value}
   * @param {number} [config.windowSelector.active] - Currently active value
   * @param {Function} [config.windowSelector.onChange] - Callback when changed
   * @param {Array} [config.actions] - Action buttons array
   * @param {string} config.actions[].id - Button ID
   * @param {string} config.actions[].label - Button label
   * @param {string} [config.actions[].icon] - Icon class
   * @param {string} [config.actions[].variant] - "primary" | "secondary" | "danger"
   * @param {Function} config.actions[].onClick - Click handler
   * @param {boolean} [config.actions[].disabled] - Whether button is disabled
   */
  configure(config) {
    this.config = config;
    this._render();
    this._show();
  }

  /**
   * Update a specific action button's state
   * @param {string} actionId - The action button ID
   * @param {Object} updates - Properties to update (disabled, label, loading)
   */
  updateAction(actionId, updates) {
    const btn = this.container.querySelector(`[data-action-id="${actionId}"]`);
    if (!btn) return;

    if (updates.disabled !== undefined) {
      btn.disabled = updates.disabled;
    }

    if (updates.label !== undefined) {
      const labelEl = btn.querySelector(".action-bar-btn-label");
      if (labelEl) {
        labelEl.textContent = updates.label;
      }
    }

    if (updates.loading !== undefined) {
      const iconEl = btn.querySelector(".action-bar-btn-icon i");
      if (iconEl) {
        if (updates.loading) {
          iconEl.dataset.originalClass = iconEl.className;
          iconEl.className = "icon-loader";
        } else if (iconEl.dataset.originalClass) {
          iconEl.className = iconEl.dataset.originalClass;
          delete iconEl.dataset.originalClass;
        }
      }
    }
  }

  /**
   * Clear the action bar (hide it)
   */
  clear() {
    this._cleanup();
    this.config = null;
    this._hide();
  }

  /**
   * Dispose of the action bar completely
   */
  dispose() {
    this.clear();
    if (this.container) {
      this.container.innerHTML = "";
    }
    this.container = null;
  }

  // --------------------------------------------------------------------------
  // Private methods
  // --------------------------------------------------------------------------

  _show() {
    if (this.container) {
      this.container.style.display = "flex";
    }
  }

  _hide() {
    if (this.container) {
      this.container.style.display = "none";
    }
  }

  _cleanup() {
    this.eventCleanups.forEach((cleanup) => cleanup());
    this.eventCleanups = [];
  }

  _addListener(element, event, handler) {
    if (!element) return;
    element.addEventListener(event, handler);
    this.eventCleanups.push(() => element.removeEventListener(event, handler));
  }

  _render() {
    if (!this.container || !this.config) return;

    this._cleanup();

    const { title, subtitle, icon, windowSelector, actions } = this.config;

    const parts = [];

    // Left section: Title/subtitle
    if (title || subtitle) {
      const iconHtml = icon ? `<i class="${escapeHtml(icon)}"></i>` : "";
      const titleHtml = title ? `<span class="action-bar-title-text">${escapeHtml(title)}</span>` : "";
      const subtitleHtml = subtitle
        ? `<span class="action-bar-subtitle">${escapeHtml(subtitle)}</span>`
        : "";

      parts.push(`
        <div class="action-bar-left">
          <div class="action-bar-title">
            ${iconHtml}
            <div class="action-bar-title-content">
              ${titleHtml}
              ${subtitleHtml}
            </div>
          </div>
        </div>
      `);
    }

    // Center section: Window selector
    if (windowSelector && Array.isArray(windowSelector.options) && windowSelector.options.length > 0) {
      const buttonsHtml = windowSelector.options
        .map((opt) => {
          const isActive = opt.value === windowSelector.active;
          return `
            <button
              class="action-bar-window-btn ${isActive ? "active" : ""}"
              data-window-value="${escapeHtml(String(opt.value))}"
              type="button"
            >
              ${escapeHtml(opt.label)}
            </button>
          `;
        })
        .join("");

      parts.push(`
        <div class="action-bar-center">
          <div class="action-bar-window-selector">
            ${buttonsHtml}
          </div>
        </div>
      `);
    }

    // Right section: Action buttons
    if (actions && Array.isArray(actions) && actions.length > 0) {
      const buttonsHtml = actions
        .map((action) => {
          const variant = action.variant || "secondary";
          const iconHtml = action.icon
            ? `<span class="action-bar-btn-icon"><i class="${escapeHtml(action.icon)}"></i></span>`
            : "";
          const labelHtml = action.label
            ? `<span class="action-bar-btn-label">${escapeHtml(action.label)}</span>`
            : "";
          const disabled = action.disabled ? "disabled" : "";

          return `
            <button
              class="action-bar-btn action-bar-btn--${escapeHtml(variant)}"
              data-action-id="${escapeHtml(action.id)}"
              type="button"
              ${disabled}
            >
              ${iconHtml}${labelHtml}
            </button>
          `;
        })
        .join("");

      parts.push(`
        <div class="action-bar-right">
          <div class="action-bar-actions">
            ${buttonsHtml}
          </div>
        </div>
      `);
    }

    this.container.innerHTML = `<div class="action-bar">${parts.join("")}</div>`;

    // Attach event listeners
    this._attachEventListeners();
  }

  _attachEventListeners() {
    const { windowSelector, actions } = this.config || {};

    // Window selector buttons
    if (windowSelector && windowSelector.onChange) {
      const windowBtns = this.container.querySelectorAll(".action-bar-window-btn");
      windowBtns.forEach((btn) => {
        this._addListener(btn, "click", () => {
          const value = parseInt(btn.dataset.windowValue, 10);

          // Update active state visually
          windowBtns.forEach((b) => b.classList.remove("active"));
          btn.classList.add("active");

          // Call onChange
          windowSelector.onChange(value);
        });
      });
    }

    // Action buttons
    if (actions && Array.isArray(actions)) {
      actions.forEach((action) => {
        if (action.onClick) {
          const btn = this.container.querySelector(`[data-action-id="${action.id}"]`);
          if (btn) {
            this._addListener(btn, "click", action.onClick);
          }
        }
      });
    }
  }
}

/**
 * ActionBarManager - Singleton for coordinated action bar management across pages
 *
 * Similar to TabBarManager, this handles:
 * - Tracking which page has an action bar
 * - Hiding action bars when switching to pages without one
 * - Showing action bars when switching to pages with one
 */
class ActionBarManagerClass {
  constructor() {
    this.instances = new Map();
    this.currentPage = null;
  }

  register(pageName, actionBar) {
    if (!pageName || !actionBar) {
      console.warn("[ActionBarManager] Invalid registration", { pageName, actionBar });
      return;
    }
    this.instances.set(pageName, actionBar);
  }

  unregister(pageName) {
    const actionBar = this.instances.get(pageName);
    if (actionBar) {
      actionBar.dispose();
      this.instances.delete(pageName);
    }
  }

  onPageSwitch(newPage, oldPage) {
    // Use queueMicrotask to ensure DOM is ready after router innerHTML replacement
    queueMicrotask(() => {
      // Hide old page's action bar
      if (oldPage) {
        const oldActionBar = this.instances.get(oldPage);
        if (oldActionBar) {
          oldActionBar.clear();
        }
      }

      // If new page doesn't have an action bar registered, ensure container is hidden
      if (newPage && !this.instances.has(newPage)) {
        // Hide the global container if the new page doesn't use ActionBar
        const container = document.querySelector("#toolbarContainer");
        if (container) {
          container.style.display = "none";
        }
      }

      this.currentPage = newPage;
    });
  }

  hideAll() {
    this.instances.forEach((actionBar) => actionBar.clear());
  }

  getActiveActionBar() {
    return this.currentPage ? this.instances.get(this.currentPage) : null;
  }
}

export const ActionBarManager = new ActionBarManagerClass();
