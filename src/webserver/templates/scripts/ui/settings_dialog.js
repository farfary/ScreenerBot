/**
 * Settings Dialog Component
 * Full-screen settings dialog with tabs for Interface, Startup, About, Updates
 */
import * as Utils from "../core/utils.js";
import { createFocusTrap } from "../core/utils.js";
import { getCurrentPage } from "../core/router.js";
import { ConfirmationDialog } from "./confirmation_dialog.js";
import { setInterval as setPollingInterval, Poller } from "../core/poller.js";
import { enhanceAllSelects } from "./custom_select.js";
import { setSoundsEnabled, playPanelOpen, playPanelClose, playTabSwitch } from "../core/sounds.js";

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
  statusPoller: null,
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
    this._focusTrap = null;
    this._discoveryPoller = null;
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
        // Play panel open sound
        playPanelOpen();
        // Add ARIA attributes for accessibility
        const container = this.dialogEl.querySelector(".settings-container");
        if (container) {
          container.setAttribute("role", "dialog");
          container.setAttribute("aria-modal", "true");
          container.setAttribute("aria-labelledby", "settings-dialog-title");
        }
        // Activate focus trap
        this._focusTrap = createFocusTrap(this.dialogEl);
        this._focusTrap.activate();
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
      const response = await fetch("/api/updates/status");
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
      console.warn("Failed to sync update status:", err);
    }

    // If not downloading/ready, maybe check for updates if not checked yet
    if (
      !globalUpdateState.checked &&
      !globalUpdateState.checking &&
      !globalUpdateState.downloading &&
      !globalUpdateState.downloaded
    ) {
      this._performBackgroundUpdateCheck();
    } else {
      this._updateUpdatesBadge();
      if (this.currentTab === "updates") {
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
      const response = await fetch("/api/updates/check");
      const data = await response.json();

      globalUpdateState.checking = false;

      if (data.update_available) {
        globalUpdateState.available = true;
        globalUpdateState.info = data.update; // API returns 'update' not 'update_info'
      } else {
        globalUpdateState.available = false;
      }
    } catch (err) {
      console.error("Background update check failed:", err);
      globalUpdateState.checking = false;
      globalUpdateState.error = err.message;
    }

    this._updateUpdatesBadge();

    // If user is currently on updates tab, refresh it
    if (this.currentTab === "updates") {
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
    const existingIndicator = updatesBtn.querySelector(".settings-nav-indicator");
    if (existingIndicator) existingIndicator.remove();

    if (globalUpdateState.available) {
      const indicator = document.createElement("span");
      indicator.className = "settings-nav-indicator";
      indicator.title = "New update available";
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

    // Play panel close sound
    playPanelClose();

    // Deactivate focus trap
    if (this._focusTrap) {
      this._focusTrap.deactivate();
      this._focusTrap = null;
    }

    // Stop any active pollers
    if (this.downloadPoller) {
      this.downloadPoller.stop();
      this.downloadPoller = null;
    }

    // Stop discovery poller if active
    if (this._discoveryPoller) {
      clearInterval(this._discoveryPoller);
      this._discoveryPoller = null;
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
      { id: "tools", label: "Tools", icon: "icon-wrench", order: 1, enabled: true },
      {
        id: "positions",
        label: "Positions",
        icon: "icon-chart-candlestick",
        order: 2,
        enabled: true,
      },
      { id: "tokens", label: "Tokens", icon: "icon-coins", order: 3, enabled: true },
      { id: "filtering", label: "Filtering", icon: "icon-list-filter", order: 4, enabled: true },
      { id: "wallets", label: "Wallets", icon: "icon-wallet", order: 5, enabled: true },
      { id: "trader", label: "Auto Trader", icon: "icon-bot", order: 6, enabled: true },
      { id: "strategies", label: "Strategies", icon: "icon-target", order: 7, enabled: true },
      { id: "transactions", label: "Transactions", icon: "icon-activity", order: 8, enabled: true },
      { id: "services", label: "Services", icon: "icon-server", order: 9, enabled: true },
      { id: "config", label: "Config", icon: "icon-settings", order: 10, enabled: true },
      { id: "events", label: "Events", icon: "icon-radio-tower", order: 11, enabled: true },
      { id: "ai", label: "AI", icon: "icon-sparkles", order: 12, enabled: true },
    ];
  }

  /**
   * Fetch default tab configuration from backend (single source of truth)
   * Falls back to local defaults on failure
   */
  async _fetchDefaultTabs() {
    try {
      const response = await fetch("/api/config/gui/defaults");
      if (response.ok) {
        const result = await response.json();
        if (result.success && result.data?.tabs) {
          return result.data.tabs;
        }
      }
    } catch (e) {
      console.warn("Failed to fetch default tabs from API, using local fallback", e);
    }
    return this._getDefaultTabs();
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
      // Save theme to server (no localStorage)
      fetch("/api/ui-state/save", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ key: "theme", value: iface.theme }),
      }).catch((e) => console.warn("[Settings] Failed to save theme:", e));
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

    // Apply hints toggle
    if (typeof iface.show_hints === "boolean") {
      // Dispatch event for hints system to react
      document.dispatchEvent(
        new CustomEvent("hints:toggle", { detail: { enabled: iface.show_hints } })
      );
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
            <button class="settings-nav-item" data-tab="data">
              <i class="icon-database"></i>
              <span>Data</span>
            </button>
            <button class="settings-nav-item" data-tab="security">
              <i class="icon-lock"></i>
              <span>Security</span>
            </button>
            <button class="settings-nav-item" data-tab="telegram">
              <i class="icon-send"></i>
              <span>Telegram</span>
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
            <div class="settings-tab" data-tab-content="data">
              <div class="settings-loading">Loading...</div>
            </div>
            <div class="settings-tab" data-tab-content="security">
              <div class="settings-loading">Loading...</div>
            </div>
            <div class="settings-tab" data-tab-content="telegram">
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
    // Play tab switch sound
    playTabSwitch();

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
        enhanceAllSelects(content);
        break;
      case "navigation":
        content.innerHTML = this._buildNavigationTab();
        this._attachNavigationHandlers(content);
        break;
      case "startup":
        content.innerHTML = this._buildStartupTab();
        this._attachStartupHandlers(content);
        enhanceAllSelects(content);
        break;
      case "data":
        content.innerHTML = this._buildDataTab();
        this._attachDataHandlers(content);
        break;
      case "security":
        this._loadSecurityTab(content);
        break;
      case "telegram":
        this._loadTelegramTab(content);
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
              <select id="settingTheme" class="settings-select" data-custom-select>
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
              <select id="settingPolling" class="settings-select" data-custom-select>
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
              <select id="settingPageSize" class="settings-select" data-custom-select>
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

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Show Contextual Hints</label>
              <span class="settings-field-hint">Display help icons explaining dashboard features</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="settingShowHints" ${iface.show_hints !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Show Billboard Row</label>
              <span class="settings-field-hint">Display featured tokens row on Home and Tokens pages</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="settingShowBillboard" ${iface.show_billboard !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>
        </div>
      </div>

      <div class="settings-section">
        <h3 class="settings-section-title">Sound Effects</h3>
        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Enable Sounds</label>
              <span class="settings-field-hint">Subtle audio feedback for button clicks and actions</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="settingSoundsEnabled" ${iface.sounds_enabled !== false ? "checked" : ""}>
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
      showHints: content.querySelector("#settingShowHints"),
      showBillboard: content.querySelector("#settingShowBillboard"),
      soundsEnabled: content.querySelector("#settingSoundsEnabled"),
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
    if (fields.showHints) {
      fields.showHints.addEventListener("change", (e) =>
        updateSetting("show_hints", e.target.checked)
      );
    }
    if (fields.showBillboard) {
      fields.showBillboard.addEventListener("change", (e) =>
        updateSetting("show_billboard", e.target.checked)
      );
    }
    if (fields.soundsEnabled) {
      fields.soundsEnabled.addEventListener("change", (e) => {
        updateSetting("sounds_enabled", e.target.checked);
        setSoundsEnabled(e.target.checked);
      });
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
        <div class="settings-nav-tab-item" 
             data-tab-id="${tab.id}" 
             data-order="${tab.order}"
             draggable="true">
          <div class="settings-nav-tab-position">${index + 1}</div>
          <div class="settings-nav-tab-handle" title="Drag to reorder">
            <i class="icon-grip-vertical"></i>
          </div>
          <div class="settings-nav-tab-icon">
            <i class="${tab.icon}"></i>
          </div>
          <div class="settings-nav-tab-info">
            <span class="settings-nav-tab-label">${tab.label}</span>
          </div>
          <div class="settings-nav-tab-status ${tab.enabled ? "enabled" : "disabled"}">
            ${tab.enabled ? '<i class="icon-eye"></i>' : '<i class="icon-eye-off"></i>'}
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
        <div class="settings-section-header">
          <div class="settings-section-header-left">
            <h3 class="settings-section-title">
              <i class="icon-layout-grid"></i>
              Navigation Tabs
            </h3>
            <p class="settings-section-hint">Drag items to reorder. Toggle visibility with the switch.</p>
          </div>
          <button class="btn btn-secondary btn-sm" id="resetNavTabs">
            <i class="icon-rotate-ccw"></i>
            Reset
          </button>
        </div>
        <div class="settings-nav-tabs-list" id="navTabsList">
          ${tabItems}
        </div>
        <div class="settings-nav-tabs-note">
          <i class="icon-info"></i>
          <span>Changes apply after saving. Refresh the page to see updates in the navigation bar.</span>
        </div>
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

    // Drag and drop state
    let draggedItem = null;
    let draggedTabId = null;

    // Drag start
    list.addEventListener("dragstart", (e) => {
      const item = e.target.closest(".settings-nav-tab-item");
      if (!item) return;

      draggedItem = item;
      draggedTabId = item.dataset.tabId;
      item.classList.add("dragging");

      // Set drag image and data
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", draggedTabId);

      // Delay adding drag class for smooth animation
      requestAnimationFrame(() => {
        item.style.opacity = "0.5";
      });
    });

    // Drag end
    list.addEventListener("dragend", (e) => {
      const item = e.target.closest(".settings-nav-tab-item");
      if (item) {
        item.classList.remove("dragging");
        item.style.opacity = "";
      }
      draggedItem = null;
      draggedTabId = null;

      // Remove all drag-over states
      list.querySelectorAll(".settings-nav-tab-item").forEach((el) => {
        el.classList.remove("drag-over", "drag-over-top", "drag-over-bottom");
      });
    });

    // Drag over
    list.addEventListener("dragover", (e) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";

      const item = e.target.closest(".settings-nav-tab-item");
      if (!item || item === draggedItem) return;

      // Auto-scroll when near edges
      const listRect = list.getBoundingClientRect();
      const scrollZone = 50;
      const scrollSpeed = 8;
      
      if (e.clientY < listRect.top + scrollZone) {
        list.scrollTop -= scrollSpeed;
      } else if (e.clientY > listRect.bottom - scrollZone) {
        list.scrollTop += scrollSpeed;
      }

      // Calculate position for visual feedback
      const rect = item.getBoundingClientRect();
      const midY = rect.top + rect.height / 2;

      // Remove previous indicators
      list.querySelectorAll(".settings-nav-tab-item").forEach((el) => {
        if (el !== item) {
          el.classList.remove("drag-over", "drag-over-top", "drag-over-bottom");
        }
      });

      // Add indicator based on mouse position
      item.classList.add("drag-over");
      if (e.clientY < midY) {
        item.classList.add("drag-over-top");
        item.classList.remove("drag-over-bottom");
      } else {
        item.classList.add("drag-over-bottom");
        item.classList.remove("drag-over-top");
      }
    });

    // Drag leave
    list.addEventListener("dragleave", (e) => {
      const item = e.target.closest(".settings-nav-tab-item");
      if (item && !item.contains(e.relatedTarget)) {
        item.classList.remove("drag-over", "drag-over-top", "drag-over-bottom");
      }
    });

    // Drop
    list.addEventListener("drop", (e) => {
      e.preventDefault();

      const dropTarget = e.target.closest(".settings-nav-tab-item");
      if (!dropTarget || !draggedTabId || dropTarget.dataset.tabId === draggedTabId) {
        // Clean up
        list.querySelectorAll(".settings-nav-tab-item").forEach((el) => {
          el.classList.remove("drag-over", "drag-over-top", "drag-over-bottom");
        });
        return;
      }

      const tabs = getTabs();
      
      // Get current sorted order
      const sortedTabs = [...tabs].sort((a, b) => a.order - b.order);
      const draggedIdx = sortedTabs.findIndex((t) => t.id === draggedTabId);
      const dropIdx = sortedTabs.findIndex((t) => t.id === dropTarget.dataset.tabId);

      if (draggedIdx === -1 || dropIdx === -1) return;

      // Calculate insert position based on mouse
      const rect = dropTarget.getBoundingClientRect();
      const insertBefore = e.clientY < rect.top + rect.height / 2;

      // Remove from old position
      const [movedTab] = sortedTabs.splice(draggedIdx, 1);
      
      // Calculate new position (accounting for removal)
      let insertIdx = dropIdx;
      if (draggedIdx < dropIdx) {
        // Dragging down - dropIdx shifted by 1 after removal
        insertIdx = insertBefore ? dropIdx - 1 : dropIdx;
      } else {
        // Dragging up
        insertIdx = insertBefore ? dropIdx : dropIdx + 1;
      }
      
      // Clamp to valid range
      insertIdx = Math.max(0, Math.min(insertIdx, sortedTabs.length));
      
      // Insert at new position
      sortedTabs.splice(insertIdx, 0, movedTab);

      // Update order values in original tabs array
      sortedTabs.forEach((tab, idx) => {
        const originalTab = tabs.find((t) => t.id === tab.id);
        if (originalTab) originalTab.order = idx;
      });

      // Save and refresh
      setTabs(tabs);
      this._refreshNavigationList(content);

      // Clean up drag states
      list.querySelectorAll(".settings-nav-tab-item").forEach((el) => {
        el.classList.remove("drag-over", "drag-over-top", "drag-over-bottom");
      });
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
            // Update status icon
            const statusEl = item.querySelector(".settings-nav-tab-status");
            if (statusEl) {
              statusEl.className = `settings-nav-tab-status ${tab.enabled ? "enabled" : "disabled"}`;
              statusEl.innerHTML = tab.enabled
                ? '<i class="icon-eye"></i>'
                : '<i class="icon-eye-off"></i>';
            }
            setTabs(tabs);
          }
        }
      }
    });

    // Reset button handler
    const resetBtn = content.querySelector("#resetNavTabs");
    if (resetBtn) {
      resetBtn.addEventListener("click", async () => {
        // Fetch defaults from backend (single source of truth)
        const defaultTabs = await this._fetchDefaultTabs();
        this.settings.dashboard.navigation.tabs = defaultTabs;
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
        <div class="settings-nav-tab-item" 
             data-tab-id="${tab.id}" 
             data-order="${tab.order}"
             draggable="true">
          <div class="settings-nav-tab-position">${index + 1}</div>
          <div class="settings-nav-tab-handle" title="Drag to reorder">
            <i class="icon-grip-vertical"></i>
          </div>
          <div class="settings-nav-tab-icon">
            <i class="${tab.icon}"></i>
          </div>
          <div class="settings-nav-tab-info">
            <span class="settings-nav-tab-label">${tab.label}</span>
          </div>
          <div class="settings-nav-tab-status ${tab.enabled ? "enabled" : "disabled"}">
            ${tab.enabled ? '<i class="icon-eye"></i>' : '<i class="icon-eye-off"></i>'}
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
              <select id="settingDefaultPage" class="settings-select" data-custom-select>
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
   * Build Data tab content - Comprehensive data management
   */
  _buildDataTab() {
    return `
      <!-- Database Overview Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-database"></i>
          Database Storage
        </h3>
        <p class="settings-section-description">
          Overview of all databases storing your trading data, positions, and historical information.
        </p>
        
        <div class="data-overview-card" id="dataOverviewCard">
          <div class="data-stats-loading"><i class="icon-loader"></i> Loading database statistics...</div>
        </div>
      </div>

      <!-- Configuration Backup Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-settings"></i>
          Configuration Management
        </h3>
        <p class="settings-section-description">
          Export, import, and manage your bot configuration. Keep backups before making major changes.
        </p>
        
        <div class="settings-group">
          <div class="config-actions-row">
            <button id="exportConfigBtn" class="btn btn-primary">
              <i class="icon-download"></i>
              Export Config
            </button>
            <button id="importConfigBtn" class="btn btn-secondary">
              <i class="icon-upload"></i>
              Import Config
            </button>
            <button id="resetConfigBtn" class="btn btn-warning">
              <i class="icon-refresh-cw"></i>
              Reset to Defaults
            </button>
          </div>
          <input type="file" id="configFileInput" accept=".json,.toml" style="display: none;" />
          
          <div class="config-info-box">
            <div class="config-info-item">
              <span class="config-info-label">Config Location</span>
              <span class="config-info-value" id="configPathDisplay">Loading...</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Data Cleanup Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-trash-2"></i>
          Data Cleanup
        </h3>
        <p class="settings-section-description">
          Free up disk space by removing old or unused data. These actions cannot be undone.
        </p>
        
        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label>OHLCV Data Cleanup</label>
              <span class="settings-field-hint">
                Remove candlestick data for tokens that haven't been active for the specified time.
              </span>
            </div>
            <div class="settings-field-control data-action-group">
              <input type="number" id="cleanupHours" class="settings-input small" value="24" min="1" max="720" />
              <span class="input-suffix">hours</span>
              <button id="cleanupOhlcvBtn" class="btn btn-warning btn-sm">
                <i class="icon-trash-2"></i>
                Cleanup OHLCV
              </button>
            </div>
          </div>
          
          <div class="settings-field">
            <div class="settings-field-info">
              <label>UI State Cache</label>
              <span class="settings-field-hint">
                Clear saved table preferences, filter states, and view settings.
              </span>
            </div>
            <div class="settings-field-control">
              <button id="clearUiStateBtn" class="btn btn-secondary btn-sm">
                <i class="icon-refresh-cw"></i>
                Clear UI Cache
              </button>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Open Data Folder</label>
              <span class="settings-field-hint">
                Open the folder containing all ScreenerBot data in your file manager.
              </span>
            </div>
            <div class="settings-field-control">
              <button id="openDataFolderBtn" class="btn btn-secondary btn-sm">
                <i class="icon-folder"></i>
                Open Folder
              </button>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Attach handlers for Data tab
   */
  _attachDataHandlers(content) {
    // Load data overview
    this._loadDataOverview(content);

    // Display config path with copy on click
    if (this.pathsInfo?.config_path) {
      const pathDisplay = content.querySelector("#configPathDisplay");
      if (pathDisplay) {
        pathDisplay.textContent = this.pathsInfo.config_path;
        pathDisplay.title = "Click to copy path";
        pathDisplay.addEventListener("click", async () => {
          try {
            await Utils.copyToClipboard(this.pathsInfo.config_path);
            Utils.showToast("Config path copied to clipboard", "success");
          } catch (err) {
            Utils.showToast("Failed to copy path", "error");
          }
        });
      }
    }

    // Export config button
    const exportBtn = content.querySelector("#exportConfigBtn");
    if (exportBtn) {
      exportBtn.addEventListener("click", () => this._exportConfig());
    }

    // Import config button
    const importBtn = content.querySelector("#importConfigBtn");
    const fileInput = content.querySelector("#configFileInput");
    if (importBtn && fileInput) {
      importBtn.addEventListener("click", () => fileInput.click());
      fileInput.addEventListener("change", (e) => this._importConfig(e));
    }

    // Reset config button
    const resetBtn = content.querySelector("#resetConfigBtn");
    if (resetBtn) {
      resetBtn.addEventListener("click", () => this._resetConfig());
    }

    // Trading preset buttons - handle both card click and button click
    content.querySelectorAll(".preset-card").forEach((card) => {
      card.addEventListener("click", (e) => {
        // Don't trigger if clicking the button (let button handler work)
        if (e.target.closest(".preset-apply-btn")) return;
        const preset = card.dataset.preset;
        this._applyPreset(preset);
      });
    });

    content.querySelectorAll(".preset-apply-btn").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        e.stopPropagation();
        const preset = btn.dataset.preset;
        this._applyPreset(preset);
      });
    });

    // OHLCV cleanup button
    const cleanupBtn = content.querySelector("#cleanupOhlcvBtn");
    const hoursInput = content.querySelector("#cleanupHours");

    if (cleanupBtn && hoursInput) {
      cleanupBtn.addEventListener("click", async () => {
        const hours = parseInt(hoursInput.value, 10);
        if (isNaN(hours) || hours < 1) {
          Utils.showToast("Invalid hours value", "error");
          return;
        }

        const confirmResult = await ConfirmationDialog.show({
          title: "Delete OHLCV Data",
          message: `Delete OHLCV data for tokens inactive for more than ${hours} hours?`,
          confirmLabel: "Delete",
          cancelLabel: "Cancel",
          variant: "danger",
        });
        if (!confirmResult.confirmed) return;

        cleanupBtn.disabled = true;
        cleanupBtn.innerHTML = '<i class="icon-loader spin"></i> Cleaning...';

        try {
          const response = await fetch("/api/ohlcv/cleanup", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ inactive_hours: hours }),
          });

          if (response.ok) {
            const data = await response.json();
            Utils.showToast(`Cleaned up ${data.deleted_count} inactive tokens`, "success");
            this._loadDataOverview(content);
          } else {
            Utils.showToast("Cleanup failed", "error");
          }
        } catch (err) {
          Utils.showToast("Cleanup failed: " + err.message, "error");
        } finally {
          cleanupBtn.disabled = false;
          cleanupBtn.innerHTML = '<i class="icon-trash-2"></i> Cleanup OHLCV';
        }
      });
    }

    // Clear UI state button
    const clearUiBtn = content.querySelector("#clearUiStateBtn");
    if (clearUiBtn) {
      clearUiBtn.addEventListener("click", async () => {
        const confirmResult = await ConfirmationDialog.show({
          title: "Clear UI State",
          message:
            "Clear all saved UI preferences? This will reset table columns, filters, and view settings.",
          confirmLabel: "Clear",
          cancelLabel: "Cancel",
          variant: "danger",
        });
        if (!confirmResult.confirmed) return;

        const keysToRemove = [];
        for (let i = 0; i < localStorage.length; i++) {
          const key = localStorage.key(i);
          if (
            key &&
            (key.startsWith("table.") ||
              key.startsWith("tokens-table") ||
              key.startsWith("positions-table") ||
              key.includes(".state"))
          ) {
            keysToRemove.push(key);
          }
        }

        keysToRemove.forEach((key) => localStorage.removeItem(key));
        Utils.showToast(`Cleared ${keysToRemove.length} cached UI settings`, "success");
      });
    }

    // Open data folder button
    const openFolderBtn = content.querySelector("#openDataFolderBtn");
    if (openFolderBtn) {
      openFolderBtn.addEventListener("click", async () => {
        try {
          const response = await fetch("/api/system/paths/open-data", { method: "POST" });
          if (response.ok) {
            Utils.showToast("Data folder opened", "success");
          } else {
            Utils.showToast("Failed to open folder", "error");
          }
        } catch (err) {
          Utils.showToast("Failed to open folder: " + err.message, "error");
        }
      });
    }
  }

  /**
   * Load comprehensive data overview
   */
  async _loadDataOverview(content) {
    const card = content.querySelector("#dataOverviewCard");
    if (!card) return;

    try {
      const response = await fetch("/api/system/data-stats");
      if (!response.ok) throw new Error("Failed to load stats");

      const data = await response.json();
      const maxSize = Math.max(...data.databases.map((db) => db.size_bytes), 1);

      const dbItemsHtml = data.databases
        .filter((db) => db.exists)
        .map((db) => {
          const percentage = (db.size_bytes / maxSize) * 100;
          const sizeDisplay =
            db.size_mb >= 1
              ? `${db.size_mb.toFixed(1)} MB`
              : `${(db.size_bytes / 1024).toFixed(0)} KB`;
          return `
            <div class="data-db-item">
              <span class="data-db-name">${db.name}</span>
              <div class="data-db-bar-container">
                <div class="data-db-bar" style="width: ${percentage}%"></div>
              </div>
              <span class="data-db-size">${sizeDisplay}</span>
            </div>
          `;
        })
        .join("");

      card.innerHTML = `
        <div class="data-total-bar">
          <span class="data-total-label">Total Database Storage</span>
          <span class="data-total-value">${data.total_size_mb.toFixed(1)} MB</span>
        </div>
        <div class="data-db-list">
          ${dbItemsHtml}
        </div>
      `;

      // Update config path display
      const pathDisplay = content.querySelector("#configPathDisplay");
      if (pathDisplay && data.config_path) {
        pathDisplay.textContent = data.config_path;
        pathDisplay.title = data.config_path;
      }
    } catch (err) {
      card.innerHTML = '<div class="data-stats-loading">Failed to load database statistics</div>';
    }
  }

  /**
   * Export configuration to JSON file
   */
  async _exportConfig() {
    try {
      const response = await fetch("/api/config");
      if (!response.ok) throw new Error("Failed to fetch config");

      const config = await response.json();

      // Remove sensitive data
      const exportData = { ...config };
      delete exportData.wallet_encrypted;
      delete exportData.wallet_nonce;

      const dataStr = JSON.stringify(exportData, null, 2);
      const blob = new Blob([dataStr], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `screenerbot-config-${new Date().toISOString().split("T")[0]}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      Utils.showToast("Configuration exported successfully", "success");
    } catch (err) {
      Utils.showToast("Failed to export config: " + err.message, "error");
    }
  }

  /**
   * Import configuration from JSON file
   */
  async _importConfig(event) {
    const file = event.target.files?.[0];
    if (!file) return;

    // Reset file input for future imports
    event.target.value = "";

    try {
      const text = await file.text();
      const imported = JSON.parse(text);

      // Confirm import
      const confirmResult = await ConfirmationDialog.show({
        title: "Import Configuration",
        message:
          "Import this configuration? Current settings will be overwritten. Wallet credentials will be preserved.",
        confirmLabel: "Import",
        cancelLabel: "Cancel",
        variant: "warning",
      });
      if (!confirmResult.confirmed) return;

      // Import each section separately to preserve wallet
      const sections = [
        "trader",
        "positions",
        "filtering",
        "swaps",
        "tokens",
        "rpc",
        "sol_price",
        "events",
        "services",
        "monitoring",
        "ohlcv",
        "gui",
      ];

      for (const section of sections) {
        if (imported[section]) {
          const response = await fetch(`/api/config/${section}`, {
            method: "PATCH",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(imported[section]),
          });

          if (!response.ok) {
            console.warn(`Failed to import ${section} section`);
          }
        }
      }

      Utils.showToast(
        "Configuration imported successfully. Some changes may require restart.",
        "success"
      );
    } catch (err) {
      Utils.showToast("Failed to import config: " + err.message, "error");
    }
  }

  /**
   * Reset configuration to defaults
   */
  async _resetConfig() {
    const confirmResult = await ConfirmationDialog.show({
      title: "Reset Configuration",
      message:
        "Reset all settings to defaults? Your wallet credentials will be preserved, but all other settings will be reset.",
      confirmLabel: "Reset",
      cancelLabel: "Cancel",
      variant: "danger",
    });
    if (!confirmResult.confirmed) return;

    try {
      const response = await fetch("/api/config/reset", { method: "POST" });

      if (response.ok) {
        Utils.showToast("Configuration reset to defaults", "success");
      } else {
        const data = await response.json();
        Utils.showToast("Failed to reset config: " + (data.error || "Unknown error"), "error");
      }
    } catch (err) {
      Utils.showToast("Failed to reset config: " + err.message, "error");
    }
  }

  /**
   * Apply trading preset
   */
  async _applyPreset(presetName) {
    const presets = {
      conservative: {
        trader: {
          max_open_positions: 2,
          trade_size_sol: 0.005,
          roi_target_percent: 15,
        },
        filtering: {
          min_liquidity_usd: 10000,
        },
        positions: {
          stop_loss_percent: 25,
        },
      },
      moderate: {
        trader: {
          max_open_positions: 5,
          trade_size_sol: 0.01,
          roi_target_percent: 20,
        },
        filtering: {
          min_liquidity_usd: 5000,
        },
        positions: {
          stop_loss_percent: 20,
        },
      },
      aggressive: {
        trader: {
          max_open_positions: 10,
          trade_size_sol: 0.02,
          roi_target_percent: 30,
        },
        filtering: {
          min_liquidity_usd: 1000,
        },
        positions: {
          stop_loss_percent: 15,
        },
      },
    };

    const preset = presets[presetName];
    if (!preset) {
      Utils.showToast("Unknown preset", "error");
      return;
    }

    const presetDisplayName = presetName.charAt(0).toUpperCase() + presetName.slice(1);
    const confirmResult = await ConfirmationDialog.show({
      title: "Apply Trading Preset",
      message: `Apply ${presetDisplayName} trading preset? This will update your trader, filtering, and position settings.`,
      confirmLabel: "Apply",
      cancelLabel: "Cancel",
      variant: "warning",
    });
    if (!confirmResult.confirmed) return;

    try {
      // Apply each section
      for (const [section, values] of Object.entries(preset)) {
        const response = await fetch(`/api/config/${section}`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(values),
        });

        if (!response.ok) {
          console.warn(`Failed to apply ${section} preset`);
        }
      }

      Utils.showToast(`${presetDisplayName} preset applied successfully`, "success");
    } catch (err) {
      Utils.showToast("Failed to apply preset: " + err.message, "error");
    }
  }

  // ==========================================================================
  // SECURITY TAB - Lockscreen settings
  // ==========================================================================

  /**
   * Load and build Security tab content (async because we need to fetch status)
   */
  async _loadSecurityTab(content) {
    content.innerHTML =
      '<div class="settings-loading"><i class="icon-loader spin"></i> Loading security settings...</div>';

    try {
      // Fetch lockscreen status from API
      const response = await fetch("/api/lockscreen/status");
      let status = {
        enabled: false,
        has_password: false,
        password_type: "pin6",
        auto_lock_timeout_minutes: 0,
        lock_on_blur: false,
      };

      if (response.ok) {
        const data = await response.json();
        status = data.data || data;
      }

      // Also fetch TOTP status
      const totpResponse = await fetch("/api/auth/totp/status");
      let totpStatus = { enabled: false };
      if (totpResponse.ok) {
        const totpData = await totpResponse.json();
        totpStatus = totpData.data || totpData;
      }
      status.totp_enabled = totpStatus.enabled;

      content.innerHTML = this._buildSecurityTab(status);
      this._attachSecurityHandlers(content, status);
      enhanceAllSelects(content);
    } catch (error) {
      console.error("[Settings] Failed to load security status:", error);
      content.innerHTML = '<div class="settings-error">Failed to load security settings</div>';
    }
  }

  /**
   * Build Security tab HTML
   */
  _buildSecurityTab(status) {
    const hasPassword = status.has_password;
    const isEnabled = status.enabled && hasPassword;
    const passwordType = status.password_type || "pin6";
    const autoLockSecs = status.auto_lock_timeout_secs || 0;
    const lockOnBlur = status.lock_on_blur || false;
    const totpEnabled = status.totp_enabled || false;

    // Password type display name
    const typeNames = {
      pin4: "4-Digit PIN",
      pin6: "6-Digit PIN",
      text: "Text Password",
    };
    const typeName = typeNames[passwordType] || "Not Set";

    return `
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-lock"></i>
          Dashboard Lockscreen
        </h3>
        <p class="settings-section-description">
          Protect your dashboard with a PIN or password. The lockscreen will appear when triggered, requiring authentication to continue.
        </p>

        <div class="settings-group">
          <!-- Enable/Disable Lockscreen -->
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Enable Lockscreen</label>
              <span class="settings-field-hint">Protect your dashboard with password authentication</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="securityEnableLockscreen" ${isEnabled ? "checked" : ""} ${!hasPassword ? "disabled" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <!-- Password Status -->
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Password Status</label>
              <span class="settings-field-hint">
                ${hasPassword ? `Current: ${typeName}` : "No password set"}
              </span>
            </div>
            <div class="settings-field-control security-password-actions">
              ${
                hasPassword
                  ? `
                <button class="btn btn-secondary btn-sm" id="securityChangePasswordBtn">
                  <i class="icon-pencil"></i> Change
                </button>
                <button class="btn btn-warning btn-sm" id="securityRemovePasswordBtn">
                  <i class="icon-trash-2"></i> Remove
                </button>
              `
                  : `
                <button class="btn btn-primary btn-sm" id="securitySetPasswordBtn">
                  <i class="icon-key"></i> Set Password
                </button>
              `
              }
            </div>
          </div>

          <!-- Auto-Lock Timeout -->
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Auto-Lock After Inactivity</label>
              <span class="settings-field-hint">Automatically lock after period of no activity</span>
            </div>
            <div class="settings-field-control">
              <select id="securityAutoLockTimeout" class="settings-select" data-custom-select ${!hasPassword ? "disabled" : ""}>
                <option value="0" ${autoLockSecs === 0 ? "selected" : ""}>Never</option>
                <option value="60" ${autoLockSecs === 60 ? "selected" : ""}>1 minute</option>
                <option value="300" ${autoLockSecs === 300 ? "selected" : ""}>5 minutes</option>
                <option value="900" ${autoLockSecs === 900 ? "selected" : ""}>15 minutes</option>
                <option value="1800" ${autoLockSecs === 1800 ? "selected" : ""}>30 minutes</option>
                <option value="3600" ${autoLockSecs === 3600 ? "selected" : ""}>1 hour</option>
              </select>
            </div>
          </div>

          <!-- Lock on Blur -->
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Lock When Window Loses Focus</label>
              <span class="settings-field-hint">Automatically lock when you switch to another application</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="securityLockOnBlur" ${lockOnBlur ? "checked" : ""} ${!hasPassword ? "disabled" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>
        </div>
      </div>

      <!-- Lock Now Action -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-shield"></i>
          Quick Actions
        </h3>
        
        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Lock Dashboard Now</label>
              <span class="settings-field-hint">Immediately lock the dashboard</span>
            </div>
            <div class="settings-field-control">
              <button class="btn btn-primary btn-sm" id="securityLockNowBtn" ${!hasPassword || !isEnabled ? "disabled" : ""}>
                <i class="icon-lock"></i> Lock Now
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Two-Factor Authentication -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-shield-check"></i>
          Two-Factor Authentication
        </h3>
        <p class="settings-section-description">
          Add an extra layer of security using an authenticator app (Google Authenticator, Authy, etc.)
        </p>

        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label>2FA Status</label>
              <span class="settings-field-hint">
                ${totpEnabled ? "Two-factor authentication is enabled" : "Not configured"}
              </span>
            </div>
            <div class="settings-field-control">
              ${
                totpEnabled
                  ? `<button class="btn btn-warning btn-sm" id="securityDisable2FABtn" ${!hasPassword ? "disabled" : ""}>
                  <i class="icon-x"></i> Disable 2FA
                </button>`
                  : `<button class="btn btn-primary btn-sm" id="securityEnable2FABtn" ${!hasPassword ? "disabled" : ""}>
                  <i class="icon-shield-check"></i> Enable 2FA
                </button>`
              }
            </div>
          </div>
        </div>
      </div>

      <!-- Password Setup Modal Container -->
      <div id="securityPasswordModal" class="security-modal" style="display: none;">
        <div class="security-modal-backdrop"></div>
        <div class="security-modal-content">
          <div class="security-modal-header">
            <h3 id="securityModalTitle">Set Password</h3>
            <button class="security-modal-close" id="securityModalClose">
              <i class="icon-x"></i>
            </button>
          </div>
          <div class="security-modal-body" id="securityModalBody">
            <!-- Content injected dynamically -->
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Attach handlers for Security tab
   */
  _attachSecurityHandlers(content, status) {
    // Enable/disable toggle
    const enableToggle = content.querySelector("#securityEnableLockscreen");
    if (enableToggle) {
      enableToggle.addEventListener("change", async (e) => {
        await this._updateSecuritySetting("enabled", e.target.checked);
      });
    }

    // Auto-lock timeout
    const timeoutSelect = content.querySelector("#securityAutoLockTimeout");
    if (timeoutSelect) {
      timeoutSelect.addEventListener("change", async (e) => {
        await this._updateSecuritySetting("auto_lock_timeout_secs", parseInt(e.target.value, 10));
      });
    }

    // Lock on blur toggle
    const blurToggle = content.querySelector("#securityLockOnBlur");
    if (blurToggle) {
      blurToggle.addEventListener("change", async (e) => {
        await this._updateSecuritySetting("lock_on_blur", e.target.checked);
      });
    }

    // Set password button
    const setBtn = content.querySelector("#securitySetPasswordBtn");
    if (setBtn) {
      setBtn.addEventListener("click", () => this._showPasswordModal("set", content));
    }

    // Change password button
    const changeBtn = content.querySelector("#securityChangePasswordBtn");
    if (changeBtn) {
      changeBtn.addEventListener("click", () => this._showPasswordModal("change", content));
    }

    // Remove password button
    const removeBtn = content.querySelector("#securityRemovePasswordBtn");
    if (removeBtn) {
      removeBtn.addEventListener("click", () => this._removePassword(content));
    }

    // Lock now button
    const lockBtn = content.querySelector("#securityLockNowBtn");
    if (lockBtn) {
      lockBtn.addEventListener("click", () => {
        if (window.Lockscreen && window.Lockscreen.lockNow()) {
          this.close();
        } else {
          Utils.showToast("Cannot lock - lockscreen not ready", "error");
        }
      });
    }

    // Enable 2FA button
    const enable2FABtn = content.querySelector("#securityEnable2FABtn");
    if (enable2FABtn) {
      enable2FABtn.addEventListener("click", () => this._showTotpSetupModal(content));
    }

    // Disable 2FA button
    const disable2FABtn = content.querySelector("#securityDisable2FABtn");
    if (disable2FABtn) {
      disable2FABtn.addEventListener("click", () => this._disableTotp(content));
    }
  }

  /**
   * Update a security setting via API
   */
  async _updateSecuritySetting(key, value) {
    try {
      const response = await fetch("/api/lockscreen/settings", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ [key]: value }),
      });

      if (response.ok) {
        Utils.showToast("Security setting updated", "success");
        // Update lockscreen controller if available
        if (window.Lockscreen) {
          window.Lockscreen.loadStatus();
        }
      } else {
        const data = await response.json();
        Utils.showToast(data.message || "Failed to update setting", "error");
      }
    } catch (error) {
      Utils.showToast("Failed to update setting: " + error.message, "error");
    }
  }

  /**
   * Show password setup/change modal
   */
  _showPasswordModal(mode, content) {
    const modal = content.querySelector("#securityPasswordModal");
    const title = content.querySelector("#securityModalTitle");
    const body = content.querySelector("#securityModalBody");
    const closeBtn = content.querySelector("#securityModalClose");
    const backdrop = modal.querySelector(".security-modal-backdrop");

    if (!modal || !body) return;

    title.textContent = mode === "set" ? "Set Password" : "Change Password";

    body.innerHTML = `
      <div class="security-form">
        ${
          mode === "change"
            ? `
          <div class="security-form-group">
            <label>Current Password</label>
            <input type="password" id="securityCurrentPassword" class="settings-input" placeholder="Enter current password" />
          </div>
        `
            : ""
        }
        
        <div class="security-form-group">
          <label>Password Type</label>
          <select id="securityPasswordType" class="settings-select" data-custom-select>
            <option value="pin4">4-Digit PIN</option>
            <option value="pin6" selected>6-Digit PIN</option>
            <option value="text">Text Password</option>
          </select>
        </div>

        <div class="security-form-group">
          <label>New Password</label>
          <input type="password" id="securityNewPassword" class="settings-input" placeholder="Enter new password" />
        </div>

        <div class="security-form-group">
          <label>Confirm Password</label>
          <input type="password" id="securityConfirmPassword" class="settings-input" placeholder="Confirm password" />
        </div>

        <div class="security-form-actions">
          <button class="btn btn-secondary" id="securityCancelBtn">Cancel</button>
          <button class="btn btn-primary" id="securitySavePasswordBtn">
            <i class="icon-check"></i> ${mode === "set" ? "Set Password" : "Update Password"}
          </button>
        </div>
      </div>
    `;

    modal.style.display = "flex";
    enhanceAllSelects(body);

    // Close modal function with keyboard listener cleanup
    let handleKeydown;
    const closeModal = () => {
      if (handleKeydown) {
        document.removeEventListener("keydown", handleKeydown);
      }
      modal.style.display = "none";
    };

    // Keyboard handler for Escape
    handleKeydown = (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        closeModal();
      }
    };
    document.addEventListener("keydown", handleKeydown);

    closeBtn.onclick = closeModal;
    backdrop.onclick = closeModal;
    body.querySelector("#securityCancelBtn").onclick = closeModal;

    // Type change handler - validate input
    const typeSelect = body.querySelector("#securityPasswordType");
    const newPasswordInput = body.querySelector("#securityNewPassword");

    typeSelect.addEventListener("change", () => {
      const type = typeSelect.value;
      if (type === "pin4" || type === "pin6") {
        newPasswordInput.type = "password";
        newPasswordInput.inputMode = "numeric";
        newPasswordInput.pattern = type === "pin4" ? "[0-9]{4}" : "[0-9]{6}";
        newPasswordInput.placeholder = type === "pin4" ? "Enter 4-digit PIN" : "Enter 6-digit PIN";
      } else {
        newPasswordInput.type = "password";
        newPasswordInput.inputMode = "text";
        newPasswordInput.pattern = "";
        newPasswordInput.placeholder = "Enter password";
      }
    });

    // Save handler
    body.querySelector("#securitySavePasswordBtn").onclick = async () => {
      const passwordType = typeSelect.value;
      const newPassword = newPasswordInput.value;
      const confirmPassword = body.querySelector("#securityConfirmPassword").value;
      const currentPassword =
        mode === "change" ? body.querySelector("#securityCurrentPassword")?.value : null;

      // Validation
      if (!newPassword) {
        Utils.showToast("Please enter a password", "error");
        return;
      }

      if (newPassword !== confirmPassword) {
        Utils.showToast("Passwords do not match", "error");
        return;
      }

      // Validate PIN format
      if (passwordType === "pin4" && !/^\d{4}$/.test(newPassword)) {
        Utils.showToast("PIN must be exactly 4 digits", "error");
        return;
      }
      if (passwordType === "pin6" && !/^\d{6}$/.test(newPassword)) {
        Utils.showToast("PIN must be exactly 6 digits", "error");
        return;
      }
      if (passwordType === "text" && newPassword.length < 4) {
        Utils.showToast("Password must be at least 4 characters", "error");
        return;
      }

      try {
        const payload = {
          password_type: passwordType,
          new_password: newPassword,
        };
        if (currentPassword) {
          payload.current_password = currentPassword;
        }

        const response = await fetch("/api/lockscreen/set-password", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });

        if (response.ok) {
          Utils.showToast("Password saved successfully", "success");
          closeModal();
          // Reload security tab
          this._loadSecurityTab(content);
          // Update lockscreen controller
          if (window.Lockscreen) {
            window.Lockscreen.loadStatus();
          }
        } else {
          const data = await response.json();
          Utils.showToast(data.message || "Failed to save password", "error");
        }
      } catch (error) {
        Utils.showToast("Failed to save password: " + error.message, "error");
      }
    };
  }

  /**
   * Remove password - shows modal to confirm with current password
   */
  _removePassword(content) {
    const modal = content.querySelector("#securityPasswordModal");
    const title = content.querySelector("#securityModalTitle");
    const body = content.querySelector("#securityModalBody");
    const closeBtn = content.querySelector("#securityModalClose");
    const backdrop = modal.querySelector(".security-modal-backdrop");

    if (!modal || !body) return;

    title.textContent = "Remove Password";

    body.innerHTML = `
      <div class="security-form">
        <p style="color: var(--text-secondary); margin-bottom: 16px;">
          Enter your current password to remove lockscreen protection.
        </p>
        
        <div class="security-form-group">
          <label>Current Password</label>
          <input type="password" id="securityCurrentPasswordRemove" class="settings-input" placeholder="Enter current password" autofocus />
        </div>

        <div class="security-form-actions">
          <button class="btn btn-secondary" id="securityCancelRemoveBtn">Cancel</button>
          <button class="btn btn-warning" id="securityConfirmRemoveBtn">
            <i class="icon-trash-2"></i> Remove Password
          </button>
        </div>
      </div>
    `;

    modal.style.display = "flex";

    // Focus the password input
    setTimeout(() => {
      const input = body.querySelector("#securityCurrentPasswordRemove");
      if (input) input.focus();
    }, 100);

    // Store reference for cleanup
    const passwordInput = body.querySelector("#securityCurrentPasswordRemove");

    // Keyboard handler for Enter and Escape
    const handleKeydown = (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        body.querySelector("#securityConfirmRemoveBtn").click();
      } else if (e.key === "Escape") {
        e.preventDefault();
        closeModal();
      }
    };
    passwordInput.addEventListener("keydown", handleKeydown);

    // Close handlers with cleanup
    const closeModal = () => {
      passwordInput.removeEventListener("keydown", handleKeydown);
      modal.style.display = "none";
    };

    closeBtn.onclick = closeModal;
    backdrop.onclick = closeModal;
    body.querySelector("#securityCancelRemoveBtn").onclick = closeModal;

    // Confirm handler
    body.querySelector("#securityConfirmRemoveBtn").onclick = async () => {
      const currentPassword = body.querySelector("#securityCurrentPasswordRemove").value;

      if (!currentPassword) {
        Utils.showToast("Please enter your current password", "error");
        return;
      }

      try {
        const response = await fetch("/api/lockscreen/clear-password", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ current_password: currentPassword }),
        });

        if (response.ok) {
          Utils.showToast("Password removed", "success");
          closeModal();
          // Reload security tab
          this._loadSecurityTab(content);
          // Update lockscreen controller
          if (window.Lockscreen) {
            window.Lockscreen.loadStatus();
          }
        } else {
          const data = await response.json();
          Utils.showToast(data.message || "Failed to remove password", "error");
        }
      } catch (error) {
        Utils.showToast("Failed to remove password: " + error.message, "error");
      }
    };
  }

  /**
   * Show TOTP setup modal with QR code and verification
   */
  async _showTotpSetupModal(content) {
    const modal = content.querySelector("#securityPasswordModal");
    const title = content.querySelector("#securityModalTitle");
    const body = content.querySelector("#securityModalBody");
    const closeBtn = content.querySelector("#securityModalClose");
    const backdrop = modal.querySelector(".security-modal-backdrop");

    if (!modal || !body) return;

    title.textContent = "Enable Two-Factor Authentication";
    body.innerHTML = `
      <div class="totp-setup-step" id="totpStep1">
        <p style="color: var(--text-secondary); margin-bottom: 1rem;">Enter your password to continue:</p>
        <div class="security-form-group">
          <input type="password" id="totpSetupPassword" class="settings-input" placeholder="Enter password" autocomplete="current-password">
        </div>
        <div class="totp-setup-actions" style="display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem;">
          <button class="btn btn-secondary btn-sm" id="totpCancelBtn">Cancel</button>
          <button class="btn btn-primary btn-sm" id="totpContinueBtn">Continue</button>
        </div>
      </div>
      <div class="totp-setup-step" id="totpStep2" style="display: none;">
        <div id="totpQrContainer" style="text-align: center; margin: 1rem 0;"></div>
        <div class="totp-manual-entry" style="margin: 1rem 0;">
          <label style="font-size: 0.75rem; color: var(--text-secondary);">Manual entry code:</label>
          <code id="totpSecretCode" style="display: block; padding: 0.5rem; background: var(--bg-tertiary); border-radius: 4px; margin-top: 0.25rem; word-break: break-all; font-family: monospace;"></code>
        </div>
        <p style="margin: 1rem 0; color: var(--text-secondary);">Enter the 6-digit code from your authenticator app:</p>
        <div class="security-form-group">
          <input type="text" id="totpVerifyCode" class="settings-input" placeholder="000000" maxlength="6" pattern="[0-9]{6}" style="text-align: center; font-size: 1.25rem; letter-spacing: 0.25em;">
        </div>
        <div class="totp-setup-actions" style="display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem;">
          <button class="btn btn-secondary btn-sm" id="totpBackBtn">Back</button>
          <button class="btn btn-primary btn-sm" id="totpVerifyBtn">Verify & Enable</button>
        </div>
      </div>
    `;

    modal.style.display = "flex";

    let currentSecret = "";

    // Close handlers
    const closeModal = () => {
      modal.style.display = "none";
    };

    closeBtn.onclick = closeModal;
    backdrop.onclick = closeModal;

    // Cancel button
    body.querySelector("#totpCancelBtn")?.addEventListener("click", closeModal);

    // Continue to step 2
    body.querySelector("#totpContinueBtn")?.addEventListener("click", async () => {
      const password = body.querySelector("#totpSetupPassword")?.value;
      if (!password) {
        Utils.showToast("Please enter your password", "error");
        return;
      }

      try {
        const response = await fetch("/api/auth/totp/setup", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ password }),
        });

        const data = await response.json();
        if (!response.ok) {
          Utils.showToast(data.error || data.message || "Failed to setup 2FA", "error");
          return;
        }

        const result = data.data || data;
        currentSecret = result.secret;

        // Show QR code
        const qrContainer = body.querySelector("#totpQrContainer");
        qrContainer.innerHTML = `<img src="${result.qr_code}" alt="TOTP QR Code" style="max-width: 200px; height: auto; border-radius: 8px;">`;

        // Show manual code
        body.querySelector("#totpSecretCode").textContent = result.secret;

        // Switch to step 2
        body.querySelector("#totpStep1").style.display = "none";
        body.querySelector("#totpStep2").style.display = "block";
      } catch (err) {
        Utils.showToast("Failed to setup 2FA", "error");
      }
    });

    // Back button
    body.querySelector("#totpBackBtn")?.addEventListener("click", () => {
      body.querySelector("#totpStep1").style.display = "block";
      body.querySelector("#totpStep2").style.display = "none";
    });

    // Verify button
    body.querySelector("#totpVerifyBtn")?.addEventListener("click", async () => {
      const code = body.querySelector("#totpVerifyCode")?.value;
      if (!code || code.length !== 6) {
        Utils.showToast("Please enter a 6-digit code", "error");
        return;
      }

      try {
        const response = await fetch("/api/auth/totp/verify-setup", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ secret: currentSecret, code }),
        });

        const data = await response.json();
        if (!response.ok) {
          Utils.showToast(data.error || data.message || "Invalid code", "error");
          return;
        }

        Utils.showToast("Two-factor authentication enabled!", "success");
        closeModal();
        // Reload security tab
        await this._loadSecurityTab(content);
      } catch (err) {
        Utils.showToast("Failed to verify code", "error");
      }
    });
  }

  /**
   * Disable TOTP 2FA with password confirmation
   */
  async _disableTotp(content) {
    const modal = content.querySelector("#securityPasswordModal");
    const title = content.querySelector("#securityModalTitle");
    const body = content.querySelector("#securityModalBody");
    const closeBtn = content.querySelector("#securityModalClose");
    const backdrop = modal.querySelector(".security-modal-backdrop");

    if (!modal || !body) return;

    title.textContent = "Disable Two-Factor Authentication";
    body.innerHTML = `
      <div class="security-form">
        <p style="color: var(--text-secondary); margin-bottom: 1rem;">Enter your password to disable 2FA:</p>
        <div class="security-form-group">
          <input type="password" id="totpDisablePassword" class="settings-input" placeholder="Enter password" autocomplete="current-password">
        </div>
        <div class="totp-setup-actions" style="display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem;">
          <button class="btn btn-secondary btn-sm" id="totpDisableCancelBtn">Cancel</button>
          <button class="btn btn-warning btn-sm" id="totpDisableConfirmBtn">Disable 2FA</button>
        </div>
      </div>
    `;

    modal.style.display = "flex";

    // Close handlers
    const closeModal = () => {
      modal.style.display = "none";
    };

    closeBtn.onclick = closeModal;
    backdrop.onclick = closeModal;

    body.querySelector("#totpDisableCancelBtn")?.addEventListener("click", closeModal);

    body.querySelector("#totpDisableConfirmBtn")?.addEventListener("click", async () => {
      const password = body.querySelector("#totpDisablePassword")?.value;
      if (!password) {
        Utils.showToast("Please enter your password", "error");
        return;
      }

      try {
        const response = await fetch("/api/auth/totp/disable", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ password }),
        });

        const data = await response.json();
        if (!response.ok) {
          Utils.showToast(data.error || data.message || "Failed to disable 2FA", "error");
          return;
        }

        Utils.showToast("Two-factor authentication disabled", "success");
        closeModal();
        await this._loadSecurityTab(content);
      } catch (err) {
        Utils.showToast("Failed to disable 2FA", "error");
      }
    });
  }

  /**
   * Format number compactly (1.2K, 3.4M, etc)
   */
  _formatCompactNumber(num) {
    if (num >= 1000000) return (num / 1000000).toFixed(1) + "M";
    if (num >= 1000) return (num / 1000).toFixed(1) + "K";
    return String(num);
  }

  /**
   * Build Updates tab content - Modern design with version history
   */
  _buildUpdatesTab() {
    const { version, platform } = this.versionInfo;
    const state = globalUpdateState;

    // Build status section based on current state
    let statusSection = "";

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
                <span class="detail-value">${platform || "Unknown"}</span>
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
                <span class="detail-value">${platform || "Unknown"}</span>
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
            <i class="icon-triangle-alert"></i>
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

    let actionContent = "";

    if (isDownloaded) {
      actionContent = `
        <div class="download-success">
          <div class="success-badge">
            <i class="icon-circle-check"></i>
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
              <span id="download-speed-text"></span>
              <span id="download-percent-text">${Math.round(state.progress)}%</span>
            </span>
          </div>
          <div class="progress-track">
            <div class="progress-fill" id="downloadProgressBar" style="width: ${state.progress}%">
              <div class="progress-glow"></div>
            </div>
          </div>
          <div class="progress-footer">
            <span id="downloadSizeText">${fileSize ? `0 / ${fileSize}` : ""}</span>
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
            ${fileSize ? `<span class="btn-meta">(${fileSize})</span>` : ""}
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
          ${
            info.release_notes
              ? `
            <div class="release-notes-preview">
              <h4>What's New</h4>
              <div class="notes-text">${Utils.escapeHtml(info.release_notes)}</div>
            </div>
          `
              : ""
          }
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
            <i class="icon-circle-check"></i>
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
    const platform = this.versionInfo.platform || "";
    if (platform.toLowerCase().includes("macos") || platform.toLowerCase().includes("darwin")) {
      return "The installer will open. Drag ScreenerBot to your Applications folder.";
    } else if (platform.toLowerCase().includes("windows")) {
      return "The installer will guide you through the update process.";
    } else if (platform.toLowerCase().includes("linux")) {
      return "Run the AppImage or install the .deb package to update.";
    }
    return "Follow the installer instructions to complete the update.";
  }

  /**
   * Format bytes to human readable size
   */
  _formatBytes(bytes) {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  }

  /**
   * Get styles for Updates tab (styles now in settings_dialog.css)
   */
  _getUpdatesStyles() {
    return "";
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

    const checkBtn = this.dialogEl.querySelector("#checkUpdatesBtn");
    const retryBtn = this.dialogEl.querySelector("#retryUpdateBtn");
    const downloadBtn = this.dialogEl.querySelector("#downloadUpdateBtn");
    const installBtn = this.dialogEl.querySelector("#installUpdateBtn");

    // Check / Retry Handler
    const handleCheck = async () => {
      globalUpdateState.checking = true;
      globalUpdateState.error = null;
      this._updateUpdatesTabUI();

      try {
        // Call the check API
        const response = await fetch("/api/updates/check");
        const data = await response.json();

        globalUpdateState.checking = false;

        if (data.update_available) {
          globalUpdateState.available = true;
          globalUpdateState.info = data.update; // API returns 'update' not 'update_info'
        } else {
          globalUpdateState.available = false;
          globalUpdateState.info = null;
        }
      } catch (err) {
        console.error("Update check failed:", err);
        globalUpdateState.checking = false;
        globalUpdateState.error = err.message || "Failed to check for updates";
      }

      this._updateUpdatesBadge();
      this._updateUpdatesTabUI();
    };

    if (checkBtn) checkBtn.addEventListener("click", handleCheck);
    if (retryBtn) retryBtn.addEventListener("click", handleCheck);

    // Download Handler
    if (downloadBtn) {
      downloadBtn.addEventListener("click", async () => {
        if (!globalUpdateState.info) return;

        globalUpdateState.downloading = true;
        globalUpdateState.progress = 0;
        globalUpdateState.downloadStartTime = Date.now();
        globalUpdateState.downloadedBytes = 0;
        this._updateUpdatesTabUI();

        try {
          // Start download
          const response = await fetch("/api/updates/download", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ version: globalUpdateState.info.version }),
          });

          if (!response.ok) throw new Error("Failed to start download");

          // Start polling for progress
          this._startDownloadPoller();
        } catch (err) {
          console.error("Download start failed:", err);
          globalUpdateState.downloading = false;
          globalUpdateState.error = err.message;
          this._updateUpdatesTabUI();
        }
      });
    }

    // Install Handler
    if (installBtn) {
      installBtn.addEventListener("click", async () => {
        const confirmResult = await ConfirmationDialog.show({
          title: "Install Update",
          message:
            "ScreenerBot will install the update and close. The installer will launch automatically. Continue?",
          confirmLabel: "Install",
          cancelLabel: "Cancel",
          variant: "warning",
        });
        if (!confirmResult.confirmed) return;

        installBtn.disabled = true;
        const originalText = installBtn.innerHTML;
        installBtn.innerHTML = '<i class="icon-loader spinning"></i><span>Installing...</span>';

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
          {
            name: "Electron",
            license: "MIT",
            url: "https://www.electronjs.org/",
            desc: "Desktop application framework",
          },
          {
            name: "Tokio",
            license: "MIT",
            url: "https://tokio.rs/",
            desc: "Async runtime for Rust",
          },
          {
            name: "Axum",
            license: "MIT",
            url: "https://github.com/tokio-rs/axum",
            desc: "Web server framework",
          },
          {
            name: "Tower",
            license: "MIT",
            url: "https://github.com/tower-rs/tower",
            desc: "Service abstractions",
          },
          { name: "Hyper", license: "MIT", url: "https://hyper.rs/", desc: "HTTP implementation" },
        ],
      },
      {
        category: "Solana Blockchain",
        items: [
          {
            name: "solana-sdk",
            license: "Apache-2.0",
            url: "https://github.com/anza-xyz/agave",
            desc: "Solana SDK core",
          },
          {
            name: "solana-client",
            license: "Apache-2.0",
            url: "https://github.com/anza-xyz/agave",
            desc: "RPC client",
          },
          {
            name: "solana-program",
            license: "Apache-2.0",
            url: "https://github.com/anza-xyz/agave",
            desc: "Program library",
          },
          {
            name: "spl-token",
            license: "Apache-2.0",
            url: "https://github.com/solana-labs/solana-program-library",
            desc: "SPL Token program",
          },
          {
            name: "spl-token-2022",
            license: "Apache-2.0",
            url: "https://github.com/solana-labs/solana-program-library",
            desc: "Token-2022 extensions",
          },
          {
            name: "spl-associated-token-account",
            license: "Apache-2.0",
            url: "https://github.com/solana-labs/solana-program-library",
            desc: "Associated token accounts",
          },
        ],
      },
      {
        category: "Data & Storage",
        items: [
          {
            name: "SQLite",
            license: "Public Domain",
            url: "https://sqlite.org/",
            desc: "Embedded database engine",
          },
          {
            name: "rusqlite",
            license: "MIT",
            url: "https://github.com/rusqlite/rusqlite",
            desc: "SQLite Rust bindings",
          },
          {
            name: "r2d2",
            license: "MIT / Apache-2.0",
            url: "https://github.com/sfackler/r2d2",
            desc: "Database connection pool",
          },
          {
            name: "Serde",
            license: "MIT / Apache-2.0",
            url: "https://serde.rs/",
            desc: "Serialization framework",
          },
          {
            name: "TOML",
            license: "MIT / Apache-2.0",
            url: "https://github.com/toml-rs/toml",
            desc: "Configuration parsing",
          },
        ],
      },
      {
        category: "Networking",
        items: [
          {
            name: "reqwest",
            license: "MIT / Apache-2.0",
            url: "https://github.com/seanmonstar/reqwest",
            desc: "HTTP client",
          },
          {
            name: "tokio-tungstenite",
            license: "MIT",
            url: "https://github.com/snapview/tokio-tungstenite",
            desc: "WebSocket client",
          },
          {
            name: "RustLS",
            license: "MIT / Apache-2.0",
            url: "https://github.com/rustls/rustls",
            desc: "TLS implementation",
          },
        ],
      },
      {
        category: "Cryptography & Encoding",
        items: [
          {
            name: "BLAKE3",
            license: "CC0 / Apache-2.0",
            url: "https://github.com/BLAKE3-team/BLAKE3",
            desc: "Hash function",
          },
          {
            name: "SHA-2",
            license: "MIT / Apache-2.0",
            url: "https://github.com/RustCrypto/hashes",
            desc: "SHA-256/512 hashing",
          },
          {
            name: "bs58",
            license: "MIT / Apache-2.0",
            url: "https://github.com/Nullus157/bs58-rs",
            desc: "Base58 encoding",
          },
          {
            name: "base64",
            license: "MIT / Apache-2.0",
            url: "https://github.com/marshallpierce/rust-base64",
            desc: "Base64 encoding",
          },
        ],
      },
      {
        category: "UI Assets",
        items: [
          {
            name: "Lucide Icons",
            license: "ISC",
            url: "https://lucide.dev/",
            desc: "Icon font library",
          },
          {
            name: "JetBrains Mono",
            license: "OFL-1.1",
            url: "https://www.jetbrains.com/lp/mono/",
            desc: "Monospace font",
          },
          {
            name: "Orbitron",
            license: "OFL-1.1",
            url: "https://fonts.google.com/specimen/Orbitron",
            desc: "Display font",
          },
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
        const statusRes = await fetch("/api/updates/status");
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
          let etaText = "";

          if (progress.downloaded_bytes && progress.total_bytes) {
            const bytesDiff =
              progress.downloaded_bytes - (lastProgress / 100) * progress.total_bytes;
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
            const bar = this.dialogEl.querySelector("#downloadProgressBar");
            const percentText = this.dialogEl.querySelector("#download-percent-text");
            const speedText = this.dialogEl.querySelector("#download-speed-text");
            const etaElement = this.dialogEl.querySelector("#downloadEtaText");
            const sizeText = this.dialogEl.querySelector("#downloadSizeText");

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
          throw new Error(progress.error || "Download failed");
        }
      } catch (err) {
        console.error("Download poll error:", err);
        globalUpdateState.downloading = false;
        globalUpdateState.error = err.message;
        if (this.downloadPoller) this.downloadPoller.stop();
        this._updateUpdatesTabUI();
      }
    }, 1000);

    this.downloadPoller.start();
  }

  // ===========================================================================
  // TELEGRAM TAB
  // ===========================================================================

  /**
   * Load Telegram tab content
   */
  async _loadTelegramTab(content) {
    content.innerHTML =
      '<div class="settings-loading"><i class="icon-loader spin"></i> Loading Telegram settings...</div>';

    try {
      // Fetch telegram status from API
      const response = await fetch("/api/telegram/settings");
      let settings = {
        enabled: false,
        bot_token: "",
        chat_id: "",
        session_timeout_minutes: 30,
        notifications: {
          position_opened: true,
          position_closed: true,
          partial_exit: true,
          dca_executed: true,
          errors: true,
          startup_shutdown: true,
          filtering_alerts: true,
          trade_alerts: true,
          daily_summary: false,
        },
        commands_enabled: true,
        inline_actions: true,
        sessions: [],
      };

      if (response.ok) {
        const data = await response.json();
        settings = { ...settings, ...data };
      }

      content.innerHTML = this._buildTelegramTab(settings);
      this._attachTelegramHandlers(content, settings);

      // Load Password + TOTP authentication state
      await this._loadTelegramAuthState(content);
    } catch (error) {
      console.error("[Settings] Failed to load Telegram settings:", error);
      content.innerHTML = '<div class="settings-error">Failed to load Telegram settings</div>';
    }
  }

  /**
   * Build Telegram tab HTML
   */
  _buildTelegramTab(settings) {
    // Build sessions list HTML
    const sessionsHtml =
      (settings.sessions || []).length > 0
        ? (settings.sessions || [])
            .map(
              (s) => `
        <div class="session-item" data-session-id="${s.user_id}">
          <div class="session-info">
            <span class="session-user">${s.username || "Unknown"}</span>
            <span class="session-time">Active: ${Utils.formatDuration(s.created_at_secs * 1000)}</span>
          </div>
          <button class="btn btn-danger btn-sm session-revoke-btn" data-session-id="${s.user_id}">
            <i class="icon-x"></i> Revoke
          </button>
        </div>
      `
            )
            .join("")
        : '<div class="sessions-empty">No active sessions</div>';

    return `
      <!-- Connection Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-send"></i>
          Connection
        </h3>
        <p class="settings-section-description">
          Connect your Telegram bot to receive notifications and control ScreenerBot remotely.
        </p>

        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Enable Telegram</label>
              <span class="settings-field-hint">Enable Telegram bot integration</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="tgEnabled" ${settings.enabled ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Bot Token</label>
              <span class="settings-field-hint">${settings.bot_token && settings.bot_token.endsWith("...") ? '<i class="icon-circle-check" style="color: var(--success);"></i> Token saved' : "Get this from @BotFather on Telegram"}</span>
            </div>
            <div class="settings-field-control telegram-token-field">
              <input type="password" id="tgBotToken" class="settings-input" placeholder="${settings.bot_token && settings.bot_token.endsWith("...") ? "Token saved (enter new to change)" : "Enter bot token"}" value="" autocomplete="off">
              <button class="btn btn-secondary btn-sm btn-icon" id="tgToggleToken" title="Show/Hide">
                <i class="icon-eye"></i>
              </button>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Chat ID</label>
              <span class="settings-field-hint" id="tgChatIdHint">
                ${settings.chat_id ? `Connected to chat: ${settings.chat_id}` : "Discover your chat ID automatically"}
              </span>
            </div>
            <div class="settings-field-control" id="tgChatIdControl">
              ${
                settings.chat_id
                  ? `
                <span class="chat-id-display">
                  <code>${settings.chat_id}</code>
                  <button class="btn btn-secondary btn-sm" id="tgChangeChatBtn" title="Change">
                    <i class="icon-pencil"></i>
                  </button>
                </span>
              `
                  : `
                <button class="btn btn-primary btn-sm" id="tgDiscoverBtn">
                  <i class="icon-search"></i> Discover Chat ID
                </button>
              `
              }
            </div>
          </div>

          <!-- Discovery Section (hidden by default) -->
          <div class="settings-field discovery-section" id="tgDiscoverySection" style="display: none;">
            <div class="discovery-status" id="tgDiscoveryStatus">
              <div class="discovery-instructions">
                <div class="discovery-step">
                  <span class="step-number">1</span>
                  <span>Add your bot to a Telegram group, or start a direct chat with it</span>
                </div>
                <div class="discovery-step">
                  <span class="step-number">2</span>
                  <span>For groups: Check @BotFather → /mybots → [your bot] → Bot Settings → Group Privacy</span>
                </div>
                <div class="discovery-info-box">
                  <i class="icon-info"></i>
                  <div>
                    <strong>Privacy Mode OFF:</strong> Bot receives all group messages<br/>
                    <strong>Privacy Mode ON:</strong> Bot only receives messages when @mentioned
                  </div>
                </div>
                <div class="discovery-step">
                  <span class="step-number">3</span>
                  <span>Send any message (or @mention your bot if Privacy Mode is ON)</span>
                </div>
              </div>
              <div class="discovery-spinner">
                <i class="icon-loader spin"></i>
                <span>Listening for messages...</span>
              </div>
            </div>
            <div class="discovered-chats-list" id="tgDiscoveredChats">
              <!-- Discovered chats will appear here -->
            </div>
            <div class="discovery-actions">
              <button class="btn btn-secondary btn-sm" id="tgCancelDiscovery">
                <i class="icon-x"></i> Cancel
              </button>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Test Connection</label>
              <span class="settings-field-hint">Send a test message to verify configuration</span>
            </div>
            <div class="settings-field-control">
              <button class="btn btn-primary btn-sm" id="tgTestBtn">
                <i class="icon-send"></i> Send Test
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Authentication Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-shield"></i>
          Command Authentication
        </h3>
        <p class="settings-section-description">
          Telegram commands use the same 2FA as the dashboard lockscreen.
        </p>

        <div class="settings-group telegram-auth-section" id="tgAuthSection">
          <!-- Command Authentication Subsection -->
          <div class="telegram-auth-subsection">
            <div class="telegram-auth-header">
              <div class="telegram-auth-title">
                <i class="icon-shield"></i>
                <span>Command Authentication</span>
              </div>
              <div class="telegram-auth-status" id="tg-auth-status" role="status" aria-live="polite">
                <i class="icon-loader spin"></i> Loading...
              </div>
            </div>
            <div class="telegram-auth-content" id="tg-auth-content"></div>
          </div>

          <!-- Session Timeout -->
          <div class="telegram-auth-subsection">
            <div class="telegram-auth-header">
              <div class="telegram-auth-title">
                <i class="icon-clock"></i>
                <span>Session Timeout</span>
              </div>
            </div>
            <div class="telegram-auth-content">
              <div class="telegram-auth-row">
                <div class="telegram-auth-info">
                  <span>How long an authenticated session stays active</span>
                </div>
                <select id="tgSessionTimeout" class="settings-select" data-custom-select>
                  <option value="5" ${settings.session_timeout_minutes === 5 ? "selected" : ""}>5 minutes</option>
                  <option value="15" ${settings.session_timeout_minutes === 15 ? "selected" : ""}>15 minutes</option>
                  <option value="30" ${settings.session_timeout_minutes === 30 ? "selected" : ""}>30 minutes</option>
                  <option value="60" ${settings.session_timeout_minutes === 60 ? "selected" : ""}>1 hour</option>
                  <option value="120" ${settings.session_timeout_minutes === 120 ? "selected" : ""}>2 hours</option>
                  <option value="1440" ${settings.session_timeout_minutes === 1440 ? "selected" : ""}>24 hours</option>
                </select>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Sessions Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-users"></i>
          Active Sessions
        </h3>
        <div id="tgSessionsList" class="sessions-list">
          ${sessionsHtml}
        </div>
      </div>

      <!-- Notifications Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-bell"></i>
          Notification Settings
        </h3>
        <p class="settings-section-description">
          Choose which events trigger Telegram notifications.
        </p>

        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Position Opened</label>
              <span class="settings-field-hint">Notify when a new position is opened</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="tgNotifyOpened" ${settings.notifications?.position_opened !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Position Closed</label>
              <span class="settings-field-hint">Notify when a position is closed</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="tgNotifyClosed" ${settings.notifications?.position_closed !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Partial Exit</label>
              <span class="settings-field-hint">Notify on partial position exits</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="tgNotifyPartial" ${settings.notifications?.partial_exit !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>DCA Executed</label>
              <span class="settings-field-hint">Notify when DCA orders are executed</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="tgNotifyDca" ${settings.notifications?.dca_executed !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Errors</label>
              <span class="settings-field-hint">Notify on errors and failures</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="tgNotifyError" ${settings.notifications?.errors !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Startup/Shutdown</label>
              <span class="settings-field-hint">Notify when bot starts or stops</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="tgNotifyStartup" ${settings.notifications?.startup_shutdown !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Filtering Alerts</label>
              <span class="settings-field-hint">Notify when new tokens pass filtering criteria</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="tgNotifyFiltering" ${settings.notifications?.filtering_alerts !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Trade Alerts</label>
              <span class="settings-field-hint">Notify on significant trades for watched tokens</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="tgNotifyTradeAlerts" ${settings.notifications?.trade_alerts !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Daily Summary</label>
              <span class="settings-field-hint">Receive daily trading activity and P&L summary</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="tgNotifyDailySummary" ${settings.notifications?.daily_summary === true ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>
        </div>
      </div>

      <!-- Features Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class=icon-sliders-horizontal></i>
          Features
        </h3>
        <p class="settings-section-description">
          Configure Telegram bot capabilities.
        </p>

        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Enable Commands</label>
              <span class="settings-field-hint">Allow controlling the bot via Telegram commands</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="tgCommandsEnabled" ${settings.commands_enabled !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Require 2FA for Commands</label>
              <span class="settings-field-hint">When sessions expire, require 2FA code to reactivate. Uses lockscreen 2FA.</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="tgRequire2fa" ${settings.commands_require_2fa !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Inline Action Buttons</label>
              <span class="settings-field-hint">Show action buttons in notification messages</span>
            </div>
            <div class="settings-field-control">
              <label class="settings-toggle">
                <input type="checkbox" id="tgInlineActions" ${settings.inline_actions !== false ? "checked" : ""}>
                <span class="settings-toggle-slider"></span>
              </label>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Attach handlers for Telegram tab
   */
  _attachTelegramHandlers(content, settings) {
    // Helper to update telegram settings
    const updateSetting = async (key, value) => {
      try {
        const response = await fetch("/api/telegram/settings", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ [key]: value }),
        });

        if (response.ok) {
          Utils.showToast("Telegram setting updated", "success");
        } else {
          const data = await response.json();
          Utils.showToast(data.message || "Failed to update setting", "error");
        }
      } catch {
        Utils.showToast("Failed to update setting", "error");
      }
    };

    // Enable toggle
    const enableToggle = content.querySelector("#tgEnabled");
    if (enableToggle) {
      enableToggle.addEventListener("change", (e) => updateSetting("enabled", e.target.checked));
    }

    // Bot token field
    const tokenField = content.querySelector("#tgBotToken");
    const toggleTokenBtn = content.querySelector("#tgToggleToken");
    if (tokenField && toggleTokenBtn) {
      toggleTokenBtn.addEventListener("click", () => {
        const isPassword = tokenField.type === "password";
        tokenField.type = isPassword ? "text" : "password";
        toggleTokenBtn.querySelector("i").className = isPassword ? "icon-eye-off" : "icon-eye";
      });
      tokenField.addEventListener("change", (e) => updateSetting("bot_token", e.target.value));
    }

    // Chat ID discovery functionality
    const discoverBtn = content.querySelector("#tgDiscoverBtn");
    const changeChatBtn = content.querySelector("#tgChangeChatBtn");
    const discoverySection = content.querySelector("#tgDiscoverySection");
    const discoveredChatsEl = content.querySelector("#tgDiscoveredChats");
    const cancelDiscoveryBtn = content.querySelector("#tgCancelDiscovery");

    const startDiscovery = async () => {
      try {
        const response = await fetch("/api/telegram/discovery/start", { method: "POST" });
        if (!response.ok) {
          const data = await response.json();
          Utils.showToast(data.message || "Failed to start discovery", "error");
          return;
        }

        // Show discovery section
        if (discoverySection) discoverySection.style.display = "";
        if (discoverBtn) discoverBtn.style.display = "none";

        // Start polling for discovered chats
        this._discoveryPoller = setInterval(async () => {
          try {
            const chatsResponse = await fetch("/api/telegram/discovery/chats");
            if (chatsResponse.ok) {
              const data = await chatsResponse.json();
              renderDiscoveredChats(data.chats || []);
            }
          } catch (e) {
            console.error("Failed to fetch discovered chats:", e);
          }
        }, 2000);
      } catch {
        Utils.showToast("Failed to start discovery", "error");
      }
    };

    const stopDiscovery = async () => {
      if (this._discoveryPoller) {
        clearInterval(this._discoveryPoller);
        this._discoveryPoller = null;
      }
      try {
        await fetch("/api/telegram/discovery/stop", { method: "POST" });
      } catch {
        // Ignore stop errors
      }
      if (discoverySection) discoverySection.style.display = "none";
      if (discoverBtn) discoverBtn.style.display = "";
    };

    const renderDiscoveredChats = (chats) => {
      if (!discoveredChatsEl) return;

      if (chats.length === 0) {
        discoveredChatsEl.innerHTML = "";
        return;
      }

      discoveredChatsEl.innerHTML = chats
        .map(
          (chat) => `
        <div class="discovered-chat-item" data-chat-id="${chat.chat_id}">
          <div class="chat-info">
            <span class="chat-name">${chat.first_name || chat.username || "Unknown"}</span>
            <span class="chat-meta">${chat.chat_type} • ID: ${chat.chat_id}</span>
            ${chat.message_preview ? `<span class="chat-preview">"${chat.message_preview}"</span>` : ""}
          </div>
          <button class="btn btn-success btn-sm select-chat-btn">
            <i class="icon-check"></i> Select
          </button>
        </div>
      `
        )
        .join("");

      // Attach click handlers
      discoveredChatsEl.querySelectorAll(".select-chat-btn").forEach((btn) => {
        btn.addEventListener("click", async (e) => {
          const chatItem = e.target.closest(".discovered-chat-item");
          const chatId = chatItem.dataset.chatId;
          await selectChat(chatId);
        });
      });
    };

    const selectChat = async (chatId) => {
      try {
        const response = await fetch(`/api/telegram/discovery/select/${chatId}`, {
          method: "POST",
        });
        if (response.ok) {
          Utils.showToast("Chat selected successfully!", "success");
          stopDiscovery();
          // Reload the Telegram tab
          this._loadTelegramTab(content);
        } else {
          const data = await response.json();
          Utils.showToast(data.message || "Failed to select chat", "error");
        }
      } catch {
        Utils.showToast("Failed to select chat", "error");
      }
    };

    if (discoverBtn) {
      discoverBtn.addEventListener("click", startDiscovery);
    }

    if (changeChatBtn) {
      changeChatBtn.addEventListener("click", () => {
        // Clear current chat_id and start discovery
        updateSetting("chat_id", "").then(() => {
          this._loadTelegramTab(content);
        });
      });
    }

    if (cancelDiscoveryBtn) {
      cancelDiscoveryBtn.addEventListener("click", stopDiscovery);
    }

    // Test button
    const testBtn = content.querySelector("#tgTestBtn");
    if (testBtn) {
      testBtn.addEventListener("click", async () => {
        if (testBtn.dataset.submitting === "true") return;
        testBtn.dataset.submitting = "true";
        testBtn.disabled = true;
        testBtn.innerHTML = '<i class="icon-loader spin"></i> Sending...';
        try {
          const response = await fetch("/api/telegram/test", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({}),
          });
          const data = await response.json();
          if (response.ok) {
            Utils.showToast("Test message sent successfully!", "success");
          } else {
            Utils.showToast(data.message || "Failed to send test message", "error");
          }
        } catch {
          Utils.showToast("Failed to send test message", "error");
        } finally {
          testBtn.dataset.submitting = "false";
          testBtn.disabled = false;
          testBtn.innerHTML = '<i class="icon-send"></i> Send Test';
        }
      });
    }

    // Session timeout
    const timeoutSelect = content.querySelector("#tgSessionTimeout");
    if (timeoutSelect) {
      timeoutSelect.addEventListener("change", (e) =>
        updateSetting("session_timeout_minutes", parseInt(e.target.value, 10))
      );
      enhanceAllSelects(content);
    }

    // Session revoke buttons
    content.querySelectorAll(".session-revoke-btn").forEach((btn) => {
      btn.addEventListener("click", async () => {
        const sessionId = btn.dataset.sessionId;
        try {
          const response = await fetch(`/api/telegram/sessions/${sessionId}/revoke`, {
            method: "POST",
          });
          if (response.ok) {
            Utils.showToast("Session revoked", "success");
            this._loadTelegramTab(content);
          } else {
            Utils.showToast("Failed to revoke session", "error");
          }
        } catch {
          Utils.showToast("Failed to revoke session", "error");
        }
      });
    });

    // Notification toggles
    const notificationHandlers = [
      { id: "#tgNotifyOpened", key: "position_opened" },
      { id: "#tgNotifyClosed", key: "position_closed" },
      { id: "#tgNotifyPartial", key: "partial_exit" },
      { id: "#tgNotifyDca", key: "dca_executed" },
      { id: "#tgNotifyError", key: "errors" },
      { id: "#tgNotifyStartup", key: "startup_shutdown" },
      { id: "#tgNotifyFiltering", key: "filtering_alerts" },
      { id: "#tgNotifyTradeAlerts", key: "trade_alerts" },
      { id: "#tgNotifyDailySummary", key: "daily_summary" },
    ];

    notificationHandlers.forEach(({ id, key }) => {
      const toggle = content.querySelector(id);
      if (toggle) {
        toggle.addEventListener("change", (e) => {
          const notifications = { ...settings.notifications, [key]: e.target.checked };
          updateSetting("notifications", notifications);
        });
      }
    });

    // Features toggles
    const commandsToggle = content.querySelector("#tgCommandsEnabled");
    if (commandsToggle) {
      commandsToggle.addEventListener("change", (e) =>
        updateSetting("commands_enabled", e.target.checked)
      );
    }

    const inlineToggle = content.querySelector("#tgInlineActions");
    if (inlineToggle) {
      inlineToggle.addEventListener("change", (e) =>
        updateSetting("inline_actions", e.target.checked)
      );
    }

    const require2faToggle = content.querySelector("#tgRequire2fa");
    if (require2faToggle) {
      require2faToggle.addEventListener("change", (e) => {
        updateSetting("commands_require_2fa", e.target.checked);
        // Refresh auth section to update status display
        this._loadTelegramAuthState(content);
      });
    }
  }

  // ===========================================================================
  // TELEGRAM AUTHENTICATION (Lockscreen 2FA Integration)
  // ===========================================================================

  /**
   * Load Telegram authentication state and render UI
   */
  async _loadTelegramAuthState(content) {
    const authSection = content.querySelector("#tgAuthSection");
    if (!authSection) return;

    const statusEl = authSection.querySelector("#tg-auth-status");
    const contentEl = authSection.querySelector("#tg-auth-content");

    try {
      // Fetch lockscreen 2FA status from the lockscreen API
      const lockscreenResponse = await fetch("/api/lockscreen/status");
      const lockscreenData = await lockscreenResponse.json();

      if (!lockscreenResponse.ok) {
        throw new Error(lockscreenData.error || "Failed to load security settings");
      }

      const totpEnabled = lockscreenData.totp_enabled || false;

      // Also check the commands_require_2fa setting
      const require2faToggle = content.querySelector("#tgRequire2fa");
      const require2fa = require2faToggle ? require2faToggle.checked : true;

      this._renderAuthSection(statusEl, contentEl, totpEnabled, require2fa);
    } catch (error) {
      if (statusEl) {
        statusEl.innerHTML =
          '<span class="status-error"><i class="icon-circle-alert"></i> Error</span>';
      }
      if (contentEl) {
        contentEl.innerHTML = `<div class="telegram-auth-error">${this._escapeHtml(error.message)}</div>`;
      }
    }
  }

  /**
   * Render the command authentication section based on lockscreen 2FA status and require2fa setting
   */
  _renderAuthSection(statusEl, contentEl, totpEnabled, require2fa = true) {
    if (totpEnabled && require2fa) {
      statusEl.innerHTML =
        '<span class="status-success"><i class="icon-circle-check"></i> Protected</span>';
      contentEl.innerHTML = `
        <div class="telegram-auth-row">
          <div class="telegram-auth-info">
            <i class="icon-shield" style="color: var(--success); margin-right: 8px;"></i>
            <span>Commands are protected by lockscreen 2FA. When sessions expire, users must provide their authenticator code via <code>/login</code> command.</span>
          </div>
        </div>
        <div class="telegram-auth-note">
          <i class="icon-info"></i>
          <span>2FA is managed in <button type="button" class="link-button" id="tg-goto-security-btn">Security Settings</button></span>
        </div>
      `;
    } else if (totpEnabled && !require2fa) {
      statusEl.innerHTML =
        '<span class="status-warning"><i class="icon-circle-alert"></i> Disabled</span>';
      contentEl.innerHTML = `
        <div class="telegram-auth-row">
          <div class="telegram-auth-info">
            <i class="icon-triangle-alert" style="color: var(--warning); margin-right: 8px;"></i>
            <span>Lockscreen 2FA is configured but disabled for Telegram. Enable "Require 2FA for Commands" above to protect Telegram commands.</span>
          </div>
        </div>
        <div class="telegram-auth-note">
          <i class="icon-info"></i>
          <span>2FA is managed in <button type="button" class="link-button" id="tg-goto-security-btn">Security Settings</button></span>
        </div>
      `;
    } else {
      statusEl.innerHTML =
        '<span class="status-warning"><i class="icon-circle-alert"></i> Not Configured</span>';
      contentEl.innerHTML = `
        <div class="telegram-auth-row">
          <div class="telegram-auth-info">
            <i class="icon-triangle-alert" style="color: var(--warning); margin-right: 8px;"></i>
            <span>Lockscreen 2FA is not configured. Without 2FA, expired sessions will auto-reactivate without verification.</span>
          </div>
        </div>
        <div class="telegram-auth-note">
          <i class="icon-info"></i>
          <span>Configure 2FA in <button type="button" class="link-button" id="tg-goto-security-btn">Security Settings</button> to require verification for Telegram commands.</span>
        </div>
      `;
    }

    // Attach handler for security settings button
    const gotoSecurityBtn = contentEl.querySelector("#tg-goto-security-btn");
    if (gotoSecurityBtn) {
      gotoSecurityBtn.addEventListener("click", () => {
        this.switchToTab("security");
      });
    }
  }

  /**
   * Escape HTML to prevent XSS
   */
  _escapeHtml(str) {
    if (!str) return "";
    const div = document.createElement("div");
    div.textContent = str;
    return div.innerHTML;
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
    let response = await fetch("/api/updates/status");
    if (!response.ok) return;

    let data = await response.json();
    let state = data.state || data;

    // If no check has happened yet, trigger one
    if (!state.last_check && !state.available_update) {
      console.log("[SettingsDialog] No update check done yet, triggering check...");
      const checkResponse = await fetch("/api/updates/check");
      if (checkResponse.ok) {
        const checkData = await checkResponse.json();
        // Update state from check response
        if (checkData.update_available && checkData.update) {
          state = {
            available_update: checkData.update,
            last_check: checkData.last_check,
            download_progress: state.download_progress || {},
          };
        }
      }
    }

    // Check if update is available or downloading
    if (state.available_update || state.download_progress?.downloading) {
      console.log("[SettingsDialog] Update available, showing dialog...");

      // Update global state
      globalUpdateState.available = true;
      globalUpdateState.info = state.available_update;

      if (state.download_progress?.downloading) {
        globalUpdateState.downloading = true;
        globalUpdateState.progress = state.download_progress.progress_percent || 0;
      }

      // Show settings dialog with Updates tab selected
      await showSettingsDialog({ tab: "updates" });
    }
  } catch (err) {
    console.warn("[SettingsDialog] Failed to check for updates on startup:", err);
  }
}

// Auto-check for updates when dashboard is ready
// Use dynamic import to avoid circular dependencies and ensure bootstrap is loaded
(async function initUpdateCheck() {
  if (typeof window === "undefined" || !window.__SCREENERBOT_GUI_MODE) {
    return;
  }

  try {
    // Dynamically import bootstrap to get waitForReady
    const { waitForReady } = await import("../core/bootstrap.js");

    // Wait for dashboard to be ready
    await waitForReady();

    // Small delay to ensure UI is fully rendered
    setTimeout(checkAndShowUpdateDialog, 1500);
  } catch (err) {
    console.warn("[SettingsDialog] Failed to initialize update check:", err);
  }
})();
