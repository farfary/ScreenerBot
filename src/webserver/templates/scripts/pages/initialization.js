import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import * as Utils from "../core/utils.js";
import { $, $$ } from "../core/dom.js";
import { requestManager } from "../core/request_manager.js";

// State management
const state = {
  currentStep: 1,
  credentials: {
    walletPrivateKey: "",
    rpcUrls: [],
  },
  validation: {
    wallet: null,
    rpc: null,
  },
  verification: {
    wallet: null,
    rpc: null,
    license: null,
  },
  errors: [],
};

// Cleanup references
let servicesPoller = null;
let eventListeners = [];

// Helper to track event listeners for cleanup
function addTrackedListener(element, event, handler) {
  if (element) {
    element.addEventListener(event, handler);
    eventListeners.push({ element, event, handler });
  }
}

function removeAllListeners() {
  eventListeners.forEach(({ element, event, handler }) => {
    if (element) {
      element.removeEventListener(event, handler);
    }
  });
  eventListeners = [];
}

// Debounce helper
function debounce(func, wait) {
  let timeout;
  return function executedFunction(...args) {
    const later = () => {
      clearTimeout(timeout);
      func(...args);
    };
    clearTimeout(timeout);
    timeout = setTimeout(later, wait);
  };
}

// Show/hide elements
function show(el) {
  if (el) el.style.display = "block";
}

function hide(el) {
  if (el) el.style.display = "none";
}

// Step navigation
function setStep(step) {
  state.currentStep = step;

  // Update progress indicators
  $$("[data-step]").forEach((el) => {
    el.classList.remove("active", "completed");
    const elStep = parseInt(el.dataset.step, 10);
    if (elStep === step) {
      el.classList.add("active");
    } else if (elStep < step) {
      el.classList.add("completed");
    }
  });

  // Update step content visibility
  $$(".init-step-content").forEach((el) => {
    el.classList.remove("active");
    if (parseInt(el.dataset.step, 10) === step) {
      el.classList.add("active");
    }
  });

  // Update button states
  const backBtn = $("#init-back");
  const nextBtn = $("#init-next");

  if (backBtn) {
    backBtn.disabled = step === 1;
  }

  if (nextBtn) {
    if (step === 3) {
      nextBtn.style.display = "none";
    } else {
      nextBtn.style.display = "inline-flex";
      nextBtn.textContent = step === 2 ? "Complete Setup →" : "Next →";
    }
  }
}

// Validation UI
function showValidation(fieldId, type, message) {
  const validationEl = $(`#${fieldId}-validation`);
  if (!validationEl) return;

  validationEl.className = `init-validation ${type}`;
  validationEl.textContent = message;
  validationEl.style.display = "block";
}

function hideValidation(fieldId) {
  const validationEl = $(`#${fieldId}-validation`);
  if (validationEl) {
    validationEl.style.display = "none";
  }
}

// Real-time validation
const validateWalletKey = debounce(async () => {
  const input = $("#wallet-private-key");
  if (!input) return;

  const value = input.value.trim();
  if (!value) {
    hideValidation("wallet");
    return;
  }

  // Basic format check
  if (value.startsWith("[") && value.endsWith("]")) {
    showValidation("wallet", "success", "✓ Valid JSON array format");
  } else if (value.length >= 32) {
    showValidation("wallet", "success", "✓ Valid base58 format");
  } else {
    showValidation("wallet", "error", "Invalid key format. Must be base58 string or JSON array.");
  }
}, 500);

const validateRpcUrls = debounce(async () => {
  const input = $("#rpc-urls");
  if (!input) return;

  const value = input.value.trim();
  if (!value) {
    hideValidation("rpc");
    return;
  }

  const urls = value
    .split("\n")
    .map((u) => u.trim())
    .filter((u) => u);

  if (urls.length === 0) {
    showValidation("rpc", "error", "Please enter at least one RPC URL");
    return;
  }

  // Check if any URLs don't start with https://
  const invalidUrls = urls.filter((url) => !url.startsWith("https://"));
  if (invalidUrls.length > 0) {
    showValidation("rpc", "error", "All RPC URLs must start with https://");
    return;
  }

  // Check if using default Solana RPC (not recommended)
  const hasDefaultRpc = urls.some((url) => url.includes("api.mainnet-beta.solana.com"));
  if (hasDefaultRpc) {
    showValidation(
      "rpc",
      "error",
      '<i class="icon-alert-triangle"></i> Default Solana RPC will not work. Please use a premium provider.'
    );
    return;
  }

  showValidation("rpc", "success", `✓ ${urls.length} valid URL${urls.length > 1 ? "s" : ""}`);
}, 500);

// Toggle password visibility
function setupToggle() {
  const toggleBtn = $('[data-toggle="wallet-private-key"]');
  if (!toggleBtn) return;

  const handler = () => {
    const input = $("#wallet-private-key");
    if (!input) return;

    const icon = toggleBtn.querySelector(".toggle-icon");
    if (input.style.webkitTextSecurity === "none" || input.style.textSecurity === "none") {
      input.style.webkitTextSecurity = "disc";
      input.style.textSecurity = "disc";
      icon.className = "toggle-icon icon-eye";
    } else {
      input.style.webkitTextSecurity = "none";
      input.style.textSecurity = "none";
      icon.className = "toggle-icon icon-eye-off";
    }
  };

  addTrackedListener(toggleBtn, "click", handler);

  // Initialize as hidden
  const input = $("#wallet-private-key");
  if (input) {
    input.style.webkitTextSecurity = "disc";
    input.style.textSecurity = "disc";
  }
}

// API calls
async function validateCredentials() {
  const walletInput = $("#wallet-private-key");
  const rpcInput = $("#rpc-urls");

  if (!walletInput || !rpcInput) {
    throw new Error("Input fields not found");
  }

  const walletPrivateKey = walletInput.value.trim();
  const rpcUrls = rpcInput.value
    .split("\n")
    .map((u) => u.trim())
    .filter((u) => u);

  if (!walletPrivateKey) {
    throw new Error("Wallet private key is required");
  }

  if (rpcUrls.length === 0) {
    throw new Error("At least one RPC URL is required");
  }

  return await requestManager.fetch("/api/initialization/validate", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      wallet_private_key: walletPrivateKey,
      rpc_urls: rpcUrls,
    }),
    priority: "high",
  });
}

async function completeInitialization() {
  const walletInput = $("#wallet-private-key");
  const rpcInput = $("#rpc-urls");

  if (!walletInput || !rpcInput) {
    throw new Error("Input fields not found");
  }

  const walletPrivateKey = walletInput.value.trim();
  const rpcUrls = rpcInput.value
    .split("\n")
    .map((u) => u.trim())
    .filter((u) => u);

  const response = await requestManager.fetch("/api/initialization/complete", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      wallet_private_key: walletPrivateKey,
      rpc_urls: rpcUrls,
    }),
    priority: "high",
  });

  return response;
}

// Verification step
async function runVerification() {
  // Update wallet status
  const walletStatus = $("#wallet-status");
  const walletDetails = $("#wallet-details");
  const rpcStatus = $("#rpc-status");
  const rpcDetails = $("#rpc-details");
  const licenseStatus = $("#license-status");
  const licenseDetails = $("#license-details");

  try {
    // Call validate API
    const result = await validateCredentials();

    // API returns ValidationResult directly with 'valid' field
    if (!result || !result.valid) {
      const errorMsg = result?.errors?.length > 0 ? result.errors.join("; ") : "Validation failed";
      throw new Error(errorMsg);
    }

    const data = result; // Result IS the data

    // Update wallet validation
    if (walletStatus) {
      walletStatus.className = "init-verification-status success";
      walletStatus.innerHTML = "<span>✓</span><span>Wallet validated successfully</span>";
    }
    if (walletDetails && data.wallet_address) {
      walletDetails.classList.add("show");
      walletDetails.textContent = `Address: ${data.wallet_address}`;
    }

    // Update RPC validation
    if (rpcStatus) {
      rpcStatus.className = "init-verification-status success";
      rpcStatus.innerHTML = "<span>✓</span><span>RPC connections verified</span>";
    }
    if (rpcDetails && data.warnings && data.warnings.length > 0) {
      rpcDetails.classList.add("show");
      rpcDetails.textContent = data.warnings.join("; ");
    } else if (rpcDetails) {
      rpcDetails.classList.add("show");
      rpcDetails.textContent = "All RPC endpoints are operational";
    }

    // Small delay for visual effect
    await new Promise((resolve) => setTimeout(resolve, 500));

    // Now complete initialization (which verifies license)
    if (licenseStatus) {
      licenseStatus.className = "init-verification-status loading";
      licenseStatus.innerHTML =
        '<span class="init-spinner"></span><span>Verifying license...</span>';
    }

    const completeResult = await completeInitialization();

    // Backend returns flat structure with success at top level
    if (!completeResult.success) {
      const errorMsg =
        completeResult.errors?.length > 0
          ? completeResult.errors.join("; ")
          : "License verification failed";
      throw new Error(errorMsg);
    }

    const licenseData = completeResult.license_status;

    if (licenseData && licenseData.valid) {
      if (licenseStatus) {
        licenseStatus.className = "init-verification-status success";
        licenseStatus.innerHTML = "<span>✓</span><span>License verified successfully</span>";
      }
      if (licenseDetails) {
        licenseDetails.classList.add("show");
        const tier = licenseData.tier || "Unknown";
        // Backend uses expiry_ts (UNIX timestamp), format it
        const expiryDate = licenseData.expiry_ts
          ? new Date(licenseData.expiry_ts * 1000).toLocaleDateString()
          : "N/A";
        licenseDetails.textContent = `Tier: ${tier} | Expires: ${expiryDate}`;
      }

      // Move to step 3
      setTimeout(() => {
        setStep(3);
        startServicesProgress();
      }, 1000);
    } else {
      throw new Error(licenseData?.reason || "No valid license found");
    }
  } catch (error) {
    // Show error
    showError(error.message);
  }
}

// Services startup progress
async function startServicesProgress() {
  const progressFill = $("#services-progress");
  const progressText = $("#services-status");

  let servicesStarted = 0;
  let totalServices = 20; // Will be updated from API

  // Clear existing poller if any
  if (servicesPoller) {
    servicesPoller.stop();
    servicesPoller.cleanup();
    servicesPoller = null;
  }

  // Use managed Poller instead of raw setInterval
  servicesPoller = new Poller(
    async () => {
      try {
        const result = await requestManager.fetch("/api/services", {
          priority: "normal",
        });
        if (result.services && Array.isArray(result.services)) {
          // Update total services count from first response
          if (totalServices === 20) {
            totalServices = result.services.length;
          }

          const runningServices = result.services.filter(
            (s) => s.health?.status === "healthy"
          ).length;
          servicesStarted = runningServices;

          const progress = (servicesStarted / totalServices) * 100;
          if (progressFill) {
            progressFill.style.width = `${progress}%`;
          }
          if (progressText) {
            progressText.textContent = `Initializing services (${servicesStarted}/${totalServices})...`;
          }

          // If all services are running, redirect to dashboard
          if (servicesStarted >= totalServices - 1) {
            // Allow for 1 service to still be starting
            servicesPoller.stop();
            servicesPoller.cleanup();
            servicesPoller = null;
            if (progressText) {
              progressText.textContent = "Complete! Redirecting to dashboard...";
            }
            setTimeout(() => {
              window.location.href = "/services";
            }, 1500);
          }
        }
      } catch (error) {
        console.error("Error polling services:", error);
        // Stop polling on persistent errors to prevent runaway requests
      }
    },
    { label: "ServicesInit", interval: 1000 }
  );

  servicesPoller.start();
}

// Error handling
function showError(message) {
  const errorEl = $("#init-error");
  const errorMessages = $("#init-error-messages");

  if (errorEl) show(errorEl);
  if (errorMessages) errorMessages.textContent = message;

  // Set all verification to error state
  ["wallet-status", "rpc-status", "license-status"].forEach((id) => {
    const el = $(`#${id}`);
    if (el) {
      el.className = "init-verification-status error";
      el.innerHTML = "<span>❌</span><span>Verification failed</span>";
    }
  });
}

function hideError() {
  const errorEl = $("#init-error");
  if (errorEl) hide(errorEl);
}

function resetVerificationStates() {
  // Reset all verification status indicators
  ["wallet-status", "rpc-status", "license-status"].forEach((id) => {
    const el = $(`#${id}`);
    if (el) {
      el.className = "init-verification-status";
      el.innerHTML = '<span class="init-spinner"></span><span>Pending</span>';
    }
  });

  // Reset password toggle to hidden
  const input = $("#wallet-private-key");
  if (input) {
    input.style.webkitTextSecurity = "disc";
    input.style.textSecurity = "disc";
  }
  const toggleBtn = $('[data-toggle="wallet-private-key"]');
  if (toggleBtn) {
    const icon = toggleBtn.querySelector(".toggle-icon");
    if (icon) icon.className = "toggle-icon icon-eye";
  }
}

// Lifecycle
function createLifecycle() {
  return {
    init() {
      // Clean up any existing listeners first
      removeAllListeners();

      // Setup event listeners with tracking
      const walletInput = $("#wallet-private-key");
      const rpcInput = $("#rpc-urls");
      const nextBtn = $("#init-next");
      const backBtn = $("#init-back");
      const retryBtn = $("#init-retry");

      addTrackedListener(walletInput, "input", validateWalletKey);
      addTrackedListener(rpcInput, "input", validateRpcUrls);

      if (nextBtn) {
        const nextHandler = async () => {
          if (state.currentStep === 1) {
            // Validate and move to step 2
            hideError();
            const walletValue = walletInput?.value.trim();
            const rpcValue = rpcInput?.value.trim();

            if (!walletValue || !rpcValue) {
              showError("Please fill in all required fields");
              return;
            }

            // Disable button to prevent concurrent calls
            nextBtn.disabled = true;
            nextBtn.textContent = "Validating...";

            try {
              setStep(2);
              await runVerification();
            } catch (error) {
              showError(error.message || "Verification failed");
              setStep(1);
            } finally {
              nextBtn.disabled = false;
              nextBtn.textContent = "Next →";
            }
          } else if (state.currentStep === 2) {
            // This shouldn't happen as button is hidden in step 2
            // But if it does, complete initialization
            runVerification();
          }
        };
        addTrackedListener(nextBtn, "click", nextHandler);
      }

      if (backBtn) {
        const backHandler = () => {
          if (state.currentStep > 1) {
            hideError();
            // Reset verification states when going back
            resetVerificationStates();
            setStep(state.currentStep - 1);
          }
        };
        addTrackedListener(backBtn, "click", backHandler);
      }

      if (retryBtn) {
        const retryHandler = () => {
          hideError();
          resetVerificationStates();
          setStep(1);
        };
        addTrackedListener(retryBtn, "click", retryHandler);
      }

      // Setup password toggle
      setupToggle();

      // Initialize at step 1
      setStep(1);
    },

    activate() {
      // Page activated
    },

    deactivate() {
      // Page deactivated
    },

    dispose() {
      // Clean up all tracked event listeners
      removeAllListeners();

      // Clean up services poller
      if (servicesPoller) {
        servicesPoller.stop();
        servicesPoller.cleanup();
        servicesPoller = null;
      }
    },
  };
}

registerPage("initialization", createLifecycle());
