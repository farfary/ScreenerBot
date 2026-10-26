// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

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
 *     { id: 'pool', label: 'Pool Service' },
 *     { id: 'all', label: 'All Tokens' }
 *   ],
 *   defaultTab: 'pool',
 *   stateKey: 'tokens.activeTab',
 *   onChange: (tabId) => { ... },
 *   beforeChange: async (newTabId, oldTabId) => true, // Can block switch
 *   onShow: () => { ... },
 *   onHide: () => { ... }
 * });
 *
 * A tab's `label` is text and `tooltip` optional plain text shown on hover and focus; the
 * bar owns their markup. Tabs carry no glyph: the selected tab is marked by colour and
 * its bar.
 *
 * The row is a tablist named by its page's main tab (`#nav-tab-{page}`), and the page
 * element (`#page-{page}`) is its tabpanel while the bar owns the row. Activation is
 * manual: the arrow keys, Home and End move focus (mirrored in a right-to-left locale),
 * and Enter or Space selects the focused tab.
 *
 * // Integrate with lifecycle
 * ctx.manageTabBar(tabBar);
 * ```
 */

/* global sessionStorage, history, queueMicrotask */

import * as AppState from "../core/app_state.js";
import { dirSign, scrollStart } from "../core/dom.js";
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
    this._claimContainer();

    this.container.innerHTML = this._buttonsHtml();

    // Attach single delegated event listener
    this.eventHandler = (event) => this._handleClick(event);
    this.container.addEventListener("click", this.eventHandler);

    // Attach keyboard navigation
    this.keyboardHandler = (event) => this._handleKeyboard(event);
    this.container.addEventListener("keydown", this.keyboardHandler);

    // Focus moved by the arrows leaves the roving tab stop on another tab; once focus
    // leaves the row, Tab must land on the selected tab again.
    this.focusOutHandler = (event) => {
      if (!this._ownsContainer() || this.container.contains(event.relatedTarget)) return;
      this._updateTabStops();
    };
    this.container.addEventListener("focusout", this.focusOutHandler);

    this.strip = attachTabScrollStrip(this.container);

    this.mounted = true;
  }

  /**
   * #subTabsContainer is shared by every page, and a cached page's TabBar keeps its
   * delegated listeners while hidden. Only the bar that currently owns the container
   * (show()/mount stamp `data-page`, hide() clears it) may react to its events.
   */
  _ownsContainer() {
    return this.container.getAttribute("data-page") === this.pageName;
  }

  /** Stamp the shared container as this page's tablist, named by the page's main tab. */
  _claimContainer() {
    this.container.setAttribute("data-page", this.pageName);
    this.container.setAttribute("role", "tablist");
    this.container.setAttribute("data-ui", "tab-bar");
    this.container.setAttribute("aria-labelledby", `nav-tab-${this.pageName}`);
  }

  /** Clear what `_claimContainer` and `_updateTabButtons` stamped, here and on the panel. */
  _releaseContainer() {
    this.container.style.display = "none";
    this.container.innerHTML = "";
    this.container.removeAttribute("data-page");
    this.container.removeAttribute("data-ui");
    this.container.removeAttribute("role");
    this.container.removeAttribute("aria-labelledby");
    const panel = document.getElementById(`page-${this.pageName}`);
    panel?.removeAttribute("role");
    panel?.removeAttribute("aria-labelledby");
  }

  _handleClick(event) {
    if (!this._ownsContainer()) return;
    const button = event.target.closest("[data-tab-id]");
    if (!button) return;

    const tabId = button.getAttribute("data-tab-id");
    if (tabId) {
      this.setActive(tabId);
    }
  }

  _handleKeyboard(event) {
    if (!this._ownsContainer()) return;
    const buttons = Array.from(this.container.querySelectorAll("[data-tab-id]"));
    if (buttons.length === 0) return;
    const focused = buttons.indexOf(document.activeElement);
    const currentIndex =
      focused !== -1
        ? focused
        : buttons.findIndex((btn) => btn.getAttribute("data-tab-id") === this.activeTab);

    let targetIndex;
    switch (event.key) {
      case "ArrowLeft":
      case "ArrowRight": {
        const step = (event.key === "ArrowRight" ? 1 : -1) * dirSign();
        targetIndex = (currentIndex + step + buttons.length) % buttons.length;
        break;
      }
      case "Home":
        targetIndex = 0;
        break;
      case "End":
        targetIndex = buttons.length - 1;
        break;
      default:
        return;
    }

    event.preventDefault();
    const target = buttons[targetIndex];
    buttons.forEach((button) => button.setAttribute("tabindex", button === target ? "0" : "-1"));
    target.focus();
    this.strip.reveal(target);
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

  _updateHash(historyMode = "replace") {
    if (this.activeTab && window.location.hash !== `#${this.activeTab}`) {
      // Update hash without triggering scroll or full navigation
      history[historyMode === "push" ? "pushState" : "replaceState"](
        { page: this.pageName, subtab: this.activeTab },
        "",
        `#${this.activeTab}`
      );
    }
  }

  // --------------------------------------------------------------------------
  // Public API
  // --------------------------------------------------------------------------

  async setActive(tabId, options = {}) {
    const {
      silent = false,
      skipValidation = false,
      historyMode = silent ? "replace" : "push",
      playSound: shouldPlaySound = !silent,
    } = options;

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
    if (shouldPlaySound) {
      playTabSwitch();
    }

    // Update DOM
    this._updateTabButtons();

    // Persist state
    this._saveState();

    // Update URL hash
    this._updateHash(historyMode);

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
    });
    this._updateTabStops();
    if (!this._ownsContainer()) return;
    const active = this.container.querySelector(".sub-tab.active");
    // The page is the panel the selected tab controls.
    const panel = document.getElementById(`page-${this.pageName}`);
    if (panel && active) {
      panel.setAttribute("role", "tabpanel");
      panel.setAttribute("aria-labelledby", active.id);
    }
    // The current sub-tab is scrolled into view whenever it changes or the bar is shown.
    this.strip.reveal(active);
  }

  /** The row's single tab stop is the selected tab. */
  _updateTabStops() {
    this.container.querySelectorAll("[data-tab-id]").forEach((button) => {
      const isActive = button.getAttribute("data-tab-id") === this.activeTab;
      button.setAttribute("tabindex", isActive ? "0" : "-1");
    });
  }

  _remountButtons() {
    if (!this.mounted) return;

    this.container.innerHTML = this._buttonsHtml();
  }

  _buttonsHtml() {
    return this.tabs
      .map((tab) => {
        const active = tab.id === this.activeTab;
        return `
          <button
            class="sub-tab"
            id="subtab-${this._escapeHtml(this.pageName)}-${this._escapeHtml(tab.id)}"
            data-tab-id="${this._escapeHtml(tab.id)}"
            role="tab"
            aria-selected="${active}"
            aria-controls="page-${this._escapeHtml(this.pageName)}"
            tabindex="${active ? "0" : "-1"}"
            type="button"${tab.tooltip ? ` data-tooltip="${this._escapeHtml(tab.tooltip)}"` : ""}
          >
            ${this._escapeHtml(tab.label)}
          </button>
        `;
      })
      .join("");
  }

  show(options = {}) {
    const { silent = false, force = false } = options;

    // Skip if already visible unless force is true
    if (this.visible && !force) return;

    // Re-claim the shared container for this page (hide() releases it), so the
    // correct buttons render and page-specific CSS keyed on data-page applies.
    this._claimContainer();

    // Remount to ensure correct buttons are displayed (important for shared containers)
    this._remountButtons();
    this.container.style.display = "flex";
    this.container.parentElement?.classList.remove("is-reserved");
    this._updateTabButtons();
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

    // Always release the shared container, even if this bar thinks it is already
    // hidden: #subTabsContainer is shared by every page, and releasing it lets the next
    // page's bar claim a clean one.
    this._releaseContainer();

    if (!this.visible) return;

    this.visible = false;

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

    if (this.focusOutHandler) {
      this.container.removeEventListener("focusout", this.focusOutHandler);
      this.focusOutHandler = null;
    }

    // Clear context
    this.context.clear();

    // Clear container if owned by this instance
    if (this._ownsContainer()) this._releaseContainer();

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

  onPageSwitch(newPage, _oldPage) {
    // Use queueMicrotask to ensure DOM is ready after router innerHTML replacement
    queueMicrotask(() => {
      // Hide all registered tab bars first (defensive — catches cases where
      // the deactivate-lifecycle cleanup missed hiding the old bar, e.g. when
      // the old page's TabBar was unregistered before the microtask ran, or
      // when the CSS [data-ui="tab-bar"] attribute rule kept the container
      // visible despite an inline display:none having been skipped).
      this.hideAll();

      // Show new page's tab bar, or forcibly hide the shared container when
      // the new page has no subtabs at all.
      const newTabBar = newPage ? this.instances.get(newPage) : null;
      if (newTabBar) {
        newTabBar.show();
      } else {
        // No TabBar registered for this page — ensure the container is hidden
        // even if no registered instance was available to hide it above, and that
        // no slot stays reserved for a row this page does not have.
        const container = document.getElementById("subTabsContainer");
        if (container) {
          container.style.display = "none";
          container.parentElement?.classList.remove("is-reserved");
        }
      }

      this.currentPage = newPage;
    });
  }

  hideAll() {
    this.instances.forEach((tabBar) => tabBar.hide({ silent: true }));
  }

  /**
   * Release the row while the router loads `pageName`. When that page already has a
   * registered bar, the row's slot stays reserved (height, surface and rule, no buttons)
   * until the bar is shown, so the row does not blink between two pages that both have
   * one. Any other page, or none, leaves no slot behind.
   */
  releaseFor(pageName) {
    this.hideAll();
    const container = document.getElementById("subTabsContainer");
    container?.parentElement?.classList.toggle(
      "is-reserved",
      Boolean(pageName && this.instances.has(pageName))
    );
  }

  getActiveTabBar() {
    return this.currentPage ? this.instances.get(this.currentPage) : null;
  }

  async syncFromLocation(pageName = this.currentPage) {
    const tabBar = pageName ? this.instances.get(pageName) : null;
    const hashTab = window.location.hash.slice(1);
    if (!tabBar || !hashTab || hashTab === tabBar.activeTab) return;
    if (!tabBar.tabs.some((tab) => tab.id === hashTab)) return;
    await tabBar.setActive(hashTab, {
      skipValidation: true,
      historyMode: "replace",
      playSound: false,
    });
  }

  getAllTabBars() {
    return Array.from(this.instances.values());
  }
}

export const TabBarManager = new TabBarManagerClass();

// ============================================================================
// Tab scroll strip - one row of tabs that scrolls sideways when it does not fit
// ============================================================================

const scrollStrips = new WeakMap();

/**
 * Give a tab row that can overflow its overflow affordances: `scroller` is the
 * `.tab-scroll-area` inside a `.tab-scroll-wrapper`, which also holds the two
 * `.tab-scroll-btn` page buttons (tab_bar.css). The wrapper carries `can-scroll-start` /
 * `can-scroll-end`, which show the edge fade and the page button on each side that
 * still has tabs out of view. Vertical wheel movement scrolls the row while it can move
 * and reaches the page at either end. A scroller gets one strip: a second call returns
 * the first handle, so the page TabBars that share `#subTabsContainer` attach it once.
 *
 * @param {HTMLElement} scroller
 * @returns {{ update: () => void, reveal: (item: Element | null) => void }}
 */
export function attachTabScrollStrip(scroller) {
  const existing = scrollStrips.get(scroller);
  if (existing) return existing;

  const wrapper = scroller.closest(".tab-scroll-wrapper");
  const maxScrollStart = () => scroller.scrollWidth - scroller.clientWidth;

  const update = () => {
    if (!wrapper) return;
    const fromStart = scrollStart(scroller);
    wrapper.classList.toggle("can-scroll-start", fromStart > 1);
    wrapper.classList.toggle("can-scroll-end", fromStart < maxScrollStart() - 1);
  };

  scroller.addEventListener(
    "wheel",
    (event) => {
      if (maxScrollStart() <= 0 || Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return;
      const fromStart = scrollStart(scroller);
      const canMove = event.deltaY < 0 ? fromStart > 0 : fromStart < maxScrollStart();
      if (!canMove) return;
      event.preventDefault();
      scroller.scrollLeft += dirSign() * event.deltaY;
    },
    { passive: false }
  );
  scroller.addEventListener("scroll", update, { passive: true });

  // A page button moves the row by most of its visible width, so the tab cut at the
  // edge stays in view as the first one on the other side.
  wrapper?.querySelectorAll(".tab-scroll-btn").forEach((button) => {
    const toward = button.classList.contains("tab-scroll-btn--end") ? 1 : -1;
    button.addEventListener("click", () => {
      scroller.scrollBy({
        left: dirSign() * toward * scroller.clientWidth * 0.8,
        behavior: "smooth",
      });
    });
  });

  // The overflow changes when the row resizes, when its tabs are re-rendered and when
  // their width changes (web fonts landing, a nav rebuilt by the settings dialog).
  const resizeObserver = new ResizeObserver(update);
  const observeRow = () => {
    resizeObserver.disconnect();
    resizeObserver.observe(scroller);
    for (const child of scroller.children) resizeObserver.observe(child);
  };
  observeRow();
  new MutationObserver(() => {
    observeRow();
    update();
  }).observe(scroller, { childList: true });
  document.fonts?.ready.then(update).catch(() => {});
  requestAnimationFrame(update);

  const strip = {
    update,
    reveal(item) {
      item?.scrollIntoView({ block: "nearest", inline: "nearest" });
    },
  };
  scrollStrips.set(scroller, strip);
  return strip;
}
