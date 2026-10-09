// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Security Tab Module - Lockscreen & 2FA settings
 * Extracted from settings_dialog.js
 */
import * as Utils from "../../core/utils.js";
import { enhanceAllSelects } from "../custom_select.js";
import { apiErrorMessage } from "../../core/request_manager.js";

// Ids are the stored lockscreen `password_type` values.
const PASSWORD_TYPE_LABELS = Object.freeze({
  pin4: "settings-security-type-pin4",
  pin6: "settings-security-type-pin6",
  text: "settings-security-type-text",
});

/**
 * Load and build Security tab content (async because we need to fetch status)
 */
export async function loadSecurityTab(dialog, content) {
  content.innerHTML =
    '<div class="settings-loading"><i class="icon-loader spin"></i> <span data-l10n-id="settings-security-loading"></span></div>';
  I18n.localizeTree(content);

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
    I18n.localizeTree(content);
    attachSecurityHandlers(dialog, content, status);
    enhanceAllSelects(content);
  } catch (error) {
    console.error("[Settings] Failed to load security status:", error);
    content.innerHTML =
      '<div class="settings-error" data-l10n-id="settings-security-load-failed"></div>';
    I18n.localizeTree(content);
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
  // A disabled action names what it waits for in its hint.
  const lockNowHint = !hasPassword
    ? "settings-security-needs-password"
    : !isEnabled
      ? "settings-security-needs-lockscreen"
      : "settings-security-lock-now-hint";

  // Password type display name
  const typeName = Object.hasOwn(PASSWORD_TYPE_LABELS, passwordType)
    ? I18n.label(PASSWORD_TYPE_LABELS, passwordType)
    : I18n.t("settings-security-type-unset");

  return `
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-lock"></i>
          <span data-l10n-id="settings-security-lockscreen-title"></span>
        </h3>
        <p class="settings-section-description" data-l10n-id="settings-security-lockscreen-description"></p>

        <div class="settings-group">
          <!-- Enable/Disable Lockscreen -->
          <div class="settings-field">
            <div class="settings-field-info">
              <label data-l10n-id="settings-security-enable-label"></label>
              <span class="settings-field-hint" data-l10n-id="settings-security-enable-hint"></span>
            </div>
            <div class="settings-field-control">
              <label class="toggle" data-level="root">
                <input type="checkbox" id="securityEnableLockscreen" ${isEnabled ? "checked" : ""} ${!hasPassword ? "disabled" : ""}>
                <span class="toggle-track"></span>
              </label>
            </div>
          </div>

          <!-- Password Status -->
          <div class="settings-field">
            <div class="settings-field-info">
              <label data-l10n-id="settings-security-password-status-label"></label>
              <span class="settings-field-hint">
                ${Utils.escapeHtml(hasPassword ? I18n.t("settings-security-password-current", { type: typeName }) : I18n.t("settings-security-password-none"))}
              </span>
            </div>
            <div class="settings-field-control security-password-actions">
              ${
                hasPassword
                  ? `
                <button class="btn btn-secondary btn-sm" id="securityChangePasswordBtn">
                  <i class="icon-pencil"></i> <span data-l10n-id="settings-security-change"></span>
                </button>
                <button class="btn btn-warning btn-sm" id="securityRemovePasswordBtn">
                  <i class="icon-trash-2"></i> <span data-l10n-id="settings-security-remove"></span>
                </button>
              `
                  : `
                <button class="btn btn-primary btn-sm" id="securitySetPasswordBtn">
                  <i class="icon-key"></i> <span data-l10n-id="settings-security-set-password"></span>
                </button>
              `
              }
            </div>
          </div>

          <!-- Auto-Lock Timeout -->
          <div class="settings-field">
            <div class="settings-field-info">
              <label data-l10n-id="settings-security-auto-lock-label"></label>
              <span class="settings-field-hint" data-l10n-id="settings-security-auto-lock-hint"></span>
            </div>
            <div class="settings-field-control">
              <select id="securityAutoLockTimeout" class="settings-select" data-custom-select ${!hasPassword ? "disabled" : ""}>
                <option value="0" ${autoLockSecs === 0 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-security-auto-lock-never"))}</option>
                <option value="60" ${autoLockSecs === 60 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-duration-minutes", { count: 1 }))}</option>
                <option value="300" ${autoLockSecs === 300 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-duration-minutes", { count: 5 }))}</option>
                <option value="900" ${autoLockSecs === 900 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-duration-minutes", { count: 15 }))}</option>
                <option value="1800" ${autoLockSecs === 1800 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-duration-minutes", { count: 30 }))}</option>
                <option value="3600" ${autoLockSecs === 3600 ? "selected" : ""}>${Utils.escapeHtml(I18n.t("settings-duration-hours", { count: 1 }))}</option>
              </select>
            </div>
          </div>

          <!-- Lock on Blur -->
          <div class="settings-field">
            <div class="settings-field-info">
              <label data-l10n-id="settings-security-lock-blur-label"></label>
              <span class="settings-field-hint" data-l10n-id="settings-security-lock-blur-hint"></span>
            </div>
            <div class="settings-field-control">
              <label class="toggle" data-level="item">
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
          <span data-l10n-id="settings-security-quick-actions-title"></span>
        </h3>

        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label data-l10n-id="settings-security-lock-now-label"></label>
              <span class="settings-field-hint" data-l10n-id="${lockNowHint}"></span>
            </div>
            <div class="settings-field-control">
              <button class="btn btn-primary btn-sm" id="securityLockNowBtn" ${!hasPassword || !isEnabled ? "disabled" : ""}>
                <i class="icon-lock"></i> <span data-l10n-id="settings-security-lock-now"></span>
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Two-Factor Authentication -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-shield-check"></i>
          <span data-l10n-id="settings-security-2fa-title"></span>
        </h3>
        <p class="settings-section-description" data-l10n-id="settings-security-2fa-description"></p>

        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label data-l10n-id="settings-security-2fa-status-label"></label>
              <span class="settings-field-hint">
                ${Utils.escapeHtml(totpEnabled ? I18n.t("settings-security-2fa-status-enabled") : I18n.t("settings-security-2fa-status-none"))}
              </span>
              ${hasPassword ? "" : '<span class="settings-field-hint" data-l10n-id="settings-security-needs-password"></span>'}
            </div>
            <div class="settings-field-control">
              ${
                totpEnabled
                  ? `<button class="btn btn-warning btn-sm" id="securityDisable2FABtn" ${!hasPassword ? "disabled" : ""}>
                  <i class="icon-x"></i> <span data-l10n-id="settings-security-2fa-disable"></span>
                </button>`
                  : `<button class="btn btn-primary btn-sm" id="securityEnable2FABtn" ${!hasPassword ? "disabled" : ""}>
                  <i class="icon-shield-check"></i> <span data-l10n-id="settings-security-2fa-enable"></span>
                </button>`
              }
            </div>
          </div>
        </div>
      </div>

      <!-- Password Setup Modal Container -->
      <div id="securityPasswordModal" class="modal-overlay" hidden>
        <div class="modal-dialog modal-sm">
          <div class="modal-header">
            <h3 id="securityModalTitle" data-l10n-id="settings-security-password-set-title"></h3>
            <button class="modal-close" id="securityModalClose" data-l10n-id="settings-security-modal-close">
              <i class="icon-x"></i>
            </button>
          </div>
          <div class="modal-body" id="securityModalBody">
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
        Utils.showToast(I18n.t("settings-security-lock-not-ready"), "error");
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
  // A saved setting raises no toast — the control the user moved already shows
  // the new value. Only a rejected save gets one, keyed so a run of failures
  // leaves a single notice instead of one per control.
  const reportFailure = (message) =>
    Utils.showToast({
      key: "security-setting",
      type: "error",
      title: I18n.t("settings-security-setting-save-failed"),
      message: message || null,
    });

  try {
    const response = await fetch("/api/lockscreen/settings", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ [key]: value }),
    });

    if (response.ok) {
      // Update lockscreen controller if available
      if (window.Lockscreen) {
        window.Lockscreen.loadStatus();
      }
      return;
    }

    const data = await response.json().catch(() => null);
    reportFailure(apiErrorMessage(data, null));
  } catch (error) {
    reportFailure(error.message);
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

  if (!modal || !body) return;

  title.textContent =
    mode === "set"
      ? I18n.t("settings-security-password-set-title")
      : I18n.t("settings-security-password-change-title");

  body.innerHTML = `
      <div class="security-form">
        ${
          mode === "change"
            ? `
          <div class="security-form-group">
            <label data-l10n-id="settings-security-password-current-label"></label>
            <input type="password" id="securityCurrentPassword" class="settings-input" data-l10n-id="settings-security-password-current-input" />
          </div>
        `
            : ""
        }
        
        <div class="security-form-group">
          <label data-l10n-id="settings-security-password-type-label"></label>
          <select id="securityPasswordType" class="settings-select" data-custom-select>
            <option value="pin4">${Utils.escapeHtml(I18n.label(PASSWORD_TYPE_LABELS, "pin4"))}</option>
            <option value="pin6" selected>${Utils.escapeHtml(I18n.label(PASSWORD_TYPE_LABELS, "pin6"))}</option>
            <option value="text">${Utils.escapeHtml(I18n.label(PASSWORD_TYPE_LABELS, "text"))}</option>
          </select>
        </div>

        <div class="security-form-group">
          <label data-l10n-id="settings-security-password-new-label"></label>
          <input type="password" id="securityNewPassword" class="settings-input" data-l10n-id="settings-security-password-new-input" />
        </div>

        <div class="security-form-group">
          <label data-l10n-id="settings-security-password-confirm-label"></label>
          <input type="password" id="securityConfirmPassword" class="settings-input" data-l10n-id="settings-security-password-confirm-input" />
        </div>

        <div class="security-form-actions">
          <button class="btn btn-secondary" id="securityCancelBtn" data-l10n-id="common-action-cancel"></button>
          <button class="btn btn-primary" id="securitySavePasswordBtn">
            <i class="icon-check"></i> ${Utils.escapeHtml(mode === "set" ? I18n.t("settings-security-set-password") : I18n.t("settings-security-password-update"))}
          </button>
        </div>
      </div>
    `;

  I18n.localizeTree(body);
  modal.hidden = false;
  enhanceAllSelects(body);

  // Close modal function with keyboard listener cleanup
  let handleKeydown;
  const closeModal = () => {
    if (handleKeydown) {
      document.removeEventListener("keydown", handleKeydown);
    }
    modal.hidden = true;
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
  modal.onclick = (event) => {
    if (event.target === modal) closeModal();
  };
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
      newPasswordInput.placeholder =
        type === "pin4"
          ? I18n.t("settings-security-placeholder-pin4")
          : I18n.t("settings-security-placeholder-pin6");
    } else {
      newPasswordInput.type = "password";
      newPasswordInput.inputMode = "text";
      newPasswordInput.pattern = "";
      newPasswordInput.placeholder = I18n.t("settings-security-placeholder-text");
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
      Utils.showToast(I18n.t("settings-security-password-required"), "error");
      return;
    }

    if (newPassword !== confirmPassword) {
      Utils.showToast(I18n.t("settings-security-password-mismatch"), "error");
      return;
    }

    // Validate PIN format
    if (passwordType === "pin4" && !/^\d{4}$/.test(newPassword)) {
      Utils.showToast(I18n.t("settings-security-pin4-invalid"), "error");
      return;
    }
    if (passwordType === "pin6" && !/^\d{6}$/.test(newPassword)) {
      Utils.showToast(I18n.t("settings-security-pin6-invalid"), "error");
      return;
    }
    if (passwordType === "text" && newPassword.length < 4) {
      Utils.showToast(I18n.t("settings-security-text-too-short"), "error");
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
        Utils.showToast(I18n.t("settings-security-password-saved"), "success");
        closeModal();
        // Reload security tab
        await loadSecurityTab(dialog, content);
        // Update lockscreen controller
        if (window.Lockscreen) {
          window.Lockscreen.loadStatus();
        }
      } else {
        const data = await response.json();
        Utils.showToast(
          apiErrorMessage(data, I18n.t("settings-security-password-save-failed")),
          "error"
        );
      }
    } catch (error) {
      Utils.showToast(
        I18n.t("settings-security-password-save-failed-detail", { message: error.message }),
        "error"
      );
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

  if (!modal || !body) return;

  title.textContent = I18n.t("settings-security-remove-title");

  body.innerHTML = `
      <div class="security-form">
        <p style="color: var(--text-secondary); margin-bottom: 16px;" data-l10n-id="settings-security-remove-description"></p>

        <div class="security-form-group">
          <label data-l10n-id="settings-security-password-current-label"></label>
          <input type="password" id="securityCurrentPasswordRemove" class="settings-input" data-l10n-id="settings-security-password-current-input" autofocus />
        </div>

        <div class="security-form-actions">
          <button class="btn btn-secondary" id="securityCancelRemoveBtn" data-l10n-id="common-action-cancel"></button>
          <button class="btn btn-warning" id="securityConfirmRemoveBtn">
            <i class="icon-trash-2"></i> <span data-l10n-id="settings-security-remove-confirm"></span>
          </button>
        </div>
      </div>
    `;

  I18n.localizeTree(body);
  modal.hidden = false;

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
    modal.hidden = true;
  };

  closeBtn.onclick = closeModal;
  modal.onclick = (event) => {
    if (event.target === modal) closeModal();
  };
  body.querySelector("#securityCancelRemoveBtn").onclick = closeModal;

  // Confirm handler
  body.querySelector("#securityConfirmRemoveBtn").onclick = async () => {
    const currentPassword = body.querySelector("#securityCurrentPasswordRemove").value;

    if (!currentPassword) {
      Utils.showToast(I18n.t("settings-security-current-required"), "error");
      return;
    }

    try {
      const response = await fetch("/api/lockscreen/clear-password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ current_password: currentPassword }),
      });

      if (response.ok) {
        Utils.showToast(I18n.t("settings-security-password-removed"), "success");
        closeModal();
        // Reload security tab
        await loadSecurityTab(dialog, content);
        // Update lockscreen controller
        if (window.Lockscreen) {
          window.Lockscreen.loadStatus();
        }
      } else {
        const data = await response.json();
        Utils.showToast(
          apiErrorMessage(data, I18n.t("settings-security-password-remove-failed")),
          "error"
        );
      }
    } catch (error) {
      Utils.showToast(
        I18n.t("settings-security-password-remove-failed-detail", { message: error.message }),
        "error"
      );
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

  if (!modal || !body) return;

  title.textContent = I18n.t("settings-security-2fa-enable-title");
  body.innerHTML = `
      <div class="totp-setup-step" id="totpStep1">
        <p style="color: var(--text-secondary); margin-bottom: 1rem;" data-l10n-id="settings-security-2fa-password-prompt"></p>
        <div class="security-form-group">
          <input type="password" id="totpSetupPassword" class="settings-input" data-l10n-id="settings-security-2fa-password-input" autocomplete="current-password">
        </div>
        <div class="totp-setup-actions" style="display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem;">
          <button class="btn btn-secondary btn-sm" id="totpCancelBtn" data-l10n-id="common-action-cancel"></button>
          <button class="btn btn-primary btn-sm" id="totpContinueBtn" data-l10n-id="settings-security-2fa-continue"></button>
        </div>
      </div>
      <div class="totp-setup-step" id="totpStep2" style="display: none;">
        <div id="totpQrContainer" style="text-align: center; margin: 1rem 0;"></div>
        <div class="totp-manual-entry" style="margin: 1rem 0;">
          <label style="font-size: 0.75rem; color: var(--text-secondary);" data-l10n-id="settings-security-2fa-manual-code"></label>
          <code id="totpSecretCode" dir="ltr" style="display: block; padding: 0.5rem; background: var(--bg-tertiary); border-radius: 4px; margin-top: 0.25rem; word-break: break-all; font-family: var(--font-mono);"></code>
        </div>
        <p style="margin: 1rem 0; color: var(--text-secondary);" data-l10n-id="settings-security-2fa-code-prompt"></p>
        <div class="security-form-group">
          <input type="text" id="totpVerifyCode" class="settings-input" placeholder="000000" maxlength="6" pattern="[0-9]{6}" style="text-align: center; font-size: 1.25rem; letter-spacing: 0.25em;">
        </div>
        <div class="totp-setup-actions" style="display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem;">
          <button class="btn btn-secondary btn-sm" id="totpBackBtn" data-l10n-id="common-action-back"></button>
          <button class="btn btn-primary btn-sm" id="totpVerifyBtn" data-l10n-id="settings-security-2fa-verify-enable"></button>
        </div>
      </div>
    `;

  I18n.localizeTree(body);
  modal.hidden = false;

  let currentSecret = "";

  // Close handlers
  const closeModal = () => {
    modal.hidden = true;
  };

  closeBtn.onclick = closeModal;
  modal.onclick = (event) => {
    if (event.target === modal) closeModal();
  };

  // Cancel button
  body.querySelector("#totpCancelBtn")?.addEventListener("click", closeModal);

  // Continue to step 2
  body.querySelector("#totpContinueBtn")?.addEventListener("click", async () => {
    const password = body.querySelector("#totpSetupPassword")?.value;
    if (!password) {
      Utils.showToast(I18n.t("settings-security-2fa-password-required"), "error");
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
        Utils.showToast(
          apiErrorMessage(data, I18n.t("settings-security-2fa-setup-failed")),
          "error"
        );
        return;
      }

      const result = data.data || data;
      currentSecret = result.secret;

      // Show QR code
      const qrContainer = body.querySelector("#totpQrContainer");
      qrContainer.innerHTML = `<img src="${result.qr_code}" data-l10n-id="settings-security-2fa-qr" style="max-width: 200px; height: auto; border-radius: 8px;">`;
      I18n.localizeTree(qrContainer);

      // Show manual code
      body.querySelector("#totpSecretCode").textContent = result.secret;

      // Switch to step 2
      body.querySelector("#totpStep1").style.display = "none";
      body.querySelector("#totpStep2").style.display = "block";
    } catch {
      Utils.showToast(I18n.t("settings-security-2fa-setup-failed"), "error");
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
      Utils.showToast(I18n.t("settings-security-2fa-code-invalid-length"), "error");
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
        Utils.showToast(
          apiErrorMessage(data, I18n.t("settings-security-2fa-code-invalid")),
          "error"
        );
        return;
      }

      Utils.showToast(I18n.t("settings-security-2fa-enabled"), "success");
      closeModal();
      // Reload security tab
      await loadSecurityTab(dialog, content);
    } catch {
      Utils.showToast(I18n.t("settings-security-2fa-verify-failed"), "error");
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

  if (!modal || !body) return;

  title.textContent = I18n.t("settings-security-2fa-disable-title");
  body.innerHTML = `
      <div class="security-form">
        <p style="color: var(--text-secondary); margin-bottom: 1rem;" data-l10n-id="settings-security-2fa-disable-prompt"></p>
        <div class="security-form-group">
          <input type="password" id="totpDisablePassword" class="settings-input" data-l10n-id="settings-security-2fa-password-input" autocomplete="current-password">
        </div>
        <div class="totp-setup-actions" style="display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem;">
          <button class="btn btn-secondary btn-sm" id="totpDisableCancelBtn" data-l10n-id="common-action-cancel"></button>
          <button class="btn btn-warning btn-sm" id="totpDisableConfirmBtn" data-l10n-id="settings-security-2fa-disable"></button>
        </div>
      </div>
    `;

  I18n.localizeTree(body);
  modal.hidden = false;

  // Close handlers
  const closeModal = () => {
    modal.hidden = true;
  };

  closeBtn.onclick = closeModal;
  modal.onclick = (event) => {
    if (event.target === modal) closeModal();
  };

  body.querySelector("#totpDisableCancelBtn")?.addEventListener("click", closeModal);

  body.querySelector("#totpDisableConfirmBtn")?.addEventListener("click", async () => {
    const password = body.querySelector("#totpDisablePassword")?.value;
    if (!password) {
      Utils.showToast(I18n.t("settings-security-2fa-password-required"), "error");
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
        Utils.showToast(
          apiErrorMessage(data, I18n.t("settings-security-2fa-disable-failed")),
          "error"
        );
        return;
      }

      Utils.showToast(I18n.t("settings-security-2fa-disabled"), "success");
      closeModal();
      await loadSecurityTab(dialog, content);
    } catch {
      Utils.showToast(I18n.t("settings-security-2fa-disable-failed"), "error");
    }
  });
}
