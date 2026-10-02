// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Login Page Module
 *
 * Handles password authentication with optional TOTP 2FA for headless mode dashboard access.
 * Its only core dependency is the request manager's error-envelope helper.
 */

import { apiErrorMessage } from "../core/request_manager.js";

// Simple DOM helper
const $ = (selector) => document.querySelector(selector);

// Replace a submit button's content with an icon and a text label.
function setButtonContent(button, iconClass, label) {
  const icon = document.createElement("i");
  icon.className = iconClass;
  const text = document.createElement("span");
  text.textContent = label;
  button.replaceChildren(icon, " ", text);
}

// State to track verified password for TOTP step
let verifiedPassword = "";

/**
 * Simple fetch wrapper for API calls
 */
async function fetchApi(url, options = {}) {
  const defaultOptions = {
    headers: {
      "Content-Type": "application/json",
    },
  };

  const mergedOptions = { ...defaultOptions, ...options };

  try {
    const response = await fetch(url, mergedOptions);
    const data = await response.json();

    if (!response.ok) {
      return { ...data, success: false };
    }

    return data;
  } catch (err) {
    return { success: false, error: { message: err.message || I18n.t("auth-error-network") } };
  }
}

/**
 * Create the login page lifecycle
 */
function createLifecycle() {
  let passwordVisible = false;

  return {
    /**
     * Initialize the login page
     */
    async init() {
      const form = $("#loginForm");
      const passwordInput = $("#loginPassword");
      const passwordToggle = $("#loginPasswordToggle");
      const passwordIcon = $("#loginPasswordIcon");
      const errorContainer = $("#loginError");
      const totpInput = $("#loginTotpCode");
      const totpBackBtn = $("#loginTotpBack");

      if (!form || !passwordInput) {
        console.error("[Login] Required elements not found");
        return;
      }

      // Focus password input
      passwordInput.focus();

      // Toggle password visibility
      if (passwordToggle) {
        passwordToggle.addEventListener("click", () => {
          passwordVisible = !passwordVisible;
          passwordInput.type = passwordVisible ? "text" : "password";
          if (passwordIcon) {
            passwordIcon.className = passwordVisible ? "icon-eye-off" : "icon-eye";
          }
        });
      }

      // Handle form submission
      form.addEventListener("submit", async (e) => {
        e.preventDefault();
        const totpStep = $("#loginTotpStep");
        if (totpStep && totpStep.style.display !== "none") {
          // TOTP step is visible - submit TOTP
          await handleTotpSubmit(totpInput, errorContainer);
        } else {
          // Password step - submit password
          await handleLogin(passwordInput, errorContainer);
        }
      });

      // TOTP input auto-submit on 6 digits
      if (totpInput) {
        totpInput.addEventListener("input", async (e) => {
          // Only allow digits
          e.target.value = e.target.value.replace(/\D/g, "");

          // Auto-submit when 6 digits entered
          if (e.target.value.length === 6) {
            await handleTotpSubmit(totpInput, errorContainer);
          }
        });
      }

      // Back button from TOTP step
      if (totpBackBtn) {
        totpBackBtn.addEventListener("click", () => {
          showPasswordStep();
        });
      }

      // Fetch auth status to customize page
      await customizeLoginPage();
    },

    /**
     * Called when page becomes active
     */
    activate() {
      const passwordInput = $("#loginPassword");
      if (passwordInput) {
        passwordInput.focus();
        passwordInput.value = "";
      }
      verifiedPassword = "";
      showPasswordStep();
      hideError();
    },

    /**
     * Called when page becomes inactive
     */
    deactivate() {
      // Clear inputs for security
      const passwordInput = $("#loginPassword");
      const totpInput = $("#loginTotpCode");
      if (passwordInput) passwordInput.value = "";
      if (totpInput) totpInput.value = "";
      verifiedPassword = "";
    },

    /**
     * Cleanup when page is disposed
     */
    dispose() {
      verifiedPassword = "";
    },
  };
}

/**
 * Handle password login submission
 */
async function handleLogin(passwordInput, errorContainer) {
  const password = passwordInput.value.trim();

  if (!password) {
    showError(errorContainer, I18n.t("auth-error-password-required"));
    shakeForm();
    return;
  }

  const submitBtn = $("#loginSubmit");
  if (submitBtn) {
    submitBtn.disabled = true;
    setButtonContent(submitBtn, "icon-loader spin", I18n.t("auth-login-signing-in"));
  }

  try {
    const response = await fetchApi("/api/auth/login", {
      method: "POST",
      body: JSON.stringify({ password }),
    });

    if (response.success) {
      // Login successful - redirect to home
      window.location.href = "/";
    } else if (response.requires_totp) {
      // Password verified, TOTP required
      verifiedPassword = password;
      showTotpStep();
      hideError();
    } else {
      // Show error
      const errorMessage = apiErrorMessage(response, I18n.t("auth-error-login-failed"));
      showError(errorContainer, errorMessage);
      shakeForm();
      passwordInput.focus();
      passwordInput.select();
    }
  } catch (err) {
    console.error("[Login] Error:", err);
    showError(errorContainer, I18n.t("auth-error-connection"));
    shakeForm();
  } finally {
    if (submitBtn) {
      submitBtn.disabled = false;
      setButtonContent(submitBtn, "icon-log-in", I18n.t("auth-login-submit"));
    }
  }
}

/**
 * Handle TOTP code submission
 */
async function handleTotpSubmit(totpInput, errorContainer) {
  const code = totpInput.value.trim();

  if (!code || code.length !== 6) {
    showError(errorContainer, I18n.t("auth-error-totp-required"));
    shakeForm();
    return;
  }

  if (!verifiedPassword) {
    // Session expired or state lost - go back to password step
    showError(errorContainer, I18n.t("auth-error-session-expired"));
    showPasswordStep();
    return;
  }

  const submitBtn = $("#loginTotpSubmit");
  if (submitBtn) {
    submitBtn.disabled = true;
    setButtonContent(submitBtn, "icon-loader spin", I18n.t("auth-totp-verifying"));
  }

  try {
    const response = await fetchApi("/api/auth/login", {
      method: "POST",
      body: JSON.stringify({
        password: verifiedPassword,
        totp_code: code,
      }),
    });

    if (response.success) {
      // Login successful - redirect to home
      verifiedPassword = "";
      window.location.href = "/";
    } else {
      // Show error
      const errorMessage = apiErrorMessage(response, I18n.t("auth-error-totp-invalid"));
      showError(errorContainer, errorMessage);
      shakeForm();
      totpInput.value = "";
      totpInput.focus();
    }
  } catch (err) {
    console.error("[Login] TOTP Error:", err);
    showError(errorContainer, I18n.t("auth-error-connection"));
    shakeForm();
  } finally {
    if (submitBtn) {
      submitBtn.disabled = false;
      setButtonContent(submitBtn, "icon-shield-check", I18n.t("auth-totp-verify"));
    }
  }
}

/**
 * Show the password step
 */
function showPasswordStep() {
  const passwordStep = $("#loginPasswordStep");
  const totpStep = $("#loginTotpStep");
  const totpInput = $("#loginTotpCode");
  const passwordInput = $("#loginPassword");

  if (passwordStep) passwordStep.style.display = "block";
  if (totpStep) totpStep.style.display = "none";
  if (totpInput) totpInput.value = "";
  if (passwordInput) passwordInput.focus();

  verifiedPassword = "";
}

/**
 * Show the TOTP step
 */
function showTotpStep() {
  const passwordStep = $("#loginPasswordStep");
  const totpStep = $("#loginTotpStep");
  const totpInput = $("#loginTotpCode");

  if (passwordStep) passwordStep.style.display = "none";
  if (totpStep) totpStep.style.display = "block";
  if (totpInput) {
    totpInput.value = "";
    totpInput.focus();
  }
}

/**
 * Customize login page based on auth status
 */
async function customizeLoginPage() {
  try {
    const response = await fetchApi("/api/auth/status");

    if (response.authenticated) {
      // Already authenticated, redirect to home
      window.location.href = "/";
      return;
    }

    // Customize branding
    const logo = $("#loginLogo");
    const brand = $("#loginBrand");
    const subtitle = $("#loginSubtitle");

    if (logo && !response.show_logo) {
      logo.style.display = "none";
    }

    if (brand && !response.show_name) {
      brand.style.display = "none";
    }

    if (subtitle && response.custom_title) {
      subtitle.textContent = response.custom_title;
    }
  } catch {
    // Ignore errors - use default styling
  }
}

/**
 * Show error message
 */
function showError(container, message) {
  if (!container) return;

  const textEl = $("#loginErrorText");
  if (textEl) {
    textEl.textContent = message;
  }

  container.style.display = "flex";
}

/**
 * Hide error message
 */
function hideError() {
  const container = $("#loginError");
  if (container) {
    container.style.display = "none";
  }
}

/**
 * Shake the form to indicate error
 */
function shakeForm() {
  const form = $("#loginForm");
  if (!form) return;

  form.classList.add("shake");
  setTimeout(() => {
    form.classList.remove("shake");
  }, 500);
}

// Auto-initialize when DOM is ready
document.addEventListener("DOMContentLoaded", async () => {
  const lifecycle = createLifecycle();
  await lifecycle.init();
  lifecycle.activate();
});

export { createLifecycle };
