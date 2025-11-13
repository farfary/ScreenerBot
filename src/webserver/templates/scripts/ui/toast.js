/**
 * Toast Component - Individual toast UI renderer
 *
 * Handles:
 * - HTML structure rendering with variants
 * - Animations (enter, active, hover, exit)
 * - Event listeners (close, actions, hover)
 * - Progress bar updates
 * - Accessibility (ARIA labels, keyboard navigation)
 * - Icon mapping for each variant
 */

export class Toast {
  constructor(config, id) {
    this.config = config;
    this.id = id;
    this.element = null;
  }

  /**
   * Render the toast HTML element
   * @returns {HTMLElement} Toast element
   */
  render() {
    this.element = document.createElement("div");
    this.element.className = `toast toast--${this.config.type}`;
    this.element.setAttribute("data-toast-id", this.id);
    this.element.setAttribute("role", "alert");
    this.element.setAttribute("aria-live", this.config.type === "error" ? "assertive" : "polite");
    this.element.setAttribute("aria-atomic", "true");

    // Build HTML structure
    this.element.innerHTML = this._buildHTML();

    // Setup ARIA relationships after HTML is inserted
    this._setupAriaRelationships();

    // Attach event listeners
    this._attachEventListeners();

    return this.element;
  }

  /**
   * Setup ARIA relationships for accessibility
   */
  _setupAriaRelationships() {
    const titleEl = this.element.querySelector(".toast__title");
    const messageEl = this.element.querySelector(".toast__message");
    const descEl = this.element.querySelector(".toast__description");

    // Create unique IDs for ARIA relationships
    const titleId = `toast-title-${this.id}`;
    const messageId = `toast-message-${this.id}`;
    const descId = `toast-desc-${this.id}`;

    if (titleEl) {
      titleEl.id = titleId;
    }
    if (messageEl) {
      messageEl.id = messageId;
    }
    if (descEl) {
      descEl.id = descId;
    }

    // Link toast to its content via aria-labelledby and aria-describedby
    const labelledBy = [titleId];
    const describedBy = [];

    if (messageEl) describedBy.push(messageId);
    if (descEl) describedBy.push(descId);

    if (labelledBy.length > 0) {
      this.element.setAttribute("aria-labelledby", labelledBy.join(" "));
    }
    if (describedBy.length > 0) {
      this.element.setAttribute("aria-describedby", describedBy.join(" "));
    }

    // Update close button label to be more specific
    const closeBtn = this.element.querySelector(".toast__close");
    if (closeBtn && this.config.title) {
      closeBtn.setAttribute("aria-label", `Close ${this.config.title}`);
    }
  }

  /**
   * Build the toast HTML structure
   * @returns {string} HTML string
   */
  _buildHTML() {
    const { type, title, description, message, icon, progress, actions } = this.config;

    // Escape HTML to prevent XSS
    const escapeHTML = (str) => {
      if (!str) return "";
      const div = document.createElement("div");
      div.textContent = str;
      return div.innerHTML;
    };

    const titleHTML = escapeHTML(title);
    const descriptionHTML = description ? escapeHTML(description) : "";
    const messageHTML = message ? escapeHTML(message) : "";

    // Icon with loading animation for loading type
    const iconHTML =
      type === "loading"
        ? `<span class="toast__icon toast__icon--loading" aria-hidden="true">${icon}</span>`
        : `<span class="toast__icon" aria-hidden="true">${icon}</span>`;

    // Progress bar (if applicable)
    const progressHTML =
      progress !== undefined
        ? `
			<div class="toast__progress" role="progressbar" aria-valuenow="${progress}" aria-valuemin="0" aria-valuemax="100">
				<div class="toast__progress-bar" style="width: ${progress}%"></div>
			</div>
		`
        : "";

    // Actions (if any)
    const actionsHTML =
      actions && actions.length > 0
        ? `
			<div class="toast__actions">
				${actions
          .map(
            (action, index) => `
					<button 
						class="toast__action toast__action--${action.style || "primary"}" 
						data-action-index="${index}"
						type="button"
					>
						${escapeHTML(action.label)}
					</button>
				`
          )
          .join("")}
			</div>
		`
        : "";

    return `
			<div class="toast__header">
				${iconHTML}
				<div class="toast__title-wrapper">
					<h4 class="toast__title">${titleHTML}</h4>
					${descriptionHTML ? `<p class="toast__description">${descriptionHTML}</p>` : ""}
				</div>
				<button class="toast__close" aria-label="Close notification" type="button">
					<svg width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
						<path d="M1 1L11 11M1 11L11 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
					</svg>
				</button>
			</div>
			${messageHTML ? `<div class="toast__message">${messageHTML}</div>` : ""}
			${progressHTML}
			${actionsHTML}
		`;
  }

  /**
   * Attach event listeners to toast elements
   */
  _attachEventListeners() {
    // Close button
    const closeBtn = this.element.querySelector(".toast__close");
    if (closeBtn) {
      closeBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        this._handleClose();
      });

      // Keyboard accessibility for close button
      closeBtn.addEventListener("keydown", (e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          this._handleClose();
        }
      });
    }

    // Action buttons
    const actionButtons = this.element.querySelectorAll(".toast__action");
    actionButtons.forEach((btn) => {
      btn.addEventListener("click", (e) => {
        e.stopPropagation();
        const index = parseInt(btn.getAttribute("data-action-index"), 10);
        this._handleAction(index);
      });
    });

    // Make toast focusable for keyboard navigation
    this.element.setAttribute("tabindex", "0");

    // Keyboard shortcuts
    this.element.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        this._handleClose();
      }
    });

    // Click on toast (optional callback)
    this.element.addEventListener("click", () => {
      if (this.config.onClick) {
        this.config.onClick();
      }
    });
  }

  /**
   * Handle close button click
   */
  _handleClose() {
    // Call onDismiss callback if provided
    if (this.config.onDismiss) {
      try {
        this.config.onDismiss();
      } catch (error) {
        console.error("Toast onDismiss callback error:", error);
      }
    }

    // Trigger dismiss from ToastManager
    // The manager handles the actual removal and animation
    import("../core/toast.js").then(({ toastManager }) => {
      toastManager.dismiss(this.id);
    });
  }

  /**
   * Handle action button click
   * @param {number} index - Action index
   */
  _handleAction(index) {
    const action = this.config.actions[index];
    if (!action || !action.callback) {
      return;
    }

    try {
      action.callback();
    } catch (error) {
      console.error(`Toast action callback error (index ${index}):`, error);
    }

    // Auto-dismiss after action (unless it's a persistent action)
    if (!action.persistent) {
      setTimeout(() => {
        import("../core/toast.js").then(({ toastManager }) => {
          toastManager.dismiss(this.id);
        });
      }, 300);
    }
  }

  /**
   * Destroy the toast (cleanup)
   */
  destroy() {
    if (this.element && this.element.parentNode) {
      this.element.remove();
    }
    this.element = null;
  }
}
