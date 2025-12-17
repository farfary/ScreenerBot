/**
 * TabBar - Global navigation component for main tabs and sub-tabs
 *
 * Features:
 * - Main tabs (server-rendered) and sub-tabs (page-configured)
 * - Accessibility (ARIA roles, keyboard navigation)
 * - State persistence (AppState integration)
 * - Event delegation (memory efficient)
 * - Lifecycle integration (auto-cleanup)
 * - Theme-aware CSS variables
 * - Deep linking support (hash-based)
 * - Async validation hooks
 * - Sound feedback on tab switches
 *
 * Usage:
 * ```js
 * import { TabBar } from "../ui/tab_bar.js";
 *
 * const tabBar = new TabBar({
 *   container: '#subTabsContainer',
 *   tabs: [
 *     { id: 'pool', label: '💧 Pool Service' },
 *     { id: 'all', label: '📋 All Tokens' }
 *   ],
 *   defaultTab: 'pool',
 *   stateKey: 'tokens.activeTab',
 *   onChange: (tabId) => { ... },
 *   beforeChange: async (newTabId, oldTabId) => true, // Can block switch
 *   onShow: () => { ... },
 *   onHide: () => { ... }
 * });
 *
 * // Integrate with lifecycle
 * ctx.manageTabBar(tabBar);
 * ```
 */

/* global sessionStorage, history, queueMicrotask */

import * as AppState from "../core/app_state.js";
import { playTabSwitch } from "../core/sounds.js";

// ============================================================================
// TabBarContext - Event pub/sub for TabBar events
// ============================================================================

class TabBarContext {
  constructor() {
    this.listeners = new Map();
  }

  on(event, callback) {
    if (typeof callback !== "function") return () => {};
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event).add(callback);
    return () => this.off(event, callback);
  }

  off(event, callback) {
    const callbacks = this.listeners.get(event);
    if (callbacks) {
      callbacks.delete(callback);
    }
  }

  emit(event, ...args) {
    const callbacks = this.listeners.get(event);
    if (!callbacks) return;
    callbacks.forEach((callback) => {
      try {
        callback(...args);
      } catch (error) {
        console.error(`[TabBar] Event handler error (${event}):`, error);
      }
    });
  }

  clear() {
    this.listeners.clear();
  }
}

// ============================================================================
// TabBar Component
// ============================================================================

export class TabBar {
  constructor(options = {}) {
    // Validate and setup
    this.container =
      typeof options.container === "string"
        ? document.querySelector(options.container)
        : options.container;

    if (!this.container) {
      throw new Error("[TabBar] Container not found: " + (options.container || "undefined"));
    }

    this.tabs = Array.isArray(options.tabs) ? options.tabs : [];
    this.defaultTab = options.defaultTab || this.tabs[0]?.id;
    this.stateKey = options.stateKey;
    this.useSessionStorage = options.useSessionStorage ?? false; // Prevent multi-tab conflicts
    this.pageName = options.pageName || "unknown";

    // Event handlers
    this.onChange = options.onChange || (() => {});
    this.beforeChange = options.beforeChange || (() => true);
    this.onShow = options.onShow || (() => {});
    this.onHide = options.onHide || (() => {});

    // Context for pub/sub
    this.context = new TabBarContext();

    // State
    this.activeTab = null;
    this.mounted = false;
    this.visible = false;
    this.eventHandler = null;

    // Initialize
    this._mount();
    this._restoreState();
  }

  // --------------------------------------------------------------------------
  // Private: Mounting and HTML generation
  // --------------------------------------------------------------------------

  _mount() {
    if (this.mounted) return;

    // Set data attribute for cleanup coordination
    this.container.setAttribute("data-page", this.pageName);
    this.container.setAttribute("role", "tablist");
    this.container.setAttribute("data-ui", "tab-bar");

    // Generate HTML
    const html = this.tabs
      .map((tab) => {
        const active = tab.id === this.activeTab;
        return `
          <button
            class="sub-tab"
            data-tab-id="${this._escapeHtml(tab.id)}"
            role="tab"
            aria-selected="${active}"
            tabindex="${active ? "0" : "-1"}"
            type="button"
          >
            ${tab.label}
          </button>
        `;
      })
      .join("");

    this.container.innerHTML = html;

    // Attach single delegated event listener
    this.eventHandler = (event) => this._handleClick(event);
    this.container.addEventListener("click", this.eventHandler);

    // Attach keyboard navigation
    this.keyboardHandler = (event) => this._handleKeyboard(event);
    this.container.addEventListener("keydown", this.keyboardHandler);

    // Setup scroll navigation (mouse wheel + overflow detection)
    this._setupScrollNavigation();

    this.mounted = true;
  }

  // --------------------------------------------------------------------------
  // Private: Scroll Navigation
  // --------------------------------------------------------------------------

  _setupScrollNavigation() {
    // Mouse wheel horizontal scroll support
    this.wheelHandler = (event) => {
      // Only handle if there's horizontal overflow
      if (this.container.scrollWidth <= this.container.clientWidth) return;

      // Convert vertical scroll to horizontal
      if (Math.abs(event.deltaY) > Math.abs(event.deltaX)) {
        event.preventDefault();
        this.container.scrollLeft += event.deltaY;
        this._updateScrollIndicators();
      }
    };
    this.container.addEventListener("wheel", this.wheelHandler, { passive: false });

    // Track scroll position for indicators
    this.scrollHandler = () => this._updateScrollIndicators();
    this.container.addEventListener("scroll", this.scrollHandler, { passive: true });

    // Watch for resize to update indicators
    this.resizeObserver = new ResizeObserver(() => {
      this._updateScrollIndicators();
    });
    this.resizeObserver.observe(this.container);

    // Initial update
    requestAnimationFrame(() => this._updateScrollIndicators());
  }

  _updateScrollIndicators() {
    const { scrollLeft, scrollWidth, clientWidth } = this.container;
    const wrapper = this.container.parentElement;

    // Check if wrapper has the scroll wrapper class
    if (!wrapper || !wrapper.classList.contains("tab-scroll-wrapper")) {
      // Add wrapper classes directly to container's parent if it exists
      // or use container itself for indicator tracking
      const target = wrapper || this.container;

      const canScrollLeft = scrollLeft > 1;
      const canScrollRight = scrollLeft < scrollWidth - clientWidth - 1;

      target.classList.toggle("can-scroll-left", canScrollLeft);
      target.classList.toggle("can-scroll-right", canScrollRight);
      return;
    }

    const canScrollLeft = scrollLeft > 1;
    const canScrollRight = scrollLeft < scrollWidth - clientWidth - 1;

    wrapper.classList.toggle("can-scroll-left", canScrollLeft);
    wrapper.classList.toggle("can-scroll-right", canScrollRight);
  }

  _cleanupScrollNavigation() {
    if (this.wheelHandler) {
      this.container.removeEventListener("wheel", this.wheelHandler);
      this.wheelHandler = null;
    }
    if (this.scrollHandler) {
      this.container.removeEventListener("scroll", this.scrollHandler);
      this.scrollHandler = null;
    }
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
      this.resizeObserver = null;
    }
  }

  _handleClick(event) {
    const button = event.target.closest("[data-tab-id]");
    if (!button) return;

    const tabId = button.getAttribute("data-tab-id");
    if (tabId) {
      this.setActive(tabId);
    }
  }

  _handleKeyboard(event) {
    const buttons = Array.from(this.container.querySelectorAll("[data-tab-id]"));
    const currentIndex = buttons.findIndex(
      (btn) => btn.getAttribute("data-tab-id") === this.activeTab
    );

    let targetIndex = currentIndex;

    switch (event.key) {
      case "ArrowLeft":
        event.preventDefault();
        targetIndex = currentIndex > 0 ? currentIndex - 1 : buttons.length - 1;
        break;
      case "ArrowRight":
        event.preventDefault();
        targetIndex = currentIndex < buttons.length - 1 ? currentIndex + 1 : 0;
        break;
      case "Home":
        event.preventDefault();
        targetIndex = 0;
        break;
      case "End":
        event.preventDefault();
        targetIndex = buttons.length - 1;
        break;
      default:
        return;
    }

    if (targetIndex !== currentIndex && buttons[targetIndex]) {
      const tabId = buttons[targetIndex].getAttribute("data-tab-id");
      this.setActive(tabId);
      buttons[targetIndex].focus();
    }
  }

  _escapeHtml(value) {
    if (value === null || value === undefined) return "";
    return String(value)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  // --------------------------------------------------------------------------
  // Private: State management
  // --------------------------------------------------------------------------

  _restoreState() {
    let savedTab = null;

    // Try to restore from URL hash first (deep linking)
    const hash = window.location.hash.slice(1); // Remove #
    if (hash && this.tabs.some((t) => t.id === hash)) {
      savedTab = hash;
    }

    // Try to restore from storage
    if (!savedTab && this.stateKey) {
      const storage = this.useSessionStorage ? sessionStorage : AppState;
      const loaded = this.useSessionStorage
        ? storage.getItem(this.stateKey)
        : storage.load(this.stateKey);
      if (loaded && this.tabs.some((t) => t.id === loaded)) {
        savedTab = loaded;
      }
    }

    // Fallback to default
    const initialTab = savedTab || this.defaultTab;
    if (initialTab) {
      this.setActive(initialTab, { silent: true, skipValidation: true });
    }
  }

  _saveState() {
    if (!this.stateKey || !this.activeTab) return;

    const storage = this.useSessionStorage ? sessionStorage : AppState;
    if (this.useSessionStorage) {
      storage.setItem(this.stateKey, this.activeTab);
    } else {
      storage.save(this.stateKey, this.activeTab);
    }
  }

  _updateHash() {
    if (this.activeTab && window.location.hash !== `#${this.activeTab}`) {
      // Update hash without triggering scroll or full navigation
      history.replaceState(null, "", `#${this.activeTab}`);
    }
  }

  // --------------------------------------------------------------------------
  // Public API
  // --------------------------------------------------------------------------

  async setActive(tabId, options = {}) {
    const { silent = false, skipValidation = false } = options;

    if (!tabId || !this.tabs.some((t) => t.id === tabId)) {
      console.warn(`[TabBar] Invalid tab ID: ${tabId}`);
      return false;
    }

    if (this.activeTab === tabId) {
      return true;
    }

    // Run beforeChange validation (can be async)
    if (!skipValidation) {
      try {
        const canChange = await Promise.resolve(this.beforeChange(tabId, this.activeTab));
        if (!canChange) {
          return false;
        }
      } catch (error) {
        console.error("[TabBar] beforeChange validation failed:", error);
        return false;
      }
    }

    const oldTab = this.activeTab;
    this.activeTab = tabId;

    // Play tab switch sound (only for user-initiated switches, not silent/restore)
    if (!silent) {
      playTabSwitch();
    }

    // Update DOM
    this._updateTabButtons();

    // Persist state
    this._saveState();

    // Update URL hash
    this._updateHash();

    // Emit events
    if (!silent) {
      this.context.emit("change", tabId, oldTab);
      try {
        this.onChange(tabId, oldTab);
      } catch (error) {
        console.error("[TabBar] onChange handler error:", error);
      }
    }

    return true;
  }

  _updateTabButtons() {
    this.container.querySelectorAll("[data-tab-id]").forEach((button) => {
      const tabId = button.getAttribute("data-tab-id");
      const isActive = tabId === this.activeTab;
      button.classList.toggle("active", isActive);
      button.setAttribute("aria-selected", isActive);
      button.setAttribute("tabindex", isActive ? "0" : "-1");
    });
  }

  _remountButtons() {
    if (!this.mounted) return;

    // Regenerate HTML for current tabs
    const html = this.tabs
      .map((tab) => {
        const active = tab.id === this.activeTab;
        return `
          <button
            class="sub-tab"
            data-tab-id="${this._escapeHtml(tab.id)}"
            role="tab"
            aria-selected="${active}"
            tabindex="${active ? "0" : "-1"}"
            type="button"
          >
            ${tab.label}
          </button>
        `;
      })
      .join("");

    // Update container content
    this.container.innerHTML = html;
  }

  show(options = {}) {
    const { silent = false, force = false } = options;

    // Skip if already visible unless force is true
    if (this.visible && !force) return;

    // Remount to ensure correct buttons are displayed (important for shared containers)
    this._remountButtons();
    this._updateTabButtons();

    this.container.style.display = "flex";
    this.visible = true;

    if (!silent) {
      this.context.emit("show");
      try {
        this.onShow();
      } catch (error) {
        console.error("[TabBar] onShow handler error:", error);
      }
    }
  }

  hide(options = {}) {
    const { silent = false } = options;

    if (!this.visible) return;

    this.container.style.display = "none";
    this.visible = false;

    // Clear container content when hiding to prevent visual artifacts
    // This is important for shared containers across pages
    this.container.innerHTML = "";

    if (!silent) {
      this.context.emit("hide");
      try {
        this.onHide();
      } catch (error) {
        console.error("[TabBar] onHide handler error:", error);
      }
    }
  }

  getActiveTab() {
    return this.activeTab;
  }

  setTabs(tabs, options = {}) {
    const { merge = false } = options;

    if (merge) {
      // Merge new tabs with existing ones
      const existingIds = new Set(this.tabs.map((t) => t.id));
      const newTabs = tabs.filter((t) => !existingIds.has(t.id));
      this.tabs = [...this.tabs, ...newTabs];
    } else {
      this.tabs = tabs;
    }

    // Re-render
    if (this.mounted) {
      this.mounted = false;
      this._mount();
      this._updateTabButtons();
    }
  }

  destroy() {
    if (!this.mounted) return;

    // Remove event listeners
    if (this.eventHandler) {
      this.container.removeEventListener("click", this.eventHandler);
      this.eventHandler = null;
    }

    if (this.keyboardHandler) {
      this.container.removeEventListener("keydown", this.keyboardHandler);
      this.keyboardHandler = null;
    }

    // Cleanup scroll navigation
    this._cleanupScrollNavigation();

    // Clear context
    this.context.clear();

    // Clear container if owned by this instance
    if (this.container.getAttribute("data-page") === this.pageName) {
      this.container.innerHTML = "";
      this.container.style.display = "none";
      this.container.removeAttribute("data-page");
      this.container.removeAttribute("role");
      this.container.removeAttribute("data-ui");
    }

    this.mounted = false;
    this.visible = false;
    this.activeTab = null;
  }

  // Pub/sub helpers
  on(event, callback) {
    return this.context.on(event, callback);
  }

  off(event, callback) {
    this.context.off(event, callback);
  }
}

// ============================================================================
// TabBarManager - Singleton for global coordination
// ============================================================================

class TabBarManagerClass {
  constructor() {
    this.instances = new Map(); // pageName -> TabBar instance
    this.currentPage = null;
  }

  register(pageName, tabBar) {
    if (!pageName || !tabBar) {
      console.warn("[TabBarManager] Invalid registration", {
        pageName,
        tabBar,
      });
      return;
    }
    this.instances.set(pageName, tabBar);
  }

  unregister(pageName) {
    const tabBar = this.instances.get(pageName);
    if (tabBar) {
      tabBar.destroy();
      this.instances.delete(pageName);
    }
  }

  onPageSwitch(newPage, oldPage) {
    // Use queueMicrotask to ensure DOM is ready after router innerHTML replacement
    queueMicrotask(() => {
      // Hide old page's tab bar
      if (oldPage) {
        const oldTabBar = this.instances.get(oldPage);
        if (oldTabBar) {
          oldTabBar.hide({ silent: true });
        }
      }

      // Show new page's tab bar
      if (newPage) {
        const newTabBar = this.instances.get(newPage);
        if (newTabBar) {
          newTabBar.show();
        }
      }

      this.currentPage = newPage;
    });
  }

  hideAll() {
    this.instances.forEach((tabBar) => tabBar.hide({ silent: true }));
  }

  getActiveTabBar() {
    return this.currentPage ? this.instances.get(this.currentPage) : null;
  }

  getAllTabBars() {
    return Array.from(this.instances.values());
  }
}

export const TabBarManager = new TabBarManagerClass();
