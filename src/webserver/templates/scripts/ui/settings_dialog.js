/**
 * Settings Dialog Component
 * Full-screen settings dialog with tabs for Interface, Startup, About, Updates
 */
import * as Utils from "../core/utils.js";
import { createFocusTrap } from "../core/utils.js";
import { pushEscapeHandler } from "../core/escape_stack.js";
import { getCurrentPage } from "../core/router.js";
import { setInterval as setPollingInterval, Poller } from "../core/poller.js";
import { enhanceAllSelects } from "./custom_select.js";
import { playTabSwitch } from "../core/sounds.js";
import { loadSecurityTab } from "./settings/security_tab.js";
import { buildDataTab, attachDataHandlers } from "./settings/data_tab.js";
import { buildUpdatesTab, attachUpdatesHandlers } from "./settings/updates_tab.js";
import { buildInterfaceTab, attachInterfaceHandlers } from "./settings/interface_tab.js";
import { buildHintsTab, attachHintsHandlers } from "./settings/hints_tab.js";
import {
  loadAgentConnectionsTab,
  teardownAgentConnectionsTab,
} from "./settings/agent_connections_tab.js";
import { buildNavigationTab, attachNavigationHandlers } from "./settings/navigation_tab.js";
import { buildLicensesTab, attachLicensesHandlers } from "./settings/licenses_tab.js";
import { loadTelegramTab } from "./settings/telegram_tab.js";
import {
  buildAccountTab,
  attachAccountHandlers,
  teardownAccountTab,
} from "./settings/account_tab.js";

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
        globalUpdateState.checked = Boolean(state.last_check_attempt || state.last_check);
        globalUpdateState.error = state.check_error || progress.error || null;

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
        } else {
          globalUpdateState.available = false;
          globalUpdateState.info = null;
          globalUpdateState.downloaded = false;
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
      if (!response.ok || data.success === false) {
        throw new Error(data.error?.message || data.error || "Failed to check for updates");
      }

      globalUpdateState.checking = false;
      globalUpdateState.error = null;

      if (data.update_available) {
        globalUpdateState.available = true;
        globalUpdateState.info = data.update; // API returns 'update' not 'update_info'
      } else {
        globalUpdateState.available = false;
        globalUpdateState.info = null;
        globalUpdateState.downloaded = false;
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

    // Deactivate focus trap
    if (this._focusTrap) {
      this._focusTrap.deactivate();
      this._focusTrap = null;
    }

    // Stop any active pollers
    if (this.downloadPoller) {
      this.downloadPoller.cleanup();
      this.downloadPoller = null;
    }

    // Stop discovery poller if active
    if (this._discoveryPoller) {
      clearInterval(this._discoveryPoller);
      this._discoveryPoller = null;
    }

    // The account panel polls while a browser sign-in is in flight.
    teardownAccountTab();
    teardownAgentConnectionsTab();

    this.dialogEl.classList.remove("active");

    setTimeout(() => {
      this._releaseEscape?.();
      this._releaseEscape = null;

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
      // Overlay the live DOM theme — header toggle may have changed it without updating gui config
      const liveTheme = document.documentElement.getAttribute("data-theme");
      if (liveTheme && this.settings?.dashboard?.interface) {
        this.settings.dashboard.interface.theme = liveTheme;
      }
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
          token_logo_shape: "circle",
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
      {
        id: "assistant",
        label: "Assistant",
        icon: "icon-bot-message-square",
        order: 1,
        enabled: true,
      },
      {
        id: "positions",
        label: "Positions",
        icon: "icon-chart-candlestick",
        order: 2,
        enabled: true,
      },
      { id: "tokens", label: "Tokens", icon: "icon-coins", order: 3, enabled: true },
      { id: "filtering", label: "Filtering", icon: "icon-list-filter", order: 4, enabled: true },
      { id: "trader", label: "Auto Trader", icon: "icon-bot", order: 5, enabled: true },
      { id: "wallets", label: "Wallets", icon: "icon-wallet", order: 7, enabled: true },
      { id: "transactions", label: "Transactions", icon: "icon-activity", order: 8, enabled: true },
      { id: "tools", label: "Tools", icon: "icon-wrench", order: 9, enabled: true },
      { id: "services", label: "Services", icon: "icon-server", order: 10, enabled: true },
      { id: "events", label: "Events", icon: "icon-radio-tower", order: 11, enabled: true },
      { id: "config", label: "Config", icon: "icon-settings", order: 12, enabled: true },
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
      // Keep localStorage in sync for FOUC prevention
      try {
        localStorage.setItem("theme", iface.theme);
      } catch {
        /* storage unavailable */
      }
      // Save theme to server
      fetch("/api/ui-state/save", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ key: "theme", value: iface.theme }),
      }).catch((e) => console.warn("[Settings] Failed to save theme:", e));
      // Sync header toggle icon: dark → show sun (to switch to light); light → show moon
      const themeIcon = document.getElementById("themeIcon");
      if (themeIcon) {
        themeIcon.className =
          iface.theme === "dark" ? "action-icon icon-sun" : "action-icon icon-moon";
      }
    }

    // Apply animations
    if (typeof iface.enable_animations === "boolean") {
      document.documentElement.classList.toggle("no-animations", !iface.enable_animations);
    }

    const tokenLogoShape =
      iface.token_logo_shape === "rounded-square" ? "rounded-square" : "circle";
    document.documentElement.setAttribute("data-token-logo-shape", tokenLogoShape);

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
        <header class="settings-header modal-header">
          <h2 id="settings-dialog-title" class="modal-title">
            <i class="icon-settings"></i>
            <span>Settings</span>
          </h2>
          <div class="settings-header-actions">
            <button class="btn btn-primary" id="settingsSaveBtn" type="button" disabled>
              <i class="icon-save"></i>
              <span>Save Changes</span>
            </button>
            <button class="modal-close" type="button" title="Close (ESC)" aria-label="Close settings">
              <i class="icon-x"></i>
            </button>
          </div>
        </header>

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
            <button class="settings-nav-item" data-tab="hints">
              <i class="icon-lightbulb"></i>
              <span>Hints</span>
            </button>
            <button class="settings-nav-item" data-tab="data">
              <i class="icon-database"></i>
              <span>Data</span>
            </button>
            <button class="settings-nav-item" data-tab="security">
              <i class="icon-lock"></i>
              <span>Security</span>
            </button>
            <button class="settings-nav-item" data-tab="account">
              <i class="icon-circle-user"></i>
              <span>Account</span>
            </button>
            <button class="settings-nav-item" data-tab="telegram">
              <i class="icon-send"></i>
              <span>Telegram</span>
            </button>
            <button class="settings-nav-item" data-tab="agent-connections">
              <i class="icon-plug"></i>
              <span>Agent Connections</span>
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
            <div class="settings-tab" data-tab-content="hints">
              <div class="settings-loading">Loading...</div>
            </div>
            <div class="settings-tab" data-tab-content="data">
              <div class="settings-loading">Loading...</div>
            </div>
            <div class="settings-tab" data-tab-content="security">
              <div class="settings-loading">Loading...</div>
            </div>
            <div class="settings-tab" data-tab-content="account">
              <div class="settings-loading">Loading...</div>
            </div>
            <div class="settings-tab" data-tab-content="telegram">
              <div class="settings-loading">Loading...</div>
            </div>
            <div class="settings-tab" data-tab-content="agent-connections">
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
    const closeBtn = this.dialogEl.querySelector(".modal-close");
    closeBtn.addEventListener("click", () => this.close());

    // Backdrop click
    const backdrop = this.dialogEl.querySelector(".settings-backdrop");
    backdrop.addEventListener("click", () => this.close());

    // ESC key
    this._releaseEscape = pushEscapeHandler(() => this.close());

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

    if (this.currentTab === "agent-connections") {
      teardownAgentConnectionsTab();
    }

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
        content.innerHTML = buildInterfaceTab(this.settings);
        attachInterfaceHandlers(this, content);
        break;
      case "navigation":
        content.innerHTML = buildNavigationTab(this.settings);
        attachNavigationHandlers(this, content);
        break;
      case "startup":
        content.innerHTML = this._buildStartupTab();
        this._attachStartupHandlers(content);
        enhanceAllSelects(content);
        break;
      case "hints":
        content.innerHTML = buildHintsTab();
        attachHintsHandlers(this, content);
        break;
      case "data":
        content.innerHTML = buildDataTab();
        attachDataHandlers(this, content, this.pathsInfo);
        break;
      case "security":
        loadSecurityTab(this, content);
        break;
      case "account":
        content.innerHTML = buildAccountTab();
        attachAccountHandlers();
        break;
      case "telegram":
        loadTelegramTab(this, content);
        break;
      case "agent-connections":
        loadAgentConnectionsTab(this, content);
        break;
      case "updates":
        content.innerHTML = buildUpdatesTab(this, this.versionInfo, globalUpdateState);
        attachUpdatesHandlers(
          this,
          globalUpdateState,
          () => this._updateUpdatesTabUI(),
          () => this._startDownloadPoller(),
          () => this._updateUpdatesBadge()
        );
        break;
      case "licenses":
        content.innerHTML = buildLicensesTab();
        attachLicensesHandlers(content);
        break;
      case "about":
        content.innerHTML = this._buildAboutTab();
        this._attachAboutHandlers(content);
        break;
    }
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
              <label class="toggle">
                <input type="checkbox" id="settingAutoStart" ${startup.auto_start_trader ? "checked" : ""} disabled>
                <span class="toggle-track"></span>
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
              <label class="toggle">
                <input type="checkbox" id="settingBgNotifications" ${startup.show_background_notifications !== false ? "checked" : ""}>
                <span class="toggle-track"></span>
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
              <label class="toggle">
                <input type="checkbox" id="settingCheckUpdates" ${startup.check_updates_on_startup ? "checked" : ""} disabled>
                <span class="toggle-track"></span>
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

  // ==========================================================================
  // SECURITY TAB - Lockscreen settings (extracted to settings/security_tab.js)
  // ==========================================================================

  // Security tab methods have been extracted to settings/security_tab.js
  // The loadSecurityTab function is imported and used in _loadTabContent

  // ==========================================================================
  // UPDATES TAB - Version checking and auto-update (extracted to settings/updates_tab.js)
  // ==========================================================================

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
   * Update the Updates tab UI without full rebuild
   */
  _updateUpdatesTabUI() {
    if (!this.dialogEl) return;
    const updatesTab = this.dialogEl.querySelector('.settings-tab[data-tab-content="updates"]');
    if (updatesTab) {
      updatesTab.innerHTML = buildUpdatesTab(this, this.versionInfo, globalUpdateState);
      attachUpdatesHandlers(
        this,
        globalUpdateState,
        () => this._updateUpdatesTabUI(),
        () => this._startDownloadPoller(),
        () => this._updateUpdatesBadge()
      );
    }
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
        <p class="settings-about-tagline">Native Solana Trading Engine</p>
        <div class="settings-about-version">
          <span>v${version}</span>
        </div>

        <div class="settings-about-links">
          <button class="settings-about-link" data-external-url="https://github.com/farfary/ScreenerBot">
            <i class="icon-github"></i>
            <span>GitHub</span>
          </button>
          <button class="settings-about-link" data-external-url="https://screenerbot.io/docs">
            <i class="icon-book-open"></i>
            <span>Documentation</span>
          </button>
          <button class="settings-about-link" data-external-url="https://t.me/screenerbotio">
            <i class="icon-message-circle"></i>
            <span>Telegram</span>
          </button>
          <button class="settings-about-link" data-external-url="https://screenerbot.io">
            <i class="icon-globe"></i>
            <span>Website</span>
          </button>
        </div>

        <div class="settings-about-separator"></div>

        <div class="settings-about-path-card" style="text-align: center; cursor: pointer;" onclick="navigator.clipboard.writeText('')">
          <div class="settings-about-path-icon">
            <i class="icon-heart" style="color: #9945FF;"></i>
          </div>
          <div class="settings-about-path-details">
            <p class="settings-about-path-label">Support Development — Donate SOL</p>
            <p class="settings-about-path-value" style="font-size: 11px; word-break: break-all;"></p>
            <p class="settings-about-path-hint">Click to copy address</p>
          </div>
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
          <p class="settings-about-copyright">© ${new Date().getFullYear()} ScreenerBot. All rights reserved.</p>
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
    if (this.downloadPoller) this.downloadPoller.cleanup();

    // Track download speed
    let lastProgress = 0;
    let lastTime = Date.now();
    let speedHistory = [];

    this.downloadPoller = new Poller(
      async () => {
        try {
          const statusRes = await fetch("/api/updates/status");
          const data = await statusRes.json();
          if (!statusRes.ok || data.success === false) {
            throw new Error(data.error?.message || data.error || "Failed to read update status");
          }
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
            if (this.downloadPoller) this.downloadPoller.cleanup();
            this.downloadPoller = null;
            this._updateUpdatesTabUI();
          } else if (progress.error) {
            throw new Error(progress.error || "Download failed");
          }
        } catch (err) {
          console.error("Download poll error:", err);
          globalUpdateState.downloading = false;
          globalUpdateState.error = err.message;
          if (this.downloadPoller) this.downloadPoller.cleanup();
          this.downloadPoller = null;
          this._updateUpdatesTabUI();
        }
      },
      { label: "UpdateDownload", intervalMs: 1000, pauseWhenHidden: false }
    );

    this.downloadPoller.start();
  }

  // ===========================================================================
  // TELEGRAM TAB
  // ===========================================================================

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

if (window.electronAPI?.onCheckForUpdates) {
  window.electronAPI.onCheckForUpdates(async () => {
    await showSettingsDialog({ tab: "updates" });
    setTimeout(() => settingsDialogInstance?._performBackgroundUpdateCheck(), 150);
  });
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
