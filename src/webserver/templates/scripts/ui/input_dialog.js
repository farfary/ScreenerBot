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
 * - Sound feedback on actions
 *
 * Presentation lives entirely in `styles/ui/input_dialog.css`. This module
 * writes markup and state classes and nothing else - it used to assign every
 * colour, box and hover effect as an inline style, which no stylesheet could
 * override and which left the dialog dark-only.
 */

import { playClick, playError } from "../core/sounds.js";

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

    // Add to DOM
    document.body.appendChild(this.backdrop);
    document.body.appendChild(this.element);

    // Trigger animation
    requestAnimationFrame(() => {
      this.backdrop.classList.add("input-dialog-backdrop--visible");
      this.element.classList.add("input-dialog--visible");
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

  _attachEventListeners() {
    // Confirm button - hover and focus are the stylesheet's, not ours.
    const confirmBtn = this.element.querySelector('[data-action="confirm"]');
    if (confirmBtn) {
      this._confirmHandler = () => this._handleConfirm();
      confirmBtn.addEventListener("click", this._confirmHandler);
    }

    // Cancel button
    const cancelBtn = this.element.querySelector('[data-action="cancel"]');
    if (cancelBtn) {
      this._cancelHandler = () => this._handleCancel();
      cancelBtn.addEventListener("click", this._cancelHandler);
    }

    // Backdrop click cancels
    this._backdropHandler = () => this._handleCancel();
    this.backdrop.addEventListener("click", this._backdropHandler);

    // Clear error on input
    if (this.inputElement) {
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
      this.errorElement.classList.add("input-dialog__error--visible");
    }
    if (this.inputElement) {
      this.inputElement.classList.add("input-dialog__input--invalid");
    }
  }

  _clearError() {
    if (this.errorElement) {
      this.errorElement.textContent = "";
      this.errorElement.classList.remove("input-dialog__error--visible");
    }
    if (this.inputElement) {
      this.inputElement.classList.remove("input-dialog__input--invalid");
    }
  }

  _handleConfirm() {
    if (!this._validateInput()) {
      playError();
      // Restart the shake: removing the class and reading a layout property
      // forces the animation to run again on a second failed attempt.
      this.inputElement.classList.remove("input-dialog__input--shake");
      void this.inputElement.offsetHeight;
      this.inputElement.classList.add("input-dialog__input--shake");
      return;
    }

    playClick();

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
    playClick();
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

    if (this.inputElement && this._inputChangeHandler) {
      this.inputElement.removeEventListener("input", this._inputChangeHandler);
      this._inputChangeHandler = null;
    }

    // Remove focus trap handler
    if (this._trapFocusHandler && this.element) {
      this.element.removeEventListener("keydown", this._trapFocusHandler);
      this._trapFocusHandler = null;
    }

    // Animate out: the resting rule already holds the closed state.
    if (this.backdrop) {
      this.backdrop.classList.remove("input-dialog-backdrop--visible");
    }
    if (this.element) {
      this.element.classList.remove("input-dialog--visible");
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

// Export for use in other modules
export { InputDialog };
