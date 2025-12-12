// Setup Controller
// Handles the full-screen initialization form

class SetupControllerClass {
  constructor() {
    this.currentStep = 1;
    this.totalSteps = 3;
    this.initialized = false;
    this.servicesPoller = null;
  }

  init() {
    if (this.initialized) return;

    // Cache DOM elements
    this.stepContents = document.querySelectorAll(".setup-step-content");
    this.stepIndicators = document.querySelectorAll(".setup-step[data-step]");
    this.backBtn = document.getElementById("setup-back");
    this.nextBtn = document.getElementById("setup-next");
    this.retryBtn = document.getElementById("setup-retry");
    this.errorEl = document.getElementById("setup-error");
    this.errorMessages = document.getElementById("setup-error-messages");

    // Input fields
    this.walletInput = document.getElementById("wallet-private-key");
    this.rpcInput = document.getElementById("rpc-urls");

    if (!this.stepContents.length) {
      console.warn("[Setup] Step contents not found");
      return;
    }

    this.bindEvents();
    this.setupInputMasking();
    this.setStep(1);
    this.initialized = true;
  }

  bindEvents() {
    // Navigation buttons
    if (this.backBtn) {
      this.backBtn.addEventListener("click", () => this.goBack());
    }
    if (this.nextBtn) {
      this.nextBtn.addEventListener("click", () => this.goNext());
    }
    if (this.retryBtn) {
      this.retryBtn.addEventListener("click", () => this.retry());
    }

    // Input validation
    if (this.walletInput) {
      this.walletInput.addEventListener("input", () => this.validateWallet());
    }
    if (this.rpcInput) {
      this.rpcInput.addEventListener("input", () => this.validateRpc());
    }

    // Toggle password visibility
    const toggleBtn = document.querySelector('[data-toggle="wallet-private-key"]');
    if (toggleBtn) {
      toggleBtn.addEventListener("click", () => this.toggleVisibility());
    }
  }

  setupInputMasking() {
    // Initially mask wallet input
    if (this.walletInput) {
      this.walletInput.style.webkitTextSecurity = "disc";
      this.walletInput.style.textSecurity = "disc";
    }
  }

  toggleVisibility() {
    if (!this.walletInput) return;

    const toggleBtn = document.querySelector('[data-toggle="wallet-private-key"]');
    const icon = toggleBtn?.querySelector(".toggle-icon");

    if (
      this.walletInput.style.webkitTextSecurity === "none" ||
      this.walletInput.style.textSecurity === "none"
    ) {
      this.walletInput.style.webkitTextSecurity = "disc";
      this.walletInput.style.textSecurity = "disc";
      if (icon) icon.className = "toggle-icon icon-eye";
    } else {
      this.walletInput.style.webkitTextSecurity = "none";
      this.walletInput.style.textSecurity = "none";
      if (icon) icon.className = "toggle-icon icon-eye-off";
    }
  }

  setStep(step) {
    this.currentStep = step;

    // Update step indicators
    this.stepIndicators.forEach((el) => {
      const stepNum = parseInt(el.dataset.step, 10);
      el.classList.remove("active", "completed");
      if (stepNum === step) {
        el.classList.add("active");
      } else if (stepNum < step) {
        el.classList.add("completed");
      }
    });

    // Update step content visibility
    this.stepContents.forEach((el) => {
      el.classList.remove("active");
      if (parseInt(el.dataset.step, 10) === step) {
        el.classList.add("active");
      }
    });

    // Update button states
    this.updateButtons();
  }

  updateButtons() {
    if (this.backBtn) {
      this.backBtn.disabled = this.currentStep === 1;
      this.backBtn.style.visibility = this.currentStep === 3 ? "hidden" : "visible";
    }

    if (this.nextBtn) {
      if (this.currentStep === 3) {
        this.nextBtn.style.display = "none";
      } else if (this.currentStep === 2) {
        this.nextBtn.style.display = "none"; // Hidden during verification
      } else {
        this.nextBtn.style.display = "inline-flex";
        this.nextBtn.innerHTML = 'Continue <i class="icon-chevron-right"></i>';
      }
    }
  }

  goBack() {
    if (this.currentStep > 1) {
      this.hideError();
      this.resetVerificationStates();
      this.setStep(this.currentStep - 1);
    }
  }

  async goNext() {
    if (this.currentStep === 1) {
      await this.validateAndProceed();
    }
  }

  retry() {
    this.hideError();
    this.resetVerificationStates();
    this.setStep(1);
  }

  // Validation methods
  validateWallet() {
    const value = this.walletInput?.value.trim() || "";
    const validationEl = document.getElementById("wallet-validation");
    if (!validationEl) return;

    if (!value) {
      validationEl.className = "setup-validation";
      validationEl.style.display = "none";
      return;
    }

    // Basic format check
    if (value.startsWith("[") && value.endsWith("]")) {
      validationEl.className = "setup-validation success";
      validationEl.innerHTML = '<i class="icon-check"></i> Valid JSON array format';
    } else if (value.length >= 32) {
      validationEl.className = "setup-validation success";
      validationEl.innerHTML = '<i class="icon-check"></i> Valid base58 format';
    } else {
      validationEl.className = "setup-validation error";
      validationEl.innerHTML =
        '<i class="icon-x"></i> Invalid key format. Must be base58 string or JSON array.';
    }
  }

  validateRpc() {
    const value = this.rpcInput?.value.trim() || "";
    const validationEl = document.getElementById("rpc-validation");
    if (!validationEl) return;

    if (!value) {
      validationEl.className = "setup-validation";
      validationEl.style.display = "none";
      return;
    }

    const urls = value
      .split("\n")
      .map((u) => u.trim())
      .filter((u) => u);

    if (urls.length === 0) {
      validationEl.className = "setup-validation error";
      validationEl.innerHTML = '<i class="icon-x"></i> Please enter at least one RPC URL';
      return;
    }

    const invalidUrls = urls.filter((url) => !url.startsWith("https://"));
    if (invalidUrls.length > 0) {
      validationEl.className = "setup-validation error";
      validationEl.innerHTML = '<i class="icon-x"></i> All RPC URLs must start with https://';
      return;
    }

    const hasDefaultRpc = urls.some((url) => url.includes("api.mainnet-beta.solana.com"));
    if (hasDefaultRpc) {
      validationEl.className = "setup-validation error";
      validationEl.innerHTML =
        '<i class="icon-triangle-alert"></i> Default Solana RPC will not work. Use a premium provider.';
      return;
    }

    validationEl.className = "setup-validation success";
    validationEl.innerHTML = `<i class="icon-check"></i> ${urls.length} valid URL${urls.length > 1 ? "s" : ""}`;
  }

  async validateAndProceed() {
    const walletValue = this.walletInput?.value.trim();
    const rpcValue = this.rpcInput?.value.trim();

    if (!walletValue || !rpcValue) {
      this.showError("Please fill in all required fields");
      return;
    }

    // Disable button during validation
    if (this.nextBtn) {
      this.nextBtn.disabled = true;
      this.nextBtn.innerHTML = '<i class="icon-loader-2"></i> Validating...';
    }

    try {
      this.hideError();
      this.setStep(2);
      await this.runVerification();
    } catch (error) {
      this.showError(error.message || "Verification failed");
      this.setStep(1);
    } finally {
      if (this.nextBtn) {
        this.nextBtn.disabled = false;
        this.nextBtn.innerHTML = 'Continue <i class="icon-chevron-right"></i>';
      }
    }
  }

  async runVerification() {
    const walletStatus = document.getElementById("wallet-status");
    const walletDetails = document.getElementById("wallet-details");
    const rpcStatus = document.getElementById("rpc-status");
    const rpcDetails = document.getElementById("rpc-details");

    try {
      // Call validate API
      const result = await this.validateCredentials();

      if (!result || !result.valid) {
        const errorMsg =
          result?.errors?.length > 0 ? result.errors.join("; ") : "Validation failed";
        throw new Error(errorMsg);
      }

      // Update wallet validation
      if (walletStatus) {
        walletStatus.className = "setup-verification-status success";
        walletStatus.innerHTML =
          '<i class="icon-check"></i><span>Wallet validated successfully</span>';
      }
      if (walletDetails && result.wallet_address) {
        walletDetails.classList.add("show");
        walletDetails.textContent = `Address: ${result.wallet_address}`;
      }

      // Update RPC validation
      if (rpcStatus) {
        rpcStatus.className = "setup-verification-status success";
        rpcStatus.innerHTML = '<i class="icon-check"></i><span>RPC connections verified</span>';
      }
      if (rpcDetails) {
        rpcDetails.classList.add("show");
        if (result.warnings && result.warnings.length > 0) {
          rpcDetails.textContent = result.warnings.join("; ");
        } else {
          rpcDetails.textContent = "All RPC endpoints are operational";
        }
      }

      // Small delay for visual effect
      await new Promise((resolve) => setTimeout(resolve, 500));

      // Complete initialization
      const completeResult = await this.completeInitialization();

      if (!completeResult.success) {
        const errorMsg =
          completeResult.errors?.length > 0
            ? completeResult.errors.join("; ")
            : "Initialization failed";
        throw new Error(errorMsg);
      }

      // Move to step 3
      setTimeout(() => {
        this.setStep(3);
        this.startServicesProgress();
      }, 1000);
    } catch (error) {
      this.showError(error.message);
      // Reset verification states
      if (walletStatus) {
        walletStatus.className = "setup-verification-status error";
        walletStatus.innerHTML = '<i class="icon-x"></i><span>Verification failed</span>';
      }
      if (rpcStatus) {
        rpcStatus.className = "setup-verification-status error";
        rpcStatus.innerHTML = '<i class="icon-x"></i><span>Verification failed</span>';
      }
    }
  }

  async validateCredentials() {
    const walletPrivateKey = this.walletInput?.value.trim();
    const rpcUrls = this.rpcInput?.value
      .split("\n")
      .map((u) => u.trim())
      .filter((u) => u);

    const response = await fetch("/api/initialization/validate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        wallet_private_key: walletPrivateKey,
        rpc_urls: rpcUrls,
      }),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    return response.json();
  }

  async completeInitialization() {
    const walletPrivateKey = this.walletInput?.value.trim();
    const rpcUrls = this.rpcInput?.value
      .split("\n")
      .map((u) => u.trim())
      .filter((u) => u);

    const response = await fetch("/api/initialization/complete", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        wallet_private_key: walletPrivateKey,
        rpc_urls: rpcUrls,
      }),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    return response.json();
  }

  startServicesProgress() {
    const progressFill = document.getElementById("services-progress");
    const progressText = document.getElementById("services-status");

    let totalServices = 20;

    // Clear existing poller
    if (this.servicesPoller) {
      clearInterval(this.servicesPoller);
      this.servicesPoller = null;
    }

    this.servicesPoller = setInterval(async () => {
      try {
        const response = await fetch("/api/services");
        const result = await response.json();

        if (result.services && Array.isArray(result.services)) {
          totalServices = result.services.length;
          const runningServices = result.services.filter(
            (s) => s.health?.status === "healthy"
          ).length;

          const progress = (runningServices / totalServices) * 100;
          if (progressFill) {
            progressFill.style.width = `${progress}%`;
          }
          if (progressText) {
            progressText.textContent = `Initializing services (${runningServices}/${totalServices})...`;
          }

          // If all services are running, redirect to dashboard
          if (runningServices >= totalServices - 1) {
            clearInterval(this.servicesPoller);
            this.servicesPoller = null;
            if (progressText) {
              progressText.textContent = "Complete! Redirecting to dashboard...";
            }
            setTimeout(() => {
              window.location.href = "/home";
            }, 1500);
          }
        }
      } catch (error) {
        console.error("[Setup] Error polling services:", error);
      }
    }, 1000);
  }

  showError(message) {
    if (this.errorEl) {
      this.errorEl.classList.add("show");
    }
    if (this.errorMessages) {
      this.errorMessages.textContent = message;
    }
  }

  hideError() {
    if (this.errorEl) {
      this.errorEl.classList.remove("show");
    }
  }

  resetVerificationStates() {
    ["wallet-status", "rpc-status"].forEach((id) => {
      const el = document.getElementById(id);
      if (el) {
        el.className = "setup-verification-status";
        el.innerHTML = '<span class="setup-spinner"></span><span>Pending</span>';
      }
    });

    ["wallet-details", "rpc-details"].forEach((id) => {
      const el = document.getElementById(id);
      if (el) {
        el.classList.remove("show");
      }
    });

    // Reset password toggle
    if (this.walletInput) {
      this.walletInput.style.webkitTextSecurity = "disc";
      this.walletInput.style.textSecurity = "disc";
    }
    const toggleBtn = document.querySelector('[data-toggle="wallet-private-key"]');
    if (toggleBtn) {
      const icon = toggleBtn.querySelector(".toggle-icon");
      if (icon) icon.className = "toggle-icon icon-eye";
    }
  }

  dispose() {
    if (this.servicesPoller) {
      clearInterval(this.servicesPoller);
      this.servicesPoller = null;
    }
    this.initialized = false;
  }
}

// Export for use
window.SetupController = new SetupControllerClass();
