/**
 * Security Tab Module - Lockscreen & 2FA settings
 * Extracted from settings_dialog.js
 */
import * as Utils from "../../core/utils.js";
import { enhanceAllSelects } from "../custom_select.js";

/**
 * Load and build Security tab content (async because we need to fetch status)
 */
export async function loadSecurityTab(dialog, content) {
  content.innerHTML =
    '<div class="settings-loading"><i class="icon-loader spin"></i> Loading security settings...</div>';

  try {
    // Fetch lockscreen status from API
    const response = await fetch("/api/lockscreen/status");
    let status = {
      enabled: false,
      has_password: false,
      password_type: "pin6",
      auto_lock_timeout_minutes: 0,
      lock_on_blur: false,
    };

    if (response.ok) {
      const data = await response.json();
      status = data.data || data;
    }

    // Also fetch TOTP status
    const totpResponse = await fetch("/api/auth/totp/status");
    let totpStatus = { enabled: false };
    if (totpResponse.ok) {
      const totpData = await totpResponse.json();
      totpStatus = totpData.data || totpData;
    }
    status.totp_enabled = totpStatus.enabled;

    content.innerHTML = buildSecurityTab(status);
    attachSecurityHandlers(dialog, content, status);
    enhanceAllSelects(content);
  } catch (error) {
    console.error("[Settings] Failed to load security status:", error);
    content.innerHTML = '<div class="settings-error">Failed to load security settings</div>';
  }
}

/**
 * Build Security tab HTML
 */
function buildSecurityTab(status) {
  const hasPassword = status.has_password;
  const isEnabled = status.enabled && hasPassword;
  const passwordType = status.password_type || "pin6";
  const autoLockSecs = status.auto_lock_timeout_secs || 0;
  const lockOnBlur = status.lock_on_blur || false;
  const totpEnabled = status.totp_enabled || false;

  // Password type display name
  const typeNames = {
    pin4: "4-Digit PIN",
    pin6: "6-Digit PIN",
    text: "Text Password",
  };
  const typeName = typeNames[passwordType] || "Not Set";

  return `
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-lock"></i>
          Dashboard Lockscreen
        </h3>
        <p class="settings-section-description">
          Protect your dashboard with a PIN or password. The lockscreen will appear when triggered, requiring authentication to continue.
        </p>

        <div class="settings-group">
          <!-- Enable/Disable Lockscreen -->
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Enable Lockscreen</label>
              <span class="settings-field-hint">Protect your dashboard with password authentication</span>
            </div>
            <div class="settings-field-control">
              <label class="toggle">
                <input type="checkbox" id="securityEnableLockscreen" ${isEnabled ? "checked" : ""} ${!hasPassword ? "disabled" : ""}>
                <span class="toggle-track"></span>
              </label>
            </div>
          </div>

          <!-- Password Status -->
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Password Status</label>
              <span class="settings-field-hint">
                ${hasPassword ? `Current: ${typeName}` : "No password set"}
              </span>
            </div>
            <div class="settings-field-control security-password-actions">
              ${
                hasPassword
                  ? `
                <button class="btn btn-secondary btn-sm" id="securityChangePasswordBtn">
                  <i class="icon-pencil"></i> Change
                </button>
                <button class="btn btn-warning btn-sm" id="securityRemovePasswordBtn">
                  <i class="icon-trash-2"></i> Remove
                </button>
              `
                  : `
                <button class="btn btn-primary btn-sm" id="securitySetPasswordBtn">
                  <i class="icon-key"></i> Set Password
                </button>
              `
              }
            </div>
          </div>

          <!-- Auto-Lock Timeout -->
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Auto-Lock After Inactivity</label>
              <span class="settings-field-hint">Automatically lock after period of no activity</span>
            </div>
            <div class="settings-field-control">
              <select id="securityAutoLockTimeout" class="settings-select" data-custom-select ${!hasPassword ? "disabled" : ""}>
                <option value="0" ${autoLockSecs === 0 ? "selected" : ""}>Never</option>
                <option value="60" ${autoLockSecs === 60 ? "selected" : ""}>1 minute</option>
                <option value="300" ${autoLockSecs === 300 ? "selected" : ""}>5 minutes</option>
                <option value="900" ${autoLockSecs === 900 ? "selected" : ""}>15 minutes</option>
                <option value="1800" ${autoLockSecs === 1800 ? "selected" : ""}>30 minutes</option>
                <option value="3600" ${autoLockSecs === 3600 ? "selected" : ""}>1 hour</option>
              </select>
            </div>
          </div>

          <!-- Lock on Blur -->
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Lock When Window Loses Focus</label>
              <span class="settings-field-hint">Automatically lock when you switch to another application</span>
            </div>
            <div class="settings-field-control">
              <label class="toggle">
                <input type="checkbox" id="securityLockOnBlur" ${lockOnBlur ? "checked" : ""} ${!hasPassword ? "disabled" : ""}>
                <span class="toggle-track"></span>
              </label>
            </div>
          </div>
        </div>
      </div>

      <!-- Lock Now Action -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-shield"></i>
          Quick Actions
        </h3>
        
        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Lock Dashboard Now</label>
              <span class="settings-field-hint">Immediately lock the dashboard</span>
            </div>
            <div class="settings-field-control">
              <button class="btn btn-primary btn-sm" id="securityLockNowBtn" ${!hasPassword || !isEnabled ? "disabled" : ""}>
                <i class="icon-lock"></i> Lock Now
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Two-Factor Authentication -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-shield-check"></i>
          Two-Factor Authentication
        </h3>
        <p class="settings-section-description">
          Add an extra layer of security using an authenticator app (Google Authenticator, Authy, etc.)
        </p>

        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label>2FA Status</label>
              <span class="settings-field-hint">
                ${totpEnabled ? "Two-factor authentication is enabled" : "Not configured"}
              </span>
            </div>
            <div class="settings-field-control">
              ${
                totpEnabled
                  ? `<button class="btn btn-warning btn-sm" id="securityDisable2FABtn" ${!hasPassword ? "disabled" : ""}>
                  <i class="icon-x"></i> Disable 2FA
                </button>`
                  : `<button class="btn btn-primary btn-sm" id="securityEnable2FABtn" ${!hasPassword ? "disabled" : ""}>
                  <i class="icon-shield-check"></i> Enable 2FA
                </button>`
              }
            </div>
          </div>
        </div>
      </div>

      <!-- Password Setup Modal Container -->
      <div id="securityPasswordModal" class="security-modal" style="display: none;">
        <div class="security-modal-backdrop"></div>
        <div class="security-modal-content">
          <div class="security-modal-header">
            <h3 id="securityModalTitle">Set Password</h3>
            <button class="security-modal-close" id="securityModalClose">
              <i class="icon-x"></i>
            </button>
          </div>
          <div class="security-modal-body" id="securityModalBody">
            <!-- Content injected dynamically -->
          </div>
        </div>
      </div>
    `;
}

/**
 * Attach handlers for Security tab
 */
export function attachSecurityHandlers(dialog, content, _status) {
  // Enable/disable toggle
  const enableToggle = content.querySelector("#securityEnableLockscreen");
  if (enableToggle) {
    enableToggle.addEventListener("change", async (e) => {
      await updateSecuritySetting("enabled", e.target.checked);
    });
  }

  // Auto-lock timeout
  const timeoutSelect = content.querySelector("#securityAutoLockTimeout");
  if (timeoutSelect) {
    timeoutSelect.addEventListener("change", async (e) => {
      await updateSecuritySetting("auto_lock_timeout_secs", parseInt(e.target.value, 10));
    });
  }

  // Lock on blur toggle
  const blurToggle = content.querySelector("#securityLockOnBlur");
  if (blurToggle) {
    blurToggle.addEventListener("change", async (e) => {
      await updateSecuritySetting("lock_on_blur", e.target.checked);
    });
  }

  // Set password button
  const setBtn = content.querySelector("#securitySetPasswordBtn");
  if (setBtn) {
    setBtn.addEventListener("click", () => showPasswordModal(dialog, "set", content));
  }

  // Change password button
  const changeBtn = content.querySelector("#securityChangePasswordBtn");
  if (changeBtn) {
    changeBtn.addEventListener("click", () => showPasswordModal(dialog, "change", content));
  }

  // Remove password button
  const removeBtn = content.querySelector("#securityRemovePasswordBtn");
  if (removeBtn) {
    removeBtn.addEventListener("click", () => removePassword(dialog, content));
  }

  // Lock now button
  const lockBtn = content.querySelector("#securityLockNowBtn");
  if (lockBtn) {
    lockBtn.addEventListener("click", () => {
      if (window.Lockscreen && window.Lockscreen.lockNow()) {
        dialog.close();
      } else {
        Utils.showToast("Cannot lock - lockscreen not ready", "error");
      }
    });
  }

  // Enable 2FA button
  const enable2FABtn = content.querySelector("#securityEnable2FABtn");
  if (enable2FABtn) {
    enable2FABtn.addEventListener("click", () => showTotpSetupModal(dialog, content));
  }

  // Disable 2FA button
  const disable2FABtn = content.querySelector("#securityDisable2FABtn");
  if (disable2FABtn) {
    disable2FABtn.addEventListener("click", () => disableTotp(dialog, content));
  }
}

/**
 * Update a security setting via API
 */
async function updateSecuritySetting(key, value) {
  try {
    const response = await fetch("/api/lockscreen/settings", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ [key]: value }),
    });

    if (response.ok) {
      Utils.showToast("Security setting updated", "success");
      // Update lockscreen controller if available
      if (window.Lockscreen) {
        window.Lockscreen.loadStatus();
      }
    } else {
      const data = await response.json();
      Utils.showToast(data.message || "Failed to update setting", "error");
    }
  } catch (error) {
    Utils.showToast("Failed to update setting: " + error.message, "error");
  }
}

/**
 * Show password setup/change modal
 */
function showPasswordModal(dialog, mode, content) {
  const modal = content.querySelector("#securityPasswordModal");
  const title = content.querySelector("#securityModalTitle");
  const body = content.querySelector("#securityModalBody");
  const closeBtn = content.querySelector("#securityModalClose");
  const backdrop = modal.querySelector(".security-modal-backdrop");

  if (!modal || !body) return;

  title.textContent = mode === "set" ? "Set Password" : "Change Password";

  body.innerHTML = `
      <div class="security-form">
        ${
          mode === "change"
            ? `
          <div class="security-form-group">
            <label>Current Password</label>
            <input type="password" id="securityCurrentPassword" class="settings-input" placeholder="Enter current password" />
          </div>
        `
            : ""
        }
        
        <div class="security-form-group">
          <label>Password Type</label>
          <select id="securityPasswordType" class="settings-select" data-custom-select>
            <option value="pin4">4-Digit PIN</option>
            <option value="pin6" selected>6-Digit PIN</option>
            <option value="text">Text Password</option>
          </select>
        </div>

        <div class="security-form-group">
          <label>New Password</label>
          <input type="password" id="securityNewPassword" class="settings-input" placeholder="Enter new password" />
        </div>

        <div class="security-form-group">
          <label>Confirm Password</label>
          <input type="password" id="securityConfirmPassword" class="settings-input" placeholder="Confirm password" />
        </div>

        <div class="security-form-actions">
          <button class="btn btn-secondary" id="securityCancelBtn">Cancel</button>
          <button class="btn btn-primary" id="securitySavePasswordBtn">
            <i class="icon-check"></i> ${mode === "set" ? "Set Password" : "Update Password"}
          </button>
        </div>
      </div>
    `;

  modal.style.display = "flex";
  enhanceAllSelects(body);

  // Close modal function with keyboard listener cleanup
  let handleKeydown;
  const closeModal = () => {
    if (handleKeydown) {
      document.removeEventListener("keydown", handleKeydown);
    }
    modal.style.display = "none";
  };

  // Keyboard handler for Escape
  handleKeydown = (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      closeModal();
    }
  };
  document.addEventListener("keydown", handleKeydown);

  closeBtn.onclick = closeModal;
  backdrop.onclick = closeModal;
  body.querySelector("#securityCancelBtn").onclick = closeModal;

  // Type change handler - validate input
  const typeSelect = body.querySelector("#securityPasswordType");
  const newPasswordInput = body.querySelector("#securityNewPassword");

  typeSelect.addEventListener("change", () => {
    const type = typeSelect.value;
    if (type === "pin4" || type === "pin6") {
      newPasswordInput.type = "password";
      newPasswordInput.inputMode = "numeric";
      newPasswordInput.pattern = type === "pin4" ? "[0-9]{4}" : "[0-9]{6}";
      newPasswordInput.placeholder = type === "pin4" ? "Enter 4-digit PIN" : "Enter 6-digit PIN";
    } else {
      newPasswordInput.type = "password";
      newPasswordInput.inputMode = "text";
      newPasswordInput.pattern = "";
      newPasswordInput.placeholder = "Enter password";
    }
  });

  // Save handler
  body.querySelector("#securitySavePasswordBtn").onclick = async () => {
    const passwordType = typeSelect.value;
    const newPassword = newPasswordInput.value;
    const confirmPassword = body.querySelector("#securityConfirmPassword").value;
    const currentPassword =
      mode === "change" ? body.querySelector("#securityCurrentPassword")?.value : null;

    // Validation
    if (!newPassword) {
      Utils.showToast("Please enter a password", "error");
      return;
    }

    if (newPassword !== confirmPassword) {
      Utils.showToast("Passwords do not match", "error");
      return;
    }

    // Validate PIN format
    if (passwordType === "pin4" && !/^\d{4}$/.test(newPassword)) {
      Utils.showToast("PIN must be exactly 4 digits", "error");
      return;
    }
    if (passwordType === "pin6" && !/^\d{6}$/.test(newPassword)) {
      Utils.showToast("PIN must be exactly 6 digits", "error");
      return;
    }
    if (passwordType === "text" && newPassword.length < 4) {
      Utils.showToast("Password must be at least 4 characters", "error");
      return;
    }

    try {
      const payload = {
        password_type: passwordType,
        new_password: newPassword,
      };
      if (currentPassword) {
        payload.current_password = currentPassword;
      }

      const response = await fetch("/api/lockscreen/set-password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });

      if (response.ok) {
        Utils.showToast("Password saved successfully", "success");
        closeModal();
        // Reload security tab
        await loadSecurityTab(dialog, content);
        // Update lockscreen controller
        if (window.Lockscreen) {
          window.Lockscreen.loadStatus();
        }
      } else {
        const data = await response.json();
        Utils.showToast(data.message || "Failed to save password", "error");
      }
    } catch (error) {
      Utils.showToast("Failed to save password: " + error.message, "error");
    }
  };
}

/**
 * Remove password - shows modal to confirm with current password
 */
function removePassword(dialog, content) {
  const modal = content.querySelector("#securityPasswordModal");
  const title = content.querySelector("#securityModalTitle");
  const body = content.querySelector("#securityModalBody");
  const closeBtn = content.querySelector("#securityModalClose");
  const backdrop = modal.querySelector(".security-modal-backdrop");

  if (!modal || !body) return;

  title.textContent = "Remove Password";

  body.innerHTML = `
      <div class="security-form">
        <p style="color: var(--text-secondary); margin-bottom: 16px;">
          Enter your current password to remove lockscreen protection.
        </p>
        
        <div class="security-form-group">
          <label>Current Password</label>
          <input type="password" id="securityCurrentPasswordRemove" class="settings-input" placeholder="Enter current password" autofocus />
        </div>

        <div class="security-form-actions">
          <button class="btn btn-secondary" id="securityCancelRemoveBtn">Cancel</button>
          <button class="btn btn-warning" id="securityConfirmRemoveBtn">
            <i class="icon-trash-2"></i> Remove Password
          </button>
        </div>
      </div>
    `;

  modal.style.display = "flex";

  // Focus the password input
  setTimeout(() => {
    const input = body.querySelector("#securityCurrentPasswordRemove");
    if (input) input.focus();
  }, 100);

  // Store reference for cleanup
  const passwordInput = body.querySelector("#securityCurrentPasswordRemove");

  // Keyboard handler for Enter and Escape
  const handleKeydown = (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      body.querySelector("#securityConfirmRemoveBtn").click();
    } else if (e.key === "Escape") {
      e.preventDefault();
      closeModal();
    }
  };
  passwordInput.addEventListener("keydown", handleKeydown);

  // Close handlers with cleanup
  const closeModal = () => {
    passwordInput.removeEventListener("keydown", handleKeydown);
    modal.style.display = "none";
  };

  closeBtn.onclick = closeModal;
  backdrop.onclick = closeModal;
  body.querySelector("#securityCancelRemoveBtn").onclick = closeModal;

  // Confirm handler
  body.querySelector("#securityConfirmRemoveBtn").onclick = async () => {
    const currentPassword = body.querySelector("#securityCurrentPasswordRemove").value;

    if (!currentPassword) {
      Utils.showToast("Please enter your current password", "error");
      return;
    }

    try {
      const response = await fetch("/api/lockscreen/clear-password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ current_password: currentPassword }),
      });

      if (response.ok) {
        Utils.showToast("Password removed", "success");
        closeModal();
        // Reload security tab
        await loadSecurityTab(dialog, content);
        // Update lockscreen controller
        if (window.Lockscreen) {
          window.Lockscreen.loadStatus();
        }
      } else {
        const data = await response.json();
        Utils.showToast(data.message || "Failed to remove password", "error");
      }
    } catch (error) {
      Utils.showToast("Failed to remove password: " + error.message, "error");
    }
  };
}

/**
 * Show TOTP setup modal with QR code and verification
 */
async function showTotpSetupModal(dialog, content) {
  const modal = content.querySelector("#securityPasswordModal");
  const title = content.querySelector("#securityModalTitle");
  const body = content.querySelector("#securityModalBody");
  const closeBtn = content.querySelector("#securityModalClose");
  const backdrop = modal.querySelector(".security-modal-backdrop");

  if (!modal || !body) return;

  title.textContent = "Enable Two-Factor Authentication";
  body.innerHTML = `
      <div class="totp-setup-step" id="totpStep1">
        <p style="color: var(--text-secondary); margin-bottom: 1rem;">Enter your password to continue:</p>
        <div class="security-form-group">
          <input type="password" id="totpSetupPassword" class="settings-input" placeholder="Enter password" autocomplete="current-password">
        </div>
        <div class="totp-setup-actions" style="display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem;">
          <button class="btn btn-secondary btn-sm" id="totpCancelBtn">Cancel</button>
          <button class="btn btn-primary btn-sm" id="totpContinueBtn">Continue</button>
        </div>
      </div>
      <div class="totp-setup-step" id="totpStep2" style="display: none;">
        <div id="totpQrContainer" style="text-align: center; margin: 1rem 0;"></div>
        <div class="totp-manual-entry" style="margin: 1rem 0;">
          <label style="font-size: 0.75rem; color: var(--text-secondary);">Manual entry code:</label>
          <code id="totpSecretCode" style="display: block; padding: 0.5rem; background: var(--bg-tertiary); border-radius: 4px; margin-top: 0.25rem; word-break: break-all; font-family: monospace;"></code>
        </div>
        <p style="margin: 1rem 0; color: var(--text-secondary);">Enter the 6-digit code from your authenticator app:</p>
        <div class="security-form-group">
          <input type="text" id="totpVerifyCode" class="settings-input" placeholder="000000" maxlength="6" pattern="[0-9]{6}" style="text-align: center; font-size: 1.25rem; letter-spacing: 0.25em;">
        </div>
        <div class="totp-setup-actions" style="display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem;">
          <button class="btn btn-secondary btn-sm" id="totpBackBtn">Back</button>
          <button class="btn btn-primary btn-sm" id="totpVerifyBtn">Verify & Enable</button>
        </div>
      </div>
    `;

  modal.style.display = "flex";

  let currentSecret = "";

  // Close handlers
  const closeModal = () => {
    modal.style.display = "none";
  };

  closeBtn.onclick = closeModal;
  backdrop.onclick = closeModal;

  // Cancel button
  body.querySelector("#totpCancelBtn")?.addEventListener("click", closeModal);

  // Continue to step 2
  body.querySelector("#totpContinueBtn")?.addEventListener("click", async () => {
    const password = body.querySelector("#totpSetupPassword")?.value;
    if (!password) {
      Utils.showToast("Please enter your password", "error");
      return;
    }

    try {
      const response = await fetch("/api/auth/totp/setup", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ password }),
      });

      const data = await response.json();
      if (!response.ok) {
        Utils.showToast(data.error || data.message || "Failed to setup 2FA", "error");
        return;
      }

      const result = data.data || data;
      currentSecret = result.secret;

      // Show QR code
      const qrContainer = body.querySelector("#totpQrContainer");
      qrContainer.innerHTML = `<img src="${result.qr_code}" alt="TOTP QR Code" style="max-width: 200px; height: auto; border-radius: 8px;">`;

      // Show manual code
      body.querySelector("#totpSecretCode").textContent = result.secret;

      // Switch to step 2
      body.querySelector("#totpStep1").style.display = "none";
      body.querySelector("#totpStep2").style.display = "block";
    } catch {
      Utils.showToast("Failed to setup 2FA", "error");
    }
  });

  // Back button
  body.querySelector("#totpBackBtn")?.addEventListener("click", () => {
    body.querySelector("#totpStep1").style.display = "block";
    body.querySelector("#totpStep2").style.display = "none";
  });

  // Verify button
  body.querySelector("#totpVerifyBtn")?.addEventListener("click", async () => {
    const code = body.querySelector("#totpVerifyCode")?.value;
    if (!code || code.length !== 6) {
      Utils.showToast("Please enter a 6-digit code", "error");
      return;
    }

    try {
      const response = await fetch("/api/auth/totp/verify-setup", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ secret: currentSecret, code }),
      });

      const data = await response.json();
      if (!response.ok) {
        Utils.showToast(data.error || data.message || "Invalid code", "error");
        return;
      }

      Utils.showToast("Two-factor authentication enabled!", "success");
      closeModal();
      // Reload security tab
      await loadSecurityTab(dialog, content);
    } catch {
      Utils.showToast("Failed to verify code", "error");
    }
  });
}

/**
 * Disable TOTP 2FA with password confirmation
 */
async function disableTotp(dialog, content) {
  const modal = content.querySelector("#securityPasswordModal");
  const title = content.querySelector("#securityModalTitle");
  const body = content.querySelector("#securityModalBody");
  const closeBtn = content.querySelector("#securityModalClose");
  const backdrop = modal.querySelector(".security-modal-backdrop");

  if (!modal || !body) return;

  title.textContent = "Disable Two-Factor Authentication";
  body.innerHTML = `
      <div class="security-form">
        <p style="color: var(--text-secondary); margin-bottom: 1rem;">Enter your password to disable 2FA:</p>
        <div class="security-form-group">
          <input type="password" id="totpDisablePassword" class="settings-input" placeholder="Enter password" autocomplete="current-password">
        </div>
        <div class="totp-setup-actions" style="display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem;">
          <button class="btn btn-secondary btn-sm" id="totpDisableCancelBtn">Cancel</button>
          <button class="btn btn-warning btn-sm" id="totpDisableConfirmBtn">Disable 2FA</button>
        </div>
      </div>
    `;

  modal.style.display = "flex";

  // Close handlers
  const closeModal = () => {
    modal.style.display = "none";
  };

  closeBtn.onclick = closeModal;
  backdrop.onclick = closeModal;

  body.querySelector("#totpDisableCancelBtn")?.addEventListener("click", closeModal);

  body.querySelector("#totpDisableConfirmBtn")?.addEventListener("click", async () => {
    const password = body.querySelector("#totpDisablePassword")?.value;
    if (!password) {
      Utils.showToast("Please enter your password", "error");
      return;
    }

    try {
      const response = await fetch("/api/auth/totp/disable", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ password }),
      });

      const data = await response.json();
      if (!response.ok) {
        Utils.showToast(data.error || data.message || "Failed to disable 2FA", "error");
        return;
      }

      Utils.showToast("Two-factor authentication disabled", "success");
      closeModal();
      await loadSecurityTab(dialog, content);
    } catch {
      Utils.showToast("Failed to disable 2FA", "error");
    }
  });
}
