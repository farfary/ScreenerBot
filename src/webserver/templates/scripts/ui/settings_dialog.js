/**
 * Settings Dialog Component
 * Full-screen settings dialog with tabs for Interface, Startup, About, Updates
 */
import * as Utils from "../core/utils.js";

const VERSION = "0.1.0";
const BUILD_DATE = "2025-12-02";

export class SettingsDialog {
  constructor(options = {}) {
    this.onClose = options.onClose || (() => {});
    this.dialogEl = null;
    this.currentTab = "interface";
    this.settings = null;
    this.originalSettings = null;
    this.hasChanges = false;
    this.isSaving = false;
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
    await this._loadSettings();
    this._loadTabContent("interface");

    requestAnimationFrame(() => {
      if (this.dialogEl) {
        this.dialogEl.classList.add("active");
      }
    });
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
        throw new Error(`Failed to save settings: ${response.statusText}`);
      }

      this.originalSettings = JSON.parse(JSON.stringify(this.settings));
      this.hasChanges = false;
      this._updateSaveButton();

      Utils.showToast('<i class="icon-check"></i> Settings saved successfully', "success");

      // Apply theme change immediately if changed
      this._applyInterfaceSettings();
    } catch (error) {
      console.error("Failed to save settings:", error);
      Utils.showToast(`<i class="icon-x"></i> ${error.message}`, "error");
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
            <button class="settings-nav-item" data-tab="startup">
              <i class="icon-zap"></i>
              <span>Startup</span>
            </button>
            <div class="settings-nav-divider"></div>
            <button class="settings-nav-item" data-tab="updates">
              <i class="icon-refresh-cw"></i>
              <span>Updates</span>
            </button>
            <button class="settings-nav-item" data-tab="about">
              <i class="icon-info"></i>
              <span>About</span>
            </button>
          </nav>

          <div class="settings-content">
            <div class="settings-tab active" data-tab-content="interface">
              <div class="settings-loading">Loading...</div>
            </div>
            <div class="settings-tab" data-tab-content="startup">
              <div class="settings-loading">Loading...</div>
            </div>
            <div class="settings-tab" data-tab-content="updates">
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
      case "startup":
        content.innerHTML = this._buildStartupTab();
        this._attachStartupHandlers(content);
        break;
      case "updates":
        content.innerHTML = this._buildUpdatesTab();
        this._attachUpdatesHandlers(content);
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
              <label>Polling Interval</label>
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
    return `
      <div class="settings-updates-section">
        <div class="settings-section">
          <h3 class="settings-section-title">Software Updates</h3>
          <div class="settings-update-card">
            <div class="settings-update-icon">
              <i class="icon-circle-check"></i>
            </div>
            <div class="settings-update-info">
              <h4>You're up to date!</h4>
              <p>ScreenerBot ${VERSION} is the latest version.</p>
            </div>
            <button class="settings-update-btn" id="checkUpdatesBtn">
              <i class="icon-refresh-cw"></i>
              <span>Check for Updates</span>
            </button>
          </div>
        </div>

        <div class="settings-section">
          <h3 class="settings-section-title">Release Notes</h3>
          <div class="settings-release-notes">
            <div class="settings-release">
              <div class="settings-release-header">
                <span class="settings-release-version">v${VERSION}</span>
                <span class="settings-release-date">${BUILD_DATE}</span>
                <span class="settings-release-badge">Current</span>
              </div>
              <ul class="settings-release-changes">
                <li>Full-featured settings dialog</li>
                <li>Interface customization options</li>
                <li>Startup behavior configuration</li>
                <li>Theme and animation controls</li>
              </ul>
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
      checkBtn.addEventListener("click", () => {
        Utils.showToast(
          '<i class="icon-info"></i> Update checking is not available yet',
          "info"
        );
      });
    }
  }

  /**
   * Build About tab content
   */
  _buildAboutTab() {
    return `
      <div class="settings-about">
        <div class="settings-about-logo">
          <img src="/assets/logo.svg" alt="ScreenerBot" />
        </div>
        <h2 class="settings-about-name">ScreenerBot</h2>
        <p class="settings-about-tagline">Advanced Solana Trading Automation</p>
        <div class="settings-about-version">
          <span>Version ${VERSION}</span>
          <span class="settings-about-separator">•</span>
          <span>Build ${BUILD_DATE}</span>
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
          <p>Built with ❤️ for Solana traders</p>
          <p class="settings-about-copyright">© 2025 ScreenerBot. All rights reserved.</p>
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
