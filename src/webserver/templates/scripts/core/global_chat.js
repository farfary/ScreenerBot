/**
 * Global Chat - Floating button + dialog that works on every page.
 * Instantiates a ChatWidget inside a dialog overlay.
 */
import { ChatWidget } from "./chat_widget.js";
import { pushEscapeHandler } from "./escape_stack.js";
import { playPanelOpen, playPanelClose } from "./sounds.js";

class GlobalChat {
  constructor() {
    this._widget = null;
    this._isOpen = false;
    this._built = false;
    this._releaseEscape = null;
    this._previousFocus = null;
  }

  init() {
    if (this._built) return;
    this._built = true;

    this._buildDOM();
    this._setupEvents();
    this._updateVisibility();

    // Hide button on AI page
    window.addEventListener("popstate", () => this._updateVisibility());
    const origPush = window.history.pushState;
    window.history.pushState = (...args) => {
      origPush.apply(window.history, args);
      this._updateVisibility();
    };
  }

  _buildDOM() {
    // Header action button
    this._btn = document.createElement("button");
    this._btn.type = "button";
    this._btn.className = "header-action-btn btn-icon global-chat-btn";
    this._btn.setAttribute("aria-label", "AI Assistant");
    this._btn.setAttribute("title", "AI Assistant");
    this._btn.innerHTML = '<i class="action-icon icon-bot-message-square"></i>';

    // Overlay
    this._overlay = document.createElement("div");
    this._overlay.className = "global-chat-overlay";
    this._overlay.setAttribute("aria-hidden", "true");
    this._overlay.innerHTML = `
      <div class="global-chat-overlay-bg"></div>
      <div class="global-chat-dialog" role="dialog" aria-modal="true" aria-labelledby="global-chat-title">
        <div class="global-chat-dialog-header">
          <div class="global-chat-dialog-title" id="global-chat-title">
            <i class="icon-bot-message-square"></i>
            AI Assistant
          </div>
          <button type="button" class="global-chat-dialog-close" aria-label="Close chat" title="Close (Esc)">
            <i class="icon-x"></i>
          </button>
        </div>
        <div class="global-chat-body"></div>
      </div>
    `;

    // Insert as first child of the action items group (before search button) so it
    // folds together with the other actions on mid screens. Fall back to the
    // actions container, then body, if the expected structure is missing.
    const headerActionsItems =
      document.querySelector(".header-actions-items") ||
      document.querySelector(".header-actions");
    if (headerActionsItems) {
      headerActionsItems.insertBefore(this._btn, headerActionsItems.firstChild);
    } else {
      document.body.appendChild(this._btn);
    }
    document.body.appendChild(this._overlay);
  }

  _setupEvents() {
    this._btn.addEventListener("click", () => this.toggle());

    // Close via overlay background click
    this._overlay.querySelector(".global-chat-overlay-bg").addEventListener("click", () => this.close());

    // Close button
    this._overlay.querySelector(".global-chat-dialog-close").addEventListener("click", () => this.close());

  }

  toggle() {
    if (this._isOpen) {
      this.close();
    } else {
      this.open();
    }
  }

  open() {
    if (this._isOpen) return;
    this._isOpen = true;
    this._previousFocus = document.activeElement;

    // Lazy-init widget on first open
    if (!this._widget) {
      const body = this._overlay.querySelector(".global-chat-body");
      this._widget = new ChatWidget(body, {
        showSidebar: true,
        onClose: () => this.close(),
      });
    }

    this._btn.classList.add("is-open");
    this._overlay.classList.add("is-open");
    this._overlay.setAttribute("aria-hidden", "false");
    this._releaseEscape = pushEscapeHandler(() => this.close());

    // Load sessions and start polling
    this._widget.loadSessions();
    this._widget.startPolling(5000);

    playPanelOpen();

    // Focus the input after animation
    setTimeout(() => {
      const input = this._widget.$(".cw-chat-input");
      if (input) input.focus();
    }, 350);
  }

  close() {
    if (!this._isOpen) return;
    this._isOpen = false;

    this._btn.classList.remove("is-open");
    this._overlay.classList.remove("is-open");
    this._overlay.setAttribute("aria-hidden", "true");
    this._releaseEscape?.();
    this._releaseEscape = null;

    if (this._widget) {
      this._widget.stopPolling();
    }

    this._previousFocus?.focus?.();
    this._previousFocus = null;

    playPanelClose();
  }

  _updateVisibility() {
    if (!this._btn) return;
    // Hide on AI page since chat is already inline there
    const path = window.location.pathname;
    const isAiPage = path === "/ai" || path.startsWith("/ai/");
    this._btn.classList.toggle("hidden", isAiPage);

    // Close dialog if navigating to AI page
    if (isAiPage && this._isOpen) {
      this.close();
    }
  }
}

// Auto-init when module loads
const globalChat = new GlobalChat();

// Wait for DOM ready
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", () => globalChat.init());
} else {
  globalChat.init();
}

export { globalChat };
