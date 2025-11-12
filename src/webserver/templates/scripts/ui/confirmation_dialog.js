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
 */

class ConfirmationDialog {
	static activeDialog = null;

	/**
	 * Show a confirmation dialog
	 * @param {Object} config - Dialog configuration
	 * @param {string} config.title - Dialog title
	 * @param {string} config.message - Dialog message
	 * @param {string} [config.confirmLabel='Confirm'] - Confirm button label
	 * @param {string} [config.cancelLabel='Cancel'] - Cancel button label
	 * @param {string} [config.variant='warning'] - Variant: danger, warning, info
	 * @param {string} [config.checkbox] - Optional checkbox text (e.g., "Don't ask again")
	 * @returns {Promise<{confirmed: boolean, checkboxChecked: boolean}>}
	 */
	static async show(config) {
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
			title: config.title || "Confirm Action",
			message: config.message || "Are you sure?",
			confirmLabel: config.confirmLabel || "Confirm",
			cancelLabel: config.cancelLabel || "Cancel",
			variant: config.variant || "warning",
			checkbox: config.checkbox || null,
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
		this.element.className = `confirmation-dialog confirmation-dialog--${this.config.variant}`;
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
			danger: "⚠",
			warning: "❗",
			info: "ℹ",
		};

		const icon = variantIcons[this.config.variant] || "❗";

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
				<button 
					class="confirmation-dialog__button confirmation-dialog__button--confirm" 
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

		// Trigger animation
		requestAnimationFrame(() => {
			this.backdrop.classList.add("confirmation-dialog-backdrop--visible");
			this.element.classList.add("confirmation-dialog--visible");
		});

		// Focus confirm button by default
		setTimeout(() => {
			const confirmBtn = this.element.querySelector('[data-action="confirm"]');
			if (confirmBtn) {
				confirmBtn.focus();
			}
		}, 100);

		// Trap focus within dialog
		this._trapFocus();
	}

	_attachEventListeners() {
		// Confirm button
		const confirmBtn = this.element.querySelector('[data-action="confirm"]');
		if (confirmBtn) {
			confirmBtn.addEventListener("click", () => this._handleConfirm());
		}

		// Cancel button
		const cancelBtn = this.element.querySelector('[data-action="cancel"]');
		if (cancelBtn) {
			cancelBtn.addEventListener("click", () => this._handleCancel());
		}

		// Backdrop click cancels
		this.backdrop.addEventListener("click", () => this._handleCancel());

		// Checkbox
		const checkbox = this.element.querySelector("#confirmation-dialog-checkbox");
		if (checkbox) {
			checkbox.addEventListener("change", (e) => {
				this.checkboxChecked = e.target.checked;
			});
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

	_handleConfirm() {
		if (this.resolver) {
			this.resolver({
				confirmed: true,
				checkboxChecked: this.checkboxChecked,
			});
			this.resolver = null;
		}
		this.destroy();
	}

	_handleCancel() {
		if (this.resolver) {
			this.resolver({
				confirmed: false,
				checkboxChecked: this.checkboxChecked,
			});
			this.resolver = null;
		}
		this.destroy();
	}

	destroy() {
		// Resolve promise if still pending (user closed without action)
		if (this.resolver) {
			this.resolver({
				confirmed: false,
				checkboxChecked: this.checkboxChecked,
				cancelled: true,
			});
			this.resolver = null;
		}

		// Remove event listeners
		if (this._keydownHandler) {
			document.removeEventListener("keydown", this._keydownHandler);
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
			'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
		);

		if (focusableElements.length === 0) {
			return;
		}

		const firstElement = focusableElements[0];
		const lastElement = focusableElements[focusableElements.length - 1];

		this.element.addEventListener("keydown", (e) => {
			if (e.key === "Tab") {
				if (e.shiftKey && document.activeElement === firstElement) {
					e.preventDefault();
					lastElement.focus();
				} else if (!e.shiftKey && document.activeElement === lastElement) {
					e.preventDefault();
					firstElement.focus();
				}
			}
		});
	}
}

// Export for use in other modules
export { ConfirmationDialog };
