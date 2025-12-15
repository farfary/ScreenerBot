// Lockscreen Controller
// Full-screen security overlay with PIN/password unlock

/**
 * LockscreenController - Manages dashboard locking/unlocking
 *
 * Features:
 * - PIN entry (4 or 6 digit) with visual keypad
 * - Text password entry with visibility toggle
 * - Inactivity timeout auto-lock
 * - Lock on blur (window loses focus)
 * - Error animation on wrong password
 * - Keyboard support for PIN entry
 */
class LockscreenController {
  constructor() {
    // DOM elements
    this.lockscreenEl = null;
    this.pinContainer = null;
    this.passwordContainer = null;
    this.pinDigitsEl = null;
    this.passwordInput = null;
    this.errorEl = null;
    this.versionEl = null;

    // State
    this.isLocked = false;
    this.isInitialized = false;
    this.passwordType = "pin6"; // "pin4", "pin6", "text"
    this.pinLength = 6;
    this.enteredPin = "";
    this.isVerifying = false;

    // Inactivity tracking
    this.inactivityTimeout = null;
    this.inactivityMs = 0; // 0 = disabled
    this.lockOnBlur = false;
    this.isEnabled = false;
    this.hasPassword = false;

    // Activity events to track
    this.activityEvents = ["mousedown", "mousemove", "keydown", "touchstart", "scroll", "wheel"];

    // Bound handlers for cleanup
    this._boundActivityHandler = this._handleActivity.bind(this);
    this._boundBlurHandler = this._handleWindowBlur.bind(this);
    this._boundKeyHandler = this._handleKeyboard.bind(this);
  }

  /**
   * Initialize the lockscreen controller
   */
  async init() {
    if (this.isInitialized) return;

    // Get DOM elements
    this.lockscreenEl = document.getElementById("lockscreen");
    this.pinContainer = document.getElementById("lockscreenPinContainer");
    this.passwordContainer = document.getElementById("lockscreenPasswordContainer");
    this.pinDigitsEl = document.getElementById("lockscreenPinDigits");
    this.passwordInput = document.getElementById("lockscreenPasswordInput");
    this.errorEl = document.getElementById("lockscreenError");
    this.versionEl = document.getElementById("lockscreenVersion");

    if (!this.lockscreenEl) {
      console.warn("[Lockscreen] Lockscreen element not found");
      return;
    }

    // Load version
    await this._loadVersion();

    // Load lockscreen status from API
    await this.loadStatus();

    // Bind event handlers
    this._bindEvents();

    // Start inactivity tracking if enabled
    if (this.isEnabled && this.inactivityMs > 0) {
      this._startInactivityTracking();
    }

    // Start blur tracking if enabled
    if (this.isEnabled && this.lockOnBlur) {
      this._startBlurTracking();
    }

    this.isInitialized = true;
    console.log("[Lockscreen] Initialized");
  }

  /**
   * Load lockscreen status from API
   */
  async loadStatus() {
    try {
      const response = await fetch("/api/lockscreen/status");
      if (!response.ok) {
        console.warn("[Lockscreen] Failed to load status:", response.status);
        return;
      }

      const data = await response.json();
      const status = data.data || data;

      this.isEnabled = status.enabled || false;
      this.hasPassword = status.has_password || false;
      this.passwordType = status.password_type || "pin6";
      this.inactivityMs = (status.auto_lock_timeout_secs || 0) * 1000;
      this.lockOnBlur = status.lock_on_blur || false;

      // Set PIN length based on type
      if (this.passwordType === "pin4") {
        this.pinLength = 4;
      } else if (this.passwordType === "pin6") {
        this.pinLength = 6;
      }

      // Update inactivity tracking
      if (this.isEnabled && this.inactivityMs > 0) {
        this._startInactivityTracking();
      } else {
        this._stopInactivityTracking();
      }

      // Update blur tracking
      if (this.isEnabled && this.lockOnBlur) {
        this._startBlurTracking();
      } else {
        this._stopBlurTracking();
      }
    } catch (error) {
      console.error("[Lockscreen] Failed to load status:", error);
    }
  }

  /**
   * Load version info
   */
  async _loadVersion() {
    try {
      const response = await fetch("/api/version");
      if (response.ok) {
        const data = await response.json();
        if (this.versionEl && data.version) {
          this.versionEl.textContent = `v${data.version}`;
        }
      }
    } catch (err) {
      console.warn("[Lockscreen] Failed to load version:", err);
    }
  }

  /**
   * Bind event handlers
   */
  _bindEvents() {
    // Keypad clicks
    const keypadKeys = this.lockscreenEl.querySelectorAll(".lockscreen-key[data-key]");
    keypadKeys.forEach((key) => {
      key.addEventListener("click", (e) => {
        e.preventDefault();
        const keyValue = key.dataset.key;
        if (keyValue === "backspace") {
          this._handlePinBackspace();
        } else {
          this._handlePinDigit(keyValue);
        }
      });
    });

    // Password toggle visibility
    const passwordToggle = document.getElementById("lockscreenPasswordToggle");
    if (passwordToggle) {
      passwordToggle.addEventListener("click", () => this._togglePasswordVisibility());
    }

    // Password submit button
    const passwordSubmit = document.getElementById("lockscreenPasswordSubmit");
    if (passwordSubmit) {
      passwordSubmit.addEventListener("click", () => this._submitPassword());
    }

    // Password input enter key
    if (this.passwordInput) {
      this.passwordInput.addEventListener("keydown", (e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          this._submitPassword();
        }
      });
    }

    // Global keyboard handler for PIN input
    document.addEventListener("keydown", this._boundKeyHandler);
  }

  /**
   * Handle keyboard input when lockscreen is visible
   */
  _handleKeyboard(e) {
    if (!this.isLocked) return;

    // Only handle for PIN mode
    if (this.passwordType !== "pin4" && this.passwordType !== "pin6") return;

    // Ignore if focus is on password input
    if (document.activeElement === this.passwordInput) return;

    if (e.key >= "0" && e.key <= "9") {
      e.preventDefault();
      this._handlePinDigit(e.key);
    } else if (e.key === "Backspace") {
      e.preventDefault();
      this._handlePinBackspace();
    }
  }

  /**
   * Handle PIN digit entry
   */
  _handlePinDigit(digit) {
    if (this.isVerifying) return;
    if (this.enteredPin.length >= this.pinLength) return;

    this.enteredPin += digit;
    this._updatePinDisplay();

    // Auto-submit when PIN is complete
    if (this.enteredPin.length === this.pinLength) {
      this._verifyPin();
    }
  }

  /**
   * Handle PIN backspace
   */
  _handlePinBackspace() {
    if (this.isVerifying) return;
    if (this.enteredPin.length === 0) return;

    this.enteredPin = this.enteredPin.slice(0, -1);
    this._updatePinDisplay();
    this._hideError();
  }

  /**
   * Update PIN digit display
   */
  _updatePinDisplay() {
    if (!this.pinDigitsEl) return;

    const digits = this.pinDigitsEl.querySelectorAll(".lockscreen-pin-digit");
    digits.forEach((digit, index) => {
      digit.classList.remove("filled", "current");
      if (index < this.enteredPin.length) {
        digit.classList.add("filled");
      } else if (index === this.enteredPin.length) {
        digit.classList.add("current");
      }
    });
  }

  /**
   * Verify PIN
   */
  async _verifyPin() {
    await this.verify(this.enteredPin);
  }

  /**
   * Submit text password
   */
  async _submitPassword() {
    if (!this.passwordInput) return;
    const password = this.passwordInput.value;
    if (!password) return;

    await this.verify(password);
  }

  /**
   * Toggle password visibility
   */
  _togglePasswordVisibility() {
    if (!this.passwordInput) return;

    const icon = document.getElementById("lockscreenPasswordIcon");
    if (this.passwordInput.type === "password") {
      this.passwordInput.type = "text";
      if (icon) icon.className = "icon-eye-off";
    } else {
      this.passwordInput.type = "password";
      if (icon) icon.className = "icon-eye";
    }
  }

  /**
   * Lock the dashboard
   */
  lock() {
    if (this.isLocked) return;
    if (!this.isEnabled || !this.hasPassword) return;

    this.isLocked = true;
    this.enteredPin = "";

    // Setup appropriate input type
    this._setupInputType();

    // Show lockscreen
    if (this.lockscreenEl) {
      this.lockscreenEl.setAttribute("aria-hidden", "false");
      this.lockscreenEl.classList.add("fade-in");
      this.lockscreenEl.classList.remove("fade-out", "unlocking");
    }

    // Focus appropriate input
    requestAnimationFrame(() => {
      if (this.passwordType === "text" && this.passwordInput) {
        this.passwordInput.focus();
      }
    });

    console.log("[Lockscreen] Dashboard locked");
  }

  /**
   * Unlock the dashboard
   */
  unlock() {
    if (!this.isLocked) return;

    this.isLocked = false;

    // Play unlock animation
    if (this.lockscreenEl) {
      this.lockscreenEl.classList.add("unlocking");
      this.lockscreenEl.classList.remove("fade-in");

      setTimeout(() => {
        this.lockscreenEl.setAttribute("aria-hidden", "true");
        this.lockscreenEl.classList.remove("unlocking");
        this._resetInputs();
      }, 400);
    }

    // Restart inactivity tracking
    if (this.inactivityMs > 0) {
      this._resetInactivityTimer();
    }

    console.log("[Lockscreen] Dashboard unlocked");
  }

  /**
   * Verify password/PIN with backend
   */
  async verify(password) {
    if (this.isVerifying) return;
    if (!password) return;

    this.isVerifying = true;
    this._hideError();
    this._setLoadingState(true);

    try {
      const response = await fetch("/api/lockscreen/verify", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ password }),
      });

      const data = await response.json();

      if (response.ok && data.valid) {
        this.unlock();
      } else {
        this._showError(data.message || "Incorrect password");
        this._showInputError();
        this._resetInputs();
      }
    } catch (error) {
      console.error("[Lockscreen] Verification failed:", error);
      this._showError("Verification failed");
      this._showInputError();
      this._resetInputs();
    } finally {
      this.isVerifying = false;
      this._setLoadingState(false);
    }
  }

  /**
   * Set loading state on submit button
   */
  _setLoadingState(loading) {
    const submitBtn = document.getElementById("lockscreenPasswordSubmit");
    if (submitBtn) {
      submitBtn.disabled = loading;
      submitBtn.classList.toggle("verifying", loading);
    }
  }

  /**
   * Show error styling on inputs
   */
  _showInputError() {
    // Add error class to inputs
    if (this.passwordType === "pin4" || this.passwordType === "pin6") {
      const digits = this.pinDigitsEl?.querySelectorAll(".lockscreen-pin-digit");
      digits?.forEach((d) => d.classList.add("error"));
    } else if (this.passwordInput) {
      this.passwordInput.classList.add("error");
    }

    // Remove error class after timeout
    setTimeout(() => {
      this._clearInputError();
    }, 2000);
  }

  /**
   * Clear error styling from inputs
   */
  _clearInputError() {
    const digits = this.pinDigitsEl?.querySelectorAll(".lockscreen-pin-digit");
    digits?.forEach((d) => d.classList.remove("error"));
    this.passwordInput?.classList.remove("error");
  }

  /**
   * Setup input type (PIN or password)
   */
  _setupInputType() {
    if (!this.pinContainer || !this.passwordContainer) return;

    if (this.passwordType === "pin4" || this.passwordType === "pin6") {
      this.pinContainer.style.display = "flex";
      this.passwordContainer.style.display = "none";
      this._buildPinDigits();
    } else {
      this.pinContainer.style.display = "none";
      this.passwordContainer.style.display = "flex";
    }
  }

  /**
   * Build PIN digit boxes
   */
  _buildPinDigits() {
    if (!this.pinDigitsEl) return;

    let html = "";
    for (let i = 0; i < this.pinLength; i++) {
      const currentClass = i === 0 ? " current" : "";
      html += `<div class="lockscreen-pin-digit${currentClass}"></div>`;
    }
    this.pinDigitsEl.innerHTML = html;
  }

  /**
   * Reset all inputs
   */
  _resetInputs() {
    this.enteredPin = "";
    if (this.pinDigitsEl) {
      this._buildPinDigits();
    }
    if (this.passwordInput) {
      this.passwordInput.value = "";
      this.passwordInput.type = "password";
    }
    const icon = document.getElementById("lockscreenPasswordIcon");
    if (icon) icon.className = "icon-eye";
  }

  /**
   * Show error message with shake animation
   */
  _showError(message) {
    if (!this.errorEl) return;

    const textEl = document.getElementById("lockscreenErrorText");
    if (textEl) textEl.textContent = message;

    this.errorEl.classList.remove("visible");

    // Force reflow for animation restart
    void this.errorEl.offsetWidth;

    this.errorEl.classList.add("visible");

    // Also shake the PIN digits or password field
    if (this.passwordType === "pin4" || this.passwordType === "pin6") {
      if (this.pinDigitsEl) {
        this.pinDigitsEl.style.animation = "none";
        void this.pinDigitsEl.offsetWidth;
        this.pinDigitsEl.style.animation = "lockscreenShake 0.5s ease-out";
      }
    } else {
      if (this.passwordInput) {
        this.passwordInput.style.animation = "none";
        void this.passwordInput.offsetWidth;
        this.passwordInput.style.animation = "lockscreenShake 0.5s ease-out";
      }
    }
  }

  /**
   * Hide error message
   */
  _hideError() {
    if (!this.errorEl) return;
    this.errorEl.classList.remove("visible");
  }

  // =========================================================================
  // INACTIVITY TRACKING
  // =========================================================================

  /**
   * Start tracking user activity
   */
  _startInactivityTracking() {
    this._stopInactivityTracking();

    this.activityEvents.forEach((event) => {
      document.addEventListener(event, this._boundActivityHandler, { passive: true });
    });

    this._resetInactivityTimer();
  }

  /**
   * Stop tracking user activity
   */
  _stopInactivityTracking() {
    this.activityEvents.forEach((event) => {
      document.removeEventListener(event, this._boundActivityHandler);
    });

    if (this.inactivityTimeout) {
      clearTimeout(this.inactivityTimeout);
      this.inactivityTimeout = null;
    }
  }

  /**
   * Handle user activity
   */
  _handleActivity() {
    if (this.isLocked) return;
    this._resetInactivityTimer();
  }

  /**
   * Reset inactivity timer
   */
  _resetInactivityTimer() {
    if (this.inactivityTimeout) {
      clearTimeout(this.inactivityTimeout);
    }

    if (this.inactivityMs > 0 && !this.isLocked) {
      this.inactivityTimeout = setTimeout(() => {
        if (this.isEnabled && this.hasPassword && !this.isLocked) {
          console.log("[Lockscreen] Inactivity timeout - locking");
          this.lock();
        }
      }, this.inactivityMs);
    }
  }

  // =========================================================================
  // BLUR TRACKING (Lock on window blur)
  // =========================================================================

  /**
   * Start tracking window blur
   */
  _startBlurTracking() {
    this._stopBlurTracking();
    window.addEventListener("blur", this._boundBlurHandler);
  }

  /**
   * Stop tracking window blur
   */
  _stopBlurTracking() {
    window.removeEventListener("blur", this._boundBlurHandler);
  }

  /**
   * Handle window blur (lost focus)
   */
  _handleWindowBlur() {
    if (this.isLocked) return;
    if (!this.isEnabled || !this.hasPassword || !this.lockOnBlur) return;

    console.log("[Lockscreen] Window lost focus - locking");
    this.lock();
  }

  // =========================================================================
  // PUBLIC API
  // =========================================================================

  /**
   * Manually trigger lock
   */
  lockNow() {
    if (!this.isEnabled || !this.hasPassword) {
      console.warn("[Lockscreen] Cannot lock - not enabled or no password set");
      return false;
    }
    this.lock();
    return true;
  }

  /**
   * Check if lockscreen is currently locked
   */
  isCurrentlyLocked() {
    return this.isLocked;
  }

  /**
   * Check if lockscreen is enabled
   */
  isLockscreenEnabled() {
    return this.isEnabled && this.hasPassword;
  }

  /**
   * Update settings (called from settings dialog)
   */
  updateSettings(settings) {
    this.isEnabled = settings.enabled || false;
    this.hasPassword = settings.has_password || false;
    this.passwordType = settings.password_type || "pin6";
    this.inactivityMs = (settings.auto_lock_timeout_secs || 0) * 1000;
    this.lockOnBlur = settings.lock_on_blur || false;

    if (this.passwordType === "pin4") {
      this.pinLength = 4;
    } else if (this.passwordType === "pin6") {
      this.pinLength = 6;
    }

    // Update tracking
    if (this.isEnabled && this.inactivityMs > 0) {
      this._startInactivityTracking();
    } else {
      this._stopInactivityTracking();
    }

    if (this.isEnabled && this.lockOnBlur) {
      this._startBlurTracking();
    } else {
      this._stopBlurTracking();
    }
  }

  /**
   * Cleanup controller
   */
  dispose() {
    this._stopInactivityTracking();
    this._stopBlurTracking();
    document.removeEventListener("keydown", this._boundKeyHandler);
    this.isInitialized = false;
  }
}

// Create and export global instance
const Lockscreen = new LockscreenController();

// Initialize on DOM ready
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", () => Lockscreen.init());
} else {
  // DOM already loaded (e.g., script at end of body)
  Lockscreen.init();
}

// Export for use in other modules
window.Lockscreen = Lockscreen;
