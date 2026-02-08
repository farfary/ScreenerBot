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
    this.opts = { showSidebar: true, ...opts };

    this.state = {
      sessions: [],
      currentSession: null,
      messages: [],
      isLoading: false,
      pendingConfirmation: null,
    };

    this._abortController = null;
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
    const sidebarClass = this.opts.showSidebar ? "" : " cw-no-sidebar";
    this.root.innerHTML = `
      <div class="chat-container${sidebarClass}">
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

        <div class="chat-main">
          <div class="chat-header">
            <span class="chat-title cw-chat-title">New Chat</span>
            <div class="chat-actions">
              <button class="chat-action-btn cw-summarize-btn" type="button" title="Summarize" aria-label="Summarize conversation">
                <i class="icon-file-text"></i>
              </button>
              <button class="chat-action-btn cw-delete-btn" type="button" title="Delete" aria-label="Delete session">
                <i class="icon-trash"></i>
              </button>
            </div>
          </div>

          <div class="chat-messages cw-chat-messages" aria-live="polite" aria-atomic="false">
            <div class="chat-empty-state cw-empty-state">
              <div class="empty-state-icon"><i class="icon-bot-message-square"></i></div>
              <h3>How can I help you today?</h3>
              <p class="empty-state-subtitle">Ask anything about your portfolio, analyze tokens, or execute trading actions.</p>
              <div class="quick-prompts">
                <button class="quick-prompt" type="button" data-prompt="What's my current wallet balance and open positions?">
                  <i class="icon-wallet"></i><span>Check my balance</span>
                </button>
                <button class="quick-prompt" type="button" data-prompt="Analyze the security and risks of this token: ">
                  <i class="icon-shield-check"></i><span>Security check</span>
                </button>
                <button class="quick-prompt" type="button" data-prompt="Show me my open positions with current P&amp;L">
                  <i class="icon-trending-up"></i><span>View positions</span>
                </button>
                <button class="quick-prompt" type="button" data-prompt="What tokens are passing the filter criteria right now?">
                  <i class="icon-list-filter"></i><span>Filtered tokens</span>
                </button>
                <button class="quick-prompt" type="button" data-prompt="Help me configure my trading entry and exit settings">
                  <i class="icon-settings"></i><span>Configure trading</span>
                </button>
                <button class="quick-prompt" type="button" data-prompt="What's the current market status and any notable opportunities?">
                  <i class="icon-activity"></i><span>Market overview</span>
                </button>
              </div>
            </div>
          </div>

          <div class="modal-overlay cw-tool-modal" style="display:none">
            <div class="modal-dialog modal-sm">
              <div class="modal-header">
                <h3><i class="icon-alert-triangle"></i> Confirm Tool Execution</h3>
                <button class="modal-close cw-tool-modal-close" type="button"><i class="icon-x"></i></button>
              </div>
              <div class="modal-body">
                <p><strong class="cw-tool-name">Tool Name</strong></p>
                <p class="cw-tool-description">This tool requires your approval to execute.</p>
                <div class="tool-call-section">
                  <div class="tool-call-label">Input:</div>
                  <pre class="cw-tool-input tool-call-code">{}</pre>
                </div>
              </div>
              <div class="modal-footer">
                <button class="btn btn-secondary cw-deny-tool" type="button">Deny</button>
                <button class="btn btn-primary cw-confirm-tool" type="button">Allow</button>
              </div>
            </div>
          </div>

          <div class="chat-input-area">
            <div class="chat-context cw-chat-context"></div>
            <div class="chat-input-container cw-input-container">
              <div class="chat-input-wrapper">
                <textarea class="cw-chat-input" placeholder="Message Assistant..." rows="1" aria-label="Message input"></textarea>
                <div class="input-hint cw-input-hint"><kbd>⌘</kbd><kbd>↵</kbd> to send</div>
              </div>
              <div class="chat-input-actions">
                <button class="cancel-btn cw-cancel-btn" type="button" aria-label="Cancel request" title="Cancel (Esc)">
                  <i class="icon-x"></i>
                </button>
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

    // Sessions search
    this._on(this.$(".cw-sessions-search"), "input", () => this._renderSessions());

    // Send button
    this._on(this.$(".cw-send-btn"), "click", () => this.sendMessage());

    // Cancel button
    this._on(this.$(".cw-cancel-btn"), "click", () => this.cancelRequest());

    // Chat input
    const input = this.$(".cw-chat-input");
    this._on(input, "input", () => this._handleInputChange());
    this._on(input, "keydown", (e) => this._handleKeydown(e));

    // Tool confirmation buttons
    this._on(this.$(".cw-confirm-tool"), "click", () => this.confirmTool(true));
    this._on(this.$(".cw-deny-tool"), "click", () => this.confirmTool(false));
    this._on(this.$(".cw-tool-modal-close"), "click", () => this._hideToolConfirmation());

    // Modal overlay click to close
    const modal = this.$(".cw-tool-modal");
    this._on(modal, "click", (e) => {
      if (e.target === modal) this._hideToolConfirmation();
    });

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
            Utils.showToast({ type: "success", title: "Copied", message: "Message copied to clipboard" });
          })
          .catch(() => Utils.showToast({ type: "error", title: "Error", message: "Failed to copy message" }));
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
    try {
      const response = await fetch("/api/ai/chat/sessions");
      if (!response.ok) throw new Error("Failed to load chat sessions");

      const data = await response.json();
      this.state.sessions = Array.isArray(data) ? data : data.sessions || [];

      this._renderSessions();

      if (!this.state.currentSession && this.state.sessions.length > 0) {
        await this.selectSession(this.state.sessions[0].id);
      } else if (this.state.currentSession) {
        const cur = this.state.sessions.find((s) => s.id === this.state.currentSession);
        if (cur) await this._loadMessages(cur);
      }
    } catch (error) {
      console.error("[ChatWidget] Error loading sessions:", error);
      Utils.showToast({ type: "error", title: "Error", message: "Failed to load chat sessions" });
    }
  }

  async createSession() {
    try {
      const response = await fetch("/api/ai/chat/sessions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
      });
      if (!response.ok) throw new Error("Failed to create session");

      const data = await response.json();
      playToggleOn();
      await this.loadSessions();
      await this.selectSession(data.session_id);
    } catch (error) {
      console.error("[ChatWidget] Error creating session:", error);
      playError();
      Utils.showToast({ type: "error", title: "Error", message: "Failed to create chat session" });
    }
  }

  async selectSession(sessionId) {
    const numericId = typeof sessionId === "string" ? parseInt(sessionId, 10) : sessionId;
    this.state.currentSession = numericId;

    const session = this.state.sessions.find((s) => s.id === numericId);
    if (!session) {
      console.error("[ChatWidget] Session not found:", numericId);
      return;
    }

    this._renderSessions();
    this._updateChatHeader(session);
    this._showChatInterface();
    await this._loadMessages(session, true);
  }

  async deleteSession(sessionId) {
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

  async summarizeSession(sessionId) {
    try {
      const response = await fetch(`/api/ai/chat/sessions/${sessionId}/summarize`, { method: "POST" });
      if (!response.ok) throw new Error("Failed to summarize session");

      const data = await response.json();
      playToggleOn();
      await this.loadSessions();
      Utils.showToast({ type: "success", title: "Success", message: `Session summarized: ${data.summary}` });
    } catch (error) {
      console.error("[ChatWidget] Error summarizing session:", error);
      playError();
      Utils.showToast({ type: "error", title: "Error", message: "Failed to summarize session" });
    }
  }

  async generateSessionTitle(sessionId) {
    try {
      const response = await fetch(`/api/ai/chat/sessions/${sessionId}/generate-title`, { method: "POST" });
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
    const input = this.$(".cw-chat-input");
    if (!input) return;

    const message = input.value.trim();
    if (!message) return;

    if (message.length > 4000) {
      Utils.showToast({ type: "error", title: "Message too long", message: "Please shorten your message to under 4,000 characters" });
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
    const signal = this._abortController.signal;

    input.value = "";
    input.style.height = "auto";
    input.disabled = true;
    this.state.isLoading = true;

    this._updateSendButton();
    this._updateCharCount();
    this._updateInputStatus('<span class="typing-dots"><span></span><span></span><span></span></span> Thinking...', "sending");

    const userMessage = { role: "user", content: message, timestamp: new Date().toISOString() };
    this.state.messages.push(userMessage);
    this._renderMessages();
    this._showTypingIndicator();

    try {
      const response = await fetch("/api/ai/chat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ session_id: this.state.currentSession, message }),
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
        this.generateSessionTitle(this.state.currentSession);
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

      this._updateInputStatus(`<i class="icon-alert-circle"></i> ${error.message || "Failed to send"}`, "error");
      setTimeout(() => this._updateInputStatus(""), 5000);

      Utils.showToast({ type: "error", title: "Error", message: error.message || "Failed to send message" });
    } finally {
      this._abortController = null;
      input.disabled = false;
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
    this.state.messages = this.state.messages.slice(0, lastUserIndex + 1);

    if (this._abortController) this._abortController.abort();
    this._abortController = new AbortController();
    const signal = this._abortController.signal;

    this._renderMessages();
    this._showTypingIndicator();
    this.state.isLoading = true;
    this._updateSendButton();
    this._updateInputStatus('<span class="typing-dots"><span></span><span></span><span></span></span> Regenerating...', "sending");

    try {
      const response = await fetch("/api/ai/chat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ session_id: this.state.currentSession, message: lastUserMessage }),
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

      Utils.showToast({ type: "success", title: "Regenerated", message: "Response regenerated successfully" });
    } catch (error) {
      if (error.name === "AbortError") return;
      console.error("[ChatWidget] Error regenerating:", error);
      playError();
      this._hideTypingIndicator();
      this._updateInputStatus(`<i class="icon-alert-circle"></i> ${error.message || "Failed to regenerate"}`, "error");
      setTimeout(() => this._updateInputStatus(""), 5000);
      Utils.showToast({ type: "error", title: "Error", message: error.message || "Failed to regenerate response" });
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
      input.disabled = false;
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
      const response = await fetch(`/api/ai/chat/confirm/${confirmation.id}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ approved }),
      });
      if (!response.ok) throw new Error("Failed to confirm tool");

      const data = await response.json();

      if (approved) {
        playToggleOn();
        Utils.showToast({ type: "success", title: "Success", message: "Tool executed successfully" });
        if (data.result) {
          this.state.messages.push({
            role: "assistant",
            content: data.result.content || "",
            tool_calls: data.result.tool_calls || [],
            timestamp: new Date().toISOString(),
          });
          this._renderMessages();
        }
      } else {
        Utils.showToast({ type: "info", title: "Cancelled", message: "Tool execution cancelled" });
        this.state.messages.push({ role: "assistant", content: "Tool execution was cancelled.", timestamp: new Date().toISOString() });
        this._renderMessages();
      }

      this.state.pendingConfirmation = null;
      await this.loadSessions();
    } catch (error) {
      console.error("[ChatWidget] Error confirming tool:", error);
      playError();
      Utils.showToast({ type: "error", title: "Error", message: "Failed to confirm tool execution" });
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

    const isSessionChange = this.state.currentSession !== session.id;

    try {
      const response = await fetch(`/api/ai/chat/sessions/${session.id}`);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);

      const data = await response.json();
      const newMessages = data.messages || [];

      if (!forceRender && !isSessionChange && this.state.messages.length === newMessages.length) {
        const lastOld = this.state.messages[this.state.messages.length - 1];
        const lastNew = newMessages[newMessages.length - 1];
        if (lastOld?.id === lastNew?.id) return;
      }

      this.state.messages = newMessages;

      if (isSessionChange || forceRender) {
        this._renderMessagesForce();
      } else {
        this._renderMessages();
      }
    } catch (error) {
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

    sessions.sort((a, b) => new Date(b.updated_at || b.created_at) - new Date(a.updated_at || a.created_at));

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
      JSON.stringify(sessions.map((s) => ({ id: s.id, title: s.title, message_count: s.message_count, updated_at: s.updated_at }))) +
      "|" + this.state.currentSession + "|" + searchQuery;

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
      container.insertAdjacentHTML("beforeend", this.state.messages.map((m) => this._renderMessage(m)).join(""));
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
    container.insertAdjacentHTML("beforeend", this.state.messages.map((m) => this._renderMessage(m)).join(""));
    this._setupToolExpandHandlers();
    this._scrollToBottom();
  }

  _renderMessage(msg) {
    const isUser = msg.role === "user";
    const timestamp = msg.timestamp
      ? new Date(msg.timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
      : "";

    let parsedToolCalls = msg.tool_calls;
    if (typeof parsedToolCalls === "string") {
      try { parsedToolCalls = JSON.parse(parsedToolCalls); } catch (_e) { parsedToolCalls = null; }
    }

    const toolCallsHtml =
      parsedToolCalls && Array.isArray(parsedToolCalls) && parsedToolCalls.length > 0
        ? parsedToolCalls.map((t) => this._renderToolCall(t)).join("")
        : "";

    const escapedContent = msg.content ? msg.content.replace(/"/g, "&quot;") : "";
    const actionsHtml = msg.content
      ? `<div class="message-actions">
          <button class="message-action-btn" title="Copy" data-action="copy" data-content="${Utils.escapeHtml(escapedContent)}">
            <i class="icon-copy"></i>
          </button>
          ${!isUser ? '<button class="message-action-btn" title="Regenerate" data-action="regenerate"><i class="icon-refresh-cw"></i></button>' : ""}
        </div>`
      : "";

    return `
      <div class="message ${isUser ? "user" : "assistant"}">
        <div class="message-avatar"><i class="icon-${isUser ? "user" : "bot"}"></i></div>
        <div class="message-content">
          ${toolCallsHtml}
          ${msg.content ? `<div class="message-bubble">${Utils.escapeHtml(msg.content)}${actionsHtml}</div>` : ""}
          <div class="message-meta">${timestamp}</div>
        </div>
      </div>`;
  }

  _renderToolCall(tool) {
    const statusRaw = tool.status || "pending";
    const statusClass = statusRaw.toLowerCase();
    const statusText =
      statusClass === "executed" ? "Executed"
        : statusClass === "failed" ? "Failed"
        : statusClass === "denied" ? "Denied"
        : statusClass === "pendingconfirmation" ? "Awaiting Confirmation"
        : "Pending";

    const toolName = tool.tool_name || tool.name || "Unknown Tool";

    return `
      <div class="tool-call ${statusClass}">
        <div class="tool-call-header">
          <div class="tool-call-title"><i class="icon-wrench"></i> ${Utils.escapeHtml(toolName)}</div>
          <span class="tool-call-status ${statusClass}">${statusText}</span>
          <button class="tool-call-expand" type="button"><i class="icon-chevron-down"></i></button>
        </div>
        <div class="tool-call-body" style="display:none;">
          <div class="tool-call-section">
            <div class="tool-call-label">Input:</div>
            <div class="tool-call-input"><pre class="tool-call-code">${JSON.stringify(tool.input || {}, null, 2)}</pre></div>
          </div>
          ${tool.output ? `<div class="tool-call-section"><div class="tool-call-label">Output:</div><div class="tool-call-output"><pre class="tool-call-code">${JSON.stringify(tool.output, null, 2)}</pre></div></div>` : ""}
          ${tool.error ? `<div class="tool-call-section"><div class="tool-call-label">Error:</div><div class="tool-call-error"><pre class="tool-call-code">${Utils.escapeHtml(tool.error)}</pre></div></div>` : ""}
        </div>
      </div>`;
  }

  // ---------------------------------------------------------------------------
  // Private - UI helpers
  // ---------------------------------------------------------------------------

  _setupToolExpandHandlers() {
    this.$$(".tool-call-expand").forEach((btn) => {
      btn.onclick = (e) => {
        e.stopPropagation();
        const body = btn.closest(".tool-call").querySelector(".tool-call-body");
        if (body.style.display === "none") {
          body.style.display = "block";
          btn.classList.add("expanded");
        } else {
          body.style.display = "none";
          btn.classList.remove("expanded");
        }
      };
    });
  }

  _showTypingIndicator() {
    const container = this.$(".cw-chat-messages");
    if (!container || container.querySelector(".typing-indicator")) return;

    const indicator = document.createElement("div");
    indicator.className = "typing-indicator";
    indicator.innerHTML = `
      <div class="message-avatar"><i class="icon-bot"></i></div>
      <div class="typing-dots">
        <span class="typing-dot"></span>
        <span class="typing-dot"></span>
        <span class="typing-dot"></span>
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
    if (desc) desc.textContent = confirmation.description || "This tool requires your approval to execute.";
    if (inp) inp.textContent = JSON.stringify(confirmation.input || {}, null, 2);
    modal.style.display = "flex";
  }

  _hideToolConfirmation() {
    const modal = this.$(".cw-tool-modal");
    if (modal) modal.style.display = "none";
  }

  _updateChatHeader(session) {
    const title = this.$(".cw-chat-title");
    if (title) title.textContent = session.title || "New Chat";

    const summarizeBtn = this.$(".cw-summarize-btn");
    if (summarizeBtn) summarizeBtn.onclick = () => this.summarizeSession(session.id);

    const deleteBtn = this.$(".cw-delete-btn");
    if (deleteBtn) deleteBtn.onclick = () => this.deleteSession(session.id);
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
    const isMac = navigator.platform?.toUpperCase().indexOf("MAC") >= 0 || navigator.userAgent?.toUpperCase().indexOf("MAC") >= 0;
    hint.innerHTML = isMac ? "<kbd>⌘</kbd><kbd>↵</kbd> to send" : "<kbd>Ctrl</kbd><kbd>↵</kbd> to send";
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
    if (len === 0) { counter.textContent = ""; counter.className = "char-count cw-char-count"; }
    else if (len > 4000) { counter.textContent = `${len.toLocaleString()} / 4,000`; counter.className = "char-count cw-char-count danger"; }
    else if (len > 3500) { counter.textContent = `${len.toLocaleString()} / 4,000`; counter.className = "char-count cw-char-count warning"; }
    else if (len > 100) { counter.textContent = len.toLocaleString(); counter.className = "char-count cw-char-count"; }
    else { counter.textContent = ""; counter.className = "char-count cw-char-count"; }
  }

  _updateInputStatus(status, type = "") {
    const el = this.$(".cw-input-status");
    if (!el) return;
    el.className = `input-status cw-input-status${type ? ` status-${type}` : ""}`;
    el.innerHTML = status;
  }

  _updateSendButton() {
    const sendBtn = this.$(".cw-send-btn");
    const cancelBtn = this.$(".cw-cancel-btn");
    const input = this.$(".cw-chat-input");
    const container = this.$(".cw-input-container");

    if (!sendBtn || !input) return;

    const hasText = input.value.trim().length > 0;
    const isOverLimit = input.value.length > 4000;
    const canSend = hasText && !this.state.isLoading && !isOverLimit;

    sendBtn.disabled = !canSend;
    sendBtn.setAttribute("aria-label", canSend ? "Send message" : "Type a message to send");
    sendBtn.classList.toggle("is-loading", this.state.isLoading);

    if (cancelBtn) cancelBtn.classList.toggle("visible", this.state.isLoading);
    if (container) container.classList.toggle("is-sending", this.state.isLoading);
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
      if (this.state.isLoading) {
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
