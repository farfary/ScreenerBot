/**
 * Settings Dialog Component
 * Full-screen settings dialog with tabs for Interface, Startup, About, Updates
 */
import * as Utils from "../core/utils.js";
import { getCurrentPage } from "../core/router.js";
import { setInterval as setPollingInterval } from "../core/poller.js";

export class SettingsDialog {
  constructor(options = {}) {
    this.onClose = options.onClose || (() => {});
    this.dialogEl = null;
    this.currentTab = "interface";
    this.settings = null;
    this.originalSettings = null;
    this.hasChanges = false;
    this.isSaving = false;
    // Version info fetched from /api/version
    this.versionInfo = { version: "...", build_number: "..." };
  }

  /**
   * Show the settings dialog
   */
  async show() {
    if (this.dialogEl) {
      return;
    }

    this._createDialog();
    this._attachEventHandlers();
    await Promise.all([this._loadSettings(), this._loadVersionInfo()]);
    this._loadTabContent("interface");

    requestAnimationFrame(() => {
      if (this.dialogEl) {
        this.dialogEl.classList.add("active");
      }
    });
  }

  /**
   * Load version info from API
   */
  async _loadVersionInfo() {
    try {
      const response = await fetch("/api/version");
      if (response.ok) {
        const data = await response.json();
        this.versionInfo = {
          version: data.version || "0.0.0",
          build_number: data.build_number || "?",
        };
      }
    } catch (error) {
      console.error("Failed to load version info:", error);
    }
  }

  /**
   * Close dialog
   */
  close() {
    if (!this.dialogEl) return;

    this.dialogEl.classList.remove("active");

    setTimeout(() => {
      if (this._escapeHandler) {
        document.removeEventListener("keydown", this._escapeHandler);
        this._escapeHandler = null;
      }

      if (this.dialogEl) {
        this.dialogEl.remove();
        this.dialogEl = null;
      }

      this.settings = null;
      this.originalSettings = null;
      this.hasChanges = false;
      this.currentTab = "interface";

      this.onClose();
    }, 300);
  }

  /**
   * Load settings from API
   */
  async _loadSettings() {
    try {
      const response = await fetch("/api/config/gui");
      if (!response.ok) {
        throw new Error(`Failed to load settings: ${response.statusText}`);
      }
      const result = await response.json();
      // API returns { success: true, data: { data: GuiConfig, timestamp: ... } }
      this.settings = result.data?.data || result.data || result;
      this.originalSettings = JSON.parse(JSON.stringify(this.settings));
    } catch (error) {
      console.error("Failed to load settings:", error);
      this.settings = this._getDefaultSettings();
      this.originalSettings = JSON.parse(JSON.stringify(this.settings));
    }
  }

  /**
   * Get default settings structure
   */
  _getDefaultSettings() {
    return {
      zoom_level: 1.0,
      dashboard: {
        interface: {
          theme: "dark",
          polling_interval_ms: 5000,
          show_ticker_bar: true,
          enable_animations: true,
          compact_mode: false,
          auto_expand_categories: false,
          table_page_size: 25,
        },
        startup: {
          auto_start_trader: false,
          default_page: "dashboard",
          check_updates_on_startup: false,
          show_background_notifications: true,
        },
        navigation: {
          tabs: this._getDefaultTabs(),
        },
      },
    };
  }

  /**
   * Get default tab configuration
   */
  _getDefaultTabs() {
    return [
      { id: "home", label: "Home", icon: "icon-house", order: 0, enabled: true },
      { id: "positions", label: "Positions", icon: "icon-chart-candlestick", order: 1, enabled: true },
      { id: "tokens", label: "Tokens", icon: "icon-coins", order: 2, enabled: true },
      { id: "filtering", label: "Filtering", icon: "icon-list-filter", order: 3, enabled: true },
      { id: "wallet", label: "Wallet", icon: "icon-wallet", order: 4, enabled: true },
      { id: "trader", label: "Auto Trader", icon: "icon-bot", order: 5, enabled: true },
      { id: "strategies", label: "Strategies", icon: "icon-target", order: 6, enabled: true },
      { id: "transactions", label: "Transactions", icon: "icon-activity", order: 7, enabled: true },
      { id: "services", label: "Services", icon: "icon-server", order: 8, enabled: true },
      { id: "config", label: "Config", icon: "icon-settings", order: 9, enabled: true },
      { id: "events", label: "Events", icon: "icon-radio-tower", order: 10, enabled: true },
    ];
  }

  /**
   * Save settings to API
   */
  async _saveSettings() {
    if (this.isSaving) return;

    this.isSaving = true;
    this._updateSaveButton();

    try {
      const response = await fetch("/api/config/gui", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(this.settings),
      });

      if (!response.ok) {
        throw new Error(`Failed to save settings: ${response.statusText}`);
      }

      this.originalSettings = JSON.parse(JSON.stringify(this.settings));
      this.hasChanges = false;
      this._updateSaveButton();

      Utils.showToast({
        type: "success",
        title: "Settings saved successfully",
      });

      // Apply settings immediately
      this._applyInterfaceSettings();
      this._applyNavigationSettings();
    } catch (error) {
      console.error("Failed to save settings:", error);
      Utils.showToast({
        type: "error",
        title: "Failed to save settings",
        message: error.message,
      });
    } finally {
      this.isSaving = false;
      this._updateSaveButton();
    }
  }

  /**
   * Apply interface settings immediately
   */
  _applyInterfaceSettings() {
    const iface = this.settings?.dashboard?.interface;
    if (!iface) return;

    // Apply theme
    if (iface.theme) {
      document.documentElement.setAttribute("data-theme", iface.theme);
      localStorage.setItem("theme", iface.theme);
      const themeIcon = document.getElementById("themeIcon");
      if (themeIcon) {
        themeIcon.className = iface.theme === "dark" ? "icon-moon" : "icon-sun";
      }
    }

    // Apply animations
    if (typeof iface.enable_animations === "boolean") {
      document.documentElement.classList.toggle("no-animations", !iface.enable_animations);
    }

    // Apply compact mode
    if (typeof iface.compact_mode === "boolean") {
      document.documentElement.classList.toggle("compact-mode", iface.compact_mode);
    }

    // Apply polling/refresh interval
    if (iface.polling_interval_ms && iface.polling_interval_ms > 0) {
      setPollingInterval(iface.polling_interval_ms);
    }
  }

  /**
   * Apply navigation settings immediately (update nav bar without page reload)
   */
  _applyNavigationSettings() {
    const navContainer = document.getElementById("navTabs");
    if (!navContainer) return;

    const tabs = this.settings?.dashboard?.navigation?.tabs || [];
    const enabledTabs = tabs.filter((t) => t.enabled).sort((a, b) => a.order - b.order);

    // Get current active page from router
    const currentPage = getCurrentPage() || "home";

    // Rebuild navigation HTML
    const tabsHTML = enabledTabs
      .map((tab) => {
        const activeClass = tab.id === currentPage ? " active" : "";
        return `<a href="#" data-page="${tab.id}" class="tab${activeClass}"><i class="${tab.icon}"></i> ${tab.label}</a>`;
      })
      .join("\n        ");

    navContainer.innerHTML = tabsHTML;
  }

  /**
   * Create dialog DOM structure
   */
  _createDialog() {
    this.dialogEl = document.createElement("div");
    this.dialogEl.className = "settings-dialog";
    this.dialogEl.innerHTML = this._getDialogHTML();
    document.body.appendChild(this.dialogEl);
  }

  /**
   * Generate dialog HTML structure
   */
  _getDialogHTML() {
    return `
      <div class="settings-backdrop"></div>
      <div class="settings-container">
        <div class="settings-header">
          <div class="settings-header-left">
            <div class="settings-icon"><i class="icon-settings"></i></div>
            <div class="settings-title">
              <h2>Settings</h2>
              <span class="settings-subtitle">Configure dashboard preferences</span>
            </div>
          </div>
          <div class="settings-header-actions">
            <button class="settings-save-btn" id="settingsSaveBtn" disabled>
              <i class="icon-save"></i>
              <span>Save Changes</span>
            </button>
            <button class="settings-close-btn" title="Close (ESC)">
              <i class="icon-x"></i>
            </button>
          </div>
        </div>

        <div class="settings-body">
          <nav class="settings-nav">
            <button class="settings-nav-item active" data-tab="interface">
              <i class="icon-palette"></i>
              <span>Interface</span>
            </button>
            <button class="settings-nav-item" data-tab="navigation">
              <i class="icon-layout-grid"></i>
              <span>Navigation</span>
            </button>
            <button class="settings-nav-item" data-tab="startup">
              <i class="icon-zap"></i>
              <span>Startup</span>
            </button>
            <div class="settings-nav-divider"></div>
            <button class="settings-nav-item" data-tab="updates">
              <i class="icon-refresh-cw"></i>
              <span>Updates</span>
            </button>
            <button class="settings-nav-item" data-tab="licenses">
              <i class="icon-scale"></i>
              <span>Licenses</span>
            </button>
            <button class="settings-nav-item" data-tab="about">
              <i class="icon-info"></i>
              <span>About</span>
            </button>
            <div class="settings-nav-divider"></div>
            <a href="https://screenerbot.io/privacy" target="_blank" rel="noopener" class="settings-nav-item settings-nav-link">
              <i class="icon-shield"></i>
              <span>Privacy Policy</span>
              <i class="icon-external-link settings-nav-external"></i>
            </a>
            <a href="https://screenerbot.io/terms" target="_blank" rel="noopener" class="settings-nav-item settings-nav-link">
              <i class="icon-file-text"></i>
              <span>Terms of Service</span>
              <i class="icon-external-link settings-nav-external"></i>
            </a>
          </nav>

          <div class="settings-content">
            <div class="settings-tab active" data-tab-content="interface">
              <div class="settings-loading">Loading...</div>
            </div>
            <div class="settings-tab" data-tab-content="navigation">
              <div class="settings-loading">Loading...</div>
            </div>
            <div class="settings-tab" data-tab-content="startup">
              <div class="settings-loading">Loading...</div>
            </div>
            <div class="settings-tab" data-tab-content="updates">
              <div class="settings-loading">Loading...</div>
            </div>
            <div class="settings-tab" data-tab-content="licenses">
              <div class="settings-loading">Loading...</div>
            </div>
            <div class="settings-tab" data-tab-content="about">
              <div class="settings-loading">Loading...</div>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Attach event handlers
   */
  _attachEventHandlers() {
    // Close button
    const closeBtn = this.dialogEl.querySelector(".settings-close-btn");
    closeBtn.addEventListener("click", () => this.close());

    // Backdrop click
    const backdrop = this.dialogEl.querySelector(".settings-backdrop");
    backdrop.addEventListener("click", () => this.close());

    // ESC key
    this._escapeHandler = (e) => {
      if (e.key === "Escape") {
        this.close();
      }
    };
    document.addEventListener("keydown", this._escapeHandler);

    // Save button
    const saveBtn = this.dialogEl.querySelector("#settingsSaveBtn");
    saveBtn.addEventListener("click", () => this._saveSettings());

    // Tab navigation
    const tabButtons = this.dialogEl.querySelectorAll(".settings-nav-item");
    tabButtons.forEach((btn) => {
      btn.addEventListener("click", () => {
        const tab = btn.dataset.tab;
        if (tab && tab !== this.currentTab) {
          this._switchTab(tab);
        }
      });
    });
  }

  /**
   * Switch to a different tab
   */
  _switchTab(tab) {
    // Update nav buttons
    this.dialogEl.querySelectorAll(".settings-nav-item").forEach((btn) => {
      btn.classList.toggle("active", btn.dataset.tab === tab);
    });

    // Update tab content
    this.dialogEl.querySelectorAll(".settings-tab").forEach((content) => {
      content.classList.toggle("active", content.dataset.tabContent === tab);
    });

    this.currentTab = tab;
    this._loadTabContent(tab);
  }

  /**
   * Load content for a specific tab
   */
  _loadTabContent(tab) {
    const content = this.dialogEl.querySelector(`[data-tab-content="${tab}"]`);
    if (!content) return;

    switch (tab) {
      case "interface":
        content.innerHTML = this._buildInterfaceTab();
        this._attachInterfaceHandlers(content);
        break;
      case "navigation":
        content.innerHTML = this._buildNavigationTab();
        this._attachNavigationHandlers(content);
        break;
      case "startup":
        content.innerHTML = this._buildStartupTab();
        this._attachStartupHandlers(content);
        break;
      case "updates":
        content.innerHTML = this._buildUpdatesTab();
        this._attachUpdatesHandlers(content);
        break;
      case "licenses":
        content.innerHTML = this._buildLicensesTab();
        break;
      case "about":
        content.innerHTML = this._buildAboutTab();
        break;
    }
  }

  /**
   * Build Interface tab content
   */
  _buildInterfaceTab() {
    const iface = this.settings?.dashboard?.interface || {};

    return `
      <div class="settings-section">
        <h3 class="settings-section-title">Appearance</h3>
        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Theme</label>
              <span class="settings-field-hint">Choose your preferred color scheme</span>
            </div>
            <div class="settings-field-control">
              <select id="settingTheme" class="settings-select">
                <option value="dark" ${iface.theme === "dark" ? "selected" : ""}>Dark</option>
                <option value="light" ${iface.theme === "light" ? "selected" : ""}>Light</option>
              </select>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Enable Animations</label>
              <span class="settings-field-hint">Smooth transitions and effects</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="settingAnimations" ${iface.enable_animations !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Compact Mode</label>
              <span class="settings-field-hint">Reduce padding for more content</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="settingCompact" ${iface.compact_mode ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>
        </div>
      </div>

      <div class="settings-section">
        <h3 class="settings-section-title">Data & Display</h3>
        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Refresh Interval</label>
              <span class="settings-field-hint">How often to refresh data</span>
            </div>
            <div class="settings-field-control">
              <select id="settingPolling" class="settings-select">
                <option value="1000" ${iface.polling_interval_ms === 1000 ? "selected" : ""}>1 second</option>
                <option value="2000" ${iface.polling_interval_ms === 2000 ? "selected" : ""}>2 seconds</option>
                <option value="5000" ${iface.polling_interval_ms === 5000 || !iface.polling_interval_ms ? "selected" : ""}>5 seconds</option>
                <option value="10000" ${iface.polling_interval_ms === 10000 ? "selected" : ""}>10 seconds</option>
                <option value="30000" ${iface.polling_interval_ms === 30000 ? "selected" : ""}>30 seconds</option>
                <option value="60000" ${iface.polling_interval_ms === 60000 ? "selected" : ""}>1 minute</option>
              </select>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Show Ticker Bar</label>
              <span class="settings-field-hint">Live metrics ticker in header</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="settingTicker" ${iface.show_ticker_bar !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Table Page Size</label>
              <span class="settings-field-hint">Default rows per table page</span>
            </div>
            <div class="settings-field-control">
              <select id="settingPageSize" class="settings-select">
                <option value="10" ${iface.table_page_size === 10 ? "selected" : ""}>10 rows</option>
                <option value="25" ${iface.table_page_size === 25 || !iface.table_page_size ? "selected" : ""}>25 rows</option>
                <option value="50" ${iface.table_page_size === 50 ? "selected" : ""}>50 rows</option>
                <option value="100" ${iface.table_page_size === 100 ? "selected" : ""}>100 rows</option>
              </select>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Auto-expand Categories</label>
              <span class="settings-field-hint">Expand config categories by default</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="settingAutoExpand" ${iface.auto_expand_categories ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Attach handlers for Interface tab
   */
  _attachInterfaceHandlers(content) {
    const fields = {
      theme: content.querySelector("#settingTheme"),
      animations: content.querySelector("#settingAnimations"),
      compact: content.querySelector("#settingCompact"),
      polling: content.querySelector("#settingPolling"),
      ticker: content.querySelector("#settingTicker"),
      pageSize: content.querySelector("#settingPageSize"),
      autoExpand: content.querySelector("#settingAutoExpand"),
    };

    const updateSetting = (path, value) => {
      if (!this.settings.dashboard) this.settings.dashboard = {};
      if (!this.settings.dashboard.interface) this.settings.dashboard.interface = {};
      this.settings.dashboard.interface[path] = value;
      this._checkForChanges();
    };

    if (fields.theme) {
      fields.theme.addEventListener("change", (e) => updateSetting("theme", e.target.value));
    }
    if (fields.animations) {
      fields.animations.addEventListener("change", (e) =>
        updateSetting("enable_animations", e.target.checked)
      );
    }
    if (fields.compact) {
      fields.compact.addEventListener("change", (e) =>
        updateSetting("compact_mode", e.target.checked)
      );
    }
    if (fields.polling) {
      fields.polling.addEventListener("change", (e) =>
        updateSetting("polling_interval_ms", parseInt(e.target.value, 10))
      );
    }
    if (fields.ticker) {
      fields.ticker.addEventListener("change", (e) =>
        updateSetting("show_ticker_bar", e.target.checked)
      );
    }
    if (fields.pageSize) {
      fields.pageSize.addEventListener("change", (e) =>
        updateSetting("table_page_size", parseInt(e.target.value, 10))
      );
    }
    if (fields.autoExpand) {
      fields.autoExpand.addEventListener("change", (e) =>
        updateSetting("auto_expand_categories", e.target.checked)
      );
    }
  }

  /**
   * Build Navigation tab content
   */
  _buildNavigationTab() {
    const navigation = this.settings?.dashboard?.navigation || {};
    const tabs = navigation.tabs || this._getDefaultTabs();

    // Sort tabs by order for display
    const sortedTabs = [...tabs].sort((a, b) => a.order - b.order);

    const tabItems = sortedTabs
      .map(
        (tab, index) => `
        <div class="settings-nav-tab-item" data-tab-id="${tab.id}" data-order="${tab.order}">
          <div class="settings-nav-tab-handle">
            <i class="icon-grip-vertical"></i>
          </div>
          <div class="settings-nav-tab-icon">
            <i class="${tab.icon}"></i>
          </div>
          <div class="settings-nav-tab-info">
            <span class="settings-nav-tab-label">${tab.label}</span>
            <span class="settings-nav-tab-id">${tab.id}</span>
          </div>
          <div class="settings-nav-tab-actions">
            <button class="settings-nav-tab-btn settings-nav-tab-up" ${index === 0 ? "disabled" : ""} title="Move up">
              <i class="icon-chevron-up"></i>
            </button>
            <button class="settings-nav-tab-btn settings-nav-tab-down" ${index === sortedTabs.length - 1 ? "disabled" : ""} title="Move down">
              <i class="icon-chevron-down"></i>
            </button>
          </div>
          <div class="settings-nav-tab-toggle">
            <label class="settings-toggle">
              <input type="checkbox" ${tab.enabled ? "checked" : ""} ${tab.id === "home" ? "disabled" : ""}>
              <span class="settings-toggle-slider"></span>
            </label>
          </div>
        </div>
      `
      )
      .join("");

    return `
      <div class="settings-section">
        <h3 class="settings-section-title">Navigation Tabs</h3>
        <p class="settings-section-hint">Reorder and toggle visibility of navigation tabs. Home tab cannot be disabled.</p>
        <div class="settings-nav-tabs-list" id="navTabsList">
          ${tabItems}
        </div>
      </div>

      <div class="settings-section">
        <h3 class="settings-section-title">Actions</h3>
        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Reset to Defaults</label>
              <span class="settings-field-hint">Restore default tab order and visibility</span>
            </div>
            <div class="settings-field-control">
              <button class="settings-update-btn" id="resetNavTabs">
                <i class="icon-rotate-ccw"></i>
                <span>Reset</span>
              </button>
            </div>
          </div>
        </div>
      </div>

      <div class="settings-nav-tabs-note">
        <i class="icon-info"></i>
        <span>Changes require a page refresh to take effect in the navigation bar.</span>
      </div>
    `;
  }

  /**
   * Attach handlers for Navigation tab
   */
  _attachNavigationHandlers(content) {
    const list = content.querySelector("#navTabsList");
    if (!list) return;

    // Ensure navigation config exists
    if (!this.settings.dashboard) this.settings.dashboard = {};
    if (!this.settings.dashboard.navigation) {
      this.settings.dashboard.navigation = { tabs: this._getDefaultTabs() };
    }

    const getTabs = () => this.settings.dashboard.navigation.tabs;
    const setTabs = (tabs) => {
      this.settings.dashboard.navigation.tabs = tabs;
      this._checkForChanges();
    };

    // Move up handler
    list.addEventListener("click", (e) => {
      const upBtn = e.target.closest(".settings-nav-tab-up");
      if (upBtn && !upBtn.disabled) {
        const item = upBtn.closest(".settings-nav-tab-item");
        const tabId = item.dataset.tabId;
        this._moveTab(tabId, -1);
        this._refreshNavigationList(content);
      }
    });

    // Move down handler
    list.addEventListener("click", (e) => {
      const downBtn = e.target.closest(".settings-nav-tab-down");
      if (downBtn && !downBtn.disabled) {
        const item = downBtn.closest(".settings-nav-tab-item");
        const tabId = item.dataset.tabId;
        this._moveTab(tabId, 1);
        this._refreshNavigationList(content);
      }
    });

    // Toggle handler
    list.addEventListener("change", (e) => {
      if (e.target.type === "checkbox") {
        const item = e.target.closest(".settings-nav-tab-item");
        const tabId = item.dataset.tabId;
        if (tabId !== "home") {
          const tabs = getTabs();
          const tab = tabs.find((t) => t.id === tabId);
          if (tab) {
            tab.enabled = e.target.checked;
            setTabs(tabs);
          }
        }
      }
    });

    // Reset button handler
    const resetBtn = content.querySelector("#resetNavTabs");
    if (resetBtn) {
      resetBtn.addEventListener("click", () => {
        this.settings.dashboard.navigation.tabs = this._getDefaultTabs();
        this._checkForChanges();
        this._refreshNavigationList(content);
        Utils.showToast({
          type: "info",
          title: "Navigation reset to defaults",
        });
      });
    }
  }

  /**
   * Move a tab up or down in the order
   */
  _moveTab(tabId, direction) {
    const tabs = this.settings.dashboard.navigation.tabs;
    const sortedTabs = [...tabs].sort((a, b) => a.order - b.order);
    const currentIndex = sortedTabs.findIndex((t) => t.id === tabId);

    if (currentIndex === -1) return;

    const newIndex = currentIndex + direction;
    if (newIndex < 0 || newIndex >= sortedTabs.length) return;

    // Swap orders
    const currentTab = sortedTabs[currentIndex];
    const swapTab = sortedTabs[newIndex];

    const tempOrder = currentTab.order;
    currentTab.order = swapTab.order;
    swapTab.order = tempOrder;

    this._checkForChanges();
  }

  /**
   * Refresh the navigation list after reordering
   */
  _refreshNavigationList(content) {
    const listContainer = content.querySelector("#navTabsList");
    if (!listContainer) return;

    const tabs = this.settings?.dashboard?.navigation?.tabs || this._getDefaultTabs();
    const sortedTabs = [...tabs].sort((a, b) => a.order - b.order);

    const tabItems = sortedTabs
      .map(
        (tab, index) => `
        <div class="settings-nav-tab-item" data-tab-id="${tab.id}" data-order="${tab.order}">
          <div class="settings-nav-tab-handle">
            <i class="icon-grip-vertical"></i>
          </div>
          <div class="settings-nav-tab-icon">
            <i class="${tab.icon}"></i>
          </div>
          <div class="settings-nav-tab-info">
            <span class="settings-nav-tab-label">${tab.label}</span>
            <span class="settings-nav-tab-id">${tab.id}</span>
          </div>
          <div class="settings-nav-tab-actions">
            <button class="settings-nav-tab-btn settings-nav-tab-up" ${index === 0 ? "disabled" : ""} title="Move up">
              <i class="icon-chevron-up"></i>
            </button>
            <button class="settings-nav-tab-btn settings-nav-tab-down" ${index === sortedTabs.length - 1 ? "disabled" : ""} title="Move down">
              <i class="icon-chevron-down"></i>
            </button>
          </div>
          <div class="settings-nav-tab-toggle">
            <label class="settings-toggle">
              <input type="checkbox" ${tab.enabled ? "checked" : ""} ${tab.id === "home" ? "disabled" : ""}>
              <span class="settings-toggle-slider"></span>
            </label>
          </div>
        </div>
      `
      )
      .join("");

    listContainer.innerHTML = tabItems;
  }

  /**
   * Build Startup tab content
   */
  _buildStartupTab() {
    const startup = this.settings?.dashboard?.startup || {};

    return `
      <div class="settings-section">
        <h3 class="settings-section-title">Startup Behavior</h3>
        <div class="settings-group">
          <div class="settings-field settings-field--disabled">
            <div class="settings-field-info">
              <label>Auto-start Trader</label>
              <span class="settings-field-hint">Automatically start trader on launch</span>
              <span class="settings-field-badge">Coming Soon</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="settingAutoStart" ${startup.auto_start_trader ? "checked" : ""} disabled>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Default Page</label>
              <span class="settings-field-hint">Page to show when opening the app</span>
            </div>
            <div class="settings-field-control">
              <select id="settingDefaultPage" class="settings-select">
                <option value="dashboard" ${startup.default_page === "dashboard" || !startup.default_page ? "selected" : ""}>Dashboard</option>
                <option value="tokens" ${startup.default_page === "tokens" ? "selected" : ""}>Tokens</option>
                <option value="positions" ${startup.default_page === "positions" ? "selected" : ""}>Positions</option>
                <option value="wallet" ${startup.default_page === "wallet" ? "selected" : ""}>Wallet</option>
                <option value="config" ${startup.default_page === "config" ? "selected" : ""}>Config</option>
              </select>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Show Background Notifications</label>
              <span class="settings-field-hint">Display notifications for background events</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="settingBgNotifications" ${startup.show_background_notifications !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>
        </div>
      </div>

      <div class="settings-section">
        <h3 class="settings-section-title">Update Checks</h3>
        <div class="settings-group">
          <div class="settings-field settings-field--disabled">
            <div class="settings-field-info">
              <label>Check for Updates on Startup</label>
              <span class="settings-field-hint">Automatically check for new versions</span>
              <span class="settings-field-badge">Coming Soon</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="settingCheckUpdates" ${startup.check_updates_on_startup ? "checked" : ""} disabled>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Attach handlers for Startup tab
   */
  _attachStartupHandlers(content) {
    const fields = {
      defaultPage: content.querySelector("#settingDefaultPage"),
      bgNotifications: content.querySelector("#settingBgNotifications"),
    };

    const updateSetting = (path, value) => {
      if (!this.settings.dashboard) this.settings.dashboard = {};
      if (!this.settings.dashboard.startup) this.settings.dashboard.startup = {};
      this.settings.dashboard.startup[path] = value;
      this._checkForChanges();
    };

    if (fields.defaultPage) {
      fields.defaultPage.addEventListener("change", (e) =>
        updateSetting("default_page", e.target.value)
      );
    }
    if (fields.bgNotifications) {
      fields.bgNotifications.addEventListener("change", (e) =>
        updateSetting("show_background_notifications", e.target.checked)
      );
    }
  }

  /**
   * Build Updates tab content
   */
  _buildUpdatesTab() {
    const { version, build_number } = this.versionInfo;
    return `
      <div class="settings-updates-section">
        <div class="settings-section">
          <h3 class="settings-section-title">Software Updates</h3>
          <div class="settings-update-card" id="updateStatusCard">
            <div class="settings-update-icon" id="updateStatusIcon">
              <i class="icon-circle-check"></i>
            </div>
            <div class="settings-update-info" id="updateStatusInfo">
              <h4 id="updateStatusTitle">You're up to date!</h4>
              <p id="updateStatusMessage">ScreenerBot v${version} (build ${build_number}) is the latest version.</p>
            </div>
            <button class="settings-update-btn" id="checkUpdatesBtn">
              <i class="icon-refresh-cw"></i>
              <span>Check for Updates</span>
            </button>
          </div>
          <div class="settings-update-card" id="updateAvailableCard" style="display: none; border-color: var(--warning-color);">
            <div class="settings-update-icon" style="color: var(--warning-color);">
              <i class="icon-arrow-up-circle"></i>
            </div>
            <div class="settings-update-info">
              <h4 style="color: var(--warning-color);">Update Available!</h4>
              <p id="updateAvailableMessage">A new version is available.</p>
            </div>
            <a href="/updates" class="settings-update-btn" style="text-decoration: none;">
              <i class="icon-download"></i>
              <span>View Update</span>
            </a>
          </div>
        </div>

        <div class="settings-section">
          <h3 class="settings-section-title">Current Release</h3>
          <div class="settings-release-notes">
            <div class="settings-release">
              <div class="settings-release-header">
                <span class="settings-release-version">v${version}</span>
                <span class="settings-release-date">Build ${build_number}</span>
                <span class="settings-release-badge">Current</span>
              </div>
              <p id="currentReleaseNotes" style="color: var(--text-secondary); font-size: var(--font-size-sm); margin-top: var(--spacing-sm);">
                Visit the Updates page for full release notes and download options.
              </p>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Attach handlers for Updates tab
   */
  _attachUpdatesHandlers(content) {
    const checkBtn = content.querySelector("#checkUpdatesBtn");
    if (checkBtn) {
      checkBtn.addEventListener("click", async () => {
        // Update button state
        checkBtn.disabled = true;
        checkBtn.innerHTML = '<i class="icon-loader spinning"></i><span>Checking...</span>';

        try {
          const response = await fetch("/api/updates/check");
          const data = await response.json();

          const statusCard = content.querySelector("#updateStatusCard");
          const availableCard = content.querySelector("#updateAvailableCard");
          const statusIcon = content.querySelector("#updateStatusIcon");
          const statusTitle = content.querySelector("#updateStatusTitle");
          const statusMessage = content.querySelector("#updateStatusMessage");

          if (data.update_available && data.update) {
            // Update available
            statusCard.style.display = "none";
            availableCard.style.display = "flex";
            
            const availableMsg = content.querySelector("#updateAvailableMessage");
            if (availableMsg) {
              availableMsg.textContent = `Version ${data.update.version} is now available!`;
            }

            Utils.showToast({
              type: "warning",
              title: "Update Available",
              message: `Version ${data.update.version} is ready to download`,
            });
          } else {
            // No update
            statusCard.style.display = "flex";
            availableCard.style.display = "none";
            statusIcon.innerHTML = '<i class="icon-circle-check"></i>';
            statusTitle.textContent = "You're up to date!";
            statusMessage.textContent = `ScreenerBot v${this.versionInfo.version} (build ${this.versionInfo.build_number}) is the latest version.`;

            Utils.showToast({
              type: "success",
              title: "No updates available",
              message: "You're running the latest version",
            });
          }
        } catch (err) {
          console.error("Update check failed:", err);
          Utils.showToast({
            type: "error",
            title: "Update check failed",
            message: err.message || "Could not connect to update server",
          });
        } finally {
          checkBtn.disabled = false;
          checkBtn.innerHTML = '<i class="icon-refresh-cw"></i><span>Check for Updates</span>';
        }
      });
    }
  }

  /**
   * Build About tab content
   */
  _buildAboutTab() {
    const { version, build_number } = this.versionInfo;
    return `
      <div class="settings-about">
        <div class="settings-about-logo">
          <img src="/assets/logo.svg" alt="ScreenerBot" />
        </div>
        <h2 class="settings-about-name">ScreenerBot</h2>
        <p class="settings-about-tagline">Advanced Solana Trading Automation</p>
        <div class="settings-about-version">
          <span>v${version}</span>
          <span class="settings-about-separator">•</span>
          <span>Build ${build_number}</span>
        </div>

        <div class="settings-about-links">
          <a href="https://github.com/farfary/ScreenerBot" target="_blank" rel="noopener" class="settings-about-link">
            <i class="icon-github"></i>
            <span>GitHub</span>
          </a>
          <a href="https://docs.screenerbot.app" target="_blank" rel="noopener" class="settings-about-link">
            <i class="icon-book-open"></i>
            <span>Documentation</span>
          </a>
          <a href="https://discord.gg/screenerbot" target="_blank" rel="noopener" class="settings-about-link">
            <i class="icon-message-circle"></i>
            <span>Discord</span>
          </a>
        </div>

        <div class="settings-about-credits">
          <p>Built with <i class="icon-heart" style="color: #ef4444;"></i> for Solana traders</p>
          <p class="settings-about-copyright">© 2025 ScreenerBot. All rights reserved.</p>
        </div>
      </div>
    `;
  }

  /**
   * Build Licenses tab content
   */
  _buildLicensesTab() {
    const licenses = [
      {
        category: "Application Framework",
        items: [
          { name: "Tauri", license: "MIT / Apache-2.0", url: "https://tauri.app/", desc: "Desktop application framework" },
          { name: "Tokio", license: "MIT", url: "https://tokio.rs/", desc: "Async runtime for Rust" },
          { name: "Axum", license: "MIT", url: "https://github.com/tokio-rs/axum", desc: "Web server framework" },
          { name: "Tower", license: "MIT", url: "https://github.com/tower-rs/tower", desc: "Service abstractions" },
          { name: "Hyper", license: "MIT", url: "https://hyper.rs/", desc: "HTTP implementation" },
        ],
      },
      {
        category: "Solana Blockchain",
        items: [
          { name: "solana-sdk", license: "Apache-2.0", url: "https://github.com/anza-xyz/agave", desc: "Solana SDK core" },
          { name: "solana-client", license: "Apache-2.0", url: "https://github.com/anza-xyz/agave", desc: "RPC client" },
          { name: "solana-program", license: "Apache-2.0", url: "https://github.com/anza-xyz/agave", desc: "Program library" },
          { name: "spl-token", license: "Apache-2.0", url: "https://github.com/solana-labs/solana-program-library", desc: "SPL Token program" },
          { name: "spl-token-2022", license: "Apache-2.0", url: "https://github.com/solana-labs/solana-program-library", desc: "Token-2022 extensions" },
          { name: "spl-associated-token-account", license: "Apache-2.0", url: "https://github.com/solana-labs/solana-program-library", desc: "Associated token accounts" },
        ],
      },
      {
        category: "Data & Storage",
        items: [
          { name: "SQLite", license: "Public Domain", url: "https://sqlite.org/", desc: "Embedded database engine" },
          { name: "rusqlite", license: "MIT", url: "https://github.com/rusqlite/rusqlite", desc: "SQLite Rust bindings" },
          { name: "r2d2", license: "MIT / Apache-2.0", url: "https://github.com/sfackler/r2d2", desc: "Database connection pool" },
          { name: "Serde", license: "MIT / Apache-2.0", url: "https://serde.rs/", desc: "Serialization framework" },
          { name: "TOML", license: "MIT / Apache-2.0", url: "https://github.com/toml-rs/toml", desc: "Configuration parsing" },
        ],
      },
      {
        category: "Networking",
        items: [
          { name: "reqwest", license: "MIT / Apache-2.0", url: "https://github.com/seanmonstar/reqwest", desc: "HTTP client" },
          { name: "tokio-tungstenite", license: "MIT", url: "https://github.com/snapview/tokio-tungstenite", desc: "WebSocket client" },
          { name: "RustLS", license: "MIT / Apache-2.0", url: "https://github.com/rustls/rustls", desc: "TLS implementation" },
        ],
      },
      {
        category: "Cryptography & Encoding",
        items: [
          { name: "BLAKE3", license: "CC0 / Apache-2.0", url: "https://github.com/BLAKE3-team/BLAKE3", desc: "Hash function" },
          { name: "SHA-2", license: "MIT / Apache-2.0", url: "https://github.com/RustCrypto/hashes", desc: "SHA-256/512 hashing" },
          { name: "bs58", license: "MIT / Apache-2.0", url: "https://github.com/Nullus157/bs58-rs", desc: "Base58 encoding" },
          { name: "base64", license: "MIT / Apache-2.0", url: "https://github.com/marshallpierce/rust-base64", desc: "Base64 encoding" },
        ],
      },
      {
        category: "UI Assets",
        items: [
          { name: "Lucide Icons", license: "ISC", url: "https://lucide.dev/", desc: "Icon font library" },
          { name: "JetBrains Mono", license: "OFL-1.1", url: "https://www.jetbrains.com/lp/mono/", desc: "Monospace font" },
          { name: "Orbitron", license: "OFL-1.1", url: "https://fonts.google.com/specimen/Orbitron", desc: "Display font" },
        ],
      },
    ];

    const categoriesHtml = licenses
      .map(
        (cat) => `
        <div class="license-category">
          <h4 class="license-category-title">${Utils.escapeHtml(cat.category)}</h4>
          <div class="license-items">
            ${cat.items
              .map(
                (item) => `
              <div class="license-item">
                <div class="license-item-header">
                  <a href="${Utils.escapeHtml(item.url)}" target="_blank" rel="noopener" class="license-item-name">
                    ${Utils.escapeHtml(item.name)}
                    <i class="icon-external-link"></i>
                  </a>
                  <span class="license-item-badge">${Utils.escapeHtml(item.license)}</span>
                </div>
                <p class="license-item-desc">${Utils.escapeHtml(item.desc)}</p>
              </div>
            `
              )
              .join("")}
          </div>
        </div>
      `
      )
      .join("");

    return `
      <div class="settings-licenses">
        <div class="licenses-header">
          <i class="icon-scale"></i>
          <div>
            <h3>Open Source Licenses</h3>
            <p>ScreenerBot is built with the following open source software</p>
          </div>
        </div>
        <div class="licenses-content">
          ${categoriesHtml}
        </div>
        <div class="licenses-footer">
          <p>
            <i class="icon-info"></i>
            Full license texts are available in the project repository and within each dependency's source code.
          </p>
        </div>
      </div>
    `;
  }

  /**
   * Check if settings have changed from original
   */
  _checkForChanges() {
    const current = JSON.stringify(this.settings);
    const original = JSON.stringify(this.originalSettings);
    this.hasChanges = current !== original;
    this._updateSaveButton();
  }

  /**
   * Update save button state
   */
  _updateSaveButton() {
    const saveBtn = this.dialogEl?.querySelector("#settingsSaveBtn");
    if (!saveBtn) return;

    saveBtn.disabled = !this.hasChanges || this.isSaving;

    const icon = saveBtn.querySelector("i");
    const text = saveBtn.querySelector("span");

    if (this.isSaving) {
      icon.className = "icon-loader";
      text.textContent = "Saving...";
    } else {
      icon.className = "icon-save";
      text.textContent = this.hasChanges ? "Save Changes" : "Saved";
    }
  }
}

// Singleton instance for easy access
let settingsDialogInstance = null;

export function showSettingsDialog() {
  if (!settingsDialogInstance) {
    settingsDialogInstance = new SettingsDialog({
      onClose: () => {
        settingsDialogInstance = null;
      },
    });
  }
  settingsDialogInstance.show();
}

export function closeSettingsDialog() {
  if (settingsDialogInstance) {
    settingsDialogInstance.close();
  }
}
