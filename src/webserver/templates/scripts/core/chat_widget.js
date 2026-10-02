// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * ChatWidget - Reusable chat component
 *
 * Extracted from the Assistant page (`pages/assistant.js`) so it can serve both
 * the Assistant page chat tab and the global floating chat dialog. All DOM
 * queries are scoped to the provided root element so multiple instances can
 * coexist.
 */
import * as Utils from "./utils.js";
import { formatNumber, formatTimestamp } from "./format.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { AGENT_TOOL_LABELS } from "../ui/agent_tool.js";
import { TOOL_CALL_STATUS_LABELS } from "../ui/tool_call_status.js";
import { playSuccess, playError } from "./sounds.js";
import { apiErrorMessage } from "./request_manager.js";

// Longest message the composer accepts.
const MESSAGE_LIMIT = 4000;

// Message key of each message author role.
const CHAT_ROLE_LABELS = Object.freeze({
  user: "assistant-chat-role-user",
  assistant: "assistant-chat-role-assistant",
});

// Message key of each session date group, in display order.
const SESSION_GROUP_LABELS = Object.freeze({
  today: "assistant-chat-group-today",
  yesterday: "assistant-chat-group-yesterday",
  week: "assistant-chat-group-week",
  older: "assistant-chat-group-older",
});

// Starter prompts. `send` submits the prompt at once; a prompt that ends with an
// open subject is placed in the composer for the user to complete.
function quickPrompts() {
  return [
    {
      icon: "wallet",
      label: I18n.t("assistant-chat-prompt-positions-label"),
      text: I18n.t("assistant-chat-prompt-positions-text"),
      send: true,
    },
    {
      icon: "shield-check",
      label: I18n.t("assistant-chat-prompt-token-label"),
      text: I18n.t("assistant-chat-prompt-token-text"),
      send: false,
    },
    {
      icon: "activity",
      label: I18n.t("assistant-chat-prompt-activity-label"),
      text: I18n.t("assistant-chat-prompt-activity-text"),
      send: true,
    },
  ];
}

export class ChatWidget {
  /**
   * @param {HTMLElement} root - Container element to render chat into
   * @param {Object} opts
   * @param {"page"|"dialog"} [opts.layout="page"] - Host-specific session layout
   * @param {Function} [opts.onClose] - Called when user wants to close (Escape in dialog)
   */
  constructor(root, opts = {}) {
    this.root = root;
    this.opts = { layout: "page", ...opts };
    if (!new Set(["page", "dialog"]).has(this.opts.layout)) {
      throw new Error(`[ChatWidget] Unsupported layout: ${this.opts.layout}`);
    }

    this.state = {
      sessions: [],
      currentSession: null,
      messages: [],
      isLoading: false,
      pendingConfirmation: null,
    };

    this._abortController = null;
    this._sessionLoadGeneration = 0;
    this._messageLoadGeneration = 0;
    this._isDraft = false;
    this._prevSessionsJson = "";
    this._cleanups = [];
    this._pollTimer = null;
    this._destroyed = false;

    this._buildHTML();
    this._setupHandlers();
  }

  // ---------------------------------------------------------------------------
  // Scoped DOM helpers
  // ---------------------------------------------------------------------------

  $(sel) {
    return this.root.querySelector(sel);
  }
  $$(sel) {
    return this.root.querySelectorAll(sel);
  }

  _on(el, evt, fn) {
    if (!el) return;
    el.addEventListener(evt, fn);
    this._cleanups.push(() => el.removeEventListener(evt, fn));
  }

  // ---------------------------------------------------------------------------
  // HTML Template
  // ---------------------------------------------------------------------------

  _buildHTML() {
    const hostClass = ` cw-host-${this.opts.layout}`;
    const quickPromptsHtml = quickPrompts()
      .map(
        (prompt) => `
                <button class="quick-prompt" type="button" data-prompt="${Utils.escapeHtml(prompt.text)}" data-send="${prompt.send}">
                  <i class="icon-${prompt.icon}"></i><span>${Utils.escapeHtml(prompt.label)}</span><i class="icon-arrow-up-right"></i>
                </button>`
      )
      .join("");
    this.root.innerHTML = `
      <div class="chat-widget chat-container${hostClass}">
        <div class="chat-sessions-sidebar">
          <div class="sessions-header">
            <h3 data-l10n-id="assistant-chat-sessions-title"></h3>
            <button class="new-session-btn" type="button" data-l10n-id="assistant-chat-sidebar-new">
              <i class="icon-plus"></i>
            </button>
          </div>
          <div class="sessions-search">
            <i class="icon-search"></i>
            <input type="text" class="cw-sessions-search" data-l10n-id="assistant-chat-search" />
          </div>
          <div class="sessions-list cw-sessions-list"></div>
        </div>
        <button class="chat-sessions-scrim" type="button" data-l10n-id="assistant-chat-history-close"></button>

        <div class="chat-main">
          <div class="chat-header">
            <span class="chat-title cw-chat-title" data-l10n-id="assistant-chat-title-new"></span>
            <div class="chat-actions">
              <button class="chat-action-btn cw-sessions-toggle" type="button" data-l10n-id="assistant-chat-history-open" aria-expanded="false">
                <i class="icon-panel-left"></i>
              </button>
              <button class="chat-action-btn cw-new-session-btn" type="button" data-l10n-id="assistant-chat-header-new">
                <i class="icon-plus"></i>
              </button>
              <button class="chat-action-btn cw-delete-btn" type="button" data-l10n-id="assistant-chat-delete">
                <i class="icon-trash"></i>
              </button>
              ${this.opts.onClose ? '<button class="chat-action-btn cw-close-btn" type="button" data-l10n-id="assistant-chat-close"><i class="icon-x"></i></button>' : ""}
            </div>
          </div>

          <div class="chat-messages cw-chat-messages" aria-live="polite" aria-atomic="false">
            <div class="chat-empty-state cw-empty-state">
              <div class="empty-state-kicker"><i class="icon-bot-message-square"></i><span data-l10n-id="assistant-chat-empty-kicker"></span></div>
              <h3 data-l10n-id="assistant-chat-empty-title"></h3>
              <p class="empty-state-subtitle" data-l10n-id="assistant-chat-empty-subtitle"></p>
              <div class="quick-prompts">${quickPromptsHtml}</div>
            </div>
          </div>

          <section class="tool-confirmation cw-tool-modal" aria-live="polite" hidden>
            <div class="tool-confirmation-copy">
              <div class="tool-confirmation-title"><i class="icon-triangle-alert"></i><strong class="cw-tool-name"></strong></div>
              <p class="cw-tool-description"></p>
              <details class="tool-confirmation-details">
                <summary data-l10n-id="assistant-chat-tool-review"></summary>
                <pre class="cw-tool-input tool-call-code"></pre>
              </details>
            </div>
            <div class="confirmation-actions">
              <button class="btn btn-secondary cw-deny-tool" type="button" data-l10n-id="assistant-chat-tool-deny"></button>
              <button class="btn btn-primary cw-confirm-tool" type="button" data-l10n-id="assistant-chat-tool-allow"></button>
            </div>
          </section>

          <div class="chat-input-area">
            <div class="chat-context cw-chat-context"></div>
            <div class="chat-input-container cw-input-container">
              <div class="chat-input-wrapper">
                <textarea class="cw-chat-input" rows="1" data-l10n-id="assistant-chat-input"></textarea>
                <div class="input-hint cw-input-hint">
                  <kbd data-l10n-id="assistant-chat-hint-key-enter"></kbd><span data-l10n-id="assistant-chat-hint-send"></span>
                  <span>·</span>
                  <kbd data-l10n-id="assistant-chat-hint-key-shift"></kbd><kbd data-l10n-id="assistant-chat-hint-key-enter"></kbd><span data-l10n-id="assistant-chat-hint-newline"></span>
                </div>
              </div>
              <div class="chat-input-actions">
                <button class="send-btn cw-send-btn" type="button" disabled data-l10n-id="assistant-chat-send">
                  <i class="icon-send"></i>
                </button>
              </div>
            </div>
            <div class="chat-input-footer">
              <span class="input-status cw-input-status"></span>
              <span class="char-count cw-char-count"></span>
            </div>
          </div>
        </div>
      </div>
    `;
    I18n.localizeTree(this.root);
  }

  // ---------------------------------------------------------------------------
  // Event Handlers Setup
  // ---------------------------------------------------------------------------

  _setupHandlers() {
    // New session button
    this._on(this.$(".new-session-btn"), "click", () => this.createSession());
    this._on(this.$(".cw-new-session-btn"), "click", () => this.createSession());
    this._on(this.$(".cw-sessions-toggle"), "click", () => {
      const container = this.$(".chat-container");
      this._setSessionsOpen(!container?.classList.contains("sessions-open"));
    });
    this._on(this.$(".chat-sessions-scrim"), "click", () => this._setSessionsOpen(false));
    this._on(this.$(".cw-close-btn"), "click", () => this.opts.onClose?.());

    // Sessions search
    this._on(this.$(".cw-sessions-search"), "input", () => this._renderSessions());

    // Send button
    this._on(this.$(".cw-send-btn"), "click", () => {
      if (this.state.isLoading) this.cancelRequest();
      else this.sendMessage();
    });

    // Chat input
    const input = this.$(".cw-chat-input");
    this._on(input, "input", () => this._handleInputChange());
    this._on(input, "keydown", (e) => this._handleKeydown(e));

    // Tool confirmation buttons
    this._on(this.$(".cw-confirm-tool"), "click", () => this.confirmTool(true));
    this._on(this.$(".cw-deny-tool"), "click", () => this.confirmTool(false));
    // Quick prompt buttons
    this.$$(".quick-prompt").forEach((btn) => {
      this._on(btn, "click", () => {
        const prompt = btn.getAttribute("data-prompt");
        if (!prompt) return;
        const chatInput = this.$(".cw-chat-input");
        if (!chatInput) return;
        const sendNow = btn.dataset.send === "true";
        chatInput.value = sendNow ? prompt : `${prompt} `;
        chatInput.focus();
        chatInput.dispatchEvent(new Event("input", { bubbles: true }));
        if (sendNow) this.sendMessage();
      });
    });

    // Message actions (copy, regenerate) via delegation
    const msgs = this.$(".cw-chat-messages");
    this._on(msgs, "click", (e) => {
      const actionBtn = e.target.closest(".message-action-btn");
      if (!actionBtn) return;
      const action = actionBtn.dataset.action;
      if (action === "copy") {
        const content = actionBtn.dataset.content;
        navigator.clipboard
          .writeText(content)
          .then(() => {
            const icon = actionBtn.querySelector("i");
            const orig = icon.className;
            icon.className = "icon-check";
            setTimeout(() => (icon.className = orig), 1500);
            Utils.notifyCopied(I18n.t("assistant-chat-copied-message"));
          })
          .catch(() =>
            Utils.showToast({ type: "error", title: I18n.t("assistant-chat-copy-failed") })
          );
      } else if (action === "regenerate") {
        this.regenerateLastMessage();
      }
    });

    // Session items (select / delete) via delegation
    const sessionsList = this.$(".cw-sessions-list");
    if (sessionsList) {
      this._on(sessionsList, "click", (e) => {
        // Delete button
        const delBtn = e.target.closest(".session-delete");
        if (delBtn) {
          e.stopPropagation();
          const id = delBtn.closest(".session-item")?.dataset.sessionId;
          if (id) this.deleteSession(id);
          return;
        }
        // Session item
        const item = e.target.closest(".session-item");
        if (item?.dataset.sessionId) {
          this.selectSession(item.dataset.sessionId);
        }
      });
    }
  }

  // ---------------------------------------------------------------------------
  // Public API
  // ---------------------------------------------------------------------------

  /** Start polling sessions every intervalMs */
  startPolling(intervalMs = 3000) {
    this.stopPolling();
    this._pollTimer = setInterval(() => {
      if (!this._destroyed) this.loadSessions();
    }, intervalMs);
  }

  stopPolling() {
    if (this._pollTimer) {
      clearInterval(this._pollTimer);
      this._pollTimer = null;
    }
  }

  async loadSessions() {
    const generation = ++this._sessionLoadGeneration;
    try {
      const response = await fetch("/api/assistant/chat/sessions");
      if (!response.ok) throw new Error("Failed to load chat sessions");

      const data = await response.json();
      if (generation !== this._sessionLoadGeneration) return;
      this.state.sessions = Array.isArray(data) ? data : data.sessions || [];

      this._renderSessions();

      if (!this.state.currentSession && !this._isDraft && this.state.sessions.length > 0) {
        await this.selectSession(this.state.sessions[0].id);
      } else if (!this.state.currentSession && this.state.sessions.length === 0) {
        this._showDraft();
      } else if (this.state.currentSession) {
        const cur = this.state.sessions.find((s) => s.id === this.state.currentSession);
        if (cur && !this.state.isLoading) await this._loadMessages(cur);
        else if (!cur && !this.state.isLoading) this._showDraft();
      }
    } catch (error) {
      if (generation !== this._sessionLoadGeneration) return;
      console.error("[ChatWidget] Error loading sessions:", error);
      Utils.showToast({
        key: "chat-sessions",
        type: "error",
        title: I18n.t("assistant-chat-sessions-load-failed"),
      });
    }
  }

  async createSession() {
    if (this.state.isLoading) this.cancelRequest();
    this._showDraft();
    this.$(".cw-chat-input")?.focus();
  }

  async selectSession(sessionId) {
    if (this.state.isLoading) this.cancelRequest();
    const numericId = typeof sessionId === "string" ? parseInt(sessionId, 10) : sessionId;
    this._isDraft = false;
    this.state.currentSession = numericId;

    const session = this.state.sessions.find((s) => s.id === numericId);
    if (!session) {
      console.error("[ChatWidget] Session not found:", numericId);
      return;
    }

    this._renderSessions();
    this._updateChatHeader(session);
    this._showChatInterface();
    this._setSessionsOpen(false);
    await this._loadMessages(session, true);
  }

  async deleteSession(sessionId) {
    if (this.state.isLoading) this.cancelRequest();
    try {
      const confirmed = await ConfirmationDialog.show({
        title: I18n.t("assistant-chat-delete-title"),
        message: I18n.t("assistant-chat-delete-message"),
        confirmText: I18n.t("common-action-delete"),
        cancelText: I18n.t("common-action-cancel"),
        type: "danger",
      });
      if (!confirmed) return;

      const response = await fetch(`/api/assistant/chat/sessions/${sessionId}`, {
        method: "DELETE",
      });
      if (!response.ok) throw new Error("Failed to delete session");

      playSuccess();

      if (this.state.currentSession === Number(sessionId)) {
        this.state.currentSession = null;
        this.state.messages = [];
      }

      await this.loadSessions();
      Utils.showToast({ type: "success", title: I18n.t("assistant-chat-delete-done") });
    } catch (error) {
      console.error("[ChatWidget] Error deleting session:", error);
      playError();
      Utils.showToast({ type: "error", title: I18n.t("assistant-chat-delete-failed") });
    }
  }

  async generateSessionTitle(sessionId) {
    try {
      const response = await fetch(`/api/assistant/chat/sessions/${sessionId}/generate-title`, {
        method: "POST",
      });
      if (!response.ok) return;

      const data = await response.json();
      if (data.title) {
        const session = this.state.sessions.find((s) => s.id === sessionId);
        if (session) {
          session.title = data.title;
          this._renderSessions();
          this._updateChatHeader(session);
        }
      }
    } catch (error) {
      console.warn("[ChatWidget] Error generating title:", error);
    }
  }

  async sendMessage() {
    if (this.state.isLoading) return;
    const input = this.$(".cw-chat-input");
    if (!input) return;

    const message = input.value.trim();
    if (!message) return;

    if (message.length > MESSAGE_LIMIT) {
      Utils.showToast({
        type: "error",
        title: I18n.t("assistant-chat-message-too-long-title"),
        message: I18n.t("assistant-chat-message-too-long-message", {
          limit: formatNumber(MESSAGE_LIMIT, 0),
        }),
      });
      return;
    }

    // Auto-create session if none exists
    if (!this.state.currentSession) {
      try {
        const response = await fetch("/api/assistant/chat/sessions", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({}),
        });
        if (!response.ok) throw new Error("Failed to create session");
        const data = await response.json();
        this.state.currentSession = data.session_id;
        this._isDraft = false;
        await this.loadSessions();
        this._renderSessions();
        this._showChatInterface();
      } catch (error) {
        console.error("[ChatWidget] Error auto-creating session:", error);
        Utils.showToast({ type: "error", title: I18n.t("assistant-chat-start-failed") });
        return;
      }
    }

    if (this._abortController) this._abortController.abort();
    this._abortController = new AbortController();
    const controller = this._abortController;
    const signal = this._abortController.signal;
    const sessionId = this.state.currentSession;

    input.value = "";
    input.style.height = "auto";
    this.state.isLoading = true;

    this._updateSendButton();
    this._updateCharCount();
    this._updateInputStatus("");

    const userMessage = { role: "user", content: message, timestamp: new Date().toISOString() };
    this.state.messages.push(userMessage);
    this._renderMessages();
    this._showTypingIndicator();

    try {
      const data = await this._streamChat({ session_id: sessionId, message }, signal, (event) =>
        this._updateAgentProgress(event)
      );
      if (signal.aborted) return;
      if (this.state.currentSession !== sessionId) return;

      this._hideTypingIndicator();
      this._updateInputStatus("");

      const streamedError = data.error; // api-body-ok: presence check of the streamed envelope, rendered by apiErrorMessage
      if (streamedError) {
        throw new Error(apiErrorMessage(data, I18n.t("assistant-chat-error-unknown")));
      }

      if (data.content !== undefined) {
        this.state.messages.push({
          id: data.message_id,
          role: "assistant",
          content: data.content || "",
          tool_calls: data.tool_calls || [],
          timestamp: new Date().toISOString(),
        });
        this._renderMessages();
      }

      if (data.pending_confirmations?.length > 0) {
        this.state.pendingConfirmation = data.pending_confirmations[0];
        this._showToolConfirmation(data.pending_confirmations[0]);
      }

      await this.loadSessions();

      if (this.state.messages.length === 2) {
        this.generateSessionTitle(sessionId);
      }
    } catch (error) {
      if (error.name === "AbortError") return;

      console.error("[ChatWidget] Error sending message:", error);
      playError();
      this._hideTypingIndicator();
      this.state.messages = this.state.messages.filter((item) => item !== userMessage);
      this._renderMessagesForce();
      if (!input.value.trim()) {
        input.value = message;
        this._handleInputChange();
      }

      const container = this.$(".cw-input-container");
      if (container) {
        container.classList.add("has-error");
        setTimeout(() => container.classList.remove("has-error"), 400);
      }

      this._updateInputStatus(
        `<i class="icon-circle-alert"></i> ${Utils.escapeHtml(I18n.t("assistant-chat-send-failed"))}`,
        "error"
      );
    } finally {
      if (this._abortController === controller) this._abortController = null;
      this.state.isLoading = false;
      this._updateSendButton();
      input.focus();
    }
  }

  async regenerateLastMessage() {
    const lastUserIndex = this.state.messages.map((m) => m.role).lastIndexOf("user");
    if (lastUserIndex === -1) {
      Utils.showToast({ type: "error", title: I18n.t("assistant-chat-regenerate-none") });
      return;
    }

    const lastUserMessage = this.state.messages[lastUserIndex].content;
    const previousAssistant = this.state.messages[lastUserIndex + 1];
    if (!previousAssistant?.id || previousAssistant.role !== "assistant") {
      Utils.showToast({
        type: "error",
        title: I18n.t("assistant-chat-regenerate-reload"),
      });
      return;
    }
    const previousMessages = this.state.messages;
    this.state.messages = this.state.messages.slice(0, lastUserIndex + 1);

    if (this._abortController) this._abortController.abort();
    this._abortController = new AbortController();
    const signal = this._abortController.signal;

    this._renderMessages();
    this._showTypingIndicator();
    this.state.isLoading = true;
    this._updateSendButton();
    this._updateInputStatus("");

    try {
      const data = await this._streamChat(
        {
          session_id: this.state.currentSession,
          message: lastUserMessage,
          regenerate_message_id: previousAssistant.id,
        },
        signal,
        (event) => this._updateAgentProgress(event)
      );
      if (signal.aborted) return;
      this._hideTypingIndicator();
      this._updateInputStatus("");

      const streamedError = data.error; // api-body-ok: presence check of the streamed envelope, rendered by apiErrorMessage
      if (streamedError) {
        throw new Error(apiErrorMessage(data, I18n.t("assistant-chat-error-unknown")));
      }

      if (data.content !== undefined) {
        this.state.messages.push({
          id: data.message_id,
          role: "assistant",
          content: data.content || "",
          tool_calls: data.tool_calls || [],
          timestamp: new Date().toISOString(),
        });
        this._renderMessages();
      }

      if (data.pending_confirmations?.length > 0) {
        this.state.pendingConfirmation = data.pending_confirmations[0];
        this._showToolConfirmation(data.pending_confirmations[0]);
      }

      await this.loadSessions();

      Utils.showToast({
        type: "success",
        title: I18n.t("assistant-chat-regenerate-done-title"),
        message: I18n.t("assistant-chat-regenerate-done-message"),
      });
    } catch (error) {
      if (error.name === "AbortError") return;
      console.error("[ChatWidget] Error regenerating:", error);
      this.state.messages = previousMessages;
      this._renderMessagesForce();
      playError();
      this._hideTypingIndicator();
      this._updateInputStatus(
        `<i class="icon-circle-alert"></i> ${Utils.escapeHtml(error.message || I18n.t("assistant-chat-regenerate-failed"))}`,
        "error"
      );
      setTimeout(() => this._updateInputStatus(""), 5000);
      Utils.showToast({
        type: "error",
        title: error.message || I18n.t("assistant-chat-regenerate-failed-toast"),
      });
    } finally {
      this._abortController = null;
      this.state.isLoading = false;
      this._updateSendButton();
    }
  }

  cancelRequest() {
    if (!this._abortController) return;
    this._abortController.abort();
    this._abortController = null;

    const input = this.$(".cw-chat-input");
    if (input) {
      input.focus();
    }

    this.state.isLoading = false;
    this._hideTypingIndicator();
    this._updateSendButton();
    this._updateInputStatus(Utils.escapeHtml(I18n.t("assistant-chat-cancelled-message")), "");
    setTimeout(() => this._updateInputStatus(""), 2000);

    Utils.showToast({
      type: "info",
      title: I18n.t("assistant-chat-cancelled-title"),
      message: I18n.t("assistant-chat-cancelled-message"),
    });
  }

  async confirmTool(approved) {
    const confirmation = this.state.pendingConfirmation;
    if (!confirmation) return;

    this._hideToolConfirmation();

    try {
      const response = await fetch(`/api/assistant/chat/confirm/${confirmation.confirmation_id}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ approved, session_id: this.state.currentSession }),
      });
      if (!response.ok) throw new Error("Failed to confirm tool");

      const data = await response.json();

      if (approved) {
        playSuccess();
        Utils.showToast({ type: "success", title: I18n.t("assistant-chat-tool-executed") });
      } else {
        Utils.showToast({
          type: "info",
          title: I18n.t("assistant-chat-cancelled-title"),
          message: I18n.t("assistant-chat-tool-cancelled"),
        });
      }

      this.state.pendingConfirmation = data.pending_confirmations?.[0] || null;
      await this.loadSessions();
      if (this.state.pendingConfirmation)
        this._showToolConfirmation(this.state.pendingConfirmation);
    } catch (error) {
      console.error("[ChatWidget] Error confirming tool:", error);
      playError();
      Utils.showToast({ type: "error", title: I18n.t("assistant-chat-tool-confirm-failed") });
      this._showToolConfirmation(confirmation);
    }
  }

  /** Clean up timers, listeners, abort controllers */
  destroy() {
    this._destroyed = true;
    this.stopPolling();
    if (this._abortController) {
      this._abortController.abort();
      this._abortController = null;
    }
    this._cleanups.forEach((fn) => fn());
    this._cleanups.length = 0;
  }

  // ---------------------------------------------------------------------------
  // Private - Messages
  // ---------------------------------------------------------------------------

  async _loadMessages(session, forceRender = false) {
    if (!session?.id) return;

    const generation = ++this._messageLoadGeneration;
    const sessionId = session.id;

    try {
      const response = await fetch(`/api/assistant/chat/sessions/${session.id}`);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);

      const data = await response.json();
      if (generation !== this._messageLoadGeneration || this.state.currentSession !== sessionId)
        return;
      const newMessages = data.messages || [];

      if (!forceRender && this.state.messages.length === newMessages.length) {
        const lastOld = this.state.messages[this.state.messages.length - 1];
        const lastNew = newMessages[newMessages.length - 1];
        if (lastOld?.id === lastNew?.id) return;
      }

      this.state.messages = newMessages;

      this._renderMessagesForce();
    } catch (error) {
      if (generation !== this._messageLoadGeneration || this.state.currentSession !== sessionId)
        return;
      console.error("[ChatWidget] Error loading messages:", error.message || error);
      this.state.messages = [];
      this._renderMessagesForce();
    }
  }

  // ---------------------------------------------------------------------------
  // Private - Render
  // ---------------------------------------------------------------------------

  _renderSessions() {
    const container = this.$(".cw-sessions-list");
    if (!container) return;

    const searchInput = this.$(".cw-sessions-search");
    const searchQuery = searchInput?.value?.toLowerCase().trim() || "";

    let sessions = [...this.state.sessions];
    if (searchQuery) {
      sessions = sessions.filter(
        (s) =>
          (s.title || "").toLowerCase().includes(searchQuery) ||
          (s.summary || "").toLowerCase().includes(searchQuery)
      );
    }

    sessions.sort(
      (a, b) => new Date(b.updated_at || b.created_at) - new Date(a.updated_at || a.created_at)
    );

    if (sessions.length === 0) {
      container.innerHTML = `
        <div class="sessions-empty">
          <i class="icon-message-square"></i>
          <p>${searchQuery ? Utils.escapeHtml(I18n.t("assistant-chat-sessions-empty-search")) : Utils.escapeHtml(I18n.t("assistant-chat-sessions-empty"))}</p>
          ${!searchQuery ? `<button class="btn btn-sm cw-empty-new-session"><i class="icon-plus"></i> ${Utils.escapeHtml(I18n.t("assistant-chat-sessions-new"))}</button>` : ""}
        </div>`;
      this._prevSessionsJson = "";
      // Wire up the empty-state new-session button
      const btn = container.querySelector(".cw-empty-new-session");
      if (btn) btn.onclick = () => this.createSession();
      return;
    }

    const fp =
      JSON.stringify(
        sessions.map((s) => ({
          id: s.id,
          title: s.title,
          message_count: s.message_count,
          updated_at: s.updated_at,
        }))
      ) +
      "|" +
      this.state.currentSession +
      "|" +
      searchQuery;

    if (fp === this._prevSessionsJson) return;
    this._prevSessionsJson = fp;

    const groups = this._groupSessionsByDate(sessions);
    let html = "";

    for (const [groupId, groupSessions] of Object.entries(groups)) {
      if (groupSessions.length === 0) continue;
      html += '<div class="sessions-group">';
      html += `<div class="sessions-group-header">${Utils.escapeHtml(I18n.label(SESSION_GROUP_LABELS, groupId))}</div>`;

      for (const session of groupSessions) {
        const isActive = session.id === this.state.currentSession;
        const title = Utils.escapeHtml(session.title || I18n.t("assistant-chat-title-new"));
        const preview = session.summary ? Utils.escapeHtml(session.summary.substring(0, 60)) : "";

        html += `
          <div class="session-item ${isActive ? "active" : ""}" data-session-id="${session.id}">
            <div class="session-info">
              <div class="session-title" dir="auto">${title}</div>
              ${preview ? `<div class="session-preview" dir="auto">${preview}${session.summary.length > 60 ? "..." : ""}</div>` : ""}
            </div>
            ${isActive ? '<button class="session-delete" type="button"><i class="icon-trash-2"></i></button>' : ""}
          </div>`;
      }
      html += "</div>";
    }

    container.innerHTML = html;
  }

  _renderMessages() {
    const container = this.$(".cw-chat-messages");
    if (!container) return;

    const emptyState = container.querySelector(".chat-empty-state");

    if (this.state.messages.length === 0) {
      if (emptyState) emptyState.style.display = "flex";
      container.querySelectorAll(".message").forEach((el) => el.remove());
      return;
    }

    if (emptyState) emptyState.style.display = "none";

    const existing = container.querySelectorAll(".message");
    const existingCount = existing.length;
    const newCount = this.state.messages.length;

    if (newCount > existingCount) {
      const frag = document.createDocumentFragment();
      for (let i = existingCount; i < newCount; i++) {
        const wrapper = document.createElement("div");
        wrapper.innerHTML = this._renderMessage(this.state.messages[i]);
        frag.appendChild(wrapper.firstElementChild);
      }
      container.appendChild(frag);
      this._setupToolExpandHandlers();
      this._scrollToBottom();
    } else if (newCount < existingCount) {
      container.innerHTML = "";
      if (emptyState) container.appendChild(emptyState);
      emptyState.style.display = "none";
      container.insertAdjacentHTML(
        "beforeend",
        this.state.messages.map((m) => this._renderMessage(m)).join("")
      );
      this._setupToolExpandHandlers();
      this._scrollToBottom();
    }
  }

  _renderMessagesForce() {
    const container = this.$(".cw-chat-messages");
    if (!container) return;

    const emptyState = container.querySelector(".chat-empty-state");

    if (this.state.messages.length === 0) {
      container.querySelectorAll(".message").forEach((el) => el.remove());
      if (emptyState) emptyState.style.display = "flex";
      return;
    }

    if (emptyState) emptyState.style.display = "none";

    container.querySelectorAll(".message").forEach((el) => el.remove());
    container.insertAdjacentHTML(
      "beforeend",
      this.state.messages.map((m) => this._renderMessage(m)).join("")
    );
    this._setupToolExpandHandlers();
    this._scrollToBottom();
  }

  _formatMarkdown(text) {
    if (!text) return "";
    let html = Utils.escapeHtml(text);

    // Store code blocks to protect from further processing
    const codeBlocks = [];
    html = html.replace(/```(?:\w*)\n?([\s\S]*?)```/g, (_match, code) => {
      const idx = codeBlocks.length;
      codeBlocks.push(`<pre class="chat-code-block"><code>${code.trim()}</code></pre>`);
      return `\x00CODEBLOCK${idx}\x00`;
    });

    // Inline code (protect from further processing)
    const inlineCodes = [];
    html = html.replace(/`([^`\n]+)`/g, (_match, code) => {
      const idx = inlineCodes.length;
      inlineCodes.push(`<code class="chat-inline-code">${code}</code>`);
      return `\x00INLINE${idx}\x00`;
    });

    const inline = (line) =>
      line
        .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
        .replace(/(?<!\w)\*(?!\s)(.+?)(?<!\s)\*(?!\w)/g, "<em>$1</em>");

    // Block structure, line by line: headings, bullet and numbered lists, and
    // paragraphs separated by blank lines. A code block keeps its own line.
    const blocks = [];
    let paragraph = [];
    let list = null;
    const flushParagraph = () => {
      if (paragraph.length)
        blocks.push(`<p class="chat-md-paragraph">${paragraph.join("<br>")}</p>`);
      paragraph = [];
    };
    const flushList = () => {
      if (list) {
        const items = list.items.map((item) => `<li>${item}</li>`).join("");
        blocks.push(`<${list.tag} class="chat-md-list">${items}</${list.tag}>`);
      }
      list = null;
    };

    for (const rawLine of html.split("\n")) {
      const line = rawLine.trimEnd();
      if (!line.trim()) {
        flushParagraph();
        flushList();
        continue;
      }
      const trimmed = line.trim();
      if (trimmed.startsWith("\x00CODEBLOCK") && trimmed.endsWith("\x00")) {
        flushParagraph();
        flushList();
        blocks.push(trimmed);
        continue;
      }
      const heading = /^#{1,6}\s+(.+)$/.exec(line);
      if (heading) {
        flushParagraph();
        flushList();
        blocks.push(`<p class="chat-md-heading">${inline(heading[1])}</p>`);
        continue;
      }
      const bullet = /^\s*[-*+]\s+(.+)$/.exec(line);
      const numbered = bullet ? null : /^\s*\d+[.)]\s+(.+)$/.exec(line);
      if (bullet || numbered) {
        flushParagraph();
        const tag = bullet ? "ul" : "ol";
        if (list && list.tag !== tag) flushList();
        list ??= { tag, items: [] };
        list.items.push(inline((bullet || numbered)[1]));
        continue;
      }
      flushList();
      paragraph.push(inline(line));
    }
    flushParagraph();
    flushList();
    html = blocks.join("");

    // Restore inline codes
    inlineCodes.forEach((code, idx) => {
      html = html.replace(`\x00INLINE${idx}\x00`, code);
    });

    // Restore code blocks
    codeBlocks.forEach((block, idx) => {
      html = html.replace(`\x00CODEBLOCK${idx}\x00`, block);
    });

    return html;
  }

  _renderMessage(msg) {
    const isUser = msg.role === "user";
    const messageTime = msg.timestamp || msg.created_at;
    const timestamp = messageTime
      ? formatTimestamp(new Date(messageTime), { includeDate: false, includeSeconds: false })
      : "";

    let parsedToolCalls = msg.tool_calls;
    if (typeof parsedToolCalls === "string") {
      try {
        parsedToolCalls = JSON.parse(parsedToolCalls);
      } catch {
        parsedToolCalls = null;
      }
    }

    const toolCallsHtml =
      parsedToolCalls && Array.isArray(parsedToolCalls) && parsedToolCalls.length > 0
        ? parsedToolCalls.map((t) => this._renderToolCall(t)).join("")
        : "";

    const actionsHtml = msg.content
      ? `<div class="message-actions" aria-label="${Utils.escapeHtml(I18n.attr("assistant-chat-message-actions", "aria-label"))}">
          <button class="message-action-btn" type="button" title="${Utils.escapeHtml(I18n.attr("assistant-chat-message-copy", "title"))}" aria-label="${Utils.escapeHtml(I18n.attr("assistant-chat-message-copy", "aria-label"))}" data-action="copy" data-content="${Utils.escapeHtml(msg.content)}">
            <i class="icon-copy"></i>
          </button>
          ${!isUser ? `<button class="message-action-btn" type="button" title="${Utils.escapeHtml(I18n.attr("assistant-chat-message-regenerate", "title"))}" aria-label="${Utils.escapeHtml(I18n.attr("assistant-chat-message-regenerate", "aria-label"))}" data-action="regenerate"><i class="icon-refresh-cw"></i></button>` : ""}
        </div>`
      : "";

    return `
      <div class="message ${isUser ? "user" : "assistant"}">
        ${isUser ? "" : '<div class="message-avatar" aria-hidden="true"><i class="icon-bot"></i></div>'}
        <div class="message-content">
          <div class="message-author">${Utils.escapeHtml(I18n.label(CHAT_ROLE_LABELS, isUser ? "user" : "assistant"))}</div>
          ${toolCallsHtml}
          ${msg.content ? `<div class="message-bubble" dir="auto">${isUser ? Utils.escapeHtml(msg.content) : this._formatMarkdown(msg.content)}</div>` : ""}
          <div class="message-footer"><div class="message-meta">${timestamp}</div>${actionsHtml}</div>
        </div>
      </div>`;
  }

  _renderToolCall(tool) {
    const statusRaw = tool.status || "pending";
    const statusClass = statusRaw.toLowerCase();
    const statusText = I18n.label(
      TOOL_CALL_STATUS_LABELS,
      Object.hasOwn(TOOL_CALL_STATUS_LABELS, statusClass) ? statusClass : "pending"
    );
    const statusIcon =
      statusClass === "executed" || statusClass === "success"
        ? "circle-check"
        : statusClass === "failed" || statusClass === "denied"
          ? "circle-x"
          : "clock-3";

    const toolId = tool.tool_name || tool.name;
    const toolName = toolId || I18n.t("assistant-chat-tool-unknown");
    const toolLabel = toolId ? I18n.label(AGENT_TOOL_LABELS, toolId) : toolName;

    return `
      <div class="tool-call ${statusClass}">
        <button class="tool-call-header" type="button" aria-expanded="false">
          <span class="tool-call-title" title="${Utils.escapeHtml(toolName)}"><i class="icon-wrench"></i><span>${Utils.escapeHtml(toolLabel)}</span></span>
          <span class="tool-call-status ${statusClass}"><i class="icon-${statusIcon}"></i><span>${Utils.escapeHtml(statusText)}</span></span>
          <i class="tool-call-expand-icon icon-chevron-down" aria-hidden="true"></i>
        </button>
        <div class="tool-call-body" hidden>
          <div class="tool-call-section">
            <div class="tool-call-label">${Utils.escapeHtml(I18n.t("assistant-chat-tool-section-input"))}</div>
            <div class="tool-call-input"><pre class="tool-call-code">${Utils.escapeHtml(JSON.stringify(tool.input || {}, null, 2))}</pre></div>
          </div>
          ${tool.output ? `<div class="tool-call-section"><div class="tool-call-label">${Utils.escapeHtml(I18n.t("assistant-chat-tool-section-output"))}</div><div class="tool-call-output"><pre class="tool-call-code">${Utils.escapeHtml(JSON.stringify(tool.output, null, 2))}</pre></div></div>` : ""}
          ${tool.error ? `<div class="tool-call-section"><div class="tool-call-label">${Utils.escapeHtml(I18n.t("assistant-chat-tool-section-error"))}</div><div class="tool-call-error"><pre class="tool-call-code">${Utils.escapeHtml(tool.error)}</pre></div></div>` : ""}
        </div>
      </div>`;
  }

  // ---------------------------------------------------------------------------
  // Private - UI helpers
  // ---------------------------------------------------------------------------

  _setupToolExpandHandlers() {
    this.$$(".tool-call-header").forEach((header) => {
      header.onclick = () => {
        const body = header.closest(".tool-call")?.querySelector(".tool-call-body");
        if (!body) return;
        body.hidden = !body.hidden;
        header.classList.toggle("expanded", !body.hidden);
        header.setAttribute("aria-expanded", String(!body.hidden));
      };
    });
  }

  _showTypingIndicator() {
    const container = this.$(".cw-chat-messages");
    if (!container || container.querySelector(".typing-indicator")) return;

    const indicator = document.createElement("div");
    indicator.className = "typing-indicator";
    indicator.setAttribute("role", "status");
    indicator.setAttribute("aria-label", I18n.attr("assistant-chat-typing", "aria-label"));
    indicator.innerHTML = `
      <div class="message-avatar"><i class="icon-bot"></i></div>
      <div class="typing-content">
        <span class="message-author">${Utils.escapeHtml(I18n.label(CHAT_ROLE_LABELS, "assistant"))}</span>
        <div class="agent-progress-line"><span class="typing-label">${Utils.escapeHtml(I18n.t("assistant-chat-progress-preparing"))}</span>
        <span class="typing-dots" aria-hidden="true">
          <span class="typing-dot"></span><span class="typing-dot"></span><span class="typing-dot"></span>
        </span></div>
        <div class="agent-progress-tools"></div>
      </div>`;
    container.appendChild(indicator);
    this._scrollToBottom();
  }

  _hideTypingIndicator() {
    const el = this.$(".typing-indicator");
    if (el) el.remove();
  }

  _updateAgentProgress(event) {
    const indicator = this.$(".typing-indicator");
    if (!indicator) return;
    const label = indicator.querySelector(".typing-label");
    const tools = indicator.querySelector(".agent-progress-tools");
    if (event.type === "thinking") {
      if (label)
        label.textContent =
          event.iteration > 0
            ? I18n.t("assistant-chat-progress-reviewing")
            : I18n.t("assistant-chat-progress-planning");
      return;
    }
    if (event.type === "tool_started") {
      if (label) label.textContent = I18n.t("assistant-chat-progress-using-tools");
      if (!tools) return;
      const row = document.createElement("div");
      row.className = "agent-progress-tool running";
      row.dataset.toolName = event.tool_name;
      row.innerHTML = `<i class="icon-clock-3"></i><span>${Utils.escapeHtml(I18n.label(AGENT_TOOL_LABELS, event.tool_name))}</span><small>${Utils.escapeHtml(I18n.t("assistant-chat-progress-running"))}</small>`;
      tools.appendChild(row);
      this._scrollToBottom();
      return;
    }
    if (event.type === "tool_finished" && tools) {
      const call = event.tool_call;
      const row = Array.from(tools.querySelectorAll(".agent-progress-tool")).find(
        (item) => item.dataset.toolName === call.tool_name && item.classList.contains("running")
      );
      if (row) {
        const failed = String(call.status).toLowerCase() === "failed";
        row.className = `agent-progress-tool ${failed ? "failed" : "complete"}`;
        row.querySelector("i").className = failed ? "icon-circle-x" : "icon-circle-check";
        row.querySelector("small").textContent = failed
          ? I18n.t("assistant-chat-progress-failed")
          : I18n.t("assistant-chat-progress-complete");
      }
    }
  }

  async _streamChat(payload, signal, onEvent) {
    const response = await fetch("/api/assistant/chat/stream", {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "text/event-stream" },
      body: JSON.stringify(payload),
      signal,
    });
    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        apiErrorMessage(
          errorData,
          I18n.t("assistant-chat-stream-http", { status: String(response.status) })
        )
      );
    }
    if (!response.body) throw new Error(I18n.t("assistant-chat-stream-unavailable"));

    const reader = response.body.getReader();
    const decoder = new window.TextDecoder();
    let buffer = "";
    let finalResponse = null;
    while (true) {
      const { value, done } = await reader.read();
      buffer += decoder.decode(value || new Uint8Array(), { stream: !done });
      const frames = buffer.split("\n\n");
      buffer = frames.pop() || "";
      for (const frame of frames) {
        const data = frame
          .split("\n")
          .filter((line) => line.startsWith("data:"))
          .map((line) => line.slice(5).trimStart())
          .join("\n");
        if (!data) continue;
        const event = JSON.parse(data);
        if (event.type === "error") {
          throw new Error(
            apiErrorMessage(
              { error: { text: event.text, details: event.details } },
              I18n.t("assistant-chat-stream-failed")
            )
          );
        }
        onEvent?.(event);
        if (event.type === "complete") finalResponse = event.response;
      }
      if (done) break;
    }
    if (!finalResponse) throw new Error(I18n.t("assistant-chat-stream-incomplete"));
    return finalResponse;
  }

  _scrollToBottom() {
    const container = this.$(".cw-chat-messages");
    if (container) container.scrollTo({ top: container.scrollHeight, behavior: "smooth" });
  }

  _showToolConfirmation(confirmation) {
    const modal = this.$(".cw-tool-modal");
    if (!modal) return;
    const name = this.$(".cw-tool-name");
    const desc = this.$(".cw-tool-description");
    const inp = this.$(".cw-tool-input");
    if (name) {
      name.textContent = confirmation.tool_name
        ? I18n.label(AGENT_TOOL_LABELS, confirmation.tool_name)
        : I18n.t("assistant-chat-tool-unknown");
      name.title = confirmation.tool_name || "";
    }
    if (desc)
      desc.textContent =
        confirmation.description || I18n.t("assistant-chat-tool-default-description");
    if (inp) inp.textContent = JSON.stringify(confirmation.input || {}, null, 2);
    modal.hidden = false;
  }

  _hideToolConfirmation() {
    const modal = this.$(".cw-tool-modal");
    if (modal) modal.hidden = true;
  }

  _updateChatHeader(session) {
    const title = this.$(".cw-chat-title");
    if (title) title.textContent = session.title || I18n.t("assistant-chat-title-new");

    const deleteBtn = this.$(".cw-delete-btn");
    if (deleteBtn) {
      deleteBtn.disabled = false;
      deleteBtn.onclick = () => this.deleteSession(session.id);
    }
  }

  _showDraft() {
    this._setSessionsOpen(false);
    this._isDraft = true;
    this.state.currentSession = null;
    this.state.messages = [];
    this.state.pendingConfirmation = null;
    this._messageLoadGeneration++;
    this._prevSessionsJson = "";
    this._renderSessions();
    this._renderMessagesForce();
    const title = this.$(".cw-chat-title");
    if (title) title.textContent = I18n.t("assistant-chat-title-new");
    const deleteButton = this.$(".cw-delete-btn");
    if (deleteButton) deleteButton.disabled = true;
  }

  _showChatInterface() {
    const emptyState = this.$(".cw-empty-state");
    if (emptyState && this.state.messages.length === 0 && !this.state.currentSession) {
      emptyState.style.display = "flex";
    }
  }

  _handleInputChange() {
    const input = this.$(".cw-chat-input");
    if (!input) return;
    input.style.height = "auto";
    input.style.height = `${Math.min(input.scrollHeight, 180)}px`;
    this._updateSendButton();
    this._updateCharCount();
  }

  _updateCharCount() {
    const input = this.$(".cw-chat-input");
    const counter = this.$(".cw-char-count");
    if (!input || !counter) return;

    const len = input.value.length;
    if (len === 0) {
      counter.textContent = "";
      counter.className = "char-count cw-char-count";
    } else if (len > MESSAGE_LIMIT) {
      counter.textContent = `${formatNumber(len, 0)} / ${formatNumber(MESSAGE_LIMIT, 0)}`;
      counter.className = "char-count cw-char-count danger";
    } else if (len > 3500) {
      counter.textContent = `${formatNumber(len, 0)} / ${formatNumber(MESSAGE_LIMIT, 0)}`;
      counter.className = "char-count cw-char-count warning";
    } else if (len > 100) {
      counter.textContent = formatNumber(len, 0);
      counter.className = "char-count cw-char-count";
    } else {
      counter.textContent = "";
      counter.className = "char-count cw-char-count";
    }
  }

  _updateInputStatus(status, type = "") {
    const el = this.$(".cw-input-status");
    if (!el) return;
    el.className = `input-status cw-input-status${type ? ` status-${type}` : ""}`;
    el.innerHTML = status;
  }

  _updateSendButton() {
    const sendBtn = this.$(".cw-send-btn");
    const input = this.$(".cw-chat-input");

    if (!sendBtn || !input) return;

    const hasText = input.value.trim().length > 0;
    const isOverLimit = input.value.length > MESSAGE_LIMIT;
    const canSend = hasText && !this.state.isLoading && !isOverLimit;

    sendBtn.disabled = this.state.isLoading ? false : !canSend;
    if (this.state.isLoading) {
      sendBtn.setAttribute("aria-label", I18n.attr("assistant-chat-stop", "aria-label"));
      sendBtn.setAttribute("title", I18n.attr("assistant-chat-stop", "title"));
    } else {
      const labelText = canSend
        ? I18n.attr("assistant-chat-send", "aria-label")
        : I18n.attr("assistant-chat-send-empty", "aria-label");
      sendBtn.setAttribute("aria-label", labelText);
      sendBtn.setAttribute("title", I18n.attr("assistant-chat-send", "title"));
    }
    sendBtn.classList.toggle("is-stopping", this.state.isLoading);
    const icon = sendBtn.querySelector("i");
    if (icon) icon.className = this.state.isLoading ? "icon-square" : "icon-send";
  }

  _setSessionsOpen(open) {
    this.$(".chat-container")?.classList.toggle("sessions-open", open);
    this.$(".cw-sessions-toggle")?.setAttribute("aria-expanded", String(open));
  }

  _handleKeydown(e) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      this.sendMessage();
      return;
    }
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      this.sendMessage();
      return;
    }
    if (e.key === "Escape") {
      if (this.$(".chat-container")?.classList.contains("sessions-open")) {
        e.preventDefault();
        this._setSessionsOpen(false);
      } else if (this.state.pendingConfirmation) {
        e.preventDefault();
        this.confirmTool(false);
      } else if (this.state.isLoading) {
        e.preventDefault();
        this.cancelRequest();
      } else if (this.opts.onClose) {
        e.preventDefault();
        this.opts.onClose();
      } else {
        e.target.blur();
      }
      return;
    }
  }

  _groupSessionsByDate(sessions) {
    const now = new Date();
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const yesterday = new Date(today);
    yesterday.setDate(yesterday.getDate() - 1);
    const weekAgo = new Date(today);
    weekAgo.setDate(weekAgo.getDate() - 7);

    const groups = { today: [], yesterday: [], week: [], older: [] };

    for (const session of sessions) {
      const date = new Date(session.updated_at || session.created_at);
      if (date >= today) groups.today.push(session);
      else if (date >= yesterday) groups.yesterday.push(session);
      else if (date >= weekAgo) groups.week.push(session);
      else groups.older.push(session);
    }

    return groups;
  }
}
