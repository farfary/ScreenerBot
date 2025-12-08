/**
 * Settings Dialog Component
 * Full-screen settings dialog with tabs for Interface, Startup, About, Updates
 */
import * as Utils from "../core/utils.js";
import { getCurrentPage } from "../core/router.js";
import { setInterval as setPollingInterval, Poller } from "../core/poller.js";

// Global update state to persist across dialog opens
let globalUpdateState = {
  checked: false,
  checking: false,
  available: false,
  info: null, // { version, release_notes, download_url, ... }
  downloading: false,
  progress: 0,
  downloaded: false,
  error: null,
  statusPoller: null
};

export class SettingsDialog {
  constructor(options = {}) {
    this.onClose = options.onClose || (() => {});
    this.dialogEl = null;
    this.currentTab = "interface";
    this.settings = null;
    this.originalSettings = null;
    this.hasChanges = false;
    this.isSaving = false;
    this.pathsInfo = null;
    // Version info fetched from /api/version
    this.versionInfo = { version: "...", build_number: "...", platform: "..." };
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
    await Promise.all([this._loadSettings(), this._loadVersionInfo(), this._loadPathsInfo()]);
    this._loadTabContent("interface");

    // Sync update status with server (handles refreshes and background downloads)
    this._syncUpdateStatus();

    requestAnimationFrame(() => {
      if (this.dialogEl) {
        this.dialogEl.classList.add("active");
      }
    });
  }

  /**
   * Sync update status with server
   */
  async _syncUpdateStatus() {
    // If we already know we are downloading, just resume polling
    if (globalUpdateState.downloading) {
      this._startDownloadPoller();
      return;
    }

    // Otherwise check status from server
    try {
      const response = await fetch('/api/updates/status');
      if (response.ok) {
        const data = await response.json();
        // API returns { state: { available_update, download_progress, ... } }
        const state = data.state || data;
        const progress = state.download_progress || {};
        
        if (progress.downloading) {
          globalUpdateState.downloading = true;
          globalUpdateState.progress = progress.progress_percent || 0;
          globalUpdateState.available = true;
          if (!globalUpdateState.info && state.available_update) {
            globalUpdateState.info = state.available_update;
          }
          this._startDownloadPoller();
        } else if (progress.completed && progress.downloaded_path) {
          globalUpdateState.downloading = false;
          globalUpdateState.downloaded = true;
          globalUpdateState.progress = 100;
          globalUpdateState.available = true;
          if (!globalUpdateState.info && state.available_update) {
            globalUpdateState.info = state.available_update;
          }
        } else if (state.available_update) {
          // Update available but not downloading yet
          globalUpdateState.available = true;
          globalUpdateState.info = state.available_update;
        }
      }
    } catch (err) {
      console.warn('Failed to sync update status:', err);
    }

    // If not downloading/ready, maybe check for updates if not checked yet
    if (!globalUpdateState.checked && !globalUpdateState.checking && !globalUpdateState.downloading && !globalUpdateState.downloaded) {
      this._performBackgroundUpdateCheck();
    } else {
      this._updateUpdatesBadge();
      if (this.currentTab === 'updates') {
        this._updateUpdatesTabUI();
      }
    }
  }

  /**
   * Perform background update check
   */
  async _performBackgroundUpdateCheck() {
    globalUpdateState.checking = true;
    globalUpdateState.checked = true;
    this._updateUpdatesBadge(); // Might show spinner or nothing

    try {
      const response = await fetch('/api/updates/check');
      const data = await response.json();
      
      globalUpdateState.checking = false;
      
      if (data.update_available) {
        globalUpdateState.available = true;
        globalUpdateState.info = data.update;  // API returns 'update' not 'update_info'
      } else {
        globalUpdateState.available = false;
      }
    } catch (err) {
      console.error('Background update check failed:', err);
      globalUpdateState.checking = false;
      globalUpdateState.error = err.message;
    }
    
    this._updateUpdatesBadge();
    
    // If user is currently on updates tab, refresh it
    if (this.currentTab === 'updates') {
      this._updateUpdatesTabUI();
    }
  }

  /**
   * Update the badge on the Updates tab button
   */
  _updateUpdatesBadge() {
    if (!this.dialogEl) return;
    
    const updatesBtn = this.dialogEl.querySelector('.settings-nav-item[data-tab="updates"]');
    if (!updatesBtn) return;
    
    // Remove existing indicator
    const existingIndicator = updatesBtn.querySelector('.settings-nav-indicator');
    if (existingIndicator) existingIndicator.remove();
    
    if (globalUpdateState.available) {
      const indicator = document.createElement('span');
      indicator.className = 'settings-nav-indicator';
      indicator.title = 'New update available';
      updatesBtn.appendChild(indicator);
    }
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
          platform: data.platform || "Unknown",
        };
      }
    } catch (error) {
      console.error("Failed to load version info:", error);
    }
  }

  /**
   * Load filesystem paths from API
   */
  async _loadPathsInfo() {
    try {
      const response = await fetch("/api/system/paths");
      if (response.ok) {
        this.pathsInfo = await response.json();
      } else {
        this.pathsInfo = null;
      }
    } catch (error) {
      console.error("Failed to load paths info:", error);
      this.pathsInfo = null;
    }
  }

  /**
   * Close dialog
   */
  close() {
    if (!this.dialogEl) return;

    // Stop any active pollers
    if (this.downloadPoller) {
      this.downloadPoller.stop();
      this.downloadPoller = null;
    }

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
            <button class="settings-nav-item settings-nav-link" data-external-url="https://screenerbot.io/privacy">
              <i class="icon-shield"></i>
              <span>Privacy Policy</span>
              <i class="icon-external-link settings-nav-external"></i>
            </button>
            <button class="settings-nav-item settings-nav-link" data-external-url="https://screenerbot.io/terms">
              <i class="icon-file-text"></i>
              <span>Terms of Service</span>
              <i class="icon-external-link settings-nav-external"></i>
            </button>
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
      <style>
        .settings-nav-indicator {
          width: 8px;
          height: 8px;
          border-radius: 50%;
          background: var(--primary-color);
          margin-left: auto;
          position: relative;
          animation: pulse-indicator 2s ease-in-out infinite;
        }
        
        .settings-nav-indicator::before {
          content: '';
          position: absolute;
          inset: -4px;
          border-radius: 50%;
          border: 2px solid var(--primary-color);
          opacity: 0;
          animation: pulse-ring-indicator 2s ease-in-out infinite;
        }
        
        @keyframes pulse-indicator {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.6; }
        }
        
        @keyframes pulse-ring-indicator {
          0% { transform: scale(0.8); opacity: 0.8; }
          50% { transform: scale(1.2); opacity: 0.4; }
          100% { transform: scale(1.4); opacity: 0; }
        }
      </style>
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

    // Tab navigation (exclude external links)
    const tabButtons = this.dialogEl.querySelectorAll(".settings-nav-item:not(.settings-nav-link)");
    tabButtons.forEach((btn) => {
      btn.addEventListener("click", () => {
        const tab = btn.dataset.tab;
        if (tab && tab !== this.currentTab) {
          this._switchTab(tab);
        }
      });
    });

    // External links (Privacy Policy, Terms of Service)
    const externalLinks = this.dialogEl.querySelectorAll(".settings-nav-link[data-external-url]");
    externalLinks.forEach((btn) => {
      btn.addEventListener("click", () => {
        const url = btn.dataset.externalUrl;
        if (url) {
          Utils.openExternal(url);
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
        this._attachLicensesHandlers(content);
        break;
      case "about":
        content.innerHTML = this._buildAboutTab();
        this._attachAboutHandlers(content);
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
   * Build Updates tab content - Modern design with version history
   */
  _buildUpdatesTab() {
    const { version, platform } = this.versionInfo;
    const state = globalUpdateState;

    // Build status section based on current state
    let statusSection = '';
    
    if (state.checking) {
      statusSection = this._buildCheckingState();
    } else if (state.error) {
      statusSection = this._buildErrorState(state.error);
    } else if (state.available && state.info) {
      statusSection = this._buildUpdateAvailableState(state, version);
    } else {
      statusSection = this._buildUpToDateState(version);
    }

    return `
      <div class="updates-container">
        <!-- Main Content -->
        <div class="updates-main">
          <!-- Status Card -->
          ${statusSection}
        </div>
        
        <!-- Sidebar -->
        <div class="updates-sidebar">
          <!-- Current Version Card -->
          <div class="updates-version-card">
            <div class="version-card-header">
              <div class="version-icon">
                <i class="icon-box"></i>
              </div>
              <div class="version-info">
                <h4>Current Installation</h4>
                <span class="version-number">v${version}</span>
              </div>
            </div>
            <div class="version-details">
              <div class="detail-row">
                <span class="detail-label">Platform</span>
                <span class="detail-value">${platform || 'Unknown'}</span>
              </div>
            </div>
          </div>

          <!-- System Info Section -->
          <div class="updates-system-section">
            <div class="system-header">
              <div class="system-title">
                <i class="icon-info"></i>
                <span>Installation Details</span>
              </div>
            </div>
            <div class="system-details">
              <div class="detail-row">
                <span class="detail-label">Version</span>
                <span class="detail-value">v${version}</span>
              </div>
              <div class="detail-row">
                <span class="detail-label">Platform</span>
                <span class="detail-value">${platform || 'Unknown'}</span>
              </div>
              <div class="detail-row">
                <span class="detail-label">Auto Update</span>
                <span class="detail-value channel-badge">Enabled</span>
              </div>
            </div>
          </div>
        </div>
      </div>
      ${this._getUpdatesStyles()}
    `;
  }

  /**
   * Build checking for updates state
   */
  _buildCheckingState() {
    return `
      <div class="updates-status-card checking">
        <div class="status-visual">
          <div class="pulse-ring"></div>
          <div class="status-icon-wrapper">
            <i class="icon-refresh-cw spinning"></i>
          </div>
        </div>
        <div class="status-content">
          <h3>Checking for Updates</h3>
          <p>Connecting to update server...</p>
        </div>
      </div>
    `;
  }

  /**
   * Build error state
   */
  _buildErrorState(error) {
    return `
      <div class="updates-status-card error">
        <div class="status-visual">
          <div class="status-icon-wrapper error">
            <i class="icon-alert-triangle"></i>
          </div>
        </div>
        <div class="status-content">
          <h3>Update Check Failed</h3>
          <p class="error-message">${Utils.escapeHtml(error)}</p>
        </div>
        <div class="status-actions">
          <button class="updates-btn secondary" id="retryUpdateBtn">
            <i class="icon-refresh-cw"></i>
            <span>Try Again</span>
          </button>
        </div>
      </div>
    `;
  }

  /**
   * Build update available state
   */
  _buildUpdateAvailableState(state, currentVersion) {
    const info = state.info;
    const isDownloading = state.downloading;
    const isDownloaded = state.downloaded;
    const fileSize = info.file_size ? this._formatBytes(info.file_size) : null;

    let actionContent = '';
    
    if (isDownloaded) {
      actionContent = `
        <div class="download-success">
          <div class="success-badge">
            <i class="icon-check-circle"></i>
            <span>Ready to Install</span>
          </div>
          <p class="install-hint">${this._getInstallHint()}</p>
        </div>
        <div class="status-actions">
          <button class="updates-btn success" id="installUpdateBtn">
            <i class="icon-download"></i>
            <span>Install & Restart</span>
          </button>
        </div>
      `;
    } else if (isDownloading) {
      actionContent = `
        <div class="download-progress">
          <div class="progress-header">
            <span class="progress-status" id="downloadStatusText">Downloading update...</span>
            <span class="progress-stats">
              <span id="downloadSpeedText"></span>
              <span id="downloadPercentText">${Math.round(state.progress)}%</span>
            </span>
          </div>
          <div class="progress-track">
            <div class="progress-fill" id="downloadProgressBar" style="width: ${state.progress}%">
              <div class="progress-glow"></div>
            </div>
          </div>
          <div class="progress-footer">
            <span id="downloadSizeText">${fileSize ? `0 / ${fileSize}` : ''}</span>
            <span id="downloadEtaText"></span>
          </div>
        </div>
      `;
    } else {
      actionContent = `
        <div class="status-actions">
          <button class="updates-btn primary" id="downloadUpdateBtn">
            <i class="icon-download"></i>
            <span>Download Update</span>
            ${fileSize ? `<span class="btn-meta">(${fileSize})</span>` : ''}
          </button>
        </div>
      `;
    }

    return `
      <div class="updates-status-card available">
        <div class="update-badge">New Version Available</div>
        <div class="status-visual">
          <div class="version-transition">
            <span class="old-version">v${currentVersion}</span>
            <i class="icon-arrow-right"></i>
            <span class="new-version">v${info.version}</span>
          </div>
        </div>
        <div class="status-content">
          ${info.release_notes ? `
            <div class="release-notes-preview">
              <h4>What's New</h4>
              <div class="notes-text">${Utils.escapeHtml(info.release_notes)}</div>
            </div>
          ` : ''}
        </div>
        ${actionContent}
      </div>
    `;
  }

  /**
   * Build up to date state
   */
  _buildUpToDateState(version) {
    return `
      <div class="updates-status-card success">
        <div class="status-visual">
          <div class="status-icon-wrapper success">
            <i class="icon-check-circle"></i>
          </div>
        </div>
        <div class="status-content">
          <h3>You're Up to Date</h3>
          <p>ScreenerBot v${version} is the latest version.</p>
        </div>
        <div class="status-actions">
          <button class="updates-btn secondary" id="checkUpdatesBtn">
            <i class="icon-refresh-cw"></i>
            <span>Check Again</span>
          </button>
        </div>
      </div>
    `;
  }

  /**
   * Get platform-specific install hint
   */
  _getInstallHint() {
    const platform = this.versionInfo.platform || '';
    if (platform.toLowerCase().includes('macos') || platform.toLowerCase().includes('darwin')) {
      return 'The installer will open. Drag ScreenerBot to your Applications folder.';
    } else if (platform.toLowerCase().includes('windows')) {
      return 'The installer will guide you through the update process.';
    } else if (platform.toLowerCase().includes('linux')) {
      return 'Run the AppImage or install the .deb package to update.';
    }
    return 'Follow the installer instructions to complete the update.';
  }

  /**
   * Format bytes to human readable size
   */
  _formatBytes(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  /**
   * Get styles for Updates tab
   */
  _getUpdatesStyles() {
    return `
      <style>
        .updates-container {
          display: grid;
          grid-template-columns: 1fr 420px;
          gap: var(--spacing-lg);
          height: 100%;
          width: 100%;
        }

        @media (max-width: 900px) {
          .updates-container {
            grid-template-columns: 1fr;
          }
        }

        /* Main Content Area */
        .updates-main {
          display: flex;
          flex-direction: column;
          gap: var(--spacing-md);
          min-height: 0;
        }

        /* Sidebar */
        .updates-sidebar {
          display: flex;
          flex-direction: column;
          gap: var(--spacing-md);
        }

        @media (max-width: 900px) {
          .updates-sidebar {
            order: -1;
          }
        }

        /* Status Cards */
        .updates-status-card {
          background: var(--bg-secondary);
          border: 1px solid var(--border-color);
          border-radius: var(--radius-lg);
          padding: var(--spacing-xl);
          display: flex;
          flex-direction: column;
          align-items: center;
          text-align: center;
          gap: var(--spacing-lg);
          position: relative;
          overflow: hidden;
          flex: 1;
          min-height: 320px;
          justify-content: center;
        }

        .updates-status-card.available {
          border-color: var(--primary-color);
          background: linear-gradient(135deg, var(--primary-alpha-5) 0%, var(--bg-secondary) 100%);
        }

        .updates-status-card.success {
          border-color: var(--success-color);
          background: linear-gradient(135deg, var(--success-alpha-5) 0%, var(--bg-secondary) 100%);
        }

        .updates-status-card.error {
          border-color: var(--error-color);
          background: linear-gradient(135deg, var(--danger-alpha-5) 0%, var(--bg-secondary) 100%);
        }

        .update-badge {
          position: absolute;
          top: var(--spacing-sm);
          right: var(--spacing-sm);
          background: linear-gradient(135deg, var(--primary-color), #58c4dc);
          color: white;
          font-size: 0.625rem;
          font-weight: 700;
          text-transform: uppercase;
          letter-spacing: 0.06em;
          padding: 4px 10px;
          border-radius: var(--radius-xl);
          box-shadow: 0 2px 8px var(--primary-alpha-40);
        }

        /* Status Visual */
        .status-visual {
          display: flex;
          align-items: center;
          justify-content: center;
          position: relative;
        }

        .status-icon-wrapper {
          width: 80px;
          height: 80px;
          border-radius: 50%;
          display: flex;
          align-items: center;
          justify-content: center;
          background: var(--bg-card-hover);
          font-size: 36px;
          color: var(--text-secondary);
          box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
        }

        .status-icon-wrapper.success {
          background: var(--success-alpha-10);
          color: var(--success-color);
          box-shadow: 0 4px 16px var(--success-alpha-20);
        }

        .status-icon-wrapper.error {
          background: var(--danger-alpha-10);
          color: var(--error-color);
          box-shadow: 0 4px 16px var(--danger-alpha-20);
        }

        .pulse-ring {
          position: absolute;
          width: 100px;
          height: 100px;
          border-radius: 50%;
          border: 2px solid var(--primary-color);
          opacity: 0;
          animation: pulse-ring 2s ease-out infinite;
        }

        @keyframes pulse-ring {
          0% { transform: scale(0.8); opacity: 0.7; }
          100% { transform: scale(1.4); opacity: 0; }
        }

        /* Version Transition */
        .version-transition {
          display: flex;
          align-items: center;
          gap: var(--spacing-md);
          font-family: var(--font-mono);
          margin: var(--spacing-md) 0;
          padding: var(--spacing-sm) var(--spacing-lg);
          background: var(--bg-primary);
          border-radius: var(--radius-md);
          border: 1px solid var(--border-color);
        }

        .version-transition .old-version {
          color: var(--text-muted);
          text-decoration: line-through;
          opacity: 0.7;
          font-size: 0.875rem;
          font-weight: 500;
        }

        .version-transition .new-version {
          color: var(--success-color);
          font-weight: 700;
          font-size: 1.125rem;
        }

        .version-transition i {
          color: var(--text-secondary);
          font-size: 0.875rem;
          opacity: 0.6;
        }

        /* Status Content */
        .status-content h3 {
          margin: 0;
          font-size: 1.5rem;
          font-weight: 600;
          color: var(--text-primary);
        }

        .status-content p {
          margin: var(--spacing-xs) 0 0;
          color: var(--text-secondary);
          font-size: var(--font-size-base);
          max-width: 420px;
        }

        .error-message {
          color: var(--error-color) !important;
          font-family: var(--font-mono);
          font-size: var(--font-size-sm) !important;
        }

        /* Release Notes Preview */
        .release-notes-preview {
          width: 100%;
          max-width: 400px;
          text-align: left;
          background: var(--bg-primary);
          border: 1px solid var(--border-color);
          border-radius: var(--radius-md);
          padding: var(--spacing-sm) var(--spacing-md);
        }

        .release-notes-preview h4 {
          margin: 0 0 var(--spacing-xs);
          font-size: 0.625rem;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.05em;
          color: var(--text-secondary);
          display: flex;
          align-items: center;
          gap: var(--spacing-xs);
        }

        .release-notes-preview h4::before {
          content: '';
          display: inline-block;
          width: 2px;
          height: 10px;
          background: var(--primary-color);
          border-radius: 1px;
        }

        .release-notes-preview .notes-text {
          font-size: 0.8125rem;
          color: var(--text-primary);
          line-height: 1.5;
          white-space: pre-wrap;
          max-height: 80px;
          overflow-y: auto;
        }

        /* Status Actions */
        .status-actions {
          display: flex;
          flex-wrap: wrap;
          gap: var(--spacing-sm);
          margin-top: var(--spacing-lg);
          justify-content: center;
        }

        /* Download Progress */
        .download-progress {
          width: 100%;
          max-width: 400px;
          background: var(--bg-primary);
          border: 1px solid var(--border-color);
          border-radius: var(--radius-md);
          padding: var(--spacing-md);
        }

        .progress-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: var(--spacing-sm);
        }

        .progress-status {
          font-size: 0.8125rem;
          color: var(--text-primary);
          font-weight: 500;
        }

        .progress-stats {
          display: flex;
          gap: var(--spacing-sm);
          font-family: var(--font-mono);
          font-size: 0.75rem;
          align-items: center;
        }

        #downloadPercentText {
          color: var(--primary-color);
          font-weight: 600;
        }

        #downloadSpeedText {
          color: var(--text-secondary);
        }

        .progress-track {
          height: 8px;
          background: var(--bg-card-hover);
          border-radius: var(--radius-md);
          overflow: hidden;
        }

        .progress-fill {
          height: 100%;
          background: linear-gradient(90deg, var(--primary-color), #58c4dc);
          border-radius: var(--radius-sm);
          transition: width 0.3s ease;
          position: relative;
        }

        .progress-glow {
          position: absolute;
          right: 0;
          top: 0;
          bottom: 0;
          width: 40px;
          background: linear-gradient(90deg, transparent, rgba(255,255,255,0.3), transparent);
          animation: progress-shimmer 1.5s ease-in-out infinite;
        }

        @keyframes progress-shimmer {
          0% { transform: translateX(-100%); opacity: 0; }
          50% { opacity: 1; }
          100% { transform: translateX(100%); opacity: 0; }
        }

        .progress-footer {
          display: flex;
          justify-content: space-between;
          margin-top: var(--spacing-xs);
          font-size: 0.75rem;
          color: var(--text-muted);
          font-family: var(--font-mono);
        }

        /* Download Success */
        .download-success {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: var(--spacing-md);
        }

        .success-badge {
          display: inline-flex;
          align-items: center;
          gap: var(--spacing-sm);
          padding: var(--spacing-sm) var(--spacing-lg);
          background: var(--success-alpha-10);
          color: var(--success-color);
          border: 1px solid var(--success-alpha-30);
          border-radius: var(--radius-xl);
          font-weight: 600;
          font-size: var(--font-size-base);
        }

        .success-badge i {
          font-size: 1.25rem;
        }

        .install-hint {
          font-size: var(--font-size-sm);
          color: var(--text-secondary);
          text-align: center;
          max-width: 380px;
          line-height: 1.6;
        }

        /* Buttons */
        .updates-btn {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          gap: var(--spacing-sm);
          padding: 12px 24px;
          border: none;
          border-radius: var(--radius-lg);
          font-size: var(--font-size-base);
          font-weight: 600;
          cursor: pointer;
          transition: all 0.2s ease;
          white-space: nowrap;
        }

        .updates-btn i {
          font-size: 1.1rem;
        }

        .updates-btn .btn-meta {
          font-size: var(--font-size-sm);
          opacity: 0.85;
          font-weight: 400;
        }

        .updates-btn.primary {
          background: var(--primary-color);
          color: white;
          box-shadow: 0 2px 8px var(--primary-alpha-20);
        }

        .updates-btn.primary:hover {
          background: var(--primary-hover);
          transform: translateY(-2px);
          box-shadow: 0 6px 16px var(--primary-alpha-30);
        }

        .updates-btn.secondary {
          background: var(--bg-card-hover);
          color: var(--text-primary);
          border: 1px solid var(--border-color);
        }

        .updates-btn.secondary:hover {
          background: var(--bg-card-hover);
          border-color: var(--text-secondary);
          transform: translateY(-1px);
        }

        .updates-btn.success {
          background: var(--success-color);
          color: white;
          box-shadow: 0 2px 8px var(--success-alpha-20);
        }

        .updates-btn.success:hover {
          background: var(--success-hover);
          transform: translateY(-2px);
          box-shadow: 0 6px 16px var(--success-alpha-30);
        }

        /* Version Card */
        .updates-version-card {
          background: var(--bg-secondary);
          border: 1px solid var(--border-color);
          border-radius: var(--radius-lg);
          overflow: hidden;
        }

        .version-card-header {
          display: flex;
          align-items: center;
          gap: var(--spacing-sm);
          padding: var(--spacing-md);
          background: var(--bg-card-hover);
          border-bottom: 1px solid var(--border-color);
        }

        .version-icon {
          width: 44px;
          height: 44px;
          border-radius: var(--radius-md);
          background: var(--primary-alpha-10);
          color: var(--primary-color);
          display: flex;
          align-items: center;
          justify-content: center;
          font-size: 1.4rem;
          flex-shrink: 0;
        }

        .version-info {
          flex: 1;
          min-width: 0;
        }

        .version-info h4 {
          margin: 0 0 2px;
          font-size: 0.625rem;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.06em;
          color: var(--text-secondary);
        }

        .version-number {
          font-family: var(--font-mono);
          font-size: var(--font-size-lg);
          font-weight: 700;
          color: var(--text-primary);
        }

        .version-details {
          padding: var(--spacing-md);
          display: flex;
          flex-direction: column;
          gap: var(--spacing-sm);
        }

        .detail-row {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: var(--spacing-xs) 0;
        }

        .detail-label {
          font-size: var(--font-size-sm);
          color: var(--text-secondary);
          font-weight: 500;
        }

        .detail-value {
          font-size: var(--font-size-sm);
          font-weight: 600;
          color: var(--text-primary);
          font-family: var(--font-mono);
        }

        .channel-badge {
          background: var(--success-alpha-10);
          color: var(--success-color);
          padding: 4px 12px;
          border-radius: var(--radius-xl);
          font-size: 0.625rem;
          font-weight: 700;
          font-family: var(--font-sans);
          text-transform: uppercase;
          letter-spacing: 0.04em;
        }

        /* System Info Section */
        .updates-system-section {
          background: var(--bg-secondary);
          border: 1px solid var(--border-color);
          border-radius: var(--radius-lg);
          overflow: hidden;
          display: flex;
          flex-direction: column;
          flex: 1;
        }

        .system-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: var(--spacing-md);
          background: var(--bg-card-hover);
          border-bottom: 1px solid var(--border-color);
          flex-shrink: 0;
        }

        .system-title {
          display: flex;
          align-items: center;
          gap: var(--spacing-sm);
          font-weight: 600;
          font-size: var(--font-size-base);
          color: var(--text-primary);
        }

        .system-title i {
          color: var(--text-secondary);
          font-size: 1.1rem;
        }

        .system-details {
          padding: var(--spacing-lg);
          display: flex;
          flex-direction: column;
          gap: var(--spacing-md);
        }

        /* Spinner animations */
        .spinner-small {
          width: 18px;
          height: 18px;
          border: 2px solid var(--bg-card-hover);
          border-top-color: var(--primary-color);
          border-radius: 50%;
          animation: spin 0.8s linear infinite;
        }

        .spinning {
          animation: spin 1s linear infinite;
        }

        @keyframes spin {
          to { transform: rotate(360deg); }
        }
      </style>
    `;
  }

  /**
   * Update the Updates tab UI without full rebuild
   */
  _updateUpdatesTabUI() {
    if (!this.dialogEl) return;
    const updatesTab = this.dialogEl.querySelector('.settings-tab[data-tab-content="updates"]');
    if (updatesTab) {
      updatesTab.innerHTML = this._buildUpdatesTab();
      this._attachUpdatesHandlers();
    }
  }

  /**
   * Attach event handlers for Updates tab
   */
  _attachUpdatesHandlers() {
    if (!this.dialogEl) return;
    
    const checkBtn = this.dialogEl.querySelector('#checkUpdatesBtn');
    const retryBtn = this.dialogEl.querySelector('#retryUpdateBtn');
    const downloadBtn = this.dialogEl.querySelector('#downloadUpdateBtn');
    const installBtn = this.dialogEl.querySelector('#installUpdateBtn');

    // Check / Retry Handler
    const handleCheck = async () => {
      globalUpdateState.checking = true;
      globalUpdateState.error = null;
      this._updateUpdatesTabUI();

      try {
        // Call the check API
        const response = await fetch('/api/updates/check');
        const data = await response.json();

        globalUpdateState.checking = false;
        
        if (data.update_available) {
          globalUpdateState.available = true;
          globalUpdateState.info = data.update;  // API returns 'update' not 'update_info'
        } else {
          globalUpdateState.available = false;
          globalUpdateState.info = null;
        }
      } catch (err) {
        console.error('Update check failed:', err);
        globalUpdateState.checking = false;
        globalUpdateState.error = err.message || 'Failed to check for updates';
      }
      
      this._updateUpdatesBadge();
      this._updateUpdatesTabUI();
    };

    if (checkBtn) checkBtn.addEventListener('click', handleCheck);
    if (retryBtn) retryBtn.addEventListener('click', handleCheck);

    // Download Handler
    if (downloadBtn) {
      downloadBtn.addEventListener('click', async () => {
        if (!globalUpdateState.info) return;
        
        globalUpdateState.downloading = true;
        globalUpdateState.progress = 0;
        globalUpdateState.downloadStartTime = Date.now();
        globalUpdateState.downloadedBytes = 0;
        this._updateUpdatesTabUI();

        try {
          // Start download
          const response = await fetch('/api/updates/download', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ version: globalUpdateState.info.version })
          });
          
          if (!response.ok) throw new Error('Failed to start download');

          // Start polling for progress
          this._startDownloadPoller();
          
        } catch (err) {
          console.error('Download start failed:', err);
          globalUpdateState.downloading = false;
          globalUpdateState.error = err.message;
          this._updateUpdatesTabUI();
        }
      });
    }

    // Install Handler
    if (installBtn) {
      installBtn.addEventListener("click", async () => {
        if (
          !window.confirm(
            "ScreenerBot will install the update and close. The installer will launch automatically. Continue?"
          )
        )
          return;

        installBtn.disabled = true;
        const originalText = installBtn.innerHTML;
        installBtn.innerHTML =
          '<i class="icon-loader spinning"></i><span>Installing...</span>';

        try {
          const response = await fetch("/api/updates/install", {
            method: "POST",
          });
          const data = await response.json();

          if (!response.ok || !data.success) {
            throw new Error(data.error || "Failed to open installer");
          }

          // Show success message
          Utils.showToast({
            type: "success",
            title: "Update Ready",
            message: "Closing app to complete installation...",
          });

          // Exit the app via backend API after a short delay
          setTimeout(async () => {
            try {
              await fetch("/api/system/exit", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ delay_ms: 500 }),
              });
            } catch (exitErr) {
              console.warn("Exit request failed:", exitErr);
              // Fallback: just close the dialog
              this.close();
            }
          }, 1000);
        } catch (err) {
          console.error("Install failed:", err);
          installBtn.disabled = false;
          installBtn.innerHTML = originalText;

          Utils.showToast({
            type: "error",
            title: "Failed to Open Installer",
            message: err.message || "Please try downloading again.",
          });
        }
      });
    }
  }

  /**
   * Attach handlers for Licenses tab
   */
  _attachLicensesHandlers(content) {
    // External links (license project URLs)
    const externalLinks = content.querySelectorAll("[data-external-url]");
    externalLinks.forEach((btn) => {
      btn.addEventListener("click", () => {
        const url = btn.dataset.externalUrl;
        if (url) {
          Utils.openExternal(url);
        }
      });
    });
  }

  /**
   * Attach handlers for About tab
   */
  _attachAboutHandlers(content) {
    // External links (GitHub, Docs, Discord)
    const externalLinks = content.querySelectorAll("[data-external-url]");
    externalLinks.forEach((btn) => {
      btn.addEventListener("click", () => {
        const url = btn.dataset.externalUrl;
        if (url) {
          Utils.openExternal(url);
        }
      });
    });

    const openBtn = content.querySelector("#openDataFolderBtn");
    if (openBtn) {
      openBtn.addEventListener("click", async () => {
        openBtn.disabled = true;
        const originalLabel = openBtn.innerHTML;
        openBtn.innerHTML = '<i class="icon-loader spinning"></i><span>Opening...</span>';

        try {
          const response = await fetch("/api/system/paths/open-data", {
            method: "POST",
          });

          if (!response.ok) {
            const error = await response.json().catch(() => ({}));
            throw new Error(error.error?.message || "Failed to open data folder");
          }

          Utils.showToast({
            type: "success",
            title: "Data folder opened",
            message: this.pathsInfo?.data_directory || "",
          });
        } catch (err) {
          console.error("Failed to open data folder:", err);
          Utils.showToast({
            type: "error",
            title: "Unable to open data folder",
            message: err.message,
          });
        } finally {
          openBtn.disabled = false;
          openBtn.innerHTML = originalLabel;
        }
      });
    }

    const copyBtn = content.querySelector("#copyDataFolderBtn");
    if (copyBtn) {
      copyBtn.addEventListener("click", async () => {
        if (!this.pathsInfo?.data_directory) {
          Utils.showToast({
            type: "warning",
            title: "Path not available",
            message: "Data path is still loading",
          });
          return;
        }

        try {
          await Utils.copyToClipboard(this.pathsInfo.data_directory);
          Utils.showToast({
            type: "success",
            title: "Data path copied",
          });
        } catch (err) {
          console.error("Failed to copy data path:", err);
          Utils.showToast({
            type: "error",
            title: "Copy failed",
            message: err.message,
          });
        }
      });
    }
  }

  /**
   * Build About tab content
   */
  _buildAboutTab() {
    const { version } = this.versionInfo;
    const paths = this.pathsInfo || {};
    const dataPath = Utils.escapeHtml(paths.data_directory || "Loading data path...");
    const basePath = paths.base_directory ? Utils.escapeHtml(paths.base_directory) : "";
    return `
      <div class="settings-about">
        <div class="settings-about-logo">
          <img src="/assets/logo.svg" alt="ScreenerBot" />
        </div>
        <h2 class="settings-about-name">ScreenerBot</h2>
        <p class="settings-about-tagline">Advanced Solana Trading Automation</p>
        <div class="settings-about-version">
          <span>v${version}</span>
        </div>

        <div class="settings-about-links">
          <button class="settings-about-link" data-external-url="https://github.com/farfary/ScreenerBot">
            <i class="icon-github"></i>
            <span>GitHub</span>
          </button>
          <button class="settings-about-link" data-external-url="https://docs.screenerbot.app">
            <i class="icon-book-open"></i>
            <span>Documentation</span>
          </button>
          <button class="settings-about-link" data-external-url="https://discord.gg/screenerbot">
            <i class="icon-message-circle"></i>
            <span>Discord</span>
          </button>
        </div>

        <div class="settings-about-path-card">
          <div class="settings-about-path-icon">
            <i class="icon-folder"></i>
          </div>
          <div class="settings-about-path-details">
            <p class="settings-about-path-label">Data Directory</p>
            <p class="settings-about-path-value">${dataPath}</p>
            ${basePath ? `<p class="settings-about-path-hint">Base directory: ${basePath}</p>` : ""}
          </div>
          <div class="settings-about-path-actions">
            <button class="settings-update-btn" id="openDataFolderBtn">
              <i class="icon-folder-open"></i>
              <span>Open Data Folder</span>
            </button>
            <button class="settings-update-btn" id="copyDataFolderBtn">
              <i class="icon-copy"></i>
              <span>Copy Path</span>
            </button>
          </div>
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
                  <button class="license-item-name" data-external-url="${Utils.escapeHtml(item.url)}">
                    ${Utils.escapeHtml(item.name)}
                    <i class="icon-external-link"></i>
                  </button>
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

  /**
   * Start the download progress poller
   */
  _startDownloadPoller() {
    if (this.downloadPoller) this.downloadPoller.stop();
    
    // Track download speed
    let lastProgress = 0;
    let lastTime = Date.now();
    let speedHistory = [];
    
    this.downloadPoller = new Poller(async () => {
      try {
        const statusRes = await fetch('/api/updates/status');
        const data = await statusRes.json();
        // API returns { state: { download_progress: { downloading, progress_percent, completed, error, downloaded_bytes, total_bytes } } }
        const state = data.state || data;
        const progress = state.download_progress || {};
        
        if (progress.downloading) {
          const currentProgress = progress.progress_percent || 0;
          const now = Date.now();
          const elapsed = (now - lastTime) / 1000; // seconds
          
          // Calculate speed (using progress percentage and total size if available)
          let speedMBps = 0;
          let etaText = '';
          
          if (progress.downloaded_bytes && progress.total_bytes) {
            const bytesDiff = progress.downloaded_bytes - (lastProgress / 100 * progress.total_bytes);
            if (elapsed > 0 && bytesDiff > 0) {
              const bytesPerSec = bytesDiff / elapsed;
              speedMBps = bytesPerSec / (1024 * 1024);
              
              // Smooth speed using moving average
              speedHistory.push(speedMBps);
              if (speedHistory.length > 5) speedHistory.shift();
              const avgSpeed = speedHistory.reduce((a, b) => a + b, 0) / speedHistory.length;
              
              // Calculate ETA
              const remainingBytes = progress.total_bytes - progress.downloaded_bytes;
              const etaSeconds = remainingBytes / (avgSpeed * 1024 * 1024);
              if (etaSeconds < 60) {
                etaText = `${Math.round(etaSeconds)}s remaining`;
              } else if (etaSeconds < 3600) {
                etaText = `${Math.round(etaSeconds / 60)}m remaining`;
              } else {
                etaText = `${Math.round(etaSeconds / 3600)}h remaining`;
              }
              
              speedMBps = avgSpeed;
            }
          } else {
            // Fallback: estimate speed from progress change
            const progressDiff = currentProgress - lastProgress;
            if (elapsed > 0 && progressDiff > 0 && globalUpdateState.info?.file_size) {
              const totalBytes = globalUpdateState.info.file_size;
              const bytesDiff = (progressDiff / 100) * totalBytes;
              speedMBps = bytesDiff / elapsed / (1024 * 1024);
            }
          }
          
          lastProgress = currentProgress;
          lastTime = now;
          globalUpdateState.progress = currentProgress;
          
          // Update UI elements directly for smoothness
          if (this.dialogEl) {
            const bar = this.dialogEl.querySelector('#downloadProgressBar');
            const percentText = this.dialogEl.querySelector('#downloadPercentText');
            const speedText = this.dialogEl.querySelector('#downloadSpeedText');
            const etaElement = this.dialogEl.querySelector('#downloadEtaText');
            const sizeText = this.dialogEl.querySelector('#downloadSizeText');
            
            if (bar) bar.style.width = `${currentProgress}%`;
            if (percentText) percentText.textContent = `${Math.round(currentProgress)}%`;
            if (speedText && speedMBps > 0) {
              speedText.textContent = `${speedMBps.toFixed(1)} MB/s`;
            }
            if (etaElement && etaText) {
              etaElement.textContent = etaText;
            }
            if (sizeText && progress.downloaded_bytes && progress.total_bytes) {
              sizeText.textContent = `${this._formatBytes(progress.downloaded_bytes)} / ${this._formatBytes(progress.total_bytes)}`;
            }
          }
        } else if (progress.completed && progress.downloaded_path) {
          globalUpdateState.downloading = false;
          globalUpdateState.downloaded = true;
          globalUpdateState.progress = 100;
          if (this.downloadPoller) this.downloadPoller.stop();
          this._updateUpdatesTabUI();
        } else if (progress.error) {
          throw new Error(progress.error || 'Download failed');
        }
      } catch (err) {
        console.error('Download poll error:', err);
        globalUpdateState.downloading = false;
        globalUpdateState.error = err.message;
        if (this.downloadPoller) this.downloadPoller.stop();
        this._updateUpdatesTabUI();
      }
    }, 1000);
    
    this.downloadPoller.start();
  }

  /**
   * Switch to a specific tab
   */
  switchToTab(tabId) {
    if (!this.dialogEl) return;
    
    const navItem = this.dialogEl.querySelector(`.settings-nav-item[data-tab="${tabId}"]`);
    if (navItem) {
      navItem.click();
    }
  }
}

// Singleton instance for easy access
let settingsDialogInstance = null;

export async function showSettingsDialog(options = {}) {
  if (!settingsDialogInstance) {
    settingsDialogInstance = new SettingsDialog({
      onClose: () => {
        settingsDialogInstance = null;
      },
    });
  }
  await settingsDialogInstance.show();
  
  // Switch to specific tab if requested (after dialog is shown)
  if (options.tab) {
    // Small delay to ensure DOM is ready
    setTimeout(() => {
      settingsDialogInstance.switchToTab(options.tab);
    }, 100);
  }
}

export function closeSettingsDialog() {
  if (settingsDialogInstance) {
    settingsDialogInstance.close();
  }
}

/**
 * Check for updates and auto-show dialog if update available
 * Called after dashboard is fully loaded
 */
export async function checkAndShowUpdateDialog() {
  // Don't check in CLI mode (no auto-updates)
  if (!window.__SCREENERBOT_GUI_MODE) {
    return;
  }

  try {
    // First check current status
    let response = await fetch('/api/updates/status');
    if (!response.ok) return;
    
    let data = await response.json();
    let state = data.state || data;
    
    // If no check has happened yet, trigger one
    if (!state.last_check && !state.available_update) {
      console.log('[SettingsDialog] No update check done yet, triggering check...');
      const checkResponse = await fetch('/api/updates/check');
      if (checkResponse.ok) {
        const checkData = await checkResponse.json();
        // Update state from check response
        if (checkData.update_available && checkData.update) {
          state = {
            available_update: checkData.update,
            last_check: checkData.last_check,
            download_progress: state.download_progress || {}
          };
        }
      }
    }
    
    // Check if update is available or downloading
    if (state.available_update || state.download_progress?.downloading) {
      console.log('[SettingsDialog] Update available, showing dialog...');
      
      // Update global state
      globalUpdateState.available = true;
      globalUpdateState.info = state.available_update;
      
      if (state.download_progress?.downloading) {
        globalUpdateState.downloading = true;
        globalUpdateState.progress = state.download_progress.progress_percent || 0;
      }
      
      // Show settings dialog with Updates tab selected
      await showSettingsDialog({ tab: 'updates' });
    }
  } catch (err) {
    console.warn('[SettingsDialog] Failed to check for updates on startup:', err);
  }
}

// Auto-check for updates when dashboard is ready
// Use dynamic import to avoid circular dependencies and ensure bootstrap is loaded
(async function initUpdateCheck() {
  if (typeof window === 'undefined' || !window.__SCREENERBOT_GUI_MODE) {
    return;
  }

  try {
    // Dynamically import bootstrap to get waitForReady
    const { waitForReady } = await import('../core/bootstrap.js');
    
    // Wait for dashboard to be ready
    await waitForReady();
    
    // Small delay to ensure UI is fully rendered
    setTimeout(checkAndShowUpdateDialog, 1500);
  } catch (err) {
    console.warn('[SettingsDialog] Failed to initialize update check:', err);
  }
})();

