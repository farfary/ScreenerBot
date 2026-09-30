/**
 * Telegram Tab Module - Telegram bot integration settings
 * Extracted from settings_dialog.js
 */
import * as Utils from "../../core/utils.js";
import { enhanceAllSelects } from "../custom_select.js";
import { apiErrorMessage } from "../../core/request_manager.js";

// Ids are the chat kinds reported by the Telegram poller (src/telegram/polling.rs).
const CHAT_TYPE_LABELS = Object.freeze({
  private: "settings-telegram-chat-type-private",
  group: "settings-telegram-chat-type-group",
  supergroup: "settings-telegram-chat-type-supergroup",
  channel: "settings-telegram-chat-type-channel",
});

/** Replace a button's content with an icon and an already localized label. */
function setButton(button, icon, label) {
  const glyph = document.createElement("i");
  glyph.className = icon;
  button.replaceChildren(glyph, ` ${label}`);
}

/**
 * Load Telegram tab (async because we need to fetch settings and auth state)
 */
export async function loadTelegramTab(dialog, content) {
  content.innerHTML =
    '<div class="settings-loading"><i class="icon-loader spin"></i> <span data-l10n-id="settings-telegram-loading"></span></div>';
  I18n.localizeTree(content);

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
    I18n.localizeTree(content);
    attachTelegramHandlers(dialog, content, settings);

    // Load Password + TOTP authentication state
    await loadTelegramAuthState(dialog, content);
  } catch (error) {
    console.error("[Settings] Failed to load Telegram settings:", error);
    content.innerHTML =
      '<div class="settings-error" data-l10n-id="settings-telegram-load-failed"></div>';
    I18n.localizeTree(content);
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
          <span class="session-user">${s.username || Utils.escapeHtml(I18n.t("settings-telegram-unknown"))}</span>
          <span class="session-time">${Utils.escapeHtml(I18n.t("settings-telegram-session-active", { duration: Utils.formatDuration(s.created_at_secs * 1000) }))}</span>
        </div>
        <button class="btn btn-danger btn-sm session-revoke-btn" data-session-id="${s.user_id}">
          <i class="icon-x"></i> <span data-l10n-id="settings-telegram-session-revoke"></span>
        </button>
      </div>
    `
          )
          .join("")
      : '<div class="sessions-empty" data-l10n-id="settings-telegram-sessions-empty"></div>';

  return `
    <!-- Connection Section -->
    <div class="settings-section">
      <h3 class="settings-section-title">
        <i class="icon-send"></i>
        <span data-l10n-id="settings-telegram-connection-title"></span>
      </h3>
      <p class="settings-section-description" data-l10n-id="settings-telegram-connection-description"></p>

      <div class="settings-group">
        <div class="settings-field">
          <div class="settings-field-info">
            <label data-l10n-id="settings-telegram-enable-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-telegram-enable-hint"></span>
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
            <label data-l10n-id="settings-telegram-token-label"></label>
            <span class="settings-field-hint">${settings.bot_token && settings.bot_token.endsWith("...") ? '<i class="icon-circle-check" style="color: var(--success);"></i> <span data-l10n-id="settings-telegram-token-saved"></span>' : '<span data-l10n-id="settings-telegram-token-help"></span>'}</span>
          </div>
          <div class="settings-field-control telegram-token-field">
            <input type="password" id="tgBotToken" class="settings-input" placeholder="${Utils.escapeHtml(settings.bot_token && settings.bot_token.endsWith("...") ? I18n.attr("settings-telegram-token-input-saved", "placeholder") : I18n.attr("settings-telegram-token-input", "placeholder"))}" value="" autocomplete="off">
            <button class="btn btn-secondary btn-sm btn-icon" id="tgToggleToken" data-l10n-id="settings-telegram-token-toggle">
              <i class="icon-eye"></i>
            </button>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label data-l10n-id="settings-telegram-chat-label"></label>
            <span class="settings-field-hint" id="tgChatIdHint">
              ${settings.chat_id ? `<span data-l10n-id="settings-telegram-chat-connected"></span> <bdi dir="ltr">${settings.chat_id}</bdi>` : '<span data-l10n-id="settings-telegram-chat-discover-hint"></span>'}
            </span>
          </div>
          <div class="settings-field-control" id="tgChatIdControl">
            ${
              settings.chat_id
                ? `
              <span class="chat-id-display">
                <code dir="ltr">${settings.chat_id}</code>
                <button class="btn btn-secondary btn-sm" id="tgChangeChatBtn" data-l10n-id="settings-telegram-chat-change">
                  <i class="icon-pencil"></i>
                </button>
              </span>
            `
                : `
              <button class="btn btn-primary btn-sm" id="tgDiscoverBtn">
                <i class="icon-search"></i> <span data-l10n-id="settings-telegram-chat-discover"></span>
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
                <span data-l10n-id="settings-telegram-discovery-step-add"></span>
              </div>
              <div class="discovery-step">
                <span class="step-number">2</span>
                <span data-l10n-id="settings-telegram-discovery-step-privacy"></span>
              </div>
              <div class="discovery-info-box">
                <i class="icon-info"></i>
                <div data-l10n-id="settings-telegram-discovery-privacy" data-l10n-markup></div>
              </div>
              <div class="discovery-step">
                <span class="step-number">3</span>
                <span data-l10n-id="settings-telegram-discovery-step-send"></span>
              </div>
            </div>
            <div class="discovery-spinner">
              <i class="icon-loader spin"></i>
              <span data-l10n-id="settings-telegram-discovery-listening"></span>
            </div>
          </div>
          <div class="discovered-chats-list" id="tgDiscoveredChats">
            <!-- Discovered chats will appear here -->
          </div>
          <div class="discovery-actions">
            <button class="btn btn-secondary btn-sm" id="tgCancelDiscovery">
              <i class="icon-x"></i> <span data-l10n-id="common-action-cancel"></span>
            </button>
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label data-l10n-id="settings-telegram-test-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-telegram-test-hint"></span>
          </div>
          <div class="settings-field-control">
            <button class="btn btn-primary btn-sm" id="tgTestBtn">
              <i class="icon-send"></i> <span data-l10n-id="settings-telegram-test-send"></span>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Authentication Section -->
    <div class="settings-section">
      <h3 class="settings-section-title">
        <i class="icon-shield"></i>
        <span data-l10n-id="settings-telegram-auth-title"></span>
      </h3>
      <p class="settings-section-description" data-l10n-id="settings-telegram-auth-description"></p>

      <div class="settings-group telegram-auth-section" id="tgAuthSection">
        <!-- Command Authentication Subsection -->
        <div class="telegram-auth-subsection">
          <div class="telegram-auth-header">
            <div class="telegram-auth-title">
              <i class="icon-shield"></i>
              <span data-l10n-id="settings-telegram-auth-title"></span>
            </div>
            <div class="telegram-auth-status" id="tg-auth-status" role="status" aria-live="polite">
              <i class="icon-loader spin"></i> <span data-l10n-id="common-loading"></span>
            </div>
          </div>
          <div class="telegram-auth-content" id="tg-auth-content"></div>
        </div>

        <!-- Session Timeout -->
        <div class="telegram-auth-subsection">
          <div class="telegram-auth-header">
            <div class="telegram-auth-title">
              <i class="icon-clock"></i>
              <span data-l10n-id="settings-telegram-timeout-title"></span>
            </div>
          </div>
          <div class="telegram-auth-content">
            <div class="telegram-auth-row">
              <div class="telegram-auth-info">
                <span data-l10n-id="settings-telegram-timeout-description"></span>
              </div>
              <select id="tgSessionTimeout" class="settings-select" data-custom-select>
                <option value="5" ${settings.session_timeout_minutes === 5 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-duration-minutes", { count: 5 }))}</option>
                <option value="15" ${settings.session_timeout_minutes === 15 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-duration-minutes", { count: 15 }))}</option>
                <option value="30" ${settings.session_timeout_minutes === 30 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-duration-minutes", { count: 30 }))}</option>
                <option value="60" ${settings.session_timeout_minutes === 60 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-duration-hours", { count: 1 }))}</option>
                <option value="120" ${settings.session_timeout_minutes === 120 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-duration-hours", { count: 2 }))}</option>
                <option value="1440" ${settings.session_timeout_minutes === 1440 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-duration-hours", { count: 24 }))}</option>
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
        <span data-l10n-id="settings-telegram-sessions-title"></span>
      </h3>
      <div id="tgSessionsList" class="sessions-list">
        ${sessionsHtml}
      </div>
    </div>

    <!-- Notifications Section -->
    <div class="settings-section">
      <h3 class="settings-section-title">
        <i class="icon-bell"></i>
        <span data-l10n-id="settings-telegram-notifications-title"></span>
      </h3>
      <p class="settings-section-description" data-l10n-id="settings-telegram-notifications-description"></p>

      <div class="settings-group">
        <div class="settings-field">
          <div class="settings-field-info">
            <label data-l10n-id="settings-telegram-notify-opened-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-telegram-notify-opened-hint"></span>
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
            <label data-l10n-id="settings-telegram-notify-closed-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-telegram-notify-closed-hint"></span>
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
            <label data-l10n-id="settings-telegram-notify-partial-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-telegram-notify-partial-hint"></span>
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
            <label data-l10n-id="settings-telegram-notify-dca-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-telegram-notify-dca-hint"></span>
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
            <label data-l10n-id="settings-telegram-notify-errors-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-telegram-notify-errors-hint"></span>
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
            <label data-l10n-id="settings-telegram-notify-startup-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-telegram-notify-startup-hint"></span>
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
            <label data-l10n-id="settings-telegram-notify-filtering-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-telegram-notify-filtering-hint"></span>
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
            <label data-l10n-id="settings-telegram-notify-trades-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-telegram-notify-trades-hint"></span>
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
            <label data-l10n-id="settings-telegram-notify-daily-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-telegram-notify-daily-hint"></span>
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
        <span data-l10n-id="settings-telegram-features-title"></span>
      </h3>
      <p class="settings-section-description" data-l10n-id="settings-telegram-features-description"></p>

      <div class="settings-group">
        <div class="settings-field">
          <div class="settings-field-info">
            <label data-l10n-id="settings-telegram-commands-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-telegram-commands-hint"></span>
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
            <label data-l10n-id="settings-telegram-require-2fa-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-telegram-require-2fa-hint"></span>
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
            <label data-l10n-id="settings-telegram-inline-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-telegram-inline-hint"></span>
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
  // Helper to update telegram settings.
  //
  // A saved setting raises NO toast: the control the user just moved is the
  // confirmation. Only a REJECTED save is worth a notice, because the control
  // then shows a value the backend does not have — so it is also reverted, and
  // the notice is keyed so flipping several failing toggles leaves one notice.
  const updateSetting = async (key, value, control = null) => {
    const revert = () => {
      if (control && control.type === "checkbox") control.checked = !value;
    };

    try {
      const response = await fetch("/api/telegram/settings", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ [key]: value }),
      });

      if (!response.ok) {
        const data = await response.json().catch(() => null);
        revert();
        Utils.showToast({
          key: "telegram-setting",
          type: "error",
          title: I18n.t("settings-telegram-setting-save-failed"),
          message: apiErrorMessage(data, null),
        });
      }
    } catch (error) {
      revert();
      Utils.showToast({
        key: "telegram-setting",
        type: "error",
        title: I18n.t("settings-telegram-setting-save-failed"),
        message: error?.message || null,
      });
    }
  };

  // Enable toggle
  const enableToggle = content.querySelector("#tgEnabled");
  if (enableToggle) {
    enableToggle.addEventListener("change", (e) =>
      updateSetting("enabled", e.target.checked, e.target)
    );
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
        Utils.showToast({
          type: "error",
          title: I18n.t("settings-telegram-discovery-start-failed"),
          message: apiErrorMessage(data, null),
        });
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
      Utils.showToast({ type: "error", title: I18n.t("settings-telegram-discovery-start-failed") });
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
          <span class="chat-name">${chat.first_name || chat.username || Utils.escapeHtml(I18n.t("settings-telegram-unknown"))}</span>
          <span class="chat-meta">${Utils.escapeHtml(Object.hasOwn(CHAT_TYPE_LABELS, chat.chat_type) ? I18n.label(CHAT_TYPE_LABELS, chat.chat_type) : chat.chat_type)} • <span data-l10n-id="settings-telegram-chat-id-label"></span> <bdi dir="ltr">${chat.chat_id}</bdi></span>
          ${chat.message_preview ? `<span class="chat-preview">"${chat.message_preview}"</span>` : ""}
        </div>
        <button class="btn btn-success btn-sm select-chat-btn">
          <i class="icon-check"></i> <span data-l10n-id="settings-telegram-discovery-select"></span>
        </button>
      </div>
    `
      )
      .join("");
    I18n.localizeTree(discoveredChatsEl);

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
        Utils.showToast(I18n.t("settings-telegram-chat-selected"), "success");
        stopDiscovery();
        // Reload the Telegram tab
        loadTelegramTab(dialog, content);
      } else {
        const data = await response.json();
        Utils.showToast({
          type: "error",
          title: I18n.t("settings-telegram-chat-select-failed"),
          message: apiErrorMessage(data, null),
        });
      }
    } catch {
      Utils.showToast({ type: "error", title: I18n.t("settings-telegram-chat-select-failed") });
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
      setButton(testBtn, "icon-loader spin", I18n.t("settings-telegram-test-sending"));
      try {
        const response = await fetch("/api/telegram/test", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({}),
        });
        const data = await response.json();
        if (response.ok) {
          Utils.showToast(I18n.t("settings-telegram-test-sent"), "success");
        } else {
          Utils.showToast({
            type: "error",
            title: I18n.t("settings-telegram-test-failed"),
            message: apiErrorMessage(data, null),
          });
        }
      } catch {
        Utils.showToast({ type: "error", title: I18n.t("settings-telegram-test-failed") });
      } finally {
        testBtn.dataset.submitting = "false";
        testBtn.disabled = false;
        setButton(testBtn, "icon-send", I18n.t("settings-telegram-test-send"));
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
          Utils.showToast(I18n.t("settings-telegram-session-revoked"), "success");
          loadTelegramTab(dialog, content);
        } else {
          Utils.showToast({
            type: "error",
            title: I18n.t("settings-telegram-session-revoke-failed"),
          });
        }
      } catch {
        Utils.showToast({
          type: "error",
          title: I18n.t("settings-telegram-session-revoke-failed"),
        });
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
      updateSetting("commands_enabled", e.target.checked, e.target)
    );
  }

  const inlineToggle = content.querySelector("#tgInlineActions");
  if (inlineToggle) {
    inlineToggle.addEventListener("change", (e) =>
      updateSetting("inline_actions", e.target.checked, e.target)
    );
  }

  const require2faToggle = content.querySelector("#tgRequire2fa");
  if (require2faToggle) {
    require2faToggle.addEventListener("change", (e) => {
      updateSetting("commands_require_2fa", e.target.checked, e.target);
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
      throw new Error(lockscreenData.error || I18n.t("settings-security-load-failed"));
    }

    const totpEnabled = lockscreenData.totp_enabled || false;

    // Also check the commands_require_2fa setting
    const require2faToggle = content.querySelector("#tgRequire2fa");
    const require2fa = require2faToggle ? require2faToggle.checked : true;

    renderAuthSection(dialog, statusEl, contentEl, totpEnabled, require2fa);
  } catch (error) {
    if (statusEl) {
      statusEl.innerHTML =
        '<span class="status-error"><i class="icon-circle-alert"></i> <span data-l10n-id="settings-telegram-auth-error"></span></span>';
      I18n.localizeTree(statusEl);
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
      '<span class="status-success"><i class="icon-circle-check"></i> <span data-l10n-id="settings-telegram-auth-protected"></span></span>';
    contentEl.innerHTML = `
      <div class="telegram-auth-row">
        <div class="telegram-auth-info">
          <i class="icon-shield" style="color: var(--success); margin-right: 8px;"></i>
          <span data-l10n-id="settings-telegram-auth-protected-note" data-l10n-markup></span>
        </div>
      </div>
      <div class="telegram-auth-note">
        <i class="icon-info"></i>
        <span><span data-l10n-id="settings-telegram-auth-managed-in"></span> <button type="button" class="link-button" id="tg-goto-security-btn" data-l10n-id="settings-telegram-security-link"></button></span>
      </div>
    `;
  } else if (totpEnabled && !require2fa) {
    statusEl.innerHTML =
      '<span class="status-warning"><i class="icon-circle-alert"></i> <span data-l10n-id="settings-telegram-auth-disabled"></span></span>';
    contentEl.innerHTML = `
      <div class="telegram-auth-row">
        <div class="telegram-auth-info">
          <i class="icon-triangle-alert" style="color: var(--warning); margin-right: 8px;"></i>
          <span data-l10n-id="settings-telegram-auth-disabled-note"></span>
        </div>
      </div>
      <div class="telegram-auth-note">
        <i class="icon-info"></i>
        <span><span data-l10n-id="settings-telegram-auth-managed-in"></span> <button type="button" class="link-button" id="tg-goto-security-btn" data-l10n-id="settings-telegram-security-link"></button></span>
      </div>
    `;
  } else {
    statusEl.innerHTML =
      '<span class="status-warning"><i class="icon-circle-alert"></i> <span data-l10n-id="settings-telegram-auth-not-configured"></span></span>';
    contentEl.innerHTML = `
      <div class="telegram-auth-row">
        <div class="telegram-auth-info">
          <i class="icon-triangle-alert" style="color: var(--warning); margin-right: 8px;"></i>
          <span data-l10n-id="settings-telegram-auth-missing-note"></span>
        </div>
      </div>
      <div class="telegram-auth-note">
        <i class="icon-info"></i>
        <span><span data-l10n-id="settings-telegram-auth-configure-in"></span> <button type="button" class="link-button" id="tg-goto-security-btn" data-l10n-id="settings-telegram-security-link"></button> <span data-l10n-id="settings-telegram-auth-configure-suffix"></span></span>
      </div>
    `;
  }

  I18n.localizeTree(statusEl);
  I18n.localizeTree(contentEl);

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
