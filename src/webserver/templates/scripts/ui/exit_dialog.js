/**
 * Exit Confirmation Dialog Component
 *
 * Modern, animated dialog shown when user tries to close the ScreenerBot app.
 * Offers three options:
 * - Minimize to Tray: Hide window but keep app running
 * - Exit App: Fully close the application
 * - Cancel: Close dialog, stay in app
 *
 * Only shows in Tauri context (desktop app).
 */

class ExitDialog {
  static instance = null;
  static isVisible = false;

  constructor() {
    this.element = null;
    this.backdrop = null;
    this.resolver = null;
    this._keydownHandler = null;
  }

  /**
   * Initialize the exit dialog and hook into window close event
   * Should be called once when the app initializes
   */
  static async init() {
    // Only initialize in Tauri context
    if (!window.__TAURI__) {
      console.log("[ExitDialog] Not in Tauri context, skipping initialization");
      return;
    }

    console.log("[ExitDialog] Initializing exit confirmation dialog...");

    try {
      // Get the current window
      let appWindow = null;
      if (window.__TAURI__.window?.getCurrentWindow) {
        appWindow = window.__TAURI__.window.getCurrentWindow();
      } else if (window.__TAURI__.webviewWindow?.getCurrentWebviewWindow) {
        appWindow = window.__TAURI__.webviewWindow.getCurrentWebviewWindow();
      }

      if (!appWindow) {
        console.warn("[ExitDialog] Could not get Tauri window API");
        return;
      }

      // Listen for window close requests
      await appWindow.onCloseRequested(async (event) => {
        console.log("[ExitDialog] Window close requested, showing confirmation");

        // Prevent the default close behavior
        event.preventDefault();

        // Show our custom dialog
        const result = await ExitDialog.show();

        if (result === "exit") {
          console.log("[ExitDialog] User chose to exit app");
          // Use destroy() to force close without triggering another close request
          await appWindow.destroy();
        } else if (result === "minimize") {
          console.log("[ExitDialog] User chose to minimize to tray");
          await appWindow.hide();
        } else {
          console.log("[ExitDialog] User cancelled, staying in app");
        }
      });

      console.log("[ExitDialog] Successfully hooked into window close event");
    } catch (error) {
      console.error("[ExitDialog] Failed to initialize:", error);
    }
  }

  /**
   * Show the exit confirmation dialog
   * @returns {Promise<'exit'|'minimize'|'cancel'>} User's choice
   */
  static async show() {
    // Prevent multiple dialogs
    if (ExitDialog.isVisible) {
      return "cancel";
    }

    return new Promise((resolve) => {
      const dialog = new ExitDialog();
      ExitDialog.instance = dialog;
      ExitDialog.isVisible = true;
      dialog.resolver = resolve;
      dialog.render();
    });
  }

  render() {
    // Create backdrop
    this.backdrop = document.createElement("div");
    this.backdrop.className = "exit-dialog-backdrop";
    this.backdrop.setAttribute("role", "presentation");

    // Create dialog
    this.element = document.createElement("div");
    this.element.className = "exit-dialog";
    this.element.setAttribute("role", "dialog");
    this.element.setAttribute("aria-modal", "true");
    this.element.setAttribute("aria-labelledby", "exit-dialog-title");
    this.element.setAttribute("aria-describedby", "exit-dialog-description");

    this.element.innerHTML = `
      <div class="exit-dialog__header">
        <div class="exit-dialog__icon-wrapper">
          <i class="exit-dialog__icon icon-power"></i>
        </div>
        <h2 class="exit-dialog__title" id="exit-dialog-title">Close ScreenerBot?</h2>
        <p class="exit-dialog__description" id="exit-dialog-description">
          Choose how you'd like to close the application
        </p>
      </div>

      <div class="exit-dialog__options">
        <button class="exit-dialog__option exit-dialog__option--minimize" data-action="minimize" type="button">
          <div class="exit-dialog__option-icon">
            <i class="icon-minimize-2"></i>
          </div>
          <div class="exit-dialog__option-content">
            <span class="exit-dialog__option-title">Minimize to Tray</span>
            <span class="exit-dialog__option-subtitle">Keep running in background</span>
          </div>
          <div class="exit-dialog__option-arrow">
            <i class="icon-chevron-right"></i>
          </div>
        </button>

        <button class="exit-dialog__option exit-dialog__option--exit" data-action="exit" type="button">
          <div class="exit-dialog__option-icon exit-dialog__option-icon--danger">
            <i class="icon-power-off"></i>
          </div>
          <div class="exit-dialog__option-content">
            <span class="exit-dialog__option-title">Exit App</span>
            <span class="exit-dialog__option-subtitle">Close completely and stop all services</span>
          </div>
          <div class="exit-dialog__option-arrow">
            <i class="icon-chevron-right"></i>
          </div>
        </button>
      </div>

      <div class="exit-dialog__footer">
        <button class="exit-dialog__cancel" data-action="cancel" type="button">
          <i class="icon-x"></i>
          <span>Cancel</span>
        </button>
      </div>

      <div class="exit-dialog__glow exit-dialog__glow--1"></div>
      <div class="exit-dialog__glow exit-dialog__glow--2"></div>
    `;

    // Attach event listeners
    this._attachEventListeners();

    // Add to DOM
    document.body.appendChild(this.backdrop);
    document.body.appendChild(this.element);

    // Trigger animation
    requestAnimationFrame(() => {
      this.backdrop.classList.add("exit-dialog-backdrop--visible");
      this.element.classList.add("exit-dialog--visible");
    });

    // Focus the minimize option by default (safer option)
    setTimeout(() => {
      const minimizeBtn = this.element.querySelector('[data-action="minimize"]');
      if (minimizeBtn) {
        minimizeBtn.focus();
      }
    }, 100);
  }

  _attachEventListeners() {
    // Option buttons
    const options = this.element.querySelectorAll(".exit-dialog__option");
    options.forEach((option) => {
      option.addEventListener("click", (e) => {
        const action = e.currentTarget.dataset.action;
        this._handleAction(action);
      });
    });

    // Cancel button
    const cancelBtn = this.element.querySelector('[data-action="cancel"]');
    if (cancelBtn) {
      cancelBtn.addEventListener("click", () => this._handleAction("cancel"));
    }

    // Backdrop click cancels
    this.backdrop.addEventListener("click", () => this._handleAction("cancel"));

    // Keyboard shortcuts
    this._keydownHandler = (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        this._handleAction("cancel");
      } else if (e.key === "Tab") {
        // Trap focus within dialog
        this._handleTabKey(e);
      }
    };
    document.addEventListener("keydown", this._keydownHandler);
  }

  _handleTabKey(e) {
    const focusableElements = this.element.querySelectorAll(
      'button:not([disabled]), [tabindex]:not([tabindex="-1"])'
    );

    if (focusableElements.length === 0) return;

    const firstElement = focusableElements[0];
    const lastElement = focusableElements[focusableElements.length - 1];

    if (e.shiftKey && document.activeElement === firstElement) {
      e.preventDefault();
      lastElement.focus();
    } else if (!e.shiftKey && document.activeElement === lastElement) {
      e.preventDefault();
      firstElement.focus();
    }
  }

  _handleAction(action) {
    if (this.resolver) {
      this.resolver(action);
      this.resolver = null;
    }
    this.destroy();
  }

  destroy() {
    // Remove keydown handler
    if (this._keydownHandler) {
      document.removeEventListener("keydown", this._keydownHandler);
      this._keydownHandler = null;
    }

    // Animate out
    if (this.backdrop) {
      this.backdrop.classList.remove("exit-dialog-backdrop--visible");
    }
    if (this.element) {
      this.element.classList.remove("exit-dialog--visible");
    }

    // Remove from DOM after animation
    setTimeout(() => {
      if (this.backdrop && this.backdrop.parentNode) {
        this.backdrop.remove();
      }
      if (this.element && this.element.parentNode) {
        this.element.remove();
      }

      // Clear static state
      ExitDialog.instance = null;
      ExitDialog.isVisible = false;
    }, 300);
  }
}

// Export for use in other modules
export { ExitDialog };

// Auto-initialize when loaded in Tauri context
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", () => ExitDialog.init());
} else {
  ExitDialog.init();
}
