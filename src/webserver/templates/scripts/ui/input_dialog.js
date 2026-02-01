/**
 * Input Dialog Component
 *
 * Modern replacement for window.prompt() with:
 * - Async/await support (returns Promise<{value: string}> or null)
 * - Customizable title, message, placeholder, default value
 * - Input types: text, number
 * - Validation callback support
 * - Keyboard support (Enter = confirm, Esc = cancel)
 * - Focus trap within dialog
 * - Glassmorphic design matching confirmation dialog
 * - Sound feedback on actions
 */

import { playPanelOpen, playPanelClose, playSuccess } from "../core/sounds.js";

class InputDialog {
  static activeDialog = null;

  /**
   * Show an input dialog
   * @param {Object} config - Dialog configuration
   * @param {string} config.title - Dialog title
   * @param {string} [config.message] - Optional description text
   * @param {string} [config.placeholder=''] - Input placeholder
   * @param {string} [config.defaultValue=''] - Default input value
   * @param {string} [config.confirmLabel='Continue'] - Confirm button label
   * @param {string} [config.cancelLabel='Cancel'] - Cancel button label
   * @param {string} [config.variant='default'] - Variant: default, warning
   * @param {string} [config.type='text'] - Input type: text, number
   * @param {Function} [config.validate] - Validation function (value) => string|null (null = valid)
   * @param {Function} [config.formatValue] - Value formatter (value) => formattedValue
   * @returns {Promise<{value: string}|null>} Returns {value} or null if cancelled
   */
  static async show(config) {
    // Close any existing dialog
    if (InputDialog.activeDialog) {
      InputDialog.activeDialog.destroy();
    }

    return new Promise((resolve) => {
      const dialog = new InputDialog(config, resolve);
      InputDialog.activeDialog = dialog;
      dialog.render();
    });
  }

  constructor(config, resolver) {
    this.config = {
      title: config.title || "Enter Value",
      message: config.message || null,
      placeholder: config.placeholder || "",
      defaultValue: config.defaultValue || "",
      confirmLabel: config.confirmLabel || "Continue",
      cancelLabel: config.cancelLabel || "Cancel",
      variant: config.variant || "default",
      type: config.type || "text",
      validate: config.validate || null,
      formatValue: config.formatValue || null,
    };
    this.resolver = resolver;
    this.element = null;
    this.backdrop = null;
    this.inputElement = null;
    this.errorElement = null;
  }

  render() {
    // Create backdrop
    this.backdrop = document.createElement("div");
    this.backdrop.className = "input-dialog-backdrop";
    this.backdrop.setAttribute("role", "presentation");

    // Create dialog
    this.element = document.createElement("div");
    this.element.className = `input-dialog input-dialog--${this.config.variant}`;
    this.element.setAttribute("role", "dialog");
    this.element.setAttribute("aria-modal", "true");
    this.element.setAttribute("aria-labelledby", "input-dialog-title");
    if (this.config.message) {
      this.element.setAttribute("aria-describedby", "input-dialog-message");
    }

    // Escape HTML helper
    const escapeHTML = (str) => {
      const div = document.createElement("div");
      div.textContent = str;
      return div.innerHTML;
    };

    // Variant icons
    const variantIcons = {
      default: '<i class="icon-pencil"></i>',
      warning: '<i class="icon-circle-alert"></i>',
    };

    const icon = variantIcons[this.config.variant] || variantIcons.default;

    // Build dialog HTML
    this.element.innerHTML = `
      <div class="input-dialog__header">
        <span class="input-dialog__icon" aria-hidden="true">${icon}</span>
        <h3 class="input-dialog__title" id="input-dialog-title">
          ${escapeHTML(this.config.title)}
        </h3>
      </div>
      <div class="input-dialog__content">
        ${
          this.config.message
            ? `
          <p class="input-dialog__message" id="input-dialog-message">
            ${escapeHTML(this.config.message)}
          </p>
        `
            : ""
        }
        <div class="input-dialog__field">
          <input
            type="${this.config.type}"
            class="input-dialog__input"
            id="input-dialog-input"
            placeholder="${escapeHTML(this.config.placeholder)}"
            value="${escapeHTML(this.config.defaultValue)}"
            autocomplete="off"
            spellcheck="false"
          />
          <span class="input-dialog__error" id="input-dialog-error" aria-live="polite"></span>
        </div>
      </div>
      <div class="input-dialog__footer">
        <button
          class="input-dialog__button input-dialog__button--cancel"
          type="button"
          data-action="cancel"
        >
          ${escapeHTML(this.config.cancelLabel)}
        </button>
        <button
          class="input-dialog__button input-dialog__button--confirm"
          type="button"
          data-action="confirm"
        >
          ${escapeHTML(this.config.confirmLabel)}
        </button>
      </div>
    `;

    // Get references to input and error elements
    this.inputElement = this.element.querySelector("#input-dialog-input");
    this.errorElement = this.element.querySelector("#input-dialog-error");

    // Attach event listeners
    this._attachEventListeners();

    // Apply inline styles (using CSS variables from foundation.css)
    this._applyStyles();

    // Add to DOM
    document.body.appendChild(this.backdrop);
    document.body.appendChild(this.element);

    // Trigger animation
    requestAnimationFrame(() => {
      this.backdrop.classList.add("input-dialog-backdrop--visible");
      this.element.classList.add("input-dialog--visible");
      playPanelOpen();
    });

    // Focus input after animation
    setTimeout(() => {
      if (this.inputElement) {
        this.inputElement.focus();
        this.inputElement.select();
      }
    }, 100);

    // Trap focus within dialog
    this._trapFocus();
  }

  _applyStyles() {
    // Backdrop styles
    Object.assign(this.backdrop.style, {
      position: "fixed",
      inset: "0",
      background: "rgba(0, 0, 0, 0.6)",
      backdropFilter: "blur(4px)",
      zIndex: "10002",
      opacity: "0",
      transition: "opacity 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
    });

    // Dialog styles
    Object.assign(this.element.style, {
      position: "fixed",
      top: "50%",
      left: "50%",
      transform: "translate(-50%, -50%) scale(0.9)",
      width: "90%",
      maxWidth: "440px",
      background: "rgba(18, 24, 39, 0.95)",
      backdropFilter: "blur(20px)",
      border: "1px solid rgba(255, 255, 255, 0.1)",
      borderRadius: "12px",
      boxShadow: "0 20px 60px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255, 255, 255, 0.05) inset",
      zIndex: "10003",
      padding: "24px",
      opacity: "0",
      transition:
        "opacity 0.2s cubic-bezier(0.4, 0, 0.2, 1), transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
    });

    // Header styles
    const header = this.element.querySelector(".input-dialog__header");
    if (header) {
      Object.assign(header.style, {
        display: "flex",
        alignItems: "center",
        gap: "12px",
        marginBottom: "16px",
      });
    }

    // Icon styles
    const iconEl = this.element.querySelector(".input-dialog__icon");
    if (iconEl) {
      const isWarning = this.config.variant === "warning";
      Object.assign(iconEl.style, {
        flexShrink: "0",
        width: "40px",
        height: "40px",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontSize: "24px",
        borderRadius: "8px",
        background: isWarning ? "rgba(210, 153, 34, 0.1)" : "rgba(88, 166, 255, 0.1)",
        color: isWarning ? "#d29922" : "#58a6ff",
      });
    }

    // Title styles
    const title = this.element.querySelector(".input-dialog__title");
    if (title) {
      Object.assign(title.style, {
        margin: "0",
        fontSize: "18px",
        fontWeight: "600",
        color: "rgba(255, 255, 255, 0.95)",
        lineHeight: "1.4",
        letterSpacing: "-0.01em",
      });
    }

    // Content styles
    const content = this.element.querySelector(".input-dialog__content");
    if (content) {
      Object.assign(content.style, {
        marginBottom: "24px",
      });
    }

    // Message styles
    const message = this.element.querySelector(".input-dialog__message");
    if (message) {
      Object.assign(message.style, {
        margin: "0 0 16px 0",
        fontSize: "14px",
        color: "rgba(255, 255, 255, 0.75)",
        lineHeight: "1.6",
      });
    }

    // Field container styles
    const field = this.element.querySelector(".input-dialog__field");
    if (field) {
      Object.assign(field.style, {
        display: "flex",
        flexDirection: "column",
        gap: "6px",
      });
    }

    // Input styles
    if (this.inputElement) {
      Object.assign(this.inputElement.style, {
        width: "100%",
        padding: "12px 14px",
        fontSize: "14px",
        fontFamily: "'JetBrains Mono', monospace",
        color: "rgba(255, 255, 255, 0.95)",
        background: "rgba(255, 255, 255, 0.05)",
        border: "1px solid rgba(255, 255, 255, 0.15)",
        borderRadius: "8px",
        outline: "none",
        transition: "all 0.2s ease",
        boxSizing: "border-box",
      });
    }

    // Error styles
    if (this.errorElement) {
      Object.assign(this.errorElement.style, {
        fontSize: "12px",
        color: "#f85149",
        minHeight: "18px",
        opacity: "0",
        transition: "opacity 0.2s ease",
      });
    }

    // Footer styles
    const footer = this.element.querySelector(".input-dialog__footer");
    if (footer) {
      Object.assign(footer.style, {
        display: "flex",
        gap: "12px",
        justifyContent: "flex-end",
      });
    }

    // Button base styles
    const buttons = this.element.querySelectorAll(".input-dialog__button");
    buttons.forEach((btn) => {
      Object.assign(btn.style, {
        padding: "10px 20px",
        fontSize: "14px",
        fontWeight: "600",
        border: "none",
        borderRadius: "8px",
        cursor: "pointer",
        transition: "all 0.2s ease",
        letterSpacing: "0.01em",
        lineHeight: "1.4",
        minWidth: "100px",
      });
    });

    // Cancel button styles
    const cancelBtn = this.element.querySelector(".input-dialog__button--cancel");
    if (cancelBtn) {
      Object.assign(cancelBtn.style, {
        background: "rgba(255, 255, 255, 0.08)",
        color: "rgba(255, 255, 255, 0.9)",
        border: "1px solid rgba(255, 255, 255, 0.15)",
      });
    }

    // Confirm button styles
    const confirmBtn = this.element.querySelector(".input-dialog__button--confirm");
    if (confirmBtn) {
      const isWarning = this.config.variant === "warning";
      Object.assign(confirmBtn.style, {
        background: isWarning ? "#d29922" : "#58a6ff",
        color: "#fff",
      });
    }
  }

  _attachEventListeners() {
    // Confirm button
    const confirmBtn = this.element.querySelector('[data-action="confirm"]');
    if (confirmBtn) {
      this._confirmHandler = () => this._handleConfirm();
      confirmBtn.addEventListener("click", this._confirmHandler);

      // Hover effects for confirm button
      confirmBtn.addEventListener("mouseenter", () => {
        confirmBtn.style.filter = "brightness(1.1)";
      });
      confirmBtn.addEventListener("mouseleave", () => {
        confirmBtn.style.filter = "brightness(1)";
      });
    }

    // Cancel button
    const cancelBtn = this.element.querySelector('[data-action="cancel"]');
    if (cancelBtn) {
      this._cancelHandler = () => this._handleCancel();
      cancelBtn.addEventListener("click", this._cancelHandler);

      // Hover effects for cancel button
      cancelBtn.addEventListener("mouseenter", () => {
        cancelBtn.style.background = "rgba(255, 255, 255, 0.12)";
        cancelBtn.style.borderColor = "rgba(255, 255, 255, 0.25)";
      });
      cancelBtn.addEventListener("mouseleave", () => {
        cancelBtn.style.background = "rgba(255, 255, 255, 0.08)";
        cancelBtn.style.borderColor = "rgba(255, 255, 255, 0.15)";
      });
    }

    // Backdrop click cancels
    this._backdropHandler = () => this._handleCancel();
    this.backdrop.addEventListener("click", this._backdropHandler);

    // Input focus/blur effects
    if (this.inputElement) {
      this._inputFocusHandler = () => {
        this.inputElement.style.borderColor = "#58a6ff";
        this.inputElement.style.boxShadow = "0 0 0 2px rgba(88, 166, 255, 0.2)";
      };
      this._inputBlurHandler = () => {
        const hasError = this.errorElement && this.errorElement.style.opacity === "1";
        this.inputElement.style.borderColor = hasError ? "#f85149" : "rgba(255, 255, 255, 0.15)";
        this.inputElement.style.boxShadow = "none";
      };
      this.inputElement.addEventListener("focus", this._inputFocusHandler);
      this.inputElement.addEventListener("blur", this._inputBlurHandler);

      // Clear error on input
      this._inputChangeHandler = () => {
        this._clearError();
      };
      this.inputElement.addEventListener("input", this._inputChangeHandler);
    }

    // Keyboard shortcuts
    this._keydownHandler = (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        this._handleConfirm();
      } else if (e.key === "Escape") {
        e.preventDefault();
        this._handleCancel();
      }
    };
    document.addEventListener("keydown", this._keydownHandler);
  }

  _validateInput() {
    if (!this.inputElement) return true;

    let value = this.inputElement.value;

    // Apply formatter if provided
    if (this.config.formatValue) {
      value = this.config.formatValue(value);
      this.inputElement.value = value;
    }

    // Run validation if provided
    if (this.config.validate) {
      const error = this.config.validate(value);
      if (error) {
        this._showError(error);
        return false;
      }
    }

    return true;
  }

  _showError(message) {
    if (this.errorElement) {
      this.errorElement.textContent = message;
      this.errorElement.style.opacity = "1";
    }
    if (this.inputElement) {
      this.inputElement.style.borderColor = "#f85149";
    }
  }

  _clearError() {
    if (this.errorElement) {
      this.errorElement.textContent = "";
      this.errorElement.style.opacity = "0";
    }
    if (this.inputElement && document.activeElement !== this.inputElement) {
      this.inputElement.style.borderColor = "rgba(255, 255, 255, 0.15)";
    }
  }

  _handleConfirm() {
    if (!this._validateInput()) {
      // Shake the input on validation error
      this.inputElement.style.animation = "none";
      void this.inputElement.offsetHeight; // Trigger reflow
      this.inputElement.style.animation = "input-dialog-shake 0.4s ease";
      return;
    }

    playSuccess();

    let value = this.inputElement ? this.inputElement.value : "";

    // Apply final formatting
    if (this.config.formatValue) {
      value = this.config.formatValue(value);
    }

    if (this.resolver) {
      this.resolver({ value });
      this.resolver = null;
    }
    this.destroy();
  }

  _handleCancel() {
    playPanelClose();
    if (this.resolver) {
      this.resolver(null);
      this.resolver = null;
    }
    this.destroy();
  }

  destroy() {
    // Resolve promise if still pending (safety net)
    if (this.resolver) {
      this.resolver(null);
      this.resolver = null;
    }

    // Remove all event listeners
    if (this._keydownHandler) {
      document.removeEventListener("keydown", this._keydownHandler);
      this._keydownHandler = null;
    }

    if (this._confirmHandler) {
      const confirmBtn = this.element?.querySelector('[data-action="confirm"]');
      if (confirmBtn) {
        confirmBtn.removeEventListener("click", this._confirmHandler);
      }
      this._confirmHandler = null;
    }

    if (this._cancelHandler) {
      const cancelBtn = this.element?.querySelector('[data-action="cancel"]');
      if (cancelBtn) {
        cancelBtn.removeEventListener("click", this._cancelHandler);
      }
      this._cancelHandler = null;
    }

    if (this._backdropHandler && this.backdrop) {
      this.backdrop.removeEventListener("click", this._backdropHandler);
      this._backdropHandler = null;
    }

    if (this.inputElement) {
      if (this._inputFocusHandler) {
        this.inputElement.removeEventListener("focus", this._inputFocusHandler);
        this._inputFocusHandler = null;
      }
      if (this._inputBlurHandler) {
        this.inputElement.removeEventListener("blur", this._inputBlurHandler);
        this._inputBlurHandler = null;
      }
      if (this._inputChangeHandler) {
        this.inputElement.removeEventListener("input", this._inputChangeHandler);
        this._inputChangeHandler = null;
      }
    }

    // Remove focus trap handler
    if (this._trapFocusHandler && this.element) {
      this.element.removeEventListener("keydown", this._trapFocusHandler);
      this._trapFocusHandler = null;
    }

    // Animate out
    if (this.backdrop) {
      this.backdrop.classList.remove("input-dialog-backdrop--visible");
      this.backdrop.style.opacity = "0";
    }
    if (this.element) {
      this.element.classList.remove("input-dialog--visible");
      this.element.style.opacity = "0";
      this.element.style.transform = "translate(-50%, -50%) scale(0.9)";
    }

    // Remove from DOM after animation
    setTimeout(() => {
      if (this.backdrop && this.backdrop.parentNode) {
        this.backdrop.remove();
      }
      if (this.element && this.element.parentNode) {
        this.element.remove();
      }

      // Clear active dialog
      if (InputDialog.activeDialog === this) {
        InputDialog.activeDialog = null;
      }
    }, 300);
  }

  _trapFocus() {
    const focusableElements = this.element.querySelectorAll(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );

    if (focusableElements.length === 0) {
      return;
    }

    const firstElement = focusableElements[0];
    const lastElement = focusableElements[focusableElements.length - 1];

    this._trapFocusHandler = (e) => {
      if (e.key === "Tab") {
        if (e.shiftKey && document.activeElement === firstElement) {
          e.preventDefault();
          lastElement.focus();
        } else if (!e.shiftKey && document.activeElement === lastElement) {
          e.preventDefault();
          firstElement.focus();
        }
      }
    };

    this.element.addEventListener("keydown", this._trapFocusHandler);
  }
}

// Inject keyframe animation for shake effect
const styleId = "input-dialog-keyframes";
if (!document.getElementById(styleId)) {
  const style = document.createElement("style");
  style.id = styleId;
  style.textContent = `
    @keyframes input-dialog-shake {
      0%, 100% { transform: translateX(0); }
      10%, 30%, 50%, 70%, 90% { transform: translateX(-4px); }
      20%, 40%, 60%, 80% { transform: translateX(4px); }
    }
    .input-dialog-backdrop--visible { opacity: 1 !important; }
    .input-dialog--visible { opacity: 1 !important; transform: translate(-50%, -50%) scale(1) !important; }
  `;
  document.head.appendChild(style);
}

// Export for use in other modules
export { InputDialog };
