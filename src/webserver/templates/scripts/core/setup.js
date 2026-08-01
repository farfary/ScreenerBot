// Full-screen setup controller.
//
// The credentials page performs only quick local checks. The verification page
// sends one immutable snapshot to the backend, receives a short-lived receipt,
// and saves that exact snapshot without repeating the RPC network tests.

(function () {
  "use strict";

  const {
    parseRpcUrls,
    requestJson,
    summarizeValidation,
    validateRpcValue,
    validateWalletValue,
    waitForScreenerBotRestart,
  } = window.SetupRuntime;

  class SetupControllerClass {
    constructor() {
      this.currentStep = 1;
      this.initialized = false;
      this.verificationBusy = false;
      this.verificationRun = 0;
      this.verificationAbort = null;
      this.restartAbort = null;
      this.snapshot = null;
      this.previousInstanceId = null;
      this.walletValidationTimeout = null;
      this.rpcValidationTimeout = null;
      this.accountPanel = null;
      this.gatewayBusy = false;
    }

    init() {
      if (this.initialized) return;

      this.screen = document.getElementById("setupScreen");
      this.stepContents = document.querySelectorAll(".setup-step-content[data-step]");
      this.stepIndicators = document.querySelectorAll(".setup-step[data-step]");
      this.footer = document.querySelector(".setup-footer");
      this.backBtn = document.getElementById("setup-back");
      this.nextBtn = document.getElementById("setup-next");
      this.skipBtn = document.getElementById("setup-skip");
      this.retryBtn = document.getElementById("setup-retry");
      this.reconnectBtn = document.getElementById("setup-reconnect");
      this.reloadBtn = document.getElementById("setup-reload");
      this.errorEl = document.getElementById("setup-error");
      this.errorMessages = document.getElementById("setup-error-messages");
      this.accountPanelEl = document.getElementById("setupAccountPanel");
      this.gatewayOptionEl = document.getElementById("setup-gateway-option");
      this.gatewayCheckboxEl = document.getElementById("setup-use-gateway");
      this.walletInput = document.getElementById("wallet-private-key");
      this.rpcInput = document.getElementById("rpc-urls");
      this.verificationSummary = document.getElementById("verification-summary");
      this.completeState = document.getElementById("setup-complete-state");
      this.completeIcon = this.completeState?.querySelector(".setup-complete-icon");
      this.completeText = document.getElementById("setup-complete-text");
      this.restartIndicator = document.getElementById("setup-restart-indicator");
      this.servicesStatus = document.getElementById("services-status");
      this.completeActions = document.getElementById("setup-complete-actions");

      if (!this.screen || !this.stepContents.length || !this.walletInput || !this.rpcInput) {
        console.warn("[Setup] Required setup elements were not found");
        return;
      }

      this.bindEvents();
      this.maskWallet(true);
      this.loadVersion();
      this.mountAccountPanel();
      this.attachGatewayHandler();
      this.resetVerificationStates();
      this.setStep(1);
      this.initialized = true;
    }

    bindEvents() {
      this.backBtn?.addEventListener("click", () => this.goBack());
      this.nextBtn?.addEventListener("click", () => this.goNext());
      this.skipBtn?.addEventListener("click", () => this.skipSetup());
      this.retryBtn?.addEventListener("click", () => this.reviewCredentials());
      this.reconnectBtn?.addEventListener("click", () => this.startRestartWait());
      this.reloadBtn?.addEventListener("click", () => window.location.reload());

      this.walletInput.addEventListener("input", () => this.debouncedValidateWallet());
      this.rpcInput.addEventListener("input", () => this.debouncedValidateRpc());

      this.toggleBtn = document.querySelector('[data-toggle="wallet-private-key"]');
      this.toggleBtn?.addEventListener("click", () => this.toggleVisibility());
      this.copyBtn = document.querySelector(".wallet-copy-btn");
      this.copyBtn?.addEventListener("click", () => this.copyWalletAddress());
    }

    async loadVersion() {
      try {
        const result = await requestJson("/api/version");
        const versionEl = document.getElementById("setup-version");
        if (versionEl && result?.version) versionEl.textContent = `v${result.version}`;
      } catch {
        // Version decoration is optional and must never block setup.
      }
    }

    setStep(step, options = {}) {
      this.currentStep = step;
      this.screen.dataset.step = String(step);

      this.stepIndicators.forEach((indicator) => {
        const indicatorStep = Number(indicator.dataset.step);
        indicator.classList.toggle("active", indicatorStep === step);
        indicator.classList.toggle("completed", indicatorStep < step);
        if (indicatorStep === step) indicator.setAttribute("aria-current", "step");
        else indicator.removeAttribute("aria-current");
      });

      this.stepContents.forEach((content) => {
        const active = Number(content.dataset.step) === step;
        content.classList.toggle("active", active);
        content.hidden = !active;
        content.setAttribute("aria-hidden", String(!active));
        if (active) content.removeAttribute("inert");
        else content.setAttribute("inert", "");
      });

      this.updateButtons();

      if (options.focusHeading) {
        const heading = this.screen.querySelector(`.setup-step-content[data-step="${step}"] h1`);
        if (heading) {
          heading.tabIndex = -1;
          window.requestAnimationFrame(() => heading.focus({ preventScroll: true }));
        }
      }
    }

    updateButtons() {
      if (this.footer) this.footer.hidden = false;

      if (this.backBtn) {
        this.backBtn.hidden = this.currentStep > 2;
        this.backBtn.disabled = this.verificationBusy;
      }
      if (this.skipBtn) {
        this.skipBtn.hidden = this.currentStep !== 1;
        this.skipBtn.disabled = this.verificationBusy;
      }
      if (this.nextBtn) {
        this.nextBtn.hidden = this.currentStep !== 1;
        this.nextBtn.disabled = this.verificationBusy;
      }
    }

    goNext() {
      if (this.currentStep === 1 && !this.verificationBusy) this.validateAndProceed();
    }

    goBack() {
      if (this.verificationBusy) return;

      if (this.currentStep === 1) {
        this.hideError();
        window.OnboardingController?.showFromSetup();
        return;
      }

      if (this.currentStep !== 2) return;
      this.cancelVerification();
      this.hideError();
      this.resetVerificationStates();
      this.setStep(1, { focusHeading: true });
    }

    reviewCredentials() {
      if (this.verificationBusy) return;
      this.hideError();
      this.resetVerificationStates();
      this.setStep(1, { focusHeading: true });
    }

    cancelVerification() {
      this.verificationRun += 1;
      this.verificationAbort?.abort();
      this.verificationAbort = null;
      this.verificationBusy = false;
      this.snapshot = null;
      this.updateButtons();
    }

    debouncedValidateWallet() {
      window.clearTimeout(this.walletValidationTimeout);
      this.walletValidationTimeout = window.setTimeout(() => this.validateWallet(false), 250);
    }

    debouncedValidateRpc() {
      window.clearTimeout(this.rpcValidationTimeout);
      this.rpcValidationTimeout = window.setTimeout(() => this.validateRpc(false), 250);
    }

    setInlineValidation(id, input, state, message) {
      const element = document.getElementById(id);
      if (!element) return;

      element.className = `setup-validation${state ? ` ${state}` : ""}`;
      element.textContent = message || "";
      element.hidden = !message;
      if (state === "error") input.setAttribute("aria-invalid", "true");
      else input.removeAttribute("aria-invalid");
    }

    validateWallet(required) {
      const result = validateWalletValue(this.walletInput.value, required);
      this.setInlineValidation("wallet-validation", this.walletInput, result.state, result.message);
      if (!result.valid) this.hideWalletPreview();
      return result.valid;
    }

    rpcUrls() {
      return parseRpcUrls(this.rpcInput.value);
    }

    validateRpc(required) {
      const result = validateRpcValue(this.rpcInput.value, required);
      this.setInlineValidation("rpc-validation", this.rpcInput, result.state, result.message);
      return result.valid;
    }

    async validateAndProceed() {
      window.clearTimeout(this.walletValidationTimeout);
      window.clearTimeout(this.rpcValidationTimeout);
      this.hideError();

      const walletValid = this.validateWallet(true);
      const rpcValid = this.validateRpc(true);
      if (!walletValid || !rpcValid) {
        (walletValid ? this.rpcInput : this.walletInput).focus({ preventScroll: true });
        return;
      }

      this.snapshot = Object.freeze({
        walletPrivateKey: this.walletInput.value.trim(),
        rpcUrls: Object.freeze([...this.rpcUrls()]),
      });
      this.verificationBusy = true;
      this.verificationRun += 1;
      const run = this.verificationRun;
      this.verificationAbort?.abort();
      this.verificationAbort = new AbortController();
      this.resetVerificationStates();
      this.setStep(2, { focusHeading: true });
      this.updateButtons();

      await this.runVerification(run, this.verificationAbort.signal, this.snapshot);
    }

    isCurrentRun(run, signal) {
      return run === this.verificationRun && !signal.aborted;
    }

    async runVerification(run, signal, snapshot) {
      this.setVerificationState(
        "wallet",
        "running",
        "Parsing private key",
        "Checking the key and deriving its public address."
      );
      this.setVerificationState(
        "rpc",
        "running",
        "Testing Solana mainnet",
        `Checking ${snapshot.rpcUrls.length} endpoint${snapshot.rpcUrls.length === 1 ? "" : "s"}.`
      );
      this.setVerificationState("save", "pending", "Waiting to save", "");
      if (this.verificationSummary) {
        this.verificationSummary.textContent = "Verifying the exact credentials you entered.";
      }

      try {
        const validation = await requestJson("/api/initialization/validate", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            wallet_private_key: snapshot.walletPrivateKey,
            rpc_urls: snapshot.rpcUrls,
          }),
          signal,
        });
        if (!this.isCurrentRun(run, signal)) return;

        this.renderValidationResult(validation);
        if (!validation?.valid || !validation?.validation_id) {
          throw new Error(validation?.errors?.join(" ") || "Credential verification failed.");
        }

        this.setVerificationState(
          "save",
          "running",
          "Encrypting and saving",
          "Writing the verified configuration on this device."
        );
        if (this.verificationSummary) {
          this.verificationSummary.textContent = "Credentials verified. Saving securely.";
        }

        const completed = await requestJson("/api/initialization/complete", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            validation_id: validation.validation_id,
            wallet_private_key: snapshot.walletPrivateKey,
            rpc_urls: snapshot.rpcUrls,
          }),
          signal,
        });
        if (!this.isCurrentRun(run, signal)) return;
        if (!completed?.success) {
          throw new Error(completed?.errors?.join(" ") || "Setup could not be saved.");
        }

        this.setVerificationState(
          "save",
          "success",
          "Configuration saved",
          "Private key encrypted; working RPC endpoints stored."
        );
        this.walletInput.value = "";
        this.hideWalletPreview();
        this.snapshot = null;
        this.verificationBusy = false;
        this.previousInstanceId = completed.instance_id;
        this.setStep(3, { focusHeading: true });
        this.startRestartWait();
      } catch (error) {
        if (error?.name === "AbortError" || !this.isCurrentRun(run, signal)) return;

        if (document.getElementById("save-verification-card")?.dataset.state === "running") {
          this.setVerificationState("save", "error", "Could not save setup", error.message);
        } else if (!document.querySelector('.setup-verification-item[data-state="error"]')) {
          this.setVerificationState("wallet", "error", "Verification request failed", "");
          this.setVerificationState("rpc", "error", "Verification request failed", "");
          this.setVerificationState("save", "pending", "Not saved", "");
        }

        if (this.verificationSummary) {
          this.verificationSummary.textContent = "Review the issue, then verify again.";
        }
        this.showError(error.message || "Verification failed.", true);
        this.verificationBusy = false;
        this.updateButtons();
      }
    }

    renderValidationResult(validation) {
      const summary = summarizeValidation(validation);
      this.setVerificationState(
        "wallet",
        summary.wallet.state,
        summary.wallet.label,
        summary.wallet.details
      );
      this.setVerificationState("rpc", summary.rpc.state, summary.rpc.label, summary.rpc.details);
      if (summary.wallet.address) this.showWalletPreview(summary.wallet.address);
    }

    setVerificationState(name, state, label, details) {
      const card = document.getElementById(`${name}-verification-card`);
      const status = document.getElementById(`${name}-status`);
      const detail = document.getElementById(`${name}-details`);
      if (!card || !status || !detail) return;

      card.dataset.state = state;
      const labelElement = status.querySelector("span:last-child");
      if (labelElement) labelElement.textContent = label;
      detail.textContent = details || "";
      detail.hidden = !details;
    }

    resetVerificationStates() {
      this.setVerificationState("wallet", "pending", "Waiting to validate", "");
      this.setVerificationState("rpc", "pending", "Waiting to test endpoints", "");
      this.setVerificationState("save", "pending", "Waiting to save", "");
      if (this.verificationSummary) {
        this.verificationSummary.textContent =
          "Checking your wallet and Solana mainnet connections.";
      }
    }

    async skipSetup() {
      if (this.verificationBusy) return;
      this.hideError();
      this.skipBtn.disabled = true;
      const originalLabel = this.skipBtn.textContent;
      this.skipBtn.textContent = "Opening preview…";

      try {
        const result = await requestJson("/api/initialization/skip", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: "{}",
        });
        if (!result?.success) {
          throw new Error(result?.errors?.join(" ") || "Preview mode could not be started.");
        }
        window.location.assign("/tokens");
      } catch (error) {
        this.skipBtn.disabled = false;
        this.skipBtn.textContent = originalLabel;
        this.showError(error.message || "Preview mode could not be started.", true);
      }
    }

    startRestartWait() {
      if (!this.previousInstanceId) return;
      this.restartAbort?.abort();
      this.restartAbort = new AbortController();
      this.completeState?.setAttribute("data-state", "waiting");
      if (this.completeIcon) {
        this.completeIcon.className = "setup-complete-icon icon-circle-check";
      }
      if (this.completeText) {
        this.completeText.textContent = "Restarting ScreenerBot with your verified configuration.";
      }
      if (this.servicesStatus) this.servicesStatus.textContent = "Waiting for the new process…";
      if (this.restartIndicator) this.restartIndicator.hidden = false;
      if (this.completeActions) this.completeActions.hidden = true;

      waitForScreenerBotRestart(this.previousInstanceId, {
        target: "/home",
        signal: this.restartAbort.signal,
        onReady: () => {
          if (this.servicesStatus) {
            this.servicesStatus.textContent = "ScreenerBot is ready. Opening dashboard…";
          }
        },
      }).catch((error) => {
        if (error?.name === "AbortError") return;
        this.completeState?.setAttribute("data-state", "error");
        if (this.completeIcon) {
          this.completeIcon.className = "setup-complete-icon icon-triangle-alert";
        }
        if (this.completeText) {
          this.completeText.textContent =
            "Your verified configuration is safely stored on this device.";
        }
        if (this.servicesStatus) this.servicesStatus.textContent = error.message;
        if (this.restartIndicator) this.restartIndicator.hidden = true;
        if (this.completeActions) this.completeActions.hidden = false;
      });
    }

    mountAccountPanel() {
      if (!this.accountPanelEl || !window.AccountPanel) return;
      this.accountPanel = window.AccountPanel.mount(this.accountPanelEl, {
        onChange: (status) => this.onAccountChanged(status),
      });
    }

    onAccountChanged(status) {
      if (!this.gatewayOptionEl || !this.gatewayCheckboxEl) return;
      const canUseGateway = Boolean(status?.signed_in) && status?.scopes?.includes("rpc:submit");
      this.gatewayOptionEl.classList.toggle("is-hidden", !canUseGateway);
      this.gatewayOptionEl.hidden = !canUseGateway;
      this.gatewayCheckboxEl.checked = Boolean(status?.use_gateway_rpc);
    }

    attachGatewayHandler() {
      if (!this.gatewayCheckboxEl) return;

      this.gatewayCheckboxEl.addEventListener("change", async () => {
        if (this.gatewayBusy) return;
        const requested = this.gatewayCheckboxEl.checked;
        this.gatewayBusy = true;
        this.gatewayCheckboxEl.disabled = true;

        try {
          const status = await requestJson("/api/account/gateway", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ enabled: requested }),
          });
          this.onAccountChanged(status);
        } catch (error) {
          this.gatewayCheckboxEl.checked = !requested;
          this.showError(error.message || "Gateway preference could not be saved.", false);
        } finally {
          this.gatewayBusy = false;
          this.gatewayCheckboxEl.disabled = false;
        }
      });
    }

    maskWallet(masked) {
      this.walletInput.style.webkitTextSecurity = masked ? "disc" : "none";
      this.walletInput.style.textSecurity = masked ? "disc" : "none";
      if (!this.toggleBtn) return;

      this.toggleBtn.setAttribute("aria-pressed", String(!masked));
      const action = masked ? "Show private key" : "Hide private key";
      this.toggleBtn.setAttribute("aria-label", action);
      this.toggleBtn.title = action;
      const icon = this.toggleBtn.querySelector(".toggle-icon");
      if (icon) icon.className = `toggle-icon ${masked ? "icon-eye" : "icon-eye-off"}`;
    }

    toggleVisibility() {
      const isMasked = this.walletInput.style.webkitTextSecurity !== "none";
      this.maskWallet(!isMasked);
    }

    showWalletPreview(address) {
      const preview = document.getElementById("wallet-address-preview");
      const text = preview?.querySelector(".wallet-address-text");
      if (!preview || !text) return;
      text.textContent = address;
      preview.hidden = false;
    }

    hideWalletPreview() {
      const preview = document.getElementById("wallet-address-preview");
      const text = preview?.querySelector(".wallet-address-text");
      if (text) text.textContent = "";
      if (preview) preview.hidden = true;
    }

    async copyWalletAddress() {
      const address = document.querySelector(".wallet-address-text")?.textContent?.trim();
      if (!address || !this.copyBtn) return;

      try {
        await navigator.clipboard.writeText(address);
        this.copyBtn.setAttribute("aria-label", "Wallet address copied");
        this.copyBtn.title = "Copied";
        window.setTimeout(() => {
          this.copyBtn?.setAttribute("aria-label", "Copy wallet address");
          if (this.copyBtn) this.copyBtn.title = "Copy wallet address";
        }, 1500);
      } catch {
        this.copyBtn.setAttribute("aria-label", "Could not copy wallet address");
        this.copyBtn.title = "Copy failed";
      }
    }

    showError(message, focus) {
      if (!this.errorEl || !this.errorMessages) return;
      this.errorMessages.textContent = message;
      if (this.retryBtn) {
        this.retryBtn.textContent = this.currentStep === 2 ? "Review credentials" : "Dismiss";
      }
      this.errorEl.hidden = false;
      if (focus) window.requestAnimationFrame(() => this.errorEl.focus({ preventScroll: true }));
    }

    hideError() {
      if (this.errorEl) this.errorEl.hidden = true;
      if (this.errorMessages) this.errorMessages.textContent = "";
    }

    dispose() {
      window.clearTimeout(this.walletValidationTimeout);
      window.clearTimeout(this.rpcValidationTimeout);
      this.cancelVerification();
      this.restartAbort?.abort();
      this.restartAbort = null;
      this.accountPanel?.destroy?.();
      this.accountPanel = null;
      this.initialized = false;
    }
  }

  window.SetupController = new SetupControllerClass();
})();
