/**
 * Telegram Tab Module - Telegram bot integration settings
 * Extracted from settings_dialog.js
 */
import * as Utils from "../../core/utils.js";
import { enhanceAllSelects } from "../custom_select.js";

/**
 * Load Telegram tab (async because we need to fetch settings and auth state)
 */
export async function loadTelegramTab(dialog, content) {
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

    content.innerHTML = buildTelegramTab(settings);
    attachTelegramHandlers(dialog, content, settings);

    // Load Password + TOTP authentication state
    await loadTelegramAuthState(dialog, content);
  } catch (error) {
    console.error("[Settings] Failed to load Telegram settings:", error);
    content.innerHTML = '<div class="settings-error">Failed to load Telegram settings</div>';
  }
}

/**
 * Build Telegram tab HTML
 */
function buildTelegramTab(settings) {
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
            <label class="toggle" data-level="root">
              <input type="checkbox" id="tgEnabled" ${settings.enabled ? "checked" : ""}>
              <span class="toggle-track"></span>
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
            <label class="toggle" data-level="item">
              <input type="checkbox" id="tgNotifyOpened" ${settings.notifications?.position_opened !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Position Closed</label>
            <span class="settings-field-hint">Notify when a position is closed</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle" data-level="item">
              <input type="checkbox" id="tgNotifyClosed" ${settings.notifications?.position_closed !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Partial Exit</label>
            <span class="settings-field-hint">Notify on partial position exits</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle" data-level="item">
              <input type="checkbox" id="tgNotifyPartial" ${settings.notifications?.partial_exit !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>DCA Executed</label>
            <span class="settings-field-hint">Notify when DCA orders are executed</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle" data-level="item">
              <input type="checkbox" id="tgNotifyDca" ${settings.notifications?.dca_executed !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Errors</label>
            <span class="settings-field-hint">Notify on errors and failures</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle" data-level="item">
              <input type="checkbox" id="tgNotifyError" ${settings.notifications?.errors !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Startup/Shutdown</label>
            <span class="settings-field-hint">Notify when bot starts or stops</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle" data-level="item">
              <input type="checkbox" id="tgNotifyStartup" ${settings.notifications?.startup_shutdown !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Filtering Alerts</label>
            <span class="settings-field-hint">Notify when new tokens pass filtering criteria</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle" data-level="item">
              <input type="checkbox" id="tgNotifyFiltering" ${settings.notifications?.filtering_alerts !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Trade Alerts</label>
            <span class="settings-field-hint">Notify on significant trades for watched tokens</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle" data-level="item">
              <input type="checkbox" id="tgNotifyTradeAlerts" ${settings.notifications?.trade_alerts !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Daily Summary</label>
            <span class="settings-field-hint">Receive daily trading activity and P&L summary</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle" data-level="item">
              <input type="checkbox" id="tgNotifyDailySummary" ${settings.notifications?.daily_summary === true ? "checked" : ""}>
              <span class="toggle-track"></span>
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
            <label class="toggle" data-level="group">
              <input type="checkbox" id="tgCommandsEnabled" ${settings.commands_enabled !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Require 2FA for Commands</label>
            <span class="settings-field-hint">When sessions expire, require 2FA code to reactivate. Uses lockscreen 2FA.</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle" data-level="item">
              <input type="checkbox" id="tgRequire2fa" ${settings.commands_require_2fa !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label>Inline Action Buttons</label>
            <span class="settings-field-hint">Show action buttons in notification messages</span>
          </div>
          <div class="settings-field-control">
            <label class="toggle" data-level="item">
              <input type="checkbox" id="tgInlineActions" ${settings.inline_actions !== false ? "checked" : ""}>
              <span class="toggle-track"></span>
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
function attachTelegramHandlers(dialog, content, settings) {
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
      dialog._discoveryPoller = setInterval(async () => {
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
    if (dialog._discoveryPoller) {
      clearInterval(dialog._discoveryPoller);
      dialog._discoveryPoller = null;
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
        loadTelegramTab(dialog, content);
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
        loadTelegramTab(dialog, content);
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
          loadTelegramTab(dialog, content);
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
      loadTelegramAuthState(dialog, content);
    });
  }
}

// ===========================================================================
// TELEGRAM AUTHENTICATION (Lockscreen 2FA Integration)
// ===========================================================================

/**
 * Load Telegram authentication state and render UI
 */
async function loadTelegramAuthState(dialog, content) {
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

    renderAuthSection(dialog, statusEl, contentEl, totpEnabled, require2fa);
  } catch (error) {
    if (statusEl) {
      statusEl.innerHTML =
        '<span class="status-error"><i class="icon-circle-alert"></i> Error</span>';
    }
    if (contentEl) {
      contentEl.innerHTML = `<div class="telegram-auth-error">${escapeHtml(error.message)}</div>`;
    }
  }
}

/**
 * Render the command authentication section based on lockscreen 2FA status and require2fa setting
 */
function renderAuthSection(dialog, statusEl, contentEl, totpEnabled, require2fa = true) {
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
      dialog.switchToTab("security");
    });
  }
}

/**
 * Escape HTML to prevent XSS
 */
function escapeHtml(str) {
  if (!str) return "";
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}
