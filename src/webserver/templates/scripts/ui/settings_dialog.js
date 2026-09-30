/**
 * Settings Dialog Component
 * Full-screen settings dialog with tabs for Interface, Startup, About, Updates
 */
import * as Utils from "../core/utils.js";
import { createFocusTrap } from "../core/utils.js";
import { pushEscapeHandler } from "../core/escape_stack.js";
import { getCurrentPage } from "../core/router.js";
import { apiErrorMessage } from "../core/request_manager.js";
import { setInterval as setPollingInterval } from "../core/poller.js";
import { enhanceAllSelects } from "./custom_select.js";
import { playTabSwitch } from "../core/sounds.js";
import { loadSecurityTab } from "./settings/security_tab.js";
import { buildDataTab, attachDataHandlers } from "./settings/data_tab.js";
import {
  buildUpdatesTab,
  attachUpdatesHandlers,
  requestUpdateCheck,
  teardownUpdatesTab,
} from "./settings/updates_tab.js";
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

// Whether an update needs a decision, for the nav indicator. Everything else about the
// update lifecycle is owned by settings/updates_tab.js and read straight from
// the backend, so there is only ever one description of it.
let updateNeedsAttention = false;
let lastSurfacedUpdateKey = null;
const GUI_DRAFT_TABS = new Set(["interface", "navigation", "startup"]);

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
    this.versionInfo = {
      version: "...",
      build_number: "...",
      platform: "...",
      shell_revision: null,
      core_staged: false,
    };
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

  /** Read whether an update needs attention for the nav indicator. */
  async _syncUpdateStatus() {
    try {
      const response = await fetch("/api/updates/status");
      if (!response.ok) return;
      const body = await response.json();
      const payload = body.data || body;
      this._setUpdateBadge(Boolean(payload.requires_user_action));
    } catch (err) {
      console.warn("Failed to read update status:", err);
    }
  }

  /**
   * Show or clear the status indicator on the Updates nav item.
   */
  _setUpdateBadge(needsAttention) {
    updateNeedsAttention = needsAttention;
    if (!this.dialogEl) return;

    const updatesBtn = this.dialogEl.querySelector('.settings-nav-item[data-tab="updates"]');
    if (!updatesBtn) return;

    const existingIndicator = updatesBtn.querySelector(".settings-nav-indicator");
    if (existingIndicator && updateNeedsAttention) return;
    if (existingIndicator) existingIndicator.remove();

    if (updateNeedsAttention) {
      const indicator = document.createElement("span");
      indicator.className = "settings-nav-indicator";
      indicator.innerHTML =
        '<i class="icon-circle-alert" aria-hidden="true"></i><span class="sr-only"></span>';
      indicator.querySelector(".sr-only").textContent = I18n.t("settings-dialog-update-attention");
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
          shell_revision: data.shell_revision || null,
          core_staged: Boolean(data.core_staged),
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

    // The Updates tab polls only while a download or install is in flight.
    teardownUpdatesTab();

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
          show_background_notifications: true,
        },
        navigation: {
          tabs: [],
        },
      },
    };
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
        const body = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(body, response.statusText || `HTTP ${response.status}`));
      }

      // The server renders localized HTML, so a new display language only
      // takes effect on a fresh page load.
      const languageChanged =
        this.settings?.dashboard?.interface?.language !==
        this.originalSettings?.dashboard?.interface?.language;

      this.originalSettings = JSON.parse(JSON.stringify(this.settings));
      this.hasChanges = false;
      this._updateSaveButton();

      if (languageChanged) {
        window.location.reload();
        return;
      }

      Utils.showToast({
        type: "success",
        title: I18n.t("settings-dialog-save-success"),
      });

      // Apply settings immediately
      this._applyInterfaceSettings();
      this._applyNavigationSettings();
    } catch (error) {
      console.error("Failed to save settings:", error);
      Utils.showToast({
        type: "error",
        title: I18n.t("settings-dialog-save-failed"),
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
        return `<a href="#" data-page="${tab.id}" class="tab${activeClass}"><i class="${tab.icon}"></i> <span>${I18n.t("nav-" + tab.id)}</span></a>`; // l10n-dynamic: nav-
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
    I18n.localizeTree(this.dialogEl);
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
            <span data-l10n-id="settings-dialog-title"></span>
          </h2>
          <div class="settings-header-actions">
            <button class="btn btn-primary settings-save-btn" id="settingsSaveBtn" type="button" disabled>
              <i class="icon-save"></i>
              <span data-l10n-id="settings-dialog-save"></span>
            </button>
            <button class="modal-close" type="button" data-l10n-id="settings-dialog-close">
              <i class="icon-x"></i>
            </button>
          </div>
        </header>

        <div class="settings-body">
          <nav class="settings-nav">
            <button class="settings-nav-item active" data-tab="interface">
              <i class="icon-palette"></i>
              <span data-l10n-id="settings-dialog-tab-interface"></span>
            </button>
            <button class="settings-nav-item" data-tab="navigation">
              <i class="icon-layout-grid"></i>
              <span data-l10n-id="settings-dialog-tab-navigation"></span>
            </button>
            <button class="settings-nav-item" data-tab="startup">
              <i class="icon-zap"></i>
              <span data-l10n-id="settings-dialog-tab-startup"></span>
            </button>
            <button class="settings-nav-item" data-tab="hints">
              <i class="icon-lightbulb"></i>
              <span data-l10n-id="settings-dialog-tab-hints"></span>
            </button>
            <button class="settings-nav-item" data-tab="data">
              <i class="icon-database"></i>
              <span data-l10n-id="settings-dialog-tab-data"></span>
            </button>
            <button class="settings-nav-item" data-tab="security">
              <i class="icon-lock"></i>
              <span data-l10n-id="settings-dialog-tab-security"></span>
            </button>
            <button class="settings-nav-item" data-tab="account">
              <i class="icon-circle-user"></i>
              <span data-l10n-id="settings-dialog-tab-account"></span>
            </button>
            <button class="settings-nav-item" data-tab="telegram">
              <i class="icon-send"></i>
              <span data-l10n-id="settings-dialog-tab-telegram"></span>
            </button>
            <button class="settings-nav-item" data-tab="agent-connections">
              <i class="icon-plug"></i>
              <span data-l10n-id="settings-dialog-tab-agent-connections"></span>
            </button>
            <div class="settings-nav-divider"></div>
            <button class="settings-nav-item" data-tab="updates">
              <i class="icon-refresh-cw"></i>
              <span data-l10n-id="settings-dialog-tab-updates"></span>
            </button>
            <button class="settings-nav-item" data-tab="licenses">
              <i class="icon-scale"></i>
              <span data-l10n-id="settings-dialog-tab-licenses"></span>
            </button>
            <button class="settings-nav-item" data-tab="about">
              <i class="icon-info"></i>
              <span data-l10n-id="settings-dialog-tab-about"></span>
            </button>
            <div class="settings-nav-divider"></div>
            <button class="settings-nav-item settings-nav-link" data-external-url="https://screenerbot.io/privacy">
              <i class="icon-shield"></i>
              <span data-l10n-id="settings-dialog-link-privacy"></span>
              <i class="icon-external-link settings-nav-external"></i>
            </button>
            <button class="settings-nav-item settings-nav-link" data-external-url="https://screenerbot.io/terms">
              <i class="icon-file-text"></i>
              <span data-l10n-id="settings-dialog-link-terms"></span>
              <i class="icon-external-link settings-nav-external"></i>
            </button>
          </nav>

          <div class="settings-content">
            <div class="settings-tab active" data-tab-content="interface">
              <div class="settings-loading" data-l10n-id="common-loading"></div>
            </div>
            <div class="settings-tab" data-tab-content="navigation">
              <div class="settings-loading" data-l10n-id="common-loading"></div>
            </div>
            <div class="settings-tab" data-tab-content="startup">
              <div class="settings-loading" data-l10n-id="common-loading"></div>
            </div>
            <div class="settings-tab" data-tab-content="hints">
              <div class="settings-loading" data-l10n-id="common-loading"></div>
            </div>
            <div class="settings-tab" data-tab-content="data">
              <div class="settings-loading" data-l10n-id="common-loading"></div>
            </div>
            <div class="settings-tab" data-tab-content="security">
              <div class="settings-loading" data-l10n-id="common-loading"></div>
            </div>
            <div class="settings-tab" data-tab-content="account">
              <div class="settings-loading" data-l10n-id="common-loading"></div>
            </div>
            <div class="settings-tab" data-tab-content="telegram">
              <div class="settings-loading" data-l10n-id="common-loading"></div>
            </div>
            <div class="settings-tab" data-tab-content="agent-connections">
              <div class="settings-loading" data-l10n-id="common-loading"></div>
            </div>
            <div class="settings-tab" data-tab-content="updates">
              <div class="settings-loading" data-l10n-id="common-loading"></div>
            </div>
            <div class="settings-tab" data-tab-content="licenses">
              <div class="settings-loading" data-l10n-id="common-loading"></div>
            </div>
            <div class="settings-tab" data-tab-content="about">
              <div class="settings-loading" data-l10n-id="common-loading"></div>
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
    if (this.currentTab === "updates") {
      teardownUpdatesTab();
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
    this._updateSaveButton();
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
        I18n.localizeTree(content);
        attachInterfaceHandlers(this, content);
        break;
      case "navigation":
        content.innerHTML = buildNavigationTab(this.settings);
        attachNavigationHandlers(this, content);
        break;
      case "startup":
        content.innerHTML = this._buildStartupTab();
        I18n.localizeTree(content);
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
        content.innerHTML = buildUpdatesTab(this.versionInfo);
        attachUpdatesHandlers(content, (available) => this._setUpdateBadge(available));
        break;
      case "licenses":
        content.innerHTML = buildLicensesTab();
        attachLicensesHandlers(content);
        break;
      case "about":
        content.innerHTML = this._buildAboutTab();
        I18n.localizeTree(content);
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
        <h3 class="settings-section-title" data-l10n-id="settings-startup-section-title"></h3>
        <div class="settings-group">
          <div class="settings-field settings-field--disabled">
            <div class="settings-field-info">
              <label data-l10n-id="settings-startup-auto-start-label"></label>
              <span class="settings-field-hint" data-l10n-id="settings-startup-auto-start-hint"></span>
              <span class="settings-field-badge" data-l10n-id="settings-startup-coming-soon"></span>
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
              <label data-l10n-id="settings-startup-default-page-label"></label>
              <span class="settings-field-hint" data-l10n-id="settings-startup-default-page-hint"></span>
            </div>
            <div class="settings-field-control">
              <select id="settingDefaultPage" class="settings-select" data-custom-select>
                <option value="dashboard" ${startup.default_page === "dashboard" || !startup.default_page ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-startup-page-dashboard"))}</option>
                <option value="tokens" ${startup.default_page === "tokens" ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-startup-page-tokens"))}</option>
                <option value="positions" ${startup.default_page === "positions" ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-startup-page-positions"))}</option>
                <option value="wallet" ${startup.default_page === "wallet" ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-startup-page-wallet"))}</option>
                <option value="config" ${startup.default_page === "config" ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-startup-page-config"))}</option>
              </select>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label data-l10n-id="settings-startup-notifications-label"></label>
              <span class="settings-field-hint" data-l10n-id="settings-startup-notifications-hint"></span>
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
   * Attach handlers for About tab.
   *
   * About answers "what is this and where do I find it"; every data location
   * and the actions on it belong to the Data tab, which already owns them.
   */
  _attachAboutHandlers(content) {
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
   * Build About tab content
   */
  _buildAboutTab() {
    const { version } = this.versionInfo;
    return `
      <div class="settings-about">
        <div class="settings-about-logo">
          <img src="/assets/logo.svg" alt="" data-l10n-id="settings-about-logo" />
        </div>
        <h2 class="settings-about-name" translate="no">ScreenerBot</h2>
        <p class="settings-about-tagline" data-l10n-id="settings-about-tagline"></p>
        <div class="settings-about-version">
          <span translate="no">v${version}</span>
        </div>

        <div class="settings-about-links">
          <button class="settings-about-link" data-external-url="https://github.com/farfary/ScreenerBot">
            <i class="icon-github"></i>
            <span data-l10n-id="settings-about-link-github"></span>
          </button>
          <button class="settings-about-link" data-external-url="https://screenerbot.io/docs">
            <i class="icon-book-open"></i>
            <span data-l10n-id="settings-about-link-docs"></span>
          </button>
          <button class="settings-about-link" data-external-url="https://t.me/screenerbotio">
            <i class="icon-message-circle"></i>
            <span data-l10n-id="settings-about-link-telegram"></span>
          </button>
          <button class="settings-about-link" data-external-url="https://screenerbot.io">
            <i class="icon-globe"></i>
            <span data-l10n-id="settings-about-link-website"></span>
          </button>
        </div>

        <div class="settings-about-credits">
          <p data-l10n-id="settings-about-credits"></p>
          <p class="settings-about-copyright" data-l10n-id="settings-about-copyright" data-l10n-args='{"year":"${new Date().getFullYear()}"}'></p>
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

    saveBtn.hidden = !this.hasChanges && !GUI_DRAFT_TABS.has(this.currentTab);
    saveBtn.disabled = !this.hasChanges || this.isSaving;

    const icon = saveBtn.querySelector("i");
    const text = saveBtn.querySelector("span");

    if (this.isSaving) {
      icon.className = "icon-loader";
      text.textContent = I18n.t("settings-dialog-saving");
    } else {
      icon.className = "icon-save";
      text.textContent = this.hasChanges
        ? I18n.t("settings-dialog-save")
        : I18n.t("settings-dialog-saved");
    }
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

  // show() creates the complete dialog DOM before it resolves. Switch directly
  // so callers can safely invoke tab-owned actions after awaiting this function.
  if (options.tab) {
    settingsDialogInstance.switchToTab(options.tab);
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
    await requestUpdateCheck();
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
    const response = await fetch("/api/updates/status");
    if (!response.ok) return;

    const body = await response.json();
    const payload = body.data || body;
    let state = payload.state || payload;
    state.blocked_reason = payload.blocked_reason || null;
    state.requires_user_action = Boolean(payload.requires_user_action);

    // If no check has even been attempted yet, trigger the startup check. A
    // failed attempt is retried by the backend policy; the observer must not add
    // another manual request every minute while the network remains unavailable.
    if (!state.last_check_attempt && !state.available_update) {
      const checkResponse = await fetch("/api/updates/check");
      if (checkResponse.ok) {
        const refreshed = await fetch("/api/updates/status");
        if (refreshed.ok) {
          const refreshedBody = await refreshed.json();
          const refreshedPayload = refreshedBody.data || refreshedBody;
          state = refreshedPayload.state || refreshedPayload;
          state.blocked_reason = refreshedPayload.blocked_reason || null;
          state.requires_user_action = Boolean(refreshedPayload.requires_user_action);
        }
      }
    }

    // Only surface the panel for an update that still needs a decision. A core
    // update that installs itself must not steal the screen on every launch.
    const needsAttention = state.requires_user_action;
    const attentionKey = needsAttention
      ? `${state.available_update?.version || "unknown"}:${state.phase}:${state.download_progress?.error || ""}`
      : null;
    if (attentionKey && attentionKey !== lastSurfacedUpdateKey) {
      lastSurfacedUpdateKey = attentionKey;
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
    // The backend checker runs after the dashboard opens. Observe its durable
    // state so a later discovered update is surfaced without reopening Settings.
    window.setInterval(checkAndShowUpdateDialog, 60_000);
  } catch (err) {
    console.warn("[SettingsDialog] Failed to initialize update check:", err);
  }
})();
