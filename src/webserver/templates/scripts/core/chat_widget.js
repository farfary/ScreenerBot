/**
 * ChatWidget - Reusable chat component
 *
 * Extracted from ai.js to allow usage in both the AI page chat tab
 * and the global floating chat dialog. All DOM queries are scoped to
 * the provided root element so multiple instances can coexist.
 */
import * as Utils from "./utils.js";
import { ConfirmationDialog } from "../ui/confirmation_dialog.js";
import { playToggleOn, playError } from "./sounds.js";

export class ChatWidget {
  /**
   * @param {HTMLElement} root - Container element to render chat into
   * @param {Object} opts
   * @param {boolean} [opts.showSidebar=true] - Show sessions sidebar
   * @param {Function} [opts.onClose] - Called when user wants to close (Escape in dialog)
   */
  constructor(root, opts = {}) {
    this.root = root;
    this.opts = { showSidebar: true, historyDrawer: false, ...opts };

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
    this._updateKeyboardHint();
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
    const sidebarClass = `${this.opts.showSidebar ? "" : " cw-no-sidebar"}${this.opts.historyDrawer ? " cw-history-drawer" : ""}`;
    this.root.innerHTML = `
      <div class="chat-widget chat-container${sidebarClass}">
        ${
          this.opts.showSidebar
            ? `
        <div class="chat-sessions-sidebar">
          <div class="sessions-header">
            <h3>Sessions</h3>
            <button class="new-session-btn" type="button" title="New Chat" aria-label="Create new chat session">
              <i class="icon-plus"></i>
            </button>
          </div>
          <div class="sessions-search">
            <i class="icon-search"></i>
            <input type="text" class="cw-sessions-search" placeholder="Search chats..." aria-label="Search chat sessions" />
          </div>
          <div class="sessions-list cw-sessions-list"></div>
        </div>`
            : ""
        }
        ${this.opts.showSidebar && this.opts.historyDrawer ? '<button class="chat-sessions-scrim" type="button" aria-label="Close chat history"></button>' : ""}

        <div class="chat-main">
          <div class="chat-header">
            <span class="chat-title cw-chat-title">New Chat</span>
            <div class="chat-actions">
              <button class="chat-action-btn cw-sessions-toggle" type="button" title="Chat history" aria-label="Open chat history">
                <i class="icon-panel-left"></i>
              </button>
              <button class="chat-action-btn cw-new-session-btn" type="button" title="New Chat" aria-label="Start a new chat">
                <i class="icon-plus"></i>
              </button>
              <button class="chat-action-btn cw-delete-btn" type="button" title="Delete" aria-label="Delete session">
                <i class="icon-trash"></i>
              </button>
              ${this.opts.onClose ? '<button class="chat-action-btn cw-close-btn" type="button" title="Close" aria-label="Close Assistant"><i class="icon-x"></i></button>' : ""}
            </div>
          </div>

          <div class="chat-messages cw-chat-messages" aria-live="polite" aria-atomic="false">
            <div class="chat-empty-state cw-empty-state">
              <div class="empty-state-kicker"><i class="icon-bot-message-square"></i><span>Assistant</span></div>
              <h3>How can I help you today?</h3>
              <p class="empty-state-subtitle">Review your portfolio, investigate a token, or understand recent trading activity.</p>
              <div class="quick-prompts">
                <button class="quick-prompt" type="button" data-prompt="What's my current wallet balance and open positions?">
                  <i class="icon-wallet"></i><span>Review open positions</span><i class="icon-arrow-up-right"></i>
                </button>
                <button class="quick-prompt" type="button" data-prompt="Analyze the security and risks of this token: ">
                  <i class="icon-shield-check"></i><span>Analyze a token</span><i class="icon-arrow-up-right"></i>
                </button>
                <button class="quick-prompt" type="button" data-prompt="Explain my recent trading activity and any important outcomes">
                  <i class="icon-activity"></i><span>Explain recent activity</span><i class="icon-arrow-up-right"></i>
                </button>
              </div>
            </div>
          </div>

          <section class="tool-confirmation cw-tool-modal" aria-live="polite" hidden>
            <div class="tool-confirmation-copy">
              <div class="tool-confirmation-title"><i class="icon-triangle-alert"></i><strong class="cw-tool-name">Tool Name</strong></div>
              <p class="cw-tool-description">This action requires your approval.</p>
              <details class="tool-confirmation-details">
                <summary>Review input</summary>
                <pre class="cw-tool-input tool-call-code">{}</pre>
              </details>
            </div>
            <div class="confirmation-actions">
              <button class="btn btn-secondary cw-deny-tool" type="button">Deny</button>
              <button class="btn btn-primary cw-confirm-tool" type="button">Allow</button>
            </div>
          </section>

          <div class="chat-input-area">
            <div class="chat-context cw-chat-context"></div>
            <div class="chat-input-container cw-input-container">
              <div class="chat-input-wrapper">
                <textarea class="cw-chat-input" placeholder="Message Assistant..." rows="1" aria-label="Message input"></textarea>
                <div class="input-hint cw-input-hint"><kbd>Enter</kbd> to send <span>·</span> <kbd>Shift</kbd><kbd>Enter</kbd> for a new line</div>
              </div>
              <div class="chat-input-actions">
                <button class="send-btn cw-send-btn" type="button" disabled aria-label="Send message" title="Send message">
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
  }

  // ---------------------------------------------------------------------------
  // Event Handlers Setup
  // ---------------------------------------------------------------------------

  _setupHandlers() {
    // New session button
    this._on(this.$(".new-session-btn"), "click", () => this.createSession());
    this._on(this.$(".cw-new-session-btn"), "click", () => this.createSession());
    this._on(this.$(".cw-sessions-toggle"), "click", () => {
      this.$(".chat-container")?.classList.toggle("sessions-open");
    });
    this._on(this.$(".chat-sessions-scrim"), "click", () => {
      this.$(".chat-container")?.classList.remove("sessions-open");
    });
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
        chatInput.value = prompt;
        chatInput.focus();
        chatInput.dispatchEvent(new Event("input", { bubbles: true }));
        if (!prompt.trim().endsWith(":")) this.sendMessage();
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
            Utils.showToast({
              type: "success",
              title: "Copied",
              message: "Message copied to clipboard",
            });
          })
          .catch(() =>
            Utils.showToast({ type: "error", title: "Error", message: "Failed to copy message" })
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
      const response = await fetch("/api/ai/chat/sessions");
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
      Utils.showToast({ type: "error", title: "Error", message: "Failed to load chat sessions" });
    }
  }

  async createSession() {
    if (this.state.isLoading) this.cancelRequest();
    this._showDraft();
    playToggleOn();
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
    this.$(".chat-container")?.classList.remove("sessions-open");
    await this._loadMessages(session, true);
  }

  async deleteSession(sessionId) {
    if (this.state.isLoading) this.cancelRequest();
    try {
      const confirmed = await ConfirmationDialog.show({
        title: "Delete Chat Session",
        message: "Are you sure you want to delete this chat session? This action cannot be undone.",
        confirmText: "Delete",
        cancelText: "Cancel",
        type: "danger",
      });
      if (!confirmed) return;

      const response = await fetch(`/api/ai/chat/sessions/${sessionId}`, { method: "DELETE" });
      if (!response.ok) throw new Error("Failed to delete session");

      playToggleOn();

      if (this.state.currentSession === Number(sessionId)) {
        this.state.currentSession = null;
        this.state.messages = [];
      }

      await this.loadSessions();
      Utils.showToast({ type: "success", title: "Success", message: "Chat session deleted" });
    } catch (error) {
      console.error("[ChatWidget] Error deleting session:", error);
      playError();
      Utils.showToast({ type: "error", title: "Error", message: "Failed to delete chat session" });
    }
  }

  async generateSessionTitle(sessionId) {
    try {
      const response = await fetch(`/api/ai/chat/sessions/${sessionId}/generate-title`, {
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

    if (message.length > 4000) {
      Utils.showToast({
        type: "error",
        title: "Message too long",
        message: "Please shorten your message to under 4,000 characters",
      });
      return;
    }

    // Auto-create session if none exists
    if (!this.state.currentSession) {
      try {
        const response = await fetch("/api/ai/chat/sessions", {
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
        Utils.showToast({ type: "error", title: "Error", message: "Failed to start chat session" });
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
      const response = await fetch("/api/ai/chat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ session_id: sessionId, message }),
        signal,
      });

      if (signal.aborted) return;
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error?.message || `API error: ${response.status}`);
      }

      const data = await response.json();
      if (this.state.currentSession !== sessionId) return;

      this._hideTypingIndicator();
      this._updateInputStatus("");

      if (data.error) throw new Error(data.error.message || "Unknown error");

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

      const container = this.$(".cw-input-container");
      if (container) {
        container.classList.add("has-error");
        setTimeout(() => container.classList.remove("has-error"), 400);
      }

      this._updateInputStatus(
        `<i class="icon-circle-alert"></i> ${Utils.escapeHtml(error.message || "Failed to send")}`,
        "error"
      );
      setTimeout(() => this._updateInputStatus(""), 5000);

      Utils.showToast({
        type: "error",
        title: "Error",
        message: error.message || "Failed to send message",
      });
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
      Utils.showToast({ type: "error", title: "Error", message: "No message to regenerate" });
      return;
    }

    const lastUserMessage = this.state.messages[lastUserIndex].content;
    const previousAssistant = this.state.messages[lastUserIndex + 1];
    if (!previousAssistant?.id || previousAssistant.role !== "assistant") {
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Reload the chat before regenerating this response",
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
      const response = await fetch("/api/ai/chat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          session_id: this.state.currentSession,
          message: lastUserMessage,
          regenerate_message_id: previousAssistant.id,
        }),
        signal,
      });

      if (signal.aborted) return;
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error?.message || `API error: ${response.status}`);
      }

      const data = await response.json();
      this._hideTypingIndicator();
      this._updateInputStatus("");

      if (data.error) throw new Error(data.error.message || "Unknown error");

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
        title: "Regenerated",
        message: "Response regenerated successfully",
      });
    } catch (error) {
      if (error.name === "AbortError") return;
      console.error("[ChatWidget] Error regenerating:", error);
      this.state.messages = previousMessages;
      this._renderMessagesForce();
      playError();
      this._hideTypingIndicator();
      this._updateInputStatus(
        `<i class="icon-circle-alert"></i> ${Utils.escapeHtml(error.message || "Failed to regenerate")}`,
        "error"
      );
      setTimeout(() => this._updateInputStatus(""), 5000);
      Utils.showToast({
        type: "error",
        title: "Error",
        message: error.message || "Failed to regenerate response",
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
    this._updateInputStatus("Request cancelled", "");
    setTimeout(() => this._updateInputStatus(""), 2000);

    Utils.showToast({ type: "info", title: "Cancelled", message: "Request cancelled" });
  }

  async confirmTool(approved) {
    const confirmation = this.state.pendingConfirmation;
    if (!confirmation) return;

    this._hideToolConfirmation();

    try {
      const response = await fetch(`/api/ai/chat/confirm/${confirmation.confirmation_id}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ approved, session_id: this.state.currentSession }),
      });
      if (!response.ok) throw new Error("Failed to confirm tool");

      const data = await response.json();

      if (approved) {
        playToggleOn();
        Utils.showToast({
          type: "success",
          title: "Success",
          message: "Tool executed successfully",
        });
      } else {
        Utils.showToast({ type: "info", title: "Cancelled", message: "Tool execution cancelled" });
      }

      this.state.pendingConfirmation = data.pending_confirmations?.[0] || null;
      await this.loadSessions();
      if (this.state.pendingConfirmation)
        this._showToolConfirmation(this.state.pendingConfirmation);
    } catch (error) {
      console.error("[ChatWidget] Error confirming tool:", error);
      playError();
      Utils.showToast({
        type: "error",
        title: "Error",
        message: "Failed to confirm tool execution",
      });
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
      const response = await fetch(`/api/ai/chat/sessions/${session.id}`);
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
          <p>${searchQuery ? "No matching chats" : "No chat sessions yet"}</p>
          ${!searchQuery ? '<button class="btn btn-sm cw-empty-new-session"><i class="icon-plus"></i> New Chat</button>' : ""}
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

    for (const [groupName, groupSessions] of Object.entries(groups)) {
      if (groupSessions.length === 0) continue;
      html += '<div class="sessions-group">';
      html += `<div class="sessions-group-header">${groupName}</div>`;

      for (const session of groupSessions) {
        const isActive = session.id === this.state.currentSession;
        const title = Utils.escapeHtml(session.title || "New Chat");
        const preview = session.summary ? Utils.escapeHtml(session.summary.substring(0, 60)) : "";

        html += `
          <div class="session-item ${isActive ? "active" : ""}" data-session-id="${session.id}">
            <div class="session-info">
              <div class="session-title">${title}</div>
              ${preview ? `<div class="session-preview">${preview}${session.summary.length > 60 ? "..." : ""}</div>` : ""}
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

    // Bold
    html = html.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");
    // Italic (single *, not preceded/followed by space to avoid list markers)
    html = html.replace(/(?<!\w)\*(?!\s)(.+?)(?<!\s)\*(?!\w)/g, "<em>$1</em>");
    // Newlines to <br> (but not before/after code block placeholders)
    html = html.replace(/\n/g, "<br>");

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
      ? new Date(messageTime).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
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
      ? `<div class="message-actions" aria-label="Message actions">
          <button class="message-action-btn" type="button" title="Copy" aria-label="Copy message" data-action="copy" data-content="${Utils.escapeHtml(msg.content)}">
            <i class="icon-copy"></i>
          </button>
          ${!isUser ? '<button class="message-action-btn" type="button" title="Regenerate" aria-label="Regenerate response" data-action="regenerate"><i class="icon-refresh-cw"></i></button>' : ""}
        </div>`
      : "";

    return `
      <div class="message ${isUser ? "user" : "assistant"}">
        ${isUser ? "" : '<div class="message-avatar" aria-hidden="true"><i class="icon-bot"></i></div>'}
        <div class="message-content">
          <div class="message-author">${isUser ? "You" : "Assistant"}</div>
          ${toolCallsHtml}
          ${msg.content ? `<div class="message-bubble">${isUser ? Utils.escapeHtml(msg.content) : this._formatMarkdown(msg.content)}</div>` : ""}
          <div class="message-footer"><div class="message-meta">${timestamp}</div>${actionsHtml}</div>
        </div>
      </div>`;
  }

  _renderToolCall(tool) {
    const statusRaw = tool.status || "pending";
    const statusClass = statusRaw.toLowerCase();
    const statusText =
      statusClass === "executed"
        ? "Executed"
        : statusClass === "failed"
          ? "Failed"
          : statusClass === "denied"
            ? "Denied"
            : statusClass === "pendingconfirmation"
              ? "Awaiting Confirmation"
              : "Pending";
    const statusIcon =
      statusClass === "executed" || statusClass === "success"
        ? "circle-check"
        : statusClass === "failed" || statusClass === "denied"
          ? "circle-x"
          : "clock-3";

    const toolName = tool.tool_name || tool.name || "Unknown Tool";
    const toolLabel = toolName
      .replace(/[_-]+/g, " ")
      .replace(/^\w/, (character) => character.toUpperCase());

    return `
      <div class="tool-call ${statusClass}">
        <button class="tool-call-header" type="button" aria-expanded="false">
          <span class="tool-call-title" title="${Utils.escapeHtml(toolName)}"><i class="icon-wrench"></i><span>${Utils.escapeHtml(toolLabel)}</span></span>
          <span class="tool-call-status ${statusClass}"><i class="icon-${statusIcon}"></i><span>${statusText}</span></span>
          <i class="tool-call-expand-icon icon-chevron-down" aria-hidden="true"></i>
        </button>
        <div class="tool-call-body" hidden>
          <div class="tool-call-section">
            <div class="tool-call-label">Input:</div>
            <div class="tool-call-input"><pre class="tool-call-code">${Utils.escapeHtml(JSON.stringify(tool.input || {}, null, 2))}</pre></div>
          </div>
          ${tool.output ? `<div class="tool-call-section"><div class="tool-call-label">Output:</div><div class="tool-call-output"><pre class="tool-call-code">${Utils.escapeHtml(JSON.stringify(tool.output, null, 2))}</pre></div></div>` : ""}
          ${tool.error ? `<div class="tool-call-section"><div class="tool-call-label">Error:</div><div class="tool-call-error"><pre class="tool-call-code">${Utils.escapeHtml(tool.error)}</pre></div></div>` : ""}
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
    indicator.setAttribute("aria-label", "Assistant is thinking");
    indicator.innerHTML = `
      <div class="message-avatar"><i class="icon-bot"></i></div>
      <div class="typing-content">
        <span class="message-author">Assistant</span>
        <span class="typing-label">Thinking</span>
        <span class="typing-dots" aria-hidden="true">
          <span class="typing-dot"></span><span class="typing-dot"></span><span class="typing-dot"></span>
        </span>
      </div>`;
    container.appendChild(indicator);
    this._scrollToBottom();
  }

  _hideTypingIndicator() {
    const el = this.$(".typing-indicator");
    if (el) el.remove();
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
    if (name) name.textContent = confirmation.tool_name || "Unknown Tool";
    if (desc)
      desc.textContent = confirmation.description || "This tool requires your approval to execute.";
    if (inp) inp.textContent = JSON.stringify(confirmation.input || {}, null, 2);
    modal.hidden = false;
  }

  _hideToolConfirmation() {
    const modal = this.$(".cw-tool-modal");
    if (modal) modal.hidden = true;
  }

  _updateChatHeader(session) {
    const title = this.$(".cw-chat-title");
    if (title) title.textContent = session.title || "New Chat";

    const deleteBtn = this.$(".cw-delete-btn");
    if (deleteBtn) {
      deleteBtn.disabled = false;
      deleteBtn.onclick = () => this.deleteSession(session.id);
    }
  }

  _showDraft() {
    this._isDraft = true;
    this.state.currentSession = null;
    this.state.messages = [];
    this.state.pendingConfirmation = null;
    this._messageLoadGeneration++;
    this._prevSessionsJson = "";
    this._renderSessions();
    this._renderMessagesForce();
    const title = this.$(".cw-chat-title");
    if (title) title.textContent = "New Chat";
    const deleteButton = this.$(".cw-delete-btn");
    if (deleteButton) deleteButton.disabled = true;
  }

  _showChatInterface() {
    const emptyState = this.$(".cw-empty-state");
    if (emptyState && this.state.messages.length === 0 && !this.state.currentSession) {
      emptyState.style.display = "flex";
    }
  }

  _updateKeyboardHint() {
    const hint = this.$(".cw-input-hint");
    if (!hint) return;
    hint.innerHTML =
      "<kbd>Enter</kbd> to send <span>·</span> <kbd>Shift</kbd><kbd>Enter</kbd> for a new line";
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
    } else if (len > 4000) {
      counter.textContent = `${len.toLocaleString()} / 4,000`;
      counter.className = "char-count cw-char-count danger";
    } else if (len > 3500) {
      counter.textContent = `${len.toLocaleString()} / 4,000`;
      counter.className = "char-count cw-char-count warning";
    } else if (len > 100) {
      counter.textContent = len.toLocaleString();
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
    const isOverLimit = input.value.length > 4000;
    const canSend = hasText && !this.state.isLoading && !isOverLimit;

    sendBtn.disabled = this.state.isLoading ? false : !canSend;
    sendBtn.setAttribute(
      "aria-label",
      this.state.isLoading ? "Stop response" : canSend ? "Send message" : "Type a message to send"
    );
    sendBtn.setAttribute("title", this.state.isLoading ? "Stop response (Esc)" : "Send message");
    sendBtn.classList.toggle("is-stopping", this.state.isLoading);
    const icon = sendBtn.querySelector("i");
    if (icon) icon.className = this.state.isLoading ? "icon-square" : "icon-send";
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
      if (this.state.pendingConfirmation) {
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

    const groups = { Today: [], Yesterday: [], "Previous 7 Days": [], Older: [] };

    for (const session of sessions) {
      const date = new Date(session.updated_at || session.created_at);
      if (date >= today) groups["Today"].push(session);
      else if (date >= yesterday) groups["Yesterday"].push(session);
      else if (date >= weekAgo) groups["Previous 7 Days"].push(session);
      else groups["Older"].push(session);
    }

    return groups;
  }
}
