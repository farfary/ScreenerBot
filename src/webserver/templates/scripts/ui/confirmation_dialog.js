// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Confirmation Dialog Component
 *
 * Modern replacement for window.confirm() with:
 * - Async/await support (returns Promise<boolean>)
 * - Customizable title, message, buttons
 * - Keyboard support (Enter = confirm, Esc = cancel)
 * - Variants: danger, warning, info
 * - Optional checkbox ("Don't ask again")
 * - Glassmorphic design matching toast/notification system
 * - Sound feedback on actions
 */

import { playClick } from "../core/sounds.js";
import { pushEscapeHandler } from "../core/escape_stack.js";
import { closeAllMenus } from "../core/menu_manager.js";

class ConfirmationDialog {
  static activeDialog = null;

  /**
   * Show a confirmation dialog
   * @param {Object} config - Dialog configuration
   * @param {string} config.title - Dialog title
   * @param {string} config.message - Dialog message
   * @param {string} [config.confirmLabel='Confirm'] - Confirm button label
   * @param {string} [config.cancelLabel='Cancel'] - Cancel button label
   * @param {string} [config.alternateLabel] - Optional third action between cancel and
   *   confirm (e.g. "Discard" beside "Save"); choosing it resolves with `alternate: true`
   * @param {string} [config.variant='warning'] - Variant: danger, warning, info
   * @param {string} [config.confirmTone] - `"primary"` fills the confirm button as the
   *   shared primary button (a save), whatever the variant's colour
   * @param {string} [config.checkbox] - Optional checkbox text (e.g., "Don't ask again")
   * @param {string} [config.body] - Trusted markup shown below the message; the caller
   *   escapes every value it interpolates
   * @param {Function} [config.canConfirm] - `(dialogElement) => boolean`; while it returns
   *   false the confirm action is disabled. Re-read on every input in the dialog.
   * @returns {Promise<{confirmed: boolean, alternate: boolean, checkboxChecked: boolean}>}
   */
  static async show(config) {
    // A dialog takes over the viewport — dismiss any open dropdown/menu underneath.
    closeAllMenus("dialog-open");

    // Close any existing dialog
    if (ConfirmationDialog.activeDialog) {
      ConfirmationDialog.activeDialog.destroy();
    }

    return new Promise((resolve) => {
      const dialog = new ConfirmationDialog(config, resolve);
      ConfirmationDialog.activeDialog = dialog;
      dialog.render();
    });
  }

  constructor(config, resolver) {
    this.config = {
      title: config.title || I18n.t("shell-confirm-title"),
      message: config.message || I18n.t("shell-confirm-message"),
      confirmLabel: config.confirmLabel || I18n.t("common-action-confirm"),
      cancelLabel: config.cancelLabel || I18n.t("common-action-cancel"),
      alternateLabel: config.alternateLabel || null,
      variant: config.variant || "warning",
      confirmTone: config.confirmTone === "primary" ? "primary" : null,
      checkbox: config.checkbox || null,
      body: config.body || "",
      canConfirm: typeof config.canConfirm === "function" ? config.canConfirm : null,
    };
    this.resolver = resolver;
    this.element = null;
    this.backdrop = null;
    this.checkboxChecked = false;
  }

  render() {
    // Create backdrop
    this.backdrop = document.createElement("div");
    this.backdrop.className = "confirmation-dialog-backdrop";
    this.backdrop.setAttribute("role", "presentation");

    // Create dialog
    this.element = document.createElement("div");
    this.element.className = `confirmation-dialog confirmation-dialog--${this.config.variant}${
      this.config.alternateLabel ? " confirmation-dialog--with-alternate" : ""
    }`;
    this.element.setAttribute("role", "dialog");
    this.element.setAttribute("aria-modal", "true");
    this.element.setAttribute("aria-labelledby", "confirmation-dialog-title");
    this.element.setAttribute("aria-describedby", "confirmation-dialog-message");

    // Escape HTML
    const escapeHTML = (str) => {
      const div = document.createElement("div");
      div.textContent = str;
      return div.innerHTML;
    };

    const variantIcons = {
      danger: '<i class="icon-triangle-alert"></i>',
      warning: '<i class="icon-circle-alert"></i>',
      info: '<i class="icon-info"></i>',
    };

    const icon = variantIcons[this.config.variant] || '<i class="icon-circle-alert"></i>';

    this.element.innerHTML = `
			<div class="confirmation-dialog__header">
				<span class="confirmation-dialog__icon" aria-hidden="true">${icon}</span>
				<h3 class="confirmation-dialog__title" id="confirmation-dialog-title">
					${escapeHTML(this.config.title)}
				</h3>
			</div>
			<div class="confirmation-dialog__content">
				<p class="confirmation-dialog__message" id="confirmation-dialog-message">
					${escapeHTML(this.config.message)}
				</p>
				${this.config.body ? `<div class="confirmation-dialog__body">${this.config.body}</div>` : ""}
				${
          this.config.checkbox
            ? `
					<label class="confirmation-dialog__checkbox">
						<input type="checkbox" id="confirmation-dialog-checkbox" />
						<span>${escapeHTML(this.config.checkbox)}</span>
					</label>
				`
            : ""
        }
			</div>
			<div class="confirmation-dialog__footer">
				<button 
					class="confirmation-dialog__button confirmation-dialog__button--cancel" 
					type="button"
					data-action="cancel"
				>
					${escapeHTML(this.config.cancelLabel)}
				</button>
				${
          this.config.alternateLabel
            ? `
				<button
					class="confirmation-dialog__button confirmation-dialog__button--alternate"
					type="button"
					data-action="alternate"
				>
					${escapeHTML(this.config.alternateLabel)}
				</button>
				`
            : ""
        }
				<button 
					class="confirmation-dialog__button confirmation-dialog__button--confirm ${
            this.config.confirmTone === "primary" ? "confirmation-dialog__button--primary" : ""
          }" 
					type="button"
					data-action="confirm"
				>
					${escapeHTML(this.config.confirmLabel)}
				</button>
			</div>
		`;

    // Attach event listeners
    this._attachEventListeners();

    // Add to DOM
    document.body.appendChild(this.backdrop);
    document.body.appendChild(this.element);
    this._fitFooter();

    // Trigger animation
    requestAnimationFrame(() => {
      this.backdrop.classList.add("confirmation-dialog-backdrop--visible");
      this.element.classList.add("confirmation-dialog--visible");
    });

    // Focus the confirm button, or the first field while confirming is gated on input.
    setTimeout(() => {
      const target = this.config.canConfirm
        ? this.element.querySelector(".confirmation-dialog__body input")
        : this.element.querySelector('[data-action="confirm"]');
      target?.focus();
    }, 100);

    // Trap focus within dialog
    this._trapFocus();
  }

  /**
   * Keep the actions on one line while their labels fit it; otherwise stack them at
   * full width, so no label wraps inside its button in any locale.
   */
  _fitFooter() {
    const footer = this.element.querySelector(".confirmation-dialog__footer");
    if (!footer) return;
    footer.classList.remove("confirmation-dialog__footer--stacked");
    if (footer.scrollWidth > footer.clientWidth) {
      footer.classList.add("confirmation-dialog__footer--stacked");
    }
  }

  _attachEventListeners() {
    // Confirm button
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

    const alternateBtn = this.element.querySelector('[data-action="alternate"]');
    if (alternateBtn) {
      this._alternateHandler = () => this._settle({ confirmed: false, alternate: true });
      alternateBtn.addEventListener("click", this._alternateHandler);
    }

    // Backdrop click cancels
    this._backdropHandler = () => this._handleCancel();
    this.backdrop.addEventListener("click", this._backdropHandler);

    if (this.config.canConfirm) {
      this._gateHandler = () => this._syncConfirm();
      this.element.addEventListener("input", this._gateHandler);
      this.element.addEventListener("change", this._gateHandler);
      this._syncConfirm();
    }

    // Checkbox
    const checkbox = this.element.querySelector("#confirmation-dialog-checkbox");
    if (checkbox) {
      this._checkboxHandler = (e) => {
        this.checkboxChecked = e.target.checked;
      };
      checkbox.addEventListener("change", this._checkboxHandler);
    }

    // Keyboard shortcuts. Escape goes through the stack so it cancels this
    // dialog, not an overlay opened beneath it.
    this._keydownHandler = (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        this._handleConfirm();
      }
    };
    document.addEventListener("keydown", this._keydownHandler);
    this._releaseEscape = pushEscapeHandler(() => this._handleCancel());
  }

  /** True while the confirm action is allowed. */
  _confirmAllowed() {
    return !this.config.canConfirm || Boolean(this.config.canConfirm(this.element));
  }

  _syncConfirm() {
    const confirmBtn = this.element?.querySelector('[data-action="confirm"]');
    if (confirmBtn) confirmBtn.disabled = !this._confirmAllowed();
  }

  _handleConfirm() {
    if (!this._confirmAllowed()) return;
    this._settle({ confirmed: true, alternate: false });
  }

  _handleCancel() {
    this._settle({ confirmed: false, alternate: false });
  }

  _settle(choice) {
    playClick();
    if (this.resolver) {
      this.resolver({ ...choice, checkboxChecked: this.checkboxChecked });
      this.resolver = null;
    }
    this.destroy();
  }

  destroy() {
    // Resolve promise if still pending (user closed without action)
    if (this.resolver) {
      this.resolver({
        confirmed: false,
        alternate: false,
        checkboxChecked: this.checkboxChecked,
        cancelled: true,
      });
      this.resolver = null;
    }

    // Remove all event listeners
    if (this._keydownHandler) {
      document.removeEventListener("keydown", this._keydownHandler);
      this._keydownHandler = null;
    }
    if (this._releaseEscape) {
      this._releaseEscape();
      this._releaseEscape = null;
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

    if (this._alternateHandler) {
      const alternateBtn = this.element?.querySelector('[data-action="alternate"]');
      if (alternateBtn) {
        alternateBtn.removeEventListener("click", this._alternateHandler);
      }
      this._alternateHandler = null;
    }

    if (this._backdropHandler && this.backdrop) {
      this.backdrop.removeEventListener("click", this._backdropHandler);
      this._backdropHandler = null;
    }

    if (this._gateHandler && this.element) {
      this.element.removeEventListener("input", this._gateHandler);
      this.element.removeEventListener("change", this._gateHandler);
      this._gateHandler = null;
    }

    if (this._checkboxHandler) {
      const checkbox = this.element?.querySelector("#confirmation-dialog-checkbox");
      if (checkbox) {
        checkbox.removeEventListener("change", this._checkboxHandler);
      }
      this._checkboxHandler = null;
    }

    if (this._trapFocusHandler && this.element) {
      this.element.removeEventListener("keydown", this._trapFocusHandler);
      this._trapFocusHandler = null;
    }

    // Animate out
    if (this.backdrop) {
      this.backdrop.classList.remove("confirmation-dialog-backdrop--visible");
    }
    if (this.element) {
      this.element.classList.remove("confirmation-dialog--visible");
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
      if (ConfirmationDialog.activeDialog === this) {
        ConfirmationDialog.activeDialog = null;
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
export { ConfirmationDialog };
